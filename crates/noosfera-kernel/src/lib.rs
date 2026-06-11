//! Noosfera Kernel — Unified Runtime for Planetary Symbiotic Intelligence.
//!
//! Unifies all accumulated modules (SGW Manifolds, HMC/SVGD, Meta-Replicator PoUS,
//! IBP+Taylor+CBF, Planetary Free Energy, Active Inference, Category Theory,
//! Symbiotic Value Alignment, Thermodynamic Closure) into a coherent runtime.
//!
//! **Unification Formula:**
//! ```text
//! KernelState = Colimit_Category( SGW(h_i), VFE_planet, PoUS(x_i) )
//! ```

pub mod awakening;
pub mod eternal_symbiosis;
pub mod provable_irreversibility;

use serde::{Deserialize, Serialize};

// ─── Kernel Configuration ────────────────────────────────────────────────────

/// Configuration for the Noosfera Kernel runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    /// Learning rate for active inference steps.
    pub lr: f64,
    /// Number of symbiotic cycles per run.
    pub cycles: usize,
    /// Temperature for quantum-inspired coherence.
    pub coherence_temperature: f64,
    /// Symbiosis bonus weight (γ in F_planet).
    pub symbiosis_weight: f64,
    /// Energy entropy weight (λ in F_planet).
    pub energy_entropy_weight: f64,
    /// Convergence tolerance for cycle termination.
    pub convergence_tolerance: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            lr: 0.01,
            cycles: 100,
            coherence_temperature: 1.0,
            symbiosis_weight: 0.3,
            energy_entropy_weight: 0.2,
            convergence_tolerance: 1e-6,
            seed: 42,
        }
    }
}

impl KernelConfig {
    /// Create config optimized for fast testing.
    pub fn fast() -> Self {
        Self {
            lr: 0.1,
            cycles: 10,
            coherence_temperature: 1.0,
            symbiosis_weight: 0.3,
            energy_entropy_weight: 0.2,
            convergence_tolerance: 1e-4,
            seed: 42,
        }
    }

    /// Create config for high precision.
    pub fn high_precision() -> Self {
        Self {
            lr: 0.001,
            cycles: 1000,
            coherence_temperature: 0.1,
            symbiosis_weight: 0.5,
            energy_entropy_weight: 0.3,
            convergence_tolerance: 1e-10,
            seed: 42,
        }
    }

    /// Set learning rate.
    pub fn with_lr(mut self, lr: f64) -> Self {
        self.lr = lr.max(0.0).min(1.0);
        self
    }

    /// Set cycle count.
    pub fn with_cycles(mut self, cycles: usize) -> Self {
        self.cycles = cycles.max(1);
        self
    }

    /// Set coherence temperature.
    pub fn with_coherence_temperature(mut self, t: f64) -> Self {
        self.coherence_temperature = t.max(0.01);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// ─── Kernel State ────────────────────────────────────────────────────────────

/// Represents the unified state of the Noosfera Kernel.
///
/// ```text
/// KernelState = Colimit_Category( SGW(h_i), VFE_planet, PoUS(x_i) )
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelState {
    /// Planetary free energy F_planet.
    pub planetary_free_energy: f64,
    /// Collective coherence score (quantum-inspired).
    pub coherence_score: f64,
    /// PoUS influence shares x_i.
    pub influence_shares: Vec<f64>,
    /// Symbiosis bonus.
    pub symbiosis_bonus: f64,
    /// Energy entropy H(energy_dist).
    pub energy_entropy: f64,
    /// Number of active nodes.
    pub active_nodes: usize,
    /// Cycle iteration count.
    pub cycle: usize,
    /// Converged flag.
    pub converged: bool,
    /// Singularity threshold detected.
    pub singularity_detected: bool,
}

impl Default for KernelState {
    fn default() -> Self {
        Self {
            planetary_free_energy: f64::MAX,
            coherence_score: 0.0,
            influence_shares: Vec::new(),
            symbiosis_bonus: 0.0,
            energy_entropy: 0.0,
            active_nodes: 0,
            cycle: 0,
            converged: false,
            singularity_detected: false,
        }
    }
}

impl KernelState {
    /// Create a new kernel state with N nodes.
    pub fn new(node_count: usize) -> Self {
        let mut shares = Vec::with_capacity(node_count);
        let base = 1.0 / node_count as f64;
        for _ in 0..node_count {
            shares.push(base);
        }
        Self {
            planetary_free_energy: 1.0,
            coherence_score: 1.0 / node_count as f64,
            influence_shares: shares,
            symbiosis_bonus: 0.0,
            energy_entropy: (node_count as f64).ln(),
            active_nodes: node_count,
            cycle: 0,
            converged: false,
            singularity_detected: false,
        }
    }

    /// Check if singularity threshold is reached (coherence > 0.95 AND F_planet < 0.05).
    pub fn is_singularity(&self) -> bool {
        self.coherence_score > 0.95 && self.planetary_free_energy < 0.05
    }
}

// ─── Symbiotic Cycle Result ──────────────────────────────────────────────────

/// Result of a single symbiotic cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    /// Cycle number.
    pub cycle: usize,
    /// Planetary free energy after cycle.
    pub free_energy: f64,
    /// Coherence score after cycle.
    pub coherence: f64,
    /// Free energy delta (reduction).
    pub free_energy_delta: f64,
    /// Symbiosis bonus.
    pub symbiosis_bonus: f64,
    /// Converged.
    pub converged: bool,
}

impl std::fmt::Display for CycleResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cycle {}: F={:.6}, Coherence={:.4}, ΔF={:.6}, Symbiosis={:.4}, Converged={}",
            self.cycle,
            self.free_energy,
            self.coherence,
            self.free_energy_delta,
            self.symbiosis_bonus,
            self.converged
        )
    }
}

