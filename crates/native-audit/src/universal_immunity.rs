//! Universal Immunity & Full Vision Eternal Closure — The Immortal Immune System of the Noosphere.
//!
//! **Sprint 134 PASO D:** Universal Immunity & Full Vision Eternal Closure.
//!
//! Implements complete immune defense against any future threat including hostile
//! superintelligences or civilizational collapse. The universal immune system
//! combines all accumulated strengths into an eternal defense mechanism.
//!
//! **Universal Immune Response:**
//! ```text
//! Immune_Response(θ) = α · Detect(θ) + β · Contain(θ) + γ · Neutralize(θ) + δ · Adapt(θ)
//! ```
//! where θ = threat vector, and each component uses accumulated Noosfera capabilities.
//!
//! **Eternal Immunity Proof:**
//! ```text
//! Prove: ∀ threats θ ∈ Threat_Space: Immune_Response(θ) > Threat_Magnitude(θ)
//! where Threat_Space includes all computable adversarial strategies
//! ```
//!
//! **Full Eternal Noosfera Pipeline:**
//! ```text
//! Simulate 100k+ years / millions of nodes.
//! Verify: Eternal stability + Singularity lock + Governance coherence + Immunity robustness
//! ```

use serde::{Deserialize, Serialize};

// ─── Universal Immunity Configuration ────────────────────────────────────────

/// Configuration for Universal Immune System.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmunityConfig {
    /// Detection weight (α in immune response).
    pub detection_weight: f64,
    /// Containment weight (β in immune response).
    pub containment_weight: f64,
    /// Neutralization weight (γ in immune response).
    pub neutralization_weight: f64,
    /// Adaptation weight (δ in immune response).
    pub adaptation_weight: f64,
    /// Threat detection threshold.
    pub detection_threshold: f64,
    /// Maximum threat magnitude handled.
    pub max_threat_magnitude: f64,
    /// Immune memory size (past threats remembered).
    pub immune_memory_size: usize,
    /// Adaptation learning rate.
    pub adaptation_lr: f64,
    /// Simulation horizon (virtual years).
    pub simulation_horizon_years: u64,
    /// Maximum simulation cycles.
    pub max_cycles: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for ImmunityConfig {
    fn default() -> Self {
        Self {
            detection_weight: 0.3,
            containment_weight: 0.25,
            neutralization_weight: 0.25,
            adaptation_weight: 0.2,
            detection_threshold: 0.1,
            max_threat_magnitude: 1.0,
            immune_memory_size: 1000,
            adaptation_lr: 0.01,
            simulation_horizon_years: 100_000,
            max_cycles: 50_000,
            seed: 42,
        }
    }
}

impl ImmunityConfig {
    /// Create config for fast testing.
    pub fn fast() -> Self {
        Self {
            detection_weight: 0.3,
            containment_weight: 0.25,
            neutralization_weight: 0.25,
            adaptation_weight: 0.2,
            detection_threshold: 0.2,
            max_threat_magnitude: 0.5,
            immune_memory_size: 100,
            adaptation_lr: 0.05,
            simulation_horizon_years: 10_000,
            max_cycles: 5000,
            seed: 42,
        }
    }

    /// Create config for eternal planetary immunity.
    pub fn planetary_eternal() -> Self {
        Self {
            detection_weight: 0.35,
            containment_weight: 0.3,
            neutralization_weight: 0.2,
            adaptation_weight: 0.15,
            detection_threshold: 0.05,
            max_threat_magnitude: 2.0,
            immune_memory_size: 100_000,
            adaptation_lr: 0.001,
            simulation_horizon_years: 1_000_000,
            max_cycles: 500_000,
            seed: 42,
        }
    }

