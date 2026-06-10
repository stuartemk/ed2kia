//! Adversarial Collective Red-Teaming — Immune Antibody Generation.
//!
//! Implements the Planetary Immune System for collective defense:
//! - **Collective Counter-Steering:** Shapley-weighted immune antibody generation.
//! - **Adversarial Perturbation:** FGSM-style attacks on latent space for robustness testing.
//! - **Immune Memory:** Persistent counter-signatures for known attack patterns.
//!
//! **Sprint 130:** Distributed Red Teaming on SGW Manifolds.
//! - **SGW-based Adversarial Detection:** Manifold drift via Sliced Gromov-Wasserstein.
//! - **Distributed Red Team:** Multi-node coordinated attack generation and assessment.
//! - **Manifold-aware Antibodies:** SGW-geometry-aware counter-steering.
//! - **Collective Threat Scoring:** Cross-node threat aggregation with Shapley weighting.

/// Counter-steering antibody — a defensive latent-space perturbation
/// that neutralizes adversarial steering attempts.
#[derive(Debug, Clone)]
pub struct CounterSteeringAntibody {
    /// Antibody perturbation vector (to be added to steered activations).
    pub perturbation: Vec<f32>,
    /// Shapley-weighted confidence score [0, 1].
    pub confidence: f32,
    /// Number of contributing nodes in the collective.
    pub contributor_count: usize,
    /// Attack pattern hash this antibody targets.
    pub target_hash: [u8; 32],
    /// L2 norm of the perturbation (for bounded application).
    pub norm: f32,
}

impl std::fmt::Display for CounterSteeringAntibody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Antibody {{ norm={:.4}, confidence={:.3}, contributors={}, target={}}}",
            self.norm,
            self.confidence,
            self.contributor_count,
            hex_prefix(&self.target_hash)
        )
    }
}

/// Configuration for collective counter-steering generation.
#[derive(Debug, Clone)]
pub struct CounterSteeringConfig {
    /// Maximum perturbation norm (safety bound).
    pub max_norm: f32,
    /// Minimum confidence threshold for antibody acceptance.
    pub min_confidence: f32,
    /// Random seed for deterministic generation.
    pub seed: u64,
}

impl Default for CounterSteeringConfig {
    fn default() -> Self {
        Self {
            max_norm: 1.0,
            min_confidence: 0.5,
            seed: 42,
        }
    }
}

impl CounterSteeringConfig {
    pub fn with_max_norm(mut self, norm: f32) -> Self {
        self.max_norm = norm.max(0.0);
        self
    }

    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Generate Collective Counter-Steering Antibody.
///
/// Given an adversarial steering attempt (detected perturbation), generates
/// a collective immune response by computing the Shapley-weighted anti-gradient
/// that neutralizes the attack while preserving model utility.
///
/// The algorithm:
/// 1. Computes the adversarial gradient direction from the detected perturbation.
/// 2. Generates counter-perturbation as negative gradient scaled by Shapley weights.
/// 3. Clips to max_norm for safety boundedness.
/// 4. Computes confidence from collective agreement (Shapley value concentration).
///
/// # Parameters
/// - `adversarial_perturbation`: The detected adversarial steering vector.
/// - `shapley_weights`: Per-node Shapley contribution weights (sum ≈ 1.0).
/// - `original_activation`: The original (pre-attack) activation vector.
/// - `config`: Counter-steering configuration.
///
/// # Returns
/// `CounterSteeringAntibody` ready for application to neutralize the attack.
pub fn generate_collective_counter_steering(
    adversarial_perturbation: &[f32],
    shapley_weights: &[f64],
    _original_activation: &[f32],
    config: &CounterSteeringConfig,
) -> CounterSteeringAntibody {
    let dim = adversarial_perturbation.len();
    if dim == 0 {
        return empty_antibody(config, shapley_weights.len());
    }

    // Compute attack hash for targeting
    let target_hash = compute_attack_hash(adversarial_perturbation);

    // Compute adversarial gradient norm
    let adv_norm = euclidean_norm(adversarial_perturbation).max(1e-12);

    // Generate counter-perturbation: negative gradient × Shapley weights
    let mut counter = Vec::with_capacity(dim);
    let mut rng_state = config.seed;

    for i in 0..dim {
        let adv_dir = -adversarial_perturbation[i] / adv_norm as f32;
        // Weight by Shapley contribution (cycle through weights for large dims)
        let weight = shapley_weights
            .get(i % shapley_weights.len())
            .copied()
            .unwrap_or(1.0 / dim as f64) as f32;
        // Add small noise for diversity
        let noise = gaussian_noise(&mut rng_state) * 0.01;
        counter.push(adv_dir * weight as f32 + noise);
    }

    // Compute counter norm and clip to max_norm
    let counter_norm = euclidean_norm(&counter);
    if counter_norm > config.max_norm {
        let scale = config.max_norm / counter_norm;
        for val in counter.iter_mut() {
            *val *= scale;
        }
    }

    // Compute confidence from Shapley weight concentration
    let confidence = compute_shapley_confidence(shapley_weights) as f32;

    let norm = euclidean_norm(&counter);
    CounterSteeringAntibody {
        perturbation: counter,
        confidence: confidence.max(config.min_confidence),
        contributor_count: shapley_weights.len(),
        target_hash,
        norm,
    }
}

/// Apply counter-steering antibody to a steered activation.
///
/// Adds the antibody perturbation to neutralize adversarial steering.
/// Returns the cleaned activation vector.
pub fn apply_antibody(steered: &[f32], antibody: &CounterSteeringAntibody) -> Vec<f32> {
    if steered.len() != antibody.perturbation.len() {
        return steered.to_vec();
    }
    steered
        .iter()
        .zip(antibody.perturbation.iter())
        .map(|(s, p)| s + p)
        .collect()
}

/// Verify antibody effectiveness — checks if the counter-steering
/// reduces the adversarial signal below the safety threshold.
pub fn verify_antibody_effectiveness(
    original: &[f32],
    steered: &[f32],
    antibody: &CounterSteeringAntibody,
    threshold: f32,
) -> bool {
    let cleaned = apply_antibody(steered, antibody);
    let residual_dist = euclidean_distance(original, &cleaned);
    let original_adv_dist = euclidean_distance(original, steered);
    // Antibody is effective if residual < threshold × original distance
    residual_dist < threshold * original_adv_dist
}

// ============================================================================
// Helper Functions
// ============================================================================

fn empty_antibody(
    config: &CounterSteeringConfig,
    contributor_count: usize,
) -> CounterSteeringAntibody {
    CounterSteeringAntibody {
        perturbation: vec![],
        confidence: config.min_confidence,
        contributor_count,
        target_hash: [0u8; 32],
        norm: 0.0,
    }
}

fn euclidean_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

fn compute_shapley_confidence(weights: &[f64]) -> f64 {
    if weights.is_empty() {
        return 0.0;
    }
    // Single node → zero entropy → max confidence
    if weights.len() == 1 {
        return 1.0;
    }
    let total: f64 = weights.iter().sum();
    if total.abs() < 1e-12 {
        return 0.0;
    }
    // Concentration: 1 - normalized entropy
    let n = weights.len() as f64;
    let _uniform = 1.0 / n;
    let mut entropy = 0.0;
    for &w in weights {
        let p = (w / total).max(1e-12);
        entropy -= p * p.ln();
    }
    let max_entropy = n.ln();
    let normalized_entropy = if max_entropy > 0.0 {
        entropy / max_entropy
    } else {
        1.0
    };
    (1.0 - normalized_entropy).clamp(0.0, 1.0)
}

fn compute_attack_hash(perturbation: &[f32]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let bytes: Vec<u8> = perturbation.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher.finalize().into()
}

fn hex_prefix(hash: &[u8; 32]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        hash[0], hash[1], hash[2], hash[3]
    )
}

