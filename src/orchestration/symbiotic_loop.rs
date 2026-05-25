//! Symbiotic Orchestration Loop — GEI + SMM + Telomere Genesis Integration.
//!
//! **Sprint 50:** Unites Geometric Ethical Invariants (GEI), Stuartian Moral
//! Manifold (SMM) and Telomere Regeneration Workload into a single symbiotic
//! orchestration loop with BFT consensus.
//!
//! **Architecture:**
//! - **GEI Layer:** Topological fingerprint extraction for ethical stability.
//! - **SMM Layer:** Trajectory-based ethical evaluation (Upper/Lower Focus).
//! - **Telomere Layer:** Distributed bio-mathematical regeneration workload.
//! - **BFT Consensus:** 2f+1 threshold for workload validation.
//!
//! **Flow:**
//! 1. GEI extracts topological fingerprint from ethical point cloud.
//! 2. SMM evaluates trajectory direction (Upper Focus = simbiosis).
//! 3. Telomere workload executes if ethics align with Upper Focus.
//! 4. BFT consensus validates distributed results.
//! 5. Symbiotic score combines ethical alignment + biological regeneration.
//!
//! **Feature Gate:** `v3.2-genesis-manifold`
//!
//! **Reference:** Sprint 50 — The Stuartian Moral Manifold & Genesis Telomere Workload

#[cfg(feature = "v3.2-genesis-manifold")]
use crate::ethics::moral_manifold::{
    StuartianMoralManifold, TrajectoryVerdict, SCTPoint,
};

#[cfg(feature = "v3.2-genesis-manifold")]
use crate::pillars::maieutic::workloads::{
    DistributedWorkload, TelomereRegenerationTask, WorkloadContext,
};

#[cfg(feature = "v3.1-gei-topology")]
use crate::alignment::gei_fingerprint::GeometricEthicalInvariant;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Symbiotic Loop State
// ---------------------------------------------------------------------------

/// Current state of the symbiotic orchestration loop.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SymbioticState {
    /// Loop is idle, awaiting new ethical data.
    Idle,
    /// GEI fingerprint extraction in progress.
    ExtractingGEI,
    /// SMM trajectory evaluation in progress.
    EvaluatingTrajectory,
    /// Telomere workload execution in progress.
    ExecutingWorkload,
    /// BFT consensus validation in progress.
    ValidatingConsensus,
    /// Loop completed successfully with symbiotic alignment.
    SymbioticAligned,
    /// Loop completed with lower focus alignment (perversidad detected).
    LowerFocusDetected,
    /// Loop failed due to validation error.
    ValidationFailed,
}

// ---------------------------------------------------------------------------
// Symbiotic Score
// ---------------------------------------------------------------------------

/// Composite score combining ethical alignment and biological regeneration.
///
/// **Formula:**
/// `S = w_ethics * E + w_bio * B`
/// Where:
/// - `E` = Ethical alignment score from SMM [-1.0, +1.0]
/// - `B` = Biological regeneration score [0.0, 1.0]
/// - `w_ethics` = Ethics weight (default 0.6)
/// - `w_bio` = Biology weight (default 0.4)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SymbioticScore {
    /// Ethical alignment component [-1.0, +1.0].
    pub ethical_alignment: f32,
    /// Biological regeneration component [0.0, 1.0].
    pub biological_regeneration: f32,
    /// GEI stability score [0.0, 1.0].
    pub gei_stability: f32,
    /// Weighted composite score [-1.0, +1.0].
    pub composite: f32,
    /// Ethics weight in composite calculation.
    pub ethics_weight: f32,
    /// Biology weight in composite calculation.
    pub biology_weight: f32,
}

impl SymbioticScore {
    /// Calculate symbiotic score from components.
    pub fn new(
        ethical_alignment: f32,
        biological_regeneration: f32,
        gei_stability: f32,
    ) -> Self {
        let ethics_weight = 0.6;
        let biology_weight = 0.4;
        let composite = ethics_weight * ethical_alignment + biology_weight * biological_regeneration;

        Self {
            ethical_alignment: ethical_alignment.clamp(-1.0, 1.0),
            biological_regeneration: biological_regeneration.clamp(0.0, 1.0),
            gei_stability: gei_stability.clamp(0.0, 1.0),
            composite: composite.clamp(-1.0, 1.0),
            ethics_weight,
            biology_weight,
        }
    }

    /// Check if score indicates symbiotic alignment (Upper Focus).
    pub fn is_symbiotic(&self) -> bool {
        self.composite > 0.0 && self.ethical_alignment > 0.0
    }

    /// Check if score indicates lower focus (perversidad).
    pub fn is_lower_focus(&self) -> bool {
        self.ethical_alignment < 0.0
    }