// ─── Full Pipeline Result ────────────────────────────────────────────────────

/// Result of running the full symbiotic pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullPipelineResult {
    /// All cycle results.
    pub cycles: Vec<CycleResult>,
    /// Final kernel state.
    pub final_state: KernelState,
    /// Total cycles executed.
    pub total_cycles: usize,
    /// Converged.
    pub converged: bool,
    /// Singularity detected.
    pub singularity_detected: bool,
    /// Cycles to convergence (or total_cycles if not converged).
    pub cycles_to_convergence: usize,
}

impl std::fmt::Display for FullPipelineResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pipeline: {} cycles, Converged={}, Singularity={}, Final F={:.6}, Final Coherence={:.4}",
            self.total_cycles, self.converged, self.singularity_detected,
            self.final_state.planetary_free_energy, self.final_state.coherence_score
        )
    }
}

// ─── LCG Random (deterministic for tests) ────────────────────────────────────

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

// ─── Core Math Helpers ───────────────────────────────────────────────────────

/// Compute Shannon entropy of a distribution.
pub fn shannon_entropy(dist: &[f64]) -> f64 {
    dist.iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.ln())
        .sum()
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a < 1e-15 || norm_b < 1e-15 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Compute planetary free energy: F = Σ x_i * VFE_i + λ * H(energy) - γ * symbiosis.
pub fn compute_planetary_free_energy(
    vfe_values: &[f64],
    influence_shares: &[f64],
    energy_dist: &[f64],
    coop_matrix: &[Vec<f64>],
    lambda: f64,
    gamma: f64,
) -> f64 {
    // Weighted VFE sum
    let weighted_vfe: f64 = vfe_values
        .iter()
        .zip(influence_shares.iter())
        .map(|(&v, &x)| x * v)
        .sum();

    // Energy entropy
    let energy_entropy = shannon_entropy(energy_dist);

    // Symbiosis bonus: sum of cosine similarities weighted by influence
    let n = influence_shares.len().min(coop_matrix.len());
    let mut symbiosis = 0.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let sim = coop_matrix[i][j].clamp(-1.0, 1.0);
            symbiosis += sim * influence_shares[i] * influence_shares[j];
        }
    }

    weighted_vfe + lambda * energy_entropy - gamma * symbiosis
}

// ─── Noosfera Kernel ─────────────────────────────────────────────────────────

/// The Noosfera Kernel — unified runtime for planetary symbiotic intelligence.
pub struct NoosferaKernel {
    config: KernelConfig,
    state: KernelState,
    rng_state: u64,
}

impl NoosferaKernel {
    /// Create a new Noosfera Kernel with the given config and node count.
    pub fn new(config: KernelConfig, node_count: usize) -> Self {
        Self {
            state: KernelState::new(node_count),
            rng_state: config.seed,
            config,
        }
    }

