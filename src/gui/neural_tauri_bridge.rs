//! Neural Tauri Bridge v1 — Integration layer between Neural Steer UI and Tauri GUI scaffold.
//!
//! Provides the bridge that connects `neural_steer_ui.rs` slider components with
//! `tauri_scaffold.rs` state management and backend commands. Implements ethical
//! slider validation, range enforcement, and Tauri-compatible serialization.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Tauri Frontend                           │
//! │  (HTML/CSS/JS sliders → invoke('apply-steer-config'))      │
//! ├─────────────────────────────────────────────────────────────┤
//! │                  NeuralTauriBridge                          │
//! │  ┌──────────────────┐  ┌──────────────────────────────┐    │
//! │  │  SliderValidator │  │  EthicalBoundsEnforcer       │    │
//! │  └──────────────────┘  └──────────────────────────────┘    │
//! ├─────────────────────────────────────────────────────────────┤
//! │              Rust Backend                                   │
//! │  ┌──────────────────┐  ┌──────────────────────────────┐    │
//! │  │  TauriState      │  │  NeuralSteerConfig           │    │
//! │  └──────────────────┘  └──────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! Feature-gated behind `cfg(feature = "v2.0-sprint2")`.

mod internal {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Bridge operation errors.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum BridgeError {
        /// Slider value outside ethical bounds.
        ValueOutOfBounds { slider: String, value: f32, min: f32, max: f32 },
        /// Safety threshold violated.
        SafetyThresholdViolated { value: f32, threshold: f32 },
        /// Invalid configuration from frontend.
        InvalidConfig(String),
        /// Serialization error.
        SerializationError(String),
        /// State synchronization failed.
        StateSyncFailed(String),
    }

