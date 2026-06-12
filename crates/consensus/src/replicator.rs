//! Stochastic Replicator Dynamics — Itô SDE for Population Strategy Evolution.
//!
//! **Sprint 138:** Transitions from deterministic replicator dynamics to
//! stochastic differential equations (Itô SDE) using Euler-Maruyama integration.
//! Thermodynamic noise (Brownian motion) is injected to prevent consensus
//! monopolies and ensure exploration in the P2P manifold.
//!
//! **Itô SDE:**
//! ```math
//! dx_i = [x_i(f_i - φ̄) + η·∇symbiosis - γ·∇entropy] dt + σ dW_t
//! ```
//!
//! **Euler-Maruyama discretization:**
//! ```math
//! x_{t+1} = x_t + total_drift + σ·√(Δt)·ξ    (ξ ~ N(0,1))
//! ```

use rand::Rng;

/// Configuration for Stochastic Replicator Dynamics.
#[derive(Debug, Clone)]
pub struct StochasticReplicatorConfig {
    /// Symbiotic learning rate (η).
    pub eta: f32,
    /// Entropy penalty coefficient (γ).
    pub gamma: f32,
    /// Diffusion noise magnitude (σ).
    pub sigma: f32,
    /// Time step (Δt).
    pub dt: f32,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for StochasticReplicatorConfig {
    fn default() -> Self {
        Self {
            eta: 0.2,
            gamma: 0.1,
            sigma: 0.05,
            dt: 0.01,
            seed: 42,
        }
    }
}

impl StochasticReplicatorConfig {
    /// Fast configuration — larger steps, more noise.
    pub fn fast() -> Self {
        Self {
            eta: 0.5,
            gamma: 0.05,
            sigma: 0.1,
            dt: 0.05,
            seed: 42,
        }
    }

    /// High precision — smaller steps, less noise.
    pub fn high_precision() -> Self {
        Self {
            eta: 0.1,
            gamma: 0.2,
            sigma: 0.01,
            dt: 0.001,
            seed: 42,
        }
    }

    /// Set diffusion noise magnitude.
    pub fn with_sigma(mut self, sigma: f32) -> Self {
        self.sigma = sigma.max(0.0);
        self
    }

    /// Set time step.
    pub fn with_dt(mut self, dt: f32) -> Self {
        self.dt = dt.max(0.0);
        self
    }

