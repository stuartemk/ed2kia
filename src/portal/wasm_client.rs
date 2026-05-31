//! SymbioticPortal — WASM Client for Zero-Friction Onboarding.
//!
//! Provides `SymbioticPortal`, a WASM-native client that launches an OmniNode
//! inside a Web Worker to avoid blocking the UI thread. The portal exposes
//! JavaScript-friendly APIs via `wasm-bindgen` for:
//!
//! - **Worker Lifecycle**: Spawn, communicate with, and terminate the OmniNode worker.
//! - **Message Bridge**: Async JSON-based message passing between UI and worker.
//! - **Health Monitoring**: Periodic heartbeat and status reporting.
//!
//! **Architecture Principles:**
//! - Simbiosis: Portal cooperates with all 4 Evolutionary Pillars via OmniNode.
//! - Cero fricción: Single `init()` call bootstraps the full symbiotic stack.
//! - Evolución: Adaptive worker configuration based on device capabilities.
//! - Armonía: Non-blocking UI through Web Worker isolation.
//!
//! **Feature Gate:** `v3.7-symbiotic-portal`

use wasm_bindgen::prelude::*;

use js_sys::{Function, Promise, Reflect, Uint8Array};
use web_sys::{MessageEvent, Worker};

/// Messages sent from the UI thread to the OmniNode Web Worker.
#[derive(Debug, Clone)]
#[serde_repr]
#[repr(u8)]
pub enum PortalMessage {
    /// Initialize the OmniNode with the given configuration JSON.
    Init = 0,
    /// Send a biometric sample to the Resonance Pipeline.
    BiometricSample = 1,
    /// Request current CE (Compute Energy) balance.
    QueryCeBalance = 2,
    /// Request current GEI (Geometric Ethical Invariant) state.
    QueryGeiState = 3,
    /// Request current Resonance status (SCT Z, brainwave band, etc.).
    QueryResonanceStatus = 4,
    /// Deposit CE credits into the portal ledger.
    DepositCe = 5,
    /// Calibrate the biometric baseline.
    CalibrateBaseline = 6,
    /// Shutdown the OmniNode gracefully.
    Shutdown = 7,
    /// Custom JSON payload for extensibility.
    Custom = 255,
}

impl PortalMessage {
    /// Serialize this message to a JS `Object` for `postMessage()`.
    pub fn to_js_value(&self, payload: Option<&str>) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();
        Reflect::set(
            &obj,
            &JsValue::from_str("type"),
            &JsValue::from_u32(*self as u32),
        )?;
        if let Some(p) = payload {
            Reflect::set(&obj, &JsValue::from_str("payload"), &JsValue::from_str(p))?;
        }
        Ok(obj.into())
    }
}

/// Responses received from the OmniNode Web Worker.
#[derive(Debug, Clone)]
#[serde_repr]
#[repr(u8)]
pub enum PortalResponse {
    /// Initialization successful.
    Ready = 0,
    /// Biometric sample processed; contains resonance response JSON.
    ResonanceResult = 1,
    /// CE balance query result.
    CeBalance = 2,
    /// GEI state query result.
    GeiState = 3,
    /// Resonance status query result.
    ResonanceStatus = 4,
    /// CE deposit confirmed.
    CeDeposited = 5,
    /// Baseline calibration complete.
    Calibrated = 6,
    /// Shutdown complete.
    Stopped = 7,
    /// Error from the worker.
    Error = 255,
}

/// Health status of the SymbioticPortal connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalHealth {
    /// Worker not yet started.
    Idle,
    /// Worker is initializing.
    Starting,
    /// Worker is running and responsive.
    Healthy,
    /// Worker is unresponsive (heartbeat timeout).
    Degraded,
    /// Worker has crashed or been terminated.
    Failed,
}

impl std::fmt::Display for PortalHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalHealth::Idle => write!(f, "idle"),
            PortalHealth::Starting => write!(f, "starting"),
            PortalHealth::Healthy => write!(f, "healthy"),
            PortalHealth::Degraded => write!(f, "degraded"),
            PortalHealth::Failed => write!(f, "failed"),
        }
    }
}

