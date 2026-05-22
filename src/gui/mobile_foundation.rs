//! Mobile Foundation v1 — Tauri/React Native bridge (mock + WASM target) for mobile GUI.
//!
//! Provides the foundational abstractions for mobile resource management:
//! - `ResourceSliderConfig` — Thermal/battery-aware resource allocation
//! - `MobileBridge` — Mock bridge for Tauri/React Native communication
//! - `ThermalLimit`, `BatteryLimit` — Hardware constraint enforcement
//!
//! Feature-gated behind `cfg(feature = "v1.9-sprint1")`.

mod internal {
    use std::fmt;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Mobile foundation errors.
    #[derive(Debug, Clone, PartialEq)]
    pub enum MobileError {
        /// Resource allocation exceeds thermal limit.
        ThermalLimitExceeded,
        /// Resource allocation exceeds battery limit.
        BatteryLimitExceeded,
        /// Bridge not available (WASM target not supported).
        BridgeUnavailable,
        /// Invalid slider value (must be in [0.0, 1.0]).
        InvalidSliderValue,
        /// Unknown platform.
        UnknownPlatform,
    }

    impl fmt::Display for MobileError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MobileError::ThermalLimitExceeded => write!(f, "mobile: thermal limit exceeded"),
                MobileError::BatteryLimitExceeded => write!(f, "mobile: battery limit exceeded"),
                MobileError::BridgeUnavailable => write!(f, "mobile: bridge unavailable"),
                MobileError::InvalidSliderValue => {
                    write!(f, "mobile: slider value must be in [0.0, 1.0]")
                }
                MobileError::UnknownPlatform => write!(f, "mobile: unknown platform"),
            }
        }
    }

    impl std::error::Error for MobileError {}

    // ============================================================================
    // Platform
    // ============================================================================

    /// Target platform for the mobile bridge.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Platform {
        /// iOS via React Native.
        Ios,
        /// Android via React Native.
        Android,
        /// Desktop via Tauri.
        Desktop,
        /// WASM target (browser).
        Wasm,
    }

    impl fmt::Display for Platform {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Platform::Ios => write!(f, "ios"),
                Platform::Android => write!(f, "android"),
                Platform::Desktop => write!(f, "desktop"),
                Platform::Wasm => write!(f, "wasm"),
            }
        }
    }

    // ============================================================================
    // Resource Slider Config
    // ============================================================================

    /// Configuration for resource allocation sliders (thermal/battery aware).
    #[derive(Debug, Clone, PartialEq)]
    pub struct ResourceSliderConfig {
        /// CPU allocation slider: 0.0 (min) to 1.0 (max).
        pub cpu_slider: f64,
        /// Memory allocation slider: 0.0 (min) to 1.0 (max).
        pub memory_slider: f64,
        /// Network bandwidth slider: 0.0 (min) to 1.0 (max).
        pub network_slider: f64,
        /// Thermal limit in Celsius (0 = no limit).
        pub thermal_limit_celsius: f64,
        /// Battery percentage threshold (0 = no limit).
        pub battery_threshold_pct: f64,
        /// Current platform target.
        pub platform: Platform,
    }

    impl ResourceSliderConfig {
        /// Create a new resource slider configuration.
        ///
        /// # Arguments
        /// * `cpu_slider` - CPU allocation [0.0, 1.0]
        /// * `memory_slider` - Memory allocation [0.0, 1.0]
        /// * `network_slider` - Network allocation [0.0, 1.0]
        /// * `thermal_limit_celsius` - Thermal limit (0 = no limit)
        /// * `battery_threshold_pct` - Battery threshold (0 = no limit)
        /// * `platform` - Target platform
        ///
        /// # Errors
        /// * `MobileError::InvalidSliderValue` if any slider outside [0.0, 1.0]
        pub fn new(
            cpu_slider: f64,
            memory_slider: f64,
            network_slider: f64,
            thermal_limit_celsius: f64,
            battery_threshold_pct: f64,
            platform: Platform,
        ) -> Result<Self, MobileError> {
            for &slider in &[cpu_slider, memory_slider, network_slider] {
                if slider < 0.0 || slider > 1.0 {
                    return Err(MobileError::InvalidSliderValue);
                }
            }
            Ok(Self {
                cpu_slider,
                memory_slider,
                network_slider,
                thermal_limit_celsius,
                battery_threshold_pct,
                platform,
            })
        }

        /// Check if current thermal reading exceeds the configured limit.
        ///
        /// # Arguments
        /// * `current_temp_celsius` - Current temperature reading
        pub fn check_thermal_limit(&self, current_temp_celsius: f64) -> Result<(), MobileError> {
            if self.thermal_limit_celsius > 0.0 && current_temp_celsius > self.thermal_limit_celsius
            {
                return Err(MobileError::ThermalLimitExceeded);
            }
            Ok(())
        }

        /// Check if current battery level is below the configured threshold.
        ///
        /// # Arguments
        /// * `current_battery_pct` - Current battery percentage
        pub fn check_battery_limit(&self, current_battery_pct: f64) -> Result<(), MobileError> {
            if self.battery_threshold_pct > 0.0 && current_battery_pct < self.battery_threshold_pct
            {
                return Err(MobileError::BatteryLimitExceeded);
            }
            Ok(())
        }

        /// Apply thermal/battery constraints to slider values, returning adjusted config.
        ///
        /// Reduces sliders proportionally when constraints are active.
        ///
        /// # Arguments
        /// * `current_temp_celsius` - Current temperature
        /// * `current_battery_pct` - Current battery level
        pub fn apply_constraints(
            &self,
            current_temp_celsius: f64,
            current_battery_pct: f64,
        ) -> Self {
            let thermal_factor = if self.thermal_limit_celsius > 0.0 {
                let margin = self.thermal_limit_celsius - current_temp_celsius;
                if margin < 0.0 {
                    0.0
                } else {
                    (margin / self.thermal_limit_celsius).clamp(0.0, 1.0)
                }
            } else {
                1.0
            };

            let battery_factor = if self.battery_threshold_pct > 0.0 {
                let margin = current_battery_pct - self.battery_threshold_pct;
                if margin < 0.0 {
                    0.0
                } else {
                    (margin / (100.0 - self.battery_threshold_pct)).clamp(0.0, 1.0)
                }
            } else {
                1.0
            };

            let factor = thermal_factor.min(battery_factor);

            Self {
                cpu_slider: (self.cpu_slider * factor).clamp(0.0, 1.0),
                memory_slider: (self.memory_slider * factor).clamp(0.0, 1.0),
                network_slider: (self.network_slider * factor).clamp(0.0, 1.0),
                thermal_limit_celsius: self.thermal_limit_celsius,
                battery_threshold_pct: self.battery_threshold_pct,
                platform: self.platform.clone(),
            }
        }

        /// Calculate effective resource allocation as a single score [0.0, 1.0].
        pub fn effective_allocation(&self) -> f64 {
            (self.cpu_slider + self.memory_slider + self.network_slider) / 3.0
        }
    }

    impl Default for ResourceSliderConfig {
        fn default() -> Self {
            Self {
                cpu_slider: 0.5,
                memory_slider: 0.5,
                network_slider: 0.5,
                thermal_limit_celsius: 85.0,
                battery_threshold_pct: 20.0,
                platform: Platform::Desktop,
            }
        }
    }

    // ============================================================================
    // Mobile Bridge (Mock)
    // ============================================================================

    /// Mock bridge for Tauri/React Native communication.
    ///
    /// In production, this interfaces with Tauri commands or React Native modules.
    /// For testing and WASM targets, provides mock implementations.
    #[derive(Debug, Clone)]
    pub struct MobileBridge {
        /// Current platform.
        platform: Platform,
        /// Bridge is active (mock state).
        active: bool,
        /// Message log for testing.
        messages: Vec<String>,
    }

    impl MobileBridge {
        /// Create a new mobile bridge for the given platform.
        pub fn new(platform: Platform) -> Self {
            Self {
                platform,
                active: true,
                messages: Vec::new(),
            }
        }

        /// Send a message through the bridge (mock).
        ///
        /// # Arguments
        /// * `channel` - Target channel name
        /// * `payload` - Message payload
        pub fn send(&mut self, channel: &str, payload: &str) -> Result<(), MobileError> {
            if !self.active {
                return Err(MobileError::BridgeUnavailable);
            }
            let msg = format!("[{}] {}", channel, payload);
            self.messages.push(msg);
            Ok(())
        }

        /// Get the message log (for testing).
        pub fn messages(&self) -> &[String] {
            &self.messages
        }

        /// Check if the bridge is active.
        pub fn is_active(&self) -> bool {
            self.active
        }

        /// Deactivate the bridge (for testing).
        pub fn deactivate(&mut self) {
            self.active = false;
        }

        /// Get the current platform.
        pub fn platform(&self) -> &Platform {
            &self.platform
        }

        /// Clear the message log.
        pub fn clear_messages(&mut self) {
            self.messages.clear();
        }
    }

    impl Default for MobileBridge {
        fn default() -> Self {
            Self::new(Platform::Desktop)
        }
    }

    // ============================================================================
    // Resource Manager
    // ============================================================================

    /// Manages resource allocation with thermal/battery awareness.
    #[derive(Debug)]
    pub struct ResourceManager {
        /// Current configuration.
        config: ResourceSliderConfig,
        /// Current temperature reading.
        current_temp_celsius: f64,
        /// Current battery level.
        current_battery_pct: f64,
        /// Allocation history.
        history: Vec<f64>,
    }

    impl ResourceManager {
        /// Create a new resource manager with the given config.
        pub fn new(config: ResourceSliderConfig) -> Self {
            Self {
                config,
                current_temp_celsius: 40.0,
                current_battery_pct: 100.0,
                history: Vec::new(),
            }
        }

        /// Update the current temperature reading.
        pub fn set_temperature(&mut self, temp_celsius: f64) {
            self.current_temp_celsius = temp_celsius;
        }

        /// Update the current battery level.
        pub fn set_battery(&mut self, battery_pct: f64) {
            self.current_battery_pct = battery_pct;
        }

        /// Get the current effective allocation, applying constraints.
        pub fn get_effective_allocation(&self) -> f64 {
            let adjusted = self
                .config
                .apply_constraints(self.current_temp_celsius, self.current_battery_pct);
            adjusted.effective_allocation()
        }

        /// Record an allocation event.
        pub fn record_allocation(&mut self, allocation: f64) {
            self.history.push(allocation);
        }

        /// Get the allocation history.
        pub fn history(&self) -> &[f64] {
            &self.history
        }

        /// Check if current state allows full allocation.
        pub fn allows_full_allocation(&self) -> bool {
            self.config
                .check_thermal_limit(self.current_temp_celsius)
                .is_ok()
                && self
                    .config
                    .check_battery_limit(self.current_battery_pct)
                    .is_ok()
        }
    }

    impl Default for ResourceManager {
        fn default() -> Self {
            Self::new(ResourceSliderConfig::default())
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn test_config() -> ResourceSliderConfig {
            ResourceSliderConfig::new(0.8, 0.6, 0.7, 85.0, 20.0, Platform::Ios).unwrap()
        }

        #[test]
        fn test_config_creation() {
            let config = test_config();
            assert_eq!(config.cpu_slider, 0.8);
            assert_eq!(config.platform, Platform::Ios);
        }

        #[test]
        fn test_config_invalid_slider() {
            assert_eq!(
                ResourceSliderConfig::new(1.5, 0.5, 0.5, 85.0, 20.0, Platform::Android)
                    .unwrap_err(),
                MobileError::InvalidSliderValue
            );
        }

        #[test]
        fn test_thermal_limit_ok() {
            let config = test_config();
            assert!(config.check_thermal_limit(70.0).is_ok());
        }

        #[test]
        fn test_thermal_limit_exceeded() {
            let config = test_config();
            assert_eq!(
                config.check_thermal_limit(90.0).unwrap_err(),
                MobileError::ThermalLimitExceeded
            );
        }

        #[test]
        fn test_battery_limit_ok() {
            let config = test_config();
            assert!(config.check_battery_limit(50.0).is_ok());
        }

        #[test]
        fn test_battery_limit_exceeded() {
            let config = test_config();
            assert_eq!(
                config.check_battery_limit(10.0).unwrap_err(),
                MobileError::BatteryLimitExceeded
            );
        }

        #[test]
        fn test_apply_constraints_no_reduction() {
            let config = test_config();
            let adjusted = config.apply_constraints(40.0, 80.0);
            assert!(adjusted.cpu_slider > 0.0);
        }

        #[test]
        fn test_apply_constraints_thermal_reduction() {
            let config = test_config();
            let adjusted = config.apply_constraints(90.0, 80.0);
            assert!(adjusted.cpu_slider < config.cpu_slider);
        }

        #[test]
        fn test_apply_constraints_battery_reduction() {
            let config = test_config();
            let adjusted = config.apply_constraints(40.0, 10.0);
            assert!(adjusted.cpu_slider < config.cpu_slider);
        }

        #[test]
        fn test_effective_allocation() {
            let config = test_config();
            let alloc = config.effective_allocation();
            assert!((alloc - 0.7).abs() < 0.01);
        }

        #[test]
        fn test_config_default() {
            let config = ResourceSliderConfig::default();
            assert_eq!(config.platform, Platform::Desktop);
            assert_eq!(config.cpu_slider, 0.5);
        }

        #[test]
        fn test_bridge_send() {
            let mut bridge = MobileBridge::new(Platform::Ios);
            assert!(bridge.send("resource", "update").is_ok());
            assert_eq!(bridge.messages().len(), 1);
        }

        #[test]
        fn test_bridge_unavailable() {
            let mut bridge = MobileBridge::new(Platform::Ios);
            bridge.deactivate();
            assert_eq!(
                bridge.send("resource", "update").unwrap_err(),
                MobileError::BridgeUnavailable
            );
        }

        #[test]
        fn test_bridge_platform() {
            let bridge = MobileBridge::new(Platform::Android);
            assert_eq!(bridge.platform(), &Platform::Android);
        }

        #[test]
        fn test_bridge_clear_messages() {
            let mut bridge = MobileBridge::new(Platform::Ios);
            bridge.send("ch", "msg").unwrap();
            bridge.clear_messages();
            assert_eq!(bridge.messages().len(), 0);
        }

        #[test]
        fn test_resource_manager_creation() {
            let manager = ResourceManager::new(test_config());
            assert!(manager.allows_full_allocation());
        }

        #[test]
        fn test_resource_manager_temperature() {
            let mut manager = ResourceManager::default();
            manager.set_temperature(90.0);
            assert!(!manager.allows_full_allocation());
        }

        #[test]
        fn test_resource_manager_battery() {
            let mut manager = ResourceManager::default();
            manager.set_battery(10.0);
            assert!(!manager.allows_full_allocation());
        }

        #[test]
        fn test_resource_manager_record_allocation() {
            let mut manager = ResourceManager::default();
            manager.record_allocation(0.75);
            assert_eq!(manager.history().len(), 1);
        }

        #[test]
        fn test_platform_display() {
            assert_eq!(format!("{}", Platform::Ios), "ios");
            assert_eq!(format!("{}", Platform::Android), "android");
            assert_eq!(format!("{}", Platform::Desktop), "desktop");
            assert_eq!(format!("{}", Platform::Wasm), "wasm");
        }

        #[test]
        fn test_error_display() {
            assert!(!format!("{}", MobileError::ThermalLimitExceeded).is_empty());
            assert!(!format!("{}", MobileError::BatteryLimitExceeded).is_empty());
        }

        #[test]
        fn test_bridge_default() {
            let bridge = MobileBridge::default();
            assert_eq!(bridge.platform(), &Platform::Desktop);
            assert!(bridge.is_active());
        }

        #[test]
        fn test_manager_default() {
            let manager = ResourceManager::default();
            assert!(manager.allows_full_allocation());
        }
    }
}

pub use internal::{MobileBridge, MobileError, Platform, ResourceManager, ResourceSliderConfig};
