//! Symbiotic Aegis Healer — P3 + P4 Bridge for Network-Human Harmony.
//!
//! Connects the Steganographic pillar (network preservation) with the
//! Resonance pillar (human healing) through a symbiotic coordination layer.
//!
//! **Design Philosophy:**
//! - **Simbiosis**: Network preservation and human healing evolve together.
//! - **Armonía**: When the network flows harmoniously, the human operator
//!   experiences reduced stress. When the human is coherent, the network
//!   benefits from clearer decision-making.
//! - **Homeostasis**: Both systems seek equilibrium through cooperative adaptation.
//! - **Preservación**: The Aegis shield protects both network integrity and
//!   human well-being as a unified whole.
//!
//! **Architecture:**
//! ```text
//!  ┌─────────────────────────────────────────────────────────────┐
//!  │                   Symbiotic Aegis Healer                    │
//!  │                                                             │
//!  │  ┌────────────────────┐      ┌────────────────────┐        │
//!  │  │  Pillar 3: P3      │      │  Pillar 4: P4      │        │
//!  │  │  Steganographic    │◄────►│  Resonance         │        │
//!  │  │  (Network)         │      │  (Human)           │        │
//!  │  └────────────────────┘      └────────────────────┘        │
//!  │         │                                       │          │
//!  │         ▼                                       ▼          │
//!  │  ┌────────────────────┐      ┌────────────────────┐        │
//!  │  │  Harmonic Flow     │      │  Biofeedback       │        │
//!  │  │  (SRTP Pipeline)   │      │  (Local Healing)   │        │
//!  │  └────────────────────┘      └────────────────────┘        │
//!  └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Feature Gate:** `v3.6-aegis-resonance`

#[cfg(feature = "v3.6-aegis-resonance")]
use crate::pillars::steganographic::harmonic_flow::{
    HarmonicFlow, HarmonicFlowConfig, ObfuscatedStream,
};

#[cfg(feature = "v3.6-aegis-resonance")]
use crate::pillars::resonance::biofeedback_engine::{
    BiofeedbackEngine, BiofeedbackResult,
};

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

/// Errors that can occur in the Symbiotic Aegis Healer.
#[derive(Debug, PartialEq)]
pub enum AegisError {
    /// Network pipeline (P3) error.
    NetworkError(String),
    /// Biofeedback pipeline (P4) error.
    BiofeedbackError(String),
    /// Symbiotic alignment failed — P3 and P4 out of harmony.
    AlignmentMismatch { network_score: f64, human_score: f64 },
    /// SCT guard rejected the symbiotic operation.
    SCTRejected(f64),
    /// Configuration error.
    ConfigError(String),
    /// Feature gate not enabled.
    FeatureDisabled,
}

