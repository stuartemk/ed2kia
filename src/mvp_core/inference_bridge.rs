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
    let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;

    // Build detail object.
    let mut detail_obj = js_sys::Object::new();
    js_sys::Reflect::set_str(&detail_obj, "task_id", task_id)?;
    js_sys::Reflect::set(&detail_obj, "activations".into(), &JsValue::from(activations))?;
    js_sys::Reflect::set(&detail_obj, "confidence".into(), &JsValue::from(confidence))?;
    js_sys::Reflect::set(&detail_obj, "processing_time_ms".into(), &JsValue::from(processing_time_ms))?;

    // Create CustomEvent.
    let event = web_sys::CustomEvent::new("inference_complete")?;
    js_sys::Reflect::set(&event, "detail".into(), &detail_obj)?;

    // Dispatch on document.
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
    pub fn new(task_id: String, activations: Vec<f32>, confidence: f64, processing_time_ms: u64) -> Self {
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
    pub async fn run_sae_forward(&mut self, task_ids: &[String]) -> Result<Vec<InferenceResult>, InferenceError> {
        if task_ids.is_empty() {
            return Err(InferenceError::NoTasks);
        }

        // Scaffold: simulate SAE forward pass with mock results
        let mut batch_results = Vec::new();

        for task_id in task_ids {
            let result = InferenceResult::new(
                task_id.clone(),
                vec![0.85, 0.92, 0.78, 0.95], // Mock activations
                0.93,                          // Mock confidence
                42,                            // Mock processing time
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
        let task_ids = vec!["task-001".to_string(), "task-002".to_string(), "task-003".to_string()];
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
        let result = InferenceResult::new(
            "task-999".to_string(),
            vec![0.5, 0.6, 0.7],
            0.88,
            30,
        );
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
