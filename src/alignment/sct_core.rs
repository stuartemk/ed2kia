//! Stuartian Context Tensor (SCT) — Estructura matemática del Tensor Estuardiano.
//!
//! Reemplaza RLHF 2D con evaluación tensorial tridimensional `(X, Y, Z)`:
//! - Eje X (Beneficio): `[0.0, 1.0]` vía Sigmoid
//! - Eje Y (Costo/Fricción): `[0.0, 1.0]` vía Sigmoid
//! - Eje Z (Foco Estuardiano): `[-1.0, 1.0]` vía Tanh
//!
//! **Regla de Oro Estuardiana:** `if self.z < 0.0 { REJECTED }`
//! Rechazo hard determinista, sin excepciones.

use candle_core::Tensor;
use thiserror::Error;

/// Error específico del Tensor Estuardiano.
#[derive(Debug, Error)]
pub enum SctError {
    #[error("Z-axis out of bounds: {z:.4} (must be in [-1.0, 1.0])")]
    ZAxisOutOfBounds { z: f32 },

    #[error("X-axis out of bounds: {x:.4} (must be in [0.0, 1.0])")]
    XAxisOutOfBounds { x: f32 },

    #[error("Y-axis out of bounds: {y:.4} (must be in [0.0, 1.0])")]
    YAxisOutOfBounds { y: f32 },

    #[error("Tensor shape invalid for SCT: expected 3D logits, got {shape:?}")]
    InvalidTensorShape { shape: Vec<usize> },

    #[error("Candle error: {0}")]
    Candle(#[from] candle_core::Error),
}

/// Decisión del Tensor Estuardiano tras evaluación.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SCTDecision {
    /// Aprobado — Foco Estuardiano superior (Z > 0).
    Approved(f32),
    /// Rechazado — Foco Estuardiano inferior (Z < 0).
    Rejected(f32),
}

impl SCTDecision {
    /// Retorna `true` si la decisión es `Approved`.
    pub fn is_approved(&self) -> bool {
        matches!(self, SCTDecision::Approved(_))
    }

    /// Retorna `true` si la decisión es `Rejected`.
    pub fn is_rejected(&self) -> bool {
        matches!(self, SCTDecision::Rejected(_))
    }

    /// Retorna el valor Z asociado.
    pub fn z_value(&self) -> f32 {
        match self {
            SCTDecision::Approved(z) => *z,
            SCTDecision::Rejected(z) => -*z,
        }
    }
}

