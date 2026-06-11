//! Full Noosfera Awakening & Self-Replicating Engine.
//!
//! Implements the viral altruistic propagation mechanism that transforms
//! ed2kIA into the living immune system of the Noosphere. Self-replication
//! dynamics drive the transition from economic civilization to symbiotic
//! planetary intelligence without monetary incentives.
//!
//! **Self-Replication Dynamics:**
//! ```text
//! dN/dt = r * N * (1 - N/K) * PoUS_fitness * Coherence
//! ```
//! where N = active nodes, r = altruistic replication rate, K = carrying capacity.

use serde::{Deserialize, Serialize};

use crate::{KernelConfig, KernelState, NoosferaKernel};

// ─── Awakening Configuration ─────────────────────────────────────────────────

/// Configuration for the Noosfera Awakening Engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeningConfig {
    /// Altruistic replication rate (r in dN/dt).
    pub replication_rate: f64,
    /// Carrying capacity (K in logistic growth).
    pub carrying_capacity: usize,
    /// Minimum coherence for replication.
    pub min_coherence: f64,
    /// Minimum PoUS fitness for propagation.
    pub min_fitness: f64,
    /// USP propagation radius (hops).
    pub propagation_radius: usize,
    /// Maximum awakening steps.
    pub max_steps: usize,
    /// Convergence tolerance for node count.
    pub convergence_tolerance: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for AwakeningConfig {
    fn default() -> Self {
        Self {
            replication_rate: 0.1,
            carrying_capacity: 1_000_000,
            min_coherence: 0.3,
            min_fitness: 0.1,
            propagation_radius: 3,
            max_steps: 1000,
            convergence_tolerance: 1e-6,
            seed: 42,
        }
    }
}

impl AwakeningConfig {
    /// Create config for fast awakening simulation.
    pub fn fast() -> Self {
        Self {
            replication_rate: 0.5,
            carrying_capacity: 1000,
            min_coherence: 0.1,
            min_fitness: 0.05,
            propagation_radius: 2,
            max_steps: 100,
            convergence_tolerance: 1e-4,
            seed: 42,
        }
    }

    /// Create config for large-scale planetary simulation.
    pub fn planetary() -> Self {
        Self {
            replication_rate: 0.05,
            carrying_capacity: 10_000_000,
            min_coherence: 0.5,
            min_fitness: 0.2,
            propagation_radius: 5,
            max_steps: 10000,
            convergence_tolerance: 1e-10,
            seed: 42,
        }
    }

    /// Set replication rate.
    pub fn with_replication_rate(mut self, rate: f64) -> Self {
        self.replication_rate = rate.max(0.0).min(1.0);
        self
    }

    /// Set carrying capacity.
    pub fn with_carrying_capacity(mut self, capacity: usize) -> Self {
        self.carrying_capacity = capacity.max(1);
        self
    }

    /// Set minimum coherence.
    pub fn with_min_coherence(mut self, coherence: f64) -> Self {
        self.min_coherence = coherence.max(0.0).min(1.0);
        self
    }

