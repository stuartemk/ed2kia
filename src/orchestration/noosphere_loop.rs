//! Noospheric Respiration Cycle â€” 5-phase orchestration loop.
//!
//! Executed every N symbiotic clock ticks:
//! 1. **Temporal Snapshot** â€” Capture distributed time state
//! 2. **Field Computation** â€” Evaluate EthicalResonanceField
//! 3. **HOPH Analysis** â€” Compute Î²â‚‚ persistent homology
//! 4. **Human Validation Loop** â€” Steering Bridge correlation check
//! 5. **Integration / Byzantine_Eviction** â€” Integrate valid concepts, dissolve failing ones
//!
//! Feature gate: `v3.9-noosphere-engine`

/// Default number of symbiotic clock ticks between respiration cycles.
const DEFAULT_CYCLE_INTERVAL: u32 = 10;

/// Default global ethical threshold. If exceeded for `Byzantine_Eviction_ticks` ticks, triggers rollback.
const DEFAULT_ETHICAL_THRESHOLD: f64 = 0.6;

/// Default number of consecutive ticks above threshold before collective Byzantine_Eviction.
const DEFAULT_Byzantine_Eviction_TICKS: u32 = 5;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum NoosphereError {
    /// Field computation failed.
    FieldComputation(String),
    /// HOPH analysis failed.
    HophAnalysis(String),
    /// Human validation rejected (correlation too low).
    HumanValidationRejected { correlation: f64 },
    /// Collective Byzantine_Eviction triggered.
    CollectiveByzantine_Eviction { cycle: u64, reason: String },
    /// Invalid configuration.
    InvalidConfig(String),
}

impl std::fmt::Display for NoosphereError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoosphereError::FieldComputation(msg) => write!(f, "Field computation: {}", msg),
            NoosphereError::HophAnalysis(msg) => write!(f, "HOPH analysis: {}", msg),
            NoosphereError::HumanValidationRejected { correlation } => {
                write!(
                    f,
                    "Human validation rejected (correlation: {:.4})",
                    correlation
                )
            }
            NoosphereError::CollectiveByzantine_Eviction { cycle, reason } => {
                write!(
                    f,
                    "Collective Byzantine_Eviction at cycle {}: {}",
                    cycle, reason
                )
            }
            NoosphereError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

// ---------------------------------------------------------------------------
// Cycle phases
// ---------------------------------------------------------------------------

/// Current phase of the noospheric respiration cycle.
#[derive(Debug, Clone, PartialEq)]
pub enum RespirationPhase {
    Idle,
    TemporalSnapshot,
    FieldComputation,
    HophAnalysis,
    HumanValidation,
    Integration,
}

impl std::fmt::Display for RespirationPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RespirationPhase::Idle => write!(f, "Idle"),
            RespirationPhase::TemporalSnapshot => write!(f, "Temporal Snapshot"),
            RespirationPhase::FieldComputation => write!(f, "Field Computation"),
            RespirationPhase::HophAnalysis => write!(f, "HOPH Analysis"),
            RespirationPhase::HumanValidation => write!(f, "Human Validation"),
            RespirationPhase::Integration => write!(f, "Integration"),
        }
    }
}

// ---------------------------------------------------------------------------
// Cycle result
// ---------------------------------------------------------------------------