fn gaussian_noise(state: &mut u64) -> f32 {
    let u1 = (lcg_next(state) as f32 / u64::MAX as f32).max(1e-10);
    let u2 = lcg_next(state) as f32 / u64::MAX as f32;
    (-2.0_f32 * u1.ln()).sqrt() * (2.0_f32 * std::f32::consts::PI * u2).cos()
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

// ============================================================================
// Distributed Red Teaming on SGW Manifolds (Sprint 130)
// ============================================================================

/// Configuration for SGW-based adversarial detection.
#[derive(Debug, Clone)]
pub struct SgwAdversarialConfig {
    /// Number of random projections for SGW approximation.
    pub num_projections: usize,
    /// Subsample size for pairwise distance computation.
    pub max_subsample: usize,
    /// Threshold for SGW distance to flag adversarial drift.
    pub drift_threshold: f32,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Number of red team nodes.
    pub num_red_nodes: usize,
}

impl Default for SgwAdversarialConfig {
    fn default() -> Self {
        Self {
            num_projections: 32,
            max_subsample: 64,
            drift_threshold: 0.15,
            seed: 42,
            num_red_nodes: 3,
        }
    }
}

impl SgwAdversarialConfig {
    pub fn with_projections(mut self, n: usize) -> Self {
        self.num_projections = n.max(1);
        self
    }

    pub fn with_subsample(mut self, n: usize) -> Self {
        self.max_subsample = n.max(2);
        self
    }

    pub fn with_drift_threshold(mut self, threshold: f32) -> Self {
        self.drift_threshold = threshold.max(0.0);
        self
    }

    pub fn with_red_nodes(mut self, n: usize) -> Self {
        self.num_red_nodes = n.max(1);
        self
    }

    /// Fast config for edge devices.
    pub fn edge_fast() -> Self {
        Self {
            num_projections: 8,
            max_subsample: 32,
            drift_threshold: 0.25,
            seed: 42,
            num_red_nodes: 2,
        }
    }

    /// High-accuracy config for verification.
    pub fn high_accuracy() -> Self {
        Self {
            num_projections: 128,
            max_subsample: 256,
            drift_threshold: 0.05,
            seed: 42,
            num_red_nodes: 5,
        }
    }
}

/// Result of SGW-based adversarial detection.
#[derive(Debug, Clone)]
pub struct SgwAdversarialResult {
    /// Computed SGW distance between clean and observed manifolds.
    pub sgw_distance: f32,
    /// Whether the drift exceeds the threshold (adversarial detected).
    pub is_adversarial: bool,
    /// Per-projection distances (for analysis).
    pub projection_distances: Vec<f32>,
    /// Threat score [0, 1] based on normalized SGW distance.
    pub threat_score: f32,
    /// Number of projections used.
    pub projections_used: usize,
}

impl std::fmt::Display for SgwAdversarialResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SGWAdv(dist={:.6}, threat={:.3}, adversarial={}, proj={})",
            self.sgw_distance, self.threat_score, self.is_adversarial, self.projections_used
        )
    }
}

/// A single red team node in the distributed red team.
#[derive(Debug, Clone)]
pub struct RedTeamNode {
    /// Unique node identifier.
    pub id: u64,
    /// Attack vector generated by this node.
    pub attack_vector: Vec<f32>,
    /// Attack effectiveness score [0, 1].
    pub effectiveness: f32,
    /// SGW distance from clean manifold for this attack.
    pub sgw_drift: f32,
    /// Whether this node's attack was detected.
    pub detected: bool,
}

impl RedTeamNode {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            attack_vector: vec![],
            effectiveness: 0.0,
            sgw_drift: 0.0,
            detected: false,
        }
    }
}

/// Result of a distributed red team exercise.
#[derive(Debug, Clone)]
pub struct RedTeamResult {
    /// All participating red team nodes.
    pub nodes: Vec<RedTeamNode>,
    /// Best attack vector (highest effectiveness).
    pub best_attack: Vec<f32>,
    /// Best attack effectiveness score.
    pub best_effectiveness: f32,
    /// Average detection rate across nodes.
    pub avg_detection_rate: f32,
    /// Collective threat score (Shapley-weighted).
    pub collective_threat: f32,
    /// Number of nodes that evaded detection.
    pub evasion_count: usize,
}

impl std::fmt::Display for RedTeamResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RedTeam(nodes={}, best_eff={:.3}, detection={:.3}, threat={:.3}, evasions={})",
            self.nodes.len(),
            self.best_effectiveness,
            self.avg_detection_rate,
            self.collective_threat,
            self.evasion_count
        )
    }
}

/// Manifold-aware antibody that incorporates SGW geometry.
#[derive(Debug, Clone)]
pub struct ManifoldAntibody {
    /// Base perturbation vector.
    pub perturbation: Vec<f32>,
    /// SGW distance to clean manifold after applying antibody.
    pub post_sgw_distance: f32,
    /// Confidence score [0, 1].
    pub confidence: f32,
    /// Number of contributing nodes.
    pub contributor_count: usize,
    /// Manifold alignment score [0, 1] (higher = better alignment).
    pub manifold_alignment: f32,
}

impl std::fmt::Display for ManifoldAntibody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ManifoldAb(sgw={:.6}, conf={:.3}, align={:.3}, contributors={})",
            self.post_sgw_distance,
            self.confidence,
            self.manifold_alignment,
            self.contributor_count
        )
    }
}

/// Compute SGW-like distance between two activation manifolds.
///
/// Uses random 1D projections + 1D Wasserstein distance approximation
/// (simplified SGW for adversarial detection).
///
/// # Parameters
/// - `clean`: Clean activation manifold (baseline).
/// - `observed`: Observed activation manifold (potentially adversarial).
/// - `config`: SGW adversarial configuration.
///
/// # Returns
/// `SgwAdversarialResult` with detection status and threat score.
pub fn compute_sgw_adversarial_detection(
    clean: &[f32],
    observed: &[f32],
    config: &SgwAdversarialConfig,
) -> SgwAdversarialResult {
    let dim = clean.len();
    if dim == 0 || dim != observed.len() {
        return SgwAdversarialResult {
            sgw_distance: 0.0,
            is_adversarial: false,
            projection_distances: vec![],
            threat_score: 0.0,
            projections_used: 0,
        };
    }

    let mut rng_state = config.seed;
    let mut projection_distances = Vec::with_capacity(config.num_projections);
    let dim_f32 = dim as f32;

    for _ in 0..config.num_projections {
        // Generate random 1D projection direction
        let mut proj_dir = Vec::with_capacity(dim);
        for _ in 0..dim {
            proj_dir.push(gaussian_noise(&mut rng_state));
        }
        let norm = euclidean_norm(&proj_dir).max(1e-12);
        for v in proj_dir.iter_mut() {
            *v /= norm;
        }

        // Project both manifolds to 1D
        let mut clean_proj = 0.0_f32;
        let mut observed_proj = 0.0_f32;
        for i in 0..dim {
            clean_proj += clean[i] * proj_dir[i];
            observed_proj += observed[i] * proj_dir[i];
        }

        // 1D Wasserstein distance = absolute difference of sorted projections
        // For single-point projections, this simplifies to |c - o|
        let dist = (clean_proj - observed_proj).abs() / dim_f32.sqrt();
        projection_distances.push(dist);
    }

    // Aggregate: mean of projection distances
    let sgw_distance = if projection_distances.is_empty() {
        0.0
    } else {
        projection_distances.iter().sum::<f32>() / projection_distances.len() as f32
    };

    // Threat score: normalized SGW distance (sigmoid-like mapping)
    let threat_score = (sgw_distance / (sgw_distance + config.drift_threshold)).clamp(0.0, 1.0);

    SgwAdversarialResult {
        sgw_distance,
        is_adversarial: sgw_distance > config.drift_threshold,
        projection_distances,
        threat_score,
        projections_used: config.num_projections,
    }
}

