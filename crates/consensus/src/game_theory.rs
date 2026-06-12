//! Game Theory — Deterministic Mirror Descent for Symbiotic ESS Evolution.
//!
//! Implements Stochastic Mirror Descent with KL proximal regularization
//! and seeded noise for BFT consensus compliance (Kushner-Clark compliant).
//!
//! **[ANTI-TRAMPA]:** Uses `StdRng::seed_from_u64` (NOT `rand::thread_rng()`)
//! to guarantee deterministic reproducibility across all nodes.

use rand::{SeedableRng, rngs::StdRng};
use rand_distr::{Normal, Distribution};

/// Configuration for Mirror Descent Replicator Dynamics.
#[derive(Debug, Clone)]
pub struct MirrorDescentConfig {
    /// Learning rate (step size) for mirror descent.
    pub eta: f32,
    /// Noise scale for stochastic approximation (Kushner-Clark).
    pub noise_scale: f32,
    /// Global seed for deterministic reproducibility across BFT nodes.
    pub global_seed: u64,
}

impl Default for MirrorDescentConfig {
    fn default() -> Self {
        Self {
            eta: 0.1,
            noise_scale: 0.01,
            global_seed: 42,
        }
    }
}

impl MirrorDescentConfig {
    /// Fast convergence preset (higher eta, lower noise).
    pub fn fast() -> Self {
        Self {
            eta: 0.5,
            noise_scale: 0.001,
            global_seed: 42,
        }
    }

    /// High precision preset (lower eta, minimal noise).
    pub fn high_precision() -> Self {
        Self {
            eta: 0.01,
            noise_scale: 0.0001,
            global_seed: 12345,
        }
    }

    /// Builder: set learning rate.
    pub fn with_eta(mut self, eta: f32) -> Self {
        self.eta = eta.max(0.0).min(1.0);
        self
    }

    /// Builder: set noise scale.
    pub fn with_noise_scale(mut self, noise_scale: f32) -> Self {
        self.noise_scale = noise_scale.max(0.0).min(1.0);
        self
    }

    /// Builder: set global seed.
    pub fn with_global_seed(mut self, global_seed: u64) -> Self {
        self.global_seed = global_seed;
        self
    }
}

/// Result of a single mirror descent step.
#[derive(Debug, Clone)]
pub struct MirrorDescentResult {
    /// Updated strategy distribution (simplex-normalized).
    pub new_strategies: Vec<f32>,
    /// Strategy entropy (Shannon).
    pub entropy: f32,
    /// Dominant strategy index.
    pub dominant_index: usize,
    /// Dominant strategy share.
    pub dominant_share: f32,
}

impl std::fmt::Display for MirrorDescentResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MirrorDescentResult {{ dominant: strategy[{}] = {:.4}, entropy: {:.4} }}",
            self.dominant_index, self.dominant_share, self.entropy
        )
    }
}

/// Stochastic Mirror Descent + KL Proximal for Symbiotic ESS (Kushner-Clark compliant).
///
/// Update rule: `x_next ∝ x_i * exp(η * ∇f_i + noise)`
/// where noise ~ N(0, noise_scale²) seeded with `global_seed`.
///
/// **[ANTI-TRAMPA]:** Uses `StdRng::seed_from_u64` for deterministic reproducibility.
pub fn mirror_descent_replicator_step(
    current_strategies: &[f32],
    fitness_gradients: &[f32],
    eta: f32,
    noise_scale: f32,
    global_seed: u64,
) -> Vec<f32> {
    assert_eq!(
        current_strategies.len(),
        fitness_gradients.len(),
        "Strategy and gradient lengths must match"
    );

    let mut rng = StdRng::seed_from_u64(global_seed);
    let normal = Normal::new(0.0, noise_scale as f64).unwrap();

    let mut unnorm = Vec::with_capacity(current_strategies.len());
    let mut z = 0.0f32;

    for (i, &x_i) in current_strategies.iter().enumerate() {
        let noise = normal.sample(&mut rng) as f32;
        // Mirror Descent update: x <- argmin <x, -grad> + (1/η) KL(x || x_prev)
        // Closed form: x_next ∝ x_i * exp(η * ∇f_i + noise)
        let exp_term = (eta * fitness_gradients[i] + noise).exp();
        let x_next = x_i * exp_term;
        unnorm.push(x_next);
        z += x_next;
    }

    // Simplex normalization
    unnorm.iter().map(|&x| x / (z + 1e-8)).collect()
}

