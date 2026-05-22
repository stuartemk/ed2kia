//! QLoRA Adapter — Aplicación de diffs QLoRA sobre modelos base GGUF.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Cero desperdicio computacional.
//! En lugar de distribuir modelos completos (GB), se distribuyen diffs QLoRA (KB/MB).
//!
//! **Matemática fundamental:** W' = W + B @ A
//! Donde:
//! - W: Pesos base del modelo (inmutables)
//! - A: Matriz de proyección (r x d_model)
//! - B: Matriz de reconstrucción (d_model x r)
//! - r: Rank (r << d_model), típicamente r ∈ [4, 64]

use std::fmt;

use candle_core::{DType, Device, Result as CandleResult, Tensor};

/// Error al crear o aplicar un QLoRA Adapter.
#[derive(Debug)]
pub enum QloraAdapterError {
    /// Rank inválido (debe ser 0 < r << d_model).
    InvalidRank(String),
    /// Dimensiones incompatibles.
    DimensionMismatch(String),
    /// Error de serialización.
    SerializationError(String),
    /// Error de Candle (tensor operation).
    CandleError(String),
    /// Data corrupta o inválida.
    CorruptedData(String),
}

impl fmt::Display for QloraAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QloraAdapterError::InvalidRank(msg) => write!(f, "Invalid QLoRA rank: {}", msg),
            QloraAdapterError::DimensionMismatch(msg) => {
                write!(f, "Dimension mismatch: {}", msg)
            }
            QloraAdapterError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            QloraAdapterError::CandleError(msg) => write!(f, "Candle error: {}", msg),
            QloraAdapterError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
        }
    }
}

impl std::error::Error for QloraAdapterError {}

impl From<candle_core::Error> for QloraAdapterError {
    fn from(err: candle_core::Error) -> Self {
        QloraAdapterError::CandleError(err.to_string())
    }
}

/// Metadata del adapter QLoRA.
#[derive(Debug, Clone)]
pub struct AdapterInfo {
    /// Identificador único del adapter.
    pub adapter_id: String,
    /// Modelo base al que aplica (SHA256).
    pub base_model_sha256: String,
    /// Rank del adapter (r << d_model).
    pub rank: usize,
    /// Dimensión del modelo base (d_model).
    pub d_model: usize,
    /// Dimensión SAE (d_sae, si aplica).
    pub d_sae: Option<usize>,
    /// Número de capas afectadas.
    pub layers_count: usize,
    /// Tipo de cuantización (INT8, FP8, FP16, FP32).
    pub quantization: QuantizationType,
}

/// Tipo de cuantización para las matrices A y B.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationType {
    /// INT8 — 8-bit integer (lazy dequantization).
    Int8,
    /// FP8 — 8-bit float (experimental).
    Fp8,
    /// FP16 — 16-bit float (standard).
    Fp16,
    /// FP32 — 32-bit float (full precision).
    Fp32,
}

impl fmt::Display for QuantizationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantizationType::Int8 => write!(f, "INT8"),
            QuantizationType::Fp8 => write!(f, "FP8"),
            QuantizationType::Fp16 => write!(f, "FP16"),
            QuantizationType::Fp32 => write!(f, "FP32"),
        }
    }
}

impl QuantizationType {
    /// Retorna el DType de Candle correspondiente.
    pub fn to_dtype(&self) -> DType {
        match self {
            QuantizationType::Int8 => DType::U8, // INT8 stored as U8 for candle compatibility
            QuantizationType::Fp8 => DType::F16, // FP8 not natively supported, use FP16
            QuantizationType::Fp16 => DType::F16,
            QuantizationType::Fp32 => DType::F32,
        }
    }
}

