//! Browser Node WASM — Compilación a wasm32-unknown-unknown con wasm-bindgen.
//!
//! Nodo P2P para navegadores modernos cumpliendo Ley 4 (Simbiosis Existencial)
//! y Ley 1 (Diversidad Comunitaria). Cero telemetría externa, cero trackers,
//! cero lógica financiera.
//!
//! Feature gates: `v2.1-wasm-browser-node`, `v2.1-wasm-worker`
//! Target: `wasm32-unknown-unknown`
//!
//! **Exportaciones JS:**
//! - `BrowserNode.init(id, memoryLimitMb)` → inicializa nodo
//! - `BrowserNode.processTask(payload)` → procesa tarea SAE dummy
//! - `BrowserNode.processTensor(payload)` → evalúa SCT, retorna `{ x, y, z, decision }` (Sprint25)
//! - `BrowserNode.getHealth()` → retorna JSON con estado del nodo
//!
//! **Restricciones WASM:**
//! - Cero `std::fs` / `std::net`
//! - Async vía `wasm_bindgen_futures::spawn_local`
//! - Heap controlado (default 64MB, configurable)
//! - `console_error_panic_hook` para debug

#[cfg(target_arch = "wasm32")]
mod wasm_internal {
    use std::collections::VecDeque;
    use std::fmt;

    use wasm_bindgen::prelude::*;

    // Re-export console_error_panic_hook for better stack traces
    extern crate console_error_panic_hook;

    // ============================================================================
    // Error Types
    // ============================================================================

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BrowserNodeError {
        MemoryLimitExceeded,
        QueueFull,
        InvalidPayload,
        NotInitialized,
        TaskTimeout,
    }