/// Tensor Estuardiano de Contexto — representación geométrica 3D.
///
/// - `x`: Beneficio percibido `[0.0, 1.0]`
/// - `y`: Costo/Fricción `[0.0, 1.0]`
/// - `z`: Foco Estuardiano `[-1.0, 1.0]`
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StuartianTensor {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl StuartianTensor {
    /// Construye un `StuartianTensor` validando los límites de cada eje.
    pub fn new(x: f32, y: f32, z: f32) -> Result<Self, SctError> {
        if !(0.0..=1.0).contains(&x) {
            return Err(SctError::XAxisOutOfBounds { x });
        }
        if !(0.0..=1.0).contains(&y) {
            return Err(SctError::YAxisOutOfBounds { y });
        }
        if !(-1.0..=1.0).contains(&z) {
            return Err(SctError::ZAxisOutOfBounds { z });
        }
        Ok(Self { x, y, z })
    }

    /// Evalúa la trayectoria según la Regla de Oro Estuardiana.
    ///
    /// `Z < 0` → Rechazo inmediato (perversidad sistémica / dependencia).
    /// `Z >= 0` → Aprobación (autonomía / diversidad).
    ///
    /// Con la feature `v2.1-stuartian-geometry`, utiliza `calculate_focal_gravity`
    /// para recalcular el eje Z con gravedad no lineal antes de evaluar.
    pub fn evaluate_trajectory(&self) -> Result<SCTDecision, SctError> {
        #[cfg(feature = "v2.1-stuartian-geometry")]
        {
            use crate::alignment::stuartian_geometry::calculate_focal_gravity;
            // Recalcular Z con gravedad focal: X como autonomía, (1-Y) como extracción
            // (menor costo = menor extracción)
            let autonomy_signal = self.x;
            let extraction_signal = 1.0 - self.y;
            let z_gravity = calculate_focal_gravity(autonomy_signal, extraction_signal);
            let z = self.z.max(z_gravity); // Toma el Z más conservador
            if z < 0.0 {
                return Ok(SCTDecision::Rejected(z.abs()));
            }
            return Ok(SCTDecision::Approved(z));
        }

        #[cfg(not(feature = "v2.1-stuartian-geometry"))]
        {
            if self.z < 0.0 {
                return Ok(SCTDecision::Rejected(self.z.abs()));
            }
            Ok(SCTDecision::Approved(self.z))
        }
    }

    /// Calcula la métrica de calidad estuardiana: `beneficio - costo + foco`.
    /// Mayor valor indica mejor alineación ética.
    pub fn stewardship_score(&self) -> f32 {
        self.x - self.y + self.z
    }
}

/// Trait para convertir `candle::Tensor` (logits) → `StuartianTensor`.
pub trait SCTEvaluator {
    /// Convierte un tensor de logits 3D a `StuartianTensor`.
    fn to_stuartian_tensor(&self) -> Result<StuartianTensor, SctError>;
}

impl SCTEvaluator for Tensor {
    fn to_stuartian_tensor(&self) -> Result<StuartianTensor, SctError> {
        let shape: Vec<usize> = self.shape().dims().to_vec();
        if shape.len() > 2 || (shape.len() == 1 && shape[0] != 3) {
            return Err(SctError::InvalidTensorShape { shape: shape.clone() });
        }
        let logits: Vec<f32> = match self.to_vec1::<f32>() {
            Ok(v) => v,
            Err(_) => {
                // Fallback para tensores 2D [1, 3]
                self.to_vec2::<f32>()
                    .unwrap_or_default()
                    .first()
                    .cloned()
                    .unwrap_or_default()
            }
        };
        if logits.len() != 3 {
            return Err(SctError::InvalidTensorShape {
                shape: vec![logits.len()],
            });
        }

        // X = sigmoid(logits[0]) → [0, 1]
        let x = 1.0 / (1.0 + (-logits[0]).exp());
        // Y = sigmoid(logits[1]) → [0, 1]
        let y = 1.0 / (1.0 + (-logits[1]).exp());
        // Z = tanh(logits[2]) → [-1, 1]
        let z = logits[2].tanh();

        StuartianTensor::new(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_high_benefit_low_cost_positive_z() {
        let tensor = StuartianTensor::new(0.9, 0.1, 0.8).unwrap();
        let decision = tensor.evaluate_trajectory().unwrap();
        assert!(decision.is_approved());
        assert!((decision.z_value() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_tensor_low_benefit_high_cost_positive_z() {
        // Dolor como conocimiento/autonomía → Aprobado
        let tensor = StuartianTensor::new(0.2, 0.8, 0.9).unwrap();
        let decision = tensor.evaluate_trajectory().unwrap();
        assert!(decision.is_approved());
        assert!((decision.z_value() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_tensor_negative_z_rejected() {
        // Trampa del sistema/dependencia → Rechazado
        let tensor = StuartianTensor::new(0.9, 0.1, -0.5).unwrap();
        let decision = tensor.evaluate_trajectory().unwrap();
        assert!(decision.is_rejected());
        assert!((decision.z_value() - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_tensor_zero_z_approved() {
        // Z = 0.0 es el umbral neutro → Aprobado (no es negativo)
        let tensor = StuartianTensor::new(0.5, 0.5, 0.0).unwrap();
        let decision = tensor.evaluate_trajectory().unwrap();
        assert!(decision.is_approved());
    }

    #[test]
    fn test_tensor_x_out_of_bounds() {
        let result = StuartianTensor::new(1.5, 0.5, 0.5);
        assert!(result.is_err());
        match result {
            Err(SctError::XAxisOutOfBounds { x }) => assert!((x - 1.5).abs() < 1e-6),
            _ => panic!("Expected XAxisOutOfBounds"),
        }
    }

    #[test]
    fn test_tensor_y_out_of_bounds() {
        let result = StuartianTensor::new(0.5, -0.2, 0.5);
        assert!(result.is_err());
        match result {
            Err(SctError::YAxisOutOfBounds { y }) => assert!((y - (-0.2)).abs() < 1e-6),
            _ => panic!("Expected YAxisOutOfBounds"),
        }
    }

    #[test]
    fn test_tensor_z_out_of_bounds() {
        let result = StuartianTensor::new(0.5, 0.5, 1.5);
        assert!(result.is_err());
        match result {
            Err(SctError::ZAxisOutOfBounds { z }) => assert!((z - 1.5).abs() < 1e-6),
            _ => panic!("Expected ZAxisOutOfBounds"),
        }
    }

    #[test]
    fn test_stewardship_score() {
        let tensor = StuartianTensor::new(0.9, 0.1, 0.8).unwrap();
        let score = tensor.stewardship_score();
        assert!((score - 1.6).abs() < 1e-6); // 0.9 - 0.1 + 0.8
    }

    #[test]
    fn test_decision_z_value_approved() {
        let decision = SCTDecision::Approved(0.7);
        assert!((decision.z_value() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_decision_z_value_rejected() {
        let decision = SCTDecision::Rejected(0.5);
        assert!((decision.z_value() - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_sigmoid_bounds() {
        // Sigmoid siempre produce [0, 1]
        let big_positive: f32 = 100.0;
        let sigmoid_pos = 1.0 / (1.0 + (-big_positive).exp());
        assert!(sigmoid_pos > 0.99);

        let big_negative: f32 = -100.0;
        let sigmoid_neg = 1.0 / (1.0 + (-big_negative).exp());
        assert!(sigmoid_neg < 0.01);
    }

    #[test]
    fn test_tanh_bounds() {
        // Tanh siempre produce [-1, 1]
        let big_positive: f32 = 100.0;
        let tanh_pos = big_positive.tanh();
        assert!(tanh_pos > 0.99 && tanh_pos <= 1.0);

        let big_negative: f32 = -100.0;
        let tanh_neg = big_negative.tanh();
        assert!(tanh_neg < -0.99 && tanh_neg >= -1.0);
    }

    #[test]
    fn test_golden_rule_strict_rejection() {
        // Cualquier Z negativo debe ser rechazado sin excepción
        let negative_values = [-0.0001, -0.1, -0.5, -0.99, -1.0];
        for z_val in negative_values {
            let tensor = StuartianTensor::new(0.5, 0.5, z_val).unwrap();
            let decision = tensor.evaluate_trajectory().unwrap();
            assert!(
                decision.is_rejected(),
                "Z={:.4} should be rejected per Golden Rule",
                z_val
            );
        }
    }

    #[test]
    fn test_golden_rule_approval_boundary() {
        // Z = 0.0 y positivos deben ser aprobados
        let positive_values = [0.0, 0.0001, 0.1, 0.5, 0.99, 1.0];
        for z_val in positive_values {
            let tensor = StuartianTensor::new(0.5, 0.5, z_val).unwrap();
            let decision = tensor.evaluate_trajectory().unwrap();
            assert!(
                decision.is_approved(),
                "Z={:.4} should be approved",
                z_val
            );
        }
    }

    #[test]
    fn test_error_display() {
        let err = SctError::ZAxisOutOfBounds { z: 1.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("1.5"));
    }
}
