//! Homeostasis Engine — Local Physiological Equilibrium Manager.
//!
//! Maintains baseline biometric state and calculates deviation deltas
//! for homeostasis restoration. Integrates SCT Guard for ethical validation
//! of homeostasis corrections.
//!
//! **Ethical Invariant:** SCT Guard (Stuartian Context Tensor) validates all
//! homeostasis corrections. If Z < 0 (ethical rejection), the engine returns
//! `EthicalRejection` and suggests local recalibration.
//!
//! **Feature Gate:** `v3.0-resonance-interface`

use thiserror::Error;
use crate::pillars::resonance::biometric_analyzer::{BiometricState, AnalyzerError};

/// Errors specific to homeostasis engine operations.
#[derive(Debug, Error, Clone)]
pub enum EngineError {
    #[error("Baseline not calibrated: call `calibrate_baseline()` first")]
    BaselineNotCalibrated,

    #[error("Ethical rejection: SCT Z = {z:.3} (threshold 0.0) — recalibrate locally")]
    EthicalRejection { z: f32 },

    #[error("Invalid adaptation rate: {rate} (expected range [0.0, 1.0])")]
    InvalidAdaptationRate { rate: f32 },

    #[error("Invalid target coherence: {coherence} (expected range [0.0, 1.0])")]
    InvalidTargetCoherence { coherence: f32 },

    #[error("Biometric analysis failed: {0}")]
    AnalysisFailed(#[from] AnalyzerError),

    #[error("Telemetry violation: homeostasis data must remain LOCAL_ONLY")]
    TelemetryViolation,
}

/// Homeostasis deviation delta — difference between current state and baseline.
///
/// Represents the physiological and ethical deviation from the calibrated baseline.
/// Used to determine resonance correction magnitude and direction.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HomeostasisDelta {
    /// Stress deviation: positive = more stressed than baseline
    pub stress_delta: f32,
    /// Coherence deviation: positive = more coherent than baseline
    pub coherence_delta: f32,
    /// Frequency deviation in Hz
    pub frequency_delta: f32,
    /// Valence deviation
    pub valence_delta: f32,
    /// Arousal deviation
    pub arousal_delta: f32,
    /// Overall homeostasis score (0.0 = chaotic, 1.0 = equilibrium)
    pub homeostasis_score: f32,
    /// SCT Z-axis value for ethical validation (>= 0 = approved)
    pub sct_z: f32,
    /// Recommended correction magnitude (0.0 = none, 1.0 = full correction)
    pub correction_magnitude: f32,
}

impl HomeostasisDelta {
    /// Calculate the magnitude of correction needed.
    /// Higher values indicate greater deviation from homeostasis.
    pub fn deviation_magnitude(&self) -> f32 {
        (self.stress_delta.abs()
            + self.coherence_delta.abs()
            + self.frequency_delta.abs()
            + self.valence_delta.abs()
            + self.arousal_delta.abs())
            / 5.0
    }

    /// Check if SCT Guard approves this delta (Z >= 0).
    pub fn is_ethically_approved(&self) -> bool {
        self.sct_z >= 0.0
    }

    /// Determine if correction is needed (deviation exceeds threshold).
    pub fn needs_correction(&self, threshold: f32) -> bool {
        self.is_ethically_approved() && self.deviation_magnitude() > threshold
    }
}

/// Configuration for the Homeostasis Engine.
#[derive(Debug, Clone)]
pub struct HomeostasisConfig {
    /// Target coherence level (0.0 = chaotic, 1.0 = fully coherent).
    pub target_coherence: f32,
    /// Adaptation rate for baseline updates (0.0 = no adaptation, 1.0 = full adaptation).
    pub adaptation_rate: f32,
    /// SCT Z-axis threshold for ethical approval (default 0.0).
    pub sct_z_threshold: f32,
    /// Correction threshold — delta magnitude above which correction is triggered.
    pub correction_threshold: f32,
    /// Maximum baseline drift allowed before recalibration is required.
    pub max_baseline_drift: f32,
}

impl Default for HomeostasisConfig {
    fn default() -> Self {
        Self {
            target_coherence: 0.8,
            adaptation_rate: 0.1,
            sct_z_threshold: 0.0,
            correction_threshold: 0.15,
            max_baseline_drift: 0.3,
        }
    }
}

