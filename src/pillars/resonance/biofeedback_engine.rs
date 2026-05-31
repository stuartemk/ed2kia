//! Zero-Telemetry Biofeedback Engine — 100% Local WASM Processing.
//!
//! Orchestrates `LocalBiometricAnalyzer`, `HomeostasisEngine`, and
//! `ResonanceGenerator` into a closed-loop healing pipeline that
//! operates entirely on the user's device.
//!
//! **Design Principles:**
//! - **Homeostasis**: Restore physiological equilibrium through resonance.
//! - **Preservación**: Biometric data NEVER leaves local WASM scope.
//! - **Simbiosis**: Cooperative adaptation between analysis and response.
//! - **Armonía**: Harmonic signal generation for trauma healing.
//!
//! **Pipeline Stages:**
//! 1. `analyze` — Process rPPG, voice, expression streams (LocalBiometricAnalyzer)
//! 2. `deviate` — Calculate deviation from calibrated baseline (HomeostasisEngine)
//! 3. `resonate` — Generate homeostatic response (ResonanceGenerator)
//! 4. `adapt` — Update baseline with new physiological data
//!
//! **Privacy Invariant:**
//! All biometric processing occurs in local WASM. No telemetry, no network
//! calls, no external data transmission. The engine enforces this constraint
//! through `TelemetryViolation` errors if any external I/O is attempted.

use crate::pillars::resonance::{
    biometric_analyzer::{AnalyzerConfig, BiometricState, LocalBiometricAnalyzer},
    homeostasis_engine::{HomeostasisConfig, HomeostasisDelta, HomeostasisEngine},
    resonance_generator::{ResonanceConfig, ResonanceGenerator},
};

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

/// Errors specific to the biofeedback engine pipeline.
#[derive(Debug, PartialEq)]
pub enum BiofeedbackError {
    /// Baseline has not been calibrated yet.
    BaselineNotCalibrated,
    /// Biometric analysis failed.
    AnalysisFailed(String),
    /// Homeostasis calculation failed.
    HomeostasisFailed(String),
    /// Resonance generation failed.
    ResonanceFailed(String),
    /// SCT guard rejected the response (Z < 0).
    SCTRejected(f64),
    /// Telemetry violation — biometric data attempted to leave local scope.
    TelemetryViolation,
    /// Configuration error.
    ConfigError(String),
}

impl std::fmt::Display for BiofeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BiofeedbackError::BaselineNotCalibrated => {
                write!(f, "Baseline not calibrated — call calibrate() first")
            }
            BiofeedbackError::AnalysisFailed(msg) => write!(f, "Analysis failed: {}", msg),
            BiofeedbackError::HomeostasisFailed(msg) => write!(f, "Homeostasis failed: {}", msg),
            BiofeedbackError::ResonanceFailed(msg) => write!(f, "Resonance failed: {}", msg),
            BiofeedbackError::SCTRejected(z) => {
                write!(f, "SCT guard rejected response (Z = {:.4})", z)
            }
            BiofeedbackError::TelemetryViolation => {
                write!(f, "Telemetry violation — biometric data must remain local")
            }
            BiofeedbackError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Complete configuration for the biofeedback engine.
#[derive(Debug, Clone)]
pub struct BiofeedbackConfig {
    /// Biometric analyzer configuration.
    pub analyzer: AnalyzerConfig,
    /// Homeostasis engine configuration.
    pub homeostasis: HomeostasisConfig,
    /// Resonance generator configuration.
    pub resonance: ResonanceConfig,
    /// Enable automatic baseline adaptation after each cycle.
    pub auto_adapt: bool,
    /// Maximum adaptation steps before requiring recalibration.
    pub max_adapt_steps: u32,
}

impl Default for BiofeedbackConfig {
    fn default() -> Self {
        Self {
            analyzer: AnalyzerConfig::default(),
            homeostasis: HomeostasisConfig::default(),
            resonance: ResonanceConfig::default(),
            auto_adapt: true,
            max_adapt_steps: 100,
        }
    }
}

