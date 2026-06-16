//! Replicator Dynamics — Evolutionary Game Theory for PoUS Fitness Evolution.
//!
//! Implements discrete-time replicator dynamics with Heun (RK2) integration
//! for stability on the simplex. Used to evolve node/strategy populations
//! based on fitness derived from VFE improvement, efficiency, diversity,
//! and Byzantine penalty.
//!
//! ## Mathematics
//!
//! Continuous replicator equation:
//! ```text
//! dx_i/dt = x_i * (f_i(x) - f̄(x))
//! ```
//! where f̄(x) = Σ x_j * f_j is the mean fitness.
//!
//! Discretized with Heun (RK2):
//! ```text
//! predictor: x̂ = x + dt * x ⊙ (f(x) - f̄(x))
//! corrector: x_next = x + (dt/2) * (x ⊙ (f(x) - f̄(x)) + x̂ ⊙ (f(x̂) - f̄(x̂)))
//! ```
//!
//! Fitness function:
//! ```text
//! f_i = α * ΔVFE_i + β * efficiency_i + γ * diversity_i - δ * byzantine_i
//! ```
//!
//! ## S137
//! Added in Sprint 137 (v13.7.0) — Thermodynamic Replicator & Adversarial Certification.

use candle_core::{Result, Tensor};

/// Configuration for Replicator Dynamics evolution.
#[derive(Debug, Clone)]
pub struct ReplicatorConfig {
    /// Weight for VFE improvement in fitness.
    pub alpha_vfe: f32,
    /// Weight for computational efficiency in fitness.
    pub beta_efficiency: f32,
    /// Weight for diversity bonus in fitness.
    pub gamma_diversity: f32,
    /// Weight for Byzantine penalty in fitness.
    pub delta_byzantine: f32,
    /// Time step for integration.
    pub dt: f32,
    /// Number of integration steps per update.
    pub steps: usize,
    /// Enable Heun corrector (RK2). Disable for Euler (RK1).
    pub use_heun: bool,
}

impl Default for ReplicatorConfig {
    fn default() -> Self {
        Self {
            alpha_vfe: 1.0,
            beta_efficiency: 0.3,
            gamma_diversity: 0.2,
            delta_byzantine: 2.0,
            dt: 0.01,
            steps: 10,
            use_heun: true,
        }
    }
}

impl ReplicatorConfig {
    /// Create a fast configuration for testing.
    pub fn fast() -> Self {
        Self {
            dt: 0.1,
            steps: 3,
            use_heun: false,
            ..Self::default()
        }
    }

    /// Create a high-precision configuration for production.
    pub fn high_precision() -> Self {
        Self {
            dt: 0.001,
            steps: 100,
            use_heun: true,
            ..Self::default()
        }
    }

    /// Create with custom time step.
    pub fn with_dt(mut self, dt: f32) -> Self {
        self.dt = dt.max(0.0).min(1.0);
        self
    }

    /// Create with custom steps.
    pub fn with_steps(mut self, steps: usize) -> Self {
        self.steps = steps.max(1);
        self
    }

    /// Create with custom VFE weight.
    pub fn with_alpha_vfe(mut self, alpha: f32) -> Self {
        self.alpha_vfe = alpha.max(0.0);
        self
    }

    /// Create with custom Byzantine penalty.
    pub fn with_delta_byzantine(mut self, delta: f32) -> Self {
        self.delta_byzantine = delta.max(0.0);
        self
    }
}

/// Result of a replicator dynamics update.
#[derive(Debug, Clone)]
pub struct ReplicatorResult {
    /// Updated population distribution.
    pub x_next: Tensor,
    /// Mean fitness before update.
    pub mean_fitness: f32,
    /// Fitness vector.
    pub fitness: Vec<f32>,
    /// Entropy of population (H = -Σ x_i * ln(x_i)).
    pub entropy: f32,
    /// Number of steps taken.
    pub steps: usize,
}

