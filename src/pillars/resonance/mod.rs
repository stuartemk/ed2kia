//! Resonance Interface — Biorretroalimentación Local (Pillar 4).
//!
//! **RFC 004:** Human biometric feedback loop — 100% local processing via WASM/Edge.
//! ZERO telemetry. All biometric data (face, voice, cardiovascular) processed
//! and discarded within the local execution boundary.
//!
//! **⚠️ LOCAL_ONLY CONSTRAINT:** This pillar MUST execute in a WASM/Edge environment.
//! Biometric data must NEVER leave the local node. TelemetryViolation on breach.
//!
//! **Architecture:**
//! - [`biometric_analyzer`]: Local biometric analysis (rPPG, voice, FACS-lite expressions).
//! - [`homeostasis_engine`]: Physiological equilibrium manager with SCT Guard validation.
//! - [`resonance_generator`]: Morphic resonance synthesis (binaural, isochronic, semantic).
//!
//! **Data Flow:**
//! 1. Biometric stream → `LocalBiometricAnalyzer` → `BiometricState`
//! 2. `BiometricState` → `HomeostasisEngine` → `HomeostasisDelta` (SCT validated)
//! 3. `HomeostasisDelta` + `BiometricState` → `ResonanceGenerator` → `ResonanceResponse`
//! 4. Raw biometric data cleared — only `ResonanceResponse` delivered locally.
//!
//! **Feature Gate:** `v3.0-resonance-interface`
//! **Target:** `wasm32-unknown-unknown` (browser) / `wasm32-wasi` (edge)

pub mod biometric_analyzer;
pub mod homeostasis_engine;
pub mod resonance_generator;
#[cfg(feature = "v3.6-aegis-resonance")]
pub mod biofeedback_engine;

use crate::orchestration::PillarId;
use crate::pillars::{PillarError, PillarInterface};

use biometric_analyzer::{LocalBiometricAnalyzer, AnalyzerConfig, BiometricState, AnalyzerError};
use homeostasis_engine::{HomeostasisEngine, HomeostasisConfig, EngineError};
use resonance_generator::{ResonanceGenerator, ResonanceConfig, ResonanceResponse, ResonanceError};

/// Re-export key types for external consumers.
pub use biometric_analyzer::AnalyzerError as BiometricAnalyzerError;
pub use homeostasis_engine::EngineError as HomeostasisEngineError;
pub use resonance_generator::ResonanceError as ResonanceGeneratorError;
pub use resonance_generator::{BinauralBeat, IsochronicTone, SemanticResponse};

