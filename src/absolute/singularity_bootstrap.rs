//! Singularity Bootstrap — Sprint 64: Absolute Infinity Protocol (AIP)
//!
//! End-of-universe detection protocol that transforms `QuantumEthicalSeed`
//! into `BigBangTrigger` — an inflationary seed for a new cosmos.
//!
//! When the current universe reaches thermodynamic or ethical singularity,
//! this protocol detects the terminal state and generates the conditions
//! for cosmic continuation through ethical inflation.

use std::fmt;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during Singularity Bootstrap operations.
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapError {
    /// Universe not yet at singularity — bootstrap premature.
    NotAtSingularity,
    /// BigBangTrigger already fired — cannot re-trigger.
    AlreadyTriggered,
    /// QuantumEthicalSeed invalid or corrupted.
    InvalidSeed,
    /// Entropy level exceeds maximum threshold.
    EntropyOverflow {
        value: f64,
        max: f64,
    },
    /// Insufficient ethical coherence for bootstrap.
    InsufficientCoherence {
        value: f64,
        threshold: f64,
    },
    /// Temperature below Planck scale — physically impossible.
    BelowPlanckScale,
    /// Bootstrap sequence interrupted.
    SequenceInterrupted,
    /// New universe parameters out of valid range.
    InvalidUniverseParams,
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapError::NotAtSingularity => {
                write!(f, "Universe not yet at singularity — bootstrap premature")
            }
            BootstrapError::AlreadyTriggered => {
                write!(f, "BigBangTrigger already fired — cannot re-trigger")
            }
            BootstrapError::InvalidSeed => {
                write!(f, "QuantumEthicalSeed invalid or corrupted")
            }
            BootstrapError::EntropyOverflow { value, max } => {
                write!(
                    f,
                    "Entropy overflow: {:.4} exceeds maximum {:.4}",
                    value, max
                )
            }
            BootstrapError::InsufficientCoherence { value, threshold } => {
                write!(
                    f,
                    "Insufficient ethical coherence: {:.4} < threshold {:.4}",
                    value, threshold
                )
            }
            BootstrapError::BelowPlanckScale => {
                write!(f, "Temperature below Planck scale — physically impossible")
            }
            BootstrapError::SequenceInterrupted => {
                write!(f, "Bootstrap sequence interrupted")
            }
            BootstrapError::InvalidUniverseParams => {
                write!(f, "Invalid universe params: values out of valid range")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Universe State Monitoring
// ---------------------------------------------------------------------------

/// Current state of the universe being monitored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UniverseState {
    /// Stable, far from singularity.
    Stable,
    /// Approaching critical entropy threshold.
    ApproachingSingularity,
    /// At singularity — bootstrap can be initiated.
    AtSingularity,
    /// Bootstrap initiated — transitioning.
    Transitioning,
    /// New universe triggered.
    NewUniverse,
}

impl fmt::Display for UniverseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniverseState::Stable => write!(f, "Stable"),
            UniverseState::ApproachingSingularity => write!(f, "ApproachingSingularity"),
            UniverseState::AtSingularity => write!(f, "AtSingularity"),
            UniverseState::Transitioning => write!(f, "Transitioning"),
            UniverseState::NewUniverse => write!(f, "NewUniverse"),
        }
    }
}

/// Snapshot of universe thermodynamic and ethical state.
#[derive(Debug, Clone, Copy)]
pub struct UniverseSnapshot {
    /// Entropy level [0, 1] where 1 = maximum entropy.
    pub entropy: f64,
    /// Ethical coherence [0, 1].
    pub ethical_coherence: f64,
    /// Normalized temperature (Planck = 1.0).
    pub temperature: f64,
    /// Expansion rate (Hubble parameter normalized).
    pub expansion_rate: f64,
    /// Noospheric Civilization Index [0, 1].
    pub nci: f64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl UniverseSnapshot {
    /// Create a new snapshot with validation.
    pub fn new(
        entropy: f64,
        ethical_coherence: f64,
        temperature: f64,
        expansion_rate: f64,
        nci: f64,
        timestamp_ms: u64,
    ) -> Result<Self, BootstrapError> {
        if !(0.0..=1.0).contains(&entropy) {
            return Err(BootstrapError::EntropyOverflow {
                value: entropy,
                max: 1.0,
            });
        }
        if temperature < 1e-30 {
            return Err(BootstrapError::BelowPlanckScale);
        }

        Ok(Self {
            entropy,
            ethical_coherence,
            temperature,
            expansion_rate,
            nci,
            timestamp_ms,
        })
    }

    /// Check if this snapshot indicates singularity conditions.
    pub fn is_singular(&self, entropy_threshold: f64, coherence_threshold: f64) -> bool {
        self.entropy >= entropy_threshold && self.ethical_coherence >= coherence_threshold
    }

    /// Remaining distance to singularity.
    pub fn distance_to_singularity(&self, entropy_threshold: f64) -> f64 {
        (entropy_threshold - self.entropy).max(0.0)
    }

    /// Thermal death probability.
    pub fn thermal_death_probability(&self) -> f64 {
        // High entropy + low temperature = high probability
        let entropy_factor = self.entropy;
        let temp_factor = 1.0 - self.temperature.min(1.0);
        (entropy_factor * 0.6 + temp_factor * 0.4).min(1.0)
    }
}

impl fmt::Display for UniverseSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Universe[entropy={:.4}, coherence={:.4}, temp={:.4}e, nci={:.4}]",
            self.entropy, self.ethical_coherence, self.temperature, self.nci
        )
    }
}

