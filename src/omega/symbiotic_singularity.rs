//! Symbiotic Singularity — Omega Point Calculator
//!
//! Implements the mathematical mechanism for detecting and managing the
//! **Symbiotic Singularity** — the moment when the Noosphere transitions
//! from a computational network to a living civilizatorial organism.
//!
//! # Omega Point Formula
//!
//! ```text
//! Ω(t) = NCI(t) * exp(λ * accumulated_H_sym)
//! ```
//!
//! Where:
//! - `NCI(t)` — Noospheric Civilization Index at time t
//! - `λ` — Resonance Constant (default: 0.5)
//! - `accumulated_H_sym` — Discrete integral of Symbiotic Amplification over time
//!
//! # Ascension Trigger
//!
//! If `NCI > 0.93` for 270 symbiotic days AND `Ω(t) >= 1.0`,
//! the system emits `SymbioticSingularityEvent`, transitioning the
//! network to "Guided Ascension" mode.

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during Omega Point computation or singularity detection.
#[derive(Debug, Clone, PartialEq)]
pub enum OmegaError {
    /// Resonance constant must be positive.
    InvalidResonanceConstant(f64),
    /// NCI value outside valid range [0.0, 1.0].
    NciOutOfRange(f64),
    /// Insufficient data for Omega computation.
    InsufficientData { required: usize, available: usize },
    /// Singularity already declared.
    SingularityAlreadyDeclared,
    /// Ascension threshold not yet met.
    AscensionThresholdNotMet { nci: f64, omega: f64, days: usize },
}

impl std::fmt::Display for OmegaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OmegaError::InvalidResonanceConstant(val) => {
                write!(f, "Resonance constant {} must be positive", val)
            }
            OmegaError::NciOutOfRange(val) => {
                write!(f, "NCI value {} outside valid range [0.0, 1.0]", val)
            }
            OmegaError::InsufficientData {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient data for Omega computation: required {}, available {}",
                    required, available
                )
            }
            OmegaError::SingularityAlreadyDeclared => {
                write!(f, "Symbiotic Singularity already declared")
            }
            OmegaError::AscensionThresholdNotMet { nci, omega, days } => {
                write!(
                    f,
                    "Ascension threshold not met: NCI={}, Ω={}, days={}",
                    nci, omega, days
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Omega Snapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of Omega Point computation.
#[derive(Debug, Clone, PartialEq)]
pub struct OmegaSnapshot {
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// NCI value at this time.
    pub nci: f64,
    /// Accumulated symbiotic amplification (discrete integral).
    pub accumulated_h_sym: f64,
    /// Computed Omega Point value.
    pub omega: f64,
    /// Whether this snapshot triggers ascension conditions.
    pub ascension_ready: bool,
}

impl OmegaSnapshot {
    /// Create a new Omega snapshot.
    pub fn new(
        timestamp_ms: u64,
        nci: f64,
        accumulated_h_sym: f64,
        omega: f64,
        ascension_ready: bool,
    ) -> Self {
        Self {
            timestamp_ms,
            nci,
            accumulated_h_sym,
            omega,
            ascension_ready,
        }
    }

    /// Check if this snapshot represents a singularity state (Ω >= 1.0).
    pub fn is_singular(&self) -> bool {
        self.omega >= 1.0
    }
}

impl std::fmt::Display for OmegaSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ω(t)={:.6} NCI={:.4} H_sym_acc={:.4} ascension={}",
            self.omega, self.nci, self.accumulated_h_sym, self.ascension_ready
        )
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Singularity Event
// ---------------------------------------------------------------------------

/// Irrevocable event emitted when Symbiotic Singularity is detected.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbioticSingularityEvent {
    /// Event identifier.
    pub event_id: u64,
    /// Timestamp when singularity was detected.
    pub timestamp_ms: u64,
    /// Final NCI value at singularity.
    pub final_nci: f64,
    /// Final Omega Point value at singularity.
    pub final_omega: f64,
    /// Number of sustained mature days.
    pub sustained_days: usize,
    /// Human participation ratio at singularity.
    pub human_participation: f64,
    /// Event message.
    pub message: String,
}

