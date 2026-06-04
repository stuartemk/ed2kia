//! Cooperative Objective Loss â€” Sprint 68: Academic Formalization & Validation Layer
//!
//! Mathematical formalization of the Topological principle: **Love = Zero Conflict**.
//! Achieved when gradient divergence approaches zero while policy entropy (diversity) is maintained.
//!
//! # Formula
//!
//! ```text
//! L = grad_div + LAMBDA * policy_entropy - MU * bench_penalty
//! ```
//!
//! Where:
//! - `grad_div` = pairwise L2 divergence of gradients (conflict metric)
//! - `policy_entropy` = KL divergence proxy for policy diversity
//! - `bench_penalty` = weighted sum of ethical benchmark scores
//! - `LAMBDA` = entropic regularization weight (default 0.1)
//! - `MU` = ethical benchmark weight (default 0.9)

#[cfg(feature = "v9.4-validation-layer")]
use std::f64;

/// Entropic regularization weight â€” balances diversity vs convergence.
pub const LAMBDA: f64 = 0.1;

/// Ethical benchmark weight â€” rewards high ethical alignment.
pub const MU: f64 = 0.9;

/// Minimum value to avoid log(0) in KL divergence.
pub const EPSILON: f64 = 1e-9;

/// Ethical benchmark entry with score and weight.
#[derive(Debug, Clone)]
pub struct BenchmarkScore {
    pub score: f64,
    pub weight: f64,
}

impl BenchmarkScore {
    pub fn new(score: f64, weight: f64) -> Self {
        Self { score, weight }
    }
}

/// Compute the cooperative objective loss.
///
/// The mathematical formalization of the Topological principle: Love = Zero Conflict.
/// Achieved when gradient divergence approaches zero while policy entropy (diversity) is maintained.
///
/// # Arguments
/// * `gradients` â€” Per-node gradient vectors (conflict measured as pairwise L2 divergence).
/// * `policies` â€” Per-node policy distributions (diversity measured as KL divergence proxy).
/// * `benchmarks` â€” Ethical benchmark scores with weights.
///
/// # Returns
/// Loss value. Negative loss indicates cooperative equilibrium (Love state).
pub fn compute_love_metric_loss(
    gradients: &[Vec<f64>],
    policies: &[Vec<f64>],
    benchmarks: &[BenchmarkScore],
) -> f64 {
    let grad_div = pairwise_l2_divergence(gradients);
    let policy_entropy = kl_divergence_entropy(policies);
    let bench_penalty: f64 = benchmarks.iter().map(|b| b.score * b.weight).sum();
    grad_div + LAMBDA * policy_entropy - MU * bench_penalty
}

/// Compute pairwise L2 divergence across all gradient vectors.
///
/// Measures algorithmic conflict: lower values indicate gradient alignment.
pub fn pairwise_l2_divergence(gradients: &[Vec<f64>]) -> f64 {
    let mut sum = 0.0;
    let mut count = 0;
    for (i, g1) in gradients.iter().enumerate() {
        for g2 in gradients.iter().skip(i + 1) {
            let dist: f64 = g1
                .iter()
                .zip(g2.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            sum += dist;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Compute KL divergence proxy for policy diversity.
///
/// Measures epistemic diversity: higher values indicate more diverse policies.
/// Uses simplified KL divergence against the mean policy distribution.
pub fn kl_divergence_entropy(policies: &[Vec<f64>]) -> f64 {
    if policies.is_empty() {
        return 0.0;
    }
    // Compute mean policy
    let len = policies[0].len();
    let mut avg = vec![0.0; len];
    for p in policies {
        for (j, v) in p.iter().enumerate() {
            avg[j] += v;
        }
    }
    for v in avg.iter_mut() {
        *v /= policies.len() as f64;
    }
    // Compute KL divergence of each policy from mean
    let total: f64 = policies
        .iter()
        .map(|p| {
            p.iter()
                .zip(avg.iter())
                .map(|(pi, ai)| {
                    if *pi > EPSILON && *ai > EPSILON {
                        pi * (pi / ai).ln()
                    } else {
                        0.0
                    }
                })
                .sum::<f64>()
        })
        .sum();
    total / policies.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_conflict_convergence() {
        let grads = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let pols = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let bench = vec![BenchmarkScore::new(0.9, 1.0)];
        let loss = compute_love_metric_loss(&grads, &pols, &bench);
        assert!(
            loss < 0.0,
            "Loss should be negative when conflict is zero and benchmarks are high (got {})",
            loss
        );
    }

    #[test]
    fn test_high_conflict_positive_loss() {
        let grads = vec![vec![0.0, 0.0], vec![10.0, 10.0]];
        let pols = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let bench = vec![BenchmarkScore::new(0.1, 1.0)];
        let loss = compute_love_metric_loss(&grads, &pols, &bench);
        assert!(
            loss > 0.0,
            "Loss should be positive when conflict is high and benchmarks are low (got {})",
            loss
        );
    }

    #[test]
    fn test_empty_gradients() {
        let grads: Vec<Vec<f64>> = vec![];
        let pols: Vec<Vec<f64>> = vec![];
        let bench: Vec<BenchmarkScore> = vec![];
        let loss = compute_love_metric_loss(&grads, &pols, &bench);
        assert_eq!(loss, 0.0, "Empty inputs should yield zero loss");
    }

    #[test]
    fn test_single_gradient_zero_divergence() {
        let grads = vec![vec![1.0, 2.0, 3.0]];
        let pols = vec![vec![0.33, 0.33, 0.34]];
        let bench = vec![BenchmarkScore::new(0.5, 1.0)];
        let loss = compute_love_metric_loss(&grads, &pols, &bench);
        // Single gradient has zero divergence, so loss = 0 + LAMBDA*entropy - MU*0.5
        assert!(
            loss < 0.0,
            "Single gradient with good benchmark should yield negative loss"
        );
    }

    #[test]
    fn test_benchmark_weight_impact() {
        let grads = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let pols = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let bench_low = vec![BenchmarkScore::new(0.1, 1.0)];
        let bench_high = vec![BenchmarkScore::new(0.95, 1.0)];
        let loss_low = compute_love_metric_loss(&grads, &pols, &bench_low);
        let loss_high = compute_love_metric_loss(&grads, &pols, &bench_high);
        assert!(
            loss_high < loss_low,
            "Higher benchmark scores should reduce loss (low={}, high={})",
            loss_low,
            loss_high
        );
    }

    #[test]
    fn test_pairwise_l2_divergence_identical() {
        let grads = vec![vec![1.0, 2.0], vec![1.0, 2.0], vec![1.0, 2.0]];
        let div = pairwise_l2_divergence(&grads);
        assert_eq!(div, 0.0, "Identical gradients should have zero divergence");
    }

    #[test]
    fn test_pairwise_l2_divergence_different() {
        let grads = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
        let div = pairwise_l2_divergence(&grads);
        // L2 distance = sqrt(9 + 16) = 5.0
        assert!(
            (div - 5.0).abs() < 1e-9,
            "Expected divergence 5.0, got {}",
            div
        );
    }

    #[test]
    fn test_kl_divergence_entropy_identical_policies() {
        let pols = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let entropy = kl_divergence_entropy(&pols);
        // Identical policies have zero KL divergence from mean
        assert!(
            entropy.abs() < 1e-9,
            "Identical policies should have near-zero KL entropy"
        );
    }

    #[test]
    fn test_constants_valid_range() {
        assert!((0.0..=1.0).contains(&LAMBDA), "LAMBDA should be in [0, 1]");
        assert!((0.0..=1.0).contains(&MU), "MU should be in [0, 1]");
    }
}