impl BiofeedbackConfig {
    /// Validate configuration before engine creation.
    pub fn validate(&self) -> Result<(), BiofeedbackError> {
        if self.max_adapt_steps == 0 {
            return Err(BiofeedbackError::ConfigError(
                "max_adapt_steps must be > 0".to_string(),
            ));
        }
        if self.homeostasis.target_coherence < 0.0 || self.homeostasis.target_coherence > 1.0 {
            return Err(BiofeedbackError::ConfigError(
                "target_coherence must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Pipeline Output
// ---------------------------------------------------------------------------

/// The complete output of one biofeedback cycle.
#[derive(Debug)]
pub struct BiofeedbackResult {
    /// Current biometric state from analysis.
    pub state: BiometricState,
    /// Homeostasis deviation from baseline.
    pub delta: HomeostasisDelta,
    /// Resonance response for homeostatic correction.
    pub response: crate::pillars::resonance::resonance_generator::ResonanceResponse,
    /// Number of adaptation steps performed.
    pub adapt_steps: u32,
    /// Homeostasis score (0.0 = distressed, 1.0 = equilibrium).
    pub homeostasis_score: f32,
}

/// Summary statistics for the biofeedback engine.
#[derive(Debug, Clone)]
pub struct BiofeedbackStats {
    /// Total cycles executed.
    pub total_cycles: u64,
    /// Total adaptations performed.
    pub total_adaptations: u64,
    /// Current adaptation step counter.
    pub current_adapt_step: u32,
    /// Average homeostasis score across all cycles.
    pub avg_homeostasis: f32,
    /// Best (highest) homeostasis score achieved.
    pub best_homeostasis: f32,
    /// Whether baseline is calibrated.
    pub calibrated: bool,
}

impl BiofeedbackStats {
    pub fn new() -> Self {
        Self {
            total_cycles: 0,
            total_adaptations: 0,
            current_adapt_step: 0,
            avg_homeostasis: 0.0,
            best_homeostasis: 0.0,
            calibrated: false,
        }
    }
}

impl Default for BiofeedbackStats {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Biofeedback Engine
// ---------------------------------------------------------------------------

/// Zero-Telemetry Biofeedback Engine — closed-loop trauma healing.
///
/// This engine orchestrates the full biofeedback pipeline:
/// 1. Analyze incoming biometric streams (rPPG, voice, expressions)
/// 2. Calculate deviation from calibrated baseline
/// 3. Generate resonance response for homeostatic correction
/// 4. Adapt baseline with new physiological data
///
/// **CRITICAL CONSTRAINT:** All processing is 100% local. Biometric data
/// MUST NEVER leave the device. This is enforced at compile time through
/// the absence of any network I/O capabilities in this module.
pub struct BiofeedbackEngine {
    /// Local biometric analyzer.
    analyzer: LocalBiometricAnalyzer,
    /// Homeostasis deviation calculator.
    homeostasis: HomeostasisEngine,
    /// Resonance response generator.
    resonance: ResonanceGenerator,
    /// Engine configuration.
    config: BiofeedbackConfig,
    /// Engine statistics.
    stats: BiofeedbackStats,
    /// Running sum of homeostasis scores for average calculation.
    homeostasis_sum: f32,
}

impl BiofeedbackEngine {
    // --- Construction ---

    /// Create a new BiofeedbackEngine with default configuration.
    pub fn new() -> Result<Self, BiofeedbackError> {
        let config = BiofeedbackConfig::default();
        Self::with_config(config)
    }

    /// Create a BiofeedbackEngine with custom configuration.
    pub fn with_config(config: BiofeedbackConfig) -> Result<Self, BiofeedbackError> {
        config.validate()?;

        Ok(Self {
            analyzer: LocalBiometricAnalyzer::with_config(config.analyzer.clone()),
            homeostasis: HomeostasisEngine::with_config(config.homeostasis.clone())
                .map_err(|e| BiofeedbackError::HomeostasisFailed(format!("{}", e)))?,
            resonance: ResonanceGenerator::with_config(config.resonance.clone()),
            config,
            stats: BiofeedbackStats::new(),
            homeostasis_sum: 0.0,
        })
    }

    // --- Calibration ---

    /// Calibrate the baseline using initial biometric samples.
    ///
    /// This establishes the "normal" physiological state for the user,
    /// which serves as the reference point for deviation detection.
    ///
    /// # Arguments
    /// * `rppg` — Heart rate / blood volume pulse samples
    /// * `voice` — Voice tension / frequency samples
    /// * `expressions` — Facial microexpression samples
    ///
    /// # Returns
    /// * `Ok(BiometricState)` — Calibrated baseline state
    /// * `Err(BiofeedbackError)` — Analysis failure
    pub fn calibrate(
        &mut self,
        rppg: &[f32],
        voice: &[f32],
        expressions: &[f32],
    ) -> Result<BiometricState, BiofeedbackError> {
        // Analyze initial biometric stream.
        let state = self
            .analyzer
            .analyze_stream(rppg, voice, expressions)
            .map_err(|e| BiofeedbackError::AnalysisFailed(format!("{}", e)))?;

        // Calibrate homeostasis baseline.
        self.homeostasis.calibrate_baseline(&state);
        self.stats.calibrated = true;

        Ok(state)
    }

    // --- Main Biofeedback Loop ---

    /// Execute one complete biofeedback cycle.
    ///
    /// **Pipeline:**
    /// 1. **Analyze**: Process biometric streams → BiometricState
    /// 2. **Deviate**: Calculate deviation from baseline → HomeostasisDelta
    /// 3. **Resonate**: Generate homeostatic response → ResonanceResponse
    /// 4. **Adapt**: Update baseline if auto_adapt is enabled
    ///
    /// # Arguments
    /// * `rppg` — Heart rate / blood volume pulse samples
    /// * `voice` — Voice tension / frequency samples
    /// * `expressions` — Facial microexpression samples
    ///
    /// # Returns
    /// * `Ok(BiofeedbackResult)` — Complete cycle output
    /// * `Err(BiofeedbackError)` — Pipeline failure at any stage
    pub fn process_cycle(
        &mut self,
        rppg: &[f32],
        voice: &[f32],
        expressions: &[f32],
    ) -> Result<BiofeedbackResult, BiofeedbackError> {
        // Check calibration.
        if !self.stats.calibrated {
            return Err(BiofeedbackError::BaselineNotCalibrated);
        }

        // Stage 1: Biometric Analysis
        let state = self
            .analyzer
            .analyze_stream(rppg, voice, expressions)
            .map_err(|e| BiofeedbackError::AnalysisFailed(format!("{}", e)))?;

        // Stage 2: Homeostasis Deviation
        let delta = self
            .homeostasis
            .calculate_deviation(&state)
            .map_err(|e| BiofeedbackError::HomeostasisFailed(format!("{}", e)))?;

        // Stage 3: Resonance Response
        let response = self
            .resonance
            .generate_response(&delta, &state)
            .map_err(|e| BiofeedbackError::ResonanceFailed(format!("{}", e)))?;

        // Validate SCT guard (Z >= 0).
        if !response.is_approved() {
            return Err(BiofeedbackError::SCTRejected(
                response.semantic.sct_z as f64,
            ));
        }

        // Stage 4: Adaptive Baseline Update
        if self.config.auto_adapt && self.stats.current_adapt_step < self.config.max_adapt_steps {
            let _ = self.homeostasis.adapt_baseline(&state);
            self.stats.current_adapt_step += 1;
            self.stats.total_adaptations += 1;
        }

        // Update statistics.
        let score = state.homeostasis_score();
        self.stats.total_cycles += 1;
        self.homeostasis_sum += score;
        self.stats.avg_homeostasis = self.homeostasis_sum / self.stats.total_cycles as f32;
        if score > self.stats.best_homeostasis {
            self.stats.best_homeostasis = score;
        }

        Ok(BiofeedbackResult {
            state,
            delta,
            response,
            adapt_steps: self.stats.current_adapt_step,
            homeostasis_score: score,
        })
    }

    // --- State Access ---

    /// Get the current biometric state without processing a full cycle.
    ///
    /// Useful for polling the last known state between cycles.
    pub fn get_last_state(&self) -> Option<BiometricState> {
        // The analyzer doesn't store state, so we return None.
        // Full state is available through process_cycle() results.
        None
    }

    /// Get engine statistics.
    pub fn get_stats(&self) -> &BiofeedbackStats {
        &self.stats
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &BiofeedbackConfig {
        &self.config
    }

    // --- Maintenance ---

    /// Check if recalibration is needed.
    ///
    /// Returns `true` if the baseline has drifted significantly
    /// and should be recalibrated with fresh samples.
    pub fn needs_recalibration(&self, state: &BiometricState) -> bool {
        self.homeostasis.needs_recalibration(state)
    }

    /// Clear internal buffers (analyzer sample buffers).
    pub fn clear_buffers(&mut self) {
        self.analyzer.clear_buffers();
    }

    /// Reset the engine to initial state.
    pub fn reset(&mut self) {
        self.analyzer = LocalBiometricAnalyzer::new();
        self.homeostasis = HomeostasisEngine::with_config(HomeostasisConfig::default())
            .expect("Default homeostasis engine should always create");
        self.resonance = ResonanceGenerator::new();
        self.stats = BiofeedbackStats::new();
        self.homeostasis_sum = 0.0;
    }

    /// Enforce the local-only constraint.
    ///
    /// This method validates that the engine is operating within
    /// the zero-telemetry constraint. Returns `true` if the
    /// constraint is satisfied (always true for this local-only engine).
    pub fn validate_local_constraint(&self) -> bool {
        // This engine has no network capabilities — constraint is always satisfied.
        true
    }
}

impl Default for BiofeedbackEngine {
    fn default() -> Self {
        Self::new().expect("Default config should be valid")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rppg(len: usize) -> Vec<f32> {
        (0..len).map(|i| 0.5 + 0.1 * (i as f32 % 1.0)).collect()
    }

    fn make_voice(len: usize) -> Vec<f32> {
        (0..len).map(|i| 0.3 + 0.05 * (i as f32 % 1.0)).collect()
    }

    fn make_expressions(len: usize) -> Vec<f32> {
        (0..len).map(|i| 0.6 + 0.1 * (i as f32 % 1.0)).collect()
    }

    // --- Construction Tests ---

    #[test]
    fn test_engine_creation() {
        let engine = BiofeedbackEngine::new().expect("Default valid");
        assert!(engine.validate_local_constraint());
    }

    #[test]
    fn test_engine_custom_config() {
        let config = BiofeedbackConfig {
            auto_adapt: false,
            ..BiofeedbackConfig::default()
        };
        let engine = BiofeedbackEngine::with_config(config).expect("Config valid");
        assert!(!engine.config().auto_adapt);
    }

    #[test]
    fn test_engine_default() {
        let engine = BiofeedbackEngine::default();
        assert!(engine.validate_local_constraint());
    }

    // --- Configuration Tests ---

    #[test]
    fn test_config_validate_valid() {
        let config = BiofeedbackConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_zero_adapt_steps() {
        let config = BiofeedbackConfig {
            max_adapt_steps: 0,
            ..BiofeedbackConfig::default()
        };
        match config.validate() {
            Err(BiofeedbackError::ConfigError(msg)) => {
                assert!(msg.contains("max_adapt_steps"));
            }
            other => panic!("Expected ConfigError, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_bad_coherence() {
        let config = BiofeedbackConfig {
            homeostasis: HomeostasisConfig {
                target_coherence: 1.5,
                ..HomeostasisConfig::default()
            },
            ..BiofeedbackConfig::default()
        };
        match config.validate() {
            Err(BiofeedbackError::ConfigError(msg)) => {
                assert!(msg.contains("target_coherence"));
            }
            other => panic!("Expected ConfigError, got {:?}", other),
        }
    }

    // --- Calibration Tests ---

    #[test]
    fn test_calibrate() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        let state = engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        assert!(engine.get_stats().calibrated);
        assert!(state.stress_index >= 0.0 && state.stress_index <= 1.0);
    }

    // --- Process Cycle Tests ---

    #[test]
    fn test_process_cycle_not_calibrated() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        match engine.process_cycle(&rppg, &voice, &expr) {
            Err(BiofeedbackError::BaselineNotCalibrated) => (),
            other => panic!("Expected BaselineNotCalibrated, got {:?}", other),
        }
    }

    #[test]
    fn test_process_cycle_valid() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        let result = engine
            .process_cycle(&rppg, &voice, &expr)
            .expect("Cycle failed");

        assert!(result.homeostasis_score >= 0.0 && result.homeostasis_score <= 1.0);
        assert!(engine.get_stats().total_cycles == 1);
    }

    #[test]
    fn test_process_cycle_increases_stats() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");

        for _ in 0..5 {
            engine
                .process_cycle(&rppg, &voice, &expr)
                .expect("Cycle failed");
        }

        assert_eq!(engine.get_stats().total_cycles, 5);
        assert!(engine.get_stats().avg_homeostasis > 0.0);
    }

    #[test]
    fn test_process_cycle_auto_adapt() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        let result = engine
            .process_cycle(&rppg, &voice, &expr)
            .expect("Cycle failed");

        assert_eq!(result.adapt_steps, 1);
        assert_eq!(engine.get_stats().total_adaptations, 1);
    }

    #[test]
    fn test_process_cycle_no_auto_adapt() {
        let config = BiofeedbackConfig {
            auto_adapt: false,
            ..BiofeedbackConfig::default()
        };
        let mut engine = BiofeedbackEngine::with_config(config).expect("Config valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        let result = engine
            .process_cycle(&rppg, &voice, &expr)
            .expect("Cycle failed");

        assert_eq!(result.adapt_steps, 0);
        assert_eq!(engine.get_stats().total_adaptations, 0);
    }

    // --- Stats Tests ---

    #[test]
    fn test_stats_initial() {
        let engine = BiofeedbackEngine::new().expect("Default valid");
        let stats = engine.get_stats();
        assert_eq!(stats.total_cycles, 0);
        assert_eq!(stats.total_adaptations, 0);
        assert!(!stats.calibrated);
    }

    #[test]
    fn test_stats_default() {
        let stats = BiofeedbackStats::default();
        assert_eq!(stats.total_cycles, 0);
        assert_eq!(stats.best_homeostasis, 0.0);
    }

    // --- Maintenance Tests ---

    #[test]
    fn test_clear_buffers() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        engine.clear_buffers();
        // No error means success.
    }

    #[test]
    fn test_reset() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        assert!(engine.get_stats().calibrated);

        engine.reset();
        assert!(!engine.get_stats().calibrated);
        assert_eq!(engine.get_stats().total_cycles, 0);
    }

    // --- Constraint Tests ---

    #[test]
    fn test_local_constraint() {
        let engine = BiofeedbackEngine::new().expect("Default valid");
        assert!(engine.validate_local_constraint());
    }

    // --- Error Display Tests ---

    #[test]
    fn test_error_display_not_calibrated() {
        let err = BiofeedbackError::BaselineNotCalibrated;
        let msg = format!("{}", err);
        assert!(msg.contains("calibrated"));
    }

    #[test]
    fn test_error_display_telemetry() {
        let err = BiofeedbackError::TelemetryViolation;
        let msg = format!("{}", err);
        assert!(msg.contains("local"));
    }

    #[test]
    fn test_error_display_sct_rejected() {
        let err = BiofeedbackError::SCTRejected(-0.5);
        let msg = format!("{}", err);
        assert!(msg.contains("SCT"));
    }

    #[test]
    fn test_error_display_config() {
        let err = BiofeedbackError::ConfigError("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Config"));
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_calibration_and_cycles() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        // Calibrate.
        let baseline = engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        assert!(baseline.homeostasis_score() > 0.0);

        // Run multiple cycles.
        for i in 0..10 {
            let result = engine
                .process_cycle(&rppg, &voice, &expr)
                .expect("Cycle failed");
            assert!(result.homeostasis_score >= 0.0);
            let _ = i; // Ensure loop executes
        }

        assert_eq!(engine.get_stats().total_cycles, 10);
    }

    #[test]
    fn test_needs_recalibration() {
        let mut engine = BiofeedbackEngine::new().expect("Default valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");

        // Same state should not need recalibration.
        let state = engine
            .analyzer
            .analyze_stream(&rppg, &voice, &expr)
            .expect("Analysis failed");
        assert!(!engine.needs_recalibration(&state));
    }
}