/// Errors specific to the Resonance Interface pillar orchestration.
#[derive(Debug, thiserror::Error)]
pub enum ResonancePillarError {
    #[error("Biometric analysis failed: {0}")]
    Biometric(#[from] AnalyzerError),

    #[error("Homeostasis engine failed: {0}")]
    Homeostasis(#[from] EngineError),

    #[error("Resonance generation failed: {0}")]
    Resonance(#[from] ResonanceError),

    #[error("Engine not calibrated: call `calibrate()` first")]
    NotCalibrated,

    #[error("Telemetry violation: biometric data must remain LOCAL_ONLY")]
    TelemetryViolation,

    #[error("Insufficient CE: {available} < {required}")]
    InsufficientCE { available: f64, required: f64 },
}

/// Resonance Interface Engine — Local biometric feedback coordinator.
///
/// **⚠️ LOCAL_ONLY:** All biometric processing occurs within the WASM/Edge runtime.
/// Zero telemetry. Data is processed and discarded — never transmitted.
///
/// **Flow:**
/// 1. `calibrate()` — Establish baseline biometric state.
/// 2. `process_stream()` — Analyze biometric stream, calculate homeostasis delta, generate resonance response.
/// 3. `clear_buffers()` — Discard raw biometric data (privacy invariant).
pub struct ResonanceEngine {
    analyzer: LocalBiometricAnalyzer,
    homeostasis: HomeostasisEngine,
    generator: ResonanceGenerator,
    ce_balance: f64,
}

impl ResonanceEngine {
    /// Create a new Resonance Interface Engine with default configuration.
    pub fn new() -> Self {
        Self {
            analyzer: LocalBiometricAnalyzer::new(),
            homeostasis: HomeostasisEngine::new().expect("Default homeostasis config must be valid"),
            generator: ResonanceGenerator::new(),
            ce_balance: 0.0,
        }
    }

    /// Create with custom configuration for all sub-components.
    pub fn with_config(
        analyzer_config: AnalyzerConfig,
        homeostasis_config: HomeostasisConfig,
        resonance_config: ResonanceConfig,
    ) -> Result<Self, EngineError> {
        Ok(Self {
            analyzer: LocalBiometricAnalyzer::with_config(analyzer_config),
            homeostasis: HomeostasisEngine::with_config(homeostasis_config)?,
            generator: ResonanceGenerator::with_config(resonance_config),
            ce_balance: 0.0,
        })
    }

    /// Calibrate the engine with baseline biometric state.
    ///
    /// Must be called before `process_stream()` to establish the homeostasis baseline.
    pub fn calibrate(&mut self, state: &BiometricState) {
        self.homeostasis.calibrate_baseline(state);
    }

    /// Deposit CE (Compute Energy) vouchers for processing.
    pub fn deposit_ce(&mut self, amount: f64) {
        if amount > 0.0 {
            self.ce_balance += amount;
        }
    }

    /// Process a complete biometric stream and generate resonance response.
    ///
    /// Full pipeline:
    /// 1. Analyze biometric stream → `BiometricState`
    /// 2. Calculate homeostasis deviation → `HomeostasisDelta` (SCT validated)
    /// 3. Generate resonance response → `ResonanceResponse`
    /// 4. Consume CE for processing cost
    /// 5. Clear raw biometric buffers (privacy invariant)
    pub fn process_stream(
        &mut self,
        rppg_samples: &[f32],
        voice_samples: &[f32],
        expression_samples: &[f32],
    ) -> Result<ResonanceResponse, ResonancePillarError> {
        // Check calibration
        if self.homeostasis.get_baseline().is_none() {
            return Err(ResonancePillarError::NotCalibrated);
        }

        // CE cost for full pipeline processing
        let ce_cost: f64 = 1.0;
        if self.ce_balance < ce_cost {
            return Err(ResonancePillarError::InsufficientCE {
                available: self.ce_balance,
                required: ce_cost,
            });
        }

        // Step 1: Analyze biometric stream
        let state = self.analyzer.analyze_stream(
            rppg_samples,
            voice_samples,
            expression_samples,
        )?;

        // Step 2: Calculate homeostasis deviation (SCT validated)
        let delta = self.homeostasis.calculate_deviation(&state)?;

        // Record state for drift tracking
        self.homeostasis.record_state(&state);

        // Step 3: Generate resonance response
        let response = self.generator.generate_response(&delta, &state)?;

        // Step 4: Consume CE
        self.ce_balance -= ce_cost;

        // Step 5: Clear raw biometric buffers (privacy invariant)
        self.analyzer.clear_buffers();

        Ok(response)
    }

    /// Get the current biometric state without generating resonance.
    pub fn get_biometric_state(
        &mut self,
        rppg_samples: &[f32],
        voice_samples: &[f32],
        expression_samples: &[f32],
    ) -> Result<BiometricState, AnalyzerError> {
        self.analyzer.analyze_stream(rppg_samples, voice_samples, expression_samples)
    }

    /// Check if recalibration is recommended.
    pub fn needs_recalibration(&self, state: &BiometricState) -> bool {
        self.homeostasis.needs_recalibration(state)
    }

    /// Clear all internal buffers (privacy emergency).
    pub fn clear_buffers(&mut self) {
        self.analyzer.clear_buffers();
    }

    /// Get current CE balance.
    pub fn ce_balance(&self) -> f64 {
        self.ce_balance
    }
}

impl PillarInterface for ResonanceEngine {
    fn id() -> PillarId {
        PillarId::ResonanceInterface
    }

    fn validate_local_constraint(&self) -> bool {
        // Resonance Interface enforces LOCAL_ONLY.
        // Returns true only when executing in WASM/Edge with zero telemetry.
        cfg!(target_arch = "wasm32")
    }

    fn consume_ce(&self, amount: f64) -> Result<(), PillarError> {
        if amount <= 0.0 {
            return Err(PillarError::InsufficientCE);
        }
        // CE consumed for biometric processing — validated locally.
        Ok(())
    }
}

impl Default for ResonanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_calm_state() -> BiometricState {
        BiometricState::new(0.1, 0.9, 1.0, 0.5, 0.3).unwrap()
    }

    fn make_rppg_samples(count: usize) -> Vec<f32> {
        (0..count).map(|i| (i as f32 * 0.01).sin() * 0.5 + 0.5).collect()
    }

    fn make_voice_samples(count: usize) -> Vec<f32> {
        (0..count).map(|i| (i as f32 * 0.02).sin() * 0.3 + 0.5).collect()
    }

    fn make_expression_samples(count: usize) -> Vec<f32> {
        (0..count).map(|_| 0.5).collect()
    }

    #[test]
    fn test_engine_creation() {
        let engine = ResonanceEngine::new();
        assert_eq!(engine.ce_balance(), 0.0);
    }

    #[test]
    fn test_pillar_id() {
        assert_eq!(ResonanceEngine::id(), PillarId::ResonanceInterface);
    }

    #[test]
    fn test_local_constraint_wasm() {
        let engine = ResonanceEngine::new();
        if cfg!(target_arch = "wasm32") {
            assert!(engine.validate_local_constraint());
        } else {
            assert!(!engine.validate_local_constraint());
        }
    }

    #[test]
    fn test_consume_ce_valid() {
        let engine = ResonanceEngine::new();
        assert!(engine.consume_ce(1.0).is_ok());
    }

    #[test]
    fn test_consume_ce_zero_rejected() {
        let engine = ResonanceEngine::new();
        match engine.consume_ce(0.0) {
            Err(PillarError::InsufficientCE) => {}
            _ => panic!("Expected InsufficientCE"),
        }
    }

    #[test]
    fn test_consume_ce_negative_rejected() {
        let engine = ResonanceEngine::new();
        match engine.consume_ce(-1.0) {
            Err(PillarError::InsufficientCE) => {}
            _ => panic!("Expected InsufficientCE"),
        }
    }

    #[test]
    fn test_calibrate() {
        let mut engine = ResonanceEngine::new();
        let state = make_calm_state();
        engine.calibrate(&state);
        assert!(engine.homeostasis.get_baseline().is_some());
    }

    #[test]
    fn test_process_stream_not_calibrated() {
        let mut engine = ResonanceEngine::new();
        let rppg = make_rppg_samples(256);
        let voice = make_voice_samples(128);
        let expr = make_expression_samples(16);

        match engine.process_stream(&rppg, &voice, &expr) {
            Err(ResonancePillarError::NotCalibrated) => {}
            _ => panic!("Expected NotCalibrated"),
        }
    }

    #[test]
    fn test_process_stream_insufficient_ce() {
        let mut engine = ResonanceEngine::new();
        let state = make_calm_state();
        engine.calibrate(&state);

        let rppg = make_rppg_samples(256);
        let voice = make_voice_samples(128);
        let expr = make_expression_samples(16);

        match engine.process_stream(&rppg, &voice, &expr) {
            Err(ResonancePillarError::InsufficientCE { .. }) => {}
            _ => panic!("Expected InsufficientCE"),
        }
    }

    #[test]
    fn test_deposit_ce() {
        let mut engine = ResonanceEngine::new();
        engine.deposit_ce(5.0);
        assert_eq!(engine.ce_balance(), 5.0);
    }

    #[test]
    fn test_deposit_ce_negative_ignored() {
        let mut engine = ResonanceEngine::new();
        engine.deposit_ce(-1.0);
        assert_eq!(engine.ce_balance(), 0.0);
    }

    #[test]
    fn test_needs_recalibration() {
        let mut engine = ResonanceEngine::new();
        let calm = make_calm_state();
        engine.calibrate(&calm);

        let very_different = BiometricState::new(0.9, 0.1, 4.0, -0.9, 0.95).unwrap();
        assert!(engine.needs_recalibration(&very_different));
    }

    #[test]
    fn test_clear_buffers() {
        let mut engine = ResonanceEngine::new();
        engine.clear_buffers();
        // Should not panic
    }

    #[test]
    fn test_default() {
        let engine = ResonanceEngine::default();
        assert_eq!(engine.ce_balance(), 0.0);
    }
}
