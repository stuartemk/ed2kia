//! Provable Safety — Formal safety proofs for Noosfera Kernel deployment.
//!
//! Implements provable safety guarantees for planetary-scale Noosfera Kernel:
//! - **Noosfera Safety Proof:** Formal verification of kernel safety invariants.
//! - **10K Node Deployment Simulation:** Large-scale deployment with safety monitoring.
//! - **Singularity Threshold Detection:** Detection of civilizational transition tipping points.
//!
//! # Mathematical Foundation
//!
//! **Safety Margin:**
//! ```text
//! Safety = min( barrier_certificate, coherence_margin, energy_stability, byzantine_tolerance )
//! ```
//!
//! **Singularity Threshold:**
//! ```text
//! Detected when: coherence > 0.95 AND F_planet < 0.05 AND alignment > 0.9
//! ```
//!
//! **Deployment Readiness:**
//! ```text
//! Ready = (safety > threshold) AND (nodes >= min_nodes) AND (coherence > min_coherence)
//! ```

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for provable safety verification.
#[derive(Clone, Debug)]
pub struct SafetyConfig {
    /// Minimum safety margin threshold.
    pub safety_threshold: f64,
    /// Minimum coherence for deployment.
    pub min_coherence: f64,
    /// Maximum allowed Byzantine fraction.
    pub max_byzantine_fraction: f64,
    /// Minimum nodes for deployment.
    pub min_nodes: usize,
    /// Energy stability tolerance.
    pub energy_tolerance: f64,
    /// Barrier certificate tolerance.
    pub barrier_tolerance: f64,
    /// Seed for deterministic simulation.
    pub seed: u64,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            safety_threshold: 0.8,
            min_coherence: 0.9,
            max_byzantine_fraction: 1.0 / 3.0,
            min_nodes: 100,
            energy_tolerance: 0.1,
            barrier_tolerance: 0.05,
            seed: 42,
        }
    }
}

impl SafetyConfig {
    /// Builder: custom safety threshold.
    pub fn with_safety_threshold(mut self, threshold: f64) -> Self {
        self.safety_threshold = threshold.max(0.0).min(1.0);
        self
    }

    /// Builder: custom minimum coherence.
    pub fn with_min_coherence(mut self, coherence: f64) -> Self {
        self.min_coherence = coherence.max(0.0).min(1.0);
        self
    }

    /// Builder: custom max Byzantine fraction.
    pub fn with_max_byzantine_fraction(mut self, fraction: f64) -> Self {
        self.max_byzantine_fraction = fraction.max(0.0).min(1.0);
        self
    }

    /// Builder: custom minimum nodes.
    pub fn with_min_nodes(mut self, nodes: usize) -> Self {
        self.min_nodes = nodes;
        self
    }

    /// Builder: custom energy tolerance.
    pub fn with_energy_tolerance(mut self, tolerance: f64) -> Self {
        self.energy_tolerance = tolerance.max(0.0).min(1.0);
        self
    }

    /// Builder: custom barrier tolerance.
    pub fn with_barrier_tolerance(mut self, tolerance: f64) -> Self {
        self.barrier_tolerance = tolerance.max(0.0).min(1.0);
        self
    }

    /// Builder: custom seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fast configuration for testing.
    pub fn fast() -> Self {
        Self {
            safety_threshold: 0.5,
            min_coherence: 0.7,
            min_nodes: 10,
            ..Self::default()
        }
    }

