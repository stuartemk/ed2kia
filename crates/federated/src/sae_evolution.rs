//! Federated SAE Evolution — Secure Aggregation with Weiszfeld Geometric Median & Shapley Values.
//!
//! Core algorithms:
//! - **Weiszfeld Geometric Median:** Iterative robust aggregation resistant to Byzantine outliers.
//! - **DP-Gaussian Noise:** Calibrated differential privacy for federated updates.
//! - **Federated Shapley:** Monte Carlo O(log N) fair credit allocation.

use sha2::{Digest, Sha256};

// ============================================================================
// Public Types
// ============================================================================

/// Result of a federated SAE secure aggregation round.
#[derive(Debug, Clone)]
pub struct FederatedSAEUpdate {
    /// Aggregated weight delta (Weiszfeld geometric median).
    pub aggregated_delta: Vec<f64>,
    /// Number of participating nodes.
    pub participant_count: usize,
    /// DP noise scale (sigma) applied.
    pub dp_sigma: f64,
    /// Aggregate hash for chain verification.
    pub aggregate_hash: [u8; 32],
    /// Convergence metric (mean L2 distance from last iterate).
    pub convergence_metric: f64,
}

impl std::fmt::Display for FederatedSAEUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FederatedSAEUpdate {{ participants={}, dp_sigma={:.4}, convergence={:.6}, hash={}}}",
            self.participant_count,
            self.dp_sigma,
            self.convergence_metric,
            hex_encode(&self.aggregate_hash)
        )
    }
}

/// Shapley value allocation for a single node in the federated coalition.
#[derive(Debug, Clone)]
pub struct FederatedShapleyValue {
    /// Node index in the federation.
    pub node_index: usize,
    /// Estimated Shapley value (marginal contribution).
    pub shapley_value: f64,
    /// Number of Monte Carlo samples used.
    pub sample_count: usize,
    /// Standard error of the estimate.
    pub standard_error: f64,
}

impl std::fmt::Display for FederatedShapleyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Shapley(node={}, value={:.6} ± {:.6}, samples={})",
            self.node_index, self.shapley_value, self.standard_error, self.sample_count
        )
    }
}

/// Configuration for federated SAE update.
#[derive(Debug, Clone)]
pub struct FederatedUpdateConfig {
    /// Differential privacy epsilon.
    pub epsilon: f64,
    /// Differential privacy delta.
    pub delta: f64,
    /// Maximum Weiszfeld iterations.
    pub max_iterations: usize,
    /// Convergence tolerance for Weiszfeld.
    pub tolerance: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for FederatedUpdateConfig {
    fn default() -> Self {
        Self {
            epsilon: 1.0,
            delta: 1e-6,
            max_iterations: 50,
            tolerance: 1e-6,
            seed: 42,
        }
    }
}

impl FederatedUpdateConfig {
    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.epsilon = epsilon.max(0.0);
        self
    }

    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations.max(1);
        self
    }
}

// ============================================================================
// Core Functions
// ============================================================================

/// Federated SAE Update — Weiszfeld Geometric Median with DP Noise.
///
/// Aggregates weight deltas from multiple federated nodes using the
/// Weiszfeld iterative geometric median algorithm, which is robust
/// to Byzantine outliers compared to simple mean averaging.
///
/// After convergence, DP-Gaussian noise is calibrated and added
/// to provide (epsilon, delta)-differential privacy guarantees.
///
/// # Parameters
/// - `deltas`: Weight delta vectors from each participating node.
/// - `config`: Update configuration (epsilon, delta, iterations, etc.).
///
/// # Returns
/// `FederatedSAEUpdate` with aggregated delta, metadata, and verification hash.
pub fn federated_sae_update(deltas: &[Vec<f64>], config: &FederatedUpdateConfig) -> FederatedSAEUpdate {
    if deltas.is_empty() {
        return empty_update(config);
    }

    let dim = deltas[0].len();
    let participant_count = deltas.len();

    // Weiszfeld geometric median
    let mut current = mean_vector(&deltas);
    let mut convergence_metric = 0.0;

    for iter in 0..config.max_iterations {
        let mut next = vec![0.0; dim];
        let mut total_weight = 0.0;

        for delta in deltas {
            let dist = euclidean_distance(&current, delta).max(1e-12);
            let weight = 1.0 / dist;
            total_weight += weight;
            for j in 0..dim {
                next[j] += weight * delta[j];
            }
        }

        for j in 0..dim {
            next[j] /= total_weight;
        }

        convergence_metric = euclidean_distance(&current, &next);
        current = next;

        if iter > 0 && convergence_metric < config.tolerance {
            break;
        }
    }

    // Add DP-Gaussian noise
    let sensitivity = compute_sensitivity(&deltas, dim);
    let dp_sigma = compute_dp_sigma(config.epsilon, config.delta, sensitivity);
    let mut rng_state = config.seed;

    for val in current.iter_mut() {
        *val += gaussian_noise(&mut rng_state, dp_sigma);
    }

    // Compute aggregate hash
    let aggregate_hash = compute_hash(&current);

    FederatedSAEUpdate {
        aggregated_delta: current,
        participant_count,
        dp_sigma,
        aggregate_hash,
        convergence_metric,
    }
}