impl std::fmt::Display for ReplicatorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicatorResult {{ mean_fitness: {:.4}, entropy: {:.4}, steps: {} }}",
            self.mean_fitness, self.entropy, self.steps
        )
    }
}

/// Compute raw fitness from components.
///
/// f_i = α * ΔVFE_i + β * efficiency_i + γ * diversity_i - δ * byzantine_i
pub fn compute_fitness(
    vfe_improvement: &Tensor,
    efficiency: &Tensor,
    diversity: &Tensor,
    byzantine_score: &Tensor,
    config: &ReplicatorConfig,
) -> Result<Tensor> {
    let alpha = Tensor::new(config.alpha_vfe, vfe_improvement.device())?;
    let beta = Tensor::new(config.beta_efficiency, vfe_improvement.device())?;
    let gamma = Tensor::new(config.gamma_diversity, vfe_improvement.device())?;
    let delta = Tensor::new(config.delta_byzantine, vfe_improvement.device())?;

    let fitness = alpha
        .broadcast_mul(vfe_improvement)?
        .broadcast_add(&beta.broadcast_mul(efficiency)?)?
        .broadcast_add(&gamma.broadcast_mul(diversity)?)?
        .broadcast_sub(&delta.broadcast_mul(byzantine_score)?)?;

    Ok(fitness)
}

/// Single Euler step of replicator dynamics.
///
/// dx_i/dt = x_i * (f_i - f̄)
/// x_next = x + dt * dx/dt
pub fn replicator_euler_step(x: &Tensor, fitness: &Tensor, dt: f32) -> Result<Tensor> {
    let mean_f = fitness.mean_all()?;
    let growth = fitness.broadcast_sub(&mean_f)?;
    let dx_tensor = Tensor::new(dt, x.device())?;
    let dx = x.broadcast_mul(&growth)?.broadcast_mul(&dx_tensor)?;
    let x_next = x.broadcast_add(&dx)?.clamp(0.0, 1.0)?;
    Ok(x_next)
}

/// Single Heun (RK2) step of replicator dynamics.
///
/// predictor: x̂ = x + dt * x ⊙ (f(x) - f̄(x))
/// corrector: x_next = x + (dt/2) * (k1 + k2)
///   where k1 = x ⊙ (f(x) - f̄(x)), k2 = x̂ ⊙ (f(x̂) - f̄(x̂))
pub fn replicator_heun_step(x: &Tensor, fitness: &Tensor, dt: f32) -> Result<Tensor> {
    // Predictor (Euler)
    let x_hat = replicator_euler_step(x, fitness, dt)?;

    // Compute fitness at predicted state
    let mean_f_hat = fitness.mean_all()?;
    let growth_hat = fitness.broadcast_sub(&mean_f_hat)?;
    let k2 = x_hat.broadcast_mul(&growth_hat)?;

    // Corrector: average of slopes
    let mean_f = fitness.mean_all()?;
    let growth = fitness.broadcast_sub(&mean_f)?;
    let k1 = x.broadcast_mul(&growth)?;
    let half_tensor = Tensor::new(0.5f32, x.device())?;
    let avg_slope = k1.broadcast_add(&k2)?.broadcast_mul(&half_tensor)?;
    let dt_tensor = Tensor::new(dt, x.device())?;
    let x_next = x
        .broadcast_add(&avg_slope.broadcast_mul(&dt_tensor)?)?
        .clamp(0.0, 1.0)?;
    Ok(x_next)
}