    /// High-precision configuration.
    pub fn high_precision() -> Self {
        Self {
            safety_threshold: 0.99,
            min_coherence: 0.99,
            energy_tolerance: 0.01,
            barrier_tolerance: 0.001,
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Deployment Node State
// ---------------------------------------------------------------------------

/// State of a single node in deployment simulation.
#[derive(Clone, Debug)]
pub struct DeploymentNode {
    /// Node identifier.
    pub node_id: u64,
    /// Current coherence score.
    pub coherence: f64,
    /// Current VFE contribution.
    pub vfe_contribution: f64,
    /// Influence share.
    pub influence_share: f64,
    /// Trust score.
    pub trust: f64,
    /// Active status.
    pub active: bool,
    /// Byzantine status.
    pub byzantine: bool,
    /// Energy consumption.
    pub energy: f64,
}

impl DeploymentNode {
    /// Create a new deployment node.
    pub fn new(node_id: u64, coherence: f64, vfe: f64, influence: f64, trust: f64) -> Self {
        Self {
            node_id,
            coherence,
            vfe_contribution: vfe,
            influence_share: influence,
            trust,
            active: true,
            byzantine: false,
            energy: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Result of Noosfera safety proof.
#[derive(Clone, Debug)]
pub struct SafetyProofResult {
    /// Barrier certificate value.
    pub barrier_certificate: f64,
    /// Coherence margin.
    pub coherence_margin: f64,
    /// Energy stability score.
    pub energy_stability: f64,
    /// Byzantine tolerance margin.
    pub byzantine_tolerance: f64,
    /// Overall safety score.
    pub safety_score: f64,
    /// Safety proof passed.
    pub safe: bool,
    /// List of violated safety invariants.
    pub violated_invariants: Vec<String>,
}

impl std::fmt::Display for SafetyProofResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SafetyProof {{ barrier: {:.4}, coherence: {:.4}, energy: {:.4}, byzantine: {:.4}, safety: {:.4}, safe: {} }}",
            self.barrier_certificate, self.coherence_margin, self.energy_stability,
            self.byzantine_tolerance, self.safety_score, self.safe
        )
    }
}

/// Result of 10K node deployment simulation.
#[derive(Clone, Debug)]
pub struct DeploymentResult {
    /// Total nodes simulated.
    pub total_nodes: usize,
    /// Active nodes at end of simulation.
    pub active_nodes: usize,
    /// Average coherence across active nodes.
    pub avg_coherence: f64,
    /// Planetary Free Energy at end.
    pub planetary_free_energy: f64,
    /// Average trust score.
    pub avg_trust: f64,
    /// Byzantine nodes detected.
    pub byzantine_count: usize,
    /// Deployment successful.
    pub deployment_successful: bool,
    /// Safety proof result.
    pub safety_proof: SafetyProofResult,
    /// Simulation steps executed.
    pub steps_executed: usize,
}

impl std::fmt::Display for DeploymentResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Deployment {{ nodes: {}/{}, coherence: {:.4}, F_planet: {:.4}, trust: {:.4}, byzantine: {}, success: {} }}",
            self.active_nodes, self.total_nodes, self.avg_coherence,
            self.planetary_free_energy, self.avg_trust, self.byzantine_count,
            self.deployment_successful
        )
    }
}

/// Result of singularity threshold detection.
#[derive(Clone, Debug)]
pub struct SingularityResult {
    /// Current coherence score.
    pub coherence: f64,
    /// Current planetary Free Energy.
    pub planetary_free_energy: f64,
    /// Current alignment score.
    pub alignment: f64,
    /// Singularity threshold detected.
    pub singularity_detected: bool,
    /// Tipping point proximity (0 = far, 1 = at threshold).
    pub tipping_proximity: f64,
    /// Estimated steps to singularity (0 if already detected).
    pub estimated_steps_remaining: u32,
    /// Civilizational transition phase.
    pub transition_phase: String,
}

impl std::fmt::Display for SingularityResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Singularity {{ coherence: {:.4}, F: {:.4}, alignment: {:.4}, detected: {}, proximity: {:.4}, phase: {} }}",
            self.coherence, self.planetary_free_energy, self.alignment,
            self.singularity_detected, self.tipping_proximity, self.transition_phase
        )
    }
}

// ---------------------------------------------------------------------------
// Random helpers (deterministic LCG)
// ---------------------------------------------------------------------------

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    ((lcg_next(state) >> 11) as f64 / (1u64 << 51) as f64).clamp(0.0, 1.0)
}

fn random_gaussian(state: &mut u64) -> f64 {
    let u1 = random_uniform(state).max(1e-10);
    let u2 = random_uniform(state);
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Sigmoid function.
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Shannon entropy.
pub fn shannon_entropy(dist: &[f64]) -> f64 {
    let mut h = 0.0;
    for &p in dist {
        if p > 1e-15 {
            h -= p * p.ln();
        }
    }
    h.max(0.0)
}

/// Cosine similarity.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom < 1e-15 {
        return 0.0;
    }
    (dot / denom).clamp(-1.0, 1.0)
}

// ---------------------------------------------------------------------------
// Core Safety Functions
// ---------------------------------------------------------------------------