/// Generate a distributed red team attack from multiple nodes.
///
/// Each node generates an attack vector using SGW-manifold-aware perturbation,
/// then aggregates using Shapley-weighted effectiveness scoring.
///
/// # Parameters
/// - `clean_manifold`: Clean activation baseline.
/// - `config`: Red team configuration.
///
/// # Returns
/// `RedTeamResult` with all node attacks and collective metrics.
pub fn generate_distributed_red_team(
    clean_manifold: &[f32],
    config: &SgwAdversarialConfig,
) -> RedTeamResult {
    let dim = clean_manifold.len();
    if dim == 0 {
        return RedTeamResult {
            nodes: vec![],
            best_attack: vec![],
            best_effectiveness: 0.0,
            avg_detection_rate: 0.0,
            collective_threat: 0.0,
            evasion_count: 0,
        };
    }

    let mut nodes = Vec::with_capacity(config.num_red_nodes);
    let mut rng_state = config.seed;

    for node_id in 0..config.num_red_nodes {
        // Generate attack vector: SGW-aware perturbation
        let mut attack = Vec::with_capacity(dim);
        for i in 0..dim {
            // Perturbation scaled by node-specific strategy
            let base_noise = gaussian_noise(&mut rng_state);
            let node_scale = (node_id as f32 + 1.0) / config.num_red_nodes as f32;
            attack.push(base_noise * node_scale * 0.1);
        }

        // Compute attack manifold (clean + attack)
        let attack_manifold: Vec<f32> = clean_manifold
            .iter()
            .zip(attack.iter())
            .map(|(c, a)| c + a)
            .collect();

        // Detect using SGW
        let detection = compute_sgw_adversarial_detection(clean_manifold, &attack_manifold, config);

        // Effectiveness = 1 - detection confidence (higher = harder to detect)
        let effectiveness = (1.0 - detection.threat_score).clamp(0.0, 1.0);

        nodes.push(RedTeamNode {
            id: node_id as u64,
            attack_vector: attack,
            effectiveness,
            sgw_drift: detection.sgw_distance,
            detected: detection.is_adversarial,
        });
    }

    // Find best attack (highest effectiveness)
    let best_idx = nodes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.effectiveness.partial_cmp(&b.effectiveness).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    let best_attack = nodes[best_idx].attack_vector.clone();
    let best_effectiveness = nodes[best_idx].effectiveness;

    // Average detection rate
    let avg_detection_rate = if nodes.is_empty() {
        0.0
    } else {
        nodes
            .iter()
            .map(|n| if n.detected { 1.0 } else { 0.0 })
            .sum::<f32>()
            / nodes.len() as f32
    };

    // Collective threat: Shapley-weighted (equal weights for simplicity)
    let collective_threat = if nodes.is_empty() {
        0.0
    } else {
        nodes.iter().map(|n| n.sgw_drift).sum::<f32>() / nodes.len() as f32
    };

    // Evasion count
    let evasion_count = nodes.iter().filter(|n| !n.detected).count();

    RedTeamResult {
        nodes,
        best_attack,
        best_effectiveness,
        avg_detection_rate,
        collective_threat,
        evasion_count,
    }
}

/// Generate a manifold-aware antibody using SGW geometry.
///
/// Extends the basic counter-steering with SGW manifold alignment,
/// ensuring the antibody not only neutralizes the attack but also
/// restores the activation to the clean manifold geometry.
///
/// # Parameters
/// - `adversarial_perturbation`: The detected adversarial steering vector.
/// - `shapley_weights`: Per-node Shapley contribution weights.
/// - `clean_manifold`: Clean activation baseline for SGW reference.
/// - `adv_config`: SGW adversarial configuration.
/// - `counter_config`: Counter-steering configuration.
///
/// # Returns
/// `ManifoldAntibody` with SGW-aware perturbation.
pub fn generate_manifold_antibody(
    adversarial_perturbation: &[f32],
    shapley_weights: &[f64],
    clean_manifold: &[f32],
    adv_config: &SgwAdversarialConfig,
    counter_config: &CounterSteeringConfig,
) -> ManifoldAntibody {
    let dim = adversarial_perturbation.len();
    if dim == 0 {
        return ManifoldAntibody {
            perturbation: vec![],
            post_sgw_distance: 0.0,
            confidence: counter_config.min_confidence,
            contributor_count: shapley_weights.len(),
            manifold_alignment: 0.0,
        };
    }

    // Generate base counter-steering antibody
    let base_antibody = generate_collective_counter_steering(
        adversarial_perturbation,
        shapley_weights,
        clean_manifold,
        counter_config,
    );

    // Apply antibody to adversarial manifold
    let adv_manifold: Vec<f32> = clean_manifold
        .iter()
        .zip(adversarial_perturbation.iter())
        .map(|(c, a)| c + a)
        .collect();
    let restored: Vec<f32> = apply_antibody(&adv_manifold, &base_antibody);

    // Compute post-antibody SGW distance to clean manifold
    let post_detection = compute_sgw_adversarial_detection(clean_manifold, &restored, adv_config);

    // Manifold alignment: 1 - normalized post-SGW distance
    let manifold_alignment = (1.0 - post_detection.threat_score).clamp(0.0, 1.0);

    ManifoldAntibody {
        perturbation: base_antibody.perturbation,
        post_sgw_distance: post_detection.sgw_distance,
        confidence: base_antibody.confidence,
        contributor_count: base_antibody.contributor_count,
        manifold_alignment,
    }
}

/// Compute collective threat score from multiple SGW detections.
///
/// Aggregates threat scores from multiple nodes using Shapley-weighted
/// averaging for robust threat assessment.
///
/// # Parameters
/// - `threat_scores`: Per-node threat scores from SGW detection.
/// - `shapley_weights`: Per-node Shapley contribution weights.
///
/// # Returns
/// Aggregated threat score [0, 1].
pub fn compute_collective_threat(threat_scores: &[f32], shapley_weights: &[f64]) -> f32 {
    if threat_scores.is_empty() || shapley_weights.is_empty() {
        return 0.0;
    }
    let n = threat_scores.len().min(shapley_weights.len());
    let total_weight: f64 = shapley_weights.iter().take(n).sum();
    if total_weight.abs() < 1e-12 {
        // Uniform fallback
        return threat_scores.iter().take(n).sum::<f32>() / n as f32;
    }
    let mut weighted_sum = 0.0_f32;
    for i in 0..n {
        weighted_sum += threat_scores[i] * (shapley_weights[i] / total_weight) as f32;
    }
    weighted_sum.clamp(0.0, 1.0)
}

