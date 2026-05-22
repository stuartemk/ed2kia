//! Inference Bridge Module — SAE forward pass and result return for MVP core loop.
//!
//! Feature-gated behind `v2.1-mvp-core`. Provides the bridge between distributed
//! tensor tasks and SAE (Sparse Autoencoder) inference execution.
//!
//! When compiled for wasm32 with `v2.1-wasm-telemetry`, provides
//! `#[wasm_bindgen]` annotated `emit_inference_complete` that dispatches
//! a `CustomEvent` to the browser DOM for real-time JS consumption.
//!
//! **Status:** Scaffold — mock data for validation.
//! **License:** Apache 2.0 + Ethical Use Clause

use thiserror::Error;

// ─── WASM Telemetry Bridge (feature-gated) ───
#[cfg(all(target_arch = "wasm32", feature = "v2.1-wasm-telemetry"))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", feature = "v2.1-wasm-telemetry"))]
#[wasm_bindgen]
/// Emit inference_complete event to browser DOM.
///
/// Dispatches a `CustomEvent` named `inference_complete` with detail:
/// `{ task_id, activations, confidence, processing_time_ms }`
pub fn emit_inference_complete(
    task_id: &str,
    activations: &[f32],
    confidence: f64,
    processing_time_ms: u64,
) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;

    // Build detail object.
    let mut detail_obj = js_sys::Object::new();
    js_sys::Reflect::set_str(&detail_obj, "task_id", task_id)?;
    js_sys::Reflect::set(
        &detail_obj,
        "activations".into(),
        &JsValue::from(activations),
    )?;
    js_sys::Reflect::set(&detail_obj, "confidence".into(), &JsValue::from(confidence))?;
    js_sys::Reflect::set(
        &detail_obj,
        "processing_time_ms".into(),
        &JsValue::from(processing_time_ms),
    )?;

    // Create CustomEvent with detail.
    let event_init = web_sys::CustomEventInit::new();
    event_init.detail(&detail_obj);
    let event = web_sys::CustomEvent::new_with_event_init_dict("inference_complete", &event_init)?;

    // Dispatch on document.
    document.dispatch_event(&event)?;
    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "v2.1-wasm-telemetry"))]
#[wasm_bindgen]
/// Emit task_received event to browser DOM.
///
/// Dispatches a `CustomEvent` named `task_received` with detail:
/// `{ task_id, timestamp_ms }`
pub fn emit_task_received(task_id: &str, timestamp_ms: u64) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;

    let mut detail_obj = js_sys::Object::new();
    js_sys::Reflect::set_str(&detail_obj, "task_id", task_id)?;
    js_sys::Reflect::set(
        &detail_obj,
        "timestamp_ms".into(),
        &JsValue::from(timestamp_ms),
    )?;

    let event_init = web_sys::CustomEventInit::new();
    event_init.detail(&detail_obj);
    let event = web_sys::CustomEvent::new_with_event_init_dict("task_received", &event_init)?;

    document.dispatch_event(&event)?;
    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "v2.1-wasm-telemetry"))]
#[wasm_bindgen]
/// Emit peer_connected event to browser DOM.
///
/// Dispatches a `CustomEvent` named `peer_connected` with detail:
/// `{ peer_id, timestamp_ms }`
pub fn emit_peer_connected(peer_id: &str, timestamp_ms: u64) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;

    let mut detail_obj = js_sys::Object::new();
    js_sys::Reflect::set_str(&detail_obj, "peer_id", peer_id)?;
    js_sys::Reflect::set(
        &detail_obj,
        "timestamp_ms".into(),
        &JsValue::from(timestamp_ms),
    )?;

    let event_init = web_sys::CustomEventInit::new();
    event_init.detail(&detail_obj);
    let event = web_sys::CustomEvent::new_with_event_init_dict("peer_connected", &event_init)?;

    document.dispatch_event(&event)?;
    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "v2.1-wasm-telemetry"))]
#[wasm_bindgen]
/// Emit error event to browser DOM.
///
/// Dispatches a `CustomEvent` named `error` with detail:
/// `{ message, source, timestamp_ms }`
pub fn emit_error(message: &str, source: &str, timestamp_ms: u64) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;

    let mut detail_obj = js_sys::Object::new();
    js_sys::Reflect::set_str(&detail_obj, "message", message)?;
    js_sys::Reflect::set_str(&detail_obj, "source", source)?;
    js_sys::Reflect::set(
        &detail_obj,
        "timestamp_ms".into(),
        &JsValue::from(timestamp_ms),
    )?;

    let event_init = web_sys::CustomEventInit::new();
    event_init.detail(&detail_obj);
    let event = web_sys::CustomEvent::new_with_event_init_dict("wasm_error", &event_init)?;

    document.dispatch_event(&event)?;
    Ok(())
}