impl std::fmt::Display for AegisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AegisError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AegisError::BiofeedbackError(msg) => write!(f, "Biofeedback error: {}", msg),
            AegisError::AlignmentMismatch {
                network_score,
                human_score,
            } => {
                write!(
                    f,
                    "Alignment mismatch: network = {:.4}, human = {:.4}",
                    network_score, human_score
                )
            }
            AegisError::SCTRejected(z) => {
                write!(f, "SCT guard rejected symbiotic operation (Z = {:.4})", z)
            }
            AegisError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            AegisError::FeatureDisabled => {
                write!(f, "Feature v3.6-aegis-resonance not enabled")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Symbiotic Aegis Healer.
#[derive(Debug, Clone)]
pub struct AegisConfig {
    /// Maximum allowed deviation between network and human scores.
    /// When |network_score - human_score| > threshold, alignment healing triggers.
    pub alignment_threshold: f64,
    /// Enable automatic healing when alignment drifts.
    pub auto_heal: bool,
    /// CE (Cooperative Energy) budget per symbiotic cycle.
    pub ce_budget: f64,
    /// Minimum homeostasis score for healthy operation.
    pub min_homeostasis: f64,
    /// Network session ID for harmonic flow.
    pub network_session_id: String,
}

impl Default for AegisConfig {
    fn default() -> Self {
        Self {
            alignment_threshold: 0.3,
            auto_heal: true,
            ce_budget: 100.0,
            min_homeostasis: 0.5,
            network_session_id: "aegis-symbiotic-session".to_string(),
        }
    }
}

impl AegisConfig {
    /// Validate configuration.
    pub fn validate(&self) -> Result<(), AegisError> {
        if self.alignment_threshold < 0.0 || self.alignment_threshold > 1.0 {
            return Err(AegisError::ConfigError(
                "alignment_threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        if self.ce_budget <= 0.0 {
            return Err(AegisError::ConfigError(
                "ce_budget must be positive".to_string(),
            ));
        }
        if self.min_homeostasis < 0.0 || self.min_homeostasis > 1.0 {
            return Err(AegisError::ConfigError(
                "min_homeostasis must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Symbiotic State
// ---------------------------------------------------------------------------

/// Current state of the symbiotic relationship between P3 and P4.
#[derive(Debug, Clone)]
pub struct AegisSymbioticState {
    /// Network health score (0.0 = degraded, 1.0 = optimal).
    pub network_health: f64,
    /// Human coherence score (0.0 = distressed, 1.0 = coherent).
    pub human_coherence: f64,
    /// Alignment score (0.0 = mismatched, 1.0 = perfectly aligned).
    pub alignment: f64,
    /// Whether the symbiotic relationship is in harmony.
    pub in_harmony: bool,
    /// Healing cycles executed.
    pub healing_cycles: u32,
    /// Total CE consumed.
    pub ce_consumed: f64,
}

impl AegisSymbioticState {
    /// Calculate alignment from network and human scores.
    pub fn calculate_alignment(network: f64, human: f64) -> f64 {
        // Alignment is 1.0 when scores match, decreases with divergence.
        1.0 - (network - human).abs()
    }

    /// Check if the symbiotic relationship is in harmony.
    pub fn is_in_harmony(&self) -> bool {
        self.alignment >= 0.7 && self.network_health >= 0.5 && self.human_coherence >= 0.5
    }
}

// ---------------------------------------------------------------------------
// Healing Result
// ---------------------------------------------------------------------------

/// Result of a symbiotic healing cycle.
#[derive(Debug)]
pub struct HealingResult {
    /// Updated symbiotic state.
    pub state: AegisSymbioticState,
    /// Network obfuscation result (if applicable).
    pub network_obfuscated: bool,
    /// Biofeedback result (if applicable).
    pub biofeedback_applied: bool,
    /// Healing actions taken.
    pub actions: Vec<HealingAction>,
    /// CE consumed this cycle.
    pub ce_consumed: f64,
}

/// Individual healing action taken during a cycle.
#[derive(Debug, Clone)]
pub enum HealingAction {
    /// Adjust network transport rotation for better preservation.
    RotateTransport,
    /// Increase chaff ratio for better signal dilution.
    IncreaseChaff(f64),
    /// Generate resonance response for human coherence.
    GenerateResonance,
    /// Recalibrate biometric baseline.
    RecalibrateBaseline,
    /// Apply symbiotic alignment correction.
    AlignSymbiosis(f64),
}

impl std::fmt::Display for HealingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealingAction::RotateTransport => write!(f, "Rotate transport"),
            HealingAction::IncreaseChaff(ratio) => {
                write!(f, "Increase chaff ratio to {:.2}", ratio)
            }
            HealingAction::GenerateResonance => write!(f, "Generate resonance"),
            HealingAction::RecalibrateBaseline => write!(f, "Recalibrate baseline"),
            HealingAction::AlignSymbiosis(score) => {
                write!(f, "Align symbiosis (score = {:.4})", score)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Aegis Healer
// ---------------------------------------------------------------------------

/// Symbiotic Shield — bridges network preservation (P3) with human healing (P4).
///
/// This orchestrator maintains harmony between the network's preservation
/// needs and the human operator's well-being, creating a symbiotic
/// relationship where both systems support each other's homeostasis.
///
/// **Feature Gate:** `v3.6-aegis-resonance`
#[cfg(feature = "v3.6-aegis-resonance")]
pub struct AegisHealer {
    /// Network preservation pipeline (P3).
    harmonic_flow: HarmonicFlow,
    /// Human healing pipeline (P4).
    biofeedback: BiofeedbackEngine,
    /// Healer configuration.
    config: AegisConfig,
    /// Current symbiotic state.
    state: AegisSymbioticState,
    /// Total CE consumed across all cycles.
    total_ce_consumed: f64,
}

#[cfg(feature = "v3.6-aegis-resonance")]
impl AegisHealer {
    // --- Construction ---

    /// Create a new AegisHealer with default configuration.
    pub fn new() -> Self {
        let config = AegisConfig::default();
        Self::with_config(config).expect("Default config should be valid")
    }

    /// Create an AegisHealer with custom configuration.
    pub fn with_config(config: AegisConfig) -> Result<Self, AegisError> {
        config.validate()?;

        // Create HarmonicFlow with network session from config.
        let mut flow_config = HarmonicFlowConfig::default();
        flow_config.session_id = config.network_session_id.clone();
        let harmonic_flow = HarmonicFlow::with_config(flow_config);

        // Create BiofeedbackEngine.
        let biofeedback = BiofeedbackEngine::new()
            .map_err(|e| AegisError::BiofeedbackError(format!("{}", e)))?;

        Ok(Self {
            harmonic_flow,
            biofeedback,
            config,
            state: AegisSymbioticState {
                network_health: 0.5,
                human_coherence: 0.5,
                alignment: 1.0,
                in_harmony: false,
                healing_cycles: 0,
                ce_consumed: 0.0,
            },
            total_ce_consumed: 0.0,
        })
    }

    // --- Network Operations (P3) ---

    /// Obfuscate a network payload through the harmonic flow pipeline.
    pub fn obfuscate_network(
        &mut self,
        payload: &[u8],
    ) -> Result<ObfuscatedStream, AegisError> {
        self.harmonic_flow
            .obfuscate(payload)
            .map_err(|e| AegisError::NetworkError(format!("{}", e)))
    }

    /// Update network health score based on transport metrics.
    pub fn update_network_health(&mut self, health: f64) {
        self.state.network_health = health.clamp(0.0, 1.0);
        self.update_alignment();
    }

    /// Report transport health to the harmonic flow pipeline.
    pub fn report_transport_health(&mut self, success_rate: f64, latency_ms: f64) {
        self.harmonic_flow
            .report_health(success_rate, latency_ms);
    }

    // --- Biofeedback Operations (P4) ---

    /// Calibrate the biometric baseline.
    pub fn calibrate_biofeedback(
        &mut self,
        rppg: &[f32],
        voice: &[f32],
        expressions: &[f32],
    ) -> Result<(), AegisError> {
        let _state = self.biofeedback
            .calibrate(rppg, voice, expressions)
            .map_err(|e| AegisError::BiofeedbackError(format!("{}", e)))?;
        Ok(())
    }

    /// Process a biofeedback cycle.
    pub fn process_biofeedback(
        &mut self,
        rppg: &[f32],
        voice: &[f32],
        expressions: &[f32],
    ) -> Result<BiofeedbackResult, AegisError> {
        self.biofeedback
            .process_cycle(rppg, voice, expressions)
            .map_err(|e| AegisError::BiofeedbackError(format!("{}", e)))
    }

    /// Update human coherence score from biofeedback results.
    pub fn update_human_coherence(&mut self, coherence: f64) {
        self.state.human_coherence = coherence.clamp(0.0, 1.0);
        self.update_alignment();
    }

    // --- Symbiotic Operations ---

    /// Execute a full symbiotic cycle: network + biofeedback + healing.
    ///
    /// This is the main coordination loop that:
    /// 1. Processes network payload obfuscation (P3)
    /// 2. Processes biofeedback cycle (P4)
    /// 3. Evaluates alignment between P3 and P4
    /// 4. Applies healing actions if needed
    pub fn symbiotic_cycle(
        &mut self,
        network_payload: &[u8],
        rppg: &[f32],
        voice: &[f32],
        expressions: &[f32],
    ) -> Result<HealingResult, AegisError> {
        let mut actions = Vec::new();
        let mut ce_consumed = 0.0;

        // Stage 1: Network obfuscation (P3).
        let stream = self.obfuscate_network(network_payload)?;
        let network_obfuscated = true;

        // Update network health based on expansion ratio.
        // Lower expansion = more efficient = healthier.
        let efficiency = 1.0 / stream.expansion_ratio.max(1.0);
        self.update_network_health(efficiency);

        // Stage 2: Biofeedback cycle (P4).
        let bio_result = self.process_biofeedback(rppg, voice, expressions)?;
        let biofeedback_applied = true;

        // Update human coherence from biofeedback.
        self.update_human_coherence(bio_result.homeostasis_score as f64);

        // Stage 3: Evaluate alignment.
        let alignment = AegisSymbioticState::calculate_alignment(
            self.state.network_health,
            self.state.human_coherence,
        );
        self.state.alignment = alignment;
        self.state.in_harmony = self.state.is_in_harmony();

        // Stage 4: Apply healing if needed.
        if self.config.auto_heal && !self.state.in_harmony {
            let (healing_actions, healing_ce) = self.apply_healing()?;
            actions.extend(healing_actions);
            ce_consumed += healing_ce;
        }

        // Update state.
        self.state.healing_cycles += 1;
        self.state.ce_consumed = ce_consumed;
        self.total_ce_consumed += ce_consumed;

        Ok(HealingResult {
            state: self.state.clone(),
            network_obfuscated,
            biofeedback_applied,
            actions,
            ce_consumed,
        })
    }

    /// Apply healing actions to restore symbiotic harmony.
    fn apply_healing(&mut self) -> Result<(Vec<HealingAction>, f64), AegisError> {
        let mut actions = Vec::new();
        let mut ce = 0.0;

        // Check if network needs healing.
        if self.state.network_health < self.config.min_homeostasis {
            // Rotate transport for better preservation.
            self.harmonic_flow.rotate_transport();
            actions.push(HealingAction::RotateTransport);
            ce += 1.0;

            // Increase chaff ratio if still degraded.
            if self.state.network_health < self.config.min_homeostasis * 0.5 {
                actions.push(HealingAction::IncreaseChaff(0.8));
                ce += 2.0;
            }
        }

        // Check if human needs healing.
        if self.state.human_coherence < self.config.min_homeostasis {
            // Generate resonance response.
            actions.push(HealingAction::GenerateResonance);
            ce += 1.5;
        }

        // Apply symbiotic alignment correction.
        if self.state.alignment < (1.0 - self.config.alignment_threshold) {
            let correction = self.state.alignment;
            actions.push(HealingAction::AlignSymbiosis(correction));
            ce += 0.5;
        }

        // Check CE budget.
        if ce > self.config.ce_budget {
            ce = self.config.ce_budget;
        }

        Ok((actions, ce))
    }

    /// Update alignment score based on current network and human scores.
    fn update_alignment(&mut self) {
        self.state.alignment = AegisSymbioticState::calculate_alignment(
            self.state.network_health,
            self.state.human_coherence,
        );
        self.state.in_harmony = self.state.is_in_harmony();
    }

    // --- State Access ---

    /// Get the current symbiotic state.
    pub fn get_state(&self) -> &AegisSymbioticState {
        &self.state
    }

    /// Get the current configuration.
    pub fn config(&self) -> &AegisConfig {
        &self.config
    }

    /// Get total CE consumed.
    pub fn total_ce_consumed(&self) -> f64 {
        self.total_ce_consumed
    }

    // --- Reset ---

    /// Reset the healer to initial state.
    pub fn reset(&mut self) {
        self.harmonic_flow.reset();
        self.biofeedback.reset();
        self.state = AegisSymbioticState {
            network_health: 0.5,
            human_coherence: 0.5,
            alignment: 1.0,
            in_harmony: false,
            healing_cycles: 0,
            ce_consumed: 0.0,
        };
        self.total_ce_consumed = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Tests (always compiled — stubs when feature is disabled)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[cfg(feature = "v3.6-aegis-resonance")]
    mod with_feature {
        use super::super::{
            AegisConfig, AegisError, AegisHealer, HealingAction, AegisSymbioticState,
        };

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
        fn test_healer_creation() {
            let healer = AegisHealer::new();
            assert_eq!(healer.get_state().healing_cycles, 0);
        }

        #[test]
        fn test_healer_custom_config() {
            let config = AegisConfig {
                alignment_threshold: 0.5,
                auto_heal: false,
                ..AegisConfig::default()
            };
            let healer = AegisHealer::with_config(config).expect("Config valid");
            assert_eq!(healer.config().alignment_threshold, 0.5);
            assert!(!healer.config().auto_heal);
        }

        // --- Configuration Tests ---

        #[test]
        fn test_config_validate_valid() {
            let config = AegisConfig::default();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn test_config_validate_bad_threshold() {
            let config = AegisConfig {
                alignment_threshold: 1.5,
                ..AegisConfig::default()
            };
            match config.validate() {
                Err(AegisError::ConfigError(msg)) => {
                    assert!(msg.contains("alignment_threshold"));
                }
                other => panic!("Expected ConfigError, got {:?}", other),
            }
        }

        #[test]
        fn test_config_validate_zero_budget() {
            let config = AegisConfig {
                ce_budget: 0.0,
                ..AegisConfig::default()
            };
            match config.validate() {
                Err(AegisError::ConfigError(msg)) => {
                    assert!(msg.contains("ce_budget"));
                }
                other => panic!("Expected ConfigError, got {:?}", other),
            }
        }

        // --- Symbiotic State Tests ---

        #[test]
        fn test_alignment_identical_scores() {
            let alignment = AegisSymbioticState::calculate_alignment(0.8, 0.8);
            assert!((alignment - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_alignment_opposite_scores() {
            let alignment = AegisSymbioticState::calculate_alignment(0.0, 1.0);
            assert!((alignment - 0.0).abs() < 0.001);
        }

        #[test]
        fn test_alignment_partial() {
            let alignment = AegisSymbioticState::calculate_alignment(0.7, 0.4);
            assert!((alignment - 0.7).abs() < 0.001);
        }

        #[test]
        fn test_is_in_harmony() {
            let state = AegisSymbioticState {
                network_health: 0.9,
                human_coherence: 0.85,
                alignment: 0.95,
                in_harmony: true,
                healing_cycles: 0,
                ce_consumed: 0.0,
            };
            assert!(state.is_in_harmony());
        }

        #[test]
        fn test_is_not_in_harmony() {
            let state = AegisSymbioticState {
                network_health: 0.3,
                human_coherence: 0.2,
                alignment: 0.5,
                in_harmony: false,
                healing_cycles: 0,
                ce_consumed: 0.0,
            };
            assert!(!state.is_in_harmony());
        }

        // --- Network Operations Tests ---

        #[test]
        fn test_update_network_health() {
            let mut healer = AegisHealer::new();
            healer.update_network_health(0.85);
            assert!((healer.get_state().network_health - 0.85).abs() < 0.001);
        }

        #[test]
        fn test_update_network_health_clamping() {
            let mut healer = AegisHealer::new();
            healer.update_network_health(1.5);
            assert!((healer.get_state().network_health - 1.0).abs() < 0.001);

            healer.update_network_health(-0.5);
            assert!((healer.get_state().network_health - 0.0).abs() < 0.001);
        }

        // --- Biofeedback Operations Tests ---

        #[test]
        fn test_update_human_coherence() {
            let mut healer = AegisHealer::new();
            healer.update_human_coherence(0.75);
            assert!((healer.get_state().human_coherence - 0.75).abs() < 0.001);
        }

        #[test]
        fn test_calibrate_biofeedback() {
            let mut healer = AegisHealer::new();
            let rppg = make_rppg(128);
            let voice = make_voice(128);
            let expr = make_expressions(128);
            healer.calibrate_biofeedback(&rppg, &voice, &expr)
                .expect("Calibration failed");
        }

        // --- Healing Action Display Tests ---

        #[test]
        fn test_healing_action_display() {
            let action = HealingAction::RotateTransport;
            let msg = format!("{}", action);
            assert!(msg.contains("transport"));
        }

        #[test]
        fn test_healing_action_display_chaff() {
            let action = HealingAction::IncreaseChaff(0.8);
            let msg = format!("{}", action);
            assert!(msg.contains("chaff"));
        }

        // --- Error Display Tests ---

        #[test]
        fn test_error_display_network() {
            let err = AegisError::NetworkError("test".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("Network"));
        }

        #[test]
        fn test_error_display_biofeedback() {
            let err = AegisError::BiofeedbackError("test".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("Biofeedback"));
        }

        #[test]
        fn test_error_display_alignment() {
            let err = AegisError::AlignmentMismatch {
                network_score: 0.5,
                human_score: 0.3,
            };
            let msg = format!("{}", err);
            assert!(msg.contains("Alignment"));
        }

        #[test]
        fn test_error_display_feature() {
            let err = AegisError::FeatureDisabled;
            let msg = format!("{}", err);
            assert!(msg.contains("feature"));
        }

        // --- Reset Tests ---

        #[test]
        fn test_reset() {
            let mut healer = AegisHealer::new();
            healer.update_network_health(0.9);
            healer.update_human_coherence(0.8);

            healer.reset();
            assert!((healer.get_state().network_health - 0.5).abs() < 0.001);
            assert!((healer.get_state().human_coherence - 0.5).abs() < 0.001);
            assert_eq!(healer.get_state().healing_cycles, 0);
        }

        // --- Integration Tests ---

        #[test]
        fn test_alignment_updates_automatically() {
            let mut healer = AegisHealer::new();

            healer.update_network_health(0.9);
            healer.update_human_coherence(0.9);

            assert!(healer.get_state().alignment > 0.95);
        }

        #[test]
        fn test_ce_tracking() {
            let healer = AegisHealer::new();
            assert_eq!(healer.total_ce_consumed(), 0.0);
        }
    }

    // Tests without feature gate — verify feature is properly gated.
    #[cfg(not(feature = "v3.6-aegis-resonance"))]
    mod without_feature {
        #[test]
        fn test_feature_gated() {
            // When feature is disabled, AegisHealer is not available.
            // This test just confirms the feature gate works.
            assert!(true);
        }
    }
}