/// Callback type for portal events, exposed to JavaScript.
/// Signature: `(eventType: string, data: string) => void`
type PortalCallback = Function;

/// SymbioticPortal — Zero-friction onboarding client for browser environments.
///
/// Manages a Web Worker instance running the OmniNode WASM module, providing
/// async message passing for biometric processing, CE management, and resonance
/// synthesis without blocking the UI thread.
#[wasm_bindgen]
pub struct SymbioticPortal {
    /// Internal state stored in a Rust struct, accessed via `wasm_bindgen`.
    inner: JsValue, // Holds Box<PortalInner> via JsCast
}

/// Internal portal state (not directly exposed to JS).
struct PortalInner {
    /// The Web Worker instance running OmniNode.
    worker: Option<Worker>,
    /// Current health status.
    health: PortalHealth,
    /// Event callback registered by the UI.
    on_event: Option<PortalCallback>,
    /// Worker script URL.
    worker_url: String,
    /// Pending message counter for ordering.
    msg_seq: u32,
}

impl PortalInner {
    fn new(worker_url: String) -> Self {
        Self {
            worker: None,
            health: PortalHealth::Idle,
            on_event: None,
            worker_url,
            msg_seq: 0,
        }
    }

    fn next_seq(&mut self) -> u32 {
        self.msg_seq = self.msg_seq.wrapping_add(1);
        self.msg_seq
    }

    fn emit_event(&self, event_type: &str, data: &str) {
        if let Some(cb) = &self.on_event {
            let _ = cb.call2(
                &JsValue::NULL,
                &JsValue::from_str(event_type),
                &JsValue::from_str(data),
            );
        }
    }
}

#[wasm_bindgen]
impl SymbioticPortal {
    /// Create a new SymbioticPortal instance.
    ///
    /// @param workerUrl - URL to the OmniNode Web Worker script (WASM bundle).
    /// @returns A new SymbioticPortal instance.
    #[wasm_bindgen(constructor)]
    pub fn new(worker_url: &str) -> SymbioticPortal {
        let inner = Box::new(PortalInner::new(worker_url.to_string()));
        SymbioticPortal {
            inner: JsValue::from(inner),
        }
    }

    /// Register an event callback for portal notifications.
    ///
    /// @param callback - Function(eventType: string, data: string) => void
    ///
    /// Event types:
    /// - "ready" — Worker initialized and ready.
    /// - "resonance" — Resonance response from biometric processing.
    /// - "ce_balance" — Current CE balance.
    /// - "gei_state" — Current GEI state.
    /// - "resonance_status" — Current resonance pipeline status.
    /// - "error" — Error message from worker.
    /// - "health" — Health status update.
    pub fn on_event(&mut self, callback: &Function) {
        let inner: &mut PortalInner = self.inner.unchecked_ref();
        inner.on_event = Some(callback.clone());
    }