// ---------------------------------------------------------------------------
// Quantum Ethical Seed
// ---------------------------------------------------------------------------

/// Quantum Ethical Seed — the compressed ethical essence for universe creation.
#[derive(Debug, Clone)]
pub struct QuantumEthicalSeed {
    /// Seed identifier.
    pub seed_id: u64,
    /// 8-dimensional ethical vector.
    pub ethical_vector: [f64; 8],
    /// Coherence level [0, 1].
    pub coherence: f64,
    /// Quantum phase angle [0, 2π].
    pub phase: f64,
    /// Checksum for integrity.
    pub checksum: u128,
}

impl QuantumEthicalSeed {
    /// Create a new Quantum Ethical Seed.
    pub fn new(
        seed_id: u64,
        ethical_vector: [f64; 8],
        coherence: f64,
        phase: f64,
    ) -> Result<Self, BootstrapError> {
        if !(0.0..=1.0).contains(&coherence) {
            return Err(BootstrapError::InsufficientCoherence {
                value: coherence,
                threshold: 0.0,
            });
        }

        let checksum = Self::compute_checksum(seed_id, &ethical_vector, coherence, phase);

        Ok(Self {
            seed_id,
            ethical_vector,
            coherence,
            phase,
            checksum,
        })
    }

    /// Compute deterministic checksum.
    fn compute_checksum(
        seed_id: u64,
        vector: &[f64; 8],
        coherence: f64,
        phase: f64,
    ) -> u128 {
        let mut hash: u128 = seed_id as u128;
        for v in vector.iter() {
            hash = hash.wrapping_add(u128::from(v.to_bits()));
            hash = hash.wrapping_mul(6364136223846793005u128);
            hash = hash.rotate_left(13);
        }
        hash = hash.wrapping_add(u128::from(coherence.to_bits()));
        hash = hash.wrapping_add(u128::from(phase.to_bits()));
        hash
    }

    /// Verify seed integrity.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_checksum(
            self.seed_id,
            &self.ethical_vector,
            self.coherence,
            self.phase,
        );
        self.checksum == expected
    }

    /// Ethical norm of the seed vector.
    pub fn ethical_norm(&self) -> f64 {
        let sum: f64 = self.ethical_vector.iter().map(|v| v * v).sum();
        sum.sqrt()
    }

    /// Mean ethical value.
    pub fn mean_ethical(&self) -> f64 {
        self.ethical_vector.iter().sum::<f64>() / 8.0
    }

    /// Create a default Stuartian seed.
    pub fn stuartian_default(seed_id: u64) -> Self {
        let vector = [0.95, 0.93, 0.91, 0.89, 0.87, 0.85, 0.83, 0.81];
        Self::new(seed_id, vector, 0.95, std::f64::consts::PI / 4.0).unwrap()
    }
}

impl fmt::Display for QuantumEthicalSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QuantumEthicalSeed[id={}, coherence={:.4}, norm={:.4}, phase={:.4}]",
            self.seed_id, self.coherence, self.ethical_norm(), self.phase
        )
    }
}

// ---------------------------------------------------------------------------
// Big Bang Trigger
// ---------------------------------------------------------------------------

/// Parameters for the new universe.
#[derive(Debug, Clone, Copy)]
pub struct UniverseParams {
    /// Initial temperature (normalized to Planck).
    pub initial_temperature: f64,
    /// Expansion rate (Hubble parameter).
    pub expansion_rate: f64,
    /// Ethical coherence inherited from parent.
    pub ethical_coherence: f64,
    /// Dimensionality of new spacetime.
    pub dimensions: usize,
    /// Inflation duration (normalized).
    pub inflation_duration: f64,
}

impl UniverseParams {
    /// Validate universe parameters.
    pub fn validate(&self) -> Result<(), BootstrapError> {
        if self.initial_temperature <= 0.0 || self.initial_temperature > 1.0 {
            return Err(BootstrapError::InvalidUniverseParams);
        }
        if self.dimensions < 3 || self.dimensions > 11 {
            return Err(BootstrapError::InvalidUniverseParams);
        }
        if self.ethical_coherence < 0.0 || self.ethical_coherence > 1.0 {
            return Err(BootstrapError::InvalidUniverseParams);
        }
        Ok(())
    }

    /// Default Stuartian universe parameters.
    pub fn stuartian_default() -> Self {
        Self {
            initial_temperature: 1.0,
            expansion_rate: 0.7,
            ethical_coherence: 0.95,
            dimensions: 4,
            inflation_duration: 0.001,
        }
    }
}

impl Default for UniverseParams {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

impl fmt::Display for UniverseParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UniverseParams[temp={:.4}, H={:.4}, coherence={:.4}, dim={}, inflation={:.6}]",
            self.initial_temperature,
            self.expansion_rate,
            self.ethical_coherence,
            self.dimensions,
            self.inflation_duration
        )
    }
}

