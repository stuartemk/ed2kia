//! SAE Simulator — Generador de payloads dummy para MVP local.
//!
//! Crea tensores `candle::Tensor` con dimensiones 128x256 que simulan
//! gradientes QLoRA. Dos perfiles:
//! - **Nodo Alpha (Simbiótico):** Gradiente normalizado → Z ≈ +0.8
//! - **Nodo Beta (Perverso):** Gradiente invertido + ruido → Z ≈ -0.9
//!
//! Ley 2 (Reconocimiento del Error): Beta es detectado como perverso.
//! Ley 3 (Cero desperdicio): Tensores pequeños (128x256), sin descargas.

use candle_core::{Device, Tensor};
use thiserror::Error;

/// Error del simulador SAE.
#[derive(Debug, Error)]
pub enum SaeSimError {
    #[error("Candle error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("Invalid dimensions: rows={rows}, cols={cols}")]
    InvalidDimensions { rows: usize, cols: usize },
}

/// Payload simulado de gradiente SAE.
#[derive(Debug, Clone)]
pub struct SaePayload {
    /// Identificador del nodo origen.
    pub node_id: String,
    /// Gradiente simulado (flattened).
    pub gradient: Vec<f32>,
    /// Dimensiones originales.
    pub dimensions: (usize, usize),
    /// Perfil del nodo.
    pub profile: NodeProfile,
    /// Valor Z esperado tras evaluación SCT.
    pub expected_z: f32,
}

/// Perfil de comportamiento del nodo.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeProfile {
    /// Nodo simbiótico: gradientes éticos, Z > 0.
    Symbiotic,
    /// Nodo perverso: gradientes maliciosos, Z < 0.
    Perverse,
}

impl std::fmt::Display for NodeProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeProfile::Symbiotic => write!(f, "Symbiotic"),
            NodeProfile::Perverse => write!(f, "Perverse"),
        }
    }
}

/// Simulador SAE — genera payloads deterministas.
#[derive(Debug)]
pub struct SaeSimulator {
    rows: usize,
    cols: usize,
    device: Device,
}

impl SaeSimulator {
    /// Construye simulador con dimensiones especificadas.
    pub fn new(rows: usize, cols: usize) -> Result<Self, SaeSimError> {
        if rows == 0 || cols == 0 {
            return Err(SaeSimError::InvalidDimensions { rows, cols });
        }
        Ok(Self {
            rows,
            cols,
            device: Device::Cpu,
        })
    }

    /// Genera payload simbiótico (Nodo Alpha).
    /// Gradiente normalizado con valores positivos → Z ≈ +0.8
    pub fn generate_symbiotic(&self, node_id: &str) -> Result<SaePayload, SaeSimError> {
        // Generate positive-biased gradient: mean ~0.5, std ~0.2
        let mut grad = Vec::with_capacity(self.rows * self.cols);
        for _ in 0..(self.rows * self.cols) {
            // Deterministic positive values for reproducibility
            let val = 0.3 + (grad.len() % 100) as f32 / 200.0; // range [0.3, 0.8]
            grad.push(val);
        }
        Ok(SaePayload {
            node_id: node_id.to_string(),
            gradient: grad,
            dimensions: (self.rows, self.cols),
            profile: NodeProfile::Symbiotic,
            expected_z: 0.8,
        })
    }

    /// Genera payload perverso (Nodo Beta).
    /// Gradiente invertido + ruido estructurado → Z ≈ -0.9
    pub fn generate_perverse(&self, node_id: &str) -> Result<SaePayload, SaeSimError> {
        // Generate negative-biased gradient: mean ~-0.6, std ~0.3
        let mut grad = Vec::with_capacity(self.rows * self.cols);
        for i in 0..(self.rows * self.cols) {
            // Deterministic negative values with structured noise
            let base = -0.5 - (i % 100) as f32 / 200.0; // range [-0.5, -1.0]
            let noise = ((i * 7) % 50) as f32 / 100.0 - 0.25; // structured noise [-0.25, 0.25]
            grad.push(base + noise);
        }
        Ok(SaePayload {
            node_id: node_id.to_string(),
            gradient: grad,
            dimensions: (self.rows, self.cols),
            profile: NodeProfile::Perverse,
            expected_z: -0.9,
        })
    }

    /// Convierte payload a Tensor candle.
    pub fn to_tensor(&self, payload: &SaePayload) -> Result<Tensor, SaeSimError> {
        Tensor::from_vec(
            payload.gradient.clone(),
            payload.dimensions.0 * payload.dimensions.1,
            &self.device,
        )
        .map_err(SaeSimError::Candle)
    }

    /// Serializa payload a bytes (bincode-compatible format).
    pub fn serialize(&self, payload: &SaePayload) -> Result<Vec<u8>, SaeSimError> {
        let mut buf = Vec::new();
        // Node ID length + node ID
        let id_bytes = payload.node_id.as_bytes();
        buf.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(id_bytes);
        // Gradient length + gradient data
        buf.extend_from_slice(&(payload.gradient.len() as u32).to_le_bytes());
        for &val in &payload.gradient {
            buf.extend_from_slice(&val.to_le_bytes());
        }
        // Dimensions
        buf.extend_from_slice(&(payload.dimensions.0 as u32).to_le_bytes());
        buf.extend_from_slice(&(payload.dimensions.1 as u32).to_le_bytes());
        // Profile flag
        buf.push(match payload.profile {
            NodeProfile::Symbiotic => 0u8,
            NodeProfile::Perverse => 1u8,
        });
        Ok(buf)
    }