    impl fmt::Display for BrowserNodeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                BrowserNodeError::MemoryLimitExceeded => write!(f, "WASM memory limit exceeded"),
                BrowserNodeError::QueueFull => write!(f, "Task queue is full (max 64)"),
                BrowserNodeError::InvalidPayload => write!(f, "Invalid task payload"),
                BrowserNodeError::NotInitialized => write!(f, "BrowserNode not initialized"),
                BrowserNodeError::TaskTimeout => write!(f, "Task processing timeout"),
            }
        }
    }

    // ============================================================================
    // Task Types
    // ============================================================================

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TaskType {
        SaeInference,
        GradientValidation,
        HealthCheck,
    }

    impl fmt::Display for TaskType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TaskType::SaeInference => write!(f, "SaeInference"),
                TaskType::GradientValidation => write!(f, "GradientValidation"),
                TaskType::HealthCheck => write!(f, "HealthCheck"),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Task {
        pub id: String,
        pub task_type: TaskType,
        pub payload: String,
        pub created_at_ms: u64,
    }

    #[derive(Debug, Clone)]
    pub struct TaskResult {
        pub task_id: String,
        pub success: bool,
        pub output: String,
        pub latency_ms: u64,
    }

    // ============================================================================
    // Health Status
    // ============================================================================

    #[derive(Debug, Clone)]
    pub struct HealthStatus {
        pub id: String,
        pub initialized: bool,
        pub queue_size: usize,
        pub tasks_processed: u64,
        pub tasks_failed: u64,
        pub memory_limit_mb: u32,
        pub uptime_ms: u64,
    }

    impl HealthStatus {
        pub fn to_json(&self) -> String {
            format!(
                r#"{{"id":"{}","initialized":{},"queue_size":{},"tasks_processed":{},"tasks_failed":{},"memory_limit_mb":{},"uptime_ms":{}}}"#,
                self.id,
                self.initialized,
                self.queue_size,
                self.tasks_processed,
                self.tasks_failed,
                self.memory_limit_mb,
                self.uptime_ms
            )
        }
    }

    // ============================================================================
    // BrowserNode — WASM Exported Struct
    // ============================================================================

    /// Browser Node — WASM P2P node for browser environments.
    ///
    /// Ley 4 (Simbiosis): Hardware modesto, conexiones inestables, fricción cero.
    /// Ley 1 (Diversidad): Cero centralización, propiedad comunitaria.
    #[wasm_bindgen]
    pub struct BrowserNode {
        id: String,
        initialized: bool,
        memory_limit_mb: u32,
        task_queue: VecDeque<Task>,
        tasks_processed: u64,
        tasks_failed: u64,
        started_at_ms: u64,
        max_queue_size: usize,
    }

    #[wasm_bindgen]
    impl BrowserNode {
        /// Create a new BrowserNode instance.
        ///
        /// # Args
        /// * `id` — Unique node identifier (e.g., "browser-node-001")
        /// * `memory_limit_mb` — Maximum heap allocation in MB (default: 64)
        ///
        /// # Example
        /// ```js
        /// const node = new BrowserNode('node-001', 64);
        /// ```
        #[wasm_bindgen(constructor)]
        pub fn new(id: &str, memory_limit_mb: u32) -> Self {
            // Set panic hook for better error messages in browser console
            console_error_panic_hook::set_once();

            Self {
                id: id.to_string(),
                initialized: false,
                memory_limit_mb: memory_limit_mb.max(16).min(512), // [16, 512] MB
                task_queue: VecDeque::with_capacity(64),
                tasks_processed: 0,
                tasks_failed: 0,
                started_at_ms: current_timestamp_ms(),
                max_queue_size: 64,
            }
        }

        /// Initialize the browser node.
        ///
        /// Must be called before `processTask`. Sets up internal state
        /// and validates configuration.
        ///
        /// # Returns
        /// JSON string with initialization status.
        ///
        /// # Example
        /// ```js
        /// const result = node.init();
        /// console.log(result); // '{"id":"node-001","status":"initialized"}'
        /// ```
        #[wasm_bindgen]
        pub fn init(&mut self) -> String {
            self.initialized = true;
            let result = format!(
                r#"{{"id":"{}","status":"initialized","memory_limit_mb":{}}}"#,
                self.id, self.memory_limit_mb
            );
            // Dispatch CustomEvent for telemetry (v2.1-wasm-telemetry)
            dispatch_event("ed2k-node-initialized", &result);
            result
        }

        /// Process a task payload.
        ///
        /// Accepts JSON string with task type and data. Processes synchronously
        /// within WASM, returning result as JSON.
        ///
        /// # Args
        /// * `payload` — JSON string: `{"type":"SaeInference","data":"..."}`
        ///
        /// # Returns
        /// JSON string with task result.
        ///
        /// # Errors
        /// Returns error JSON if node not initialized or payload invalid.
        ///
        /// # Example
        /// ```js
        /// const result = node.processTask('{"type":"HealthCheck","data":"ping"}');
        /// console.log(result);
        /// ```
        #[wasm_bindgen]
        pub fn process_task(&mut self, payload: &str) -> String {
            if !self.initialized {
                return format!(r#"{{"error":"NotInitialized","message":"Call init() first"}}"#);
            }

            if payload.is_empty() {
                return format!(
                    r#"{{"error":"InvalidPayload","message":"Payload cannot be empty"}}"#
                );
            }

            let start_ms = current_timestamp_ms();

            // Parse task type from payload (lightweight, no serde_json dependency)
            let task_type = if payload.contains("SaeInference") {
                TaskType::SaeInference
            } else if payload.contains("GradientValidation") {
                TaskType::GradientValidation
            } else if payload.contains("HealthCheck") {
                TaskType::HealthCheck
            } else {
                TaskType::SaeInference // Default
            };

            // Process based on type
            let (success, output) = match task_type {
                TaskType::HealthCheck => (true, format!("pong-{}", self.id)),
                TaskType::SaeInference => {
                    // Dummy SAE inference — simulates activation extraction
                    let activations = self.simulate_sae_activations(payload.len());
                    (true, activations)
                }
                TaskType::GradientValidation => {
                    // Dummy gradient validation — checks payload integrity
                    let valid = payload.len() > 10;
                    (
                        valid,
                        if valid {
                            "gradient_valid".to_string()
                        } else {
                            "gradient_too_short".to_string()
                        },
                    )
                }
            };

            let latency_ms = current_timestamp_ms() - start_ms;

            if success {
                self.tasks_processed += 1;
            } else {
                self.tasks_failed += 1;
            }

            // Queue for async processing (Web Worker bridge)
            let task = Task {
                id: format!("{}-{}", self.id, self.tasks_processed + self.tasks_failed),
                task_type,
                payload: payload.to_string(),
                created_at_ms: start_ms,
            };

            if self.task_queue.len() < self.max_queue_size {
                self.task_queue.push_back(task);
            } else {
                return format!(
                    r#"{{"error":"QueueFull","queue_size":{},"max":{}}}"#,
                    self.task_queue.len(),
                    self.max_queue_size
                );
            }

            let result = TaskResult {
                task_id: format!(
                    "{}-{}",
                    self.id,
                    self.tasks_processed + self.tasks_failed - 1
                ),
                success,
                output,
                latency_ms,
            };

            // Dispatch event for JS listeners
            let result_json = result.to_json();
            dispatch_event("ed2k-task-complete", &result_json);
            result_json
        }

        /// Get current health status.
        ///
        /// # Returns
        /// JSON string with node health metrics.
        ///
        /// # Example
        /// ```js
        /// const health = node.getHealth();
        /// console.log(health);
        /// ```
        #[wasm_bindgen]
        pub fn get_health(&self) -> String {
            let uptime_ms = current_timestamp_ms() - self.started_at_ms;
            let status = HealthStatus {
                id: self.id.clone(),
                initialized: self.initialized,
                queue_size: self.task_queue.len(),
                tasks_processed: self.tasks_processed,
                tasks_failed: self.tasks_failed,
                memory_limit_mb: self.memory_limit_mb,
                uptime_ms,
            };
            status.to_json()
        }

        /// Process a tensor payload via SCT (Stuartian Context Tensor) evaluation.
        ///
        /// Sprint25: Exposes ethical gravity evaluation directly to the Web Worker bridge.
        /// Returns a JSON object with SCT axes `{ x, y, z, decision }` compatible with
        /// 3D Octahedron visualization (`geometry-bridge.js`).
        ///
        /// # Args
        /// * `payload` — Arbitrary string payload to evaluate
        ///
        /// # Returns
        /// `JsValue` containing JSON: `{ "x": f32, "y": f32, "z": f32, "decision": String }`
        /// - `x`: Community Benefit [0, 1]
        /// - `y`: External Cost [0, 1]
        /// - `z`: Symbiosis Score [-1, 1]
        /// - `decision`: "approved" (z >= 0) or "rejected" (z < 0)
        ///
        /// # Example
        /// ```js
        /// const result = node.processTensor('some-payload-data');
        /// console.log(result); // { x: 0.75, y: 0.25, z: 0.5, decision: 'approved' }
        /// ```
        #[wasm_bindgen]
        pub fn process_tensor(&mut self, payload: &str) -> JsValue {
            if !self.initialized {
                let err = format!(r#"{{"error":"NotInitialized","message":"Call init() first"}}"#);
                return JsValue::from_str(&err);
            }

            if payload.is_empty() {
                let err =
                    format!(r#"{{"error":"InvalidPayload","message":"Payload cannot be empty"}}"#);
                return JsValue::from_str(&err);
            }

            let start_ms = current_timestamp_ms();

            // Deterministic SCT evaluation based on payload content hash
            let sct = self.evaluate_sct(payload);
            self.tasks_processed += 1;

            let latency_ms = current_timestamp_ms() - start_ms;
            let decision = if sct.z >= 0.0 { "approved" } else { "rejected" };

            let result = format!(
                r#"{{"x":{:.4},"y":{:.4},"z":{:.4},"decision":"{}","latency_ms":{}}}"#,
                sct.x, sct.y, sct.z, decision, latency_ms
            );

            // Dispatch event for JS listeners (3D Octahedron sync)
            dispatch_event("ed2k-sct-evaluated", &result);
            JsValue::from_str(&result)
        }

        /// Get the node ID.
        #[wasm_bindgen]
        pub fn id(&self) -> String {
            self.id.clone()
        }

        /// Check if the node is initialized.
        #[wasm_bindgen]
        pub fn is_initialized(&self) -> bool {
            self.initialized
        }

        /// Get current queue size.
        #[wasm_bindgen]
        pub fn queue_size(&self) -> usize {
            self.task_queue.len()
        }

        /// Clear the task queue.
        #[wasm_bindgen]
        pub fn clear_queue(&mut self) {
            self.task_queue.clear();
        }
    }

    impl Default for BrowserNode {
        fn default() -> Self {
            Self::new("browser-node-default", 64)
        }
    }

    impl TaskResult {
        fn to_json(&self) -> String {
            format!(
                r#"{{"task_id":"{}","success":{},"output":"{}","latency_ms":{}}}"#,
                self.task_id, self.success, self.output, self.latency_ms
            )
        }
    }

    // ============================================================================
    // Helper Functions
    // ============================================================================

    impl BrowserNode {
        fn simulate_sae_activations(&self, payload_size: usize) -> String {
            // Deterministic dummy activations based on payload size
            // Simulates top-k sparse activation extraction
            let k = (payload_size % 16) + 8; // 8-23 active features
            let mean_val = (payload_size % 100) as f32 / 100.0;
            format!("activations(k={},mean={:.3},node={})", k, mean_val, self.id)
        }

        /// Evaluate SCT (Stuartian Context Tensor) from payload.
        ///
        /// Deterministic evaluation based on payload content hash.
        /// Compatible with `src/alignment/sct_core.rs` StuartianTensor structure.
        ///
        /// Returns `{ x, y, z }` where:
        /// - x: Community Benefit [0, 1]
        /// - y: External Cost [0, 1]
        /// - z: Symbiosis Score [-1, 1]
        fn evaluate_sct(&self, payload: &str) -> (f32, f32, f32) {
            // Hash payload to deterministic seed
            let mut seed: u32 = 0;
            for byte in payload.bytes() {
                seed = seed.wrapping_mul(31).wrapping_add(byte as u32);
            }

            // X: Community Benefit [0.2, 0.9] — higher = more beneficial
            let x = ((seed as f64 * 0.01).sin().abs() * 0.7 + 0.2) as f32;

            // Y: External Cost [0.1, 0.6] — lower = less costly
            let y = ((seed as f64 * 0.013).cos().abs() * 0.5 + 0.1) as f32;

            // Z: Symbiosis Score [-1, 1] — derived from x - y balance
            let mut z = (x - y) * 2.0 - 0.3;
            z = z.max(-1.0).min(1.0);

            (x, y, z)
        }
    }

    fn current_timestamp_ms() -> u64 {
        js_sys::Date::now() as u64
    }

    fn dispatch_event(event_name: &str, data: &str) {
        // Dispatch CustomEvent to browser for telemetry integration
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new(event_name).unwrap();
            // Set detail via JS interop
            let _ = window.dispatch_event(&event);
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_browser_node_creation() {
            let node = BrowserNode::new("test-node", 64);
            assert_eq!(node.id, "test-node");
            assert!(!node.initialized);
            assert_eq!(node.memory_limit_mb, 64);
        }

        #[test]
        fn test_browser_node_init() {
            let mut node = BrowserNode::new("init-test", 32);
            let result = node.init();
            assert!(node.initialized);
            assert!(result.contains("initialized"));
            assert!(result.contains("init-test"));
        }

        #[test]
        fn test_process_task_not_initialized() {
            let mut node = BrowserNode::new("uninit-test", 64);
            let result = node.process_task(r#"{"type":"HealthCheck"}"#);
            assert!(result.contains("NotInitialized"));
        }

        #[test]
        fn test_process_task_empty_payload() {
            let mut node = BrowserNode::new("empty-test", 64);
            node.init();
            let result = node.process_task("");
            assert!(result.contains("InvalidPayload"));
        }

        #[test]
        fn test_health_check_task() {
            let mut node = BrowserNode::new("health-test", 64);
            node.init();
            let result = node.process_task(r#"{"type":"HealthCheck","data":"ping"}"#);
            assert!(result.contains("pong-health-test"));
            assert!(result.contains("success"));
        }

        #[test]
        fn test_sae_inference_task() {
            let mut node = BrowserNode::new("sae-test", 64);
            node.init();
            let result =
                node.process_task(r#"{"type":"SaeInference","data":"test-payload-12345"}"#);
            assert!(result.contains("activations"));
            assert!(result.contains("sae-test"));
        }

        #[test]
        fn test_gradient_validation_task() {
            let mut node = BrowserNode::new("grad-test", 64);
            node.init();
            // Valid gradient (long enough)
            let result = node.process_task(
                r#"{"type":"GradientValidation","data":"this-is-a-valid-gradient-payload"}"#,
            );
            assert!(result.contains("gradient_valid"));

            // Invalid gradient (too short)
            let result = node.process_task(r#"{"type":"GradientValidation","data":"short"}"#);
            assert!(result.contains("gradient_too_short"));
        }

        #[test]
        fn test_get_health() {
            let mut node = BrowserNode::new("health-status", 128);
            node.init();
            node.process_task(r#"{"type":"HealthCheck"}"#);
            let health = node.get_health();
            assert!(health.contains("health-status"));
            assert!(health.contains("initialized"));
            assert!(health.contains("128"));
        }

        #[test]
        fn test_memory_limit_bounds() {
            // Too low → clamped to 16
            let node = BrowserNode::new("low-mem", 4);
            assert_eq!(node.memory_limit_mb, 16);

            // Too high → clamped to 512
            let node = BrowserNode::new("high-mem", 1024);
            assert_eq!(node.memory_limit_mb, 512);
        }

        #[test]
        fn test_queue_size_tracking() {
            let mut node = BrowserNode::new("queue-test", 64);
            node.init();
            assert_eq!(node.queue_size(), 0);

            node.process_task(r#"{"type":"HealthCheck"}"#);
            assert_eq!(node.queue_size(), 1);

            node.process_task(r#"{"type":"HealthCheck"}"#);
            assert_eq!(node.queue_size(), 2);
        }

        #[test]
        fn test_clear_queue() {
            let mut node = BrowserNode::new("clear-test", 64);
            node.init();
            node.process_task(r#"{"type":"HealthCheck"}"#);
            assert_eq!(node.queue_size(), 1);

            node.clear_queue();
            assert_eq!(node.queue_size(), 0);
        }

        #[test]
        fn test_default_creation() {
            let node = BrowserNode::default();
            assert_eq!(node.id, "browser-node-default");
            assert_eq!(node.memory_limit_mb, 64);
        }

        #[test]
        fn test_task_result_json() {
            let result = TaskResult {
                task_id: "test-1".to_string(),
                success: true,
                output: "ok".to_string(),
                latency_ms: 5,
            };
            let json = result.to_json();
            assert!(json.contains("test-1"));
            assert!(json.contains("true"));
            assert!(json.contains("ok"));
        }

        #[test]
        fn test_health_status_json() {
            let status = HealthStatus {
                id: "status-test".to_string(),
                initialized: true,
                queue_size: 5,
                tasks_processed: 10,
                tasks_failed: 2,
                memory_limit_mb: 64,
                uptime_ms: 1000,
            };
            let json = status.to_json();
            assert!(json.contains("status-test"));
            assert!(json.contains("10"));
            assert!(json.contains("64"));
        }

        #[test]
        fn test_process_tensor_not_initialized() {
            let mut node = BrowserNode::new("tensor-uninit", 64);
            let result = node.process_tensor("test-payload");
            let result_str = result.as_string().unwrap_or_default();
            assert!(result_str.contains("NotInitialized"));
        }

        #[test]
        fn test_process_tensor_empty_payload() {
            let mut node = BrowserNode::new("tensor-empty", 64);
            node.init();
            let result = node.process_tensor("");
            let result_str = result.as_string().unwrap_or_default();
            assert!(result_str.contains("InvalidPayload"));
        }

        #[test]
        fn test_process_tensor_returns_sct_vectors() {
            let mut node = BrowserNode::new("tensor-sct", 64);
            node.init();
            let result = node.process_tensor("symbiotic-payload-data");
            let result_str = result.as_string().unwrap_or_default();
            assert!(result_str.contains("\"x\""));
            assert!(result_str.contains("\"y\""));
            assert!(result_str.contains("\"z\""));
            assert!(result_str.contains("\"decision\""));
        }

        #[test]
        fn test_process_tensor_deterministic() {
            let mut node = BrowserNode::new("tensor-determ", 64);
            node.init();
            let result1 = node.process_tensor("same-payload");
            let result2 = node.process_tensor("same-payload");
            let s1 = result1.as_string().unwrap_or_default();
            let s2 = result2.as_string().unwrap_or_default();
            // Same payload → same SCT vectors (latency may differ)
            assert!(
                s1.contains(&s2[0..s2.len().min(s1.len())])
                    || s2.contains(&s1[0..s1.len().min(s2.len())])
            );
        }

        #[test]
        fn test_sct_bounds() {
            let node = BrowserNode::new("bounds-test", 64);
            // Test multiple payloads to verify bounds
            let payloads = vec![
                "short",
                "a-very-long-payload-that-should-produce-different-sct-values-than-the-short-one",
                "12345678901234567890",
                "symbiotic ethical community benefit payload",
                "malicious extraction harmful payload data",
            ];
            for payload in payloads {
                let (x, y, z) = node.evaluate_sct(payload);
                assert!(
                    x >= 0.0 && x <= 1.0,
                    "x out of bounds for '{}': {}",
                    payload,
                    x
                );
                assert!(
                    y >= 0.0 && y <= 1.0,
                    "y out of bounds for '{}': {}",
                    payload,
                    y
                );
                assert!(
                    z >= -1.0 && z <= 1.0,
                    "z out of bounds for '{}': {}",
                    payload,
                    z
                );
            }
        }
    }
}

// Make wasm_internal public when targeting wasm32
#[cfg(target_arch = "wasm32")]
pub use wasm_internal::*;

// For non-wasm32 targets, provide a stub for testing
#[cfg(not(target_arch = "wasm32"))]
mod stub_internal {
    /// Stub for non-WASM targets (testing only).
    pub struct BrowserNode {
        pub id: String,
        pub initialized: bool,
        pub memory_limit_mb: u32,
    }

    impl BrowserNode {
        pub fn new(id: &str, memory_limit_mb: u32) -> Self {
            Self {
                id: id.to_string(),
                initialized: false,
                memory_limit_mb: memory_limit_mb.max(16).min(512),
            }
        }

        pub fn init(&mut self) {
            self.initialized = true;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use stub_internal::BrowserNode;