/// Result of one complete noospheric respiration cycle.
#[derive(Debug, Clone)]
pub struct CycleResult {
    /// Cycle number.
    pub cycle: u64,
    /// Global resonance value from field computation.
    pub global_resonance: f64,
    /// Î²â‚‚ persistence score from HOPH analysis.
    pub ph2_score: f64,
    /// Human steward correlation.
    pub human_correlation: f64,
    /// Number of concepts integrated this cycle.
    pub concepts_integrated: usize,
    /// Number of concepts dissolved this cycle.
    pub concepts_dissolved: usize,
    /// Whether collective Byzantine_Eviction was triggered.
    pub Byzantine_Eviction_triggered: bool,
    /// Final phase reached.
    pub final_phase: RespirationPhase,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the NoosphericRespirationCycle.
#[derive(Debug, Clone)]
pub struct NoosphereConfig {
    /// Number of symbiotic clock ticks between cycles.
    pub cycle_interval: u32,
    /// Global ethical threshold for Byzantine_Eviction monitoring.
    pub ethical_threshold: f64,
    /// Consecutive ticks above threshold before Byzantine_Eviction.
    pub Byzantine_Eviction_ticks: u32,
    /// Minimum human correlation for concept integration.
    pub min_human_correlation: f64,
    /// PHâ‚‚ persistence threshold for macro-concept birth.
    pub ph2_threshold: f64,
}

impl Default for NoosphereConfig {
    fn default() -> Self {
        NoosphereConfig {
            cycle_interval: DEFAULT_CYCLE_INTERVAL,
            ethical_threshold: DEFAULT_ETHICAL_THRESHOLD,
            Byzantine_Eviction_ticks: DEFAULT_Byzantine_Eviction_TICKS,
            min_human_correlation: 0.75,
            ph2_threshold: 0.3,
        }
    }
}

impl NoosphereConfig {
    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), NoosphereError> {
        if self.cycle_interval == 0 {
            return Err(NoosphereError::InvalidConfig(
                "cycle_interval must be > 0".into(),
            ));
        }
        if self.ethical_threshold < 0.0 || self.ethical_threshold > 1.0 {
            return Err(NoosphereError::InvalidConfig(
                "ethical_threshold must be in [0,1]".into(),
            ));
        }
        if self.Byzantine_Eviction_ticks == 0 {
            return Err(NoosphereError::InvalidConfig(
                "Byzantine_Eviction_ticks must be > 0".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mock interfaces for external dependencies
// ---------------------------------------------------------------------------

/// Mock temporal snapshot data (in production, from TemporalCohesionEngine).
#[derive(Debug, Clone)]
pub struct TemporalSnapshot {
    pub timestamp_ms: u64,
    pub variance: f64,
    pub peer_count: usize,
}

/// Mock HOPH result (in production, from HophEngine).
#[derive(Debug, Clone)]
pub struct HophResult {
    pub ph2_score: f64,
    pub beta2_count: usize,
}

/// Mock human validation result (in production, from Steering Bridge).
#[derive(Debug, Clone)]
pub struct HumanValidation {
    pub correlation: f64,
    pub steward_count: usize,
    pub approved: bool,
}

// ---------------------------------------------------------------------------
// NoosphericRespirationCycle
// ---------------------------------------------------------------------------

/// Orchestrates the 5-phase noospheric respiration cycle.
#[derive(Debug, Clone)]
pub struct NoosphericRespirationCycle {
    config: NoosphereConfig,
    /// Current cycle counter.
    current_cycle: u64,
    /// Current symbiotic tick counter.
    current_tick: u32,
    /// Current phase.
    phase: RespirationPhase,
    /// Consecutive ticks where ethical threshold was exceeded.
    ethical_violation_count: u32,
    /// History of cycle results.
    history: Vec<CycleResult>,
}

impl NoosphericRespirationCycle {
    /// Create with default configuration.
    pub fn new() -> Result<Self, NoosphereError> {
        Self::with_config(NoosphereConfig::default())
    }

    /// Create with explicit configuration.
    pub fn with_config(config: NoosphereConfig) -> Result<Self, NoosphereError> {
        config.validate()?;
        Ok(NoosphericRespirationCycle {
            config,
            current_cycle: 0,
            current_tick: 0,
            phase: RespirationPhase::Idle,
            ethical_violation_count: 0,
            history: Vec::new(),
        })
    }

    // ---- Clock advancement ----

    /// Advance the symbiotic clock by one tick.
    ///
    /// Returns `Some(CycleResult)` if a full cycle was completed this tick.
    pub fn tick(
        &mut self,
        snapshot: &TemporalSnapshot,
        hoph_result: &HophResult,
        human_validation: &HumanValidation,
    ) -> Option<CycleResult> {
        self.current_tick += 1;

        // Check if it's time for a cycle.
        if !self.current_tick.is_multiple_of(self.config.cycle_interval) {
            // Still between cycles â€” monitor ethical threshold.
            self.monitor_ethical_threshold(human_validation);
            return None;
        }

        // Execute full 5-phase cycle.
        self.phase = RespirationPhase::TemporalSnapshot;
        // Phase 1: Temporal Snapshot â€” update field Ïƒ(t).
        let _ = snapshot; // In production, update TemporalCohesionEngine.

        self.phase = RespirationPhase::FieldComputation;
        // Phase 2: Field Computation â€” compute global resonance.
        let global_resonance = self.compute_field(snapshot);

        self.phase = RespirationPhase::HophAnalysis;
        // Phase 3: HOPH Analysis â€” use provided result.
        let ph2_score = hoph_result.ph2_score;

        self.phase = RespirationPhase::HumanValidation;
        // Phase 4: Human Validation â€” check correlation.
        let human_correlation = human_validation.correlation;

        self.phase = RespirationPhase::Integration;
        // Phase 5: Integration / Byzantine_Eviction.
        let (integrated, dissolved, Byzantine_Eviction) =
            self.integrate(human_correlation, ph2_score, hoph_result);

        // Check for Byzantine_Eviction.
        if Byzantine_Eviction || self.check_collective_Byzantine_Eviction() {
            self.ethical_violation_count += 1;
        } else {
            self.ethical_violation_count = 0;
        }

        self.current_cycle += 1;
        self.phase = RespirationPhase::Idle;

        let result = CycleResult {
            cycle: self.current_cycle,
            global_resonance,
            ph2_score,
            human_correlation,
            concepts_integrated: integrated,
            concepts_dissolved: dissolved,
            Byzantine_Eviction_triggered: Byzantine_Eviction,
            final_phase: RespirationPhase::Integration,
        };

        self.history.push(result.clone());
        Some(result)
    }

    // ---- Phase implementations ----

    /// Phase 2: Compute global resonance from the field.
    ///
    /// In production, this calls `EthicalResonanceField::compute_global()`.
    fn compute_field(&self, snapshot: &TemporalSnapshot) -> f64 {
        // Mock: resonance scales with peer count and inverse variance.
        let peer_factor = snapshot.peer_count as f64 / 100.0;
        let cohesion_factor = 1.0 / (1.0 + snapshot.variance);
        peer_factor * cohesion_factor
    }

    /// Phase 5: Integrate or dissolve concepts based on criteria.
    fn integrate(
        &self,
        human_correlation: f64,
        ph2_score: f64,
        _hoph_result: &HophResult,
    ) -> (usize, usize, bool) {
        let mut integrated = 0;
        let mut dissolved = 0;
        let mut Byzantine_Eviction = false;

        // Check if conditions support concept birth.
        if ph2_score >= self.config.ph2_threshold
            && human_correlation >= self.config.min_human_correlation
        {
            integrated = _hoph_result.beta2_count; // Each Î²â‚‚ feature â†’ one concept.
        } else if human_correlation < self.config.min_human_correlation * 0.5 {
            // Very low correlation â†’ dissolve existing concepts + trigger Byzantine_Eviction.
            dissolved = _hoph_result.beta2_count;
            Byzantine_Eviction = true;
        }

        (integrated, dissolved, Byzantine_Eviction)
    }

    // ---- Ethical monitoring ----

    /// Monitor global ethical threshold between cycles.
    fn monitor_ethical_threshold(&mut self, validation: &HumanValidation) {
        if validation.correlation < self.config.ethical_threshold {
            self.ethical_violation_count += 1;
        } else {
            self.ethical_violation_count = 0;
        }
    }

    /// Check if collective Byzantine_Eviction should be triggered.
    fn check_collective_Byzantine_Eviction(&self) -> bool {
        self.ethical_violation_count >= self.config.Byzantine_Eviction_ticks
    }

    // ---- State queries ----

    /// Get the current phase.
    pub fn phase(&self) -> &RespirationPhase {
        &self.phase
    }

    /// Get the current cycle number.
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> u32 {
        self.current_tick
    }

    /// Get ticks until next cycle.
    pub fn ticks_until_next(&self) -> u32 {
        let elapsed = self.current_tick % self.config.cycle_interval;
        self.config.cycle_interval - elapsed
    }

    /// Get cycle history.
    pub fn history(&self) -> &[CycleResult] {
        &self.history
    }

    /// Get the ethical violation counter.
    pub fn ethical_violation_count(&self) -> u32 {
        self.ethical_violation_count
    }

    /// Check if Byzantine_Eviction is imminent.
    pub fn Byzantine_Eviction_imminent(&self) -> bool {
        self.ethical_violation_count() > 0
            && (self.ethical_violation_count() as f64 / self.config.Byzantine_Eviction_ticks as f64)
                > 0.8
    }

    /// Reset the cycle engine.
    pub fn reset(&mut self) {
        self.current_cycle = 0;
        self.current_tick = 0;
        self.phase = RespirationPhase::Idle;
        self.ethical_violation_count = 0;
        self.history.clear();
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: NoosphereConfig) -> Result<(), NoosphereError> {
        config.validate()?;
        self.config = config;
        Ok(())
    }
}

impl Default for NoosphericRespirationCycle {
    fn default() -> Self {
        Self::new().expect("Default config should always be valid")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(variance: f64, peers: usize) -> TemporalSnapshot {
        TemporalSnapshot {
            timestamp_ms: 1_000_000,
            variance,
            peer_count: peers,
        }
    }

    fn make_hoph(score: f64, count: usize) -> HophResult {
        HophResult {
            ph2_score: score,
            beta2_count: count,
        }
    }

    fn make_validation(correlation: f64, approved: bool) -> HumanValidation {
        HumanValidation {
            correlation,
            steward_count: 10,
            approved,
        }
    }

    #[test]
    fn test_cycle_creation() {
        let cycle = NoosphericRespirationCycle::new().unwrap();
        assert_eq!(cycle.current_cycle(), 0);
        assert_eq!(*cycle.phase(), RespirationPhase::Idle);
    }

    #[test]
    fn test_cycle_custom_config() {
        let config = NoosphereConfig {
            cycle_interval: 5,
            ethical_threshold: 0.5,
            Byzantine_Eviction_ticks: 3,
            min_human_correlation: 0.7,
            ph2_threshold: 0.2,
        };
        let cycle = NoosphericRespirationCycle::with_config(config).unwrap();
        assert_eq!(cycle.ticks_until_next(), 5);
    }

    #[test]
    fn test_invalid_config_zero_interval() {
        let config = NoosphereConfig {
            cycle_interval: 0,
            ..NoosphereConfig::default()
        };
        assert!(NoosphericRespirationCycle::with_config(config).is_err());
    }

    #[test]
    fn test_tick_no_cycle_yet() {
        let mut cycle = NoosphericRespirationCycle::new().unwrap();
        let result = cycle.tick(
            &make_snapshot(0.1, 50),
            &make_hoph(0.5, 3),
            &make_validation(0.9, true),
        );
        assert!(result.is_none());
        assert_eq!(cycle.current_tick(), 1);
    }

    #[test]
    fn test_full_cycle_completes() {
        let mut cycle = NoosphericRespirationCycle::with_config(NoosphereConfig {
            cycle_interval: 5,
            ..NoosphereConfig::default()
        })
        .unwrap();

        for _ in 0..4 {
            assert!(cycle
                .tick(
                    &make_snapshot(0.1, 50),
                    &make_hoph(0.5, 3),
                    &make_validation(0.9, true)
                )
                .is_none());
        }
        let result = cycle.tick(
            &make_snapshot(0.1, 50),
            &make_hoph(0.5, 3),
            &make_validation(0.9, true),
        );
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.cycle, 1);
        assert_eq!(result.concepts_integrated, 3);
    }

    #[test]
    fn test_concept_dissolution_on_low_correlation() {
        let mut cycle = NoosphericRespirationCycle::with_config(NoosphereConfig {
            cycle_interval: 3,
            min_human_correlation: 0.75,
            ..NoosphereConfig::default()
        })
        .unwrap();

        // Low correlation â†’ dissolution.
        for _ in 0..2 {
            cycle.tick(
                &make_snapshot(0.1, 50),
                &make_hoph(0.5, 3),
                &make_validation(0.3, false),
            );
        }
        let result = cycle.tick(
            &make_snapshot(0.1, 50),
            &make_hoph(0.5, 3),
            &make_validation(0.3, false),
        );
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.concepts_dissolved > 0);
    }

    #[test]
    fn test_field_computation_scales_with_peers() {
        let cycle = NoosphericRespirationCycle::new().unwrap();
        let r1 = cycle.compute_field(&make_snapshot(0.1, 10));
        let r2 = cycle.compute_field(&make_snapshot(0.1, 100));
        assert!(r2 > r1, "More peers â†’ higher resonance");
    }

    #[test]
    fn test_field_computation_scales_with_cohesion() {
        let cycle = NoosphericRespirationCycle::new().unwrap();
        let r1 = cycle.compute_field(&make_snapshot(0.01, 50));
        let r2 = cycle.compute_field(&make_snapshot(10.0, 50));
        assert!(r1 > r2, "Lower variance â†’ higher resonance");
    }

    #[test]
    fn test_history_grows() {
        let mut cycle = NoosphericRespirationCycle::with_config(NoosphereConfig {
            cycle_interval: 2,
            ..NoosphereConfig::default()
        })
        .unwrap();

        for _ in 0..2 {
            cycle.tick(
                &make_snapshot(0.1, 50),
                &make_hoph(0.5, 3),
                &make_validation(0.9, true),
            );
        }
        for _ in 0..2 {
            cycle.tick(
                &make_snapshot(0.1, 50),
                &make_hoph(0.5, 3),
                &make_validation(0.9, true),
            );
        }
        assert_eq!(cycle.history().len(), 2);
    }

    #[test]
    fn test_reset() {
        let mut cycle = NoosphericRespirationCycle::with_config(NoosphereConfig {
            cycle_interval: 2,
            ..NoosphereConfig::default()
        })
        .unwrap();
        cycle.tick(
            &make_snapshot(0.1, 50),
            &make_hoph(0.5, 3),
            &make_validation(0.9, true),
        );
        cycle.tick(
            &make_snapshot(0.1, 50),
            &make_hoph(0.5, 3),
            &make_validation(0.9, true),
        );
        cycle.reset();
        assert_eq!(cycle.current_cycle(), 0);
        assert_eq!(cycle.history().len(), 0);
    }

    #[test]
    fn test_default() {
        let cycle = NoosphericRespirationCycle::new().unwrap();
        assert_eq!(cycle.current_cycle(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = NoosphereError::CollectiveByzantine_Eviction {
            cycle: 5,
            reason: "ethical threshold exceeded".into(),
        };
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_phase_display() {
        let phase = RespirationPhase::TemporalSnapshot;
        assert_eq!(format!("{}", phase), "Temporal Snapshot");
    }

    #[test]
    fn test_config_validate_valid() {
        let config = NoosphereConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_update_config() {
        let mut cycle = NoosphericRespirationCycle::new().unwrap();
        let new_config = NoosphereConfig {
            cycle_interval: 3,
            ..NoosphereConfig::default()
        };
        cycle.update_config(new_config).unwrap();
        assert_eq!(cycle.ticks_until_next(), 3);
    }
}