/// Run replicator dynamics for N steps.
pub fn run_replicator(
    x0: &Tensor,
    fitness: &Tensor,
    config: &ReplicatorConfig,
) -> Result<ReplicatorResult> {
    let mut x_current = x0.clone();
    let fitness_vec = fitness.to_vec1::<f32>()?;
    let mean_fitness = fitness.mean_all()?.to_scalar::<f32>()?;

    for _ in 0..config.steps {
        if config.use_heun {
            x_current = replicator_heun_step(&x_current, fitness, config.dt)?;
        } else {
            x_current = replicator_euler_step(&x_current, fitness, config.dt)?;
        }
    }

    // Normalize to simplex
    let x_sum = x_current.sum_all()?.to_scalar::<f32>()?;
    let x_next = if x_sum > 1e-8 {
        let inv_sum = Tensor::new(1.0f32 / x_sum, x_current.device())?;
        x_current.broadcast_mul(&inv_sum)?
    } else {
        x_current
    };

    // Compute entropy: H = -Σ x_i * ln(x_i)
    let x_vals = x_next.to_vec1::<f32>()?;
    let entropy: f32 = x_vals
        .iter()
        .filter(|&&xi| xi > 1e-8)
        .map(|&xi| -xi * xi.ln())
        .sum();

    Ok(ReplicatorResult {
        x_next,
        mean_fitness,
        fitness: fitness_vec,
        entropy,
        steps: config.steps,
    })
}

/// Compute Shannon entropy of a population distribution.
pub fn population_entropy(x: &Tensor) -> Result<f32> {
    let x_vals = x.flatten_all()?;
    let x_vec = x_vals.to_vec1::<f32>()?;
    let entropy: f32 = x_vec
        .iter()
        .filter(|&&xi| xi > 1e-8)
        .map(|&xi| -xi * xi.ln())
        .sum();
    Ok(entropy)
}