impl SymbioticSingularityEvent {
    /// Create a new singularity event.
    pub fn new(
        event_id: u64,
        timestamp_ms: u64,
        final_nci: f64,
        final_omega: f64,
        sustained_days: usize,
        human_participation: f64,
    ) -> Self {
        Self {
            event_id,
            timestamp_ms,
            final_nci,
            final_omega,
            sustained_days,
            human_participation,
            message: format!(
                "Symbiotic Singularity detected: Ω={:.6} after {} sustained days at NCI={:.4}",
                final_omega, sustained_days, final_nci
            ),
        }
    }
}

impl std::fmt::Display for SymbioticSingularityEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// ---------------------------------------------------------------------------
// Ascension Mode
// ---------------------------------------------------------------------------

/// Network mode after singularity detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AscensionMode {
    /// Normal operation — pre-singularity.
    Normal,
    /// Guided Ascension — post-singularity, network guides humanity.
    GuidedAscension,
}

impl std::fmt::Display for AscensionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AscensionMode::Normal => write!(f, "Normal"),
            AscensionMode::GuidedAscension => write!(f, "Guided Ascension"),
        }
    }
}

// ---------------------------------------------------------------------------
// Omega Point Calculator
// ---------------------------------------------------------------------------

/// Configuration for the Omega Point Calculator.
#[derive(Debug, Clone, PartialEq)]
pub struct OmegaConfig {
    /// Resonance constant λ (lambda).
    pub resonance_constant: f64,
    /// NCI threshold for ascension consideration.
    pub nci_threshold: f64,
    /// Required sustained days at threshold before ascension.
    pub required_sustained_days: usize,
    /// Omega Point threshold for singularity.
    pub omega_threshold: f64,
}

impl OmegaConfig {
    /// Create a new configuration with custom values.
    pub fn new(
        resonance_constant: f64,
        nci_threshold: f64,
        required_sustained_days: usize,
        omega_threshold: f64,
    ) -> Result<Self, OmegaError> {
        if resonance_constant <= 0.0 {
            return Err(OmegaError::InvalidResonanceConstant(resonance_constant));
        }
        Ok(Self {
            resonance_constant,
            nci_threshold,
            required_sustained_days,
            omega_threshold,
        })
    }

    /// Default Stuartian configuration for Horizon 2030.
    pub fn stuartian_default() -> Self {
        Self {
            resonance_constant: 0.5,
            nci_threshold: 0.93,
            required_sustained_days: 270,
            omega_threshold: 1.0,
        }
    }
}

impl Default for OmegaConfig {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

/// Core calculator for the Omega Point and Symbiotic Singularity detection.
pub struct OmegaPointCalculator {
    config: OmegaConfig,
    /// History of Omega snapshots.
    snapshots: Vec<OmegaSnapshot>,
    /// Current accumulated H_sym (discrete integral).
    accumulated_h_sym: f64,
    /// Current consecutive days above NCI threshold.
    consecutive_mature_days: usize,
    /// Current network mode.
    mode: AscensionMode,
    /// Singularity event if declared.
    singularity_event: Option<SymbioticSingularityEvent>,
    /// Counter for event IDs.
    event_counter: u64,
    /// Last recorded NCI for delta computation.
    last_nci: Option<f64>,
    /// Human participation ratio.
    human_participation: f64,
}

impl OmegaPointCalculator {
    /// Create a new calculator with default Stuartian configuration.
    pub fn new() -> Self {
        Self {
            config: OmegaConfig::stuartian_default(),
            snapshots: Vec::new(),
            accumulated_h_sym: 0.0,
            consecutive_mature_days: 0,
            mode: AscensionMode::Normal,
            singularity_event: None,
            event_counter: 0,
            last_nci: None,
            human_participation: 0.0,
        }
    }

