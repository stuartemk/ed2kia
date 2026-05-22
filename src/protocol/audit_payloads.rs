//! Audit Payloads — AuditTaskPayload → AuditResultPayload for WASM-friendly serialization.
//!
//! Feature-gated behind `v2.1-audit-payloads`. Provides serializable payloads
//! for audit task distribution and result collection across P2P peers.
//!
//! **Status:** Functional scaffold with serialization + unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

#[cfg(feature = "v2.1-qwen-scope-sae")]
use crate::sae::qwen_scope_sae::QwenScopeSAE;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors specific to audit payload operations.
#[derive(Debug, Error)]
pub enum AuditPayloadError {
    #[error("Serialization failed: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[cfg(feature = "v2.1-qwen-scope-sae")]
    #[error("SAE operation failed: {0}")]
    SaeError(#[from] crate::sae::qwen_scope_sae::QwenScopeError),
}

/// Audit Task Payload — Distributed to peers for SAE inference.
///
/// Contains the SAE shard, input activation, and inference parameters
/// needed to execute a sparse autoencoder forward pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTaskPayload {
    /// Unique task identifier
    pub task_id: Uuid,
    /// SAE shard weights (flattened f32: [w_enc, w_dec, b_enc, b_dec])
    pub shard_weights: Vec<f32>,
    /// Shard shape metadata: (d_sae, d_model)
    pub shard_shape: (usize, usize),
    /// Input activation vector [batch_size * d_model]
    pub input_activation: Vec<f32>,
    /// Input batch size
    pub batch_size: usize,
    /// Top-k sparsity parameter
    pub k: usize,
    /// Sparsity threshold (0.0 - 1.0)
    pub sparsity_threshold: f32,
}

/// Audit Result Payload — Returned from peers after SAE inference.
///
/// Contains sparse activations, indices, and metadata for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResultPayload {
    /// Original task identifier
    pub task_id: Uuid,
    /// Sparse activation values [batch_size * k]
    pub sparse_values: Vec<f32>,
    /// Sparse activation indices [batch_size * k]
    pub sparse_indices: Vec<usize>,
    /// Compute time in milliseconds
    pub compute_time_ms: u64,
    /// Source node identifier
    pub node_id: String,
    /// Optional error message (if partial failure)
    pub error: Option<String>,
}

impl AuditTaskPayload {
    /// Create a new audit task payload.
    pub fn new(
        shard_weights: Vec<f32>,
        shard_shape: (usize, usize),
        input_activation: Vec<f32>,
        batch_size: usize,
        k: usize,
        sparsity_threshold: f32,
    ) -> Self {
        Self {
            task_id: Uuid::new_v4(),
            shard_weights,
            shard_shape,
            input_activation,
            batch_size,
            k,
            sparsity_threshold,
        }
    }

    /// Create a payload with a specific task ID.
    pub fn with_task_id(mut self, task_id: Uuid) -> Self {
        self.task_id = task_id;
        self
    }

    /// Validate payload integrity.
    pub fn validate(&self) -> Result<(), AuditPayloadError> {
        let (d_sae, d_model) = self.shard_shape;

        if d_sae == 0 || d_model == 0 {
            return Err(AuditPayloadError::InvalidPayload(
                "shard_shape must have non-zero dimensions".to_string(),
            ));
        }

        if self.k == 0 || self.k >= d_sae {
            return Err(AuditPayloadError::InvalidPayload(format!(
                "k must be in range 1..{}",
                d_sae
            )));
        }

        if self.input_activation.is_empty() {
            return Err(AuditPayloadError::InvalidPayload(
                "input_activation must not be empty".to_string(),
            ));
        }

        let expected_input_len = self.batch_size * d_model;
        if self.input_activation.len() != expected_input_len {
            return Err(AuditPayloadError::InvalidPayload(format!(
                "input_activation length {} doesn't match batch_size * d_model = {}",
                self.input_activation.len(),
                expected_input_len
            )));
        }

        // Validate shard weights size: w_enc + w_dec + b_enc + b_dec
        let expected_weights = d_sae * d_model + d_model * d_sae + d_sae + d_model;
        if self.shard_weights.len() != expected_weights {
            return Err(AuditPayloadError::InvalidPayload(format!(
                "shard_weights length {} doesn't match expected {}",
                self.shard_weights.len(),
                expected_weights
            )));
        }

        Ok(())
    }

    /// Serialize payload to bytes using bincode.
    pub fn serialize(&self) -> Result<Vec<u8>, AuditPayloadError> {
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize payload from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, AuditPayloadError> {
        Ok(bincode::deserialize(data)?)
    }

    /// Estimate memory footprint in bytes.
    pub fn estimate_size_bytes(&self) -> usize {
        self.shard_weights.len() * 4
            + self.input_activation.len() * 4
            + std::mem::size_of::<AuditTaskPayload>()
    }
}

impl AuditResultPayload {
    /// Create a new audit result payload.
    pub fn new(
        task_id: Uuid,
        sparse_values: Vec<f32>,
        sparse_indices: Vec<usize>,
        compute_time_ms: u64,
        node_id: String,
    ) -> Self {
        Self {
            task_id,
            sparse_values,
            sparse_indices,
            compute_time_ms,
            node_id,
            error: None,
        }
    }

