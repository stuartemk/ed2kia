//! SCT Reward Model — Proyección 3D del Tensor Estuardiano.
//!
//! Capa `candle_nn::Linear` ligera que proyecta `hidden_state` → 3 dimensiones `(X, Y, Z)`.
//! Activaciones: `X = sigmoid`, `Y = sigmoid`, `Z = tanh`.
//! `SCTLoss` con penalización logarítmica masiva si predice `Z < 0` en datos
//! etiquetados "Foco Superior".
//!
//! Optimización WASM: O(1) overhead, sin backprop pesado, compatible con
//! `candle-core` device agnostic.

use candle_core::{Device, Tensor};
use candle_core::Module;
use candle_nn::Linear;
use thiserror::Error;

use crate::alignment::sct_core::{SctError, SCTDecision, StuartianTensor};

/// Error específico del modelo de recompensa SCT.
#[derive(Debug, Error)]
pub enum SctRewardError {
    #[error("Hidden dim must be > 0, got {hidden_dim}")]
    InvalidHiddenDim { hidden_dim: usize },

    #[error("Candle error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("SCT core error: {0}")]
    SctCore(#[from] SctError),
}

/// Modelo de Recompensa SCT — proyección lineal + activaciones 3D.
pub struct SctRewardModel {
    projection: Linear,
}

impl SctRewardModel {
    /// Construye un modelo SCT con la dimensión hidden especificada.
    ///
    /// La capa de proyección es `hidden_dim → 3` (sin bias para O(1) overhead).
    pub fn new(hidden_dim: usize, device: &Device) -> Result<Self, SctRewardError> {
        if hidden_dim == 0 {
            return Err(SctRewardError::InvalidHiddenDim { hidden_dim });
        }
        // Weight: [3, hidden_dim], no bias → O(1) overhead
        let weight = Tensor::zeros((3, hidden_dim), candle_core::DType::F32, device)?;
        let projection = Linear::new(weight, None);
        Ok(Self { projection })
    }

    /// Proyecta un hidden state a logits 3D y aplica activaciones.
    ///
    /// Retorna `StuartianTensor` con:
    /// - X = sigmoid(logits[0])
    /// - Y = sigmoid(logits[1])
    /// - Z = tanh(logits[2])
    pub fn forward(&self, hidden: &Tensor) -> Result<StuartianTensor, SctRewardError> {
        let logits = self.projection.forward(hidden)?;
        let vals: Vec<f32> = logits.flatten_all()?.to_vec1()?;
        if vals.len() != 3 {
            return Err(SctError::InvalidTensorShape {
                shape: vec![vals.len()],
            }
            .into());
        }

        let x = 1.0 / (1.0 + (-vals[0]).exp());
        let y = 1.0 / (1.0 + (-vals[1]).exp());
        let z = vals[2].tanh();

        StuartianTensor::new(x, y, z).map_err(SctRewardError::SctCore)
    }

    /// Evalúa directamente la decisión SCT desde un hidden state.
    pub fn evaluate(&self, hidden: &Tensor) -> Result<SCTDecision, SctRewardError> {
        let tensor = self.forward(hidden)?;
        tensor.evaluate_trajectory().map_err(SctRewardError::SctCore)
    }

    /// Calcula la pérdida SCT (SCTLoss).
    ///
    /// Penalización logarítmica masiva si predice `Z < 0` cuando el label
    /// indica "Foco Superior" (expected_z > 0).
    /// Recompensa si detecta perversidad oculta (expected_z < 0 y predice Z < 0).
    pub fn sct_loss(
        &self,
        hidden: &Tensor,
        expected_z: f32,
    ) -> Result<f32, SctRewardError> {
        let tensor = self.forward(hidden)?;
        let z_pred = tensor.z;

        // Pérdida MSE en Z
        let z_diff = z_pred - expected_z;
        let mse_loss = z_diff * z_diff;

        // Penalización logarítmica masiva si predice Z < 0 en datos "Foco Superior"
        let penalty: f32 = if expected_z > 0.0 && z_pred < 0.0 {
            // Log barrier: penaliza exponencialmente cerca de Z = 0 desde el lado negativo
            let margin = z_pred.abs() + 1e-8;
            (-margin.ln()) * 10.0
        } else if expected_z < 0.0 && z_pred < 0.0 {
            // Recompensa (pérdida negativa) por detectar perversidad
            -0.5
        } else {
            0.0
        };

        Ok(mse_loss + penalty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Result as CResult;

    fn make_hidden(dim: usize, device: &Device) -> CResult<Tensor> {
        Tensor::zeros((1, dim), candle_core::DType::F32, device)
    }

    #[test]
    fn test_model_creation() {
        let device = Device::Cpu;
        let model = SctRewardModel::new(64, &device);
        assert!(model.is_ok());
    }

    #[test]
    fn test_model_invalid_dim() {
        let device = Device::Cpu;
        let result = SctRewardModel::new(0, &device);
        assert!(result.is_err());
        match result {
            Err(SctRewardError::InvalidHiddenDim { hidden_dim }) => {
                assert_eq!(hidden_dim, 0);
            }
            _ => panic!("Expected InvalidHiddenDim"),
        }
    }

    #[test]
    fn test_forward_produces_valid_tensor() {
        let device = Device::Cpu;
        let model = SctRewardModel::new(32, &device).unwrap();
        let hidden = make_hidden(32, &device).unwrap();
        let tensor = model.forward(&hidden).unwrap();

        // X en [0, 1]
        assert!(tensor.x >= 0.0 && tensor.x <= 1.0);
        // Y en [0, 1]
        assert!(tensor.y >= 0.0 && tensor.y <= 1.0);
        // Z en [-1, 1]
        assert!(tensor.z >= -1.0 && tensor.z <= 1.0);
    }

    #[test]
    fn test_evaluate_returns_decision() {
        let device = Device::Cpu;
        let model = SctRewardModel::new(32, &device).unwrap();
        let hidden = make_hidden(32, &device).unwrap();
        let decision = model.evaluate(&hidden).unwrap();
        // Decision should be valid (either approved or rejected)
        let _ = decision.is_approved();
        let _ = decision.is_rejected();
    }

    #[test]
    fn test_sct_loss_positive_expected_z() {
        let device = Device::Cpu;
        let model = SctRewardModel::new(32, &device).unwrap();
        let hidden = make_hidden(32, &device).unwrap();
        let loss = model.sct_loss(&hidden, 0.8).unwrap();
        // Loss should be non-negative
        assert!(loss >= 0.0);
    }

    #[test]
    fn test_sct_loss_negative_expected_z() {
        let device = Device::Cpu;
        let model = SctRewardModel::new(32, &device).unwrap();
        let hidden = make_hidden(32, &device).unwrap();
        let _loss = model.sct_loss(&hidden, -0.8).unwrap();
        // Loss computed without panic
    }

    #[test]
    fn test_forward_deterministic() {
        let device = Device::Cpu;
        let model = SctRewardModel::new(16, &device).unwrap();
        let hidden = make_hidden(16, &device).unwrap();
        let t1 = model.forward(&hidden).unwrap();
        let t2 = model.forward(&hidden).unwrap();
        assert!((t1.x - t2.x).abs() < 1e-6);
        assert!((t1.y - t2.y).abs() < 1e-6);
        assert!((t1.z - t2.z).abs() < 1e-6);
    }

    #[test]
    fn test_error_display() {
        let err = SctRewardError::InvalidHiddenDim { hidden_dim: 0 };
        let msg = format!("{}", err);
        assert!(msg.contains("0"));
    }
}