/// Adapter QLoRA — Matrices A y B para fine-tuning de bajo rank.
///
/// **Fórmula:** W' = W + B @ A
///
/// Donde A: (r x d_model) y B: (d_model x r).
/// El adapter se aplica como: output = x @ W + x @ A @ B
/// Lo cual es equivalente a: output = x @ (W + A @ B)
/// Pero la forma canónica QLoRA es W' = W + B @ A para compatibilidad.
pub struct QloraAdapter {
    /// Metadata del adapter.
    pub info: AdapterInfo,
    /// Matriz A (proyección): shape (d_model x r)
    /// En QLoRA estándar: A proyecta d_model -> r
    pub matrix_a: Tensor,
    /// Matriz B (reconstrucción): shape (r x d_model)
    /// En QLoRA estándar: B proyecta r -> d_model
    /// La combinación B @ A produce (r x d_model) @ (d_model x r) = (r x r)
    /// Pero la forma correcta es alpha * (B @ A) donde alpha es escala
    pub matrix_b: Tensor,
    /// Escala del adapter (alpha parameter).
    /// El contribution final es: (alpha / rank) * B @ A
    pub alpha: f64,
    /// Dispositivo donde residen los tensores.
    pub device: Device,
}

impl fmt::Debug for QloraAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QloraAdapter")
            .field("info", &self.info)
            .field("matrix_a_shape", &self.matrix_a.shape())
            .field("matrix_b_shape", &self.matrix_b.shape())
            .field("alpha", &self.alpha)
            .field("device", &self.device)
            .finish()
    }
}

impl QloraAdapter {
    /// Crea un nuevo QLoRA Adapter desde tensores existentes.
    ///
    /// # Arguments
    /// * `info` - Metadata del adapter.
    /// * `matrix_a` - Matriz A, shape (d_model x r).
    /// * `matrix_b` - Matriz B, shape (r x d_model).
    /// * `alpha` - Escala del adapter.
    ///
    /// # Errors
    /// Retorna `QloraAdapterError::DimensionMismatch` si las dimensiones
    /// no son compatibles con la fórmula W' = W + B @ A.
    pub fn new(
        info: AdapterInfo,
        matrix_a: Tensor,
        matrix_b: Tensor,
        alpha: f64,
    ) -> Result<Self, QloraAdapterError> {
        let device = matrix_a.device().clone();

        // Validate dimensions: A should be (d_model x r), B should be (r x d_model)
        let a_shape = matrix_a.shape();
        let b_shape = matrix_b.shape();

        if a_shape.rank() != 2 || b_shape.rank() != 2 {
            return Err(QloraAdapterError::DimensionMismatch(
                "Matrices A and B must be 2D".into(),
            ));
        }

        let (d_model_a, rank_a) = (a_shape.dims()[0], a_shape.dims()[1]);
        let (rank_b, d_model_b) = (b_shape.dims()[0], b_shape.dims()[1]);

        if rank_a != rank_b {
            return Err(QloraAdapterError::DimensionMismatch(format!(
                "Rank mismatch: A has rank {}, B has rank {}",
                rank_a, rank_b
            )));
        }

        if d_model_a != d_model_b {
            return Err(QloraAdapterError::DimensionMismatch(format!(
                "d_model mismatch: A has {}, B has {}",
                d_model_a, d_model_b
            )));
        }

        if d_model_a != info.d_model {
            return Err(QloraAdapterError::DimensionMismatch(format!(
                "d_model mismatch: info says {}, A has {}",
                info.d_model, d_model_a
            )));
        }

        if rank_a != info.rank {
            return Err(QloraAdapterError::DimensionMismatch(format!(
                "rank mismatch: info says {}, A has {}",
                info.rank, rank_a
            )));
        }

        Ok(QloraAdapter {
            info,
            matrix_a,
            matrix_b,
            alpha,
            device,
        })
    }