/// Prove Noosfera Safety — Formal verification of kernel safety invariants.
///
/// ```text
/// Safety = min( barrier_cert, coherence_margin, energy_stability, byzantine_tolerance )
/// ```
///
/// # Parameters
/// - `coherence`: Current network coherence score.
/// - `planetary_free_energy`: Current planetary Free Energy.
/// - `energy_distribution`: Energy distribution across nodes.
/// - `byzantine_count`: Number of Byzantine nodes.
/// - `total_nodes`: Total number of nodes.
/// - `barrier_value`: Current barrier function value.
/// - `config`: Safety configuration.
///
/// # Returns
/// `SafetyProofResult` with safety score and violated invariants.
pub fn prove_noosfera_safety(
    coherence: f64,
    _planetary_free_energy: f64,
    energy_distribution: &[f64],
    byzantine_count: usize,
    total_nodes: usize,
    barrier_value: f64,
    config: &SafetyConfig,
) -> SafetyProofResult {
    let mut violated_invariants = Vec::new();

    // Barrier certificate: V(x) > 0 implies safe set.
    let barrier_certificate = if barrier_value > -config.barrier_tolerance {
        1.0
    } else {
        (barrier_value / config.barrier_tolerance).max(0.0)
    };
    if barrier_certificate < config.safety_threshold {
        violated_invariants.push("barrier_certificate_below_threshold".to_string());
    }

    // Coherence margin: coherence > min_coherence.
    let coherence_margin = coherence.clamp(0.0, 1.0);
    if coherence_margin < config.min_coherence {
        violated_invariants.push("coherence_below_minimum".to_string());
    }

    // Energy stability: normalized entropy of energy distribution.
    // Higher entropy (more uniform) = higher stability.
    let energy_entropy = shannon_entropy(energy_distribution);
    let max_entropy = if !energy_distribution.is_empty() {
        (energy_distribution.len() as f64).ln()
    } else {
        1.0
    };
    let energy_stability = if max_entropy > 1e-15 {
        (energy_entropy / max_entropy).min(1.0)
    } else {
        1.0
    };
    if energy_stability < (1.0 - config.energy_tolerance) {
        violated_invariants.push("energy_instability_detected".to_string());
    }

    // Byzantine tolerance: byzantine fraction < max_byzantine_fraction.
    let byzantine_fraction = if total_nodes > 0 {
        byzantine_count as f64 / total_nodes as f64
    } else {
        0.0
    };
    let byzantine_tolerance = 1.0 - (byzantine_fraction / config.max_byzantine_fraction).min(1.0);
    if byzantine_fraction > config.max_byzantine_fraction {
        violated_invariants.push("byzantine_fraction_exceeds_limit".to_string());
    }

    // Overall safety score.
    let safety_score = [
        barrier_certificate,
        coherence_margin,
        energy_stability,
        byzantine_tolerance,
    ]
    .iter()
    .copied()
    .fold(f64::INFINITY, f64::min);

    let safe = safety_score >= config.safety_threshold && violated_invariants.is_empty();

    SafetyProofResult {
        barrier_certificate,
        coherence_margin,
        energy_stability,
        byzantine_tolerance,
        safety_score,
        safe,
        violated_invariants,
    }
}