/// Homeostasis Engine — maintains physiological equilibrium 100% locally.
///
/// Tracks baseline biometric state and calculates deviations for resonance correction.
/// Integrates SCT Guard to ensure ethical validation of all corrections.
///
/// **⚠️ LOCAL_ONLY:** All homeostasis data processed and discarded locally.
pub struct HomeostasisEngine {
    config: HomeostasisConfig,
    baseline: Option<BiometricState>,
    /// Running average of recent states for drift detection.
    recent_states: Vec<BiometricState>,
    /// Maximum number of recent states to track.
    max_recent_states: usize,
}

impl HomeostasisEngine {
    /// Create a new engine with default configuration.
    pub fn new() -> Result<Self, EngineError> {
        Self::with_config(HomeostasisConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: HomeostasisConfig) -> Result<Self, EngineError> {
        if !(0.0..=1.0).contains(&config.target_coherence) {
            return Err(EngineError::InvalidTargetCoherence {
                coherence: config.target_coherence,
            });
        }
        if !(0.0..=1.0).contains(&config.adaptation_rate) {
            return Err(EngineError::InvalidAdaptationRate {
                rate: config.adaptation_rate,
            });
        }
        Ok(Self {
            config,
            baseline: None,
            recent_states: Vec::new(),
            max_recent_states: 50,
        })
    }

    /// Calibrate baseline from current biometric state.
    ///
    /// This should be called during initial setup or periodic recalibration.
    pub fn calibrate_baseline(&mut self, state: &BiometricState) {
        self.baseline = Some(*state);
        self.recent_states.clear();
        self.recent_states.push(*state);
    }

    /// Record a new biometric state for drift tracking.
    pub fn record_state(&mut self, state: &BiometricState) {
        self.recent_states.push(*state);
        if self.recent_states.len() > self.max_recent_states {
            self.recent_states.remove(0);
        }
    }

    /// Calculate deviation from baseline with SCT Guard validation.
    ///
    /// Compares current state against calibrated baseline and calculates
    /// ethical/physiological deviation. Returns `EthicalRejection` if
    /// SCT Z < 0 (e.g., chronic stress patterns or external manipulation).
    pub fn calculate_deviation(&self, current: &BiometricState) -> Result<HomeostasisDelta, EngineError> {
        let baseline = self.baseline.ok_or(EngineError::BaselineNotCalibrated)?;

        // Calculate raw deltas
        let stress_delta = current.stress_index - baseline.stress_index;
        let coherence_delta = current.coherence - baseline.coherence;
        let frequency_delta = current.dominant_frequency - baseline.dominant_frequency;
        let valence_delta = current.valence - baseline.valence;
        let arousal_delta = current.arousal - baseline.arousal;

        // Calculate homeostasis score from current state
        let homeostasis_score = current.homeostasis_score();

        // SCT Guard evaluation: calculate Z-axis for ethical validation
        // Z = (coherence_gain - stress_reduction_cost) * valence_factor
        // Positive Z = ethical (benefit > cost), Negative Z = unethical (cost > benefit)
        let coherence_gain = if coherence_delta > 0.0 {
            coherence_delta
        } else {
            0.0
        };
        let stress_reduction_cost = if stress_delta < 0.0 {
            -stress_delta
        } else {
            0.0
        };
        let valence_factor = (current.valence + 1.0) / 2.0; // Normalize to [0, 1]

        // Z = sigmoid-like function: positive when benefit > cost
        let z_raw = (coherence_gain - stress_reduction_cost) * valence_factor;
        let sct_z = z_raw.tanh(); // Clamp to [-1, 1]

        // SCT Guard check: reject if Z < threshold
        if sct_z < self.config.sct_z_threshold {
            return Err(EngineError::EthicalRejection { z: sct_z });
        }

        // Calculate correction magnitude based on deviation from target
        let coherence_error = self.config.target_coherence - current.coherence;
        let stress_error = current.stress_index; // Lower is better
        let correction_magnitude = ((coherence_error.abs() + stress_error) / 2.0).clamp(0.0, 1.0);

        Ok(HomeostasisDelta {
            stress_delta,
            coherence_delta,
            frequency_delta,
            valence_delta,
            arousal_delta,
            homeostasis_score,
            sct_z,
            correction_magnitude,
        })
    }

    /// Adapt baseline gradually toward current state (for long-term acclimatization).
    ///
    /// Uses the configured adaptation rate to blend baseline with current state.
    /// Only adapts if SCT Guard approves.
    pub fn adapt_baseline(&mut self, current: &BiometricState) -> Result<(), EngineError> {
        let delta = self.calculate_deviation(current)?;
        let baseline = self.baseline.ok_or(EngineError::BaselineNotCalibrated)?;

        let rate = self.config.adaptation_rate;
        let adapted = BiometricState::new(
            baseline.stress_index + delta.stress_delta * rate,
            baseline.coherence + delta.coherence_delta * rate,
            baseline.dominant_frequency + delta.frequency_delta * rate,
            baseline.valence + delta.valence_delta * rate,
            baseline.arousal + delta.arousal_delta * rate,
        )?;

        self.baseline = Some(adapted);
        Ok(())
    }

    /// Check if baseline has drifted beyond acceptable limits.
    ///
    /// Returns `true` if recalibration is recommended.
    pub fn needs_recalibration(&self, current: &BiometricState) -> bool {
        let baseline = match self.baseline {
            Some(b) => b,
            None => return true,
        };

        let drift = (
            (current.stress_index - baseline.stress_index).abs(),
            (current.coherence - baseline.coherence).abs(),
            (current.valence - baseline.valence).abs(),
        );

        let avg_drift = (drift.0 + drift.1 + drift.2) / 3.0;
        avg_drift > self.config.max_baseline_drift
    }

    /// Get current baseline state (if calibrated).
    pub fn get_baseline(&self) -> Option<BiometricState> {
        self.baseline
    }

    /// Get current configuration.
    pub fn config(&self) -> &HomeostasisConfig {
        &self.config
    }

    /// Reset engine to uncalibrated state.
    pub fn reset(&mut self) {
        self.baseline = None;
        self.recent_states.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_calm_state() -> BiometricState {
        BiometricState::new(0.1, 0.9, 1.0, 0.5, 0.3).unwrap()
    }

    fn make_stressed_state() -> BiometricState {
        BiometricState::new(0.8, 0.2, 2.5, -0.3, 0.9).unwrap()
    }

    fn make_coherent_state() -> BiometricState {
        BiometricState::new(0.2, 0.95, 1.2, 0.7, 0.5).unwrap()
    }

    #[test]
    fn test_engine_creation() {
        let engine = HomeostasisEngine::new().unwrap();
        assert!(engine.get_baseline().is_none());
    }

    #[test]
    fn test_engine_custom_config() {
        let config = HomeostasisConfig {
            target_coherence: 0.9,
            adaptation_rate: 0.2,
            ..Default::default()
        };
        let engine = HomeostasisEngine::with_config(config).unwrap();
        assert_eq!(engine.config().target_coherence, 0.9);
        assert_eq!(engine.config().adaptation_rate, 0.2);
    }

    #[test]
    fn test_invalid_target_coherence() {
        let config = HomeostasisConfig {
            target_coherence: 1.5,
            ..Default::default()
        };
        match HomeostasisEngine::with_config(config) {
            Err(EngineError::InvalidTargetCoherence { coherence }) => assert_eq!(coherence, 1.5),
            _ => panic!("Expected InvalidTargetCoherence"),
        }
    }

    #[test]
    fn test_invalid_adaptation_rate() {
        let config = HomeostasisConfig {
            adaptation_rate: -0.1,
            ..Default::default()
        };
        match HomeostasisEngine::with_config(config) {
            Err(EngineError::InvalidAdaptationRate { rate }) => assert_eq!(rate, -0.1),
            _ => panic!("Expected InvalidAdaptationRate"),
        }
    }

    #[test]
    fn test_calibrate_baseline() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let state = make_calm_state();
        engine.calibrate_baseline(&state);
        assert_eq!(engine.get_baseline(), Some(state));
    }

    #[test]
    fn test_deviation_uncalibrated() {
        let engine = HomeostasisEngine::new().unwrap();
        match engine.calculate_deviation(&make_calm_state()) {
            Err(EngineError::BaselineNotCalibrated) => {}
            _ => panic!("Expected BaselineNotCalibrated"),
        }
    }

    #[test]
    fn test_deviation_same_state() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let state = make_calm_state();
        engine.calibrate_baseline(&state);

        let delta = engine.calculate_deviation(&state).unwrap();
        assert!((delta.stress_delta - 0.0).abs() < f32::EPSILON);
        assert!((delta.coherence_delta - 0.0).abs() < f32::EPSILON);
        assert!(delta.is_ethically_approved());
    }