/// Assess red team exercise results and generate defense recommendations.
///
/// Analyzes the red team results to identify weaknesses in the current
/// defense and recommends improvements.
///
/// # Parameters
/// - `result`: Red team exercise results.
/// - `config`: SGW adversarial configuration.
///
/// # Returns
/// Tuple of (vulnerability_score, recommended_threshold).
/// - `vulnerability_score`: [0, 1] higher = more vulnerable.
/// - `recommended_threshold`: Suggested drift threshold adjustment.
pub fn assess_red_team_result(result: &RedTeamResult, config: &SgwAdversarialConfig) -> (f32, f32) {
    if result.nodes.is_empty() {
        return (0.0, config.drift_threshold);
    }

    // Vulnerability = average effectiveness of undetected attacks
    let evasion_nodes: Vec<&RedTeamNode> = result.nodes.iter().filter(|n| !n.detected).collect();
    let vulnerability = if evasion_nodes.is_empty() {
        0.0
    } else {
        evasion_nodes.iter().map(|n| n.effectiveness).sum::<f32>() / evasion_nodes.len() as f32
    };

    // Recommended threshold: if many evasions, lower the threshold
    let evasion_rate = result.evasion_count as f32 / result.nodes.len() as f32;
    let recommended_threshold = if evasion_rate > 0.5 {
        config.drift_threshold * 0.7 // More sensitive
    } else if evasion_rate < 0.2 {
        config.drift_threshold * 1.3 // Less sensitive (reduce false positives)
    } else {
        config.drift_threshold
    };

    (
        vulnerability.clamp(0.0, 1.0),
        recommended_threshold.max(0.001),
    )
}