    /// Create an error result payload.
    pub fn error(task_id: Uuid, node_id: String, error_msg: String) -> Self {
        Self {
            task_id,
            sparse_values: Vec::new(),
            sparse_indices: Vec::new(),
            compute_time_ms: 0,
            node_id,
            error: Some(error_msg),
        }
    }

    /// Check if this result represents an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Serialize result to bytes using bincode.
    pub fn serialize(&self) -> Result<Vec<u8>, AuditPayloadError> {
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize result from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, AuditPayloadError> {
        Ok(bincode::deserialize(data)?)
    }

    /// Estimate memory footprint in bytes.
    pub fn estimate_size_bytes(&self) -> usize {
        self.sparse_values.len() * 4
            + self.sparse_indices.len() * 8
            + std::mem::size_of::<AuditResultPayload>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_task() -> AuditTaskPayload {
        let d_sae = 128;
        let d_model = 32;
        let k = 8;
        let batch_size = 2;

        // Create shard weights: w_enc + w_dec + b_enc + b_dec
        let mut weights = Vec::new();
        weights.extend(vec![1.0f32; d_sae * d_model]); // w_enc
        weights.extend(vec![1.0f32; d_model * d_sae]); // w_dec
        weights.extend(vec![0.0f32; d_sae]); // b_enc
        weights.extend(vec![0.0f32; d_model]); // b_dec

        // Create input activation
        let input = vec![0.5f32; batch_size * d_model];

        AuditTaskPayload::new(weights, (d_sae, d_model), input, batch_size, k, 0.9)
    }

    #[test]
    fn test_task_payload_creation() {
        let task = create_valid_task();
        assert_eq!(task.shard_shape, (128, 32));
        assert_eq!(task.k, 8);
        assert_eq!(task.batch_size, 2);
    }

    #[test]
    fn test_task_payload_validate_valid() {
        let task = create_valid_task();
        assert!(task.validate().is_ok());
    }

    #[test]
    fn test_task_payload_validate_empty_input() {
        let mut task = create_valid_task();
        task.input_activation.clear();
        assert!(task.validate().is_err());
    }

    #[test]
    fn test_task_payload_validate_wrong_input_length() {
        let mut task = create_valid_task();
        task.input_activation = vec![0.0; 10]; // Wrong length
        assert!(task.validate().is_err());
    }

    #[test]
    fn test_task_payload_validate_k_too_large() {
        let mut task = create_valid_task();
        task.k = 256; // > d_sae
        assert!(task.validate().is_err());
    }

    #[test]
    fn test_task_payload_validate_zero_shape() {
        let mut task = create_valid_task();
        task.shard_shape = (0, 32);
        assert!(task.validate().is_err());
    }

    #[test]
    fn test_task_payload_with_task_id() {
        let task = create_valid_task();
        let custom_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let task = task.with_task_id(custom_id);
        assert_eq!(task.task_id, custom_id);
    }

    #[test]
    fn test_task_payload_serialize_deserialize() {
        let task = create_valid_task();
        let bytes = task.serialize().unwrap();
        let deserialized = AuditTaskPayload::deserialize(&bytes).unwrap();
        assert_eq!(deserialized.task_id, task.task_id);
        assert_eq!(deserialized.shard_shape, task.shard_shape);
        assert_eq!(deserialized.k, task.k);
        assert_eq!(deserialized.shard_weights.len(), task.shard_weights.len());
    }

    #[test]
    fn test_task_payload_estimate_size() {
        let task = create_valid_task();
        let size = task.estimate_size_bytes();
        assert!(size > 0);
    }

    #[test]
    fn test_result_payload_creation() {
        let task_id = Uuid::new_v4();
        let result = AuditResultPayload::new(
            task_id,
            vec![0.8, 0.9, 0.7],
            vec![10, 20, 30],
            42,
            "node-1".to_string(),
        );
        assert_eq!(result.task_id, task_id);
        assert_eq!(result.compute_time_ms, 42);
        assert_eq!(result.node_id, "node-1");
        assert!(!result.is_error());
    }

    #[test]
    fn test_result_payload_error() {
        let task_id = Uuid::new_v4();
        let result = AuditResultPayload::error(
            task_id,
            "node-1".to_string(),
            "SAE forward failed".to_string(),
        );
        assert!(result.is_error());
        assert_eq!(result.error.as_deref(), Some("SAE forward failed"));
    }

    #[test]
    fn test_result_payload_serialize_deserialize() {
        let task_id = Uuid::new_v4();
        let result = AuditResultPayload::new(
            task_id,
            vec![0.8, 0.9, 0.7],
            vec![10, 20, 30],
            42,
            "node-1".to_string(),
        );
        let bytes = result.serialize().unwrap();
        let deserialized = AuditResultPayload::deserialize(&bytes).unwrap();
        assert_eq!(deserialized.task_id, result.task_id);
        assert_eq!(deserialized.sparse_values, result.sparse_values);
        assert_eq!(deserialized.sparse_indices, result.sparse_indices);
    }

    #[test]
    fn test_result_payload_estimate_size() {
        let result = AuditResultPayload::new(
            Uuid::new_v4(),
            vec![0.0; 100],
            vec![0; 100],
            10,
            "node".to_string(),
        );
        let size = result.estimate_size_bytes();
        assert!(size > 0);
    }

    #[test]
    fn test_error_display() {
        let err = AuditPayloadError::InvalidPayload("test".to_string());
        assert!(!format!("{}", err).is_empty());
    }
}