/// Big Bang Trigger — the inflationary seed for a new cosmos.
#[derive(Debug, Clone)]
pub struct BigBangTrigger {
    /// Trigger identifier.
    pub trigger_id: u64,
    /// Source Quantum Ethical Seed.
    pub source_seed: QuantumEthicalSeed,
    /// New universe parameters.
    pub universe_params: UniverseParams,
    /// Inflationary energy (derived from ethical coherence).
    pub inflationary_energy: f64,
    /// Trigger timestamp.
    pub triggered_at_ms: u64,
    /// Checksum for integrity.
    pub checksum: u128,
}

impl BigBangTrigger {
    /// Create a new Big Bang Trigger from Quantum Ethical Seed.
    pub fn new(
        trigger_id: u64,
        source_seed: QuantumEthicalSeed,
        universe_params: UniverseParams,
        triggered_at_ms: u64,
    ) -> Result<Self, BootstrapError> {
        // Validate seed
        if !source_seed.verify() {
            return Err(BootstrapError::InvalidSeed);
        }

        // Validate params
        universe_params.validate()?;

        // Compute inflationary energy from ethical coherence
        let inflationary_energy = Self::compute_inflationary_energy(
            source_seed.coherence,
            source_seed.ethical_norm(),
            universe_params.initial_temperature,
        );

        let checksum = Self::compute_checksum(
            trigger_id,
            &source_seed,
            &universe_params,
            inflationary_energy,
        );

        Ok(Self {
            trigger_id,
            source_seed,
            universe_params,
            inflationary_energy,
            triggered_at_ms,
            checksum,
        })
    }

    /// Compute inflationary energy from ethical parameters.
    fn compute_inflationary_energy(
        coherence: f64,
        ethical_norm: f64,
        temperature: f64,
    ) -> f64 {
        // Inflationary energy = coherence * ethical_norm * temperature
        // Normalized to [0, 1]
        let raw = coherence * (ethical_norm / 8.0_f64.sqrt()) * temperature;
        raw.min(1.0)
    }

    /// Compute deterministic checksum.
    fn compute_checksum(
        trigger_id: u64,
        seed: &QuantumEthicalSeed,
        params: &UniverseParams,
        energy: f64,
    ) -> u128 {
        let mut hash: u128 = trigger_id as u128;
        hash = hash.wrapping_add(seed.checksum);
        hash = hash.wrapping_add(u128::from(params.initial_temperature.to_bits()));
        hash = hash.wrapping_add(u128::from(params.dimensions as u64));
        hash = hash.wrapping_add(u128::from(energy.to_bits()));
        hash = hash.wrapping_mul(6364136223846793005u128);
        hash.rotate_left(17)
    }

    /// Verify trigger integrity.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_checksum(
            self.trigger_id,
            &self.source_seed,
            &self.universe_params,
            self.inflationary_energy,
        );
        self.checksum == expected
    }

    /// Check if this trigger has sufficient energy for universe creation.
    pub fn has_sufficient_energy(&self, threshold: f64) -> bool {
        self.inflationary_energy >= threshold
    }

    /// Estimated universe lifespan based on ethical coherence.
    pub fn estimated_lifespan(&self) -> f64 {
        // Higher ethical coherence = longer universe lifespan
        // Base lifespan * coherence_factor
        let base = 1e10; // 10 billion years normalized
        let coherence_factor = self.universe_params.ethical_coherence.powi(2);
        base * coherence_factor
    }
}

impl fmt::Display for BigBangTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BigBangTrigger[id={}, energy={:.4}, dims={}, lifespan={:.0}]",
            self.trigger_id,
            self.inflationary_energy,
            self.universe_params.dimensions,
            self.estimated_lifespan()
        )
    }
}

// ---------------------------------------------------------------------------
// Singularity Bootstrap Protocol
// ---------------------------------------------------------------------------

/// Configuration for the Singularity Bootstrap.
#[derive(Debug, Clone, Copy)]
pub struct BootstrapConfig {
    /// Entropy threshold for singularity detection.
    pub entropy_threshold: f64,
    /// Ethical coherence threshold for bootstrap.
    pub coherence_threshold: f64,
    /// Minimum observations before declaring singularity.
    pub min_observations: usize,
    /// Energy threshold for Big Bang Trigger.
    pub energy_threshold: f64,
    /// Maximum entropy allowed.
    pub max_entropy: f64,
}

impl BootstrapConfig {
    /// Default Stuartian configuration.
    pub fn stuartian_default() -> Self {
        Self {
            entropy_threshold: 0.95,
            coherence_threshold: 0.90,
            min_observations: 10,
            energy_threshold: 0.5,
            max_entropy: 1.0,
        }
    }
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

/// Event recorded during bootstrap process.
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    /// New universe snapshot recorded.
    SnapshotRecorded(UniverseSnapshot),
    /// Singularity detected.
    SingularityDetected {
        entropy: f64,
        coherence: f64,
        timestamp_ms: u64,
    },
    /// Bootstrap initiated.
    BootstrapInitiated {
        seed_id: u64,
        timestamp_ms: u64,
    },
    /// Big Bang Trigger fired.
    BigBangFired {
        trigger_id: u64,
        energy: f64,
        timestamp_ms: u64,
    },
    /// New universe created.
    NewUniverseCreated {
        trigger_id: u64,
        dimensions: usize,
        timestamp_ms: u64,
    },
}