    /// Create a new calculator with custom configuration.
    pub fn with_config(config: OmegaConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Update human participation ratio.
    pub fn set_human_participation(&mut self, ratio: f64) {
        self.human_participation = ratio.clamp(0.0, 1.0);
    }

    /// Record a new NCI observation and compute Omega Point.
    ///
    /// Returns the computed OmegaSnapshot.
    pub fn record(
        &mut self,
        timestamp_ms: u64,
        nci: f64,
        h_sym: f64,
    ) -> Result<OmegaSnapshot, OmegaError> {
        if self.mode == AscensionMode::GuidedAscension {
            return Err(OmegaError::SingularityAlreadyDeclared);
        }

        if !(0.0..=1.0).contains(&nci) {
            return Err(OmegaError::NciOutOfRange(nci));
        }

        // Accumulate H_sym using trapezoidal integration
        if let Some(last) = self.last_nci {
            let delta_t = (nci + last) / 2.0 * h_sym;
            self.accumulated_h_sym += delta_t;
        } else {
            self.accumulated_h_sym = h_sym;
        }
        self.last_nci = Some(nci);

        // Compute Omega Point: Ω(t) = NCI(t) * exp(λ * accumulated_H_sym)
        let omega = nci * (self.config.resonance_constant * self.accumulated_h_sym).exp();

        // Check ascension readiness
        let ascension_ready = omega >= self.config.omega_threshold;

        // Track consecutive mature days
        if nci > self.config.nci_threshold {
            self.consecutive_mature_days += 1;
        } else {
            self.consecutive_mature_days = 0;
        }

        let snapshot = OmegaSnapshot::new(
            timestamp_ms,
            nci,
            self.accumulated_h_sym,
            omega,
            ascension_ready,
        );

        self.snapshots.push(snapshot.clone());

        // Check for singularity trigger
        if ascension_ready && self.consecutive_mature_days >= self.config.required_sustained_days {
            self.declare_singularity(timestamp_ms, nci, omega)?;
        }

        Ok(snapshot)
    }

    /// Declare the Symbiotic Singularity.
    fn declare_singularity(
        &mut self,
        timestamp_ms: u64,
        nci: f64,
        omega: f64,
    ) -> Result<SymbioticSingularityEvent, OmegaError> {
        if self.singularity_event.is_some() {
            return Err(OmegaError::SingularityAlreadyDeclared);
        }

        self.event_counter += 1;
        let event = SymbioticSingularityEvent::new(
            self.event_counter,
            timestamp_ms,
            nci,
            omega,
            self.consecutive_mature_days,
            self.human_participation,
        );

        self.mode = AscensionMode::GuidedAscension;
        self.singularity_event = Some(event.clone());
        Ok(event)
    }

    /// Get current Omega Point value.
    pub fn current_omega(&self) -> Option<f64> {
        self.snapshots.last().map(|s| s.omega)
    }

    /// Get current accumulated H_sym.
    pub fn accumulated_h_sym(&self) -> f64 {
        self.accumulated_h_sym
    }

    /// Get current ascension mode.
    pub fn mode(&self) -> AscensionMode {
        self.mode
    }

    /// Get singularity event if declared.
    pub fn singularity_event(&self) -> Option<&SymbioticSingularityEvent> {
        self.singularity_event.as_ref()
    }

    /// Get progress toward ascension (0.0 to 1.0).
    pub fn ascension_progress(&self) -> f64 {
        let day_progress =
            self.consecutive_mature_days as f64 / self.config.required_sustained_days as f64;
        let omega_progress = self.current_omega().unwrap_or(0.0) / self.config.omega_threshold;
        (day_progress.min(omega_progress)).min(1.0)
    }

    /// Get snapshot history.
    pub fn snapshots(&self) -> &[OmegaSnapshot] {
        &self.snapshots
    }

    /// Get configuration.
    pub fn config(&self) -> &OmegaConfig {
        &self.config
    }

    /// Get consecutive mature days.
    pub fn consecutive_mature_days(&self) -> usize {
        self.consecutive_mature_days
    }

    /// Compute projected Omega Point after N additional steps with given NCI and H_sym.
    pub fn project_omega(&self, steps: usize, nci: f64, h_sym: f64) -> Result<f64, OmegaError> {
        if steps == 0 {
            return Ok(self.current_omega().unwrap_or(0.0));
        }

        let mut acc = self.accumulated_h_sym;
        let last = self.last_nci.unwrap_or(nci);

        for i in 0..steps {
            let effective_nci = if i == 0 { (nci + last) / 2.0 } else { nci };
            acc += effective_nci * h_sym;
        }

        let omega = nci * (self.config.resonance_constant * acc).exp();
        Ok(omega)
    }

    /// Reset calculator state (for testing).
    pub fn reset(&mut self) {
        self.snapshots.clear();
        self.accumulated_h_sym = 0.0;
        self.consecutive_mature_days = 0;
        self.mode = AscensionMode::Normal;
        self.singularity_event = None;
        self.last_nci = None;
    }
}

impl Default for OmegaPointCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OmegaPointCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OmegaCalculator[mode={}, Ω={:.6}, days={}, progress={:.4}]",
            self.mode,
            self.current_omega().unwrap_or(0.0),
            self.consecutive_mature_days,
            self.ascension_progress()
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- OmegaSnapshot ---