    /// Crea un QLoRA Adapter desde bytes serializados (bincode).
    ///
    /// # Arguments
    /// * `data` - Bytes serializados con bincode.
    /// * `device` - Dispositivo donde cargar los tensores.
    pub fn from_bytes(data: &[u8], _device: &Device) -> Result<Self, QloraAdapterError> {
        // TODO(Sprint16.2): Implement full bincode deserialization.
        // For now, validate basic structure.
        if data.len() < 64 {
            return Err(QloraAdapterError::CorruptedData(
                "Data too small for valid adapter".into(),
            ));
        }

        // Parse header: first 32 bytes = adapter_id, next 32 bytes = sha256
        let adapter_id = String::from_utf8_lossy(&data[..32]).to_string();
        let _base_sha256 = String::from_utf8_lossy(&data[32..64]).to_string();

        Err(QloraAdapterError::SerializationError(format!(
            "Deserialization not yet fully implemented (adapter_id={})",
            adapter_id
        )))
    }

    /// Serializa el adapter a bytes (bincode).
    ///
    /// Retorna los bytes listos para distribución P2P.
    pub fn to_bytes(&self) -> Result<Vec<u8>, QloraAdapterError> {
        // TODO(Sprint16.2): Implement full bincode serialization.
        // For now, create a basic representation.
        let mut data = Vec::new();

        // Header: adapter_id (32 bytes)
        let adapter_id_bytes = self.info.adapter_id.as_bytes();
        data.extend_from_slice(&adapter_id_bytes[..adapter_id_bytes.len().min(32)]);

        // SHA256 (32 bytes)
        let sha256_bytes = self.info.base_model_sha256.as_bytes();
        data.extend_from_slice(&sha256_bytes[..sha256_bytes.len().min(32)]);

        // Metadata
        data.extend_from_slice(&self.info.rank.to_le_bytes());
        data.extend_from_slice(&self.info.d_model.to_le_bytes());
        data.extend_from_slice(&self.alpha.to_le_bytes());

        // Tensor data (raw bytes)
        let a_data = self.matrix_a.to_vec1::<f32>().map_err(|e| {
            QloraAdapterError::SerializationError(format!("Failed to extract A: {}", e))
        })?;
        data.extend_from_slice(bytemuck::cast_slice(&a_data));

        let b_data = self.matrix_b.to_vec1::<f32>().map_err(|e| {
            QloraAdapterError::SerializationError(format!("Failed to extract B: {}", e))
        })?;
        data.extend_from_slice(bytemuck::cast_slice(&b_data));

        Ok(data)
    }

    /// Valida que el adapter es consistente.
    ///
    /// Verifica:
    /// - Rank > 0 y rank << d_model
    /// - Dimensiones de A y B son compatibles
    /// - Alpha en rango válido [0.0, 1.0]
    pub fn validate(&self) -> Result<(), QloraAdapterError> {
        if self.info.rank == 0 {
            return Err(QloraAdapterError::InvalidRank("Rank must be > 0".into()));
        }

        if self.info.rank >= self.info.d_model {
            return Err(QloraAdapterError::InvalidRank(format!(
                "Rank {} should be << d_model {}",
                self.info.rank, self.info.d_model
            )));
        }

        if !(0.0..=1.0).contains(&self.alpha) {
            return Err(QloraAdapterError::InvalidRank(format!(
                "Alpha {} must be in [0.0, 1.0]",
                self.alpha
            )));
        }

        Ok(())
    }

    /// Aplica el adapter QLoRA a un input tensor.
    ///
    /// **Fórmula:** output = x + (alpha / rank) * (x @ A) @ B
    ///
    /// Donde:
    /// - x: input tensor (batch_size x d_model)
    /// - A: (d_model x r) — proyección
    /// - B: (r x d_model) — reconstrucción
    ///
    /// Esto es equivalente a aplicar el delta W_delta = B @ A
    /// tal que W' = W + W_delta.
    ///
    /// # Arguments
    /// * `x` - Input tensor, shape (batch_size x d_model)
    ///
    /// # Returns
    /// Output tensor con el adapter aplicado: (batch_size x d_model)
    pub fn apply(&self, x: &Tensor) -> CandleResult<Tensor> {
        let scale = self.alpha / self.info.rank as f64;

        // Forward pass: x @ A @ B
        // x: (batch x d_model)
        // A: (d_model x r)
        // (x @ A): (batch x r)
        // B: (r x d_model)
        // (x @ A @ B): (batch x d_model)
        let projected = x.matmul(&self.matrix_a)?; // (batch x r)
        let reconstructed = projected.matmul(&self.matrix_b)?; // (batch x d_model)

        // Scale by alpha/rank using broadcast_mul
        let scale_tensor = Tensor::new(scale as f32, &self.device)?;
        let delta = reconstructed.broadcast_mul(&scale_tensor)?;

        // Add to original: x + delta
        x.add(&delta)
    }