/// Errors specific to inference bridge operations.
#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("SAE forward pass failed: {0}")]
    SaeForward(String),

    #[error("Result return failed: {0}")]
    ResultReturn(String),

    #[error("No tasks available for inference")]
    NoTasks,

    #[error("Inference timeout: {0}")]
    Timeout(String),
}

/// Represents the result of an SAE inference operation.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Source task ID that generated this result.
    pub task_id: String,
    /// Inference output activations.
    pub activations: Vec<f32>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
}

impl InferenceResult {
    /// Create a new inference result.
    pub fn new(
        task_id: String,
        activations: Vec<f32>,
        confidence: f64,
        processing_time_ms: u64,
    ) -> Self {
        Self {
            task_id,
            activations,
            confidence,
            processing_time_ms,
        }
    }
}

/// MVP Inference Bridge for SAE forward passes and result management.
pub struct InferenceBridge {
    /// Completed inference results.
    results: Vec<InferenceResult>,
    /// Bridge status.
    ready: bool,
}

impl InferenceBridge {
    /// Create a new inference bridge.
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            ready: true,
        }
    }

    /// Check if bridge is ready.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Run SAE forward pass on distributed tasks.
    ///
    /// In production this will:
    /// 1. Load SAE model weights
    /// 2. Execute forward pass on tensor partitions
    /// 3. Return inference results with activations
    pub async fn run_sae_forward(
        &mut self,
        task_ids: &[String],
    ) -> Result<Vec<InferenceResult>, InferenceError> {
        if task_ids.is_empty() {
            return Err(InferenceError::NoTasks);
        }

        // Scaffold: simulate SAE forward pass with mock results
        let mut batch_results = Vec::new();

        for task_id in task_ids {
            let result = InferenceResult::new(
                task_id.clone(),
                vec![0.85, 0.92, 0.78, 0.95], // Mock activations
                0.93,                         // Mock confidence
                42,                           // Mock processing time
            );
            batch_results.push(result);
        }

        self.results.extend(batch_results.clone());
        Ok(batch_results)
    }

    /// Return an inference result to the orchestrator.
    ///
    /// In production this will:
    /// 1. Serialize result payload
    /// 2. Send back via libp2p request-response
    /// 3. Acknowledge successful delivery
    pub async fn return_result(&mut self, result: InferenceResult) -> Result<(), InferenceError> {
        // Scaffold: store result for validation
        self.results.push(result);
        Ok(())
    }

    /// Get the current result count.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Get all collected results.
    pub fn get_results(&self) -> &[InferenceResult] {
        &self.results
    }

    /// Execute a full audit task: Deserialize payload → SAE forward → Serialize result.
    ///
    /// This is the core WASM flow for P2P audit:
    /// 1. Deserialize `AuditTaskPayload` (bincode)
    /// 2. Reconstruct QwenScopeSAE from shard weights
    /// 3. Execute `forward()` on input activation
    /// 4. Serialize `AuditResultPayload` (bincode)
    ///
    /// Feature-gated behind `v2.1-audit-payloads` + `v2.1-qwen-scope-sae`.
    #[cfg(all(feature = "v2.1-audit-payloads", feature = "v2.1-qwen-scope-sae"))]
    pub async fn execute_audit_task(
        &mut self,
        payload_bytes: &[u8],
        node_id: String,
    ) -> Result<Vec<u8>, InferenceError> {
        use crate::protocol::audit_payloads::{
            AuditPayloadError, AuditResultPayload, AuditTaskPayload,
        };
        use crate::sae::qwen_scope_sae::QwenScopeSAE;
        use candle_core::Device;
        use std::time::Instant;

        // Step 1: Deserialize AuditTaskPayload
        let task = AuditTaskPayload::deserialize(payload_bytes).map_err(|e| {
            InferenceError::SaeForward(format!("Payload deserialization failed: {}", e))
        })?;

        // Validate payload
        task.validate()
            .map_err(|e| InferenceError::SaeForward(format!("Payload validation failed: {}", e)))?;

        let start = Instant::now();

        // Step 2: Reconstruct QwenScopeSAE from shard weights
        let device = Device::Cpu;
        let (d_sae, d_model) = task.shard_shape;
        let k = task.k;

        // Extract weights from flat shard: w_enc + w_dec + b_enc + b_dec
        let w_enc_size = d_sae * d_model;
        let w_dec_size = d_model * d_sae;
        let b_enc_size = d_sae;
        // b_dec_size = d_model

        let w_enc_data = &task.shard_weights[0..w_enc_size];
        let w_dec_data = &task.shard_weights[w_enc_size..w_enc_size + w_dec_size];
        let b_enc_data =
            &task.shard_weights[w_enc_size + w_dec_size..w_enc_size + w_dec_size + b_enc_size];
        let b_dec_data = &task.shard_weights[w_enc_size + w_dec_size + b_enc_size..];

        // Create tensors from weight data
        let w_enc =
            Tensor::from_vec(w_enc_data.to_vec(), (d_sae, d_model), &device).map_err(|e| {
                InferenceError::SaeForward(format!("w_enc tensor creation failed: {}", e))
            })?;
        let w_dec =
            Tensor::from_vec(w_dec_data.to_vec(), (d_model, d_sae), &device).map_err(|e| {
                InferenceError::SaeForward(format!("w_dec tensor creation failed: {}", e))
            })?;
        let b_enc = Tensor::from_vec(b_enc_data.to_vec(), d_sae, &device).map_err(|e| {
            InferenceError::SaeForward(format!("b_enc tensor creation failed: {}", e))
        })?;
        let b_dec = Tensor::from_vec(b_dec_data.to_vec(), d_model, &device).map_err(|e| {
            InferenceError::SaeForward(format!("b_dec tensor creation failed: {}", e))
        })?;

        // Create SAE model
        let sae = QwenScopeSAE::new(w_enc, w_dec, b_enc, b_dec, k)
            .map_err(|e| InferenceError::SaeForward(format!("SAE construction failed: {}", e)))?;

        // Step 3: Execute forward pass
        let input_tensor = Tensor::from_vec(
            task.input_activation.clone(),
            (task.batch_size, d_model),
            &device,
        )
        .map_err(|e| InferenceError::SaeForward(format!("Input tensor creation failed: {}", e)))?;

        let (sparse_values, sparse_indices) = sae
            .forward(&input_tensor)
            .map_err(|e| InferenceError::SaeForward(format!("SAE forward failed: {}", e)))?;

        let compute_time_ms = start.elapsed().as_millis() as u64;

        // Extract results to vectors
        let values_vec: Vec<f32> = sparse_values
            .to_vec1()
            .map_err(|e| InferenceError::SaeForward(format!("Values extraction failed: {}", e)))?;
        let indices_vec: Vec<usize> = sparse_indices
            .to_vec1()
            .map_err(|e| InferenceError::SaeForward(format!("Indices extraction failed: {}", e)))?;

        // Step 4: Create and serialize AuditResultPayload
        let result = AuditResultPayload::new(
            task.task_id,
            values_vec,
            indices_vec,
            compute_time_ms,
            node_id,
        );

        result.serialize().map_err(|e| {
            InferenceError::ResultReturn(format!("Result serialization failed: {}", e))
        })
    }

    /// Execute audit task with error result (for error propagation).
    #[cfg(feature = "v2.1-audit-payloads")]
    pub fn create_error_result(
        task_id: uuid::Uuid,
        node_id: String,
        error_message: String,
    ) -> Result<Vec<u8>, InferenceError> {
        #[cfg(feature = "v2.1-audit-payloads")]
        {
            use crate::protocol::audit_payloads::AuditResultPayload;
            let result = AuditResultPayload::error(task_id, node_id, error_message);
            result.serialize().map_err(|e| {
                InferenceError::ResultReturn(format!("Error result serialization failed: {}", e))
            })
        }
        #[cfg(not(feature = "v2.1-audit-payloads"))]
        {
            Err(InferenceError::SaeForward(
                "audit-payloads feature not enabled".to_string(),
            ))
        }
    }
}