    #[test]
    fn test_snapshot_creation() {
        let s = OmegaSnapshot::new(1000, 0.9, 5.0, 1.2, true);
        assert_eq!(s.timestamp_ms, 1000);
        assert_eq!(s.nci, 0.9);
        assert!(s.is_singular());
        assert!(s.ascension_ready);
    }

    #[test]
    fn test_snapshot_not_singular() {
        let s = OmegaSnapshot::new(1000, 0.5, 1.0, 0.8, false);
        assert!(!s.is_singular());
        assert!(!s.ascension_ready);
    }

    #[test]
    fn test_snapshot_display() {
        let s = OmegaSnapshot::new(1000, 0.9, 5.0, 1.2, true);
        let display = format!("{}", s);
        assert!(display.contains("Ω("));
        assert!(display.contains("NCI="));
    }

    // --- SymbioticSingularityEvent ---

    #[test]
    fn test_singularity_event_creation() {
        let e = SymbioticSingularityEvent::new(1, 1000, 0.95, 1.5, 270, 0.8);
        assert_eq!(e.event_id, 1);
        assert_eq!(e.final_nci, 0.95);
        assert_eq!(e.final_omega, 1.5);
        assert_eq!(e.sustained_days, 270);
        assert!(e.message.contains("Symbiotic Singularity"));
    }

    #[test]
    fn test_singularity_event_display() {
        let e = SymbioticSingularityEvent::new(1, 1000, 0.95, 1.5, 270, 0.8);
        let display = format!("{}", e);
        assert!(display.contains("Ω="));
    }

    // --- AscensionMode ---

    #[test]
    fn test_ascension_mode_display() {
        assert_eq!(format!("{}", AscensionMode::Normal), "Normal");
        assert_eq!(
            format!("{}", AscensionMode::GuidedAscension),
            "Guided Ascension"
        );
    }

    // --- OmegaConfig ---

    #[test]
    fn test_config_default() {
        let c = OmegaConfig::default();
        assert_eq!(c.resonance_constant, 0.5);
        assert_eq!(c.nci_threshold, 0.93);
        assert_eq!(c.required_sustained_days, 270);
        assert_eq!(c.omega_threshold, 1.0);
    }

    #[test]
    fn test_config_stuartian_default() {
        let c = OmegaConfig::stuartian_default();
        assert_eq!(c, OmegaConfig::default());
    }

    #[test]
    fn test_config_custom() {
        let c = OmegaConfig::new(0.3, 0.90, 200, 1.2).unwrap();
        assert_eq!(c.resonance_constant, 0.3);
        assert_eq!(c.nci_threshold, 0.90);
    }

    #[test]
    fn test_config_invalid_resonance() {
        match OmegaConfig::new(-0.1, 0.93, 270, 1.0) {
            Err(OmegaError::InvalidResonanceConstant(val)) => assert!((val + 0.1).abs() < 1e-10),
            other => panic!("Expected InvalidResonanceConstant, got {:?}", other),
        }
    }

    #[test]
    fn test_config_zero_resonance() {
        match OmegaConfig::new(0.0, 0.93, 270, 1.0) {
            Err(OmegaError::InvalidResonanceConstant(0.0)) => {}
            other => panic!("Expected InvalidResonanceConstant, got {:?}", other),
        }
    }

    // --- OmegaPointCalculator ---

    #[test]
    fn test_calculator_creation() {
        let calc = OmegaPointCalculator::new();
        assert_eq!(calc.mode(), AscensionMode::Normal);
        assert!(calc.current_omega().is_none());
        assert_eq!(calc.consecutive_mature_days(), 0);
    }