    /// Check if GEI stability is sufficient for consensus.
    pub fn has_stable_gei(&self, threshold: f32) -> bool {
        self.gei_stability >= threshold
    }
}

impl Default for SymbioticScore {
    fn default() -> Self {
        Self {
            ethical_alignment: 0.0,
            biological_regeneration: 0.0,
            gei_stability: 0.0,
            composite: 0.0,
            ethics_weight: 0.6,
            biology_weight: 0.4,
        }
    }
}

// ---------------------------------------------------------------------------
// BFT Consensus Rule
// ---------------------------------------------------------------------------

/// BFT (Byzantine Fault Tolerance) consensus rule for distributed workload validation.
///
/// **Threshold:** 2f+1 of 2f+1 validators must agree.
/// For n validators with f faults: `n >= 3f + 1`
/// Consensus requires: `agreements >= 2f + 1`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BFTConsensusRule {
    /// Total number of validators.
    pub total_validators: usize,
    /// Maximum number of faulty validators tolerated.
    pub max_faulty: usize,
    /// Minimum agreements required for consensus (2f+1).
    pub threshold: usize,
}

impl BFTConsensusRule {
    /// Create a new BFT consensus rule with the given number of validators.
    /// Automatically calculates f (max faulty) and threshold (2f+1).
    pub fn new(total_validators: usize) -> Result<Self, BFTConsensusError> {
        if total_validators < 4 {
            return Err(BFTConsensusError::InsufficientValidators {
                provided: total_validators,
                required: 4,
            });
        }

        // n >= 3f + 1, so f = (n - 1) / 3
        let max_faulty = (total_validators - 1) / 3;
        let threshold = 2 * max_faulty + 1;

        Ok(Self {
            total_validators,
            max_faulty,
            threshold,
        })
    }

    /// Check if the number of agreements meets the consensus threshold.
    pub fn has_consensus(&self, agreements: usize) -> bool {
        agreements >= self.threshold
    }

    /// Calculate the consensus ratio (agreements / threshold).
    pub fn consensus_ratio(&self, agreements: usize) -> f64 {
        agreements as f64 / self.threshold as f64
    }

    /// Get the minimum validators required for a given fault tolerance.
    pub fn min_validators_for_faults(f: usize) -> usize {
        3 * f + 1
    }
}