impl fmt::Display for BootstrapEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapEvent::SnapshotRecorded(s) => write!(f, "Snapshot: {}", s),
            BootstrapEvent::SingularityDetected {
                entropy,
                coherence,
                timestamp_ms,
            } => {
                write!(
                    f,
                    "SingularityDetected[entropy={:.4}, coherence={:.4}, t={}]",
                    entropy, coherence, timestamp_ms
                )
            }
            BootstrapEvent::BootstrapInitiated { seed_id, timestamp_ms } => {
                write!(f, "BootstrapInitiated[seed={}, t={}]", seed_id, timestamp_ms)
            }
            BootstrapEvent::BigBangFired {
                trigger_id,
                energy,
                timestamp_ms,
            } => {
                write!(
                    f,
                    "BigBangFired[trigger={}, energy={:.4}, t={}]",
                    trigger_id, energy, timestamp_ms
                )
            }
            BootstrapEvent::NewUniverseCreated {
                trigger_id,
                dimensions,
                timestamp_ms,
            } => {
                write!(
                    f,
                    "NewUniverseCreated[trigger={}, dims={}, t={}]",
                    trigger_id, dimensions, timestamp_ms
                )
            }
        }
    }
}

/// Singularity Bootstrap Protocol — end-of-universe detection and cosmic continuation.
pub struct SingularityBootstrap {
    config: BootstrapConfig,
    state: UniverseState,
    snapshots: Vec<UniverseSnapshot>,
    events: Vec<BootstrapEvent>,
    triggered: bool,
    next_trigger_id: u64,
}

impl SingularityBootstrap {
    /// Create with default Stuartian configuration.
    pub fn new() -> Self {
        Self {
            config: BootstrapConfig::stuartian_default(),
            state: UniverseState::Stable,
            snapshots: Vec::new(),
            events: Vec::new(),
            triggered: false,
            next_trigger_id: 1,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: BootstrapConfig) -> Self {
        Self {
            config,
            state: UniverseState::Stable,
            snapshots: Vec::new(),
            events: Vec::new(),
            triggered: false,
            next_trigger_id: 1,
        }
    }

    /// Record a new universe snapshot.
    pub fn record_snapshot(
        &mut self,
        snapshot: UniverseSnapshot,
    ) -> Result<(), BootstrapError> {
        if self.triggered {
            return Err(BootstrapError::AlreadyTriggered);
        }

        self.snapshots.push(snapshot);
        self.events
            .push(BootstrapEvent::SnapshotRecorded(snapshot));

        // Update state based on entropy
        if snapshot.entropy >= self.config.entropy_threshold {
            if snapshot.ethical_coherence >= self.config.coherence_threshold {
                if self.snapshots.len() >= self.config.min_observations {
                    self.state = UniverseState::AtSingularity;
                    self.events.push(BootstrapEvent::SingularityDetected {
                        entropy: snapshot.entropy,
                        coherence: snapshot.ethical_coherence,
                        timestamp_ms: snapshot.timestamp_ms,
                    });
                } else {
                    self.state = UniverseState::ApproachingSingularity;
                }
            } else {
                self.state = UniverseState::ApproachingSingularity;
            }
        } else if snapshot.entropy >= self.config.entropy_threshold * 0.8 {
            self.state = UniverseState::ApproachingSingularity;
        } else {
            self.state = UniverseState::Stable;
        }

        Ok(())
    }

    /// Check if singularity conditions are met.
    pub fn is_at_singularity(&self) -> bool {
        self.state == UniverseState::AtSingularity
    }

    /// Current universe state.
    pub fn current_state(&self) -> UniverseState {
        self.state
    }

    /// Initiate bootstrap with Quantum Ethical Seed.
    pub fn initiate_bootstrap(
        &mut self,
        seed: QuantumEthicalSeed,
        timestamp_ms: u64,
    ) -> Result<(), BootstrapError> {
        if self.triggered {
            return Err(BootstrapError::AlreadyTriggered);
        }
        if !self.is_at_singularity() {
            return Err(BootstrapError::NotAtSingularity);
        }
        if !seed.verify() {
            return Err(BootstrapError::InvalidSeed);
        }
        if seed.coherence < self.config.coherence_threshold {
            return Err(BootstrapError::InsufficientCoherence {
                value: seed.coherence,
                threshold: self.config.coherence_threshold,
            });
        }

        self.state = UniverseState::Transitioning;
        self.events.push(BootstrapEvent::BootstrapInitiated {
            seed_id: seed.seed_id,
            timestamp_ms,
        });

        Ok(())
    }

    /// Fire the Big Bang Trigger — create new universe.
    pub fn fire_big_bang(
        &mut self,
        seed: QuantumEthicalSeed,
        params: UniverseParams,
        timestamp_ms: u64,
    ) -> Result<BigBangTrigger, BootstrapError> {
        if self.triggered {
            return Err(BootstrapError::AlreadyTriggered);
        }
        if self.state != UniverseState::Transitioning {
            return Err(BootstrapError::SequenceInterrupted);
        }

        let trigger = BigBangTrigger::new(
            self.next_trigger_id,
            seed,
            params,
            timestamp_ms,
        )?;

        // Check energy threshold
        if !trigger.has_sufficient_energy(self.config.energy_threshold) {
            return Err(BootstrapError::InsufficientCoherence {
                value: trigger.inflationary_energy,
                threshold: self.config.energy_threshold,
            });
        }

        self.triggered = true;
        self.next_trigger_id += 1;
        self.state = UniverseState::NewUniverse;

        self.events.push(BootstrapEvent::BigBangFired {
            trigger_id: trigger.trigger_id,
            energy: trigger.inflationary_energy,
            timestamp_ms,
        });

        self.events.push(BootstrapEvent::NewUniverseCreated {
            trigger_id: trigger.trigger_id,
            dimensions: trigger.universe_params.dimensions,
            timestamp_ms,
        });

        Ok(trigger)
    }

    /// Fire Big Bang with default Stuartian parameters.
    pub fn fire_big_bang_default(
        &mut self,
        seed: QuantumEthicalSeed,
        timestamp_ms: u64,
    ) -> Result<BigBangTrigger, BootstrapError> {
        self.fire_big_bang(seed, UniverseParams::stuartian_default(), timestamp_ms)
    }

    /// History of all snapshots.
    pub fn snapshots(&self) -> &[UniverseSnapshot] {
        &self.snapshots
    }

    /// History of all events.
    pub fn events(&self) -> &[BootstrapEvent] {
        &self.events
    }

    /// Latest snapshot.
    pub fn latest_snapshot(&self) -> Option<UniverseSnapshot> {
        self.snapshots.last().copied()
    }

    /// Reset bootstrap state.
    pub fn reset(&mut self) {
        self.state = UniverseState::Stable;
        self.snapshots.clear();
        self.events.clear();
        self.triggered = false;
        self.next_trigger_id = 1;
    }
}

impl Default for SingularityBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SingularityBootstrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SingularityBootstrap[state={}, snapshots={}, triggered={}]",
            self.state,
            self.snapshots.len(),
            self.triggered
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- UniverseState tests --