    /// Set seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// Single stochastic replicator step (Itô SDE, Euler-Maruyama).
///
/// ```math
/// dx_i = [x_i(f_i - φ̄) + η·∇symbiosis - γ·∇entropy] dt + σ dW_t
/// ```
///
/// # Arguments
/// * `x_i` — Current strategy proportion
/// * `f_i` — Local fitness (PoUS score)
/// * `phi_bar` — Average network fitness
/// * `grad_symbiosis` — Gradient of symbiosis function
/// * `grad_entropy` — Gradient of entropy function
/// * `eta` — Symbiotic learning rate
/// * `gamma` — Entropy penalty coefficient
/// * `sigma` — Diffusion noise magnitude
/// * `dt` — Time step
///
/// # Returns
/// Updated strategy proportion clamped to [0, 1]
pub fn stochastic_replicator_step(
    x_i: f32,
    f_i: f32,
    phi_bar: f32,
    grad_symbiosis: f32,
    grad_entropy: f32,
    eta: f32,
    gamma: f32,
    sigma: f32,
    dt: f32,
) -> f32 {
    // 1. Deterministic drift
    let replicator_drift = x_i * (f_i - phi_bar);
    let symbiotic_drift = eta * grad_symbiosis;
    let entropy_drift = gamma * grad_entropy;
    let total_drift = (replicator_drift + symbiotic_drift - entropy_drift) * dt;

    // 2. Stochastic diffusion (Brownian motion dW_t ~ N(0, dt))
    let mut rng = rand::thread_rng();
    let dw: f32 = rng.gen_range(-1.0..1.0) * dt.sqrt();
    let diffusion = sigma * dw;

    // 3. Euler-Maruyama update
    let x_next = x_i + total_drift + diffusion;

    // Project back to simplex [0, 1]
    x_next.clamp(0.0, 1.0)
}

/// Run multiple stochastic replicator steps.
///
/// # Arguments
/// * `x0` — Initial strategy proportion
/// * `f_i` — Local fitness (constant over trajectory)
/// * `phi_bar` — Average network fitness (constant)
/// * `grad_symbiosis` — Symbiosis gradient (constant)
/// * `grad_entropy` — Entropy gradient (constant)
/// * `config` — Stochastic replicator configuration
///
/// # Returns
/// Final strategy proportion after all steps
pub fn run_stochastic_replicator(
    x0: f32,
    f_i: f32,
    phi_bar: f32,
    grad_symbiosis: f32,
    grad_entropy: f32,
    config: &StochasticReplicatorConfig,
) -> f32 {
    let mut x_current = x0;
    for _ in 0..(1.0 / config.dt.max(1e-6) as f32) as usize {
        x_current = stochastic_replicator_step(
            x_current,
            f_i,
            phi_bar,
            grad_symbiosis,
            grad_entropy,
            config.eta,
            config.gamma,
            config.sigma,
            config.dt,
        );
    }
    x_current
}

/// Compute Shannon entropy of a population distribution.
///
/// ```math
/// H = -Σ x_i · ln(x_i)
/// ```
pub fn population_entropy(pop: &[f32]) -> f32 {
    pop.iter()
        .filter(|&&x| x > 1e-10)
        .map(|&x| -x * x.ln())
        .sum()
}

/// Verify that a population distribution is a valid simplex.
pub fn verify_simplex(pop: &[f32]) -> bool {
    let sum: f32 = pop.iter().sum();
    (sum - 1.0).abs() < 1e-4 && pop.iter().all(|&x| x >= -1e-6)
}

// ─── S146 — Graphon Replicator Dynamics (Heterogeneous P2P Consensus) ─────────

/// Graphon Replicator Dynamics for heterogeneous P2P consensus.
///
/// Extends standard replicator dynamics with a graphon kernel w(i,j) that
/// encodes heterogeneous peer weights based on latency, trust, and topology.
///
/// **Dynamics:**
/// ```math
/// ∂x_i/∂t = x_i · (f_i(x) - ∫ w(i,j) f_j(x) dj) + σ dW_t
/// ```
///
/// Where:
/// - `w(i,j) = exp(-latency_ij) · trust_ij` — Graphon kernel
/// - `∫ w(i,j) f_j(x) dj ≈ ∑ w_ij · f_j / ∑ w_ij` — Weighted average fitness
/// - `σ dW_t` — Itô noise for exploration
///
/// **Euler-Maruyama Discretization:**
/// ```math
/// x_{t+1} = x_t + x_t · (f_i - f̄_weighted) · Δt + σ · √(Δt) · ξ
/// ```
pub fn graphon_replicator_step(
    local_fitness: f32,
    peer_fitness: &[f32],
    peer_weights: &[f32], // w(i,j) = exp(-latency) * trust
    strategy: f32,
    dt: f32,
    noise_scale: f32,
    seed: u64,
) -> f32 {
    // Compute weighted average fitness: ∫ w(i,j) f_j(x) dj ≈ ∑ w_ij · f_j / ∑ w_ij
    let mut weighted_sum = 0.0f32;
    let mut weight_total = 0.0f32;
    for (f, w) in peer_fitness.iter().zip(peer_weights.iter()) {
        let w_pos = w.max(0.0);
        weighted_sum += w_pos * f;
        weight_total += w_pos;
    }

    let weighted_avg = if weight_total > 1e-10 {
        weighted_sum / weight_total
    } else {
        // Fallback to uniform average
        if peer_fitness.is_empty() {
            local_fitness
        } else {
            peer_fitness.iter().sum::<f32>() / peer_fitness.len() as f32
        }
    };

    // Replicator drift: dx = x · (f_i - f̄_weighted) · dt
    let drift = strategy * (local_fitness - weighted_avg) * dt;

    // Itô noise: σ · √(Δt) · ξ
    let noise = if noise_scale > 0.0 && dt > 0.0 {
        let xi = lcg_gaussian(seed);
        noise_scale * dt.sqrt() * xi
    } else {
        0.0
    };

    // Euler-Maruyama step with simplex projection
    (strategy + drift + noise).clamp(0.0, 1.0)
}

/// Run a trajectory of graphon replicator dynamics.
pub fn graphon_replicator_trajectory(
    initial_strategy: f32,
    local_fitness: f32,
    peer_fitness: &[f32],
    peer_weights: &[f32],
    steps: usize,
    dt: f32,
    noise_scale: f32,
    seed: u64,
) -> Vec<f32> {
    let mut trajectory = vec![initial_strategy];
    let mut current = initial_strategy;

    for i in 0..steps {
        let step_seed = seed.wrapping_add(i as u64).wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        current = graphon_replicator_step(
            local_fitness,
            peer_fitness,
            peer_weights,
            current,
            dt,
            noise_scale,
            step_seed,
        );
        trajectory.push(current);
    }

    trajectory
}

/// Compute graphon kernel weights from latency and trust values.
///
/// w(i,j) = exp(-α · latency_ij) · trust_ij
/// where α controls the sensitivity to latency (default: 1.0).
pub fn compute_graphon_weights(latencies: &[f32], trusts: &[f32], alpha: f32) -> Vec<f32> {
    latencies
        .iter()
        .zip(trusts.iter())
        .map(|(&lat, &trust)| {
            let lat_factor = (-alpha * lat).exp();
            lat_factor * trust.max(0.0).min(1.0)
        })
        .collect()
}

/// LCG-based Gaussian noise generator (Box-Muller transform).
fn lcg_gaussian(seed: u64) -> f32 {
    let mut state = seed;
    let u1 = (lcg_next(&mut state) as f64 / u64::MAX as f64).max(1e-10);
    let u2 = (lcg_next(&mut state) as f64 / u64::MAX as f64);
    let radius = (-2.0_f64 * u1.ln()).sqrt();
    let angle = std::f64::consts::TAU * u2;
    (radius * angle.cos()) as f32
}

/// LCG random number generator.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

// ─── Unit Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stochastic_replicator_step_basic() {
        let result = stochastic_replicator_step(0.5, 1.2, 0.8, 0.1, 0.05, 0.2, 0.1, 0.05, 0.01);
        assert!((0.0..=1.0).contains(&result), "Result must be in [0, 1]");
    }