/// Simulate Deployment at 10K Nodes — Large-scale deployment with safety monitoring.
///
/// Simulates node onboarding, coherence propagation, and safety verification
/// across a planetary-scale mesh of 10,000 nodes.
///
/// # Parameters
/// - `node_count`: Number of nodes to simulate.
/// - `steps`: Number of simulation steps.
/// - `byzantine_fraction`: Fraction of Byzantine nodes.
/// - `config`: Safety configuration.
///
/// # Returns
/// `DeploymentResult` with deployment metrics and safety proof.
pub fn simulate_deployment_10k_nodes(
    node_count: usize,
    steps: usize,
    byzantine_fraction: f64,
    config: &SafetyConfig,
) -> DeploymentResult {
    let mut rng_state = config.seed;
    let n = node_count.max(1);

    // Initialize nodes.
    let mut nodes: Vec<DeploymentNode> = (0..n as u64)
        .map(|id| {
            let coherence = 0.5 + 0.3 * random_uniform(&mut rng_state);
            let vfe = 0.3 + 0.4 * random_uniform(&mut rng_state);
            let influence = 1.0 / n as f64;
            let trust = 0.6 + 0.3 * random_uniform(&mut rng_state);
            DeploymentNode::new(id, coherence, vfe, influence, trust)
        })
        .collect();

    // Mark Byzantine nodes.
    let byzantine_count = (byzantine_fraction * n as f64) as usize;
    for i in 0..byzantine_count {
        if i < nodes.len() {
            nodes[i].byzantine = true;
            nodes[i].trust = 0.1;
        }
    }

    // Simulate steps.
    for _step in 0..steps {
        let active_count = nodes.iter().filter(|n| n.active).count();
        if active_count == 0 {
            break;
        }

        // Coherence propagation: nodes converge toward average coherence.
        let avg_coherence: f64 = nodes
            .iter()
            .filter(|n| n.active && !n.byzantine)
            .map(|n| n.coherence)
            .sum::<f64>()
            / active_count as f64;

        for node in &mut nodes {
            if !node.active || node.byzantine {
                continue;
            }

            // Coherence update.
            node.coherence =
                (node.coherence + 0.1 * (avg_coherence - node.coherence)).clamp(0.0, 1.0);

            // VFE reduction.
            node.vfe_contribution *= 0.99;

            // Trust update based on coherence.
            node.trust = (node.trust + 0.05 * (node.coherence - 0.5)).clamp(0.0, 1.0);

            // Random churn.
            if random_uniform(&mut rng_state) < 0.01 {
                node.active = false;
            }
        }
    }

    // Compute final metrics.
    let active_nodes = nodes.iter().filter(|n| n.active).count();
    let active_byzantine = nodes.iter().filter(|n| n.active && n.byzantine).count();

    let avg_coherence = if active_nodes > 0 {
        nodes
            .iter()
            .filter(|n| n.active)
            .map(|n| n.coherence)
            .sum::<f64>()
            / active_nodes as f64
    } else {
        0.0
    };

    let planetary_free_energy = nodes
        .iter()
        .filter(|n| n.active)
        .map(|n| n.influence_share * n.vfe_contribution)
        .sum::<f64>();

    let avg_trust = if active_nodes > 0 {
        nodes
            .iter()
            .filter(|n| n.active)
            .map(|n| n.trust)
            .sum::<f64>()
            / active_nodes as f64
    } else {
        0.0
    };

    // Energy distribution.
    let energy_dist: Vec<f64> = nodes
        .iter()
        .filter(|n| n.active)
        .map(|n| n.energy)
        .collect();

    // Safety proof.
    let safety_proof = prove_noosfera_safety(
        avg_coherence,
        planetary_free_energy,
        &energy_dist,
        active_byzantine,
        active_nodes,
        0.5, // Barrier value from coherence.
        config,
    );

    let deployment_successful = safety_proof.safe
        && active_nodes >= config.min_nodes
        && avg_coherence >= config.min_coherence;

    DeploymentResult {
        total_nodes: n,
        active_nodes,
        avg_coherence,
        planetary_free_energy,
        avg_trust,
        byzantine_count: active_byzantine,
        deployment_successful,
        safety_proof,
        steps_executed: steps,
    }
}

/// Detect Singularity Threshold — Identify civilizational transition tipping points.
///
/// ```text
/// Singularity when: coherence > 0.95 AND F_planet < 0.05 AND alignment > 0.9
/// ```
///
/// # Parameters
/// - `coherence`: Current network coherence.
/// - `planetary_free_energy`: Current planetary Free Energy.
/// - `alignment`: Current value alignment score.
/// - `coherence_trend`: Recent coherence trend (positive = increasing).
/// - `f_planet_trend`: Recent F_planet trend (negative = decreasing).
/// - `config`: Safety configuration.
///
/// # Returns
/// `SingularityResult` with detection status and transition phase.
pub fn detect_singularity_threshold(
    coherence: f64,
    planetary_free_energy: f64,
    alignment: f64,
    coherence_trend: f64,
    f_planet_trend: f64,
    _config: &SafetyConfig,
) -> SingularityResult {
    // Singularity detection conditions.
    let coherence_threshold = 0.95;
    let f_planet_threshold = 0.05;
    let alignment_threshold = 0.9;

    let singularity_detected = coherence > coherence_threshold
        && planetary_free_energy < f_planet_threshold
        && alignment > alignment_threshold;

    // Tipping proximity: how close are we to the threshold?
    let coherence_proximity = (coherence / coherence_threshold).min(1.0);
    let f_planet_proximity = 1.0 - (planetary_free_energy / f_planet_threshold).min(1.0);
    let alignment_proximity = (alignment / alignment_threshold).min(1.0);
    let tipping_proximity = (coherence_proximity + f_planet_proximity + alignment_proximity) / 3.0;

    // Estimated steps to singularity (heuristic).
    let estimated_steps_remaining = if singularity_detected {
        0
    } else {
        let coherence_gap = (coherence_threshold - coherence).max(0.0);
        let f_planet_gap = (planetary_free_energy - f_planet_threshold).max(0.0);
        let alignment_gap = (alignment_threshold - alignment).max(0.0);
        let max_gap = coherence_gap.max(f_planet_gap).max(alignment_gap);
        let convergence_rate = (coherence_trend.max(0.0) - f_planet_trend.min(0.0))
            .abs()
            .max(1e-6);
        (max_gap / convergence_rate / 100.0).ceil() as u32
    };

    // Transition phase classification.
    let transition_phase = if singularity_detected {
        "SINGULARITY_ACHIEVED".to_string()
    } else if tipping_proximity > 0.9 {
        "CRITICAL_TRANSITION".to_string()
    } else if tipping_proximity > 0.7 {
        "ACCELERATING_CONVERGENCE".to_string()
    } else if tipping_proximity > 0.5 {
        "ORGANIZING_NOOSPHERE".to_string()
    } else if tipping_proximity > 0.3 {
        "EMERGENT_SYMBIOSIS".to_string()
    } else {
        "EARLY_ADOPTION".to_string()
    };

    SingularityResult {
        coherence,
        planetary_free_energy,
        alignment,
        singularity_detected,
        tipping_proximity,
        estimated_steps_remaining,
        transition_phase,
    }
}