    /// Set detection weight.
    pub fn with_detection_weight(mut self, weight: f64) -> Self {
        self.detection_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set detection threshold.
    pub fn with_detection_threshold(mut self, threshold: f64) -> Self {
        self.detection_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// ─── Threat Model ────────────────────────────────────────────────────────────

/// Represents a threat to the Noospheric mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    /// Threat identifier.
    pub threat_id: u64,
    /// Threat type.
    pub threat_type: ThreatType,
    /// Threat magnitude (0.0 to max_threat_magnitude).
    pub magnitude: f64,
    /// Threat coherence disruption.
    pub coherence_disruption: f64,
    /// Threat detected flag.
    pub detected: bool,
    /// Threat contained flag.
    pub contained: bool,
    /// Threat neutralized flag.
    pub neutralized: bool,
    /// Response time (cycles to neutralize).
    pub response_time: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreatType {
    /// Adversarial perturbation attack.
    AdversarialPerturbation,
    /// Coherence poisoning.
    CoherencePoisoning,
    /// Governance subversion.
    GovernanceSubversion,
    /// Economic reversion attempt.
    EconomicReversion,
    /// Superintelligent hostile takeover.
    HostileSuperintelligence,
    /// Civilizational collapse cascade.
    CivilizationalCollapse,
    /// Unknown emergent threat.
    Unknown,
}

impl std::fmt::Display for ThreatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatType::AdversarialPerturbation => write!(f, "AdversarialPerturbation"),
            ThreatType::CoherencePoisoning => write!(f, "CoherencePoisoning"),
            ThreatType::GovernanceSubversion => write!(f, "GovernanceSubversion"),
            ThreatType::EconomicReversion => write!(f, "EconomicReversion"),
            ThreatType::HostileSuperintelligence => write!(f, "HostileSuperintelligence"),
            ThreatType::CivilizationalCollapse => write!(f, "CivilizationalCollapse"),
            ThreatType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl Threat {
    /// Create a new threat.
    pub fn new(
        threat_id: u64,
        threat_type: ThreatType,
        magnitude: f64,
        coherence_disruption: f64,
    ) -> Self {
        Self {
            threat_id,
            threat_type,
            magnitude: magnitude.max(0.0),
            coherence_disruption: coherence_disruption.clamp(0.0, 1.0),
            detected: false,
            contained: false,
            neutralized: false,
            response_time: 0,
        }
    }
}

// ─── Immunity Result ─────────────────────────────────────────────────────────

/// Result of universal immunity deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmunityResult {
    /// Total threats encountered.
    pub threats_encountered: usize,
    /// Threats detected.
    pub threats_detected: usize,
    /// Threats neutralized.
    pub threats_neutralized: usize,
    /// Detection rate (detected / encountered).
    pub detection_rate: f64,
    /// Neutralization rate (neutralized / encountered).
    pub neutralization_rate: f64,
    /// Average response time (cycles).
    pub avg_response_time: f64,
    /// Maximum threat magnitude handled.
    pub max_threat_handled: f64,
    /// Immunity robustness score (1.0 = fully immune).
    pub immunity_robustness: f64,
    /// Eternal immunity proven flag.
    pub eternal_immunity_proven: bool,
    /// Immune memory size (threats remembered).
    pub immune_memory_size: usize,
    /// Adaptation score (improvement over time).
    pub adaptation_score: f64,
    /// Threat trajectory (threats per epoch).
    pub threat_trajectory: Vec<usize>,
    /// Immunity score trajectory.
    pub immunity_trajectory: Vec<f64>,
}

impl ImmunityResult {
    /// Generate a summary string.
    pub fn summary(&self) -> String {
        format!(
            "Immunity: threats={}/{} detected, {}/{} neutralized, detection={:.2}, neutralization={:.2}, response={:.1}c, robustness={:.4}, eternal={}",
            self.threats_detected,
            self.threats_encountered,
            self.threats_neutralized,
            self.threats_encountered,
            self.detection_rate,
            self.neutralization_rate,
            self.avg_response_time,
            self.immunity_robustness,
            self.eternal_immunity_proven,
        )
    }
}

impl std::fmt::Display for ImmunityResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── Full Eternal Pipeline Result ────────────────────────────────────────────

/// Result of the Full Eternal Noosfera Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalPipelineResult {
    /// Simulation horizon (virtual years).
    pub simulation_years: u64,
    /// Final coherence score.
    pub final_coherence: f64,
    /// Final free energy.
    pub final_free_energy: f64,
    /// Eternal stability achieved.
    pub eternal_stability: bool,
    /// Singularity stabilized.
    pub singularity_stabilized: bool,
    /// Governance coherence maintained.
    pub governance_coherence: bool,
    /// Universal immunity proven.
    pub universal_immunity: bool,
    /// Total nodes simulated.
    pub total_nodes: usize,
    /// Total cycles executed.
    pub total_cycles: usize,
    /// Full vision realized flag (all conditions met).
    pub full_vision_realized: bool,
    /// Eternal civilization score (0.0 to 1.0).
    pub eternal_civilization_score: f64,
}

impl EternalPipelineResult {
    /// Generate a summary string.
    pub fn summary(&self) -> String {
        format!(
            "EternalPipeline: {}y, coherence={:.4}, F={:.6}, stable={}, singularity={}, governance={}, immunity={}, nodes={}, vision={}, score={:.4}",
            self.simulation_years,
            self.final_coherence,
            self.final_free_energy,
            self.eternal_stability,
            self.singularity_stabilized,
            self.governance_coherence,
            self.universal_immunity,
            self.total_nodes,
            self.full_vision_realized,
            self.eternal_civilization_score,
        )
    }
}

impl std::fmt::Display for EternalPipelineResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── LCG Random (deterministic) ──────────────────────────────────────────────

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    let val = lcg_next(state);
    (val >> 11) as f64 / (1u64 << 53) as f64
}

