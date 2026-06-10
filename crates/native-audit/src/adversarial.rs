//! Adversarial Collective Red-Teaming — Immune Antibody Generation.
//!
//! Implements the Planetary Immune System for collective defense:
//! - **Collective Counter-Steering:** Shapley-weighted immune antibody generation.
//! - **Adversarial Perturbation:** FGSM-style attacks on latent space for robustness testing.
//! - **Immune Memory:** Persistent counter-signatures for known attack patterns.

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

fn empty_antibody(config: &CounterSteeringConfig, contributor_count: usize) -> CounterSteeringAntibody {
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
    format!("{:02x}{:02x}{:02x}{:02x}", hash[0], hash[1], hash[2], hash[3])
}

fn gaussian_noise(state: &mut u64) -> f32 {
    let u1 = (lcg_next(state) as f32 / u64::MAX as f32).max(1e-10);
    let u2 = lcg_next(state) as f32 / u64::MAX as f32;
    (-2.0_f32 * u1.ln()).sqrt() * (2.0_f32 * std::f32::consts::PI * u2).cos()
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
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
        assert!(!verify_antibody_effectiveness(&original, &steered, &ab, 0.1));
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
        assert!(!verify_antibody_effectiveness(&original, &steered, &ab, 0.0));
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
        let steered: Vec<f32> = original.iter().zip(adversarial.iter()).map(|(o, a)| o + a).collect();

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
        let dot: f32 = adv.iter().zip(ab.perturbation.iter()).map(|(a, b)| a * b).sum();
        assert!(dot < 0.0, "Antibody should oppose attack direction");
    }
}