/// Compute Shannon entropy of a strategy distribution.
pub fn strategy_entropy(strategies: &[f32]) -> f32 {
    let mut entropy = 0.0f32;
    for &p in strategies {
        if p > 1e-8 {
            entropy -= p * p.ln();
        }
    }
    entropy
}

/// Run multi-step mirror descent simulation.
pub fn simulate_mirror_descent(
    initial_strategies: &[f32],
    fitness_gradients: &[f32],
    config: &MirrorDescentConfig,
    steps: usize,
) -> Vec<MirrorDescentResult> {
    let mut current = initial_strategies.to_vec();
    let mut results = Vec::with_capacity(steps);

    for step in 0..steps {
        // Increment seed each step for different noise
        let step_seed = config.global_seed.wrapping_add(step as u64);
        let new_strategies = mirror_descent_replicator_step(
            &current,
            fitness_gradients,
            config.eta,
            config.noise_scale,
            step_seed,
        );

        let entropy = strategy_entropy(&new_strategies);
        let dominant_index = new_strategies
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let dominant_share = new_strategies[dominant_index];

        results.push(MirrorDescentResult {
            new_strategies: new_strategies.clone(),
            entropy,
            dominant_index,
            dominant_share,
        });

        current = new_strategies;
    }

    results
}

// ─── Unit Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror_descent_config_default() {
        let cfg = MirrorDescentConfig::default();
        assert!((cfg.eta - 0.1).abs() < 1e-6);
        assert!((cfg.noise_scale - 0.01).abs() < 1e-6);
        assert_eq!(cfg.global_seed, 42);
    }

    #[test]
    fn test_mirror_descent_config_fast() {
        let cfg = MirrorDescentConfig::fast();
        assert!((cfg.eta - 0.5).abs() < 1e-6);
        assert!((cfg.noise_scale - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_mirror_descent_config_high_precision() {
        let cfg = MirrorDescentConfig::high_precision();
        assert!((cfg.eta - 0.01).abs() < 1e-6);
        assert!((cfg.noise_scale - 0.0001).abs() < 1e-6);
    }

    #[test]
    fn test_mirror_descent_config_with_eta() {
        let cfg = MirrorDescentConfig::default().with_eta(0.3);
        assert!((cfg.eta - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_mirror_descent_config_eta_clamped_high() {
        let cfg = MirrorDescentConfig::default().with_eta(2.0);
        assert!((cfg.eta - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mirror_descent_config_eta_clamped_low() {
        let cfg = MirrorDescentConfig::default().with_eta(-0.5);
        assert!(cfg.eta < 1e-6);
    }

    #[test]
    fn test_mirror_descent_config_with_noise_scale() {
        let cfg = MirrorDescentConfig::default().with_noise_scale(0.05);
        assert!((cfg.noise_scale - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_mirror_descent_config_with_global_seed() {
        let cfg = MirrorDescentConfig::default().with_global_seed(999);
        assert_eq!(cfg.global_seed, 999);
    }

    #[test]
    fn test_mirror_descent_replicator_step_basic() {
        let strategies = vec![0.25, 0.25, 0.25, 0.25];
        let gradients = vec![1.0, 0.5, -0.5, -1.0];
        let result = mirror_descent_replicator_step(&strategies, &gradients, 0.1, 0.01, 42);
        assert_eq!(result.len(), 4);
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "Must sum to 1.0 (simplex)");
    }

    #[test]
    fn test_mirror_descent_replicator_step_deterministic() {
        let strategies = vec![0.3, 0.3, 0.4];
        let gradients = vec![0.8, -0.2, 0.1];
        let r1 = mirror_descent_replicator_step(&strategies, &gradients, 0.1, 0.01, 123);
        let r2 = mirror_descent_replicator_step(&strategies, &gradients, 0.1, 0.01, 123);
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert!((a - b).abs() < 1e-8, "Must be deterministic with same seed");
        }
    }

    #[test]
    fn test_mirror_descent_replicator_step_high_fitness_dominates() {
        let strategies = vec![0.5, 0.5];
        let gradients = vec![2.0, -1.0]; // Strategy 0 has much higher fitness
        let result = mirror_descent_replicator_step(&strategies, &gradients, 0.5, 0.001, 42);
        assert!(
            result[0] > result[1],
            "High fitness strategy should dominate: {:.4} > {:.4}",
            result[0],
            result[1]
        );
    }

    #[test]
    fn test_mirror_descent_replicator_step_zero_noise() {
        let strategies = vec![0.33, 0.33, 0.34];
        let gradients = vec![1.0, 0.0, -1.0];
        let result = mirror_descent_replicator_step(&strategies, &gradients, 0.1, 0.0, 42);
        assert!(result[0] > strategies[0], "Positive gradient → increase share");
        assert!(result[2] < strategies[2], "Negative gradient → decrease share");
    }

    #[test]
    fn test_strategy_entropy_uniform() {
        let strategies = vec![0.25, 0.25, 0.25, 0.25];
        let entropy = strategy_entropy(&strategies);
        let expected = (4.0f32).ln();
        assert!(
            (entropy - expected).abs() < 1e-4,
            "Uniform entropy = ln(n): {:.6} ≈ {:.6}",
            entropy,
            expected
        );
    }

    #[test]
    fn test_strategy_entropy_deterministic() {
        let strategies = vec![1.0, 0.0, 0.0];
        let entropy = strategy_entropy(&strategies);
        assert!(entropy < 1e-4, "Deterministic → zero entropy");
    }

    #[test]
    fn test_simulate_mirror_descent_returns_correct_length() {
        let strategies = vec![0.5, 0.5];
        let gradients = vec![1.0, -0.5];
        let config = MirrorDescentConfig::default();
        let results = simulate_mirror_descent(&strategies, &gradients, &config, 10);
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_simulate_mirror_descent_convergence() {
        let strategies = vec![0.5, 0.5];
        let gradients = vec![2.0, -1.0]; // Strong preference for strategy 0
        let config = MirrorDescentConfig::fast();
        let results = simulate_mirror_descent(&strategies, &gradients, &config, 50);
        let final_result = results.last().unwrap();
        assert!(
            final_result.dominant_share > 0.9,
            "Should converge to dominant strategy: {:.4}",
            final_result.dominant_share
        );
    }

    #[test]
    fn test_simulate_mirror_descent_entropy_decreases() {
        let strategies = vec![0.33, 0.33, 0.34];
        let gradients = vec![2.0, 0.0, -1.0];
        let config = MirrorDescentConfig::default();
        let results = simulate_mirror_descent(&strategies, &gradients, &config, 30);
        let initial_entropy = results.first().unwrap().entropy;
        let final_entropy = results.last().unwrap().entropy;
        assert!(
            final_entropy < initial_entropy,
            "Entropy should decrease: {:.4} < {:.4}",
            final_entropy,
            initial_entropy
        );
    }

    #[test]
    fn test_mirror_descent_result_display() {
        let result = MirrorDescentResult {
            new_strategies: vec![0.7, 0.2, 0.1],
            entropy: 0.5,
            dominant_index: 0,
            dominant_share: 0.7,
        };
        let display = format!("{}", result);
        assert!(display.contains("dominant"));
        assert!(display.contains("0.7000"));
    }

    #[test]
    #[should_panic(expected = "Strategy and gradient lengths must match")]
    fn test_mirror_descent_length_mismatch_panics() {
        let strategies = vec![0.5, 0.5];
        let gradients = vec![1.0]; // Wrong length
        mirror_descent_replicator_step(&strategies, &gradients, 0.1, 0.01, 42);
    }

    #[test]
    fn test_full_ess_simulation() {
        // Symbiotic (high fitness), Parasitic (low fitness), Byzantine (negative fitness)
        let strategies = vec![0.33, 0.33, 0.34];
        let gradients = vec![1.5, -0.5, -2.0];
        let config = MirrorDescentConfig::default();
        let results = simulate_mirror_descent(&strategies, &gradients, &config, 100);
        let final_result = results.last().unwrap();

        // Symbiotic should dominate
        assert!(
            final_result.dominant_index == 0,
            "Symbiotic must be dominant strategy"
        );
        assert!(
            final_result.dominant_share > 0.9,
            "Symbiotic share must exceed 0.9: {:.4}",
            final_result.dominant_share
        );
        // Entropy should be low (concentrated on symbiotic)
        assert!(
            final_result.entropy < 0.5,
            "Entropy must be low at ESS: {:.4}",
            final_result.entropy
        );
    }
}
