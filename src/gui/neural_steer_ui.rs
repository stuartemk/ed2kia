//! Neural Steer UI v1 — Ethical slider components for AI behavior control.
//!
//! Provides slider-based UI controls for neural steering parameters:
//! - `SteeringSlider` — Empathy, Creativity, Safety sliders with ethical bounds
//! - `NeuralSteerConfig` — Aggregated steering configuration
//! - `SteeringSignalBridge` — Integration mock with async_steering.rs
//!
//! Feature-gated behind `cfg(feature = "v1.9-sprint2")`.

mod internal {
    use std::fmt;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Neural steer UI errors.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SteerUIError {
        /// Slider value out of ethical bounds.
        ValueOutOfBounds,
        /// Invalid slider name.
        InvalidSliderName,
        /// Safety threshold violated.
        SafetyThresholdViolated,
        /// Serialization format error.
        SerializationError,
        /// Rollback failed.
        RollbackFailed,
    }

    impl fmt::Display for SteerUIError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SteerUIError::ValueOutOfBounds => write!(f, "slider value out of ethical bounds"),
                SteerUIError::InvalidSliderName => write!(f, "invalid slider name"),
                SteerUIError::SafetyThresholdViolated => write!(f, "safety threshold violated"),
                SteerUIError::SerializationError => write!(f, "serialization format error"),
                SteerUIError::RollbackFailed => write!(f, "rollback failed"),
            }
        }
    }

    impl std::error::Error for SteerUIError {}

    // ============================================================================
    // Steering Slider
    // ============================================================================

    /// Slider type for neural steering parameters.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SliderType {
        /// Empathy slider — controls emotional alignment.
        Empathy,
        /// Creativity slider — controls divergent thinking.
        Creativity,
        /// Safety slider — controls risk aversion.
        Safety,
    }

    impl fmt::Display for SliderType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SliderType::Empathy => write!(f, "empathy"),
                SliderType::Creativity => write!(f, "creativity"),
                SliderType::Safety => write!(f, "safety"),
            }
        }
    }

    /// A single steering slider with ethical bounds.
    #[derive(Debug, Clone)]
    pub struct SteeringSlider {
        /// Slider type.
        pub r#type: SliderType,
        /// Current value in [-1.0, 1.0].
        pub value: f32,
        /// Minimum ethical bound.
        pub min_bound: f32,
        /// Maximum ethical bound.
        pub max_bound: f32,
        /// Safety threshold (minimum safety value allowed).
        pub safety_threshold: f32,
    }

    impl SteeringSlider {
        /// Create a new steering slider with default ethical bounds.
        pub fn new(r#type: SliderType) -> Self {
            let (min_bound, max_bound, safety_threshold) = match r#type {
                SliderType::Empathy => (-0.5, 0.8, 0.0),
                SliderType::Creativity => (-0.3, 0.9, 0.0),
                SliderType::Safety => (0.2, 1.0, 0.2),
            };
            Self {
                r#type,
                value: 0.0,
                min_bound,
                max_bound,
                safety_threshold,
            }
        }

        /// Set slider value with ethical bounds validation.
        ///
        /// # Errors
        /// * `SteerUIError::ValueOutOfBounds` if value outside [min_bound, max_bound].
        /// * `SteerUIError::SafetyThresholdViolated` if safety below threshold.
        pub fn set_value(&mut self, value: f32) -> Result<(), SteerUIError> {
            if value < self.min_bound || value > self.max_bound {
                return Err(SteerUIError::ValueOutOfBounds);
            }
            if self.r#type == SliderType::Safety && value < self.safety_threshold {
                return Err(SteerUIError::SafetyThresholdViolated);
            }
            self.value = value;
            Ok(())
        }

        /// Get current value.
        pub fn get_value(&self) -> f32 {
            self.value
        }

        /// Clamp value to ethical bounds.
        pub fn clamp(&mut self) {
            if self.value < self.min_bound {
                self.value = self.min_bound;
            }
            if self.value > self.max_bound {
                self.value = self.max_bound;
            }
            if self.r#type == SliderType::Safety && self.value < self.safety_threshold {
                self.value = self.safety_threshold;
            }
        }

        /// Check if current value is within ethical bounds.
        pub fn is_within_bounds(&self) -> bool {
            self.value >= self.min_bound
                && self.value <= self.max_bound
                && self.value >= self.safety_threshold
        }

        /// Reset to default value (0.0 or safety_threshold).
        pub fn reset(&mut self) {
            self.value = if self.r#type == SliderType::Safety {
                self.safety_threshold
            } else {
                0.0
            };
        }
    }

    // ============================================================================
    // Neural Steer Config
    // ============================================================================

    /// Aggregated steering configuration from all sliders.
    #[derive(Debug, Clone)]
    pub struct NeuralSteerConfig {
        /// Empathy slider value.
        pub empathy: f32,
        /// Creativity slider value.
        pub creativity: f32,
        /// Safety slider value.
        pub safety: f32,
        /// Configuration version.
        pub version: u32,
        /// Timestamp of last update (Unix ms).
        pub updated_at_ms: u64,
    }

    impl NeuralSteerConfig {
        /// Create a new config from slider values.
        pub fn new(empathy: f32, creativity: f32, safety: f32, updated_at_ms: u64) -> Self {
            Self {
                empathy,
                creativity,
                safety,
                version: 1,
                updated_at_ms,
            }
        }

        /// Serialize to JSON string (simplified).
        pub fn to_json(&self) -> Result<String, SteerUIError> {
            let json = format!(
                "{{\"empathy\":{},\"creativity\":{},\"safety\":{},\"version\":{},\"updated_at_ms\":{}}}",
                self.empathy, self.creativity, self.safety, self.version, self.updated_at_ms
            );
            Ok(json)
        }

        /// Deserialize from JSON string (simplified).
        pub fn from_json(json: &str) -> Result<Self, SteerUIError> {
            let empathy = parse_float_field(json, "empathy")?;
            let creativity = parse_float_field(json, "creativity")?;
            let safety = parse_float_field(json, "safety")?;
            let version = parse_uint_field(json, "version")?.unwrap_or(1);
            let updated_at_ms = parse_u64_field(json, "updated_at_ms")?.unwrap_or(0);
            Ok(Self {
                empathy,
                creativity,
                safety,
                version,
                updated_at_ms,
            })
        }

        /// Check if config meets minimum safety requirements.
        pub fn is_safe(&self) -> bool {
            self.safety >= 0.2
        }

        /// Compute weighted steering signal for async_steering integration.
        /// Weight: safety > empathy > creativity.
        pub fn compute_signal(&self) -> f32 {
            0.5 * self.safety + 0.3 * self.empathy + 0.2 * self.creativity
        }

        /// Increment version.
        pub fn bump_version(&mut self) {
            self.version += 1;
        }
    }

    // ============================================================================
    // Steering Signal Bridge
    // ============================================================================

    /// A steering signal generated from Neural Steer UI.
    #[derive(Debug, Clone)]
    pub struct NeuralSteeringSignal {
        /// Weighted signal value in [-1.0, 1.0].
        pub value: f32,
        /// Sequence number.
        pub seq: u64,
        /// Source identifier.
        pub source: String,
        /// Delay in milliseconds.
        pub delay_ms: u64,
        /// Config version that generated this signal.
        pub config_version: u32,
    }

    /// Integration bridge between Neural Steer UI and async_steering.rs.
    pub struct SteeringSignalBridge {
        /// Current config.
        current_config: NeuralSteerConfig,
        /// Previous config for rollback.
        previous_config: Option<NeuralSteerConfig>,
        /// Signal sequence counter.
        seq_counter: u64,
        /// Source identifier.
        source: String,
    }

    impl SteeringSignalBridge {
        /// Create a new bridge.
        pub fn new(source: String, initial_ms: u64) -> Self {
            Self {
                current_config: NeuralSteerConfig::new(0.0, 0.0, 0.2, initial_ms),
                previous_config: None,
                seq_counter: 0,
                source,
            }
        }

        /// Apply new config, saving previous for rollback.
        ///
        /// # Errors
        /// * `SteerUIError::SafetyThresholdViolated` if new config is unsafe.
        pub fn apply_config(
            &mut self,
            config: NeuralSteerConfig,
            current_ms: u64,
        ) -> Result<NeuralSteeringSignal, SteerUIError> {
            if !config.is_safe() {
                return Err(SteerUIError::SafetyThresholdViolated);
            }
            self.previous_config = Some(self.current_config.clone());
            self.current_config = config;
            self.seq_counter += 1;

            let signal_value = self.current_config.compute_signal();
            Ok(NeuralSteeringSignal {
                value: signal_value,
                seq: self.seq_counter,
                source: self.source.clone(),
                delay_ms: current_ms,
                config_version: self.current_config.version,
            })
        }

        /// Rollback to previous config.
        ///
        /// # Errors
        /// * `SteerUIError::RollbackFailed` if no previous config exists.
        pub fn rollback(&mut self) -> Result<NeuralSteerConfig, SteerUIError> {
            match self.previous_config.take() {
                Some(prev) => {
                    self.current_config = prev.clone();
                    self.seq_counter += 1;
                    Ok(prev)
                }
                None => Err(SteerUIError::RollbackFailed),
            }
        }

        /// Get current config.
        pub fn get_config(&self) -> &NeuralSteerConfig {
            &self.current_config
        }

        /// Get current sequence number.
        pub fn get_seq(&self) -> u64 {
            self.seq_counter
        }
    }

    // ============================================================================
    // Utilities
    // ============================================================================

    fn parse_float_field(json: &str, field: &str) -> Result<f32, SteerUIError> {
        let pattern = format!("\"{}\":", field);
        match json.find(&pattern) {
            Some(start) => {
                let value_start = start + pattern.len();
                let rest = &json[value_start..];
                let end = rest
                    .find(|c: char| c == ',' || c == '}')
                    .unwrap_or(rest.len());
                rest[..end]
                    .trim()
                    .parse()
                    .map_err(|_| SteerUIError::SerializationError)
            }
            None => Err(SteerUIError::SerializationError),
        }
    }

    fn parse_uint_field(json: &str, field: &str) -> Result<Option<u32>, SteerUIError> {
        let pattern = format!("\"{}\":", field);
        match json.find(&pattern) {
            Some(start) => {
                let value_start = start + pattern.len();
                let rest = &json[value_start..];
                let end = rest
                    .find(|c: char| c == ',' || c == '}')
                    .unwrap_or(rest.len());
                let val_str = rest[..end].trim();
                if val_str.is_empty() {
                    Ok(None)
                } else {
                    val_str
                        .parse()
                        .map(Some)
                        .map_err(|_| SteerUIError::SerializationError)
                }
            }
            None => Ok(None),
        }
    }

    fn parse_u64_field(json: &str, field: &str) -> Result<Option<u64>, SteerUIError> {
        let pattern = format!("\"{}\":", field);
        match json.find(&pattern) {
            Some(start) => {
                let value_start = start + pattern.len();
                let rest = &json[value_start..];
                let end = rest
                    .find(|c: char| c == ',' || c == '}')
                    .unwrap_or(rest.len());
                let val_str = rest[..end].trim();
                if val_str.is_empty() {
                    Ok(None)
                } else {
                    val_str
                        .parse()
                        .map(Some)
                        .map_err(|_| SteerUIError::SerializationError)
                }
            }
            None => Ok(None),
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_empathy_slider_creation() {
            let slider = SteeringSlider::new(SliderType::Empathy);
            assert_eq!(slider.r#type, SliderType::Empathy);
            assert!((slider.value - 0.0).abs() < 0.01);
            assert!((slider.min_bound - (-0.5)).abs() < 0.01);
            assert!((slider.max_bound - 0.8).abs() < 0.01);
        }

        #[test]
        fn test_creativity_slider_creation() {
            let slider = SteeringSlider::new(SliderType::Creativity);
            assert_eq!(slider.r#type, SliderType::Creativity);
            assert!((slider.min_bound - (-0.3)).abs() < 0.01);
            assert!((slider.max_bound - 0.9).abs() < 0.01);
        }

        #[test]
        fn test_safety_slider_creation() {
            let slider = SteeringSlider::new(SliderType::Safety);
            assert_eq!(slider.r#type, SliderType::Safety);
            assert!((slider.min_bound - 0.2).abs() < 0.01);
            assert!((slider.safety_threshold - 0.2).abs() < 0.01);
        }

        #[test]
        fn test_slider_set_value() {
            let mut slider = SteeringSlider::new(SliderType::Empathy);
            slider.set_value(0.5).unwrap();
            assert!((slider.get_value() - 0.5).abs() < 0.01);
        }

        #[test]
        fn test_slider_value_out_of_bounds() {
            let mut slider = SteeringSlider::new(SliderType::Empathy);
            let err = slider.set_value(1.0).unwrap_err();
            assert_eq!(err, SteerUIError::ValueOutOfBounds);
        }

        #[test]
        fn test_slider_below_min_bound() {
            let mut slider = SteeringSlider::new(SliderType::Empathy);
            let err = slider.set_value(-1.0).unwrap_err();
            assert_eq!(err, SteerUIError::ValueOutOfBounds);
        }

        #[test]
        fn test_safety_threshold_violated() {
            let mut slider = SteeringSlider::new(SliderType::Safety);
            let err = slider.set_value(0.1).unwrap_err();
            // 0.1 < min_bound(0.2) → ValueOutOfBounds checked before SafetyThresholdViolated
            assert_eq!(err, SteerUIError::ValueOutOfBounds);
        }

        #[test]
        fn test_slider_clamp() {
            let mut slider = SteeringSlider::new(SliderType::Empathy);
            slider.value = 1.0;
            slider.clamp();
            assert!((slider.value - 0.8).abs() < 0.01);
        }

        #[test]
        fn test_slider_clamp_below_min() {
            let mut slider = SteeringSlider::new(SliderType::Empathy);
            slider.value = -1.0;
            slider.clamp();
            assert!((slider.value - (-0.5)).abs() < 0.01);
        }

        #[test]
        fn test_slider_is_within_bounds() {
            let slider = SteeringSlider::new(SliderType::Empathy);
            assert!(slider.is_within_bounds());
        }

        #[test]
        fn test_slider_reset() {
            let mut slider = SteeringSlider::new(SliderType::Empathy);
            slider.set_value(0.5).unwrap();
            slider.reset();
            assert!((slider.get_value() - 0.0).abs() < 0.01);
        }

        #[test]
        fn test_safety_slider_reset() {
            let mut slider = SteeringSlider::new(SliderType::Safety);
            slider.set_value(0.8).unwrap();
            slider.reset();
            assert!((slider.get_value() - 0.2).abs() < 0.01);
        }

        #[test]
        fn test_slider_type_display() {
            assert_eq!(format!("{}", SliderType::Empathy), "empathy");
            assert_eq!(format!("{}", SliderType::Creativity), "creativity");
            assert_eq!(format!("{}", SliderType::Safety), "safety");
        }

        #[test]
        fn test_config_new() {
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.6, 1000);
            assert!((config.empathy - 0.3).abs() < 0.01);
            assert!((config.creativity - 0.5).abs() < 0.01);
            assert!((config.safety - 0.6).abs() < 0.01);
            assert_eq!(config.version, 1);
        }

        #[test]
        fn test_config_is_safe() {
            let config = NeuralSteerConfig::new(0.0, 0.0, 0.5, 1000);
            assert!(config.is_safe());
        }

        #[test]
        fn test_config_not_safe() {
            let config = NeuralSteerConfig::new(0.0, 0.0, 0.1, 1000);
            assert!(!config.is_safe());
        }

        #[test]
        fn test_config_compute_signal() {
            let config = NeuralSteerConfig::new(0.0, 0.0, 1.0, 1000);
            // 0.5 * 1.0 + 0.3 * 0.0 + 0.2 * 0.0 = 0.5
            assert!((config.compute_signal() - 0.5).abs() < 0.01);
        }

        #[test]
        fn test_config_bump_version() {
            let mut config = NeuralSteerConfig::new(0.0, 0.0, 0.5, 1000);
            config.bump_version();
            assert_eq!(config.version, 2);
        }

        #[test]
        fn test_config_to_json() {
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.6, 1000);
            let json = config.to_json().unwrap();
            assert!(json.contains("0.3"));
            assert!(json.contains("0.5"));
            assert!(json.contains("0.6"));
        }

        #[test]
        fn test_config_from_json() {
            let json = r#"{"empathy":0.3,"creativity":0.5,"safety":0.6,"version":1,"updated_at_ms":1000}"#;
            let config = NeuralSteerConfig::from_json(json).unwrap();
            assert!((config.empathy - 0.3).abs() < 0.01);
            assert!((config.creativity - 0.5).abs() < 0.01);
            assert!((config.safety - 0.6).abs() < 0.01);
        }

        #[test]
        fn test_config_json_roundtrip() {
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.6, 1000);
            let json = config.to_json().unwrap();
            let restored = NeuralSteerConfig::from_json(&json).unwrap();
            assert!((restored.empathy - config.empathy).abs() < 0.01);
            assert!((restored.creativity - config.creativity).abs() < 0.01);
            assert!((restored.safety - config.safety).abs() < 0.01);
        }

        #[test]
        fn test_bridge_new() {
            let bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            assert_eq!(bridge.get_seq(), 0);
            assert!(bridge.get_config().is_safe());
        }

        #[test]
        fn test_bridge_apply_config() {
            let mut bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.6, 2000);
            let signal = bridge.apply_config(config, 2000).unwrap();
            assert_eq!(signal.seq, 1);
            assert_eq!(signal.source, "test");
            assert!((signal.value - bridge.get_config().compute_signal()).abs() < 0.01);
        }

        #[test]
        fn test_bridge_apply_unsafe_config() {
            let mut bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.1, 2000);
            let err = bridge.apply_config(config, 2000).unwrap_err();
            assert_eq!(err, SteerUIError::SafetyThresholdViolated);
        }

        #[test]
        fn test_bridge_rollback() {
            let mut bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.6, 2000);
            bridge.apply_config(config, 2000).unwrap();
            let rolled = bridge.rollback().unwrap();
            assert!((rolled.empathy - 0.0).abs() < 0.01);
        }

        #[test]
        fn test_bridge_rollback_no_previous() {
            let mut bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            let err = bridge.rollback().unwrap_err();
            assert_eq!(err, SteerUIError::RollbackFailed);
        }

        #[test]
        fn test_bridge_double_rollback() {
            let mut bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            let config = NeuralSteerConfig::new(0.3, 0.5, 0.6, 2000);
            bridge.apply_config(config, 2000).unwrap();
            bridge.rollback().unwrap();
            let err = bridge.rollback().unwrap_err();
            assert_eq!(err, SteerUIError::RollbackFailed);
        }

        #[test]
        fn test_bridge_seq_increments() {
            let mut bridge = SteeringSignalBridge::new("test".to_string(), 1000);
            let config1 = NeuralSteerConfig::new(0.3, 0.5, 0.6, 2000);
            bridge.apply_config(config1, 2000).unwrap();
            let config2 = NeuralSteerConfig::new(0.4, 0.6, 0.7, 3000);
            bridge.apply_config(config2, 3000).unwrap();
            assert_eq!(bridge.get_seq(), 2);
        }

        #[test]
        fn test_error_display() {
            let err = SteerUIError::ValueOutOfBounds;
            assert!(!err.to_string().is_empty());
        }

        #[test]
        fn test_serialization_error() {
            let err = NeuralSteerConfig::from_json("invalid json").unwrap_err();
            assert_eq!(err, SteerUIError::SerializationError);
        }

        #[test]
        fn test_full_ui_lifecycle() {
            // Create sliders
            let mut empathy = SteeringSlider::new(SliderType::Empathy);
            let mut creativity = SteeringSlider::new(SliderType::Creativity);
            let mut safety = SteeringSlider::new(SliderType::Safety);

            // Set values
            empathy.set_value(0.5).unwrap();
            creativity.set_value(0.7).unwrap();
            safety.set_value(0.6).unwrap();

            // Create config
            let config = NeuralSteerConfig::new(
                empathy.get_value(),
                creativity.get_value(),
                safety.get_value(),
                1000,
            );

            // Verify safety
            assert!(config.is_safe());

            // Create bridge and apply
            let mut bridge = SteeringSignalBridge::new("ui-test".to_string(), 1000);
            let signal = bridge.apply_config(config, 1000).unwrap();

            assert_eq!(signal.seq, 1);
            assert!(signal.value >= -1.0 && signal.value <= 1.0);
        }
    }
}

pub use internal::{
    NeuralSteerConfig, NeuralSteeringSignal, SliderType, SteeringSignalBridge, SteeringSlider,
    SteerUIError,
};