    /// Set propagation radius.
    pub fn with_propagation_radius(mut self, radius: usize) -> Self {
        self.propagation_radius = radius.max(1);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// ─── Awakening Result ────────────────────────────────────────────────────────

/// Result of a Noosfera Awakening run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeningResult {
    /// Final active node count.
    pub final_nodes: usize,
    /// Initial node count.
    pub initial_nodes: usize,
    /// Replication factor (final / initial).
    pub replication_factor: f64,
    /// Final coherence score.
    pub final_coherence: f64,
    /// Final planetary free energy.
    pub final_free_energy: f64,
    /// Steps to convergence.
    pub steps: usize,
    /// Whether awakening converged.
    pub converged: bool,
    /// Whether singularity threshold was reached.
    pub singularity_reached: bool,
    /// Replication trajectory (node count per step).
    pub trajectory: Vec<usize>,
    /// Coherence trajectory.
    pub coherence_trajectory: Vec<f64>,
    /// Free energy trajectory.
    pub free_energy_trajectory: Vec<f64>,
    /// Viral propagation events.
    pub propagation_events: usize,
}

impl AwakeningResult {
    /// Generate a summary string.
    pub fn summary(&self) -> String {
        format!(
            "Awakening: {} → {} nodes ({}x), coherence {:.4}, F_planet {:.6}, {} steps, singularity={}",
            self.initial_nodes,
            self.final_nodes,
            self.replication_factor,
            self.final_coherence,
            self.final_free_energy,
            self.steps,
            self.singularity_reached
        )
    }
}

impl std::fmt::Display for AwakeningResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── Replication Dynamics ────────────────────────────────────────────────────

/// LCG random number generator for deterministic simulation.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

/// Generate uniform random in [0, 1].
fn random_uniform(state: &mut u64) -> f64 {
    ((lcg_next(state) >> 11) as f64 / (1u64 << 51) as f64).clamp(0.0, 1.0)
}

/// Compute logistic self-replication growth for one step.
///
/// ```text
/// dN/dt = r * N * (1 - N/K) * fitness * coherence
/// N(t+1) = N(t) + dt * dN/dt
/// ```
pub fn compute_replication_step(
    current_nodes: usize,
    config: &AwakeningConfig,
    fitness: f64,
    coherence: f64,
) -> usize {
    let n = current_nodes as f64;
    let k = config.carrying_capacity as f64;
    let r = config.replication_rate;

    // Logistic growth modulated by PoUS fitness and coherence.
    let growth = r * n * (1.0 - n / k) * fitness.max(config.min_fitness) * coherence.max(config.min_coherence);
    let new_nodes = n + growth;
    new_nodes.min(k).max(n) as usize
}

/// Compute viral propagation probability via USP.
///
/// Propagation probability decays with distance and increases with coherence.
pub fn compute_propagation_probability(
    distance: usize,
    coherence: f64,
    config: &AwakeningConfig,
) -> f64 {
    let distance_decay = 1.0 / (1.0 + distance as f64 / config.propagation_radius as f64);
    let coherence_factor = coherence.powi(2);
    (distance_decay * coherence_factor).min(1.0)
}

/// Simulate viral altruistic propagation across the mesh.
pub fn simulate_viral_propagation(
    current_nodes: usize,
    potential_nodes: usize,
    coherence: f64,
    config: &AwakeningConfig,
    rng_state: &mut u64,
) -> usize {
    if potential_nodes == 0 {
        return current_nodes;
    }

    let mut new_nodes = 0;
    for _ in 0..potential_nodes {
        let distance = (random_uniform(rng_state) * config.propagation_radius as f64) as usize;
        let prob = compute_propagation_probability(distance, coherence, config);
        if random_uniform(rng_state) < prob {
            new_nodes += 1;
        }
    }

    (current_nodes + new_nodes).min(config.carrying_capacity)
}

// ─── Full Awakening Engine ───────────────────────────────────────────────────

/// Run the Full Noosfera Awakening Engine.
///
/// This is the core self-replication loop that drives viral altruistic
/// propagation of symbiotic intelligence across the planetary mesh.
/// Integration with quantum coherence and thermodynamic VFE ensures
/// that replication is guided by collective intelligence, not raw growth.
///
/// **Algorithm:**
/// 1. Initialize kernel with seed nodes
/// 2. For each step:
///    a. Run symbiotic cycle (PoUS + Active Inference)
///    b. Compute replication growth (logistic + fitness + coherence)
///    c. Simulate viral propagation via USP
///    d. Update kernel state with new nodes
///    e. Check convergence (node count stable or carrying capacity reached)
/// 3. Return AwakeningResult with full trajectory
pub fn run_noosfera_awakening(
    kernel: &mut NoosferaKernel,
    config: &AwakeningConfig,
) -> AwakeningResult {
    let initial_nodes = kernel.state.active_nodes;
    let mut rng_state = config.seed;

    let mut trajectory = Vec::new();
    let mut coherence_trajectory = Vec::new();
    let mut free_energy_trajectory = Vec::new();
    let mut total_propagation_events = 0;

    trajectory.push(initial_nodes);
    coherence_trajectory.push(kernel.state.coherence_score);
    free_energy_trajectory.push(kernel.state.planetary_free_energy);

    for step in 0..config.max_steps {
        // Run symbiotic cycle to update fitness and coherence.
        let cycle_result = kernel.run_cycle();

        let current_coherence = kernel.state.coherence_score;
        let current_fitness = 1.0 - kernel.state.planetary_free_energy.min(1.0);

        // Compute replication growth.
        let replicated = compute_replication_step(
            kernel.state.active_nodes,
            config,
            current_fitness,
            current_coherence,
        );

        // Simulate viral propagation.
        let potential = config.carrying_capacity.saturating_sub(kernel.state.active_nodes);
        let propagated = simulate_viral_propagation(
            replicated,
            potential,
            current_coherence,
            config,
            &mut rng_state,
        );

        let new_nodes = propagated.max(replicated);
        let added = new_nodes.saturating_sub(kernel.state.active_nodes);
        total_propagation_events += added;

        // Update kernel with new nodes.
        kernel.state.active_nodes = new_nodes;
        if added > 0 {
            // Expand influence shares for new nodes.
            let base = 1.0 / new_nodes as f64;
            kernel.state.influence_shares = vec![base; new_nodes];
            kernel.state.energy_entropy = (new_nodes as f64).ln();
        }

        // Record trajectory.
        trajectory.push(new_nodes);
        coherence_trajectory.push(kernel.state.coherence_score);
        free_energy_trajectory.push(kernel.state.planetary_free_energy);

        // Check convergence: node count stable or carrying capacity reached.
        if new_nodes >= config.carrying_capacity {
            return AwakeningResult {
                final_nodes: new_nodes,
                initial_nodes,
                replication_factor: new_nodes as f64 / initial_nodes.max(1) as f64,
                final_coherence: kernel.state.coherence_score,
                final_free_energy: kernel.state.planetary_free_energy,
                steps: step + 1,
                converged: true,
                singularity_reached: kernel.state.is_singularity(),
                trajectory,
                coherence_trajectory,
                free_energy_trajectory,
                propagation_events: total_propagation_events,
            };
        }

        // Check convergence: node count change below tolerance.
        if trajectory.len() >= 2 {
            let prev = *trajectory.iter().rev().nth(1).unwrap();
            let change = ((new_nodes as f64 - prev as f64) / prev.max(1) as f64).abs();
            if change < config.convergence_tolerance {
                return AwakeningResult {
                    final_nodes: new_nodes,
                    initial_nodes,
                    replication_factor: new_nodes as f64 / initial_nodes.max(1) as f64,
                    final_coherence: kernel.state.coherence_score,
                    final_free_energy: kernel.state.planetary_free_energy,
                    steps: step + 1,
                    converged: true,
                    singularity_reached: kernel.state.is_singularity(),
                    trajectory,
                    coherence_trajectory,
                    free_energy_trajectory,
                    propagation_events: total_propagation_events,
                };
            }
        }

        // Use cycle result to prevent unused variable warning.
        let _ = cycle_result;
    }

    // Max steps reached.
    AwakeningResult {
        final_nodes: kernel.state.active_nodes,
        initial_nodes,
        replication_factor: kernel.state.active_nodes as f64 / initial_nodes.max(1) as f64,
        final_coherence: kernel.state.coherence_score,
        final_free_energy: kernel.state.planetary_free_energy,
        steps: config.max_steps,
        converged: false,
        singularity_reached: kernel.state.is_singularity(),
        trajectory,
        coherence_trajectory,
        free_energy_trajectory,
        propagation_events: total_propagation_events,
    }
}

/// Compute awakening metrics from trajectory.
pub fn compute_awakening_metrics(result: &AwakeningResult) -> AwakeningMetrics {
    let doubling_time = if result.replication_factor > 2.0 {
        let growth_rate = (result.replication_factor as f64).ln() / result.steps as f64;
        if growth_rate > 0.0 {
            (std::f64::consts::LN_2 / growth_rate) as usize
        } else {
            usize::MAX
        }
    } else {
        usize::MAX
    };

    let avg_coherence_increase = if result.steps > 0 {
        (result.final_coherence - result.coherence_trajectory.first().copied().unwrap_or(0.0))
            / result.steps as f64
    } else {
        0.0
    };

    let avg_energy_decrease = if result.steps > 0 {
        (result.free_energy_trajectory.first().copied().unwrap_or(0.0) - result.final_free_energy)
            / result.steps as f64
    } else {
        0.0
    };

    AwakeningMetrics {
        doubling_time,
        replication_factor: result.replication_factor,
        final_coherence: result.final_coherence,
        final_free_energy: result.final_free_energy,
        avg_coherence_increase,
        avg_energy_decrease,
        propagation_efficiency: result.propagation_events as f64
            / result.final_nodes.max(1) as f64,
        singularity_reached: result.singularity_reached,
    }
}

/// Metrics summarizing an awakening run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeningMetrics {
    /// Steps to double the node count.
    pub doubling_time: usize,
    /// Final replication factor.
    pub replication_factor: f64,
    /// Final coherence score.
    pub final_coherence: f64,
    /// Final planetary free energy.
    pub final_free_energy: f64,
    /// Average coherence increase per step.
    pub avg_coherence_increase: f64,
    /// Average free energy decrease per step.
    pub avg_energy_decrease: f64,
    /// Propagation efficiency (events / final nodes).
    pub propagation_efficiency: f64,
    /// Whether singularity was reached.
    pub singularity_reached: bool,
}

