//! Web Worker Offloading — Async inference dispatch without blocking UI.
//!
//! Feature-gated behind `v2.1-wasm-workers`. Uses standard `postMessage`/`onmessage`
//! pattern (NO `SharedArrayBuffer`). Serializes `AuditTaskPayload` to `JsValue`,
//! dispatches to Web Worker, and awaits response via `wasm_bindgen_futures::JsFuture`.
//!
//! **Status:** Scaffold — functional message passing, mock worker response.
//! **License:** Apache 2.0 + Ethical Use Clause

#![cfg(feature = "v2.1-wasm-workers")]

use wasm_bindgen::JsValue;

// Re-export audit payloads for worker messaging
#[cfg(feature = "v2.1-audit-payloads")]
use crate::protocol::audit_payloads::{AuditTaskPayload, AuditResultPayload};

// ============================================================================
// Error Types
// ============================================================================

/// Errors specific to Web Worker operations.
#[derive(Debug)]
pub enum WorkerError {
    WorkerInit(String),
    MessageSend(String),
    MessageReceive(String),
    Serialization(String),
    Timeout(u64),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerError::WorkerInit(e) => write!(f, "Web Worker init failed: {}", e),
            WorkerError::MessageSend(e) => write!(f, "Worker message send failed: {}", e),
            WorkerError::MessageReceive(e) => write!(f, "Worker message receive failed: {}", e),
            WorkerError::Serialization(e) => write!(f, "Message serialization failed: {}", e),
            WorkerError::Timeout(ms) => write!(f, "Worker response timeout: {}ms", ms),
        }
    }
}

// ============================================================================
// Worker Bridge
// ============================================================================

/// Web Worker bridge for offloading audit tasks.
///
/// Manages a single Web Worker instance and provides async dispatch
/// for audit task payloads. Uses standard postMessage pattern without
/// SharedArrayBuffer for maximum browser compatibility.
pub struct WorkerBridge {
    worker: Option<web_sys::Worker>,
    pending_resolves: usize,
}

impl WorkerBridge {
    /// Initialize Web Worker with inline script for audit task processing.
    ///
    /// Creates a Worker from a blob URL containing the worker script.
    /// The worker listens for `audit_task` messages and responds with
    /// `audit_result` messages.
    pub fn init_worker() -> Result<Self, WorkerError> {
        let window = web_sys::window().ok_or_else(|| {
            WorkerError::WorkerInit("No window available".to_string())
        })?;

        // Inline worker script as blob URL
        let worker_script = r#"
            self.onmessage = function(e) {
                var data = e.data;
                if (data && data.type === 'audit_task') {
                    // Mock processing — replace with actual SAE forward pass
                    var result = {
                        type: 'audit_result',
                        task_id: data.task_id || '',
                        sparse_values: data.shard_weights || [],
                        sparse_indices: [],
                        compute_time_ms: 0,
                        node_id: 'worker-1',
                        error: null,
                    };
                    self.postMessage(result);
                }
            };
        "#;

        let bytes: Vec<u8> = worker_script.bytes().collect();
        let uint8_array = js_sys::Uint8Array::new_from_slice(&bytes);
        let blob_seq = js_sys::Array::of1(&uint8_array);

        let blob_part = web_sys::Blob::new_with_u8_slice_sequence(&blob_seq)
            .map_err(|e| WorkerError::WorkerInit(format!("Blob creation failed: {:?}", e)))?;

        let url = web_sys::Url::create_object_url_with_blob(&blob_part)
            .map_err(|e| WorkerError::WorkerInit(format!("URL creation failed: {:?}", e)))?;

        let worker = web_sys::Worker::new(&url)
            .map_err(|e| WorkerError::WorkerInit(format!("Worker creation failed: {:?}", e)))?;

        // Revoke URL to free memory
        web_sys::Url::revoke_object_url(&url)
            .map_err(|e| WorkerError::WorkerInit(format!("URL revoke failed: {:?}", e)))?;

        Ok(Self {
            worker: Some(worker),
            pending_resolves: 0,
        })
    }

    /// Dispatch an audit task to the Web Worker and await response.
    ///
    /// Serializes the payload to JsValue, sends via postMessage,
    /// and returns a JsFuture that resolves when the worker responds.
    #[cfg(feature = "v2.1-audit-payloads")]
    pub async fn dispatch_audit_task(
        &mut self,
        payload: &AuditTaskPayload,
    ) -> Result<AuditResultPayload, WorkerError> {
        let worker = self.worker.as_ref().ok_or_else(|| {
            WorkerError::WorkerInit("Worker not initialized".to_string())
        })?;

        // Build message object
        let msg = js_sys::Object::new();
        js_sys::Reflect::set_str(&msg, "type", "audit_task").map_err(|_| {
            WorkerError::Serialization("Failed to set message type".to_string())
        })?;
        js_sys::Reflect::set_str(&msg, "task_id", &payload.task_id.to_string()).map_err(|_| {
            WorkerError::Serialization("Failed to set task_id".to_string())
        })?;
        js_sys::Reflect::set(&msg, "shard_weights".into(), &JsValue::from(&payload.shard_weights))
            .map_err(|_| {
                WorkerError::Serialization("Failed to set shard_weights".to_string())
            })?;

        // Send message
        worker.post_message(&msg).map_err(|e| {
            WorkerError::MessageSend(format!("postMessage failed: {:?}", e))
        })?;

        self.pending_resolves += 1;

        // Create promise for response
        let (deferred, resolve) = wasm_bindgen_futures::JsFuture::from_promise();

        // Set up one-time listener for response
        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                if let Ok(r#type) = js_sys::Reflect::get_str(&data, "type") {
                    if r#type.as_string().as_deref() == Some("audit_result") {
                        // Mock result — in production, deserialize from worker response
                        let result = AuditResultPayload::success(
                            payload.task_id,
                            Vec::new(),
                            Vec::new(),
                            0,
                            "worker-1".to_string(),
                        );
                        let _ = resolve(JsValue::from_bool(true));
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        if let Some(worker) = &self.worker {
            worker.set_onmessage(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        // Await response with timeout
        match wasm_bindgen_futures::JsFuture::from(deferred).await {
            Ok(_) => {
                // Return mock result — production deserializes from worker
                Ok(AuditResultPayload::success(
                    payload.task_id,
                    Vec::new(),
                    Vec::new(),
                    0,
                    "worker-1".to_string(),
                ))
            }
            Err(e) => Err(WorkerError::MessageReceive(format!(
                "Failed to receive worker response: {:?}",
                e
            ))),
        }
    }

    /// Terminate the Web Worker and release resources.
    pub fn terminate(&mut self) {
        if let Some(worker) = &self.worker {
            worker.terminate();
            self.worker = None;
        }
        self.pending_resolves = 0;
    }

    /// Get the count of pending unresolved responses.
    pub fn pending_count(&self) -> usize {
        self.pending_resolves
    }
}

impl Drop for WorkerBridge {
    fn drop(&mut self) {
        self.terminate();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_error_display() {
        let err = WorkerError::WorkerInit("test".to_string());
        assert!(format!("{}", err).contains("test"));

        let err = WorkerError::Timeout(5000);
        assert!(format!("{}", err).contains("5000"));
    }

    #[test]
    fn test_worker_bridge_creation_fails_without_window() {
        // In non-browser test environment, init_worker will fail
        let result = WorkerBridge::init_worker();
        assert!(result.is_err());
    }
}