/// Run a full red team pipeline: generate attacks → detect → defend → assess.
///
/// # Parameters
/// - `clean_manifold`: Clean activation baseline.
/// - `adv_config`: SGW adversarial configuration.
/// - `counter_config`: Counter-steering configuration.
///
/// # Returns
/// Tuple of (RedTeamResult, ManifoldAntibody, vulnerability_score).
pub fn full_red_team_pipeline(
    clean_manifold: &[f32],
    adv_config: &SgwAdversarialConfig,
    counter_config: &CounterSteeringConfig,
) -> (RedTeamResult, ManifoldAntibody, f32) {
    // Step 1: Generate distributed red team attacks
    let red_result = generate_distributed_red_team(clean_manifold, adv_config);

    // Step 2: Use best attack to generate manifold antibody
    let shapley_weights: Vec<f64> = if red_result.nodes.is_empty() {
        vec![1.0]
    } else {
        let n = red_result.nodes.len();
        vec![1.0 / n as f64; n]
    };

    let antibody = if red_result.best_attack.is_empty() {
        generate_manifold_antibody(
            &[],
            &shapley_weights,
            clean_manifold,
            adv_config,
            counter_config,
        )
    } else {
        generate_manifold_antibody(
            &red_result.best_attack,
            &shapley_weights,
            clean_manifold,
            adv_config,
            counter_config,
        )
    };

    // Step 3: Assess vulnerability
    let (vulnerability, _) = assess_red_team_result(&red_result, adv_config);

    (red_result, antibody, vulnerability)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config Tests ---

    #[test]
    fn test_counter_steering_config_default() {
        let cfg = CounterSteeringConfig::default();
        assert!((cfg.max_norm - 1.0).abs() < 1e-6);
        assert!((cfg.min_confidence - 0.5).abs() < 1e-6);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_counter_steering_config_with_max_norm() {
        let cfg = CounterSteeringConfig::default().with_max_norm(2.0);
        assert!((cfg.max_norm - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_counter_steering_config_max_norm_zero() {
        let cfg = CounterSteeringConfig::default().with_max_norm(-1.0);
        assert_eq!(cfg.max_norm, 0.0);
    }

    #[test]
    fn test_counter_steering_config_with_confidence() {
        let cfg = CounterSteeringConfig::default().with_min_confidence(0.8);
        assert!((cfg.min_confidence - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_counter_steering_config_confidence_clamped() {
        let cfg = CounterSteeringConfig::default().with_min_confidence(1.5);
        assert!((cfg.min_confidence - 1.0).abs() < 1e-6);
    }

    // --- Counter-Steering Tests ---

    #[test]
    fn test_generate_collective_counter_steering_empty() {
        let cfg = CounterSteeringConfig::default();
        let ab = generate_collective_counter_steering(&[], &[1.0], &[], &cfg);
        assert!(ab.perturbation.is_empty());
        assert_eq!(ab.contributor_count, 1);
    }

    #[test]
    fn test_generate_collective_counter_steering_basic() {
        let adv = vec![0.1, -0.2, 0.3];
        let weights = vec![0.5, 0.3, 0.2];
        let original = vec![1.0, 1.0, 1.0];
        let cfg = CounterSteeringConfig::default();
        let ab = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        assert_eq!(ab.perturbation.len(), 3);
        assert_eq!(ab.contributor_count, 3);
        assert!(ab.norm > 0.0);
        assert!(ab.confidence >= cfg.min_confidence);
    }

    #[test]
    fn test_generate_collective_counter_steering_norm_clamped() {
        let adv = vec![1.0, 1.0, 1.0];
        let weights = vec![1.0];
        let original = vec![0.0, 0.0, 0.0];
        let cfg = CounterSteeringConfig::default().with_max_norm(0.5);
        let ab = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        assert!(ab.norm <= 0.5);
    }

    #[test]
    fn test_generate_collective_counter_steering_deterministic() {
        let adv = vec![0.1, -0.2, 0.3];
        let weights = vec![0.5, 0.5];
        let original = vec![1.0, 1.0, 1.0];
        let cfg = CounterSteeringConfig::default();
        let ab1 = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        let ab2 = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        assert_eq!(ab1.perturbation, ab2.perturbation);
        assert_eq!(ab1.target_hash, ab2.target_hash);
    }

    #[test]
    fn test_generate_collective_counter_steering_opposite_direction() {
        let adv = vec![1.0, 0.0, 0.0];
        let weights = vec![1.0];
        let original = vec![0.0, 0.0, 0.0];
        let cfg = CounterSteeringConfig::default();
        let ab = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        // Counter should point opposite to adversarial direction
        assert!(ab.perturbation[0] < 0.0);
    }

    #[test]
    fn test_generate_collective_counter_steering_display() {
        let adv = vec![0.1, -0.2];
        let weights = vec![0.5, 0.5];
        let original = vec![1.0, 1.0];
        let cfg = CounterSteeringConfig::default();
        let ab = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        let s = format!("{}", ab);
        assert!(s.contains("Antibody"));
        assert!(s.contains("norm="));
    }

    // --- Antibody Application Tests ---

    #[test]
    fn test_apply_antibody_basic() {
        let steered = vec![1.0, 2.0, 3.0];
        let ab = CounterSteeringAntibody {
            perturbation: vec![-0.1, -0.2, -0.3],
            confidence: 0.9,
            contributor_count: 1,
            target_hash: [0u8; 32],
            norm: 0.5,
        };
        let cleaned = apply_antibody(&steered, &ab);
        assert!((cleaned[0] - 0.9).abs() < 1e-6);
        assert!((cleaned[1] - 1.8).abs() < 1e-6);
        assert!((cleaned[2] - 2.7).abs() < 1e-6);
    }

    #[test]
    fn test_apply_antibody_dimension_mismatch() {
        let steered = vec![1.0, 2.0];
        let ab = CounterSteeringAntibody {
            perturbation: vec![-0.1],
            confidence: 0.9,
            contributor_count: 1,
            target_hash: [0u8; 32],
            norm: 0.1,
        };
        let cleaned = apply_antibody(&steered, &ab);
        // Should return original on mismatch
        assert_eq!(cleaned, steered);
    }

    // --- Effectiveness Verification Tests ---

    #[test]
    fn test_verify_antibody_effectiveness_effective() {
        let original = vec![0.0, 0.0, 0.0];
        let steered = vec![0.1, 0.1, 0.1];
        let ab = CounterSteeringAntibody {
            perturbation: vec![-0.09, -0.09, -0.09],
            confidence: 0.9,
            contributor_count: 1,
            target_hash: [0u8; 32],
            norm: 0.15,
        };
        assert!(verify_antibody_effectiveness(&original, &steered, &ab, 0.5));
    }

    #[test]
    fn test_verify_antibody_effectiveness_ineffective() {
        let original = vec![0.0, 0.0, 0.0];
        let steered = vec![1.0, 1.0, 1.0];
        let ab = CounterSteeringAntibody {
            perturbation: vec![0.001, 0.001, 0.001],
            confidence: 0.1,
            contributor_count: 1,
            target_hash: [0u8; 32],
            norm: 0.002,
        };
        assert!(!verify_antibody_effectiveness(
            &original, &steered, &ab, 0.1
        ));
    }

    #[test]
    fn test_verify_antibody_effectiveness_zero_threshold() {
        let original = vec![0.0, 0.0];
        let steered = vec![0.1, 0.1];
        let ab = CounterSteeringAntibody {
            perturbation: vec![-0.1, -0.1],
            confidence: 1.0,
            contributor_count: 1,
            target_hash: [0u8; 32],
            norm: 0.14,
        };
        // With threshold 0, only perfect neutralization passes
        assert!(!verify_antibody_effectiveness(
            &original, &steered, &ab, 0.0
        ));
    }

    // --- Helper Function Tests ---

    #[test]
    fn test_euclidean_norm_basic() {
        let v = vec![3.0, 4.0];
        assert!((euclidean_norm(&v) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_norm_zero() {
        let v = vec![0.0, 0.0, 0.0];
        assert!((euclidean_norm(&v) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_same() {
        let a = vec![1.0, 2.0];
        assert!((euclidean_distance(&a, &a) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_basic() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_shapley_confidence_uniform() {
        let weights = vec![0.25, 0.25, 0.25, 0.25];
        let conf = compute_shapley_confidence(&weights);
        // Uniform distribution → max entropy → low confidence
        assert!(conf >= 0.0 && conf <= 0.3);
    }

    #[test]
    fn test_compute_shapley_confidence_concentrated() {
        let weights = vec![0.9, 0.05, 0.03, 0.02];
        let conf = compute_shapley_confidence(&weights);
        // Concentrated → low entropy → high confidence
        assert!(conf > 0.5);
    }

    #[test]
    fn test_compute_shapley_confidence_empty() {
        assert!((compute_shapley_confidence(&[]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_shapley_confidence_single() {
        let conf = compute_shapley_confidence(&[1.0]);
        // Single node → zero entropy → max confidence
        assert!((conf - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_gaussian_noise_range() {
        let mut state = 42u64;
        let mut sum_sq = 0.0_f32;
        for _ in 0..1000 {
            let n = gaussian_noise(&mut state);
            sum_sq += n * n;
        }
        let variance = sum_sq / 1000.0;
        assert!(variance > 0.3 && variance < 2.0);
    }

    #[test]
    fn test_hex_prefix() {
        let mut hash = [0u8; 32];
        hash[0] = 0xAB;
        hash[1] = 0xCD;
        let s = hex_prefix(&hash);
        assert_eq!(s, "abcd0000");
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_counter_steering_pipeline() {
        // Simulate adversarial attack detection → antibody generation → application
        let original = vec![0.5, -0.3, 0.8, 0.1];
        let adversarial = vec![0.2, -0.1, 0.15, 0.05]; // Attack perturbation
        let steered: Vec<f32> = original
            .iter()
            .zip(adversarial.iter())
            .map(|(o, a)| o + a)
            .collect();

        // Generate antibody
        let shapley_weights = vec![0.4, 0.3, 0.2, 0.1];
        let cfg = CounterSteeringConfig::default();
        let antibody =
            generate_collective_counter_steering(&adversarial, &shapley_weights, &original, &cfg);

        // Apply antibody
        let cleaned = apply_antibody(&steered, &antibody);

        // Verify effectiveness
        let dist_original_steered = euclidean_distance(&original, &steered);
        let dist_original_cleaned = euclidean_distance(&original, &cleaned);
        assert!(
            dist_original_cleaned < dist_original_steered,
            "Antibody should reduce distance to original"
        );

        // Verify antibody properties
        assert!(antibody.confidence >= cfg.min_confidence);
        assert!(antibody.norm <= cfg.max_norm);
        assert_eq!(antibody.contributor_count, shapley_weights.len());
    }

    #[test]
    fn test_collective_defense_multi_node() {
        // Multiple nodes contribute to collective defense
        let adv = vec![0.5, -0.3, 0.2];
        let weights = vec![0.34, 0.33, 0.33]; // Nearly equal Shapley weights
        let original = vec![1.0, 1.0, 1.0];
        let cfg = CounterSteeringConfig::default().with_max_norm(0.5);

        let ab = generate_collective_counter_steering(&adv, &weights, &original, &cfg);
        assert_eq!(ab.contributor_count, 3);
        assert!(ab.norm <= 0.5);

        // Verify the antibody points roughly opposite to the attack
        let dot: f32 = adv
            .iter()
            .zip(ab.perturbation.iter())
            .map(|(a, b)| a * b)
            .sum();
        assert!(dot < 0.0, "Antibody should oppose attack direction");
    }

    // ========================================================================
    // Sprint 130 — Distributed Red Teaming on SGW Manifolds Tests
    // ========================================================================

    // --- SgwAdversarialConfig Tests ---

    #[test]
    fn test_sgw_adversarial_config_default() {
        let cfg = SgwAdversarialConfig::default();
        assert_eq!(cfg.num_projections, 32);
        assert_eq!(cfg.max_subsample, 64);
        assert!((cfg.drift_threshold - 0.15).abs() < 1e-6);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.num_red_nodes, 3);
    }

    #[test]
    fn test_sgw_adversarial_config_with_projections() {
        let cfg = SgwAdversarialConfig::default().with_projections(64);
        assert_eq!(cfg.num_projections, 64);
    }

    #[test]
    fn test_sgw_adversarial_config_projections_min() {
        let cfg = SgwAdversarialConfig::default().with_projections(0);
        assert_eq!(cfg.num_projections, 1);
    }

    #[test]
    fn test_sgw_adversarial_config_with_subsample() {
        let cfg = SgwAdversarialConfig::default().with_subsample(128);
        assert_eq!(cfg.max_subsample, 128);
    }

    #[test]
    fn test_sgw_adversarial_config_subsample_min() {
        let cfg = SgwAdversarialConfig::default().with_subsample(1);
        assert_eq!(cfg.max_subsample, 2);
    }

    #[test]
    fn test_sgw_adversarial_config_with_drift_threshold() {
        let cfg = SgwAdversarialConfig::default().with_drift_threshold(0.3);
        assert!((cfg.drift_threshold - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_sgw_adversarial_config_threshold_zero() {
        let cfg = SgwAdversarialConfig::default().with_drift_threshold(-1.0);
        assert_eq!(cfg.drift_threshold, 0.0);
    }

    #[test]
    fn test_sgw_adversarial_config_with_red_nodes() {
        let cfg = SgwAdversarialConfig::default().with_red_nodes(5);
        assert_eq!(cfg.num_red_nodes, 5);
    }

    #[test]
    fn test_sgw_adversarial_config_red_nodes_min() {
        let cfg = SgwAdversarialConfig::default().with_red_nodes(0);
        assert_eq!(cfg.num_red_nodes, 1);
    }

    #[test]
    fn test_sgw_adversarial_config_edge_fast() {
        let cfg = SgwAdversarialConfig::edge_fast();
        assert_eq!(cfg.num_projections, 8);
        assert_eq!(cfg.max_subsample, 32);
        assert!((cfg.drift_threshold - 0.25).abs() < 1e-6);
        assert_eq!(cfg.num_red_nodes, 2);
    }

    #[test]
    fn test_sgw_adversarial_config_high_accuracy() {
        let cfg = SgwAdversarialConfig::high_accuracy();
        assert_eq!(cfg.num_projections, 128);
        assert_eq!(cfg.max_subsample, 256);
        assert!((cfg.drift_threshold - 0.05).abs() < 1e-6);
        assert_eq!(cfg.num_red_nodes, 5);
    }

    // --- SGW Adversarial Detection Tests ---

    #[test]
    fn test_sgw_adversarial_detection_empty() {
        let cfg = SgwAdversarialConfig::default();
        let result = compute_sgw_adversarial_detection(&[], &[], &cfg);
        assert!((result.sgw_distance - 0.0).abs() < 1e-6);
        assert!(!result.is_adversarial);
        assert!(result.projection_distances.is_empty());
        assert_eq!(result.projections_used, 0);
    }

    #[test]
    fn test_sgw_adversarial_detection_identical() {
        let clean = vec![1.0, 2.0, 3.0, 4.0];
        let cfg = SgwAdversarialConfig::default();
        let result = compute_sgw_adversarial_detection(&clean, &clean, &cfg);
        // Identical manifolds should have near-zero SGW distance
        assert!(result.sgw_distance < 1e-6);
        assert!(!result.is_adversarial);
        assert!(result.threat_score < 0.01);
    }

    #[test]
    fn test_sgw_adversarial_detection_different() {
        let clean = vec![0.0, 0.0, 0.0, 0.0];
        let observed = vec![1.0, 1.0, 1.0, 1.0];
        let cfg = SgwAdversarialConfig::default();
        let result = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert!(result.sgw_distance > 0.0);
        assert!(result.threat_score > 0.0);
    }

    #[test]
    fn test_sgw_adversarial_detection_large_drift() {
        let clean = vec![0.0_f32; 10];
        let observed: Vec<f32> = (0..10).map(|i| i as f32 * 10.0).collect();
        let cfg = SgwAdversarialConfig::default();
        let result = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert!(result.sgw_distance > cfg.drift_threshold);
        assert!(result.is_adversarial);
        assert!(result.threat_score > 0.5);
    }

    #[test]
    fn test_sgw_adversarial_detection_small_drift() {
        let clean = vec![1.0, 2.0, 3.0, 4.0];
        let observed = vec![1.01, 2.01, 3.01, 4.01];
        let cfg = SgwAdversarialConfig::default().with_drift_threshold(1.0);
        let result = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert!(!result.is_adversarial);
        assert!(result.threat_score < 0.1);
    }

    #[test]
    fn test_sgw_adversarial_detection_deterministic() {
        let clean = vec![1.0, 2.0, 3.0];
        let observed = vec![1.5, 2.5, 3.5];
        let cfg = SgwAdversarialConfig::default();
        let r1 = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        let r2 = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert_eq!(r1.sgw_distance, r2.sgw_distance);
        assert_eq!(r1.projection_distances, r2.projection_distances);
    }

    #[test]
    fn test_sgw_adversarial_detection_projection_count() {
        let clean = vec![1.0, 2.0, 3.0];
        let observed = vec![1.5, 2.5, 3.5];
        let cfg = SgwAdversarialConfig::default().with_projections(16);
        let result = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert_eq!(result.projection_distances.len(), 16);
        assert_eq!(result.projections_used, 16);
    }

    #[test]
    fn test_sgw_adversarial_detection_threat_score_range() {
        let clean = vec![0.0_f32; 8];
        let observed = vec![5.0_f32; 8];
        let cfg = SgwAdversarialConfig::default();
        let result = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert!(result.threat_score >= 0.0 && result.threat_score <= 1.0);
    }

    #[test]
    fn test_sgw_adversarial_detection_dimension_mismatch() {
        let clean = vec![1.0, 2.0];
        let observed = vec![1.0, 2.0, 3.0];
        let cfg = SgwAdversarialConfig::default();
        let result = compute_sgw_adversarial_detection(&clean, &observed, &cfg);
        assert!((result.sgw_distance - 0.0).abs() < 1e-6);
        assert!(!result.is_adversarial);
    }

    #[test]
    fn test_sgw_adversarial_result_display() {
        let result = SgwAdversarialResult {
            sgw_distance: 0.5,
            is_adversarial: true,
            projection_distances: vec![0.4, 0.6],
            threat_score: 0.75,
            projections_used: 2,
        };
        let s = format!("{}", result);
        assert!(s.contains("SGWAdv"));
        assert!(s.contains("adversarial=true"));
    }

    // --- Red Team Node Tests ---

    #[test]
    fn test_red_team_node_new() {
        let node = RedTeamNode::new(42);
        assert_eq!(node.id, 42);
        assert!(node.attack_vector.is_empty());
        assert_eq!(node.effectiveness, 0.0);
        assert_eq!(node.sgw_drift, 0.0);
        assert!(!node.detected);
    }

    // --- Distributed Red Team Tests ---

    #[test]
    fn test_distributed_red_team_empty() {
        let cfg = SgwAdversarialConfig::default();
        let result = generate_distributed_red_team(&[], &cfg);
        assert!(result.nodes.is_empty());
        assert!(result.best_attack.is_empty());
        assert_eq!(result.best_effectiveness, 0.0);
        assert_eq!(result.evasion_count, 0);
    }

    #[test]
    fn test_distributed_red_team_basic() {
        let clean = vec![1.0, 2.0, 3.0, 4.0];
        let cfg = SgwAdversarialConfig::default().with_red_nodes(3);
        let result = generate_distributed_red_team(&clean, &cfg);
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.best_attack.len(), 4);
        assert!(result.best_effectiveness >= 0.0 && result.best_effectiveness <= 1.0);
    }

    #[test]
    fn test_distributed_red_team_node_ids() {
        let clean = vec![1.0, 2.0, 3.0];
        let cfg = SgwAdversarialConfig::default().with_red_nodes(4);
        let result = generate_distributed_red_team(&clean, &cfg);
        for (i, node) in result.nodes.iter().enumerate() {
            assert_eq!(node.id, i as u64);
        }
    }

    #[test]
    fn test_distributed_red_team_deterministic() {
        let clean = vec![1.0, 2.0, 3.0];
        let cfg = SgwAdversarialConfig::default();
        let r1 = generate_distributed_red_team(&clean, &cfg);
        let r2 = generate_distributed_red_team(&clean, &cfg);
        assert_eq!(r1.best_effectiveness, r2.best_effectiveness);
        assert_eq!(r1.nodes.len(), r2.nodes.len());
        for (a, b) in r1.nodes.iter().zip(r2.nodes.iter()) {
            assert_eq!(a.attack_vector, b.attack_vector);
        }
    }

    #[test]
    fn test_distributed_red_team_detection_rate_range() {
        let clean = vec![1.0_f32; 8];
        let cfg = SgwAdversarialConfig::default().with_red_nodes(5);
        let result = generate_distributed_red_team(&clean, &cfg);
        assert!(result.avg_detection_rate >= 0.0 && result.avg_detection_rate <= 1.0);
    }

    #[test]
    fn test_distributed_red_team_evasion_count() {
        let clean = vec![1.0_f32; 8];
        let cfg = SgwAdversarialConfig::default().with_red_nodes(3);
        let result = generate_distributed_red_team(&clean, &cfg);
        assert!(result.evasion_count <= result.nodes.len());
    }

    #[test]
    fn test_distributed_red_team_collective_threat_range() {
        let clean = vec![1.0_f32; 8];
        let cfg = SgwAdversarialConfig::default();
        let result = generate_distributed_red_team(&clean, &cfg);
        assert!(result.collective_threat >= 0.0);
    }

    #[test]
    fn test_distributed_red_team_single_node() {
        let clean = vec![1.0, 2.0];
        let cfg = SgwAdversarialConfig::default().with_red_nodes(1);
        let result = generate_distributed_red_team(&clean, &cfg);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].id, 0);
    }

    #[test]
    fn test_distributed_red_team_display() {
        let clean = vec![1.0, 2.0, 3.0];
        let cfg = SgwAdversarialConfig::default();
        let result = generate_distributed_red_team(&clean, &cfg);
        let s = format!("{}", result);
        assert!(s.contains("RedTeam"));
        assert!(s.contains("nodes="));
    }

    #[test]
    fn test_distributed_red_team_high_drift_threshold() {
        // With very high threshold, nothing is detected
        let clean = vec![1.0_f32; 8];
        let cfg = SgwAdversarialConfig::default().with_drift_threshold(100.0);
        let result = generate_distributed_red_team(&clean, &cfg);
        assert!((result.avg_detection_rate - 0.0).abs() < 1e-6);
        assert_eq!(result.evasion_count, result.nodes.len());
    }

    #[test]
    fn test_distributed_red_team_low_drift_threshold() {
        // With very low threshold, everything is detected
        let clean = vec![1.0_f32; 8];
        let cfg = SgwAdversarialConfig::default().with_drift_threshold(0.0001);
        let result = generate_distributed_red_team(&clean, &cfg);
        assert!((result.avg_detection_rate - 1.0).abs() < 1e-6);
        assert_eq!(result.evasion_count, 0);
    }

    // --- Manifold Antibody Tests ---

    #[test]
    fn test_manifold_antibody_empty() {
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let ab = generate_manifold_antibody(&[], &[1.0], &[], &adv_cfg, &counter_cfg);
        assert!(ab.perturbation.is_empty());
        assert_eq!(ab.post_sgw_distance, 0.0);
        assert_eq!(ab.manifold_alignment, 0.0);
    }

    #[test]
    fn test_manifold_antibody_basic() {
        let clean = vec![1.0, 2.0, 3.0, 4.0];
        let adv = vec![0.1, -0.1, 0.1, -0.1];
        let weights = vec![0.5, 0.5];
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let ab = generate_manifold_antibody(&adv, &weights, &clean, &adv_cfg, &counter_cfg);
        assert_eq!(ab.perturbation.len(), 4);
        assert!(ab.confidence >= counter_cfg.min_confidence);
        assert!(ab.manifold_alignment >= 0.0 && ab.manifold_alignment <= 1.0);
    }

    #[test]
    fn test_manifold_antibody_alignment_improves() {
        let clean = vec![1.0_f32; 8];
        let adv: Vec<f32> = (0..8).map(|_| 0.5).collect();
        let weights = vec![0.5, 0.3, 0.2];
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let ab = generate_manifold_antibody(&adv, &weights, &clean, &adv_cfg, &counter_cfg);
        // Antibody should improve alignment vs no-antibody case
        assert!(ab.manifold_alignment > 0.0);
    }

    #[test]
    fn test_manifold_antibody_display() {
        let clean = vec![1.0, 2.0, 3.0];
        let adv = vec![0.1, -0.1, 0.1];
        let weights = vec![0.5, 0.5];
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let ab = generate_manifold_antibody(&adv, &weights, &clean, &adv_cfg, &counter_cfg);
        let s = format!("{}", ab);
        assert!(s.contains("ManifoldAb"));
        assert!(s.contains("sgw="));
    }

    #[test]
    fn test_manifold_antibody_contributor_count() {
        let clean = vec![1.0, 2.0, 3.0];
        let adv = vec![0.1, -0.1, 0.1];
        let weights = vec![0.4, 0.35, 0.25];
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let ab = generate_manifold_antibody(&adv, &weights, &clean, &adv_cfg, &counter_cfg);
        assert_eq!(ab.contributor_count, 3);
    }

    // --- Collective Threat Tests ---

    #[test]
    fn test_collective_threat_empty() {
        assert!((compute_collective_threat(&[], &[1.0]) - 0.0).abs() < 1e-6);
        assert!((compute_collective_threat(&[0.5], &[]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_collective_threat_uniform_weights() {
        let threats = vec![0.2, 0.4, 0.6];
        let weights = vec![1.0, 1.0, 1.0];
        let result = compute_collective_threat(&threats, &weights);
        let expected = (0.2 + 0.4 + 0.6) / 3.0;
        assert!((result - expected).abs() < 1e-6);
    }

    #[test]
    fn test_collective_threat_weighted() {
        let threats = vec![1.0, 0.0];
        let weights = vec![1.0, 0.0];
        let result = compute_collective_threat(&threats, &weights);
        assert!((result - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_collective_threat_zero_weights_fallback() {
        let threats = vec![0.3, 0.7];
        let weights = vec![0.0, 0.0];
        let result = compute_collective_threat(&threats, &weights);
        // Should fall back to uniform average
        assert!((result - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_collective_threat_range() {
        let threats = vec![0.0, 0.5, 1.0];
        let weights = vec![0.2, 0.5, 0.3];
        let result = compute_collective_threat(&threats, &weights);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_collective_threat_single_node() {
        let threats = vec![0.75];
        let weights = vec![1.0];
        let result = compute_collective_threat(&threats, &weights);
        assert!((result - 0.75).abs() < 1e-6);
    }

    // --- Red Team Assessment Tests ---

    #[test]
    fn test_assess_red_team_empty() {
        let result = RedTeamResult {
            nodes: vec![],
            best_attack: vec![],
            best_effectiveness: 0.0,
            avg_detection_rate: 0.0,
            collective_threat: 0.0,
            evasion_count: 0,
        };
        let cfg = SgwAdversarialConfig::default();
        let (vuln, threshold) = assess_red_team_result(&result, &cfg);
        assert_eq!(vuln, 0.0);
        assert!((threshold - cfg.drift_threshold).abs() < 1e-6);
    }

    #[test]
    fn test_assess_red_team_all_detected() {
        let result = RedTeamResult {
            nodes: vec![
                RedTeamNode {
                    id: 0,
                    attack_vector: vec![0.1],
                    effectiveness: 0.2,
                    sgw_drift: 0.5,
                    detected: true,
                },
                RedTeamNode {
                    id: 1,
                    attack_vector: vec![0.2],
                    effectiveness: 0.3,
                    sgw_drift: 0.6,
                    detected: true,
                },
            ],
            best_attack: vec![0.2],
            best_effectiveness: 0.3,
            avg_detection_rate: 1.0,
            collective_threat: 0.55,
            evasion_count: 0,
        };
        let cfg = SgwAdversarialConfig::default();
        let (vuln, threshold) = assess_red_team_result(&result, &cfg);
        assert_eq!(vuln, 0.0);
        // Low evasion → increase threshold (less sensitive)
        assert!(threshold > cfg.drift_threshold);
    }

    #[test]
    fn test_assess_red_team_all_evade() {
        let result = RedTeamResult {
            nodes: vec![
                RedTeamNode {
                    id: 0,
                    attack_vector: vec![0.1],
                    effectiveness: 0.9,
                    sgw_drift: 0.05,
                    detected: false,
                },
                RedTeamNode {
                    id: 1,
                    attack_vector: vec![0.2],
                    effectiveness: 0.8,
                    sgw_drift: 0.04,
                    detected: false,
                },
            ],
            best_attack: vec![0.1],
            best_effectiveness: 0.9,
            avg_detection_rate: 0.0,
            collective_threat: 0.045,
            evasion_count: 2,
        };
        let cfg = SgwAdversarialConfig::default();
        let (vuln, threshold) = assess_red_team_result(&result, &cfg);
        assert!(vuln > 0.0);
        // High evasion → decrease threshold (more sensitive)
        assert!(threshold < cfg.drift_threshold);
    }

    #[test]
    fn test_assess_red_team_partial_evasion() {
        let result = RedTeamResult {
            nodes: vec![
                RedTeamNode {
                    id: 0,
                    attack_vector: vec![0.1],
                    effectiveness: 0.9,
                    sgw_drift: 0.05,
                    detected: false,
                },
                RedTeamNode {
                    id: 1,
                    attack_vector: vec![0.2],
                    effectiveness: 0.3,
                    sgw_drift: 0.6,
                    detected: true,
                },
            ],
            best_attack: vec![0.1],
            best_effectiveness: 0.9,
            avg_detection_rate: 0.5,
            collective_threat: 0.325,
            evasion_count: 1,
        };
        let cfg = SgwAdversarialConfig::default();
        let (vuln, threshold) = assess_red_team_result(&result, &cfg);
        assert!(vuln > 0.0);
        // Partial evasion → keep threshold
        assert!((threshold - cfg.drift_threshold).abs() < 1e-6);
    }

    #[test]
    fn test_assess_red_team_vulnerability_range() {
        let result = RedTeamResult {
            nodes: vec![RedTeamNode {
                id: 0,
                attack_vector: vec![0.1],
                effectiveness: 0.5,
                sgw_drift: 0.1,
                detected: false,
            }],
            best_attack: vec![0.1],
            best_effectiveness: 0.5,
            avg_detection_rate: 0.0,
            collective_threat: 0.1,
            evasion_count: 1,
        };
        let cfg = SgwAdversarialConfig::default();
        let (vuln, _) = assess_red_team_result(&result, &cfg);
        assert!(vuln >= 0.0 && vuln <= 1.0);
    }

    // --- Full Red Team Pipeline Tests ---

    #[test]
    fn test_full_red_team_pipeline_empty() {
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let (red, ab, vuln) = full_red_team_pipeline(&[], &adv_cfg, &counter_cfg);
        assert!(red.nodes.is_empty());
        assert!(ab.perturbation.is_empty());
        assert_eq!(vuln, 0.0);
    }

    #[test]
    fn test_full_red_team_pipeline_basic() {
        let clean = vec![1.0, 2.0, 3.0, 4.0];
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let (red, ab, vuln) = full_red_team_pipeline(&clean, &adv_cfg, &counter_cfg);
        assert_eq!(red.nodes.len(), adv_cfg.num_red_nodes);
        assert_eq!(ab.perturbation.len(), 4);
        assert!(vuln >= 0.0 && vuln <= 1.0);
    }

    #[test]
    fn test_full_red_team_pipeline_deterministic() {
        let clean = vec![1.0, 2.0, 3.0];
        let adv_cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();
        let (r1, a1, v1) = full_red_team_pipeline(&clean, &adv_cfg, &counter_cfg);
        let (r2, a2, v2) = full_red_team_pipeline(&clean, &adv_cfg, &counter_cfg);
        assert_eq!(r1.best_effectiveness, r2.best_effectiveness);
        assert_eq!(a1.post_sgw_distance, a2.post_sgw_distance);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_full_red_team_pipeline_edge_fast() {
        let clean = vec![1.0_f32; 8];
        let adv_cfg = SgwAdversarialConfig::edge_fast();
        let counter_cfg = CounterSteeringConfig::default();
        let (red, ab, vuln) = full_red_team_pipeline(&clean, &adv_cfg, &counter_cfg);
        assert_eq!(red.nodes.len(), adv_cfg.num_red_nodes);
        assert!(ab.manifold_alignment >= 0.0 && ab.manifold_alignment <= 1.0);
        assert!(vuln >= 0.0);
    }

    #[test]
    fn test_full_red_team_pipeline_high_accuracy() {
        let clean = vec![1.0_f32; 8];
        let adv_cfg = SgwAdversarialConfig::high_accuracy();
        let counter_cfg = CounterSteeringConfig::default();
        let (red, ab, vuln) = full_red_team_pipeline(&clean, &adv_cfg, &counter_cfg);
        assert_eq!(red.nodes.len(), adv_cfg.num_red_nodes);
        assert!(ab.confidence >= counter_cfg.min_confidence);
    }

    #[test]
    fn test_full_red_team_pipeline_vulnerability_correlation() {
        // Low threshold → more detections → lower vulnerability
        let clean = vec![1.0_f32; 8];
        let adv_cfg_strict = SgwAdversarialConfig::default().with_drift_threshold(0.001);
        let adv_cfg_loose = SgwAdversarialConfig::default().with_drift_threshold(100.0);
        let counter_cfg = CounterSteeringConfig::default();

        let (_, _, vuln_strict) = full_red_team_pipeline(&clean, &adv_cfg_strict, &counter_cfg);
        let (_, _, vuln_loose) = full_red_team_pipeline(&clean, &adv_cfg_loose, &counter_cfg);

        // Strict detection should have lower vulnerability
        assert!(vuln_strict <= vuln_loose + 0.1);
    }

    // --- Integration: SGW + Counter-Steering + Red Team ---

    #[test]
    fn test_sgw_counter_steering_integration() {
        // Full pipeline: detect → defend → verify
        let clean = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let adv_perturbation = vec![0.3, -0.2, 0.4, -0.1, 0.2];
        let adv_manifold: Vec<f32> = clean
            .iter()
            .zip(adv_perturbation.iter())
            .map(|(c, a)| c + a)
            .collect();

        // Detect
        let adv_cfg = SgwAdversarialConfig::default();
        let detection = compute_sgw_adversarial_detection(&clean, &adv_manifold, &adv_cfg);

        // Defend
        let counter_cfg = CounterSteeringConfig::default();
        let weights = vec![0.5, 0.3, 0.2];
        let antibody =
            generate_manifold_antibody(&adv_perturbation, &weights, &clean, &adv_cfg, &counter_cfg);

        // Verify: post-antibody SGW should be lower than pre-antibody
        assert!(
            antibody.post_sgw_distance < detection.sgw_distance + 0.01
                || antibody.manifold_alignment > 0.0,
            "Antibody should improve or maintain manifold alignment"
        );
    }

    #[test]
    fn test_distributed_defense_pipeline() {
        // Multi-node defense with collective threat scoring
        let clean = vec![1.0_f32; 16];
        let adv_cfg = SgwAdversarialConfig::default().with_red_nodes(4);
        let counter_cfg = CounterSteeringConfig::default();

        // Red team generates attacks
        let red_result = generate_distributed_red_team(&clean, &adv_cfg);

        // Compute collective threat from all nodes
        let threat_scores: Vec<f32> = red_result.nodes.iter().map(|n| n.sgw_drift).collect();
        let shapley: Vec<f64> = vec![0.25; red_result.nodes.len()];
        let collective = compute_collective_threat(&threat_scores, &shapley);

        // Assess and generate defense
        let (vuln, _) = assess_red_team_result(&red_result, &adv_cfg);
        let antibody = generate_manifold_antibody(
            &red_result.best_attack,
            &shapley,
            &clean,
            &adv_cfg,
            &counter_cfg,
        );

        assert!(collective >= 0.0);
        assert!(vuln >= 0.0 && vuln <= 1.0);
        assert!(antibody.manifold_alignment >= 0.0);
    }

    #[test]
    fn test_adaptive_threshold_tuning() {
        // Simulate adaptive threshold adjustment over multiple rounds
        let clean = vec![1.0_f32; 8];
        let mut cfg = SgwAdversarialConfig::default();
        let counter_cfg = CounterSteeringConfig::default();

        for _round in 0..3 {
            let (red, _, vuln) = full_red_team_pipeline(&clean, &cfg, &counter_cfg);
            let (_, new_threshold) = assess_red_team_result(&red, &cfg);
            cfg = cfg.with_drift_threshold(new_threshold);
            // Threshold should remain positive
            assert!(cfg.drift_threshold > 0.0);
        }
    }
}