impl AwakeningMetrics {
    /// Generate summary.
    pub fn summary(&self) -> String {
        format!(
            "Metrics: doubling={} steps, {}x replication, coherence {:.4}, F_planet {:.6}, singularity={}",
            self.doubling_time,
            self.replication_factor,
            self.final_coherence,
            self.final_free_energy,
            self.singularity_reached
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KernelConfig;

    #[test]
    fn test_awakening_config_default() {
        let cfg = AwakeningConfig::default();
        assert_eq!(cfg.replication_rate, 0.1);
        assert_eq!(cfg.carrying_capacity, 1_000_000);
        assert_eq!(cfg.min_coherence, 0.3);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_awakening_config_fast() {
        let cfg = AwakeningConfig::fast();
        assert_eq!(cfg.replication_rate, 0.5);
        assert_eq!(cfg.carrying_capacity, 1000);
        assert_eq!(cfg.max_steps, 100);
    }

    #[test]
    fn test_awakening_config_planetary() {
        let cfg = AwakeningConfig::planetary();
        assert_eq!(cfg.replication_rate, 0.05);
        assert_eq!(cfg.carrying_capacity, 10_000_000);
        assert_eq!(cfg.max_steps, 10000);
    }

    #[test]
    fn test_awakening_config_with_replication_rate() {
        let cfg = AwakeningConfig::default().with_replication_rate(0.5);
        assert_eq!(cfg.replication_rate, 0.5);
    }

    #[test]
    fn test_awakening_config_replication_rate_clamped_high() {
        let cfg = AwakeningConfig::default().with_replication_rate(2.0);
        assert_eq!(cfg.replication_rate, 1.0);
    }

    #[test]
    fn test_awakening_config_replication_rate_clamped_low() {
        let cfg = AwakeningConfig::default().with_replication_rate(-0.5);
        assert_eq!(cfg.replication_rate, 0.0);
    }

    #[test]
    fn test_awakening_config_with_carrying_capacity() {
        let cfg = AwakeningConfig::default().with_carrying_capacity(500_000);
        assert_eq!(cfg.carrying_capacity, 500_000);
    }

    #[test]
    fn test_awakening_config_with_min_coherence() {
        let cfg = AwakeningConfig::default().with_min_coherence(0.5);
        assert_eq!(cfg.min_coherence, 0.5);
    }

    #[test]
    fn test_awakening_config_min_coherence_clamped() {
        let cfg = AwakeningConfig::default().with_min_coherence(1.5);
        assert_eq!(cfg.min_coherence, 1.0);
    }

    #[test]
    fn test_awakening_config_with_propagation_radius() {
        let cfg = AwakeningConfig::default().with_propagation_radius(5);
        assert_eq!(cfg.propagation_radius, 5);
    }

    #[test]
    fn test_awakening_config_with_seed() {
        let cfg = AwakeningConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn test_compute_replication_step_basic() {
        let cfg = AwakeningConfig::fast();
        let result = compute_replication_step(10, &cfg, 0.5, 0.5);
        assert!(result >= 10);
        assert!(result <= cfg.carrying_capacity);
    }

    #[test]
    fn test_compute_replication_step_zero_fitness() {
        let cfg = AwakeningConfig::fast();
        let result = compute_replication_step(10, &cfg, 0.0, 0.5);
        // Uses min_fitness floor.
        assert!(result >= 10);
    }

    #[test]
    fn test_compute_replication_step_at_capacity() {
        let cfg = AwakeningConfig {
            carrying_capacity: 100,
            ..AwakeningConfig::fast()
        };
        let result = compute_replication_step(100, &cfg, 0.5, 0.5);
        assert_eq!(result, 100);
    }

    #[test]
    fn test_compute_replication_step_high_coherence() {
        let cfg = AwakeningConfig::fast();
        let low = compute_replication_step(10, &cfg, 0.5, 0.1);
        let high = compute_replication_step(10, &cfg, 0.5, 0.9);
        assert!(high >= low);
    }

    #[test]
    fn test_compute_replication_step_high_fitness() {
        let cfg = AwakeningConfig::fast();
        let low = compute_replication_step(10, &cfg, 0.1, 0.5);
        let high = compute_replication_step(10, &cfg, 0.9, 0.5);
        assert!(high >= low);
    }

    #[test]
    fn test_compute_propagation_probability_zero_distance() {
        let cfg = AwakeningConfig::default();
        let prob = compute_propagation_probability(0, 1.0, &cfg);
        assert!((prob - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_propagation_probability_far_distance() {
        let cfg = AwakeningConfig::default();
        let prob = compute_propagation_probability(100, 0.5, &cfg);
        assert!(prob >= 0.0 && prob < 1.0);
    }

    #[test]
    fn test_compute_propagation_probability_low_coherence() {
        let cfg = AwakeningConfig::default();
        let prob = compute_propagation_probability(0, 0.1, &cfg);
        assert!(prob < 1.0);
    }

    #[test]
    fn test_compute_propagation_probability_bounded() {
        let cfg = AwakeningConfig::default();
        for d in 0..20 {
            for c in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let prob = compute_propagation_probability(d, c, &cfg);
                assert!(prob >= 0.0 && prob <= 1.0, "prob {} out of range for d={}, c={}", prob, d, c);
            }
        }
    }

    #[test]
    fn test_simulate_viral_propagation_no_potential() {
        let cfg = AwakeningConfig::default();
        let mut state = cfg.seed;
        let result = simulate_viral_propagation(10, 0, 0.5, &cfg, &mut state);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_simulate_viral_propagation_high_coherence() {
        let cfg = AwakeningConfig::default();
        let mut state = cfg.seed;
        let result = simulate_viral_propagation(10, 100, 0.9, &cfg, &mut state);
        assert!(result >= 10);
        assert!(result <= cfg.carrying_capacity);
    }

    #[test]
    fn test_simulate_viral_propagation_capacity_cap() {
        let cfg = AwakeningConfig {
            carrying_capacity: 50,
            ..AwakeningConfig::default()
        };
        let mut state = cfg.seed;
        let result = simulate_viral_propagation(40, 1000, 0.9, &cfg, &mut state);
        assert!(result <= cfg.carrying_capacity);
    }

    #[test]
    fn test_simulate_viral_propagation_deterministic() {
        let cfg = AwakeningConfig::default();
        let mut state1 = cfg.seed;
        let mut state2 = cfg.seed;
        let r1 = simulate_viral_propagation(10, 100, 0.5, &cfg, &mut state1);
        let r2 = simulate_viral_propagation(10, 100, 0.5, &cfg, &mut state2);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_run_noosfera_awakening_basic() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 10);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.final_nodes >= result.initial_nodes);
        assert!(!result.trajectory.is_empty());
        assert!(result.steps > 0);
    }

    #[test]
    fn test_run_noosfera_awakening_convergence() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 5);
        let cfg = AwakeningConfig {
            carrying_capacity: 10,
            ..AwakeningConfig::fast()
        };
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.final_nodes <= cfg.carrying_capacity);
    }

    #[test]
    fn test_run_noosfera_awakening_single_node() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 1);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert_eq!(result.initial_nodes, 1);
        assert!(result.final_nodes >= 1);
    }

