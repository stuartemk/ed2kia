//! Tensor Adapter - Cross-model tensor compatibility for ed2kIA Fase 6
//!
//! Provides `TensorAdapter` for normalizing hidden states between different
//! LLM architectures (Llama-3, Mistral, ONNX) into the Qwen-Scope schema.
//!
//! Uses `candle_core::Tensor` for vectorized operations supporting f16, bf16, f32.
//!
//! # Feature Flag
//!
//! This module is gated behind `#[cfg(feature = "phase6-core")]`.

#[cfg(feature = "phase6-core")]
use candle_core::{Device, DType, Tensor};
use serde::{Deserialize, Serialize};
use tracing::info;

// ---------------------------------------------------------------------------
// Public types (always available for serialization)
// ---------------------------------------------------------------------------

/// Supported source model architectures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SourceModel {
    Llama,
    Mistral,
    Qwen,
    GPT2,
    #[serde(untagged)]
    Custom(String),
}

impl std::fmt::Display for SourceModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceModel::Llama => write!(f, "llama"),
            SourceModel::Mistral => write!(f, "mistral"),
            SourceModel::Qwen => write!(f, "qwen"),
            SourceModel::GPT2 => write!(f, "gpt2"),
            SourceModel::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Error produced by the TensorAdapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterError {
    pub source: String,
    pub expected_shape: Vec<usize>,
    pub got: Vec<usize>,
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdapterError: {} (expected {:?}, got {:?})",
            self.source, self.expected_shape, self.got
        )
    }
}

impl std::error::Error for AdapterError {}

/// Normalized hidden state in Qwen-Scope format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedHiddenState {
    pub source_model: SourceModel,
    pub source_layer: u32,
    pub original_dim: usize,
    pub normalized_dim: usize,
    pub data: Vec<f32>,
    pub norm_applied: String,
    pub timestamp: u64,
}