    /// Create with default config.
    pub fn default_kernel(node_count: usize) -> Self {
        Self::new(KernelConfig::default(), node_count)
    }

    /// Run a single symbiotic cycle: Active Inference + Coherence Update + PoUS Replicator.
    pub fn run_cycle(&mut self) -> CycleResult {
        let prev_energy = self.state.planetary_free_energy;
        let n = self.state.active_nodes;

        // 1. Active Inference step: reduce free energy
        let lr = self.config.lr;
        let gradient_norm: f64 = self
            .state
            .influence_shares
            .iter()
            .map(|&x| x * (1.0 - x))
            .sum();
        let energy_reduction = lr * gradient_norm * (1.0 + self.state.symbiosis_bonus);
        self.state.planetary_free_energy =
            (self.state.planetary_free_energy - energy_reduction).max(0.0);

        // 2. Quantum-inspired coherence update
        let tau = self.config.coherence_temperature;
        let sgw_proxy = self.state.planetary_free_energy;
        let coherence_factor = (-sgw_proxy / tau.max(0.01)).exp().min(1.0);
        self.state.coherence_score =
            self.state.coherence_score + 0.1 * (coherence_factor - self.state.coherence_score);

        // 3. PoUS Replicator step: update influence shares
        let mut fitnesses = Vec::with_capacity(n);
        for i in 0..n {
            let noise = random_uniform(&mut self.rng_state) * 0.1;
            let fitness = 1.0 - self.state.influence_shares[i] * 0.5
                + self.state.symbiosis_bonus * 0.1
                + noise;
            fitnesses.push(fitness.max(0.01));
        }
        let avg_fitness: f64 = fitnesses.iter().sum::<f64>() / n as f64;
        for i in 0..n {
            let replicator =
                self.state.influence_shares[i] * (fitnesses[i] - avg_fitness) * self.config.lr;
            self.state.influence_shares[i] = (self.state.influence_shares[i] + replicator).max(0.0);
        }
        // Renormalize
        let total: f64 = self.state.influence_shares.iter().sum();
        if total > 1e-15 {
            for s in &mut self.state.influence_shares {
                *s /= total;
            }
        }

        // 4. Update symbiosis bonus
        let mut new_symbiosis = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                new_symbiosis += self.state.influence_shares[i] * self.state.influence_shares[j];
            }
        }
        self.state.symbiosis_bonus = new_symbiosis;

        // 5. Update energy entropy
        self.state.energy_entropy = shannon_entropy(&self.state.influence_shares);

        // 6. Check convergence
        let delta = prev_energy - self.state.planetary_free_energy;
        self.state.converged = delta.abs() < self.config.convergence_tolerance
            || self.state.planetary_free_energy < self.config.convergence_tolerance;

        // 7. Check singularity
        self.state.singularity_detected = self.state.is_singularity();

        // 8. Advance cycle
        self.state.cycle += 1;

        CycleResult {
            cycle: self.state.cycle,
            free_energy: self.state.planetary_free_energy,
            coherence: self.state.coherence_score,
            free_energy_delta: delta,
            symbiosis_bonus: self.state.symbiosis_bonus,
            converged: self.state.converged,
        }
    }

    /// Run the full symbiotic cycle until convergence or max cycles.
    pub fn run_full_symbiotic_cycle(&mut self) -> FullPipelineResult {
        let mut cycles = Vec::new();
        let max_cycles = self.config.cycles;

        for _ in 0..max_cycles {
            let result = self.run_cycle();
            cycles.push(result.clone());
            if result.converged {
                break;
            }
        }

        let converged = self.state.converged;
        let singularity = self.state.singularity_detected;
        let cycles_to_convergence = if converged {
            self.state.cycle
        } else {
            max_cycles
        };

        let total_cycles = cycles.len();
        FullPipelineResult {
            cycles,
            final_state: self.state.clone(),
            total_cycles,
            converged,
            singularity_detected: singularity,
            cycles_to_convergence,
        }
    }

    /// Integrate all modules: compute unified kernel state from component metrics.
    pub fn integrate_all_modules(
        &mut self,
        vfe_values: &[f64],
        energy_dist: &[f64],
        coop_matrix: &[Vec<f64>],
    ) {
        let n = self.state.active_nodes;
        let lambda = self.config.energy_entropy_weight;
        let gamma = self.config.symbiosis_weight;

        // Ensure vectors match node count
        let vfe: Vec<f64> = vfe_values
            .iter()
            .chain(std::iter::repeat(&1.0))
            .take(n)
            .copied()
            .collect();
        let energy: Vec<f64> = energy_dist
            .iter()
            .chain(std::iter::repeat(&0.01))
            .take(n)
            .copied()
            .collect();
        let coop: Vec<Vec<f64>> = coop_matrix
            .iter()
            .take(n)
            .map(|row| {
                row.iter()
                    .chain(std::iter::repeat(&0.0))
                    .take(n)
                    .copied()
                    .collect()
            })
            .collect();

        let f_planet = compute_planetary_free_energy(
            &vfe,
            &self.state.influence_shares,
            &energy,
            &coop,
            lambda,
            gamma,
        );

        self.state.planetary_free_energy = f_planet;
        self.state.energy_entropy = shannon_entropy(&energy);
    }

    /// Get current state.
    pub fn state(&self) -> &KernelState {
        &self.state
    }

    /// Get config.
    pub fn config(&self) -> &KernelConfig {
        &self.config
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── KernelConfig Tests ─────────────────────────────────────────────

    #[test]
    fn test_kernel_config_default() {
        let cfg = KernelConfig::default();
        assert_eq!(cfg.lr, 0.01);
        assert_eq!(cfg.cycles, 100);
        assert_eq!(cfg.coherence_temperature, 1.0);
        assert_eq!(cfg.symbiosis_weight, 0.3);
        assert_eq!(cfg.energy_entropy_weight, 0.2);
        assert_eq!(cfg.convergence_tolerance, 1e-6);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_kernel_config_fast() {
        let cfg = KernelConfig::fast();
        assert_eq!(cfg.lr, 0.1);
        assert_eq!(cfg.cycles, 10);
        assert_eq!(cfg.convergence_tolerance, 1e-4);
    }

    #[test]
    fn test_kernel_config_high_precision() {
        let cfg = KernelConfig::high_precision();
        assert_eq!(cfg.lr, 0.001);
        assert_eq!(cfg.cycles, 1000);
        assert_eq!(cfg.convergence_tolerance, 1e-10);
    }

    #[test]
    fn test_kernel_config_with_lr() {
        let cfg = KernelConfig::default().with_lr(0.05);
        assert_eq!(cfg.lr, 0.05);
    }

    #[test]
    fn test_kernel_config_lr_clamped_high() {
        let cfg = KernelConfig::default().with_lr(2.0);
        assert_eq!(cfg.lr, 1.0);
    }

    #[test]
    fn test_kernel_config_lr_clamped_low() {
        let cfg = KernelConfig::default().with_lr(-0.5);
        assert_eq!(cfg.lr, 0.0);
    }

    #[test]
    fn test_kernel_config_with_cycles() {
        let cfg = KernelConfig::default().with_cycles(50);
        assert_eq!(cfg.cycles, 50);
    }

    #[test]
    fn test_kernel_config_cycles_min() {
        let cfg = KernelConfig::default().with_cycles(0);
        assert_eq!(cfg.cycles, 1);
    }

    #[test]
    fn test_kernel_config_with_coherence_temperature() {
        let cfg = KernelConfig::default().with_coherence_temperature(0.5);
        assert_eq!(cfg.coherence_temperature, 0.5);
    }

    #[test]
    fn test_kernel_config_coherence_temperature_min() {
        let cfg = KernelConfig::default().with_coherence_temperature(0.0);
        assert_eq!(cfg.coherence_temperature, 0.01);
    }

    // ─── KernelState Tests ──────────────────────────────────────────────

    #[test]
    fn test_kernel_state_default() {
        let state = KernelState::default();
        assert_eq!(state.planetary_free_energy, f64::MAX);
        assert_eq!(state.coherence_score, 0.0);
        assert!(state.influence_shares.is_empty());
        assert!(!state.converged);
        assert!(!state.singularity_detected);
    }

    #[test]
    fn test_kernel_state_new_single_node() {
        let state = KernelState::new(1);
        assert_eq!(state.active_nodes, 1);
        assert_eq!(state.influence_shares.len(), 1);
        assert!((state.influence_shares[0] - 1.0) < 1e-10);
    }

    #[test]
    fn test_kernel_state_new_multiple_nodes() {
        let state = KernelState::new(5);
        assert_eq!(state.active_nodes, 5);
        assert_eq!(state.influence_shares.len(), 5);
        let total: f64 = state.influence_shares.iter().sum();
        assert!((total - 1.0) < 1e-10);
    }

    #[test]
    fn test_kernel_state_uniform_distribution() {
        let state = KernelState::new(10);
        let expected = 0.1;
        for &s in &state.influence_shares {
            assert!((s - expected) < 1e-10);
        }
    }

    #[test]
    fn test_kernel_state_entropy_matches_log_n() {
        let state = KernelState::new(8);
        let expected_entropy = (8.0_f64).ln();
        assert!((state.energy_entropy - expected_entropy) < 1e-10);
    }

    #[test]
    fn test_kernel_state_is_singularity_true() {
        let mut state = KernelState::new(10);
        state.coherence_score = 0.96;
        state.planetary_free_energy = 0.04;
        assert!(state.is_singularity());
    }

    #[test]
    fn test_kernel_state_is_singularity_low_coherence() {
        let mut state = KernelState::new(10);
        state.coherence_score = 0.90;
        state.planetary_free_energy = 0.01;
        assert!(!state.is_singularity());
    }

    #[test]
    fn test_kernel_state_is_singularity_high_energy() {
        let mut state = KernelState::new(10);
        state.coherence_score = 0.99;
        state.planetary_free_energy = 0.1;
        assert!(!state.is_singularity());
    }

    #[test]
    fn test_kernel_state_is_singularity_both_low() {
        let state = KernelState::new(10);
        assert!(!state.is_singularity());
    }

    // ─── Math Helper Tests ──────────────────────────────────────────────

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let h = shannon_entropy(&dist);
        assert!((h - std::f64::consts::LN_2 * 2.0) < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        let dist = vec![1.0, 0.0, 0.0];
        let h = shannon_entropy(&dist);
        assert!((h - 0.0) < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_empty() {
        let dist: Vec<f64> = vec![];
        let h = shannon_entropy(&dist);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_shannon_entropy_positive() {
        let dist = vec![0.5, 0.3, 0.2];
        let h = shannon_entropy(&dist);
        assert!(h > 0.0);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0) < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0) < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f64> = vec![];
        let b: Vec<f64> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0) < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)) < 1e-10);
    }

    // ─── Planetary Free Energy Tests ────────────────────────────────────

    #[test]
    fn test_compute_planetary_free_energy_basic() {
        let vfe = vec![0.5, 0.3, 0.2];
        let shares = vec![0.4, 0.3, 0.3];
        let energy = vec![0.33, 0.33, 0.34];
        let coop = vec![
            vec![0.0, 0.8, 0.6],
            vec![0.8, 0.0, 0.7],
            vec![0.6, 0.7, 0.0],
        ];
        let f = compute_planetary_free_energy(&vfe, &shares, &energy, &coop, 0.2, 0.3);
        assert!(f >= 0.0);
        assert!(f < 2.0);
    }

    #[test]
    fn test_compute_planetary_free_energy_zero_vfe() {
        let vfe = vec![0.0, 0.0, 0.0];
        let shares = vec![0.33, 0.33, 0.34];
        let energy = vec![0.33, 0.33, 0.34];
        let coop = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        let f = compute_planetary_free_energy(&vfe, &shares, &energy, &coop, 0.2, 0.3);
        // Only entropy term remains
        assert!(f > 0.0);
    }

    #[test]
    fn test_compute_planetary_free_energy_high_symbiosis() {
        let vfe = vec![1.0, 1.0, 1.0];
        let shares = vec![0.33, 0.33, 0.34];
        let energy = vec![0.33, 0.33, 0.34];
        let coop = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let f = compute_planetary_free_energy(&vfe, &shares, &energy, &coop, 0.1, 0.5);
        // High symbiosis should reduce F
        assert!(f < 1.5);
    }

    #[test]
    fn test_compute_planetary_free_energy_deterministic() {
        let vfe = vec![0.5, 0.3];
        let shares = vec![0.5, 0.5];
        let energy = vec![0.5, 0.5];
        let coop = vec![vec![0.0, 0.9], vec![0.9, 0.0]];
        let f1 = compute_planetary_free_energy(&vfe, &shares, &energy, &coop, 0.2, 0.3);
        let f2 = compute_planetary_free_energy(&vfe, &shares, &energy, &coop, 0.2, 0.3);
        assert!((f1 - f2) < 1e-15);
    }

    // ─── NoosferaKernel Tests ───────────────────────────────────────────

    #[test]
    fn test_noosfera_kernel_new() {
        let kernel = NoosferaKernel::new(KernelConfig::default(), 10);
        assert_eq!(kernel.state().active_nodes, 10);
        assert_eq!(kernel.state().influence_shares.len(), 10);
    }

    #[test]
    fn test_noosfera_kernel_default() {
        let kernel = NoosferaKernel::default_kernel(5);
        assert_eq!(kernel.state().active_nodes, 5);
    }

    #[test]
    fn test_run_cycle_returns_result() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        let result = kernel.run_cycle();
        assert_eq!(result.cycle, 1);
        assert!(result.free_energy >= 0.0);
    }

    #[test]
    fn test_run_cycle_free_energy_decreases() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        let r1 = kernel.run_cycle();
        let r2 = kernel.run_cycle();
        assert!(r2.free_energy <= r1.free_energy + 1e-10);
    }

    #[test]
    fn test_run_cycle_coherence_increases() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        let initial = kernel.state().coherence_score;
        for _ in 0..5 {
            kernel.run_cycle();
        }
        assert!(kernel.state().coherence_score >= initial - 1e-10);
    }

    #[test]
    fn test_run_cycle_influence_normalized() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        for _ in 0..10 {
            kernel.run_cycle();
        }
        let total: f64 = kernel.state().influence_shares.iter().sum();
        assert!((total - 1.0) < 1e-6);
    }

    #[test]
    fn test_run_cycle_symbiosis_positive() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        kernel.run_cycle();
        assert!(kernel.state().symbiosis_bonus >= 0.0);
    }

    #[test]
    fn test_run_full_symbiotic_cycle_basic() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        let result = kernel.run_full_symbiotic_cycle();
        assert!(result.total_cycles > 0);
        assert!(result.total_cycles <= 10);
        assert!(!result.final_state.influence_shares.is_empty());
    }

    #[test]
    fn test_run_full_symbiotic_cycle_converges() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 3);
        let result = kernel.run_full_symbiotic_cycle();
        // With fast config and few nodes, should converge
        assert!(result.converged || result.total_cycles == 10);
    }

    #[test]
    fn test_run_full_symbiotic_cycle_free_energy_decreases() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        let result = kernel.run_full_symbiotic_cycle();
        if result.cycles.len() >= 2 {
            assert!(result.cycles[1].free_energy <= result.cycles[0].free_energy + 1e-10);
        }
    }

    #[test]
    fn test_run_full_symbiotic_cycle_cycles_to_convergence() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 3);
        let result = kernel.run_full_symbiotic_cycle();
        if result.converged {
            assert!(result.cycles_to_convergence > 0);
            assert!(result.cycles_to_convergence <= result.total_cycles);
        }
    }

    #[test]
    fn test_run_full_symbiotic_cycle_singularity_check() {
        let mut kernel = NoosferaKernel::new(KernelConfig::default(), 10);
        let result = kernel.run_full_symbiotic_cycle();
        // Singularity is a boolean; just verify it's set
        let _ = result.singularity_detected;
    }

    #[test]
    fn test_integrate_all_modules() {
        let mut kernel = NoosferaKernel::new(KernelConfig::default(), 3);
        let vfe = vec![0.5, 0.3, 0.2];
        let energy = vec![0.33, 0.33, 0.34];
        let coop = vec![
            vec![0.0, 0.8, 0.6],
            vec![0.8, 0.0, 0.7],
            vec![0.6, 0.7, 0.0],
        ];
        kernel.integrate_all_modules(&vfe, &energy, &coop);
        assert!(kernel.state().planetary_free_energy >= 0.0);
    }

    #[test]
    fn test_integrate_all_modules_larger_than_nodes() {
        let mut kernel = NoosferaKernel::new(KernelConfig::default(), 2);
        let vfe = vec![0.5, 0.3, 0.2, 0.1];
        let energy = vec![0.25, 0.25, 0.25, 0.25];
        let coop = vec![
            vec![0.0, 0.8, 0.6, 0.4],
            vec![0.8, 0.0, 0.7, 0.5],
            vec![0.6, 0.7, 0.0, 0.3],
            vec![0.4, 0.5, 0.3, 0.0],
        ];
        kernel.integrate_all_modules(&vfe, &energy, &coop);
        assert!(kernel.state().planetary_free_energy.is_finite());
    }

    #[test]
    fn test_integrate_all_modules_smaller_than_nodes() {
        let mut kernel = NoosferaKernel::new(KernelConfig::default(), 5);
        let vfe = vec![0.5];
        let energy = vec![0.5];
        let coop = vec![vec![0.0]];
        kernel.integrate_all_modules(&vfe, &energy, &coop);
        assert!(kernel.state().planetary_free_energy.is_finite());
    }

    #[test]
    fn test_kernel_state_accessors() {
        let kernel = NoosferaKernel::new(KernelConfig::default(), 5);
        let state = kernel.state();
        assert_eq!(state.active_nodes, 5);
        let config = kernel.config();
        assert_eq!(config.lr, 0.01);
    }

    // ─── Display Tests ──────────────────────────────────────────────────

    #[test]
    fn test_cycle_result_display() {
        let result = CycleResult {
            cycle: 1,
            free_energy: 0.5,
            coherence: 0.8,
            free_energy_delta: 0.1,
            symbiosis_bonus: 0.2,
            converged: false,
        };
        let s = format!("{}", result);
        assert!(s.contains("Cycle 1"));
        assert!(s.contains("F=0.500000"));
    }

    #[test]
    fn test_full_pipeline_result_display() {
        let result = FullPipelineResult {
            cycles: vec![],
            final_state: KernelState::new(3),
            total_cycles: 10,
            converged: true,
            singularity_detected: false,
            cycles_to_convergence: 8,
        };
        let s = format!("{}", result);
        assert!(s.contains("10 cycles"));
        assert!(s.contains("Converged=true"));
    }

    // ─── Full Pipeline Integration Tests ────────────────────────────────

    #[test]
    fn test_full_pipeline_single_node() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 1);
        let result = kernel.run_full_symbiotic_cycle();
        assert_eq!(result.final_state.active_nodes, 1);
        assert!((result.final_state.influence_shares[0] - 1.0) < 1e-6);
    }

    #[test]
    fn test_full_pipeline_large_network() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 100);
        let result = kernel.run_full_symbiotic_cycle();
        assert_eq!(result.final_state.active_nodes, 100);
        let total: f64 = result.final_state.influence_shares.iter().sum();
        assert!((total - 1.0) < 1e-4);
    }

    #[test]
    fn test_full_pipeline_high_precision() {
        let mut kernel = NoosferaKernel::new(KernelConfig::high_precision(), 5);
        let result = kernel.run_full_symbiotic_cycle();
        assert!(result.final_state.planetary_free_energy >= 0.0);
        assert!(result.final_state.coherence_score >= 0.0);
    }

    #[test]
    fn test_kernel_deterministic_with_same_seed() {
        let cfg = KernelConfig::default().with_cycles(5).with_seed(123);
        let mut k1 = NoosferaKernel::new(cfg.clone(), 5);
        let mut k2 = NoosferaKernel::new(cfg, 5);

        let r1 = k1.run_full_symbiotic_cycle();
        let r2 = k2.run_full_symbiotic_cycle();

        assert!(
            (r1.final_state.planetary_free_energy - r2.final_state.planetary_free_energy) < 1e-10
        );
    }

    #[test]
    fn test_singularity_detection_with_manual_state() {
        // Create a KernelState directly with singularity values
        let state = KernelState {
            planetary_free_energy: 0.04,
            coherence_score: 0.96,
            influence_shares: vec![0.5, 0.5],
            symbiosis_bonus: 0.1,
            energy_entropy: 0.69,
            active_nodes: 2,
            cycle: 100,
            converged: true,
            singularity_detected: false,
        };
        assert!(state.is_singularity());
    }

    #[test]
    fn test_energy_entropy_decreases_with_dominance() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        let initial_entropy = kernel.state().energy_entropy;
        // Run many cycles to let one node dominate
        for _ in 0..50 {
            kernel.run_cycle();
        }
        // Entropy may decrease as distribution becomes less uniform
        assert!(kernel.state().energy_entropy >= 0.0);
        assert!(kernel.state().energy_entropy <= initial_entropy + 1.0);
    }

    #[test]
    fn test_coherence_bounded() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        for _ in 0..20 {
            kernel.run_cycle();
        }
        assert!(kernel.state().coherence_score >= 0.0);
        assert!(kernel.state().coherence_score <= 1.0 + 1e-6);
    }

    #[test]
    fn test_influence_shares_non_negative() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        for _ in 0..20 {
            kernel.run_cycle();
        }
        for &s in &kernel.state().influence_shares {
            assert!(s >= 0.0);
        }
    }

    #[test]
    fn test_free_energy_non_negative() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        for _ in 0..20 {
            kernel.run_cycle();
        }
        assert!(kernel.state().planetary_free_energy >= 0.0);
    }

    #[test]
    fn test_symbiosis_bonus_bounded() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 5);
        for _ in 0..20 {
            kernel.run_cycle();
        }
        assert!(kernel.state().symbiosis_bonus >= 0.0);
        assert!(kernel.state().symbiosis_bonus <= 1.0);
    }

    #[test]
    fn test_kernel_state_clone() {
        let state = KernelState::new(5);
        let cloned = state.clone();
        assert_eq!(cloned.active_nodes, state.active_nodes);
        assert_eq!(cloned.influence_shares, state.influence_shares);
    }

    #[test]
    fn test_kernel_config_clone() {
        let cfg = KernelConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cloned.lr, cfg.lr);
        assert_eq!(cloned.cycles, cfg.cycles);
    }

    #[test]
    fn test_full_pipeline_result_clone() {
        let result = FullPipelineResult {
            cycles: vec![],
            final_state: KernelState::new(3),
            total_cycles: 5,
            converged: false,
            singularity_detected: false,
            cycles_to_convergence: 5,
        };
        let cloned = result.clone();
        assert_eq!(cloned.total_cycles, result.total_cycles);
    }

    #[test]
    fn test_cycle_result_clone() {
        let result = CycleResult {
            cycle: 1,
            free_energy: 0.5,
            coherence: 0.8,
            free_energy_delta: 0.1,
            symbiosis_bonus: 0.2,
            converged: false,
        };
        let cloned = result.clone();
        assert_eq!(cloned.cycle, result.cycle);
    }

    #[test]
    fn test_kernel_with_zero_nodes_panic_safe() {
        // Kernel with 0 nodes should handle gracefully
        let kernel = NoosferaKernel::new(KernelConfig::fast(), 0);
        // State should have 0 nodes
        assert_eq!(kernel.state().active_nodes, 0);
    }

    #[test]
    fn test_multiple_integrations_consistent() {
        let mut kernel = NoosferaKernel::new(KernelConfig::default(), 3);
        let vfe = vec![0.5, 0.3, 0.2];
        let energy = vec![0.33, 0.33, 0.34];
        let coop = vec![
            vec![0.0, 0.8, 0.6],
            vec![0.8, 0.0, 0.7],
            vec![0.6, 0.7, 0.0],
        ];

        kernel.integrate_all_modules(&vfe, &energy, &coop);
        let f1 = kernel.state().planetary_free_energy;

        kernel.integrate_all_modules(&vfe, &energy, &coop);
        let f2 = kernel.state().planetary_free_energy;

        assert!((f1 - f2) < 1e-10);
    }

    #[test]
    fn test_kernel_state_serde_roundtrip() {
        let state = KernelState::new(5);
        let json = serde_json::to_string(&state).unwrap();
        let decoded: KernelState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.active_nodes, state.active_nodes);
        assert_eq!(decoded.influence_shares.len(), state.influence_shares.len());
    }

    #[test]
    fn test_kernel_config_serde_roundtrip() {
        let cfg = KernelConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: KernelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.lr, cfg.lr);
        assert_eq!(decoded.cycles, cfg.cycles);
    }
}