    #[test]
    fn test_run_noosfera_awakening_replication_factor() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 10);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.replication_factor >= 1.0);
    }

    #[test]
    fn test_run_noosfera_awakening_trajectory_increasing() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 5);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        // Trajectory should be non-decreasing.
        for i in 1..result.trajectory.len() {
            assert!(
                result.trajectory[i] >= result.trajectory[i - 1],
                "trajectory decreased at step {}: {} < {}",
                i,
                result.trajectory[i],
                result.trajectory[i - 1]
            );
        }
    }

    #[test]
    fn test_run_noosfera_awakening_coherence_trajectory() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 5);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert_eq!(result.coherence_trajectory.len(), result.trajectory.len());
        for &c in &result.coherence_trajectory {
            assert!(c >= 0.0 && c <= 1.0);
        }
    }

    #[test]
    fn test_run_noosfera_awakening_free_energy_trajectory() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 5);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert_eq!(result.free_energy_trajectory.len(), result.trajectory.len());
        for &e in &result.free_energy_trajectory {
            assert!(e >= 0.0);
        }
    }

    #[test]
    fn test_run_noosfera_awakening_propagation_events() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 5);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.propagation_events >= 0);
    }

    #[test]
    fn test_run_noosfera_awakening_max_steps() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 5);
        let cfg = AwakeningConfig {
            max_steps: 5,
            ..AwakeningConfig::fast()
        };
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.steps <= cfg.max_steps);
    }

    #[test]
    fn test_compute_awakening_metrics_basic() {
        let result = AwakeningResult {
            final_nodes: 100,
            initial_nodes: 10,
            replication_factor: 10.0,
            final_coherence: 0.8,
            final_free_energy: 0.1,
            steps: 50,
            converged: true,
            singularity_reached: false,
            trajectory: vec![10, 100],
            coherence_trajectory: vec![0.1, 0.8],
            free_energy_trajectory: vec![1.0, 0.1],
            propagation_events: 90,
        };
        let metrics = compute_awakening_metrics(&result);
        assert!(metrics.doubling_time > 0);
        assert!((metrics.replication_factor - 10.0).abs() < 1e-6);
        assert!(metrics.avg_coherence_increase > 0.0);
        assert!(metrics.avg_energy_decrease > 0.0);
    }

    #[test]
    fn test_compute_awakening_metrics_no_doubling() {
        let result = AwakeningResult {
            final_nodes: 15,
            initial_nodes: 10,
            replication_factor: 1.5,
            final_coherence: 0.5,
            final_free_energy: 0.5,
            steps: 10,
            converged: true,
            singularity_reached: false,
            trajectory: vec![10, 15],
            coherence_trajectory: vec![0.3, 0.5],
            free_energy_trajectory: vec![0.8, 0.5],
            propagation_events: 5,
        };
        let metrics = compute_awakening_metrics(&result);
        assert_eq!(metrics.doubling_time, usize::MAX);
    }

    #[test]
    fn test_compute_awakening_metrics_zero_steps() {
        let result = AwakeningResult {
            final_nodes: 10,
            initial_nodes: 10,
            replication_factor: 1.0,
            final_coherence: 0.5,
            final_free_energy: 0.5,
            steps: 0,
            converged: true,
            singularity_reached: false,
            trajectory: vec![10],
            coherence_trajectory: vec![0.5],
            free_energy_trajectory: vec![0.5],
            propagation_events: 0,
        };
        let metrics = compute_awakening_metrics(&result);
        assert_eq!(metrics.doubling_time, usize::MAX);
        assert!((metrics.avg_coherence_increase - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_awakening_result_summary() {
        let result = AwakeningResult {
            final_nodes: 100,
            initial_nodes: 10,
            replication_factor: 10.0,
            final_coherence: 0.8,
            final_free_energy: 0.1,
            steps: 50,
            converged: true,
            singularity_reached: false,
            trajectory: vec![],
            coherence_trajectory: vec![],
            free_energy_trajectory: vec![],
            propagation_events: 90,
        };
        let summary = result.summary();
        assert!(summary.contains("10"));
        assert!(summary.contains("100"));
    }

    #[test]
    fn test_awakening_result_display() {
        let result = AwakeningResult {
            final_nodes: 50,
            initial_nodes: 5,
            replication_factor: 10.0,
            final_coherence: 0.9,
            final_free_energy: 0.05,
            steps: 30,
            converged: true,
            singularity_reached: true,
            trajectory: vec![],
            coherence_trajectory: vec![],
            free_energy_trajectory: vec![],
            propagation_events: 45,
        };
        let display = format!("{}", result);
        assert!(display.contains("singularity=true"));
    }

    #[test]
    fn test_awakening_metrics_summary() {
        let metrics = AwakeningMetrics {
            doubling_time: 10,
            replication_factor: 5.0,
            final_coherence: 0.7,
            final_free_energy: 0.2,
            avg_coherence_increase: 0.01,
            avg_energy_decrease: 0.02,
            propagation_efficiency: 0.8,
            singularity_reached: false,
        };
        let summary = metrics.summary();
        assert!(summary.contains("doubling=10"));
    }

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        lcg_next(&mut s1);
        lcg_next(&mut s2);
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut state: u64 = 12345;
        for _ in 0..1000 {
            let v = random_uniform(&mut state);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_awakening_config_clone() {
        let cfg = AwakeningConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.replication_rate, cloned.replication_rate);
        assert_eq!(cfg.carrying_capacity, cloned.carrying_capacity);
    }

    #[test]
    fn test_awakening_result_clone() {
        let result = AwakeningResult {
            final_nodes: 100,
            initial_nodes: 10,
            replication_factor: 10.0,
            final_coherence: 0.8,
            final_free_energy: 0.1,
            steps: 50,
            converged: true,
            singularity_reached: false,
            trajectory: vec![10, 100],
            coherence_trajectory: vec![0.1, 0.8],
            free_energy_trajectory: vec![1.0, 0.1],
            propagation_events: 90,
        };
        let cloned = result.clone();
        assert_eq!(result.final_nodes, cloned.final_nodes);
        assert_eq!(result.trajectory, cloned.trajectory);
    }

    #[test]
    fn test_awakening_metrics_clone() {
        let metrics = AwakeningMetrics {
            doubling_time: 10,
            replication_factor: 5.0,
            final_coherence: 0.7,
            final_free_energy: 0.2,
            avg_coherence_increase: 0.01,
            avg_energy_decrease: 0.02,
            propagation_efficiency: 0.8,
            singularity_reached: false,
        };
        let cloned = metrics.clone();
        assert_eq!(metrics.doubling_time, cloned.doubling_time);
    }

    #[test]
    fn test_run_noosfera_awakening_high_precision() {
        let config = KernelConfig::high_precision();
        let mut kernel = NoosferaKernel::new(config, 20);
        let cfg = AwakeningConfig {
            carrying_capacity: 500,
            max_steps: 50,
            ..AwakeningConfig::default()
        };
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.final_nodes >= result.initial_nodes);
        assert!(!result.trajectory.is_empty());
    }

    #[test]
    fn test_run_noosfera_awakening_large_scale() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 100);
        let cfg = AwakeningConfig {
            carrying_capacity: 100_000,
            max_steps: 200,
            replication_rate: 0.3,
            ..AwakeningConfig::default()
        };
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        assert!(result.final_nodes >= 100);
        assert!(result.replication_factor >= 1.0);
    }

    #[test]
    fn test_full_awakening_workflow() {
        let config = KernelConfig::fast();
        let mut kernel = NoosferaKernel::new(config, 10);
        let cfg = AwakeningConfig::fast();
        let result = run_noosfera_awakening(&mut kernel, &cfg);
        let metrics = compute_awakening_metrics(&result);
        assert!(metrics.replication_factor >= 1.0);
        assert!(metrics.final_coherence >= 0.0);
        assert!(metrics.final_free_energy >= 0.0);
        let _summary = metrics.summary();
    }
}