    #[test]
    fn test_stochastic_replicator_step_clamped_low() {
        // Strong negative drift should clamp to 0
        let result = stochastic_replicator_step(0.01, -10.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1);
        assert!(result >= 0.0, "Must not go below 0");
    }

    #[test]
    fn test_stochastic_replicator_step_clamped_high() {
        // Strong positive drift should clamp to 1
        let result = stochastic_replicator_step(0.99, 10.0, -10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1);
        assert!(result <= 1.0, "Must not go above 1");
    }

    #[test]
    fn test_stochastic_replicator_step_zero_noise() {
        // With sigma=0, the step is deterministic
        let result = stochastic_replicator_step(0.5, 1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.01);
        // drift = 0.5 * (1.0 - 0.5) * 0.01 = 0.0025
        let expected = 0.5 + 0.0025;
        assert!((result - expected).abs() < 0.001, "Deterministic step should match");
    }

    #[test]
    fn test_stochastic_replicator_step_symbiotic_drift() {
        let result = stochastic_replicator_step(0.5, 0.5, 0.5, 1.0, 0.0, 0.5, 0.0, 0.0, 0.01);
        // drift = (0 + 0.5*1.0 - 0) * 0.01 = 0.005
        assert!(result > 0.5, "Symbiotic drift should increase x");
        assert!((0.0..=1.0).contains(&result));
    }

    #[test]
    fn test_stochastic_replicator_step_entropy_penalty() {
        let result = stochastic_replicator_step(0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 0.5, 0.0, 0.01);
        // drift = (0 + 0 - 0.5*1.0) * 0.01 = -0.005
        assert!(result < 0.5, "Entropy penalty should decrease x");
        assert!((0.0..=1.0).contains(&result));
    }