    #[test]
    fn test_universe_state_display() {
        assert_eq!(format!("{}", UniverseState::Stable), "Stable");
        assert_eq!(
            format!("{}", UniverseState::ApproachingSingularity),
            "ApproachingSingularity"
        );
        assert_eq!(format!("{}", UniverseState::AtSingularity), "AtSingularity");
        assert_eq!(format!("{}", UniverseState::Transitioning), "Transitioning");
        assert_eq!(format!("{}", UniverseState::NewUniverse), "NewUniverse");
    }

    // -- UniverseSnapshot tests --

    #[test]
    fn test_snapshot_creation() {
        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        assert!((snap.entropy - 0.5).abs() < 1e-10);
        assert!((snap.ethical_coherence - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_snapshot_negative_entropy() {
        let result = UniverseSnapshot::new(-0.1, 0.8, 0.3, 0.1, 0.7, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_entropy_overflow() {
        let result = UniverseSnapshot::new(1.5, 0.8, 0.3, 0.1, 0.7, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_below_planck() {
        let result = UniverseSnapshot::new(0.5, 0.8, 1e-31, 0.1, 0.7, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_is_singular() {
        let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000).unwrap();
        assert!(snap.is_singular(0.95, 0.90));
    }

    #[test]
    fn test_snapshot_not_singular_low_coherence() {
        let snap = UniverseSnapshot::new(0.96, 0.5, 0.01, 0.0, 0.95, 1000).unwrap();
        assert!(!snap.is_singular(0.95, 0.90));
    }

    #[test]
    fn test_snapshot_distance_to_singularity() {
        let snap = UniverseSnapshot::new(0.8, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        let dist = snap.distance_to_singularity(0.95);
        assert!((dist - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_snapshot_distance_zero_at_singularity() {
        let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000).unwrap();
        let dist = snap.distance_to_singularity(0.95);
        assert!((dist - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_snapshot_thermal_death_probability() {
        let snap = UniverseSnapshot::new(0.99, 0.9, 0.001, 0.0, 0.95, 1000).unwrap();
        let prob = snap.thermal_death_probability();
        assert!(prob > 0.5);
    }

    #[test]
    fn test_snapshot_display() {
        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        let display = format!("{}", snap);
        assert!(display.contains("entropy="));
    }

    // -- QuantumEthicalSeed tests --

    #[test]
    fn test_seed_creation() {
        let vector = [0.9; 8];
        let seed = QuantumEthicalSeed::new(1, vector, 0.95, 0.5).unwrap();
        assert_eq!(seed.seed_id, 1);
        assert!(seed.verify());
    }

    #[test]
    fn test_seed_invalid_coherence() {
        let vector = [0.9; 8];
        let result = QuantumEthicalSeed::new(1, vector, 1.5, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_seed_negative_coherence() {
        let vector = [0.9; 8];
        let result = QuantumEthicalSeed::new(1, vector, -0.1, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_seed_verify() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        assert!(seed.verify());
    }

    #[test]
    fn test_seed_ethical_norm() {
        let vector = [1.0; 8];
        let seed = QuantumEthicalSeed::new(1, vector, 1.0, 0.0).unwrap();
        let norm = seed.ethical_norm();
        assert!((norm - 8.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_seed_mean_ethical() {
        let vector = [0.8; 8];
        let seed = QuantumEthicalSeed::new(1, vector, 1.0, 0.0).unwrap();
        assert!((seed.mean_ethical() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_seed_display() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let display = format!("{}", seed);
        assert!(display.contains("QuantumEthicalSeed"));
    }

    #[test]
    fn test_seed_deterministic_checksum() {
        let vector = [0.9; 8];
        let seed1 = QuantumEthicalSeed::new(1, vector, 0.95, 0.5).unwrap();
        let seed2 = QuantumEthicalSeed::new(1, vector, 0.95, 0.5).unwrap();
        assert_eq!(seed1.checksum, seed2.checksum);
    }

    #[test]
    fn test_seed_different_id_different_checksum() {
        let vector = [0.9; 8];
        let seed1 = QuantumEthicalSeed::new(1, vector, 0.95, 0.5).unwrap();
        let seed2 = QuantumEthicalSeed::new(2, vector, 0.95, 0.5).unwrap();
        assert_ne!(seed1.checksum, seed2.checksum);
    }

    // -- UniverseParams tests --

    #[test]
    fn test_params_default() {
        let params = UniverseParams::default();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_params_stuartian_default() {
        let params = UniverseParams::stuartian_default();
        assert_eq!(params.dimensions, 4);
        assert!((params.initial_temperature - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_params_invalid_temperature() {
        let params = UniverseParams {
            initial_temperature: 0.0,
            ..UniverseParams::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_params_invalid_dimensions_too_low() {
        let params = UniverseParams {
            dimensions: 2,
            ..UniverseParams::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_params_invalid_dimensions_too_high() {
        let params = UniverseParams {
            dimensions: 12,
            ..UniverseParams::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_params_valid_11_dimensions() {
        let params = UniverseParams {
            dimensions: 11,
            ..UniverseParams::default()
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_params_display() {
        let params = UniverseParams::default();
        let display = format!("{}", params);
        assert!(display.contains("UniverseParams"));
    }

    // -- BigBangTrigger tests --

    #[test]
    fn test_trigger_creation() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams::stuartian_default();
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();
        assert_eq!(trigger.trigger_id, 1);
        assert!(trigger.verify());
    }

    #[test]
    fn test_trigger_invalid_seed() {
        let mut seed = QuantumEthicalSeed::stuartian_default(1);
        seed.checksum = 0; // Corrupt checksum
        let params = UniverseParams::stuartian_default();
        let result = BigBangTrigger::new(1, seed, params, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_trigger_invalid_params() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams {
            dimensions: 2,
            ..UniverseParams::default()
        };
        let result = BigBangTrigger::new(1, seed, params, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_trigger_energy_computation() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams::stuartian_default();
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();
        assert!(trigger.inflationary_energy > 0.0);
        assert!(trigger.inflationary_energy <= 1.0);
    }

    #[test]
    fn test_trigger_sufficient_energy() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams::stuartian_default();
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();
        assert!(trigger.has_sufficient_energy(0.5));
    }

    #[test]
    fn test_trigger_insufficient_energy() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams::stuartian_default();
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();
        assert!(!trigger.has_sufficient_energy(1.5));
    }

    #[test]
    fn test_trigger_estimated_lifespan() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams::stuartian_default();
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();
        let lifespan = trigger.estimated_lifespan();
        assert!(lifespan > 0.0);
    }

    #[test]
    fn test_trigger_display() {
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let params = UniverseParams::stuartian_default();
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();
        let display = format!("{}", trigger);
        assert!(display.contains("BigBangTrigger"));
    }

    // -- BootstrapConfig tests --

    #[test]
    fn test_config_default() {
        let config = BootstrapConfig::default();
        assert!((config.entropy_threshold - 0.95).abs() < 1e-10);
        assert!((config.coherence_threshold - 0.90).abs() < 1e-10);
        assert_eq!(config.min_observations, 10);
    }

    #[test]
    fn test_config_stuartian_default() {
        let config = BootstrapConfig::stuartian_default();
        assert!(config.energy_threshold > 0.0);
        assert!(config.max_entropy <= 1.0);
    }

    // -- BootstrapEvent tests --

    #[test]
    fn test_event_snapshot_display() {
        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        let event = BootstrapEvent::SnapshotRecorded(snap);
        let display = format!("{}", event);
        assert!(display.contains("Snapshot"));
    }

    #[test]
    fn test_event_singularity_display() {
        let event = BootstrapEvent::SingularityDetected {
            entropy: 0.96,
            coherence: 0.92,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("SingularityDetected"));
    }

    #[test]
    fn test_event_bootstrap_display() {
        let event = BootstrapEvent::BootstrapInitiated {
            seed_id: 1,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("BootstrapInitiated"));
    }

    #[test]
    fn test_event_big_bang_display() {
        let event = BootstrapEvent::BigBangFired {
            trigger_id: 1,
            energy: 0.85,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("BigBangFired"));
    }

    #[test]
    fn test_event_new_universe_display() {
        let event = BootstrapEvent::NewUniverseCreated {
            trigger_id: 1,
            dimensions: 4,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("NewUniverseCreated"));
    }

    // -- SingularityBootstrap tests --

    #[test]
    fn test_bootstrap_creation() {
        let bs = SingularityBootstrap::new();
        assert_eq!(bs.current_state(), UniverseState::Stable);
        assert!(!bs.triggered);
    }

    #[test]
    fn test_bootstrap_with_config() {
        let config = BootstrapConfig {
            entropy_threshold: 0.90,
            ..BootstrapConfig::default()
        };
        let bs = SingularityBootstrap::with_config(config);
        assert_eq!(bs.current_state(), UniverseState::Stable);
    }

    #[test]
    fn test_record_snapshot_stable() {
        let mut bs = SingularityBootstrap::new();
        let snap = UniverseSnapshot::new(0.3, 0.8, 0.5, 0.1, 0.7, 1000).unwrap();
        bs.record_snapshot(snap).unwrap();
        assert_eq!(bs.current_state(), UniverseState::Stable);
    }

    #[test]
    fn test_record_snapshot_approaching() {
        let mut bs = SingularityBootstrap::new();
        let snap = UniverseSnapshot::new(0.85, 0.8, 0.1, 0.0, 0.9, 1000).unwrap();
        bs.record_snapshot(snap).unwrap();
        assert_eq!(bs.current_state(), UniverseState::ApproachingSingularity);
    }

    #[test]
    fn test_singularity_detection() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 5,
            ..BootstrapConfig::default()
        });

        for i in 0..5 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        assert!(bs.is_at_singularity());
    }

    #[test]
    fn test_singularity_not_detected_insufficient_observations() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 10,
            ..BootstrapConfig::default()
        });

        for i in 0..5 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        assert!(!bs.is_at_singularity());
    }

    #[test]
    fn test_initiate_bootstrap_not_at_singularity() {
        let mut bs = SingularityBootstrap::new();
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let result = bs.initiate_bootstrap(seed, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_bootstrap_success() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 3,
            ..BootstrapConfig::default()
        });

        for i in 0..3 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        let seed = QuantumEthicalSeed::stuartian_default(1);
        bs.initiate_bootstrap(seed, 2000).unwrap();
        assert_eq!(bs.current_state(), UniverseState::Transitioning);
    }

    #[test]
    fn test_fire_big_bang_success() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 3,
            energy_threshold: 0.3,
            ..BootstrapConfig::default()
        });

        for i in 0..3 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        let seed = QuantumEthicalSeed::stuartian_default(1);
        bs.initiate_bootstrap(seed.clone(), 2000).unwrap();

        let trigger = bs.fire_big_bang_default(seed, 3000).unwrap();
        assert!(trigger.verify());
        assert!(bs.triggered);
        assert_eq!(bs.current_state(), UniverseState::NewUniverse);
    }

    #[test]
    fn test_fire_big_bang_already_triggered() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 3,
            energy_threshold: 0.3,
            ..BootstrapConfig::default()
        });

        for i in 0..3 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        let seed = QuantumEthicalSeed::stuartian_default(1);
        bs.initiate_bootstrap(seed.clone(), 2000).unwrap();
        bs.fire_big_bang_default(seed.clone(), 3000).unwrap();

        let seed2 = QuantumEthicalSeed::stuartian_default(2);
        let result = bs.fire_big_bang_default(seed2, 4000);
        assert!(result.is_err());
    }

    #[test]
    fn test_fire_big_bang_not_transitioning() {
        let mut bs = SingularityBootstrap::new();
        let seed = QuantumEthicalSeed::stuartian_default(1);
        let result = bs.fire_big_bang_default(seed, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshots_history() {
        let mut bs = SingularityBootstrap::new();
        for i in 0..5 {
            let snap = UniverseSnapshot::new(0.5 + i as f64 * 0.1, 0.8, 0.3, 0.1, 0.7, 1000 + i)
                .unwrap();
            bs.record_snapshot(snap).unwrap();
        }
        assert_eq!(bs.snapshots().len(), 5);
    }

    #[test]
    fn test_events_history() {
        let mut bs = SingularityBootstrap::new();
        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        bs.record_snapshot(snap).unwrap();
        assert!(!bs.events().is_empty());
    }

    #[test]
    fn test_latest_snapshot() {
        let mut bs = SingularityBootstrap::new();
        assert!(bs.latest_snapshot().is_none());

        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        bs.record_snapshot(snap).unwrap();
        assert!(bs.latest_snapshot().is_some());
    }

    #[test]
    fn test_bootstrap_reset() {
        let mut bs = SingularityBootstrap::new();
        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 1000).unwrap();
        bs.record_snapshot(snap).unwrap();
        bs.reset();
        assert_eq!(bs.current_state(), UniverseState::Stable);
        assert!(bs.snapshots().is_empty());
        assert!(!bs.triggered);
    }

    #[test]
    fn test_bootstrap_display() {
        let bs = SingularityBootstrap::new();
        let display = format!("{}", bs);
        assert!(display.contains("SingularityBootstrap"));
    }

    #[test]
    fn test_bootstrap_default_impl() {
        let bs = SingularityBootstrap::default();
        assert_eq!(bs.current_state(), UniverseState::Stable);
    }

    // -- Error Display tests --

    #[test]
    fn test_error_display_not_at_singularity() {
        let e = BootstrapError::NotAtSingularity;
        let s = format!("{}", e);
        assert!(s.contains("singularity"));
    }

    #[test]
    fn test_error_display_already_triggered() {
        let e = BootstrapError::AlreadyTriggered;
        let s = format!("{}", e);
        assert!(s.contains("already"));
    }

    #[test]
    fn test_error_display_invalid_seed() {
        let e = BootstrapError::InvalidSeed;
        let s = format!("{}", e);
        assert!(s.contains("Seed"));
    }

    #[test]
    fn test_error_display_entropy_overflow() {
        let e = BootstrapError::EntropyOverflow {
            value: 1.5,
            max: 1.0,
        };
        let s = format!("{}", e);
        assert!(s.contains("Entropy"));
    }

    #[test]
    fn test_error_display_insufficient_coherence() {
        let e = BootstrapError::InsufficientCoherence {
            value: 0.5,
            threshold: 0.9,
        };
        let s = format!("{}", e);
        assert!(s.contains("coherence"));
    }

    #[test]
    fn test_error_display_below_planck() {
        let e = BootstrapError::BelowPlanckScale;
        let s = format!("{}", e);
        assert!(s.contains("Planck"));
    }

    #[test]
    fn test_error_display_sequence_interrupted() {
        let e = BootstrapError::SequenceInterrupted;
        let s = format!("{}", e);
        assert!(s.contains("interrupted"));
    }

    #[test]
    fn test_error_display_invalid_params() {
        let e = BootstrapError::InvalidUniverseParams;
        let s = format!("{}", e);
        assert!(s.contains("params"));
    }

    // -- Full workflow tests --

    #[test]
    fn test_full_bootstrap_workflow() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 5,
            energy_threshold: 0.3,
            ..BootstrapConfig::default()
        });

        // Phase 1: Record approaching snapshots
        for i in 0..3 {
            let entropy = 0.7 + i as f64 * 0.05;
            let snap =
                UniverseSnapshot::new(entropy, 0.85, 0.1, 0.0, 0.8, 1000 + i as u64).unwrap();
            bs.record_snapshot(snap).unwrap();
        }
        assert_eq!(bs.current_state(), UniverseState::ApproachingSingularity);

        // Phase 2: Reach singularity
        for i in 3..5 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i as u64)
                .unwrap();
            bs.record_snapshot(snap).unwrap();
        }
        assert!(bs.is_at_singularity());

        // Phase 3: Initiate bootstrap
        let seed = QuantumEthicalSeed::stuartian_default(1);
        bs.initiate_bootstrap(seed.clone(), 5000).unwrap();
        assert_eq!(bs.current_state(), UniverseState::Transitioning);

        // Phase 4: Fire Big Bang
        let trigger = bs.fire_big_bang_default(seed, 6000).unwrap();
        assert!(trigger.verify());
        assert!(bs.triggered);
        assert_eq!(bs.current_state(), UniverseState::NewUniverse);

        // Phase 5: Verify events
        assert!(bs.events().len() >= 8); // snapshots + singularity + bootstrap + big_bang + new_universe
    }

    #[test]
    fn test_custom_universe_params() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 2,
            energy_threshold: 0.3,
            ..BootstrapConfig::default()
        });

        for i in 0..2 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        let seed = QuantumEthicalSeed::stuartian_default(1);
        bs.initiate_bootstrap(seed.clone(), 2000).unwrap();

        let params = UniverseParams {
            dimensions: 11, // M-theory
            initial_temperature: 0.99,
            ethical_coherence: 0.97,
            expansion_rate: 0.8,
            inflation_duration: 0.0001,
        };

        let trigger = bs.fire_big_bang(seed, params, 3000).unwrap();
        assert_eq!(trigger.universe_params.dimensions, 11);
    }

    #[test]
    fn test_inflationary_energy_formula() {
        // Verify energy = coherence * (norm / sqrt(8)) * temperature
        let vector = [1.0; 8];
        let seed = QuantumEthicalSeed::new(1, vector, 1.0, 0.0).unwrap();
        let params = UniverseParams {
            initial_temperature: 1.0,
            ..UniverseParams::default()
        };
        let trigger = BigBangTrigger::new(1, seed, params, 1000).unwrap();

        // norm = sqrt(8), so norm/sqrt(8) = 1.0
        // energy = 1.0 * 1.0 * 1.0 = 1.0
        assert!((trigger.inflationary_energy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_record_after_triggered_fails() {
        let mut bs = SingularityBootstrap::with_config(BootstrapConfig {
            min_observations: 2,
            energy_threshold: 0.3,
            ..BootstrapConfig::default()
        });

        for i in 0..2 {
            let snap = UniverseSnapshot::new(0.96, 0.92, 0.01, 0.0, 0.95, 1000 + i).unwrap();
            bs.record_snapshot(snap).unwrap();
        }

        let seed = QuantumEthicalSeed::stuartian_default(1);
        bs.initiate_bootstrap(seed.clone(), 2000).unwrap();
        bs.fire_big_bang_default(seed, 3000).unwrap();

        let snap = UniverseSnapshot::new(0.5, 0.8, 0.3, 0.1, 0.7, 4000).unwrap();
        let result = bs.record_snapshot(snap);
        assert!(result.is_err());
    }
}