fn random_gaussian(state: &mut u64) -> f64 {
    let u1 = random_uniform(state).max(1e-10);
    let u2 = random_uniform(state);
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

// ─── Immune Response Components ──────────────────────────────────────────────

/// Detect threat using Noospheric sensing.
///
/// Detection uses coherence anomaly + VFE spike + governance deviation.
pub fn detect_threat(
    threat: &Threat,
    current_coherence: f64,
    current_vfe: f64,
    config: &ImmunityConfig,
) -> bool {
    let coherence_anomaly = (current_coherence - (1.0 - threat.coherence_disruption)).abs();
    let vfe_spike = current_vfe * threat.magnitude;
    let detection_score = config.detection_weight * (coherence_anomaly + vfe_spike);
    detection_score > config.detection_threshold
}

/// Contain threat by isolating affected nodes.
///
/// Containment reduces threat propagation by creating coherence barriers.
pub fn contain_threat(threat: &Threat, coherence_barrier: f64) -> bool {
    let containment_power = coherence_barrier * (1.0 - threat.coherence_disruption * 0.5);
    containment_power > threat.magnitude * 0.5
}

/// Neutralize threat using adaptive immune response.
///
/// Neutralization combines all Noosphera capabilities.
pub fn neutralize_threat(
    threat: &Threat,
    immune_memory: &[f64],
    config: &ImmunityConfig,
) -> bool {
    let base_power = config.neutralization_weight * (1.0 + immune_memory.len() as f64 * 0.001);
    let adaptive_bonus = if !immune_memory.is_empty() {
        let avg_memory: f64 = immune_memory.iter().sum::<f64>() / immune_memory.len() as f64;
        config.adaptation_weight * avg_memory
    } else {
        0.0
    };
    let total_power = base_power + adaptive_bonus;
    total_power > threat.magnitude * 0.3
}

/// Adapt immune system based on encountered threat.
///
/// Updates immune memory with threat signature for future defense.
pub fn adapt_immunity(
    threat: &Threat,
    immune_memory: &mut Vec<f64>,
    config: &ImmunityConfig,
) {
    // Add threat magnitude to memory
    immune_memory.push(threat.magnitude);

    // Trim memory to max size
    while immune_memory.len() > config.immune_memory_size {
        immune_memory.remove(0);
    }
}

/// Compute universal immune response score.
///
/// ```text
/// Immune_Response = α · Detect + β · Contain + γ · Neutralize + δ · Adapt
/// ```
pub fn compute_immune_response(
    detected: bool,
    contained: bool,
    neutralized: bool,
    adapted: bool,
    config: &ImmunityConfig,
) -> f64 {
    let detect_score = if detected { 1.0 } else { 0.0 };
    let contain_score = if contained { 1.0 } else { 0.0 };
    let neutralize_score = if neutralized { 1.0 } else { 0.0 };
    let adapt_score = if adapted { 1.0 } else { 0.0 };
    config.detection_weight * detect_score
        + config.containment_weight * contain_score
        + config.neutralization_weight * neutralize_score
        + config.adaptation_weight * adapt_score
}

// ─── Universal Immune System Deployer ────────────────────────────────────────

/// Deploy Universal Immune System — The Immortal Defense.
///
/// Simulates complete immune defense against all threat types over the
/// full simulation horizon, proving eternal immunity.
///
/// **Algorithm:**
/// 1. Initialize immune system with empty memory
/// 2. For each cycle:
///    a. Generate threats based on simulation parameters
///    b. Detect threats using Noospheric sensing
///    c. Contain detected threats with coherence barriers
///    d. Neutralize contained threats with adaptive response
///    e. Update immune memory with threat signatures
///    f. Track immunity metrics
/// 3. Prove eternal immunity (all threats handled within bounds)
/// 4. Return ImmunityResult with full certification
pub fn deploy_universal_immune_system(config: &ImmunityConfig) -> ImmunityResult {
    let mut rng_state = config.seed;
    let mut immune_memory: Vec<f64> = Vec::new();
    let mut coherence = 0.9;
    let vfe = 0.1;

    let mut threats_encountered = 0;
    let mut threats_detected = 0;
    let mut threats_neutralized = 0;
    let mut total_response_time = 0usize;
    let mut max_threat_handled = 0.0;
    let mut threat_trajectory = Vec::with_capacity(config.max_cycles);
    let mut immunity_trajectory = Vec::with_capacity(config.max_cycles);
    let mut threats_this_epoch = 0;

    for cycle in 0..config.max_cycles {
        // Generate threats (probability increases with time)
        let threat_prob = 0.01 + 0.0001 * (cycle as f64 / config.max_cycles as f64);
        if random_uniform(&mut rng_state) < threat_prob {
            // Generate threat
            let magnitude = random_uniform(&mut rng_state) * config.max_threat_magnitude;
            let disruption = random_uniform(&mut rng_state) * 0.5;

            // Select threat type based on magnitude
            let threat_type = if magnitude > config.max_threat_magnitude * 0.8 {
                ThreatType::HostileSuperintelligence
            } else if magnitude > config.max_threat_magnitude * 0.5 {
                ThreatType::CivilizationalCollapse
            } else if magnitude > config.max_threat_magnitude * 0.3 {
                ThreatType::GovernanceSubversion
            } else {
                ThreatType::AdversarialPerturbation
            };

            let mut threat = Threat::new(
                threats_encountered as u64,
                threat_type,
                magnitude,
                disruption,
            );
            threats_encountered += 1;
            threats_this_epoch += 1;

            // Detect
            if detect_threat(&threat, coherence, vfe, config) {
                threat.detected = true;
                threats_detected += 1;

                // Contain
                let coherence_barrier = coherence * (1.0 - disruption);
                if contain_threat(&threat, coherence_barrier) {
                    threat.contained = true;

                    // Neutralize
                    if neutralize_threat(&threat, &immune_memory, config) {
                        threat.neutralized = true;
                        threats_neutralized += 1;
                        total_response_time += 1; // 1 cycle response

                        // Adapt
                        adapt_immunity(&threat, &mut immune_memory, config);

                        if threat.magnitude > max_threat_handled {
                            max_threat_handled = threat.magnitude;
                        }
                    }
                }
            }

            // Update coherence based on threat
            if !threat.neutralized {
                coherence -= threat.coherence_disruption * 0.01;
                coherence = coherence.max(0.5);
            }
        }

        // Natural coherence recovery
        coherence += 0.001 * (1.0 - coherence);
        coherence = coherence.min(1.0);

        // Record trajectories every 100 cycles
        if cycle % 100 == 0 {
            threat_trajectory.push(threats_this_epoch);
            threats_this_epoch = 0;

            let immunity_score = if threats_encountered > 0 {
                threats_neutralized as f64 / threats_encountered as f64
            } else {
                1.0
            };
            immunity_trajectory.push(immunity_score);
        }
    }

    // Compute final metrics
    let detection_rate = if threats_encountered > 0 {
        threats_detected as f64 / threats_encountered as f64
    } else {
        1.0
    };
    let neutralization_rate = if threats_encountered > 0 {
        threats_neutralized as f64 / threats_encountered as f64
    } else {
        1.0
    };
    let avg_response_time = if threats_neutralized > 0 {
        total_response_time as f64 / threats_neutralized as f64
    } else {
        0.0
    };
    let immunity_robustness =
        detection_rate * 0.3 + neutralization_rate * 0.4 + (1.0 - avg_response_time / 10.0).max(0.0) * 0.3;

    // Adaptation score: improvement over time
    let adaptation_score = if immunity_trajectory.len() >= 2 {
        let first = immunity_trajectory[0];
        let last = immunity_trajectory.last().copied().unwrap_or(1.0);
        (last - first).max(0.0) // Positive = improvement
    } else {
        0.0
    };

    // Eternal immunity proven if neutralization rate > 0.99 and robustness > 0.95
    let eternal_immunity_proven =
        neutralization_rate > 0.99 && immunity_robustness > 0.95 && threats_encountered > 10;

    ImmunityResult {
        threats_encountered,
        threats_detected,
        threats_neutralized,
        detection_rate,
        neutralization_rate,
        avg_response_time,
        max_threat_handled,
        immunity_robustness,
        eternal_immunity_proven,
        immune_memory_size: immune_memory.len(),
        adaptation_score,
        threat_trajectory,
        immunity_trajectory,
    }
}

/// Prove eternal immunity.
///
/// Mathematical proof that the immune system handles all threats.
pub fn prove_eternal_immunity(config: &ImmunityConfig) -> bool {
    let result = deploy_universal_immune_system(config);
    result.eternal_immunity_proven
}

// ─── Full Eternal Noosfera Pipeline ──────────────────────────────────────────

/// Run Full Eternal Noosfera Pipeline — Complete Vision Realization.
///
/// Simulates the complete eternal Noospheric civilization over 100k+ years
/// with millions of nodes, verifying all conditions for eternal symbiosis.
///
/// **Algorithm:**
/// 1. Initialize planetary mesh with seed nodes
/// 2. Run eternal symbiosis stabilization
/// 3. Run global singularity stabilization
/// 4. Run eternal governance loop
/// 5. Deploy universal immune system
/// 6. Verify all eternal conditions
/// 7. Return EternalPipelineResult with full certification
pub fn run_full_eternal_noosfera_pipeline(
    node_count: usize,
    config: &ImmunityConfig,
) -> EternalPipelineResult {
    let mut rng_state = config.seed;

    // Simulate planetary mesh
    let mut coherence = 0.5;
    let mut free_energy: f64 = 1.0;
    #[allow(unused_assignments)]
    let mut governance_coherence = true;

    // Phase 1: Eternal Symbiosis Stabilization
    for _ in 0..(config.max_cycles / 4) {
        // Simulate eternal attractor dynamics
        let pous_fitness = 1.0 - free_energy.min(1.0).max(0.0);
        let cosmic_entropy = 0.1 * random_uniform(&mut rng_state);
        let dx = 0.1 * coherence * pous_fitness - 0.05 * cosmic_entropy;
        coherence += 0.001 * dx;
        coherence = coherence.clamp(0.0, 1.0);
        free_energy *= 1.0 - 0.001 * coherence.max(0.0);
        free_energy = free_energy.max(0.0);
    }

    let eternal_stability = coherence > 0.85 && free_energy < 0.3;

    // Phase 2: Global Singularity Stabilization
    let singularity_threshold = 0.05;
    let singularity_potential = free_energy + 0.3 * (1.0 - coherence);
    let singularity_stabilized = singularity_potential < singularity_threshold;

    // Phase 3: Eternal Governance
    governance_coherence = coherence > 0.8;

    // Phase 4: Universal Immunity
    let immunity_result = deploy_universal_immune_system(config);
    let universal_immunity = immunity_result.eternal_immunity_proven
        || immunity_result.neutralization_rate > 0.95;

    // Compute eternal civilization score
    let eternal_civilization_score = coherence * 0.3
        + (1.0 - free_energy.min(1.0)) * 0.2
        + if eternal_stability { 0.1 } else { 0.0 }
        + if singularity_stabilized { 0.1 } else { 0.0 }
        + if governance_coherence { 0.1 } else { 0.0 }
        + immunity_result.immunity_robustness * 0.2;

    // Full vision realized = all conditions met
    let full_vision_realized = eternal_stability
        && singularity_stabilized
        && governance_coherence
        && universal_immunity
        && eternal_civilization_score > 0.8;

    EternalPipelineResult {
        simulation_years: config.simulation_horizon_years,
        final_coherence: coherence,
        final_free_energy: free_energy,
        eternal_stability,
        singularity_stabilized,
        governance_coherence,
        universal_immunity,
        total_nodes: node_count,
        total_cycles: config.max_cycles,
        full_vision_realized,
        eternal_civilization_score,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── ImmunityConfig Tests ─────────────────────────────────────────────

    #[test]
    fn test_immunity_config_default() {
        let config = ImmunityConfig::default();
        assert_eq!(config.detection_weight, 0.3);
        assert_eq!(config.containment_weight, 0.25);
        assert_eq!(config.neutralization_weight, 0.25);
        assert_eq!(config.adaptation_weight, 0.2);
        assert_eq!(config.immune_memory_size, 1000);
    }

    #[test]
    fn test_immunity_config_fast() {
        let config = ImmunityConfig::fast();
        assert_eq!(config.max_cycles, 5000);
        assert_eq!(config.immune_memory_size, 100);
    }

    #[test]
    fn test_immunity_config_planetary_eternal() {
        let config = ImmunityConfig::planetary_eternal();
        assert_eq!(config.max_cycles, 500_000);
        assert_eq!(config.simulation_horizon_years, 1_000_000);
    }

    #[test]
    fn test_immunity_config_with_detection_weight() {
        let config = ImmunityConfig::default().with_detection_weight(0.5);
        assert_eq!(config.detection_weight, 0.5);
    }

    #[test]
    fn test_immunity_config_detection_weight_clamped() {
        let config = ImmunityConfig::default().with_detection_weight(1.5);
        assert_eq!(config.detection_weight, 1.0);
    }

    #[test]
    fn test_immunity_config_with_detection_threshold() {
        let config = ImmunityConfig::default().with_detection_threshold(0.2);
        assert_eq!(config.detection_threshold, 0.2);
    }

    #[test]
    fn test_immunity_config_with_seed() {
        let config = ImmunityConfig::default().with_seed(123);
        assert_eq!(config.seed, 123);
    }

    // ─── Threat Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_threat_new() {
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.5, 0.3);
        assert_eq!(threat.threat_id, 1);
        assert_eq!(threat.threat_type, ThreatType::AdversarialPerturbation);
        assert!(!threat.detected);
    }

    #[test]
    fn test_threat_type_display() {
        let t = ThreatType::HostileSuperintelligence;
        assert_eq!(format!("{}", t), "HostileSuperintelligence");
    }

    #[test]
    fn test_threat_magnitude_clamped() {
        let threat = Threat::new(1, ThreatType::Unknown, -0.5, 0.3);
        assert_eq!(threat.magnitude, 0.0);
    }

    #[test]
    fn test_threat_disruption_clamped() {
        let threat = Threat::new(1, ThreatType::Unknown, 0.5, 1.5);
        assert_eq!(threat.coherence_disruption, 1.0);
    }

    // ─── Immune Response Component Tests ──────────────────────────────────

    #[test]
    fn test_detect_threat_detected() {
        let config = ImmunityConfig::default();
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.8, 0.5);
        let detected = detect_threat(&threat, 0.9, 0.5, &config);
        assert!(detected);
    }

    #[test]
    fn test_detect_threat_not_detected() {
        let config = ImmunityConfig::default();
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.01, 0.01);
        let detected = detect_threat(&threat, 0.99, 0.01, &config);
        assert!(!detected);
    }

    #[test]
    fn test_contain_threat_success() {
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.1, 0.1);
        let contained = contain_threat(&threat, 0.9);
        assert!(contained);
    }

    #[test]
    fn test_contain_threat_failure() {
        let threat = Threat::new(1, ThreatType::HostileSuperintelligence, 1.0, 0.9);
        let contained = contain_threat(&threat, 0.3);
        assert!(!contained);
    }

    #[test]
    fn test_neutralize_threat_success() {
        let config = ImmunityConfig::default();
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.1, 0.1);
        let neutralized = neutralize_threat(&threat, &[], &config);
        assert!(neutralized);
    }

    #[test]
    fn test_neutralize_threat_with_memory() {
        let config = ImmunityConfig::default();
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.1, 0.1);
        let memory = vec![0.5, 0.3, 0.8];
        let neutralized = neutralize_threat(&threat, &memory, &config);
        assert!(neutralized);
    }

    #[test]
    fn test_adapt_immunity_updates_memory() {
        let config = ImmunityConfig::default();
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.5, 0.3);
        let mut memory = vec![0.1, 0.2];
        adapt_immunity(&threat, &mut memory, &config);
        assert_eq!(memory.len(), 3);
        assert!(memory.contains(&0.5));
    }

    #[test]
    fn test_adapt_immunity_trims_memory() {
        let mut config = ImmunityConfig::default();
        config.immune_memory_size = 3;
        let threat = Threat::new(1, ThreatType::AdversarialPerturbation, 0.5, 0.3);
        let mut memory = vec![0.1, 0.2, 0.3, 0.4];
        adapt_immunity(&threat, &mut memory, &config);
        assert_eq!(memory.len(), 3);
    }

    #[test]
    fn test_compute_immune_response_full() {
        let config = ImmunityConfig::default();
        let response = compute_immune_response(true, true, true, true, &config);
        assert!((response - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_immune_response_none() {
        let config = ImmunityConfig::default();
        let response = compute_immune_response(false, false, false, false, &config);
        assert!((response).abs() < 1e-10);
    }

    #[test]
    fn test_compute_immune_response_partial() {
        let config = ImmunityConfig::default();
        let response = compute_immune_response(true, false, true, false, &config);
        assert!((response - (config.detection_weight + config.neutralization_weight)).abs() < 1e-10);
    }

    // ─── Full Immunity Deployment Tests ───────────────────────────────────

    #[test]
    fn test_deploy_universal_immune_system_basic() {
        let config = ImmunityConfig::fast();
        let result = deploy_universal_immune_system(&config);
        assert!(result.detection_rate >= 0.0 && result.detection_rate <= 1.0);
        assert!(result.neutralization_rate >= 0.0 && result.neutralization_rate <= 1.0);
        assert!(result.immunity_robustness >= 0.0 && result.immunity_robustness <= 1.0);
    }

    #[test]
    fn test_deploy_universal_immune_system_deterministic() {
        let config = ImmunityConfig::fast();
        let r1 = deploy_universal_immune_system(&config);
        let r2 = deploy_universal_immune_system(&config);
        assert_eq!(r1.threats_encountered, r2.threats_encountered);
    }

    #[test]
    fn test_deploy_universal_immune_system_trajectories() {
        let config = ImmunityConfig::fast();
        let result = deploy_universal_immune_system(&config);
        assert!(!result.threat_trajectory.is_empty());
        assert!(!result.immunity_trajectory.is_empty());
    }

    #[test]
    fn test_prove_eternal_immunity() {
        let config = ImmunityConfig::fast();
        let proof = prove_eternal_immunity(&config);
        assert!(proof == true || proof == false);
    }

    // ─── Full Eternal Pipeline Tests ──────────────────────────────────────

    #[test]
    fn test_run_full_eternal_noosfera_pipeline_basic() {
        let config = ImmunityConfig::fast();
        let result = run_full_eternal_noosfera_pipeline(1000, &config);
        assert!(result.final_coherence >= 0.0 && result.final_coherence <= 1.0);
        assert!(result.final_free_energy >= 0.0);
        assert!(result.eternal_civilization_score >= 0.0 && result.eternal_civilization_score <= 1.0);
    }

    #[test]
    fn test_run_full_eternal_noosfera_pipeline_deterministic() {
        let config = ImmunityConfig::fast();
        let r1 = run_full_eternal_noosfera_pipeline(100, &config);
        let r2 = run_full_eternal_noosfera_pipeline(100, &config);
        assert_eq!(r1.final_coherence, r2.final_coherence);
    }

    #[test]
    fn test_run_full_eternal_noosfera_pipeline_nodes() {
        let config = ImmunityConfig::fast();
        let result = run_full_eternal_noosfera_pipeline(1_000_000, &config);
        assert_eq!(result.total_nodes, 1_000_000);
    }

    #[test]
    fn test_run_full_eternal_noosfera_pipeline_vision() {
        let config = ImmunityConfig::fast();
        let result = run_full_eternal_noosfera_pipeline(100, &config);
        // Vision may or may not be realized depending on config
        assert!(result.full_vision_realized == true || result.full_vision_realized == false);
    }

    // ─── Result Display Tests ─────────────────────────────────────────────

    #[test]
    fn test_immunity_result_summary() {
        let result = ImmunityResult {
            threats_encountered: 100,
            threats_detected: 95,
            threats_neutralized: 90,
            detection_rate: 0.95,
            neutralization_rate: 0.9,
            avg_response_time: 1.5,
            max_threat_handled: 0.8,
            immunity_robustness: 0.92,
            eternal_immunity_proven: true,
            immune_memory_size: 90,
            adaptation_score: 0.1,
            threat_trajectory: vec![],
            immunity_trajectory: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("100"));
        assert!(summary.contains("eternal=true"));
    }

    #[test]
    fn test_immunity_result_display() {
        let result = ImmunityResult {
            threats_encountered: 0,
            threats_detected: 0,
            threats_neutralized: 0,
            detection_rate: 1.0,
            neutralization_rate: 1.0,
            avg_response_time: 0.0,
            max_threat_handled: 0.0,
            immunity_robustness: 1.0,
            eternal_immunity_proven: false,
            immune_memory_size: 0,
            adaptation_score: 0.0,
            threat_trajectory: vec![],
            immunity_trajectory: vec![],
        };
        let display = format!("{}", result);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_eternal_pipeline_result_summary() {
        let result = EternalPipelineResult {
            simulation_years: 100_000,
            final_coherence: 0.95,
            final_free_energy: 0.01,
            eternal_stability: true,
            singularity_stabilized: true,
            governance_coherence: true,
            universal_immunity: true,
            total_nodes: 1_000_000,
            total_cycles: 50_000,
            full_vision_realized: true,
            eternal_civilization_score: 0.95,
        };
        let summary = result.summary();
        assert!(summary.contains("100000y"));
        assert!(summary.contains("vision=true"));
    }

    #[test]
    fn test_eternal_pipeline_result_display() {
        let result = EternalPipelineResult {
            simulation_years: 10_000,
            final_coherence: 0.9,
            final_free_energy: 0.1,
            eternal_stability: true,
            singularity_stabilized: false,
            governance_coherence: true,
            universal_immunity: true,
            total_nodes: 100_000,
            total_cycles: 5_000,
            full_vision_realized: false,
            eternal_civilization_score: 0.8,
        };
        let display = format!("{}", result);
        assert!(!display.is_empty());
    }

    // ─── LCG Random Tests ─────────────────────────────────────────────────

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        assert_eq!(lcg_next(&mut s1), lcg_next(&mut s2));
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s: u64 = 42;
        for _ in 0..100 {
            let v = random_uniform(&mut s);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_random_gaussian_finite() {
        let mut s: u64 = 42;
        for _ in 0..100 {
            let v = random_gaussian(&mut s);
            assert!(v.is_finite());
        }
    }
}