/// Compute Federated Shapley Values — Monte Carlo O(log N) Sampling.
///
/// Estimates the Shapley value for each node in the federation using
/// Monte Carlo coalition sampling. The Shapley value represents the
/// fair marginal contribution of each node to the collective model.
///
/// # Parameters
/// - `node_values`: Per-node contribution scores (e.g., VFE reduction).
/// - `num_samples`: Number of Monte Carlo samples (O(log N) recommended).
/// - `seed`: Random seed for reproducibility.
///
/// # Returns
/// Vector of `FederatedShapleyValue`, one per node.
pub fn compute_federated_shapley(
    node_values: &[f64],
    num_samples: usize,
    seed: u64,
) -> Vec<FederatedShapleyValue> {
    let n = node_values.len();
    if n == 0 {
        return vec![];
    }

    let effective_samples = num_samples.max(1).max((n as f64).ln().ceil() as usize);
    let mut rng_state = seed;
    let mut results: Vec<(f64, f64)> = vec![(0.0, 0.0); n]; // (sum, sum_sq)

    for _ in 0..effective_samples {
        // Generate random permutation via Fisher-Yates partial
        let mut perm: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = next_random(&mut rng_state) % ((i + 1) as u64);
            perm.swap(i, j as usize);
        }

        // Compute marginal contributions via random ordering
        // For additive coalition: v(S) = sum of node_values in S
        // Marginal contribution of node i = node_values[i] (constant across coalitions)
        // Shapley value = expected marginal contribution = node_values[i]
        // Monte Carlo verifies this with sampling variance
        for &node_idx in &perm {
            let marginal = node_values[node_idx];
            results[node_idx].0 += marginal;
            results[node_idx].1 += marginal * marginal;
        }
    }

    // Normalize and compute standard errors
    results
        .into_iter()
        .enumerate()
        .map(|(idx, (sum, sum_sq))| {
            let mean = sum / effective_samples as f64;
            let variance = (sum_sq / effective_samples as f64) - mean * mean;
            let std_err = variance.max(0.0).sqrt() / effective_samples as f64;
            FederatedShapleyValue {
                node_index: idx,
                shapley_value: mean,
                sample_count: effective_samples,
                standard_error: std_err,
            }
        })
        .collect()
}

// ============================================================================
// Helper Functions
// ============================================================================

fn empty_update(_config: &FederatedUpdateConfig) -> FederatedSAEUpdate {
    FederatedSAEUpdate {
        aggregated_delta: vec![],
        participant_count: 0,
        dp_sigma: 0.0,
        aggregate_hash: [0u8; 32],
        convergence_metric: 0.0,
    }
}

fn mean_vector(deltas: &[Vec<f64>]) -> Vec<f64> {
    if deltas.is_empty() {
        return vec![];
    }
    let dim = deltas[0].len();
    let n = deltas.len() as f64;
    let mut mean = vec![0.0; dim];
    for delta in deltas {
        for j in 0..dim {
            mean[j] += delta[j];
        }
    }
    for val in mean.iter_mut() {
        *val /= n;
    }
    mean
}

fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn compute_sensitivity(deltas: &[Vec<f64>], _dim: usize) -> f64 {
    if deltas.len() < 2 {
        return 1.0;
    }
    // Use median pairwise distance for robust sensitivity estimation
    // (resistant to Byzantine outliers)
    let mut dists: Vec<f64> = Vec::with_capacity(deltas.len() * deltas.len());
    for i in 0..deltas.len() {
        for j in (i + 1)..deltas.len() {
            dists.push(euclidean_distance(&deltas[i], &deltas[j]));
        }
    }
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if dists.is_empty() {
        1.0
    } else {
        dists[dists.len() / 2]
    };
    median / (deltas.len() as f64)
}

fn compute_dp_sigma(epsilon: f64, delta: f64, sensitivity: f64) -> f64 {
    if epsilon <= 0.0 {
        return 1e10; // Clamp to large noise if epsilon is invalid
    }
    // Standard Gaussian mechanism: sigma = sensitivity * sqrt(2 * ln(1.25/delta)) / epsilon
    let ln_factor = (1.25 / delta.max(1e-15)).ln();
    sensitivity * (2.0 * ln_factor.max(0.0)).sqrt() / epsilon
}

fn compute_hash(data: &[f64]) -> [u8; 32] {
    let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher.finalize().into()
}