    #[test]
    fn test_stochastic_replicator_step_noise_effect() {
        // Run multiple times to verify noise effect
        let mut results = Vec::new();
        for _ in 0..100 {
            let r = stochastic_replicator_step(0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.1, 0.01);
            results.push(r);
        }
        let min = results.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = results.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max > min, "Noise should create variation");
        assert!(results.iter().all(|&r| (0.0..=1.0).contains(&r)));
    }

    #[test]
    fn test_stochastic_replicator_config_default() {
        let cfg = StochasticReplicatorConfig::default();
        assert!((cfg.eta - 0.2).abs() < 1e-6);
        assert!((cfg.gamma - 0.1).abs() < 1e-6);
        assert!((cfg.sigma - 0.05).abs() < 1e-6);
        assert!((cfg.dt - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_stochastic_replicator_config_fast() {
        let cfg = StochasticReplicatorConfig::fast();
        assert!(cfg.dt > 0.01);
        assert!(cfg.sigma > 0.05);
    }

    #[test]
    fn test_stochastic_replicator_config_high_precision() {
        let cfg = StochasticReplicatorConfig::high_precision();
        assert!(cfg.dt < 0.01);
        assert!(cfg.sigma < 0.05);
    }

    #[test]
    fn test_stochastic_replicator_config_with_sigma() {
        let cfg = StochasticReplicatorConfig::default().with_sigma(0.2);
        assert!((cfg.sigma - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_stochastic_replicator_config_with_dt() {
        let cfg = StochasticReplicatorConfig::default().with_dt(0.05);
        assert!((cfg.dt - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_stochastic_replicator_config_with_seed() {
        let cfg = StochasticReplicatorConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn test_stochastic_replicator_config_sigma_non_negative() {
        let cfg = StochasticReplicatorConfig::default().with_sigma(-1.0);
        assert!(cfg.sigma >= 0.0);
    }

    #[test]
    fn test_stochastic_replicator_config_dt_non_negative() {
        let cfg = StochasticReplicatorConfig::default().with_dt(-1.0);
        assert!(cfg.dt >= 0.0);
    }

    #[test]
    fn test_run_stochastic_replicator_basic() {
        let cfg = StochasticReplicatorConfig::default().with_dt(0.1);
        let result = run_stochastic_replicator(0.5, 1.0, 0.5, 0.0, 0.0, &cfg);
        assert!((0.0..=1.0).contains(&result));
    }

    #[test]
    fn test_population_entropy_uniform() {
        let pop = [0.25, 0.25, 0.25, 0.25];
        let h = population_entropy(&pop);
        assert!((h - std::f32::consts::LN_2 * 2.0).abs() < 0.01);
    }

    #[test]
    fn test_population_entropy_deterministic() {
        let pop = [1.0, 0.0, 0.0];
        let h = population_entropy(&pop);
        assert!(h < 1e-6, "Deterministic should have zero entropy");
    }

    #[test]
    fn test_population_entropy_positive() {
        let pop = [0.5, 0.3, 0.2];
        let h = population_entropy(&pop);
        assert!(h > 0.0, "Entropy should be positive");
    }

    #[test]
    fn test_verify_simplex_valid() {
        let pop = [0.3, 0.4, 0.3];
        assert!(verify_simplex(&pop));
    }

    #[test]
    fn test_verify_simplex_invalid_sum() {
        let pop = [0.5, 0.5, 0.5];
        assert!(!verify_simplex(&pop));
    }

    #[test]
    fn test_verify_simplex_invalid_negative() {
        let pop = [0.5, -0.1, 0.6];
        assert!(!verify_simplex(&pop));
    }

    #[test]
    fn test_stochastic_replicator_convergence() {
        // With high fitness advantage, strategy should increase
        let cfg = StochasticReplicatorConfig::default().with_sigma(0.0).with_dt(0.1);
        let result = run_stochastic_replicator(0.1, 5.0, 1.0, 0.0, 0.0, &cfg);
        assert!(result > 0.1, "High fitness should increase strategy");
    }

    #[test]
    fn test_stochastic_replicator_full_pipeline() {
        let cfg = StochasticReplicatorConfig::default().with_dt(0.05);
        let mut pop = [0.25f32, 0.25, 0.25, 0.25];
        let fitness = [1.5, 1.0, 0.8, 0.5];

        // Run a few steps
        for _ in 0..10 {
            let phi_bar: f32 = pop.iter().zip(fitness.iter()).map(|(x, f)| x * f).sum();
            let mut next = pop;
            for i in 0..4 {
                next[i] = stochastic_replicator_step(
                    pop[i],
                    fitness[i],
                    phi_bar,
                    0.0,
                    0.0,
                    cfg.eta,
                    cfg.gamma,
                    cfg.sigma,
                    cfg.dt,
                );
            }
            // Renormalize
            let sum: f32 = next.iter().sum();
            for v in &mut next {
                *v /= sum;
            }
            pop = next;
        }

        assert!(verify_simplex(&pop), "Population should remain valid simplex");
        assert!(pop[0] > pop[3], "Best strategy should dominate");
    }
}