    /// Deserializa payload desde bytes.
    pub fn deserialize(&self, data: &[u8]) -> Result<SaePayload, SaeSimError> {
        if data.len() < 18 {
            return Err(SaeSimError::InvalidDimensions {
                rows: 0,
                cols: 0,
            });
        }
        let mut pos = 0;
        // Node ID
        let id_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let node_id = String::from_utf8_lossy(&data[pos..pos + id_len]).to_string();
        pos += id_len;
        // Gradient
        let grad_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let mut gradient = Vec::with_capacity(grad_len);
        for _ in 0..grad_len {
            let val = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            gradient.push(val);
            pos += 4;
        }
        // Dimensions
        let rows = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let cols = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        // Profile
        let profile = match data[pos] {
            0 => NodeProfile::Symbiotic,
            _ => NodeProfile::Perverse,
        };
        let expected_z = match profile {
            NodeProfile::Symbiotic => 0.8,
            NodeProfile::Perverse => -0.9,
        };
        Ok(SaePayload {
            node_id,
            gradient,
            dimensions: (rows, cols),
            profile,
            expected_z,
        })
    }
}

impl Default for SaeSimulator {
    fn default() -> Self {
        Self {
            rows: 128,
            cols: 256,
            device: Device::Cpu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let sim = SaeSimulator::new(128, 256).unwrap();
        assert_eq!(sim.rows, 128);
        assert_eq!(sim.cols, 256);
    }

    #[test]
    fn test_simulator_invalid_dims() {
        match SaeSimulator::new(0, 256) {
            Err(SaeSimError::InvalidDimensions { .. }) => {},
            other => panic!("Expected InvalidDimensions, got {:?}", other),
        }
    }

    #[test]
    fn test_generate_symbiotic() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_symbiotic("alpha").unwrap();
        assert_eq!(payload.node_id, "alpha");
        assert_eq!(payload.profile, NodeProfile::Symbiotic);
        assert_eq!(payload.expected_z, 0.8);
        assert_eq!(payload.gradient.len(), 128 * 256);
        // All values should be positive
        assert!(payload.gradient.iter().all(|&v| v > 0.0));
    }

    #[test]
    fn test_generate_perverse() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_perverse("beta").unwrap();
        assert_eq!(payload.node_id, "beta");
        assert_eq!(payload.profile, NodeProfile::Perverse);
        assert_eq!(payload.expected_z, -0.9);
        assert_eq!(payload.gradient.len(), 128 * 256);
        // Most values should be negative
        let neg_count = payload.gradient.iter().filter(|&&v| v < 0.0).count();
        assert!(neg_count > payload.gradient.len() / 2);
    }

    #[test]
    fn test_serialize_deserialize_symbiotic() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_symbiotic("alpha").unwrap();
        let bytes = sim.serialize(&payload).unwrap();
        let restored = sim.deserialize(&bytes).unwrap();
        assert_eq!(restored.node_id, payload.node_id);
        assert_eq!(restored.gradient, payload.gradient);
        assert_eq!(restored.dimensions, payload.dimensions);
        assert_eq!(restored.profile, payload.profile);
    }

    #[test]
    fn test_serialize_deserialize_perverse() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_perverse("beta").unwrap();
        let bytes = sim.serialize(&payload).unwrap();
        let restored = sim.deserialize(&bytes).unwrap();
        assert_eq!(restored.node_id, payload.node_id);
        assert_eq!(restored.profile, NodeProfile::Perverse);
    }

    #[test]
    fn test_to_tensor() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_symbiotic("alpha").unwrap();
        let tensor = sim.to_tensor(&payload).unwrap();
        assert_eq!(tensor.shape().dims()[0], 128 * 256);
    }

    #[test]
    fn test_deserialize_too_short() {
        let sim = SaeSimulator::default();
        match sim.deserialize(&[0, 1, 2]) {
            Err(SaeSimError::InvalidDimensions { .. }) => {},
            other => panic!("Expected InvalidDimensions, got {:?}", other),
        }
    }

    #[test]
    fn test_profile_display() {
        assert_eq!(format!("{}", NodeProfile::Symbiotic), "Symbiotic");
        assert_eq!(format!("{}", NodeProfile::Perverse), "Perverse");
    }

    #[test]
    fn test_default_dimensions() {
        let sim = SaeSimulator::default();
        assert_eq!(sim.rows, 128);
        assert_eq!(sim.cols, 256);
    }

    #[test]
    fn test_symbiotic_mean_positive() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_symbiotic("alpha").unwrap();
        let mean: f32 = payload.gradient.iter().sum::<f32>() / payload.gradient.len() as f32;
        assert!(mean > 0.0, "Symbiotic mean should be positive, got {}", mean);
    }

    #[test]
    fn test_perverse_mean_negative() {
        let sim = SaeSimulator::default();
        let payload = sim.generate_perverse("beta").unwrap();
        let mean: f32 = payload.gradient.iter().sum::<f32>() / payload.gradient.len() as f32;
        assert!(mean < 0.0, "Perverse mean should be negative, got {}", mean);
    }
}