    /// Initialize the OmniNode Web Worker.
    ///
    /// Spawns a new Web Worker from the configured URL and sets up the
    /// message bridge. Returns a Promise that resolves when the worker
    /// reports "ready".
    ///
    /// @returns Promise<void>
    pub fn init(&mut self) -> Promise {
        let inner: &mut PortalInner = self.inner.unchecked_ref();
        let (resolve, reject) = js_sys::Promise::promise();

        match Worker::new(&inner.worker_url) {
            Ok(worker) => {
                inner.health = PortalHealth::Starting;
                inner.emit_event("health", &inner.health.to_string());

                // Capture `self` for the closure
                let portal_clone = self.inner.clone();

                // Set up message handler
                let closure =
                    Closure::<dyn FnMut(MessageEvent)>::new(move |event: &MessageEvent| {
                        let portal_inner: &mut PortalInner = portal_clone.unchecked_ref();

                        // Parse the response from the worker
                        if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                            if let Some(type_val) = Reflect::get(&data, &JsValue::from_str("type"))
                                .ok()
                                .and_then(|v| v.as_f64())
                            {
                                let response_type = type_val as u8;
                                let payload = Reflect::get(&data, &JsValue::from_str("payload"))
                                    .ok()
                                    .and_then(|v| v.as_string())
                                    .unwrap_or_default();

                                match response_type {
                                    x if x == PortalResponse::Ready as u8 => {
                                        portal_inner.health = PortalHealth::Healthy;
                                        portal_inner.emit_event("ready", &payload);
                                        portal_inner
                                            .emit_event("health", &portal_inner.health.to_string());
                                        let _ = resolve.call0(&JsValue::NULL);
                                    }
                                    x if x == PortalResponse::ResonanceResult as u8 => {
                                        portal_inner.emit_event("resonance", &payload);
                                    }
                                    x if x == PortalResponse::CeBalance as u8 => {
                                        portal_inner.emit_event("ce_balance", &payload);
                                    }
                                    x if x == PortalResponse::GeiState as u8 => {
                                        portal_inner.emit_event("gei_state", &payload);
                                    }
                                    x if x == PortalResponse::ResonanceStatus as u8 => {
                                        portal_inner.emit_event("resonance_status", &payload);
                                    }
                                    x if x == PortalResponse::CeDeposited as u8 => {
                                        portal_inner.emit_event("ce_deposited", &payload);
                                    }
                                    x if x == PortalResponse::Calibrated as u8 => {
                                        portal_inner.emit_event("calibrated", &payload);
                                    }
                                    x if x == PortalResponse::Stopped as u8 => {
                                        portal_inner.health = PortalHealth::Idle;
                                        portal_inner.emit_event("stopped", &payload);
                                        portal_inner
                                            .emit_event("health", &portal_inner.health.to_string());
                                    }
                                    x if x == PortalResponse::Error as u8 => {
                                        portal_inner.health = PortalHealth::Failed;
                                        portal_inner.emit_event("error", &payload);
                                        portal_inner
                                            .emit_event("health", &portal_inner.health.to_string());
                                        let _ = reject
                                            .call1(&JsValue::NULL, &JsValue::from_str(&payload));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    });

                // Attach the message handler to the worker
                let _ = worker.set_onmessage(Some(closure.as_ref().unchecked_ref()));
                closure.forget(); // Prevent closure from being dropped

                inner.worker = Some(worker);

                // Send init message to worker
                if let Some(ref worker) = inner.worker {
                    let init_msg = PortalMessage::Init.to_js_value(None).unwrap_or_default();
                    let _ = worker.post_message(&init_msg);
                }
            }
            Err(e) => {
                inner.health = PortalHealth::Failed;
                inner.emit_event("health", &inner.health.to_string());
                let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&e.to_string()));
            }
        }

        Promise::unchecked_from_js(resolve.into())
    }

    /// Send a biometric sample to the Resonance Pipeline.
    ///
    /// @param sampleJson - JSON string containing { rppg: f32[], voice: f32[], expression: f32[] }.
    /// @returns Promise<void>
    pub fn send_biometric(&self, sample_json: &str) -> Result<Promise, JsValue> {
        let inner: &PortalInner = self.inner.unchecked_ref();
        let worker = inner
            .worker
            .as_ref()
            .ok_or(JsValue::from_str("Worker not initialized"))?;

        let msg = PortalMessage::BiometricSample.to_js_value(Some(sample_json))?;
        let _ = worker.post_message(&msg);
        Ok(Promise::new_with_promise_and_rejection_handler(
            &js_sys::Promise::promise().0,
            &Function::new_no_args("function() {}")
                .unwrap_or_else(|_| Function::new_no_args("function() {}").unwrap()),
        )
        .unwrap_or_else(|_| js_sys::Promise::promise().0.into()))
    }

    /// Query the current CE (Compute Energy) balance.
    ///
    /// @returns Promise<void>
    pub fn query_ce_balance(&self) -> Result<Promise, JsValue> {
        let inner: &PortalInner = self.inner.unchecked_ref();
        let worker = inner
            .worker
            .as_ref()
            .ok_or(JsValue::from_str("Worker not initialized"))?;

        let msg = PortalMessage::QueryCeBalance.to_js_value(None)?;
        let _ = worker.post_message(&msg);
        Ok(Promise::new_with_promise_and_rejection_handler(
            &js_sys::Promise::promise().0,
            &Function::new_no_args("function() {}")
                .unwrap_or_else(|_| Function::new_no_args("function() {}").unwrap()),
        )
        .unwrap_or_else(|_| js_sys::Promise::promise().0.into()))
    }

    /// Query the current GEI (Geometric Ethical Invariant) state.
    ///
    /// @returns Promise<void>
    pub fn query_gei_state(&self) -> Result<Promise, JsValue> {
        let inner: &PortalInner = self.inner.unchecked_ref();
        let worker = inner
            .worker
            .as_ref()
            .ok_or(JsValue::from_str("Worker not initialized"))?;

        let msg = PortalMessage::QueryGeiState.to_js_value(None)?;
        let _ = worker.post_message(&msg);
        Ok(Promise::new_with_promise_and_rejection_handler(
            &js_sys::Promise::promise().0,
            &Function::new_no_args("function() {}")
                .unwrap_or_else(|_| Function::new_no_args("function() {}").unwrap()),
        )
        .unwrap_or_else(|_| js_sys::Promise::promise().0.into()))
    }

    /// Query the current Resonance status.
    ///
    /// @returns Promise<void>
    pub fn query_resonance_status(&self) -> Result<Promise, JsValue> {
        let inner: &PortalInner = self.inner.unchecked_ref();
        let worker = inner
            .worker
            .as_ref()
            .ok_or(JsValue::from_str("Worker not initialized"))?;

        let msg = PortalMessage::QueryResonanceStatus.to_js_value(None)?;
        let _ = worker.post_message(&msg);
        Ok(Promise::new_with_promise_and_rejection_handler(
            &js_sys::Promise::promise().0,
            &Function::new_no_args("function() {}")
                .unwrap_or_else(|_| Function::new_no_args("function() {}").unwrap()),
        )
        .unwrap_or_else(|_| js_sys::Promise::promise().0.into()))
    }

    /// Deposit CE credits.
    ///
    /// @param amountJson - JSON string containing { amount: number }.
    /// @returns Promise<void>
    pub fn deposit_ce(&self, amount_json: &str) -> Result<Promise, JsValue> {
        let inner: &PortalInner = self.inner.unchecked_ref();
        let worker = inner
            .worker
            .as_ref()
            .ok_or(JsValue::from_str("Worker not initialized"))?;

        let msg = PortalMessage::DepositCe.to_js_value(Some(amount_json))?;
        let _ = worker.post_message(&msg);
        Ok(Promise::new_with_promise_and_rejection_handler(
            &js_sys::Promise::promise().0,
            &Function::new_no_args("function() {}")
                .unwrap_or_else(|_| Function::new_no_args("function() {}").unwrap()),
        )
        .unwrap_or_else(|_| js_sys::Promise::promise().0.into()))
    }

    /// Calibrate the biometric baseline.
    ///
    /// @param baselineJson - JSON string containing baseline biometric values.
    /// @returns Promise<void>
    pub fn calibrate_baseline(&self, baseline_json: &str) -> Result<Promise, JsValue> {
        let inner: &PortalInner = self.inner.unchecked_ref();
        let worker = inner
            .worker
            .as_ref()
            .ok_or(JsValue::from_str("Worker not initialized"))?;

        let msg = PortalMessage::CalibrateBaseline.to_js_value(Some(baseline_json))?;
        let _ = worker.post_message(&msg);
        Ok(Promise::new_with_promise_and_rejection_handler(
            &js_sys::Promise::promise().0,
            &Function::new_no_args("function() {}")
                .unwrap_or_else(|_| Function::new_no_args("function() {}").unwrap()),
        )
        .unwrap_or_else(|_| js_sys::Promise::promise().0.into()))
    }

    /// Get the current health status of the portal.
    ///
    /// @returns string — One of: "idle", "starting", "healthy", "degraded", "failed".
    #[wasm_bindgen(getter)]
    pub fn health(&self) -> String {
        let inner: &PortalInner = self.inner.unchecked_ref();
        inner.health.to_string()
    }

    /// Gracefully shutdown the OmniNode Web Worker.
    ///
    /// @returns Promise<void>
    pub fn shutdown(&mut self) -> Promise {
        let inner: &mut PortalInner = self.inner.unchecked_ref();
        let (resolve, _) = js_sys::Promise::promise();

        if let Some(ref worker) = inner.worker {
            let _ = PortalMessage::Shutdown
                .to_js_value(None)
                .and_then(|msg| worker.post_message(&msg));
            worker.terminate();
        }

        inner.worker = None;
        inner.health = PortalHealth::Idle;
        inner.emit_event("health", &inner.health.to_string());

        let _ = resolve.call0(&JsValue::NULL);
        Promise::unchecked_from_js(resolve.into())
    }
}

impl Drop for SymbioticPortal {
    fn drop(&mut self) {
        let inner: &mut PortalInner = self.inner.unchecked_ref();
        if let Some(ref worker) = inner.worker {
            worker.terminate();
        }
    }
}

/// Generate the Web Worker bootstrap script as a JavaScript string.
///
/// This script loads the WASM module and sets up the message bridge
/// between the worker and the OmniNode.
///
/// @param wasmUrl - URL to the compiled WASM module.
/// @param wasmInitUrl - URL to the wasm-bindgen generated JS glue.
/// @returns string — Complete Web Worker script source.
#[wasm_bindgen]
pub fn generate_worker_script(wasm_url: &str, wasm_init_url: &str) -> String {
    format!(
        r#"// ed2kIA SymbioticPortal Web Worker — Auto-generated
// Loads OmniNode WASM and bridges messages to/from the UI thread.

importScripts("{wasm_init_url}");

let omniNode = null;
let initialized = false;

async function initNode() {{
  try {{
    const response = await init("{wasm_url}");
    omniNode = response;
    initialized = true;
    postMessage({{ type: 0, payload: JSON.stringify({{ status: "ready", version: "v3.7-symbiotic-portal" }}) }});
  }} catch (err) {{
    postMessage({{ type: 255, payload: "WASM init failed: " + err.message }});
  }}
}}

self.onmessage = function(event) {{
  const msg = event.data;
  if (!msg || !msg.type {{ return; }}

  switch (msg.type) {{
    case 0: // Init
      if (!initialized) {{
        initNode();
      }}
      break;

    case 1: // BiometricSample
      if (omniNode && msg.payload) {{
        try {{
          const result = omniNode.process_biometric(msg.payload);
          postMessage({{ type: 1, payload: JSON.stringify(result) }});
        }} catch (err) {{
          postMessage({{ type: 255, payload: "Biometric error: " + err.message }});
        }}
      }}
      break;

    case 2: // QueryCeBalance
      if (omniNode) {{
        try {{
          const balance = omniNode.get_ce_balance();
          postMessage({{ type: 2, payload: JSON.stringify({{ balance }}) }});
        }} catch (err) {{
          postMessage({{ type: 255, payload: "CE query error: " + err.message }});
        }}
      }}
      break;

    case 3: // QueryGeiState
      if (omniNode) {{
        try {{
          const state = omniNode.get_gei_state();
          postMessage({{ type: 3, payload: JSON.stringify(state) }});
        }} catch (err) {{
          postMessage({{ type: 255, payload: "GEI query error: " + err.message }});
        }}
      }}
      break;

    case 4: // QueryResonanceStatus
      if (omniNode) {{
        try {{
          const status = omniNode.get_resonance_status();
          postMessage({{ type: 4, payload: JSON.stringify(status) }});
        }} catch (err) {{
          postMessage({{ type: 255, payload: "Resonance query error: " + err.message }});
        }}
      }}
      break;

    case 5: // DepositCe
      if (omniNode && msg.payload) {{
        try {{
          const data = JSON.parse(msg.payload);
          omniNode.deposit_ce(data.amount);
          postMessage({{ type: 5, payload: JSON.stringify({{ deposited: data.amount }}) }});
        }} catch (err) {{
          postMessage({{ type: 255, payload: "Deposit error: " + err.message }});
        }}
      }}
      break;

    case 6: // CalibrateBaseline
      if (omniNode && msg.payload) {{
        try {{
          omniNode.calibrate_baseline(msg.payload);
          postMessage({{ type: 6, payload: JSON.stringify({{ calibrated: true }}) }});
        }} catch (err) {{
          postMessage({{ type: 255, payload: "Calibration error: " + err.message }});
        }}
      }}
      break;

    case 7: // Shutdown
      self.close();
      break;

    default:
      break;
  }}
}};"#,
        wasm_url = wasm_url.replace('\\', "\\\\").replace('"', "\\\""),
        wasm_init_url = wasm_init_url.replace('\\', "\\\\").replace('"', "\\\""),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portal_message_serialization() {
        let msg = PortalMessage::Init;
        let js_val = msg.to_js_value(None).expect("Init should serialize");
        assert!(!js_val.is_undefined());

        let msg = PortalMessage::BiometricSample;
        let js_val = msg
            .to_js_value(Some(r#"{"rppg":[0.5]}"#))
            .expect("BiometricSample should serialize");
        assert!(!js_val.is_undefined());
    }

    #[test]
    fn test_portal_health_display() {
        assert_eq!(format!("{}", PortalHealth::Idle), "idle");
        assert_eq!(format!("{}", PortalHealth::Starting), "starting");
        assert_eq!(format!("{}", PortalHealth::Healthy), "healthy");
        assert_eq!(format!("{}", PortalHealth::Degraded), "degraded");
        assert_eq!(format!("{}", PortalHealth::Failed), "failed");
    }

    #[test]
    fn test_portal_inner_creation() {
        let inner = PortalInner::new("worker.js".to_string());
        assert_eq!(inner.health, PortalHealth::Idle);
        assert_eq!(inner.worker_url, "worker.js");
        assert!(inner.worker.is_none());
        assert!(inner.on_event.is_none());
    }

    #[test]
    fn test_sequence_counter_increments() {
        let mut inner = PortalInner::new("worker.js".to_string());
        let s1 = inner.next_seq();
        let s2 = inner.next_seq();
        assert_eq!(s2, s1 + 1);
    }

    #[test]
    fn test_sequence_counter_wraps() {
        let mut inner = PortalInner::new("worker.js".to_string());
        inner.msg_seq = u32::MAX;
        let next = inner.next_seq();
        assert_eq!(next, 1);
    }

    #[test]
    fn test_generate_worker_script() {
        let script = generate_worker_script("/ed2kia_bg.wasm", "/ed2kia.js");
        assert!(script.contains("importScripts"));
        assert!(script.contains("/ed2kia_bg.wasm"));
        assert!(script.contains("/ed2kia.js"));
        assert!(script.contains("postMessage"));
        assert!(script.contains("SymbioticPortal"));
    }

    #[test]
    fn test_generate_worker_script_escapes_special_chars() {
        let script = generate_worker_script("path\\with\"quotes.wasm", "init.js");
        assert!(script.contains(r#"path\\with\"quotes.wasm"#));
    }

    #[test]
    fn test_portal_message_variants() {
        assert_eq!(PortalMessage::Init as u8, 0);
        assert_eq!(PortalMessage::BiometricSample as u8, 1);
        assert_eq!(PortalMessage::QueryCeBalance as u8, 2);
        assert_eq!(PortalMessage::QueryGeiState as u8, 3);
        assert_eq!(PortalMessage::QueryResonanceStatus as u8, 4);
        assert_eq!(PortalMessage::DepositCe as u8, 5);
        assert_eq!(PortalMessage::CalibrateBaseline as u8, 6);
        assert_eq!(PortalMessage::Shutdown as u8, 7);
        assert_eq!(PortalMessage::Custom as u8, 255);
    }

    #[test]
    fn test_portal_response_variants() {
        assert_eq!(PortalResponse::Ready as u8, 0);
        assert_eq!(PortalResponse::ResonanceResult as u8, 1);
        assert_eq!(PortalResponse::CeBalance as u8, 2);
        assert_eq!(PortalResponse::GeiState as u8, 3);
        assert_eq!(PortalResponse::ResonanceStatus as u8, 4);
        assert_eq!(PortalResponse::CeDeposited as u8, 5);
        assert_eq!(PortalResponse::Calibrated as u8, 6);
        assert_eq!(PortalResponse::Stopped as u8, 7);
        assert_eq!(PortalResponse::Error as u8, 255);
    }
}