impl Default for InferenceBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_new() {
        let bridge = InferenceBridge::new();
        assert!(bridge.is_ready());
        assert_eq!(bridge.result_count(), 0);
    }

    #[tokio::test]
    async fn test_run_sae_forward() {
        let mut bridge = InferenceBridge::new();
        let task_ids = vec![
            "task-001".to_string(),
            "task-002".to_string(),
            "task-003".to_string(),
        ];
        let results = bridge.run_sae_forward(&task_ids).await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(bridge.result_count(), 3);
    }

    #[tokio::test]
    async fn test_run_sae_forward_empty() {
        let mut bridge = InferenceBridge::new();
        let result = bridge.run_sae_forward(&[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_return_result() {
        let mut bridge = InferenceBridge::new();
        let result = InferenceResult::new("task-999".to_string(), vec![0.5, 0.6, 0.7], 0.88, 30);
        bridge.return_result(result).await.unwrap();
        assert_eq!(bridge.result_count(), 1);
    }

    #[tokio::test]
    async fn test_get_results() {
        let mut bridge = InferenceBridge::new();
        let task_ids = vec!["task-001".to_string()];
        bridge.run_sae_forward(&task_ids).await.unwrap();
        let results = bridge.get_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "task-001");
    }

    #[test]
    fn test_inference_result_new() {
        let result = InferenceResult::new("task-001".to_string(), vec![1.0, 2.0], 0.95, 50);
        assert_eq!(result.task_id, "task-001");
        assert_eq!(result.activations.len(), 2);
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.processing_time_ms, 50);
    }

    #[test]
    fn test_error_display() {
        let err = InferenceError::SaeForward("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_bridge_default() {
        let bridge = InferenceBridge::default();
        assert!(bridge.is_ready());
    }
}