    /// Calcula el delta W' = (alpha / rank) * B @ A.
    ///
    /// Este es el delta que se suma a los pesos base W:
    /// W' = W + delta
    ///
    /// # Returns
    /// Delta tensor, shape (d_model x d_model)
    pub fn compute_delta(&self) -> CandleResult<Tensor> {
        let scale = self.alpha / self.info.rank as f64;

        // A @ B: (d_model x r) @ (r x d_model) = (d_model x d_model)
        // This is the delta that gets added to W: W' = W + delta
        let delta = self
            .matrix_a
            .matmul(&self.matrix_b)
            .map_err(|e| candle_core::Error::Msg(format!("Failed to compute A @ B: {}", e)))?;

        // Scale by alpha/rank
        let scale_tensor = Tensor::new(scale as f32, &self.device)?;
        delta.broadcast_mul(&scale_tensor)
    }

    /// Crea un adapter de prueba con tensores aleatorios.
    #[cfg(test)]
    pub fn mock(d_model: usize, rank: usize, alpha: f64) -> Self {
        let device = Device::Cpu;
        let info = AdapterInfo {
            adapter_id: format!("mock-{}-{}", d_model, rank),
            base_model_sha256: "0".repeat(64),
            rank,
            d_model,
            d_sae: None,
            layers_count: 1,
            quantization: QuantizationType::Fp32,
        };

        // Create random-ish matrices using ones for deterministic testing
        let matrix_a = Tensor::ones((d_model, rank), DType::F32, &device).unwrap();
        let matrix_b = Tensor::ones((rank, d_model), DType::F32, &device).unwrap();

        QloraAdapter {
            info,
            matrix_a,
            matrix_b,
            alpha,
            device,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = QloraAdapterError::InvalidRank("test".into());
        assert!(!format!("{}", err).is_empty());

        let err = QloraAdapterError::DimensionMismatch("test".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_adapter_info_creation() {
        let info = AdapterInfo {
            adapter_id: "test-adapter".into(),
            base_model_sha256: "abc123".into(),
            rank: 8,
            d_model: 4096,
            d_sae: Some(2048),
            layers_count: 32,
            quantization: QuantizationType::Fp16,
        };
        assert_eq!(info.rank, 8);
        assert_eq!(info.d_model, 4096);
        assert_eq!(info.quantization, QuantizationType::Fp16);
    }

    #[test]
    fn test_quantization_display() {
        assert_eq!(format!("{}", QuantizationType::Int8), "INT8");
        assert_eq!(format!("{}", QuantizationType::Fp8), "FP8");
        assert_eq!(format!("{}", QuantizationType::Fp16), "FP16");
        assert_eq!(format!("{}", QuantizationType::Fp32), "FP32");
    }

    #[test]
    fn test_quantization_to_dtype() {
        assert_eq!(QuantizationType::Int8.to_dtype(), DType::U8);
        assert_eq!(QuantizationType::Fp16.to_dtype(), DType::F16);
        assert_eq!(QuantizationType::Fp32.to_dtype(), DType::F32);
    }

    #[test]
    fn test_mock_adapter_creation() {
        let adapter = QloraAdapter::mock(4096, 8, 0.5);
        assert_eq!(adapter.info.rank, 8);
        assert_eq!(adapter.info.d_model, 4096);
        assert_eq!(adapter.alpha, 0.5);
    }

    #[test]
    fn test_mock_adapter_validate() {
        let adapter = QloraAdapter::mock(4096, 8, 0.5);
        assert!(adapter.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_rank() {
        // Candle allows zero-dim tensors, but validate() should reject rank=0
        let device = Device::Cpu;
        let info = AdapterInfo {
            adapter_id: "test".into(),
            base_model_sha256: "0".repeat(64),
            rank: 0,
            d_model: 4096,
            d_sae: None,
            layers_count: 1,
            quantization: QuantizationType::Fp32,
        };
        let matrix_a = Tensor::zeros((4096, 0), DType::F32, &device).unwrap();
        let matrix_b = Tensor::zeros((0, 4096), DType::F32, &device).unwrap();

        let adapter = QloraAdapter::new(info, matrix_a, matrix_b, 0.5).unwrap();
        match adapter.validate() {
            Err(QloraAdapterError::InvalidRank(_)) => {} // Expected
            other => panic!("Expected InvalidRank, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_alpha_out_of_range() {
        let adapter = QloraAdapter::mock(4096, 8, 1.5);
        match adapter.validate() {
            Err(QloraAdapterError::InvalidRank(_)) => {} // Expected
            other => panic!("Expected InvalidRank, got {:?}", other),
        }
    }

    #[test]
    fn test_dimension_mismatch_a_b() {
        let device = Device::Cpu;
        let info = AdapterInfo {
            adapter_id: "test".into(),
            base_model_sha256: "0".repeat(64),
            rank: 8,
            d_model: 4096,
            d_sae: None,
            layers_count: 1,
            quantization: QuantizationType::Fp32,
        };
        // A: (4096 x 8), B: (16 x 4096) — rank mismatch
        let matrix_a = Tensor::ones((4096, 8), DType::F32, &device).unwrap();
        let matrix_b = Tensor::ones((16, 4096), DType::F32, &device).unwrap();

        match QloraAdapter::new(info, matrix_a, matrix_b, 0.5) {
            Err(QloraAdapterError::DimensionMismatch(_)) => {} // Expected
            other => panic!("Expected DimensionMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_apply_forward_pass() {
        let adapter = QloraAdapter::mock(128, 8, 1.0);
        // x: (1 x 128)
        let x = Tensor::ones((1, 128), DType::F32, &adapter.device).unwrap();

        let output = adapter.apply(&x).expect("apply failed");
        assert_eq!(output.shape().dims(), &[1, 128]);
    }

    #[test]
    fn test_compute_delta() {
        let adapter = QloraAdapter::mock(128, 8, 1.0);
        let delta = adapter.compute_delta().expect("compute_delta failed");
        // Delta should be (d_model x d_model) = (128 x 128)
        assert_eq!(delta.shape().dims(), &[128, 128]);
    }

    #[test]
    fn test_w_prime_formula() {
        // Verify W' = W + A @ B (per candle convention)
        // W' = W + (alpha / rank) * A @ B
        let d_model = 64;
        let rank = 4;
        let alpha = 1.0;

        let adapter = QloraAdapter::mock(d_model, rank, alpha);

        // Compute delta = (alpha / rank) * A @ B
        let delta = adapter.compute_delta().expect("delta");

        // With ones matrices and alpha=1.0, rank=4:
        // A @ B = (64 x 4) @ (4 x 64) = (64 x 64) where each element = 4
        // delta = (1.0 / 4) * 4 = 1.0 for each element
        let delta_vec: Vec<Vec<f32>> = delta.to_vec2::<f32>().expect("to_vec2");
        for row in delta_vec {
            for val in row {
                assert!((val - 1.0).abs() < 1e-5, "Expected ~1.0, got {}", val);
            }
        }
    }

    #[test]
    fn test_from_bytes_too_small() {
        let device = Device::Cpu;
        let data = vec![0u8; 32]; // Too small
        match QloraAdapter::from_bytes(&data, &device) {
            Err(QloraAdapterError::CorruptedData(_)) => {} // Expected
            other => panic!("Expected CorruptedData, got {:?}", other),
        }
    }
}