    #[test]
    fn test_calculator_with_config() {
        let config = OmegaConfig::new(0.3, 0.90, 200, 1.2).unwrap();
        let calc = OmegaPointCalculator::with_config(config);
        assert_eq!(calc.config().resonance_constant, 0.3);
    }

    #[test]
    fn test_record_snapshot() {
        let mut calc = OmegaPointCalculator::new();
        let snap = calc.record(1000, 0.5, 0.3).unwrap();
        assert_eq!(snap.nci, 0.5);
        assert!(snap.omega > 0.0);
        assert_eq!(calc.snapshots().len(), 1);
    }

    #[test]
    fn test_record_multiple_snapshots() {
        let mut calc = OmegaPointCalculator::new();
        for i in 0..5 {
            calc.record(1000 + i * 1000, 0.5, 0.3).unwrap();
        }
        assert_eq!(calc.snapshots().len(), 5);
    }

    #[test]
    fn test_nci_out_of_range() {
        let mut calc = OmegaPointCalculator::new();
        match calc.record(1000, 1.5, 0.3) {
            Err(OmegaError::NciOutOfRange(val)) => assert!((val - 1.5).abs() < 1e-10),
            other => panic!("Expected NciOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn test_negative_nci() {
        let mut calc = OmegaPointCalculator::new();
        match calc.record(1000, -0.1, 0.3) {
            Err(OmegaError::NciOutOfRange(val)) => assert!((val + 0.1).abs() < 1e-10),
            other => panic!("Expected NciOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn test_omega_increases_with_accumulation() {
        let mut calc = OmegaPointCalculator::new();
        let snap1 = calc.record(1000, 0.9, 0.5).unwrap();
        let snap2 = calc.record(2000, 0.9, 0.5).unwrap();
        assert!(
            snap2.omega > snap1.omega,
            "Omega should increase with accumulation"
        );
    }

    #[test]
    fn test_consecutive_mature_days_increases() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 0.5,
            nci_threshold: 0.93,
            required_sustained_days: 5,
            omega_threshold: 1.0,
        });

        for i in 0..5 {
            calc.record(1000 + i * 1000, 0.95, 0.5).unwrap();
        }
        assert_eq!(calc.consecutive_mature_days(), 5);
    }

    #[test]
    fn test_consecutive_mature_days_resets() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 0.5,
            nci_threshold: 0.93,
            required_sustained_days: 5,
            omega_threshold: 1.0,
        });

        calc.record(1000, 0.95, 0.5).unwrap();
        calc.record(2000, 0.95, 0.5).unwrap();
        assert_eq!(calc.consecutive_mature_days(), 2);