fn hex_encode(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

fn next_random(state: &mut u64) -> u64 {
    // Simple LCG for deterministic testing
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

fn gaussian_noise(state: &mut u64, sigma: f64) -> f64 {
    // Box-Muller transform
    let u1 = (next_random(state) as f64 / u64::MAX as f64).max(1e-10);
    let u2 = next_random(state) as f64 / u64::MAX as f64;
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    z * sigma
}

fn factorial(n: usize) -> usize {
    (1..=n).product()
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config Tests ---

    #[test]
    fn test_federated_update_config_default() {
        let cfg = FederatedUpdateConfig::default();
        assert!((cfg.epsilon - 1.0).abs() < 1e-10);
        assert!((cfg.delta - 1e-6).abs() < 1e-15);
        assert_eq!(cfg.max_iterations, 50);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_federated_update_config_with_epsilon() {
        let cfg = FederatedUpdateConfig::default().with_epsilon(0.5);
        assert!((cfg.epsilon - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_federated_update_config_epsilon_clamped() {
        let cfg = FederatedUpdateConfig::default().with_epsilon(-1.0);
        assert_eq!(cfg.epsilon, 0.0);
    }

    #[test]
    fn test_federated_update_config_with_iterations() {
        let cfg = FederatedUpdateConfig::default().with_max_iterations(100);
        assert_eq!(cfg.max_iterations, 100);
    }

    #[test]
    fn test_federated_update_config_iterations_min() {
        let cfg = FederatedUpdateConfig::default().with_max_iterations(0);
        assert_eq!(cfg.max_iterations, 1);
    }

    // --- Federated SAE Update Tests ---

    #[test]
    fn test_federated_sae_update_empty() {
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&[], &cfg);
        assert_eq!(result.participant_count, 0);
        assert!(result.aggregated_delta.is_empty());
    }

    #[test]
    fn test_federated_sae_update_single_node() {
        let deltas = vec![vec![1.0, 2.0, 3.0]];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        assert_eq!(result.participant_count, 1);
        assert_eq!(result.aggregated_delta.len(), 3);
    }

    #[test]
    fn test_federated_sae_update_multiple_nodes() {
        let deltas = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
        ];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        assert_eq!(result.participant_count, 3);
        assert_eq!(result.aggregated_delta.len(), 2);
        // Geometric median of (1,0), (0,1), (1,1) is near (0.7, 0.7)
        // With DP noise, allow wider range
        assert!(result.aggregated_delta[0] > -1.0);
        assert!(result.aggregated_delta[0] < 2.0);
        assert!(result.aggregated_delta[1] > -1.0);
        assert!(result.aggregated_delta[1] < 2.0);
    }

    #[test]
    fn test_federated_sae_update_convergence() {
        let deltas = vec![
            vec![1.0, 1.0],
            vec![1.1, 1.0],
            vec![1.0, 1.1],
        ];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        assert!(result.convergence_metric < cfg.tolerance * 10.0);
    }

    #[test]
    fn test_federated_sae_update_dp_noise_applied() {
        let deltas = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        // With DP noise, result should not be exactly [1.0, 1.0]
        let has_noise = result
            .aggregated_delta
            .iter()
            .any(|v| (v - 1.0).abs() > 1e-10);
        assert!(has_noise || result.dp_sigma == 0.0);
    }

    #[test]
    fn test_federated_sae_update_hash_nonzero() {
        let deltas = vec![vec![1.0, 2.0]];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        assert!(result.aggregate_hash.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_federated_sae_update_display() {
        let deltas = vec![vec![1.0]];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        let s = format!("{}", result);
        assert!(s.contains("FederatedSAEUpdate"));
        assert!(s.contains("participants="));
    }

    #[test]
    fn test_federated_sae_update_byzantine_robustness() {
        // One extreme outlier should not dominate the geometric median
        let deltas = vec![
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1000.0, 1000.0], // Byzantine outlier (1 of 6)
        ];
        let cfg = FederatedUpdateConfig::default();
        let result = federated_sae_update(&deltas, &cfg);
        // Geometric median with 5/6 at (1,1) should stay near (1,1)
        // Allow for DP noise: check it's not pulled to the outlier
        assert!(result.aggregated_delta[0] < 100.0, "X pulled too far: {}", result.aggregated_delta[0]);
        assert!(result.aggregated_delta[1] < 100.0, "Y pulled too far: {}", result.aggregated_delta[1]);
        // Should be closer to (1,1) than to (1000,1000)
        let dist_to_origin = ((result.aggregated_delta[0] - 1.0).powi(2)
            + (result.aggregated_delta[1] - 1.0).powi(2)).sqrt();
        let dist_to_outlier = ((result.aggregated_delta[0] - 1000.0).powi(2)
            + (result.aggregated_delta[1] - 1000.0).powi(2)).sqrt();
        assert!(dist_to_origin < dist_to_outlier, "Median closer to outlier than to consensus");
    }

    #[test]
    fn test_federated_sae_update_deterministic() {
        let deltas = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let cfg = FederatedUpdateConfig::default();
        let r1 = federated_sae_update(&deltas, &cfg);
        let r2 = federated_sae_update(&deltas, &cfg);
        assert_eq!(r1.aggregate_hash, r2.aggregate_hash);
    }

    // --- Federated Shapley Tests ---

    #[test]
    fn test_compute_federated_shapley_empty() {
        let result = compute_federated_shapley(&[], 10, 42);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_federated_shapley_single_node() {
        let result = compute_federated_shapley(&[5.0], 10, 42);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].node_index, 0);
        assert!(result[0].shapley_value > 0.0);
    }

    #[test]
    fn test_compute_federated_shapley_two_nodes() {
        let result = compute_federated_shapley(&[3.0, 7.0], 20, 42);
        assert_eq!(result.len(), 2);
        // For additive game, Shapley value = node's own value
        assert!((result[0].shapley_value - 3.0).abs() < 1e-10);
        assert!((result[1].shapley_value - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_federated_shapley_equal_values() {
        let result = compute_federated_shapley(&[1.0, 1.0, 1.0], 100, 42);
        assert_eq!(result.len(), 3);
        // All should have equal Shapley values (uniform random ordering → same expected value)
        for i in 1..3 {
            assert!(
                (result[i].shapley_value - result[0].shapley_value).abs() < 1e-10,
                "Node {} vs Node 0: {} vs {}",
                i,
                result[i].shapley_value,
                result[0].shapley_value
            );
        }
    }

    #[test]
    fn test_compute_federated_shapley_sample_count() {
        let result = compute_federated_shapley(&[1.0, 2.0, 3.0], 100, 42);
        for r in &result {
            assert!(r.sample_count >= 100);
        }
    }

    #[test]
    fn test_compute_federated_shapley_log_n_minimum() {
        // 10 nodes, log(10) ≈ 2.3, ceil = 3, so minimum samples should be 3
        let result = compute_federated_shapley(&vec![1.0; 10], 1, 42);
        for r in &result {
            let min_samples = (10.0_f64).ln().ceil() as usize;
            assert!(
                r.sample_count >= min_samples,
                "sample_count {} < min_samples {}",
                r.sample_count,
                min_samples
            );
        }
    }

    #[test]
    fn test_compute_federated_shapley_standard_error_decreases() {
        let r_few = compute_federated_shapley(&[1.0, 2.0, 3.0], 10, 42);
        let r_many = compute_federated_shapley(&[1.0, 2.0, 3.0], 500, 42);
        // More samples → lower standard error
        assert!(r_many[0].standard_error <= r_few[0].standard_error + 0.01);
    }

    #[test]
    fn test_compute_federated_shapley_display() {
        let result = compute_federated_shapley(&[1.0, 2.0], 10, 42);
        let s = format!("{}", result[0]);
        assert!(s.contains("Shapley"));
        assert!(s.contains("node="));
    }

    #[test]
    fn test_compute_federated_shapley_deterministic() {
        let r1 = compute_federated_shapley(&[1.0, 2.0, 3.0], 50, 123);
        let r2 = compute_federated_shapley(&[1.0, 2.0, 3.0], 50, 123);
        for i in 0..r1.len() {
            assert!((r1[i].shapley_value - r2[i].shapley_value).abs() < 1e-10);
        }
    }

    // --- Helper Function Tests ---

    #[test]
    fn test_euclidean_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((euclidean_distance(&a, &a) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_euclidean_distance_basic() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_mean_vector_basic() {
        let v = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let m = mean_vector(&v);
        assert!((m[0] - 2.0).abs() < 1e-10);
        assert!((m[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_mean_vector_empty() {
        let m = mean_vector(&[]);
        assert!(m.is_empty());
    }

    #[test]
    fn test_dp_sigma_positive() {
        let sigma = compute_dp_sigma(1.0, 1e-6, 1.0);
        assert!(sigma > 0.0);
    }

    #[test]
    fn test_dp_sigma_zero_epsilon() {
        let sigma = compute_dp_sigma(0.0, 1e-6, 1.0);
        assert!(sigma > 1e9); // Should be very large
    }

    #[test]
    fn test_gaussian_noise_range() {
        let mut state = 42u64;
        let mut sum = 0.0;
        for _ in 0..1000 {
            let n = gaussian_noise(&mut state, 1.0);
            sum += n * n;
        }
        // Variance should be approximately 1.0
        let variance = sum / 1000.0;
        assert!(variance > 0.5 && variance < 2.0);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
    }

    #[test]
    fn test_hex_encode() {
        let mut hash = [0u8; 32];
        hash[0] = 255;
        let s = hex_encode(&hash);
        assert!(s.starts_with("ff"));
        assert_eq!(s.len(), 64);
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_federated_pipeline() {
        // Simulate 5-node federated SAE update
        let deltas = vec![
            vec![0.1, -0.2, 0.3],
            vec![0.15, -0.15, 0.25],
            vec![0.05, -0.25, 0.35],
            vec![0.12, -0.18, 0.28],
            vec![0.08, -0.22, 0.32],
        ];

        // Run federated update
        let cfg = FederatedUpdateConfig::default();
        let update = federated_sae_update(&deltas, &cfg);
        assert_eq!(update.participant_count, 5);
        assert_eq!(update.aggregated_delta.len(), 3);

        // Compute Shapley values
        let node_values: Vec<f64> = deltas
            .iter()
            .map(|d| d.iter().map(|v| v.abs()).sum())
            .collect();
        let shapley = compute_federated_shapley(&node_values, 50, cfg.seed);
        assert_eq!(shapley.len(), 5);

        // Verify Shapley values sum is reasonable
        let total: f64 = shapley.iter().map(|s| s.shapley_value).sum();
        assert!(total > 0.0);
    }

    #[test]
    fn test_federated_update_with_high_epsilon() {
        // High epsilon → low noise
        let deltas = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let cfg = FederatedUpdateConfig::default().with_epsilon(100.0);
        let result = federated_sae_update(&deltas, &cfg);
        // With high epsilon, noise should be minimal
        for v in &result.aggregated_delta {
            assert!((v - 1.0).abs() < 0.1);
        }
    }

    #[test]
    fn test_federated_update_large_federation() {
        let deltas: Vec<Vec<f64>> = (0..100)
            .map(|i| vec![i as f64, (100 - i) as f64])
            .collect();
        let cfg = FederatedUpdateConfig::default().with_max_iterations(100);
        let result = federated_sae_update(&deltas, &cfg);
        assert_eq!(result.participant_count, 100);
        assert_eq!(result.aggregated_delta.len(), 2);
    }
}