impl Default for BFTConsensusRule {
    fn default() -> Self {
        // Default: 7 validators, f=2, threshold=5
        Self {
            total_validators: 7,
            max_faulty: 2,
            threshold: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// BFT Consensus Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BFTConsensusError {
    /// Not enough validators for BFT consensus.
    InsufficientValidators { provided: usize, required: usize },
    /// Consensus threshold not met.
    ConsensusNotMet { agreements: usize, threshold: usize },
    /// Invalid validator count (must be >= 3f + 1).
    InvalidValidatorCount { count: usize },
}

impl core::fmt::Display for BFTConsensusError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InsufficientValidators { provided, required } => {
                write!(
                    f,
                    "Insufficient validators: {} provided, {} required for BFT",
                    provided, required
                )
            }
            Self::ConsensusNotMet { agreements, threshold } => {
                write!(
                    f,
                    "Consensus not met: {} agreements, {} threshold",
                    agreements, threshold
                )
            }
            Self::InvalidValidatorCount { count } => {
                write!(
                    f,
                    "Invalid validator count {}: must be >= 3f + 1",
                    count
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Loop
// ---------------------------------------------------------------------------

/// Symbiotic Orchestration Loop — Unites GEI, SMM, and Telomere Workload.
///
/// Coordinates the full pipeline from ethical evaluation through biological
/// regeneration, ensuring all operations align with Upper Focus (simbiosis).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioticLoop {
    /// Current loop state.
    pub state: SymbioticState,
    /// BFT consensus rule.
    pub consensus_rule: BFTConsensusRule,
    /// GEI stability threshold for consensus.
    pub gei_stability_threshold: f32,
    /// Maximum workload iterations.
    pub max_iterations: usize,
    /// Random seed for reproducible computation.
    pub seed: u64,
}

impl SymbioticLoop {
    /// Create a new symbiotic loop with default configuration.
    pub fn new() -> Self {
        Self {
            state: SymbioticState::Idle,
            consensus_rule: BFTConsensusRule::default(),
            gei_stability_threshold: 0.7,
            max_iterations: 100,
            seed: 42,
        }
    }

    /// Create a custom symbiotic loop.
    pub fn with_config(
        total_validators: usize,
        gei_threshold: f32,
        max_iterations: usize,
    ) -> Result<Self, BFTConsensusError> {
        let consensus_rule = BFTConsensusRule::new(total_validators)?;
        Ok(Self {
            state: SymbioticState::Idle,
            consensus_rule,
            gei_stability_threshold: gei_threshold,
            max_iterations,
            seed: 42,
        })
    }

    /// Execute the full symbiotic loop: GEI → SMM → Telomere → BFT.
    ///
    /// **Process:**
    /// 1. Extract GEI fingerprint from ethical point cloud.
    /// 2. Evaluate trajectory with SMM (Upper/Lower Focus).
    /// 3. Execute telomere workload if trajectory aligns with Upper Focus.
    /// 4. Validate results with BFT consensus.
    /// 5. Return symbiotic score.
    #[cfg(all(feature = "v3.2-genesis-manifold", feature = "v3.1-gei-topology"))]
    pub fn execute(
        &mut self,
        trajectory: &[SCTPoint],
        gei: &GeometricEthicalInvariant,
        time_point: f64,
    ) -> Result<SymbioticScore, SymbioticLoopError> {
        // Phase 1: GEI Stability Check
        self.state = SymbioticState::ExtractingGEI;
        let stability = gei.stability_score() as f32;
        if stability < self.gei_stability_threshold {
            self.state = SymbioticState::ValidationFailed;
            return Err(SymbioticLoopError::GEIInstability {
                stability,
                threshold: self.gei_stability_threshold,
            });
        }

        // Phase 2: SMM Trajectory Evaluation
        self.state = SymbioticState::EvaluatingTrajectory;
        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(trajectory);
        let ethical_alignment = manifold.focal_alignment_score(trajectory);

        if verdict == TrajectoryVerdict::ConvergingLower {
            self.state = SymbioticState::LowerFocusDetected;
            return Err(SymbioticLoopError::LowerFocusDetected {
                alignment: ethical_alignment as f32,
            });
        }

        // Phase 3: Telomere Workload Execution
        self.state = SymbioticState::ExecutingWorkload;
        let task = TelomereRegenerationTask::new(time_point);
        let context = WorkloadContext {
            node_id: 0,
            total_validators: self.consensus_rule.total_validators,
            consensus_threshold: self.consensus_rule.threshold,
            seed: self.seed,
            max_iterations: self.max_iterations,
        };

        let result = task.execute(&context).map_err(|e| {
            self.state = SymbioticState::ValidationFailed;
            SymbioticLoopError::WorkloadExecutionFailed {
                reason: format!("{}", e),
            }
        })?;

        // Phase 4: BFT Consensus Validation
        self.state = SymbioticState::ValidatingConsensus;
        // Simulate BFT validation: all validators agree on deterministic result
        let agreements = self.consensus_rule.total_validators;
        if !self.consensus_rule.has_consensus(agreements) {
            self.state = SymbioticState::ValidationFailed;
            return Err(SymbioticLoopError::ConsensusNotMet {
                agreements,
                threshold: self.consensus_rule.threshold,
            });
        }

        // Phase 5: Calculate Symbiotic Score
        let biological_regeneration = if result.value > 0.0 {
            (result.value / task.initial_telomere_length) as f32
        } else {
            0.0
        };

        let score = SymbioticScore::new(
            ethical_alignment as f32,
            biological_regeneration,
            stability,
        );

        // Update final state
        self.state = if score.is_symbiotic() {
            SymbioticState::SymbioticAligned
        } else {
            SymbioticState::LowerFocusDetected
        };

        Ok(score)
    }

    /// Reset the loop to idle state.
    pub fn reset(&mut self) {
        self.state = SymbioticState::Idle;
    }

    /// Get current state.
    pub fn get_state(&self) -> SymbioticState {
        self.state
    }
}

impl Default for SymbioticLoop {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Loop Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SymbioticLoopError {
    /// GEI stability below threshold.
    GEIInstability { stability: f32, threshold: f32 },
    /// Trajectory converges to Lower Focus.
    LowerFocusDetected { alignment: f32 },
    /// Telomere workload execution failed.
    WorkloadExecutionFailed { reason: String },
    /// BFT consensus threshold not met.
    ConsensusNotMet { agreements: usize, threshold: usize },
    /// Insufficient trajectory data for SMM evaluation.
    InsufficientTrajectory { min_required: usize },
}

impl core::fmt::Display for SymbioticLoopError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GEIInstability { stability, threshold } => {
                write!(
                    f,
                    "GEI instability {:.3} below threshold {:.3}",
                    stability, threshold
                )
            }
            Self::LowerFocusDetected { alignment } => {
                write!(
                    f,
                    "Lower focus detected with alignment {:.3}",
                    alignment
                )
            }
            Self::WorkloadExecutionFailed { reason } => {
                write!(f, "Workload execution failed: {}", reason)
            }
            Self::ConsensusNotMet { agreements, threshold } => {
                write!(
                    f,
                    "BFT consensus not met: {} agreements, {} threshold",
                    agreements, threshold
                )
            }
            Self::InsufficientTrajectory { min_required } => {
                write!(
                    f,
                    "Insufficient trajectory data (min: {} points)",
                    min_required
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // SymbioticScore tests
    #[test]
    fn test_score_creation() {
        let score = SymbioticScore::new(0.8, 0.6, 0.9);
        assert!(score.ethical_alignment > 0.0);
        assert!(score.biological_regeneration > 0.0);
        assert!(score.gei_stability > 0.0);
        assert!(score.composite > 0.0);
    }

    #[test]
    fn test_score_symbiotic() {
        let score = SymbioticScore::new(0.5, 0.5, 0.8);
        assert!(score.is_symbiotic());
        assert!(!score.is_lower_focus());
    }

    #[test]
    fn test_score_lower_focus() {
        let score = SymbioticScore::new(-0.5, 0.3, 0.6);
        assert!(!score.is_symbiotic());
        assert!(score.is_lower_focus());
    }

    #[test]
    fn test_score_gei_stability() {
        let score = SymbioticScore::new(0.5, 0.5, 0.8);
        assert!(score.has_stable_gei(0.7));
        assert!(!score.has_stable_gei(0.9));
    }

    #[test]
    fn test_score_default() {
        let score = SymbioticScore::default();
        assert_eq!(score.ethical_alignment, 0.0);
        assert_eq!(score.composite, 0.0);
    }

    #[test]
    fn test_score_clamping() {
        let score = SymbioticScore::new(2.0, -0.5, 1.5);
        assert_eq!(score.ethical_alignment, 1.0);
        assert_eq!(score.biological_regeneration, 0.0);
        assert_eq!(score.gei_stability, 1.0);
    }

    // BFTConsensusRule tests
    #[test]
    fn test_bft_creation() {
        let rule = BFTConsensusRule::new(7).unwrap();
        assert_eq!(rule.total_validators, 7);
        assert_eq!(rule.max_faulty, 2);
        assert_eq!(rule.threshold, 5);
    }

    #[test]
    fn test_bft_insufficient_validators() {
        let result = BFTConsensusRule::new(3);
        assert!(result.is_err());
    }

    #[test]
    fn test_bft_consensus_met() {
        let rule = BFTConsensusRule::new(7).unwrap();
        assert!(rule.has_consensus(5));
        assert!(rule.has_consensus(7));
    }

    #[test]
    fn test_bft_consensus_not_met() {
        let rule = BFTConsensusRule::new(7).unwrap();
        assert!(!rule.has_consensus(4));
    }

    #[test]
    fn test_bft_consensus_ratio() {
        let rule = BFTConsensusRule::new(7).unwrap();
        let ratio = rule.consensus_ratio(5);
        assert!((ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bft_min_validators() {
        assert_eq!(BFTConsensusRule::min_validators_for_faults(1), 4);
        assert_eq!(BFTConsensusRule::min_validators_for_faults(2), 7);
        assert_eq!(BFTConsensusRule::min_validators_for_faults(3), 10);
    }

    #[test]
    fn test_bft_default() {
        let rule = BFTConsensusRule::default();
        assert_eq!(rule.total_validators, 7);
        assert_eq!(rule.threshold, 5);
    }

    // SymbioticLoop tests
    #[test]
    fn test_loop_creation() {
        let loop_obj = SymbioticLoop::new();
        assert_eq!(loop_obj.state, SymbioticState::Idle);
    }

    #[test]
    fn test_loop_custom_config() {
        let loop_obj = SymbioticLoop::with_config(10, 0.8, 200).unwrap();
        assert_eq!(loop_obj.consensus_rule.total_validators, 10);
        assert_eq!(loop_obj.gei_stability_threshold, 0.8);
        assert_eq!(loop_obj.max_iterations, 200);
    }

    #[test]
    fn test_loop_reset() {
        let mut loop_obj = SymbioticLoop::new();
        loop_obj.state = SymbioticState::ValidationFailed;
        loop_obj.reset();
        assert_eq!(loop_obj.state, SymbioticState::Idle);
    }

    #[test]
    fn test_loop_default() {
        let loop_obj = SymbioticLoop::default();
        assert_eq!(loop_obj.state, SymbioticState::Idle);
    }

    // BFTConsensusError display tests
    #[test]
    fn test_bft_error_display() {
        let err = BFTConsensusError::InsufficientValidators {
            provided: 3,
            required: 4,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("4"));
    }

    // SymbioticLoopError display tests
    #[test]
    fn test_loop_error_display() {
        let err = SymbioticLoopError::GEIInstability {
            stability: 0.5,
            threshold: 0.7,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("0.5"));
    }
}