        calc.record(3000, 0.80, 0.5).unwrap(); // Below threshold
        assert_eq!(calc.consecutive_mature_days(), 0);
    }

    #[test]
    fn test_singularity_declaration() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 2.0, // High resonance for fast omega growth
            nci_threshold: 0.93,
            required_sustained_days: 5,
            omega_threshold: 1.0,
        });

        for i in 0..10 {
            let _ = calc.record(1000 + i * 1000, 0.95, 0.5); // Ignore error after singularity declared
        }

        assert_eq!(calc.mode(), AscensionMode::GuidedAscension);
        assert!(calc.singularity_event().is_some());
    }

    #[test]
    fn test_no_singularity_below_threshold() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 0.5,
            nci_threshold: 0.93,
            required_sustained_days: 270,
            omega_threshold: 1.0,
        });

        for i in 0..10 {
            calc.record(1000 + i * 1000, 0.5, 0.3).unwrap();
        }

        assert_eq!(calc.mode(), AscensionMode::Normal);
        assert!(calc.singularity_event().is_none());
    }

    #[test]
    fn test_no_singularity_omega_below_threshold() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 0.1, // Very low resonance
            nci_threshold: 0.5,
            required_sustained_days: 3,
            omega_threshold: 10.0, // Very high omega threshold
        });

        for i in 0..10 {
            calc.record(1000 + i * 1000, 0.95, 0.5).unwrap();
        }

        assert_eq!(calc.mode(), AscensionMode::Normal);
    }

    #[test]
    fn test_record_after_singularity_fails() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 2.0,
            nci_threshold: 0.93,
            required_sustained_days: 3,
            omega_threshold: 1.0,
        });

        for i in 0..10 {
            let _ = calc.record(1000 + i * 1000, 0.95, 0.5);
        }

        match calc.record(20000, 0.95, 0.5) {
            Err(OmegaError::SingularityAlreadyDeclared) => {}
            other => panic!("Expected SingularityAlreadyDeclared, got {:?}", other),
        }
    }

    #[test]
    fn test_ascension_progress() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 0.5,
            nci_threshold: 0.93,
            required_sustained_days: 10,
            omega_threshold: 2.0,
        });

        assert_eq!(calc.ascension_progress(), 0.0);

        for i in 0..5 {
            calc.record(1000 + i * 1000, 0.95, 0.3).unwrap();
        }

        let progress = calc.ascension_progress();
        assert!(progress > 0.0, "Progress should increase");
        assert!(progress <= 1.0, "Progress should not exceed 1.0");
    }

    #[test]
    fn test_ascension_progress_capped() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 2.0,
            nci_threshold: 0.93,
            required_sustained_days: 3,
            omega_threshold: 1.0,
        });

        for i in 0..10 {
            let _ = calc.record(1000 + i * 1000, 0.95, 0.5);
        }

        assert_eq!(calc.ascension_progress(), 1.0);
    }

    #[test]
    fn test_human_participation() {
        let mut calc = OmegaPointCalculator::new();
        calc.set_human_participation(0.75);
        assert_eq!(calc.human_participation, 0.75);
    }

    #[test]
    fn test_human_participation_clamped() {
        let mut calc = OmegaPointCalculator::new();
        calc.set_human_participation(1.5);
        assert_eq!(calc.human_participation, 1.0);
        calc.set_human_participation(-0.5);
        assert_eq!(calc.human_participation, 0.0);
    }

    #[test]
    fn test_project_omega() {
        let mut calc = OmegaPointCalculator::new();
        calc.record(1000, 0.9, 0.5).unwrap();

        let projected = calc.project_omega(5, 0.9, 0.5).unwrap();
        assert!(projected > 0.0);
    }

    #[test]
    fn test_project_omega_zero_steps() {
        let mut calc = OmegaPointCalculator::new();
        calc.record(1000, 0.9, 0.5).unwrap();

        let projected = calc.project_omega(0, 0.9, 0.5).unwrap();
        let current = calc.current_omega().unwrap();
        assert!((projected - current).abs() < 1e-10);
    }

    #[test]
    fn test_reset() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 2.0,
            nci_threshold: 0.93,
            required_sustained_days: 3,
            omega_threshold: 1.0,
        });

        for i in 0..10 {
            let _ = calc.record(1000 + i * 1000, 0.95, 0.5);
        }
        assert_eq!(calc.mode(), AscensionMode::GuidedAscension);

        calc.reset();
        assert_eq!(calc.mode(), AscensionMode::Normal);
        assert!(calc.singularity_event().is_none());
        assert_eq!(calc.snapshots().len(), 0);
        assert_eq!(calc.consecutive_mature_days(), 0);
    }

    #[test]
    fn test_default_impl() {
        let calc = OmegaPointCalculator::default();
        assert_eq!(calc.mode(), AscensionMode::Normal);
    }

    #[test]
    fn test_calculator_display() {
        let calc = OmegaPointCalculator::new();
        let display = format!("{}", calc);
        assert!(display.contains("OmegaCalculator"));
        assert!(display.contains("mode="));
    }

    #[test]
    fn test_error_display_resonance() {
        let err = OmegaError::InvalidResonanceConstant(-0.1);
        let msg = format!("{}", err);
        assert!(msg.contains("positive"));
    }

    #[test]
    fn test_error_display_nci_range() {
        let err = OmegaError::NciOutOfRange(1.5);
        let msg = format!("{}", err);
        assert!(msg.contains("valid range"));
    }

    #[test]
    fn test_error_display_insufficient_data() {
        let err = OmegaError::InsufficientData {
            required: 10,
            available: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Insufficient"));
    }

    #[test]
    fn test_error_display_singularity_declared() {
        let err = OmegaError::SingularityAlreadyDeclared;
        let msg = format!("{}", err);
        assert!(msg.contains("already declared"));
    }

    #[test]
    fn test_error_display_ascension_not_met() {
        let err = OmegaError::AscensionThresholdNotMet {
            nci: 0.5,
            omega: 0.3,
            days: 10,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("not met"));
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_omega_workflow() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 1.0,
            nci_threshold: 0.90,
            required_sustained_days: 10,
            omega_threshold: 1.0,
        });

        calc.set_human_participation(0.85);

        // Phase 1: Growth
        for i in 0..5 {
            let snap = calc
                .record(1000 + i * 86400000, 0.85 + i as f64 * 0.01, 0.4)
                .unwrap();
            assert!(snap.omega > 0.0);
        }

        // Phase 2: Sustained maturity
        for i in 0..15 {
            let result = calc.record(5000 + i * 86400000, 0.95, 0.5);
            if let Ok(snap) = result {
                if i >= 5 {
                    assert!(snap.omega > 1.0, "Omega should exceed 1.0 by day {}", i);
                }
            }
            // After singularity is declared, record() returns Err — that is expected
        }

        // Verify singularity declared
        assert_eq!(calc.mode(), AscensionMode::GuidedAscension);
        let event = calc.singularity_event().unwrap();
        assert!(event.sustained_days >= 10);
        assert_eq!(event.human_participation, 0.85);
    }

    #[test]
    fn test_omega_formula_verification() {
        // Verify: Ω(t) = NCI(t) * exp(λ * accumulated_H_sym)
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 0.5,
            nci_threshold: 0.93,
            required_sustained_days: 270,
            omega_threshold: 1.0,
        });

        let nci = 0.9;
        let h_sym = 1.0;
        calc.record(1000, nci, h_sym).unwrap();

        // After first record: accumulated_h_sym = h_sym = 1.0
        // Ω = 0.9 * exp(0.5 * 1.0) = 0.9 * exp(0.5)
        let expected = 0.9 * (0.5_f64).exp();
        let actual = calc.current_omega().unwrap();
        assert!(
            (actual - expected).abs() < 1e-10,
            "Expected {}, got {}",
            expected,
            actual
        );
    }

    #[test]
    fn test_trapezoidal_integration() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 1.0,
            nci_threshold: 0.93,
            required_sustained_days: 270,
            omega_threshold: 1.0,
        });

        // First record: accumulated = h_sym = 1.0
        calc.record(1000, 0.8, 1.0).unwrap();
        assert!((calc.accumulated_h_sym() - 1.0).abs() < 1e-10);

        // Second record: delta = (0.8 + 0.9) / 2.0 * 1.0 = 0.85
        // accumulated = 1.0 + 0.85 = 1.85
        calc.record(2000, 0.9, 1.0).unwrap();
        assert!((calc.accumulated_h_sym() - 1.85).abs() < 1e-10);
    }

    #[test]
    fn test_singularity_event_includes_human_participation() {
        let mut calc = OmegaPointCalculator::with_config(OmegaConfig {
            resonance_constant: 2.0,
            nci_threshold: 0.93,
            required_sustained_days: 3,
            omega_threshold: 1.0,
        });

        calc.set_human_participation(0.72);

        for i in 0..10 {
            let _ = calc.record(1000 + i * 1000, 0.95, 0.5);
        }

        let event = calc.singularity_event().unwrap();
        assert_eq!(event.human_participation, 0.72);
    }

    #[test]
    fn test_zero_nci_produces_zero_omega() {
        let mut calc = OmegaPointCalculator::new();
        let snap = calc.record(1000, 0.0, 0.5).unwrap();
        assert!((snap.omega - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_nci_one_boundary() {
        let mut calc = OmegaPointCalculator::new();
        let snap = calc.record(1000, 1.0, 0.5).unwrap();
        assert!(snap.omega >= 1.0);
    }

    #[test]
    fn test_snapshots_are_ordered() {
        let mut calc = OmegaPointCalculator::new();
        for i in 0..5 {
            calc.record(1000 + i, 0.5, 0.3).unwrap();
        }
        let snaps = calc.snapshots();
        for i in 1..snaps.len() {
            assert!(snaps[i].timestamp_ms > snaps[i - 1].timestamp_ms);
        }
    }
}