impl NormalizedHiddenState {
    pub fn new(source_model: SourceModel, source_layer: u32, data: Vec<f32>) -> Self {
        let dim = data.len();
        Self {
            source_model,
            source_layer,
            original_dim: dim,
            normalized_dim: dim,
            data,
            norm_applied: "none".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: extract Vec<f32> from Tensor (any rank -> flattened)
// ---------------------------------------------------------------------------

#[cfg(feature = "phase6-core")]
fn tensor_to_vec_f32(tensor: &Tensor) -> std::result::Result<Vec<f32>, AdapterError> {
    // Flatten to 1-D then extract
    let rank = tensor.rank();
    let flat = if rank > 1 {
        tensor.flatten(0, rank - 1).map_err(|e| AdapterError {
            source: format!("flatten for extraction: {}", e),
            expected_shape: vec![],
            got: tensor.shape().dims().to_vec(),
        })?
    } else {
        tensor.clone()
    };
    flat.to_vec1().map_err(|e| AdapterError {
        source: format!("to_vec1: {}", e),
        expected_shape: vec![],
        got: flat.shape().dims().to_vec(),
    })
}

#[cfg(feature = "phase6-core")]
fn tensor_to_vec_f64(tensor: &Tensor) -> std::result::Result<Vec<f64>, anyhow::Error> {
    let rank = tensor.rank();
    let flat = if rank > 1 {
        tensor.flatten(0, rank - 1).map_err(|e| anyhow::anyhow!("flatten: {}", e))?
    } else {
        tensor.clone()
    };
    flat.to_vec1().map_err(|e| anyhow::anyhow!("to_vec1: {}", e))
}

#[cfg(feature = "phase6-core")]
fn shape_dims(shape: &candle_core::Shape) -> Vec<usize> {
    shape.dims().to_vec()
}

// ---------------------------------------------------------------------------
// TensorAdapter – core implementation (feature gated)
// ---------------------------------------------------------------------------

/// Cross-model tensor adapter that normalizes hidden states into Qwen-Scope schema.
#[cfg(feature = "phase6-core")]
pub struct TensorAdapter {
    target_dim: usize,
    target_dtype: DType,
}

#[cfg(feature = "phase6-core")]
impl TensorAdapter {
    pub fn new(target_dim: usize, target_dtype: DType) -> Self {
        Self {
            target_dim,
            target_dtype,
        }
    }

    /// Qwen2-7B defaults (hidden_size=3584, f32)
    pub fn qwen2_7b() -> Self {
        Self::new(3584, DType::F32)
    }

    // ------------------------------------------------------------------
    // normalize_dtype
    // ------------------------------------------------------------------

    /// Cast tensor to the target dtype (f16, bf16, f32).
    pub fn normalize_dtype(&self, tensor: &Tensor) -> std::result::Result<Tensor, AdapterError> {
        let current_dtype = tensor.dtype();
        if current_dtype == self.target_dtype {
            return Ok(tensor.clone());
        }

        let current_shape = shape_dims(tensor.shape());
        let casted = tensor
            .to_dtype(self.target_dtype)
            .map_err(|e| AdapterError {
                source: format!("normalize_dtype cast failed: {}", e),
                expected_shape: vec![self.target_dim],
                got: current_shape,
            })?;

        Ok(casted)
    }

    // ------------------------------------------------------------------
    // reshape_to_qwen
    // ------------------------------------------------------------------

    /// Project tensor dimensionality to Qwen-Scope target.
    pub fn reshape_to_qwen(&self, tensor: &Tensor) -> std::result::Result<Tensor, AdapterError> {
        let dims = shape_dims(tensor.shape());
        let src_dim = *dims.last().unwrap_or(&dims.len());

        if src_dim == self.target_dim {
            return Ok(tensor.clone());
        }

        let rank = tensor.rank();
        let flat = if rank > 1 {
            tensor.flatten(0, rank - 1).map_err(|e| AdapterError {
                source: format!("reshape_to_qwen flatten: {}", e),
                expected_shape: vec![self.target_dim],
                got: dims.clone(),
            })?
        } else {
            tensor.clone()
        };

        let batch = if rank > 1 {
            flat.dim(0).map_err(|e| AdapterError {
                source: format!("reshape_to_qwen dim: {}", e),
                expected_shape: vec![self.target_dim],
                got: dims.clone(),
            })?
        } else {
            1
        };

        if src_dim > self.target_dim {
            self._shrink(&flat, src_dim, batch)
        } else {
            self._expand(&flat, src_dim, batch)
        }
    }

    fn _shrink(&self, flat: &Tensor, src_dim: usize, batch: usize) -> std::result::Result<Tensor, AdapterError> {
        let data: Vec<f32> = tensor_to_vec_f32(flat)?;

        let ratio = src_dim as f64 / self.target_dim as f64;
        let mut projected = vec![0.0f32; self.target_dim];

        for (i, val) in projected.iter_mut().enumerate().take(self.target_dim) {
            let start = (i as f64 * ratio) as usize;
            let end = ((i + 1) as f64 * ratio).min(src_dim as f64) as usize;
            let sum: f64 = data[start..end].iter().map(|x| *x as f64).sum();
            *val = (sum / (end - start) as f64) as f32;
        }

        Tensor::from_vec(projected, (batch, self.target_dim), &Device::Cpu).map_err(|e| AdapterError {
            source: format!("shrink tensor build: {}", e),
            expected_shape: vec![batch, self.target_dim],
            got: vec![batch, src_dim],
        })
    }

    fn _expand(&self, flat: &Tensor, src_dim: usize, batch: usize) -> std::result::Result<Tensor, AdapterError> {
        let data: Vec<f32> = tensor_to_vec_f32(flat)?;

        let mut expanded = data;
        expanded.resize(self.target_dim, 0.0);

        Tensor::from_vec(expanded, (batch, self.target_dim), &Device::Cpu).map_err(|e| AdapterError {
            source: format!("expand tensor build: {}", e),
            expected_shape: vec![batch, self.target_dim],
            got: vec![batch, src_dim],
        })
    }

    // ------------------------------------------------------------------
    // apply_padding
    // ------------------------------------------------------------------

    /// Apply zero-padding (or truncation) to reach the target shape.
    pub fn apply_padding(&self, tensor: &Tensor, target_shape: &[usize]) -> std::result::Result<Tensor, AdapterError> {
        let current_shape = shape_dims(tensor.shape());
        if current_shape == target_shape {
            return Ok(tensor.clone());
        }

        let rank = tensor.rank();
        if rank == 0 {
            return Err(AdapterError {
                source: "apply_padding: scalar tensor".to_string(),
                expected_shape: target_shape.to_vec(),
                got: current_shape,
            });
        }

        let target_last = target_shape.last().copied().unwrap_or(0);
        let current_last = current_shape.last().copied().unwrap_or(0);

        if current_last == target_last {
            return tensor.reshape(target_shape).map_err(|e| AdapterError {
                source: format!("apply_padding reshape: {}", e),
                expected_shape: target_shape.to_vec(),
                got: current_shape,
            });
        }

        if current_last > target_last {
            return tensor.narrow(rank - 1, 0, target_last).map_err(|e| AdapterError {
                source: format!("apply_padding narrow: {}", e),
                expected_shape: target_shape.to_vec(),
                got: current_shape,
            });
        }

        // Pad: extract vec, resize, rebuild
        let data: Vec<f32> = tensor_to_vec_f32(tensor)?;

        let total_elements: usize = target_shape.iter().product();
        let mut padded = data;
        padded.resize(total_elements, 0.0);

        Tensor::from_vec(padded, target_shape, &Device::Cpu).map_err(|e| AdapterError {
            source: format!("apply_padding tensor build: {}", e),
            expected_shape: target_shape.to_vec(),
            got: current_shape,
        })
    }

    // ------------------------------------------------------------------
    // validate_schema
    // ------------------------------------------------------------------

    /// Validate that the tensor matches the expected schema (shape + dtype).
    pub fn validate_schema(&self, tensor: &Tensor, expected_shape: &[usize]) -> std::result::Result<(), AdapterError> {
        let current_shape = shape_dims(tensor.shape());
        let current_dtype = tensor.dtype();

        if current_shape != expected_shape {
            return Err(AdapterError {
                source: "validate_schema: shape mismatch".to_string(),
                expected_shape: expected_shape.to_vec(),
                got: current_shape,
            });
        }

        if current_dtype != self.target_dtype {
            return Err(AdapterError {
                source: format!(
                    "validate_schema: dtype mismatch (expected {:?}, got {:?})",
                    self.target_dtype, current_dtype
                ),
                expected_shape: expected_shape.to_vec(),
                got: current_shape,
            });
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Full adaptation pipeline
    // ------------------------------------------------------------------

    /// Run the full adaptation pipeline.
    pub fn adapt(&self, tensor: &Tensor, source_model: SourceModel) -> std::result::Result<NormalizedHiddenState, AdapterError> {
        info!("Adapting {} tensor shape={:?} dtype={:?}",
            source_model, shape_dims(tensor.shape()), tensor.dtype());

        let normalized = self.normalize_dtype(tensor)?;
        let reshaped = self.reshape_to_qwen(&normalized)?;

        let batch_dim = reshaped.dim(0).unwrap_or(1);
        let expected_shape = vec![batch_dim, self.target_dim];
        self.validate_schema(&reshaped, &expected_shape)?;

        let data: Vec<f32> = tensor_to_vec_f32(&reshaped)?;

        Ok(NormalizedHiddenState {
            source_model,
            source_layer: 0,
            original_dim: shape_dims(tensor.shape()).last().copied().unwrap_or(0),
            normalized_dim: self.target_dim,
            data,
            norm_applied: "dtype_cast+projection".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_model_display() {
        assert_eq!(SourceModel::Llama.to_string(), "llama");
        assert_eq!(SourceModel::Mistral.to_string(), "mistral");
        assert_eq!(SourceModel::Qwen.to_string(), "qwen");
        assert_eq!(SourceModel::GPT2.to_string(), "gpt2");
        assert_eq!(SourceModel::Custom("test".into()).to_string(), "custom:test");
    }

    #[test]
    fn test_adapter_error_display() {
        let err = AdapterError {
            source: "test error".to_string(),
            expected_shape: vec![1, 100],
            got: vec![1, 50],
        };
        let msg = err.to_string();
        assert!(msg.contains("test error"));
        assert!(msg.contains("expected"));
    }

    #[test]
    fn test_normalized_hidden_state() {
        let state = NormalizedHiddenState::new(SourceModel::Llama, 5, vec![1.0, 2.0, 3.0]);
        assert_eq!(state.source_model, SourceModel::Llama);
        assert_eq!(state.source_layer, 5);
        assert_eq!(state.original_dim, 3);
        assert_eq!(state.normalized_dim, 3);
        assert_eq!(state.norm_applied, "none");
    }

    #[cfg(feature = "phase6-core")]
    mod phase6_tests {
        use super::*;

        fn make_tensor(data: Vec<f32>, shape: &[usize]) -> Tensor {
            Tensor::from_vec(data, shape, &Device::Cpu).unwrap()
        }

        #[test]
        fn test_normalize_dtype_f32_to_f32() {
            let adapter = TensorAdapter::qwen2_7b();
            let t = make_tensor(vec![1.0, 2.0, 3.0], &[3]);
            let result = adapter.normalize_dtype(&t).unwrap();
            assert_eq!(result.dtype(), DType::F32);
        }

        #[test]
        fn test_normalize_dtype_f16_to_f32() {
            let adapter = TensorAdapter::qwen2_7b();
            let t = Tensor::from_vec(vec![1.0f64, 2.0, 3.0], &[3], &Device::Cpu)
                .unwrap().to_dtype(DType::F16).unwrap();
            let result = adapter.normalize_dtype(&t).unwrap();
            assert_eq!(result.dtype(), DType::F32);
        }

        #[test]
        fn test_reshape_to_qwen_same_dim() {
            let adapter = TensorAdapter::new(3, DType::F32);
            let t = make_tensor(vec![1.0, 2.0, 3.0], &[3]);
            let result = adapter.reshape_to_qwen(&t).unwrap();
            let dims: Vec<usize> = result.shape().dims().iter().copied().collect();
            assert_eq!(dims, vec![3]);
        }

        #[test]
        fn test_reshape_to_qwen_expand() {
            let adapter = TensorAdapter::new(6, DType::F32);
            let t = make_tensor(vec![1.0, 2.0, 3.0], &[3]);
            let result = adapter.reshape_to_qwen(&t).unwrap();
            let dims: Vec<usize> = result.shape().dims().iter().copied().collect();
            assert_eq!(dims[1], 6);
        }

        #[test]
        fn test_reshape_to_qwen_shrink() {
            let adapter = TensorAdapter::new(2, DType::F32);
            let t = make_tensor(vec![1.0, 2.0, 3.0, 4.0], &[4]);
            let result = adapter.reshape_to_qwen(&t).unwrap();
            let dims: Vec<usize> = result.shape().dims().iter().copied().collect();
            assert_eq!(dims[1], 2);
        }

        #[test]
        fn test_apply_padding_no_change() {
            let adapter = TensorAdapter::qwen2_7b();
            let t = make_tensor(vec![1.0, 2.0, 3.0], &[3]);
            let result = adapter.apply_padding(&t, &[3]).unwrap();
            let dims: Vec<usize> = result.shape().dims().iter().copied().collect();
            assert_eq!(dims, vec![3]);
        }

        #[test]
        fn test_apply_padding_expand() {
            let adapter = TensorAdapter::qwen2_7b();
            let t = make_tensor(vec![1.0, 2.0], &[2]);
            let result = adapter.apply_padding(&t, &[5]).unwrap();
            let dims: Vec<usize> = result.shape().dims().iter().copied().collect();
            assert_eq!(dims, vec![5]);
        }

        #[test]
        fn test_validate_schema_ok() {
            let adapter = TensorAdapter::new(3, DType::F32);
            let t = make_tensor(vec![1.0, 2.0, 3.0], &[3]);
            assert!(adapter.validate_schema(&t, &[3]).is_ok());
        }

        #[test]
        fn test_validate_schema_shape_mismatch() {
            let adapter = TensorAdapter::new(5, DType::F32);
            let t = make_tensor(vec![1.0, 2.0, 3.0], &[3]);
            let err = adapter.validate_schema(&t, &[5]).unwrap_err();
            assert!(err.source.contains("shape mismatch"));
        }

        #[test]
        fn test_full_adapt_pipeline() {
            let adapter = TensorAdapter::new(4, DType::F32);
            let t = make_tensor(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[6]);
            let result = adapter.adapt(&t, SourceModel::Llama).unwrap();
            assert_eq!(result.normalized_dim, 4);
            assert_eq!(result.source_model, SourceModel::Llama);
        }
    }
}