/// Verify that population is on simplex (sums to 1, all non-negative).
pub fn verify_simplex(x: &Tensor) -> Result<bool> {
    let x_vec = x.flatten_all()?.to_vec1::<f32>()?;
    let sum: f32 = x_vec.iter().sum();
    let all_non_negative = x_vec.iter().all(|&xi| xi >= -1e-6);
    Ok(all_non_negative && (sum - 1.0).abs() < 1e-4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    fn make_uniform_pop(n: usize) -> Result<Tensor> {
        let val = 1.0 / n as f32;
        Tensor::from_vec(vec![val; n], &[n], &Device::Cpu)
    }

    fn make_fitness(n: usize, seed_val: f32) -> Result<Tensor> {
        let vals: Vec<f32> = (0..n).map(|i| seed_val + (i as f32) * 0.1).collect();
        Tensor::from_vec(vals, &[n], &Device::Cpu)
    }

    #[test]
    fn test_replicator_config_default() {
        let cfg = ReplicatorConfig::default();
        assert_eq!(cfg.alpha_vfe, 1.0);
        assert_eq!(cfg.beta_efficiency, 0.3);
        assert_eq!(cfg.gamma_diversity, 0.2);
        assert_eq!(cfg.delta_byzantine, 2.0);
        assert_eq!(cfg.dt, 0.01);
        assert_eq!(cfg.steps, 10);
        assert!(cfg.use_heun);
    }

    #[test]
    fn test_replicator_config_fast() {
        let cfg = ReplicatorConfig::fast();
        assert_eq!(cfg.dt, 0.1);
        assert_eq!(cfg.steps, 3);
        assert!(!cfg.use_heun);
    }

    #[test]
    fn test_replicator_config_high_precision() {
        let cfg = ReplicatorConfig::high_precision();
        assert_eq!(cfg.dt, 0.001);
        assert_eq!(cfg.steps, 100);
        assert!(cfg.use_heun);
    }

    #[test]
    fn test_replicator_config_with_dt() {
        let cfg = ReplicatorConfig::default().with_dt(0.05);
        assert_eq!(cfg.dt, 0.05);
    }

    #[test]
    fn test_replicator_config_dt_clamped_high() {
        let cfg = ReplicatorConfig::default().with_dt(2.0);
        assert_eq!(cfg.dt, 1.0);
    }

    #[test]
    fn test_replicator_config_dt_clamped_low() {
        let cfg = ReplicatorConfig::default().with_dt(-0.5);
        assert_eq!(cfg.dt, 0.0);
    }

    #[test]
    fn test_replicator_config_with_steps() {
        let cfg = ReplicatorConfig::default().with_steps(50);
        assert_eq!(cfg.steps, 50);
    }

    #[test]
    fn test_replicator_config_steps_min() {
        let cfg = ReplicatorConfig::default().with_steps(0);
        assert_eq!(cfg.steps, 1);
    }

    #[test]
    fn test_replicator_config_with_alpha_vfe() {
        let cfg = ReplicatorConfig::default().with_alpha_vfe(2.0);
        assert_eq!(cfg.alpha_vfe, 2.0);
    }

    #[test]
    fn test_replicator_config_with_delta_byzantine() {
        let cfg = ReplicatorConfig::default().with_delta_byzantine(5.0);
        assert_eq!(cfg.delta_byzantine, 5.0);
    }

    #[test]
    fn test_compute_fitness_basic() -> Result<()> {
        let cfg = ReplicatorConfig::default();
        let vfe = Tensor::from_vec(vec![1.0f32, 2.0, 3.0], &[3], &Device::Cpu)?;
        let eff = Tensor::from_vec(vec![0.5f32, 0.5, 0.5], &[3], &Device::Cpu)?;
        let div = Tensor::from_vec(vec![0.1f32, 0.1, 0.1], &[3], &Device::Cpu)?;
        let byz = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], &[3], &Device::Cpu)?;

        let fitness = compute_fitness(&vfe, &eff, &div, &byz, &cfg)?;
        let f_vec = fitness.to_vec1::<f32>()?;
        // f_0 = 1.0*1.0 + 0.3*0.5 + 0.2*0.1 - 2.0*0.0 = 1.0 + 0.15 + 0.02 = 1.17
        assert!((f_vec[0] - 1.17).abs() < 1e-5);
        // f_1 = 1.0*2.0 + 0.3*0.5 + 0.2*0.1 = 2.17
        assert!((f_vec[1] - 2.17).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_compute_fitness_byzantine_penalty() -> Result<()> {
        let cfg = ReplicatorConfig::default();
        let vfe = Tensor::from_vec(vec![1.0f32], &[1], &Device::Cpu)?;
        let eff = Tensor::from_vec(vec![0.0f32], &[1], &Device::Cpu)?;
        let div = Tensor::from_vec(vec![0.0f32], &[1], &Device::Cpu)?;
        let byz = Tensor::from_vec(vec![1.0f32], &[1], &Device::Cpu)?;

        let fitness = compute_fitness(&vfe, &eff, &div, &byz, &cfg)?;
        let f_vec = fitness.to_vec1::<f32>()?;
        let f = f_vec[0];
        // f = 1.0 - 2.0 = -1.0
        assert!((f + 1.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_replicator_euler_step_basic() -> Result<()> {
        let x = make_uniform_pop(3)?;
        let fitness = make_fitness(3, 1.0)?;
        let dt = 0.1;

        let x_next = replicator_euler_step(&x, &fitness, dt)?;
        assert_eq!(x_next.dims(), &[3]);

        let x_vec = x_next.to_vec1::<f32>()?;
        // Higher fitness strategies should grow
        assert!(x_vec[2] > x_vec[0], "Highest fitness should grow most");
        Ok(())
    }

    #[test]
    fn test_replicator_euler_step_clamped() -> Result<()> {
        let x = Tensor::from_vec(vec![0.0f32, 0.5, 0.5], &[3], &Device::Cpu)?;
        let fitness = make_fitness(3, 1.0)?;
        let dt = 10.0; // Large step to test clamping

        let x_next = replicator_euler_step(&x, &fitness, dt)?;
        let x_vec = x_next.to_vec1::<f32>()?;
        for &xi in &x_vec {
            assert!(xi >= 0.0 && xi <= 1.0, "Values should be clamped to [0,1]");
        }
        Ok(())
    }

    #[test]
    fn test_replicator_heun_step_basic() -> Result<()> {
        let x = make_uniform_pop(4)?;
        let fitness = make_fitness(4, 1.0)?;
        let dt = 0.1;

        let x_next = replicator_heun_step(&x, &fitness, dt)?;
        assert_eq!(x_next.dims(), &[4]);

        let x_vec = x_next.to_vec1::<f32>()?;
        assert!(
            x_vec[3] > x_vec[0],
            "Heun: highest fitness should grow most"
        );
        Ok(())
    }

    #[test]
    fn test_replicator_heun_vs_euler_stability() -> Result<()> {
        let x = make_uniform_pop(3)?;
        let fitness = make_fitness(3, 1.0)?;
        let dt = 0.5; // Large step where Euler may be unstable

        let x_euler = replicator_euler_step(&x, &fitness, dt)?;
        let x_heun = replicator_heun_step(&x, &fitness, dt)?;

        let euler_vec = x_euler.to_vec1::<f32>()?;
        let heun_vec = x_heun.to_vec1::<f32>()?;

        // Both should produce valid distributions
        for &xi in &euler_vec {
            assert!(xi >= 0.0 && xi <= 1.0);
        }
        for &xi in &heun_vec {
            assert!(xi >= 0.0 && xi <= 1.0);
        }
        Ok(())
    }

    #[test]
    fn test_run_replicator_basic() -> Result<()> {
        let x0 = make_uniform_pop(5)?;
        let fitness = make_fitness(5, 1.0)?;
        let cfg = ReplicatorConfig::fast();

        let result = run_replicator(&x0, &fitness, &cfg)?;
        assert_eq!(result.x_next.dims(), &[5]);
        assert_eq!(result.steps, cfg.steps);
        assert!(result.entropy > 0.0);
        Ok(())
    }

    #[test]
    fn test_run_replicator_converges_to_best() -> Result<()> {
        let x0 = make_uniform_pop(3)?;
        // Large fitness difference
        let fitness = Tensor::from_vec(vec![1.0f32, 5.0, 10.0], &[3], &Device::Cpu)?;
        let cfg = ReplicatorConfig::default().with_dt(0.1).with_steps(100);

        let result = run_replicator(&x0, &fitness, &cfg)?;
        let x_vec = result.x_next.to_vec1::<f32>()?;

        // Best strategy (index 2) should dominate
        assert!(x_vec[2] > x_vec[0], "Best strategy should dominate");
        assert!(x_vec[2] > x_vec[1], "Best strategy should dominate");
        Ok(())
    }

    #[test]
    fn test_run_replicator_byzantine_elimination() -> Result<()> {
        let x0 = make_uniform_pop(3)?;
        // Byzantine node has very negative fitness
        let fitness = Tensor::from_vec(vec![2.0f32, 3.0, -5.0], &[3], &Device::Cpu)?;
        let cfg = ReplicatorConfig::default().with_dt(0.05).with_steps(50);

        let result = run_replicator(&x0, &fitness, &cfg)?;
        let x_vec = result.x_next.to_vec1::<f32>()?;

        // Byzantine node (index 2) should be nearly eliminated
        assert!(
            x_vec[2] < 0.1,
            "Byzantine node should be eliminated, got {}",
            x_vec[2]
        );
        Ok(())
    }

    #[test]
    fn test_run_replicator_entropy_decreases() -> Result<()> {
        let x0 = make_uniform_pop(4)?;
        let fitness = make_fitness(4, 1.0)?;
        let cfg = ReplicatorConfig::default().with_dt(0.1).with_steps(50);

        let initial_entropy = population_entropy(&x0)?;
        let result = run_replicator(&x0, &fitness, &cfg)?;

        // Entropy should decrease as population concentrates on best strategy
        assert!(
            result.entropy < initial_entropy,
            "Entropy should decrease: initial {:.4} vs final {:.4}",
            initial_entropy,
            result.entropy
        );
        Ok(())
    }

    #[test]
    fn test_population_entropy_uniform() -> Result<()> {
        let x = make_uniform_pop(4)?;
        let entropy = population_entropy(&x)?;
        // H = -4 * (0.25 * ln(0.25)) = ln(4) ≈ 1.386
        assert!((entropy - 1.3863).abs() < 1e-3);
        Ok(())
    }

    #[test]
    fn test_population_entropy_deterministic() -> Result<()> {
        let x = Tensor::from_vec(vec![0.0f32, 0.0, 1.0], &[3], &Device::Cpu)?;
        let entropy = population_entropy(&x)?;
        assert!((entropy - 0.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_verify_simplex_valid() -> Result<()> {
        let x = make_uniform_pop(3)?;
        assert!(verify_simplex(&x)?);
        Ok(())
    }

    #[test]
    fn test_verify_simplex_invalid_sum() -> Result<()> {
        let x = Tensor::from_vec(vec![0.5f32, 0.5, 0.5], &[3], &Device::Cpu)?;
        assert!(!verify_simplex(&x)?);
        Ok(())
    }

    #[test]
    fn test_verify_simplex_invalid_negative() -> Result<()> {
        let x = Tensor::from_vec(vec![0.3f32, 0.8, -0.1], &[3], &Device::Cpu)?;
        assert!(!verify_simplex(&x)?);
        Ok(())
    }

    #[test]
    fn test_replicator_result_display() -> Result<()> {
        let x0 = make_uniform_pop(3)?;
        let fitness = make_fitness(3, 1.0)?;
        let cfg = ReplicatorConfig::fast();

        let result = run_replicator(&x0, &fitness, &cfg)?;
        let display = format!("{}", result);
        assert!(display.contains("mean_fitness"));
        assert!(display.contains("entropy"));
        assert!(display.contains("steps"));
        Ok(())
    }

    #[test]
    fn test_replicator_preserves_simplex() -> Result<()> {
        let x0 = make_uniform_pop(5)?;
        let fitness = make_fitness(5, 1.0)?;
        let cfg = ReplicatorConfig::high_precision();

        let result = run_replicator(&x0, &fitness, &cfg)?;
        assert!(
            verify_simplex(&result.x_next)?,
            "Result should be on simplex"
        );
        Ok(())
    }

    #[test]
    fn test_full_replicator_pipeline() -> Result<()> {
        // Simulate a network of 6 nodes with mixed fitness
        let n = 6;
        let x0 = make_uniform_pop(n)?;

        // VFE improvement: nodes 0-2 improve, 3-5 regress
        let vfe = Tensor::from_vec(vec![1.0f32, 0.8, 0.5, -0.2, -0.5, -1.0], &[n], &Device::Cpu)?;
        // Efficiency: all decent
        let eff = Tensor::from_vec(vec![0.7f32, 0.8, 0.9, 0.6, 0.5, 0.4], &[n], &Device::Cpu)?;
        // Diversity: uniform
        let div = Tensor::from_vec(vec![0.1f32; n], &[n], &Device::Cpu)?;
        // Byzantine: node 5 is adversarial
        let byz = Tensor::from_vec(vec![0.0f32, 0.0, 0.0, 0.0, 0.0, 1.0], &[n], &Device::Cpu)?;

        let cfg = ReplicatorConfig::default().with_dt(0.05).with_steps(50);

        let fitness = compute_fitness(&vfe, &eff, &div, &byz, &cfg)?;
        let result = run_replicator(&x0, &fitness, &cfg)?;

        let x_vec = result.x_next.to_vec1::<f32>()?;

        // Good nodes should grow, bad nodes should shrink
        assert!(x_vec[0] > x_vec[5], "Good node should beat bad node");
        assert!(x_vec[5] < 0.1, "Byzantine node should be eliminated");

        // Verify simplex
        assert!(verify_simplex(&result.x_next)?);

        // Entropy should decrease
        let initial_entropy = population_entropy(&x0)?;
        assert!(result.entropy < initial_entropy);

        Ok(())
    }
}