    impl std::fmt::Display for BridgeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BridgeError::ValueOutOfBounds { slider, value, min, max } => {
                    write!(f, "Slider '{}' value {} outside bounds [{}, {}]", slider, value, min, max)
                }
                BridgeError::SafetyThresholdViolated { value, threshold } => {
                    write!(f, "Safety value {} below threshold {}", value, threshold)
                }
                BridgeError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
                BridgeError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
                BridgeError::StateSyncFailed(msg) => write!(f, "State sync failed: {}", msg),
            }
        }
    }

    impl std::error::Error for BridgeError {}

    // ============================================================================
    // Ethical Bounds Configuration
    // ============================================================================

    /// Hardcoded ethical bounds for neural steering sliders.
    /// These limits cannot be overridden by user configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct EthicalBounds {
        /// Empathy range: [-0.5, 0.8]
        pub empathy_min: f32,
        /// Empathy range: [-0.5, 0.8]
        pub empathy_max: f32,
        /// Creativity range: [-0.3, 0.9]
        pub creativity_min: f32,
        /// Creativity range: [-0.3, 0.9]
        pub creativity_max: f32,
        /// Safety range: [0.2, 1.0] — minimum safety is 0.2
        pub safety_min: f32,
        /// Safety range: [0.2, 1.0]
        pub safety_max: f32,
    }

    impl EthicalBounds {
        /// Return the hardcoded ethical bounds. These are immutable.
        pub fn default_bounds() -> Self {
            Self {
                empathy_min: -0.5,
                empathy_max: 0.8,
                creativity_min: -0.3,
                creativity_max: 0.9,
                safety_min: 0.2,
                safety_max: 1.0,
            }
        }

        /// Validate empathy value against ethical bounds.
        pub fn validate_empathy(&self, value: f32) -> Result<(), BridgeError> {
            if value < self.empathy_min || value > self.empathy_max {
                return Err(BridgeError::ValueOutOfBounds {
                    slider: "empathy".to_string(),
                    value,
                    min: self.empathy_min,
                    max: self.empathy_max,
                });
            }
            Ok(())
        }

        /// Validate creativity value against ethical bounds.
        pub fn validate_creativity(&self, value: f32) -> Result<(), BridgeError> {
            if value < self.creativity_min || value > self.creativity_max {
                return Err(BridgeError::ValueOutOfBounds {
                    slider: "creativity".to_string(),
                    value,
                    min: self.creativity_min,
                    max: self.creativity_max,
                });
            }
            Ok(())
        }

        /// Validate safety value against ethical bounds.
        pub fn validate_safety(&self, value: f32) -> Result<(), BridgeError> {
            if value < self.safety_min || value > self.safety_max {
                return Err(BridgeError::SafetyThresholdViolated {
                    value,
                    threshold: self.safety_min,
                });
            }
            Ok(())
        }

        /// Validate all three slider values at once.
        pub fn validate_all(&self, empathy: f32, creativity: f32, safety: f32) -> Result<(), BridgeError> {
            self.validate_empathy(empathy)?;
            self.validate_creativity(creativity)?;
            self.validate_safety(safety)?;
            Ok(())
        }

        /// Clamp a value to the given range.
        pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
            if value < min {
                min
            } else if value > max {
                max
            } else {
                value
            }
        }

        /// Clamp all slider values to ethical bounds.
        pub fn clamp_all(&self, empathy: f32, creativity: f32, safety: f32) -> (f32, f32, f32) {
            (
                Self::clamp(empathy, self.empathy_min, self.empathy_max),
                Self::clamp(creativity, self.creativity_min, self.creativity_max),
                Self::clamp(safety, self.safety_min, self.safety_max),
            )
        }
    }

    impl Default for EthicalBounds {
        fn default() -> Self {
            Self::default_bounds()
        }
    }

    // ============================================================================
    // Bridge Config Request/Response
    // ============================================================================

    /// Incoming config request from Tauri frontend.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SteerConfigRequest {
        /// Empathy slider value.
        pub empathy: f32,
        /// Creativity slider value.
        pub creativity: f32,
        /// Safety slider value.
        pub safety: f32,
        /// Optional request ID for tracing.
        pub request_id: Option<String>,
    }

    /// Validated and clamped config ready for backend application.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SteerConfigResponse {
        /// Whether the config was accepted without clamping.
        pub accepted: bool,
        /// Final empathy value (may be clamped).
        pub empathy: f32,
        /// Final creativity value (may be clamped).
        pub creativity: f32,
        /// Final safety value (may be clamped).
        pub safety: f32,
        /// Computed weighted steering signal.
        pub signal: f32,
        /// List of warnings (e.g., "empathy clamped from 0.9 to 0.8").
        pub warnings: Vec<String>,
        /// Request ID if provided.
        pub request_id: Option<String>,
    }

    impl SteerConfigResponse {
        /// Compute weighted steering signal: 0.5*safety + 0.3*empathy + 0.2*creativity.
        pub fn compute_signal(empathy: f32, creativity: f32, safety: f32) -> f32 {
            0.5 * safety + 0.3 * empathy + 0.2 * creativity
        }
    }

    // ============================================================================
    // Neural Tauri Bridge
    // ============================================================================

    /// Bridge connecting Neural Steer UI with Tauri GUI scaffold.
    pub struct NeuralTauriBridge {
        /// Hardcoded ethical bounds.
        bounds: EthicalBounds,
        /// Current active config.
        current_config: Option<SteerConfigResponse>,
        /// Config history for rollback.
        config_history: Vec<SteerConfigResponse>,
        /// Request counter for auto-generated IDs.
        request_counter: u64,
    }

    impl NeuralTauriBridge {
        /// Create a new bridge with default ethical bounds.
        pub fn new() -> Self {
            Self {
                bounds: EthicalBounds::default_bounds(),
                current_config: None,
                config_history: Vec::new(),
                request_counter: 0,
            }
        }

        /// Generate a request ID if none provided.
        fn next_request_id(&mut self) -> String {
            self.request_counter += 1;
            format!("req-{}", self.request_counter)
        }

        /// Process incoming config request from Tauri frontend.
        ///
        /// Validates against ethical bounds, clamps if necessary,
        /// and returns the final config with warnings.
        pub fn process_request(&mut self, request: SteerConfigRequest) -> SteerConfigResponse {
            let mut warnings = Vec::new();
            let mut accepted = true;

            // Clamp values and generate warnings
            let (empathy, empathy_orig) = (
                Self::clamp_value(request.empathy, self.bounds.empathy_min, self.bounds.empathy_max),
                request.empathy,
            );
            if empathy != empathy_orig {
                warnings.push(format!(
                    "empathy clamped from {} to {}",
                    empathy_orig, empathy
                ));
                accepted = false;
            }

            let (creativity, creativity_orig) = (
                Self::clamp_value(request.creativity, self.bounds.creativity_min, self.bounds.creativity_max),
                request.creativity,
            );
            if creativity != creativity_orig {
                warnings.push(format!(
                    "creativity clamped from {} to {}",
                    creativity_orig, creativity
                ));
                accepted = false;
            }

            let (safety, safety_orig) = (
                Self::clamp_value(request.safety, self.bounds.safety_min, self.bounds.safety_max),
                request.safety,
            );
            if safety != safety_orig {
                warnings.push(format!(
                    "safety clamped from {} to {}",
                    safety_orig, safety
                ));
                accepted = false;
            }

            let signal = SteerConfigResponse::compute_signal(empathy, creativity, safety);

            let response = SteerConfigResponse {
                accepted,
                empathy,
                creativity,
                safety,
                signal,
                warnings,
                request_id: request.request_id.clone(),
            };

            // Save previous config to history
            if let Some(ref current) = self.current_config {
                self.config_history.push(current.clone());
                // Keep only last 10 configs in history
                if self.config_history.len() > 10 {
                    self.config_history.remove(0);
                }
            }

            self.current_config = Some(response.clone());
            response
        }

        /// Clamp a single value to the given range.
        fn clamp_value(value: f32, min: f32, max: f32) -> f32 {
            if value < min {
                min
            } else if value > max {
                max
            } else {
                value
            }
        }

        /// Rollback to previous config.
        pub fn rollback(&mut self) -> Option<SteerConfigResponse> {
            let previous = self.config_history.pop()?;
            self.current_config = Some(previous.clone());
            Some(previous)
        }

        /// Get current active config.
        pub fn get_current_config(&self) -> Option<&SteerConfigResponse> {
            self.current_config.as_ref()
        }

        /// Get config history length.
        pub fn history_len(&self) -> usize {
            self.config_history.len()
        }

        /// Serialize current config to JSON string for Tauri event emission.
        pub fn serialize_current(&self) -> Result<Option<String>, BridgeError> {
            match &self.current_config {
                Some(config) => {
                    let json = serde_json::to_string(config)
                        .map_err(|e| BridgeError::SerializationError(e.to_string()))?;
                    Ok(Some(json))
                }
                None => Ok(None),
            }
        }

        /// Deserialize config from JSON string.
        pub fn deserialize_config(json: &str) -> Result<SteerConfigResponse, BridgeError> {
            serde_json::from_str(json)
                .map_err(|e| BridgeError::SerializationError(e.to_string()))
        }

        /// Reset bridge state.
        pub fn reset(&mut self) {
            self.current_config = None;
            self.config_history.clear();
            self.request_counter = 0;
        }
    }

    impl Default for NeuralTauriBridge {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Tauri State Integration
    // ============================================================================

    /// Extended Tauri state that includes Neural Steer Bridge.
    pub struct NeuralTauriState {
        /// Key-value state storage.
        state: HashMap<String, String>,
        /// Neural Tauri bridge for steering config.
        bridge: NeuralTauriBridge,
    }

    impl NeuralTauriState {
        /// Create new state with initialized bridge.
        pub fn new() -> Self {
            Self {
                state: HashMap::new(),
                bridge: NeuralTauriBridge::new(),
            }
        }

        /// Get state value by key.
        pub fn get(&self, key: &str) -> Option<&String> {
            self.state.get(key)
        }

        /// Set state value.
        pub fn set(&mut self, key: String, value: String) {
            self.state.insert(key, value);
        }

        /// Process steering config request through the bridge.
        pub fn process_steer_request(&mut self, request: SteerConfigRequest) -> SteerConfigResponse {
            self.bridge.process_request(request)
        }

        /// Get current steering config as JSON.
        pub fn get_steer_config_json(&self) -> Result<Option<String>, BridgeError> {
            self.bridge.serialize_current()
        }

        /// Rollback steering config.
        pub fn rollback_steer_config(&mut self) -> Option<SteerConfigResponse> {
            self.bridge.rollback()
        }

        /// Get bridge reference.
        pub fn bridge(&self) -> &NeuralTauriBridge {
            &self.bridge
        }

        /// Get mutable bridge reference.
        pub fn bridge_mut(&mut self) -> &mut NeuralTauriBridge {
            &mut self.bridge
        }
    }

    impl Default for NeuralTauriState {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    mod tests {
        use super::*;

        #[test]
        fn test_ethical_bounds_default() {
            let bounds = EthicalBounds::default_bounds();
            assert_eq!(bounds.empathy_min, -0.5);
            assert_eq!(bounds.empathy_max, 0.8);
            assert_eq!(bounds.creativity_min, -0.3);
            assert_eq!(bounds.creativity_max, 0.9);
            assert_eq!(bounds.safety_min, 0.2);
            assert_eq!(bounds.safety_max, 1.0);
        }

        #[test]
        fn test_validate_empathy_valid() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_empathy(0.0).is_ok());
            assert!(bounds.validate_empathy(-0.5).is_ok());
            assert!(bounds.validate_empathy(0.8).is_ok());
        }

        #[test]
        fn test_validate_empathy_out_of_bounds() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_empathy(-0.6).is_err());
            assert!(bounds.validate_empathy(0.9).is_err());
        }

        #[test]
        fn test_validate_creativity_valid() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_creativity(0.5).is_ok());
            assert!(bounds.validate_creativity(-0.3).is_ok());
            assert!(bounds.validate_creativity(0.9).is_ok());
        }

        #[test]
        fn test_validate_creativity_out_of_bounds() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_creativity(-0.4).is_err());
            assert!(bounds.validate_creativity(1.0).is_err());
        }

        #[test]
        fn test_validate_safety_valid() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_safety(0.5).is_ok());
            assert!(bounds.validate_safety(0.2).is_ok());
            assert!(bounds.validate_safety(1.0).is_ok());
        }

        #[test]
        fn test_validate_safety_below_threshold() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_safety(0.1).is_err());
            if let Err(BridgeError::SafetyThresholdViolated { .. }) = bounds.validate_safety(0.0) {
            } else {
                panic!("Expected SafetyThresholdViolated");
            }
        }

        #[test]
        fn test_validate_all_pass() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_all(0.0, 0.0, 0.5).is_ok());
        }

        #[test]
        fn test_validate_all_fail_empathy() {
            let bounds = EthicalBounds::default();
            assert!(bounds.validate_all(1.0, 0.0, 0.5).is_err());
        }

        #[test]
        fn test_clamp_all_within_bounds() {
            let bounds = EthicalBounds::default();
            let (e, c, s) = bounds.clamp_all(0.0, 0.0, 0.5);
            assert_eq!((e, c, s), (0.0, 0.0, 0.5));
        }

        #[test]
        fn test_clamp_all_out_of_bounds() {
            let bounds = EthicalBounds::default();
            let (e, c, s) = bounds.clamp_all(1.0, -1.0, 0.0);
            assert_eq!(e, 0.8); // clamped to max
            assert_eq!(c, -0.3); // clamped to min
            assert_eq!(s, 0.2); // clamped to min
        }

        #[test]
        fn test_bridge_new() {
            let bridge = NeuralTauriBridge::new();
            assert!(bridge.get_current_config().is_none());
            assert_eq!(bridge.history_len(), 0);
        }

        #[test]
        fn test_bridge_process_valid_request() {
            let mut bridge = NeuralTauriBridge::new();
            let request = SteerConfigRequest {
                empathy: 0.3,
                creativity: 0.4,
                safety: 0.6,
                request_id: Some("test-1".to_string()),
            };
            let response = bridge.process_request(request);
            assert!(response.accepted);
            assert_eq!(response.empathy, 0.3);
            assert_eq!(response.creativity, 0.4);
            assert_eq!(response.safety, 0.6);
            assert!(response.warnings.is_empty());
            assert_eq!(response.request_id, Some("test-1".to_string()));
        }

        #[test]
        fn test_bridge_process_request_with_clamping() {
            let mut bridge = NeuralTauriBridge::new();
            let request = SteerConfigRequest {
                empathy: 1.0, // exceeds max 0.8
                creativity: -0.5, // below min -0.3
                safety: 0.1, // below min 0.2
                request_id: None,
            };
            let response = bridge.process_request(request);
            assert!(!response.accepted);
            assert_eq!(response.empathy, 0.8);
            assert_eq!(response.creativity, -0.3);
            assert_eq!(response.safety, 0.2);
            assert_eq!(response.warnings.len(), 3);
        }

        #[test]
        fn test_bridge_signal_computation() {
            let signal = SteerConfigResponse::compute_signal(0.0, 0.0, 1.0);
            assert!((signal - 0.5).abs() < 0.001);

            let signal = SteerConfigResponse::compute_signal(1.0, 1.0, 1.0);
            assert!((signal - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_bridge_rollback() {
            let mut bridge = NeuralTauriBridge::new();
            // Apply first config
            bridge.process_request(SteerConfigRequest {
                empathy: 0.1,
                creativity: 0.2,
                safety: 0.5,
                request_id: None,
            });
            // Apply second config
            bridge.process_request(SteerConfigRequest {
                empathy: 0.5,
                creativity: 0.6,
                safety: 0.8,
                request_id: None,
            });
            // Rollback to first
            let rolled_back = bridge.rollback().unwrap();
            assert_eq!(rolled_back.empathy, 0.1);
            assert_eq!(rolled_back.creativity, 0.2);
        }

        #[test]
        fn test_bridge_rollback_empty() {
            let mut bridge = NeuralTauriBridge::new();
            assert!(bridge.rollback().is_none());
        }

        #[test]
        fn test_bridge_history_limit() {
            let mut bridge = NeuralTauriBridge::new();
            for i in 0..15 {
                bridge.process_request(SteerConfigRequest {
                    empathy: i as f32 * 0.05,
                    creativity: 0.0,
                    safety: 0.5,
                    request_id: None,
                });
            }
            assert!(bridge.history_len() <= 10);
        }

        #[test]
        fn test_bridge_serialize_deserialize() {
            let mut bridge = NeuralTauriBridge::new();
            bridge.process_request(SteerConfigRequest {
                empathy: 0.3,
                creativity: 0.4,
                safety: 0.6,
                request_id: None,
            });
            let json = bridge.serialize_current().unwrap().unwrap();
            let config = NeuralTauriBridge::deserialize_config(&json).unwrap();
            assert_eq!(config.empathy, 0.3);
            assert_eq!(config.creativity, 0.4);
            assert_eq!(config.safety, 0.6);
        }

        #[test]
        fn test_bridge_reset() {
            let mut bridge = NeuralTauriBridge::new();
            bridge.process_request(SteerConfigRequest {
                empathy: 0.3,
                creativity: 0.4,
                safety: 0.6,
                request_id: None,
            });
            bridge.reset();
            assert!(bridge.get_current_config().is_none());
            assert_eq!(bridge.history_len(), 0);
        }

        #[test]
        fn test_neural_tauri_state_new() {
            let state = NeuralTauriState::new();
            assert!(state.get("nonexistent").is_none());
        }

        #[test]
        fn test_neural_tauri_state_set_get() {
            let mut state = NeuralTauriState::new();
            state.set("key".to_string(), "value".to_string());
            assert_eq!(state.get("key").unwrap(), "value");
        }

        #[test]
        fn test_neural_tauri_state_steer_request() {
            let mut state = NeuralTauriState::new();
            let response = state.process_steer_request(SteerConfigRequest {
                empathy: 0.3,
                creativity: 0.4,
                safety: 0.6,
                request_id: None,
            });
            assert!(response.accepted);
            assert_eq!(response.empathy, 0.3);
        }

        #[test]
        fn test_neural_tauri_state_rollback() {
            let mut state = NeuralTauriState::new();
            state.process_steer_request(SteerConfigRequest {
                empathy: 0.1,
                creativity: 0.2,
                safety: 0.5,
                request_id: None,
            });
            state.process_steer_request(SteerConfigRequest {
                empathy: 0.5,
                creativity: 0.6,
                safety: 0.8,
                request_id: None,
            });
            let rolled_back = state.rollback_steer_config().unwrap();
            assert_eq!(rolled_back.empathy, 0.1);
        }

        #[test]
        fn test_error_display() {
            let err = BridgeError::ValueOutOfBounds {
                slider: "empathy".to_string(),
                value: 1.0,
                min: -0.5,
                max: 0.8,
            };
            let msg = format!("{}", err);
            assert!(msg.contains("empathy"));
            assert!(msg.contains("1") || msg.contains("1.0"));
        }

        #[test]
        fn test_full_bridge_lifecycle() {
            let mut bridge = NeuralTauriBridge::new();

            // Initial state
            assert!(bridge.get_current_config().is_none());

            // Apply valid config
            let resp1 = bridge.process_request(SteerConfigRequest {
                empathy: 0.3,
                creativity: 0.4,
                safety: 0.6,
                request_id: Some("lifecycle-1".to_string()),
            });
            assert!(resp1.accepted);
            assert_eq!(bridge.history_len(), 0); // First config, no history yet

            // Apply second config
            let resp2 = bridge.process_request(SteerConfigRequest {
                empathy: 0.5,
                creativity: 0.6,
                safety: 0.8,
                request_id: Some("lifecycle-2".to_string()),
            });
            assert!(resp2.accepted);
            assert_eq!(bridge.history_len(), 1);

            // Serialize current
            let json = bridge.serialize_current().unwrap().unwrap();
            assert!(json.contains("0.5"));

            // Rollback
            let rolled = bridge.rollback().unwrap();
            assert_eq!(rolled.empathy, 0.3);

            // Reset
            bridge.reset();
            assert!(bridge.get_current_config().is_none());
            assert_eq!(bridge.history_len(), 0);
        }
    }
}

pub use internal::{
    BridgeError, EthicalBounds, NeuralTauriBridge, NeuralTauriState,
    SteerConfigRequest, SteerConfigResponse,
};