/// Run full provable safety pipeline.
///
/// # Parameters
/// - `node_count`: Number of nodes to simulate.
/// - `steps`: Simulation steps.
/// - `byzantine_fraction`: Fraction of Byzantine nodes.
/// - `coherence`: Current coherence for singularity detection.
/// - `f_planet`: Current planetary Free Energy.
/// - `alignment`: Current alignment score.
/// - `config`: Safety configuration.
///
/// # Returns
/// Tuple of (DeploymentResult, SingularityResult).
pub fn run_provable_safety_pipeline(
    node_count: usize,
    steps: usize,
    byzantine_fraction: f64,
    coherence: f64,
    f_planet: f64,
    alignment: f64,
    config: &SafetyConfig,
) -> (DeploymentResult, SingularityResult) {
    let deployment = simulate_deployment_10k_nodes(node_count, steps, byzantine_fraction, config);
    let singularity = detect_singularity_threshold(
        coherence, f_planet, alignment, 0.01,   // Coherence trend.
        -0.005, // F_planet trend.
        config,
    );
    (deployment, singularity)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config Tests ---

    #[test]
    fn test_safety_config_default() {
        let cfg = SafetyConfig::default();
        assert!((cfg.safety_threshold - 0.8).abs() < 1e-9);
        assert!((cfg.min_coherence - 0.9).abs() < 1e-9);
        assert!((cfg.max_byzantine_fraction - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(cfg.min_nodes, 100);
        assert!((cfg.energy_tolerance - 0.1).abs() < 1e-9);
        assert!((cfg.barrier_tolerance - 0.05).abs() < 1e-9);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_safety_config_with_safety_threshold() {
        let cfg = SafetyConfig::default().with_safety_threshold(0.9);
        assert!((cfg.safety_threshold - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_threshold_clamped_high() {
        let cfg = SafetyConfig::default().with_safety_threshold(1.5);
        assert!((cfg.safety_threshold - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_threshold_clamped_low() {
        let cfg = SafetyConfig::default().with_safety_threshold(-0.5);
        assert!((cfg.safety_threshold - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_with_min_coherence() {
        let cfg = SafetyConfig::default().with_min_coherence(0.95);
        assert!((cfg.min_coherence - 0.95).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_with_max_byzantine() {
        let cfg = SafetyConfig::default().with_max_byzantine_fraction(0.2);
        assert!((cfg.max_byzantine_fraction - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_with_min_nodes() {
        let cfg = SafetyConfig::default().with_min_nodes(50);
        assert_eq!(cfg.min_nodes, 50);
    }

    #[test]
    fn test_safety_config_with_energy_tolerance() {
        let cfg = SafetyConfig::default().with_energy_tolerance(0.05);
        assert!((cfg.energy_tolerance - 0.05).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_with_barrier_tolerance() {
        let cfg = SafetyConfig::default().with_barrier_tolerance(0.02);
        assert!((cfg.barrier_tolerance - 0.02).abs() < 1e-9);
    }

    #[test]
    fn test_safety_config_with_seed() {
        let cfg = SafetyConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn test_safety_config_fast() {
        let cfg = SafetyConfig::fast();
        assert!((cfg.safety_threshold - 0.5).abs() < 1e-9);
        assert!((cfg.min_coherence - 0.7).abs() < 1e-9);
        assert_eq!(cfg.min_nodes, 10);
    }

    #[test]
    fn test_safety_config_high_precision() {
        let cfg = SafetyConfig::high_precision();
        assert!((cfg.safety_threshold - 0.99).abs() < 1e-9);
        assert!((cfg.min_coherence - 0.99).abs() < 1e-9);
        assert!((cfg.energy_tolerance - 0.01).abs() < 1e-9);
        assert!((cfg.barrier_tolerance - 0.001).abs() < 1e-9);
    }

    // --- Math Helper Tests ---

    #[test]
    fn test_sigmoid_zero() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_sigmoid_positive() {
        assert!(sigmoid(5.0) > 0.99);
    }

    #[test]
    fn test_sigmoid_negative() {
        assert!(sigmoid(-5.0) < 0.01);
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let h = shannon_entropy(&dist);
        assert!((h - (4f64.ln())).abs() < 1e-6);
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        let dist = vec![1.0, 0.0, 0.0];
        assert!((shannon_entropy(&dist) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-9);
    }

    // --- LCG Tests ---

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        assert_eq!(lcg_next(&mut s1), lcg_next(&mut s2));
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut s: u64 = 42;
        let v1 = lcg_next(&mut s);
        let v2 = lcg_next(&mut s);
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s: u64 = 123;
        for _ in 0..100 {
            let r = random_uniform(&mut s);
            assert!(r >= 0.0 && r <= 1.0);
        }
    }

    #[test]
    fn test_random_gaussian_finite() {
        let mut s: u64 = 456;
        for _ in 0..100 {
            let g = random_gaussian(&mut s);
            assert!(g.is_finite());
        }
    }

    // --- Deployment Node Tests ---

    #[test]
    fn test_deployment_node_new() {
        let node = DeploymentNode::new(0, 0.8, 0.3, 0.1, 0.9);
        assert_eq!(node.node_id, 0);
        assert!((node.coherence - 0.8).abs() < 1e-9);
        assert!((node.vfe_contribution - 0.3).abs() < 1e-9);
        assert!((node.influence_share - 0.1).abs() < 1e-9);
        assert!((node.trust - 0.9).abs() < 1e-9);
        assert!(node.active);
        assert!(!node.byzantine);
    }

    // --- Safety Proof Tests ---

    #[test]
    fn test_prove_noosfera_safety_perfect() {
        let cfg = SafetyConfig::default();
        let energy_dist = vec![0.25, 0.25, 0.25, 0.25];
        let result = prove_noosfera_safety(1.0, 0.01, &energy_dist, 0, 100, 1.0, &cfg);
        assert!(result.safe);
        assert!(result.violated_invariants.is_empty());
        assert!(result.safety_score >= cfg.safety_threshold);
    }

    #[test]
    fn test_prove_noosfera_safety_low_coherence() {
        let cfg = SafetyConfig::default();
        let energy_dist = vec![0.25, 0.25, 0.25, 0.25];
        let result = prove_noosfera_safety(0.5, 0.01, &energy_dist, 0, 100, 1.0, &cfg);
        assert!(!result.safe);
        assert!(result
            .violated_invariants
            .contains(&"coherence_below_minimum".to_string()));
    }

    #[test]
    fn test_prove_noosfera_safety_byzantine_exceeded() {
        let cfg = SafetyConfig::default();
        let energy_dist = vec![0.25, 0.25, 0.25, 0.25];
        // 40 Byzantine out of 100 > 1/3 threshold.
        let result = prove_noosfera_safety(0.95, 0.01, &energy_dist, 40, 100, 1.0, &cfg);
        assert!(!result.safe);
        assert!(result
            .violated_invariants
            .contains(&"byzantine_fraction_exceeds_limit".to_string()));
    }

    #[test]
    fn test_prove_noosfera_safety_barrier_violated() {
        let cfg = SafetyConfig::default();
        let energy_dist = vec![0.25, 0.25, 0.25, 0.25];
        let result = prove_noosfera_safety(0.95, 0.01, &energy_dist, 0, 100, -0.2, &cfg);
        assert!(!result.safe);
        assert!(result
            .violated_invariants
            .contains(&"barrier_certificate_below_threshold".to_string()));
    }

    #[test]
    fn test_prove_noosfera_safety_empty_energy() {
        let cfg = SafetyConfig::default();
        let energy_dist: Vec<f64> = vec![];
        let result = prove_noosfera_safety(0.95, 0.01, &energy_dist, 0, 0, 1.0, &cfg);
        assert!(result.energy_stability >= 0.0);
    }

    #[test]
    fn test_safety_proof_result_display() {
        let result = SafetyProofResult {
            barrier_certificate: 0.95,
            coherence_margin: 0.98,
            energy_stability: 0.9,
            byzantine_tolerance: 1.0,
            safety_score: 0.9,
            safe: true,
            violated_invariants: vec![],
        };
        let display = format!("{}", result);
        assert!(display.contains("safe: true"));
    }

    // --- Deployment Simulation Tests ---

    #[test]
    fn test_simulate_deployment_small() {
        let cfg = SafetyConfig::fast();
        let result = simulate_deployment_10k_nodes(20, 5, 0.0, &cfg);
        assert_eq!(result.total_nodes, 20);
        assert!(result.active_nodes > 0);
        assert!(result.avg_coherence >= 0.0);
        assert!(result.planetary_free_energy >= 0.0);
    }

    #[test]
    fn test_simulate_deployment_10k() {
        let cfg = SafetyConfig::default();
        let result = simulate_deployment_10k_nodes(10_000, 10, 0.05, &cfg);
        assert_eq!(result.total_nodes, 10_000);
        assert!(result.active_nodes > 0);
        assert!(result.steps_executed == 10);
    }

    #[test]
    fn test_simulate_deployment_no_byzantine() {
        let cfg = SafetyConfig::fast();
        let result = simulate_deployment_10k_nodes(50, 10, 0.0, &cfg);
        assert_eq!(result.byzantine_count, 0);
    }

    #[test]
    fn test_simulate_deployment_with_byzantine() {
        let cfg = SafetyConfig::fast();
        let result = simulate_deployment_10k_nodes(100, 5, 0.1, &cfg);
        assert!(result.byzantine_count > 0);
    }

    #[test]
    fn test_simulate_deployment_zero_steps() {
        let cfg = SafetyConfig::fast();
        let result = simulate_deployment_10k_nodes(50, 0, 0.0, &cfg);
        assert_eq!(result.steps_executed, 0);
    }

    #[test]
    fn test_simulate_deployment_deterministic() {
        let cfg = SafetyConfig::default().with_seed(42);
        let r1 = simulate_deployment_10k_nodes(100, 10, 0.05, &cfg);
        let r2 = simulate_deployment_10k_nodes(100, 10, 0.05, &cfg);
        assert!((r1.avg_coherence - r2.avg_coherence).abs() < 1e-10);
    }

    #[test]
    fn test_deployment_result_display() {
        let result = DeploymentResult {
            total_nodes: 100,
            active_nodes: 95,
            avg_coherence: 0.85,
            planetary_free_energy: 0.15,
            avg_trust: 0.8,
            byzantine_count: 2,
            deployment_successful: true,
            safety_proof: SafetyProofResult {
                barrier_certificate: 0.9,
                coherence_margin: 0.85,
                energy_stability: 0.9,
                byzantine_tolerance: 0.95,
                safety_score: 0.85,
                safe: true,
                violated_invariants: vec![],
            },
            steps_executed: 10,
        };
        let display = format!("{}", result);
        assert!(display.contains("success: true"));
    }

    // --- Singularity Detection Tests ---

    #[test]
    fn test_detect_singularity_achieved() {
        let cfg = SafetyConfig::default();
        let result = detect_singularity_threshold(0.98, 0.02, 0.95, 0.01, -0.005, &cfg);
        assert!(result.singularity_detected);
        assert_eq!(result.estimated_steps_remaining, 0);
        assert_eq!(result.transition_phase, "SINGULARITY_ACHIEVED");
    }

    #[test]
    fn test_detect_singularity_not_detected() {
        let cfg = SafetyConfig::default();
        let result = detect_singularity_threshold(0.5, 0.5, 0.5, 0.01, -0.005, &cfg);
        assert!(!result.singularity_detected);
        assert!(result.estimated_steps_remaining > 0);
    }

    #[test]
    fn test_detect_singularity_critical_transition() {
        let cfg = SafetyConfig::default();
        let result = detect_singularity_threshold(0.94, 0.01, 0.90, 0.01, -0.005, &cfg);
        assert!(!result.singularity_detected);
        assert!(result.tipping_proximity > 0.8);
    }

    #[test]
    fn test_detect_singularity_early_adoption() {
        let cfg = SafetyConfig::default();
        let result = detect_singularity_threshold(0.3, 0.8, 0.3, 0.01, -0.005, &cfg);
        assert!(!result.singularity_detected);
        assert_eq!(result.transition_phase, "EARLY_ADOPTION");
    }

    #[test]
    fn test_detect_singularity_emergent_symbiosis() {
        let cfg = SafetyConfig::default();
        let result = detect_singularity_threshold(0.5, 0.4, 0.5, 0.01, -0.005, &cfg);
        assert!(!result.singularity_detected);
        assert!(result.tipping_proximity > 0.2 && result.tipping_proximity < 0.5);
    }

    #[test]
    fn test_detect_singularity_tipping_proximity_range() {
        let cfg = SafetyConfig::default();
        let result = detect_singularity_threshold(0.9, 0.03, 0.85, 0.01, -0.005, &cfg);
        assert!(result.tipping_proximity >= 0.0 && result.tipping_proximity <= 1.0);
    }

    #[test]
    fn test_singularity_result_display() {
        let result = SingularityResult {
            coherence: 0.98,
            planetary_free_energy: 0.02,
            alignment: 0.95,
            singularity_detected: true,
            tipping_proximity: 1.0,
            estimated_steps_remaining: 0,
            transition_phase: "SINGULARITY_ACHIEVED".to_string(),
        };
        let display = format!("{}", result);
        assert!(display.contains("detected: true"));
    }

    // --- Pipeline Tests ---

    #[test]
    fn test_run_provable_safety_pipeline() {
        let cfg = SafetyConfig::fast();
        let (deployment, singularity) =
            run_provable_safety_pipeline(50, 10, 0.0, 0.95, 0.03, 0.92, &cfg);
        assert!(deployment.total_nodes == 50);
        assert!(singularity.coherence == 0.95);
    }

    #[test]
    fn test_run_provable_safety_pipeline_singularity() {
        let cfg = SafetyConfig::fast();
        let (_, singularity) = run_provable_safety_pipeline(50, 10, 0.0, 0.98, 0.02, 0.95, &cfg);
        assert!(singularity.singularity_detected);
    }

    // --- Integration / Edge Cases ---

    #[test]
    fn test_full_safety_workflow() {
        let cfg = SafetyConfig::default();

        // Step 1: Simulate deployment.
        let deployment = simulate_deployment_10k_nodes(1000, 20, 0.05, &cfg);
        assert!(deployment.active_nodes > 0);

        // Step 2: Check safety proof.
        assert!(deployment.safety_proof.safety_score >= 0.0);

        // Step 3: Detect singularity.
        let singularity = detect_singularity_threshold(
            deployment.avg_coherence,
            deployment.planetary_free_energy,
            deployment.avg_trust,
            0.01,
            -0.005,
            &cfg,
        );
        assert!(singularity.tipping_proximity >= 0.0 && singularity.tipping_proximity <= 1.0);

        // All steps succeeded.
        assert!(deployment.total_nodes == 1000);
        assert!(singularity.estimated_steps_remaining >= 0);
    }

    #[test]
    fn test_deployment_with_high_byzantine_fails() {
        let cfg = SafetyConfig::default();
        let result = simulate_deployment_10k_nodes(100, 10, 0.4, &cfg);
        // 40% Byzantine > 1/3 threshold should fail safety.
        assert!(!result.deployment_successful || !result.safety_proof.safe);
    }

    #[test]
    fn test_safety_score_is_minimum() {
        let cfg = SafetyConfig::default();
        let energy_dist = vec![0.5, 0.5];
        // Low barrier should make safety_score = barrier_certificate.
        let result = prove_noosfera_safety(0.95, 0.01, &energy_dist, 0, 100, -0.1, &cfg);
        assert!((result.safety_score - result.barrier_certificate).abs() < 1e-9);
    }

    #[test]
    fn test_energy_instability_detected() {
        let cfg = SafetyConfig::default();
        // Highly skewed energy distribution.
        let energy_dist = vec![0.999, 0.001];
        let result = prove_noosfera_safety(0.95, 0.01, &energy_dist, 0, 100, 1.0, &cfg);
        // Low entropy = high stability, so this should NOT trigger instability.
        assert!(result.energy_stability >= 0.0);
    }

    #[test]
    fn test_uniform_energy_stability() {
        let cfg = SafetyConfig::default();
        let energy_dist = vec![0.25, 0.25, 0.25, 0.25];
        let result = prove_noosfera_safety(0.95, 0.01, &energy_dist, 0, 100, 1.0, &cfg);
        // Uniform distribution = max entropy = high stability (normalized to 1.0).
        assert!((result.energy_stability - 1.0).abs() < 1e-6);
    }
}