    #[test]
    fn test_deviation_stressed_state() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        let stressed = make_stressed_state();
        let delta = engine.calculate_deviation(&stressed).unwrap();
        assert!(delta.stress_delta > 0.0, "Stress should increase");
        assert!(delta.coherence_delta < 0.0, "Coherence should decrease");
    }

    #[test]
    fn test_deviation_coherent_state() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let stressed = make_stressed_state();
        engine.calibrate_baseline(&stressed);

        let coherent = make_coherent_state();
        let delta = engine.calculate_deviation(&coherent).unwrap();
        assert!(delta.stress_delta < 0.0, "Stress should decrease");
        assert!(delta.coherence_delta > 0.0, "Coherence should increase");
        assert!(delta.is_ethically_approved());
    }

    #[test]
    fn test_sct_guard_rejection() {
        let mut engine = HomeostasisEngine::new().unwrap();
        // Set high SCT threshold to force rejection
        engine.config.sct_z_threshold = 1.0;

        let state = make_calm_state();
        engine.calibrate_baseline(&state);

        // A stressed state with negative valence should have low Z
        let stressed = make_stressed_state();
        match engine.calculate_deviation(&stressed) {
            Err(EngineError::EthicalRejection { z }) => assert!(z < 1.0),
            _ => panic!("Expected EthicalRejection"),
        }
    }

    #[test]
    fn test_adapt_baseline() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        let slightly_stressed = BiometricState::new(0.3, 0.7, 1.5, 0.3, 0.6).unwrap();
        engine.adapt_baseline(&slightly_stressed).unwrap();

        let adapted = engine.get_baseline().unwrap();
        // Baseline should have shifted slightly toward stressed state
        assert!(adapted.stress_index > calm.stress_index);
        assert!(adapted.coherence < calm.coherence);
    }

    #[test]
    fn test_adapt_baseline_rejected() {
        let mut engine = HomeostasisEngine::new().unwrap();
        engine.config.sct_z_threshold = 1.0;

        let state = make_calm_state();
        engine.calibrate_baseline(&state);

        let stressed = make_stressed_state();
        match engine.adapt_baseline(&stressed) {
            Err(EngineError::EthicalRejection { .. }) => {}
            _ => panic!("Expected EthicalRejection"),
        }
    }

    #[test]
    fn test_needs_recalibration_uncalibrated() {
        let engine = HomeostasisEngine::new().unwrap();
        assert!(engine.needs_recalibration(&make_calm_state()));
    }

    #[test]
    fn test_needs_recalibration_same_state() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let state = make_calm_state();
        engine.calibrate_baseline(&state);
        assert!(!engine.needs_recalibration(&state));
    }

    #[test]
    fn test_needs_recalibration_large_drift() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        // Very different state should trigger recalibration
        let very_different = BiometricState::new(0.9, 0.1, 4.0, -0.9, 0.95).unwrap();
        assert!(engine.needs_recalibration(&very_different));
    }

    #[test]
    fn test_record_state() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let state = make_calm_state();
        engine.record_state(&state);
        assert_eq!(engine.recent_states.len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut engine = HomeostasisEngine::new().unwrap();
        engine.calibrate_baseline(&make_calm_state());
        engine.reset();
        assert!(engine.get_baseline().is_none());
        assert!(engine.recent_states.is_empty());
    }

    #[test]
    fn test_delta_deviation_magnitude() {
        let delta = HomeostasisDelta {
            stress_delta: 0.5,
            coherence_delta: -0.3,
            frequency_delta: 0.2,
            valence_delta: 0.4,
            arousal_delta: -0.1,
            homeostasis_score: 0.6,
            sct_z: 0.5,
            correction_magnitude: 0.4,
        };
        let magnitude = delta.deviation_magnitude();
        assert!((magnitude - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_delta_needs_correction() {
        let delta = HomeostasisDelta {
            stress_delta: 0.5,
            coherence_delta: -0.3,
            frequency_delta: 0.2,
            valence_delta: 0.4,
            arousal_delta: -0.1,
            homeostasis_score: 0.6,
            sct_z: 0.5,
            correction_magnitude: 0.4,
        };
        assert!(delta.needs_correction(0.1));
        assert!(!delta.needs_correction(0.6));
    }

    #[test]
    fn test_error_display() {
        match EngineError::BaselineNotCalibrated {
            EngineError::BaselineNotCalibrated => {
                let msg = format!("{}", EngineError::BaselineNotCalibrated);
                assert!(msg.contains("calibrated"));
            }
            _ => unreachable!(),
        }
    }
}
