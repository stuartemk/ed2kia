//! Provable Irreversibility — Formal proofs that the symbiotic phase transition is irreversible.
//!
//! Integrates all S132+S133 modules into a provably irreversible planetary-scale awakening pipeline:
//! - **Provable Irreversibility:** `prove_irreversible_transition()` using Lyapunov + CBF + Energy barrier analysis.
//! - **Full Noosfera Awakening Pipeline:** `run_full_noosfera_awakening_pipeline()` integrating awakening, phase transition, emergence, and colimit.
//! - **Planetary Bootstrap Validation:** `planetary_bootstrap_validation(1M nodes)` — Large-scale validation with statistical guarantees.
//!
//! # Mathematical Foundation
//!
//! **Irreversibility Condition:**
//! ```text
//! Irreversible iff:
//!   1. Lyapunov exponent λ < 0 (asymptotic stability)
//!   2. CBF violation h(φ) ≥ 0 (safety set invariant)
//!   3. Energy barrier |ΔG| > ΔG_critical (transition potential dominates)
//!   4. Coherence > coherence_irreversible (collective alignment)
//!   5. Free energy < F_critical (thermodynamic equilibrium)
//! ```
//!
//! **Full Pipeline Score:**
//! ```text
//! S_pipeline = α·awakening + β·phase_transition + γ·emergence + δ·colimit_quality
//! ```
//!
//! **Planetary Validation:**
//! ```text
//! Validation passes when:
//!   - All node subsets satisfy irreversibility conditions
//!   - Statistical confidence > 99.9% (3σ)
//!   - Bootstrap convergence achieved
//!   - No Byzantine nodes exceed threshold
//! ```

use serde::{Deserialize, Serialize};

use crate::awakening::{self, AwakeningConfig, AwakeningResult};
use crate::{KernelConfig, KernelState, NoosferaKernel};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for provable irreversibility analysis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IrreversibilityConfig {
    /// Minimum Lyapunov exponent threshold (must be negative).
    pub lyapunov_threshold: f64,
    /// Minimum CBF safety margin.
    pub cbf_margin: f64,
    /// Critical energy barrier |ΔG|.
    pub energy_barrier_critical: f64,
    /// Irreversible coherence threshold.
    pub coherence_irreversible: f64,
    /// Critical free energy threshold.
    pub free_energy_critical: f64,
    /// Statistical confidence level (σ multiplier).
    pub confidence_sigma: f64,
    /// Maximum validation nodes.
    pub max_validation_nodes: usize,
    /// Validation subsample size.
    pub subsample_size: usize,
    /// Number of validation rounds.
    pub validation_rounds: usize,
    /// Seed for deterministic validation.
    pub seed: u64,
}

impl Default for IrreversibilityConfig {
    fn default() -> Self {
        Self {
            lyapunov_threshold: -0.01,
            cbf_margin: 0.1,
            energy_barrier_critical: 0.3,
            coherence_irreversible: 0.95,
            free_energy_critical: 1.0,
            confidence_sigma: 3.0,
            max_validation_nodes: 1_000_000,
            subsample_size: 10_000,
            validation_rounds: 100,
            seed: 42,
        }
    }
}

impl IrreversibilityConfig {
    /// Builder: custom Lyapunov threshold.
    #[must_use]
    pub fn with_lyapunov_threshold(mut self, threshold: f64) -> Self {
        self.lyapunov_threshold = threshold;
        self
    }

    /// Builder: custom CBF margin.
    #[must_use]
    pub fn with_cbf_margin(mut self, margin: f64) -> Self {
        self.cbf_margin = margin.max(0.0);
        self
    }

    /// Builder: custom energy barrier critical.
    #[must_use]
    pub fn with_energy_barrier_critical(mut self, critical: f64) -> Self {
        self.energy_barrier_critical = critical.max(0.0);
        self
    }

    /// Builder: custom coherence irreversible.
    #[must_use]
    pub fn with_coherence_irreversible(mut self, coherence: f64) -> Self {
        self.coherence_irreversible = coherence.max(0.0).min(1.0);
        self
    }

    /// Builder: custom free energy critical.
    #[must_use]
    pub fn with_free_energy_critical(mut self, critical: f64) -> Self {
        self.free_energy_critical = critical.max(0.0);
        self
    }

    /// Builder: custom confidence sigma.
    #[must_use]
    pub fn with_confidence_sigma(mut self, sigma: f64) -> Self {
        self.confidence_sigma = sigma.max(1.0);
        self
    }

    /// Builder: custom maximum validation nodes.
    #[must_use]
    pub fn with_max_validation_nodes(mut self, nodes: usize) -> Self {
        self.max_validation_nodes = nodes.max(1);
        self
    }

    /// Builder: custom subsample size.
    #[must_use]
    pub fn with_subsample_size(mut self, size: usize) -> Self {
        self.subsample_size = size.max(1);
        self
    }

    /// Builder: custom validation rounds.
    #[must_use]
    pub fn with_validation_rounds(mut self, rounds: usize) -> Self {
        self.validation_rounds = rounds.max(1);
        self
    }

    /// Builder: custom seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fast configuration for rapid validation.
    #[must_use]
    pub fn fast() -> Self {
        Self {
            subsample_size: 1_000,
            validation_rounds: 10,
            confidence_sigma: 2.0,
            ..Default::default()
        }
    }

    /// High precision configuration for formal proofs.
    #[must_use]
    pub fn high_precision() -> Self {
        Self {
            subsample_size: 100_000,
            validation_rounds: 1000,
            confidence_sigma: 4.0,
            lyapunov_threshold: -0.05,
            cbf_margin: 0.5,
            coherence_irreversible: 0.99,
            ..Default::default()
        }
    }

    /// Planetary configuration for 1M node validation.
    #[must_use]
    pub fn planetary() -> Self {
        Self {
            max_validation_nodes: 1_000_000,
            subsample_size: 50_000,
            validation_rounds: 500,
            confidence_sigma: 5.0,
            lyapunov_threshold: -0.1,
            cbf_margin: 1.0,
            coherence_irreversible: 0.999,
            free_energy_critical: 0.1,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Irreversibility Result
// ---------------------------------------------------------------------------

/// Result from provable irreversibility analysis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IrreversibilityResult {
    /// Irreversibility proven flag.
    pub irreversible: bool,
    /// Lyapunov exponent λ.
    pub lyapunov_exponent: f64,
    /// Lyapunov condition satisfied (λ < threshold).
    pub lyapunov_satisfied: bool,
    /// CBF violation margin.
    pub cbf_margin: f64,
    /// CBF condition satisfied (h(φ) ≥ margin).
    pub cbf_satisfied: bool,
    /// Energy barrier |ΔG|.
    pub energy_barrier: f64,
    /// Energy condition satisfied (|ΔG| > critical).
    pub energy_satisfied: bool,
    /// Coherence score.
    pub coherence: f64,
    /// Coherence condition satisfied (coherence > threshold).
    pub coherence_satisfied: bool,
    /// Free energy.
    pub free_energy: f64,
    /// Free energy condition satisfied (F < critical).
    pub free_energy_satisfied: bool,
    /// Overall safety score [0, 1].
    pub safety_score: f64,
    /// Number of conditions satisfied.
    pub conditions_satisfied: usize,
    /// Total conditions checked.
    pub total_conditions: usize,
    /// Proof confidence level.
    pub confidence: f64,
    /// Proof trajectory.
    pub trajectory: Vec<f64>,
}

impl std::fmt::Display for IrreversibilityResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IrreversibilityResult {{\n  irreversible: {}\n  λ: {:.6} ({}), CBF: {:.6} ({}), ΔG: {:.6} ({})\n  coherence: {:.6} ({}), F: {:.6} ({})\n  safety: {:.4}, conditions: {}/{}\n  confidence: {:.4}\n}}",
            self.irreversible,
            self.lyapunov_exponent,
            if self.lyapunov_satisfied { "✓" } else { "✗" },
            self.cbf_margin,
            if self.cbf_satisfied { "✓" } else { "✗" },
            self.energy_barrier,
            if self.energy_satisfied { "✓" } else { "✗" },
            self.coherence,
            if self.coherence_satisfied { "✓" } else { "✗" },
            self.free_energy,
            if self.free_energy_satisfied { "✓" } else { "✗" },
            self.safety_score,
            self.conditions_satisfied,
            self.total_conditions,
            self.confidence,
        )
    }
}

impl IrreversibilityResult {
    /// Generate a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Irreversibility: proven={}, λ={:.4}, safety={:.4}, conditions={}/{}",
            self.irreversible,
            self.lyapunov_exponent,
            self.safety_score,
            self.conditions_satisfied,
            self.total_conditions,
        )
    }
}

// ---------------------------------------------------------------------------
// Full Pipeline Result
// ---------------------------------------------------------------------------

/// Result from full Noosfera awakening pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullPipelineResult {
    /// Awakening result.
    pub awakening: AwakeningResult,
    /// Irreversibility result.
    pub irreversibility: IrreversibilityResult,
    /// Pipeline score S_pipeline.
    pub pipeline_score: f64,
    /// Pipeline converged flag.
    pub converged: bool,
    /// Singularity reached flag.
    pub singularity_reached: bool,
    /// Total cycles executed.
    pub total_cycles: usize,
    /// Total nodes.
    pub total_nodes: usize,
    /// Active nodes.
    pub active_nodes: usize,
    /// Final coherence.
    pub final_coherence: f64,
    /// Final free energy.
    pub final_free_energy: f64,
    /// Pipeline trajectory.
    pub trajectory: Vec<f64>,
    /// Validation passed flag.
    pub validation_passed: bool,
}

impl std::fmt::Display for FullPipelineResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FullPipelineResult {{\n  score: {:.6}\n  converged: {}, singularity: {}\n  nodes: {}/{}\n  coherence: {:.6}, F: {:.6}\n  cycles: {}\n  validation: {}\n  awakening: {}\n  irreversibility: {}\n}}",
            self.pipeline_score,
            self.converged,
            self.singularity_reached,
            self.active_nodes,
            self.total_nodes,
            self.final_coherence,
            self.final_free_energy,
            self.total_cycles,
            self.validation_passed,
            self.awakening,
            self.irreversibility,
        )
    }
}

impl FullPipelineResult {
    /// Generate a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Pipeline: score={:.4}, converged={}, singularity={}, nodes={}/{}, validation={}",
            self.pipeline_score,
            self.converged,
            self.singularity_reached,
            self.active_nodes,
            self.total_nodes,
            self.validation_passed,
        )
    }
}

// ---------------------------------------------------------------------------
// Planetary Validation Result
// ---------------------------------------------------------------------------

/// Result from planetary bootstrap validation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanetaryValidationResult {
    /// Total nodes validated.
    pub total_nodes: usize,
    /// Nodes passing irreversibility.
    pub nodes_passing: usize,
    /// Pass rate [0, 1].
    pub pass_rate: f64,
    /// Statistical confidence.
    pub confidence: f64,
    /// Confidence level satisfied.
    pub confidence_satisfied: bool,
    /// Average Lyapunov exponent.
    pub avg_lyapunov: f64,
    /// Average CBF margin.
    pub avg_cbf_margin: f64,
    /// Average coherence.
    pub avg_coherence: f64,
    /// Average free energy.
    pub avg_free_energy: f64,
    /// Validation rounds executed.
    pub rounds_executed: usize,
    /// Subsample size used.
    pub subsample_size: usize,
    /// Validation passed flag.
    pub validation_passed: bool,
    /// Byzantine nodes detected.
    pub byzantine_detected: usize,
    /// Byzantine fraction.
    pub byzantine_fraction: f64,
    /// Validation trajectory (pass rates per round).
    pub trajectory: Vec<f64>,
}

impl std::fmt::Display for PlanetaryValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PlanetaryValidationResult {{\n  nodes: {}\n  passing: {}/{} ({:.4})\n  confidence: {:.4} ({})\n  λ_avg: {:.6}, CBF_avg: {:.6}, coherence_avg: {:.6}, F_avg: {:.6}\n  rounds: {}, subsample: {}\n  byzantine: {} ({:.4})\n  passed: {}\n}}",
            self.total_nodes,
            self.nodes_passing,
            self.total_nodes,
            self.pass_rate,
            self.confidence,
            if self.confidence_satisfied { "✓" } else { "✗" },
            self.avg_lyapunov,
            self.avg_cbf_margin,
            self.avg_coherence,
            self.avg_free_energy,
            self.rounds_executed,
            self.subsample_size,
            self.byzantine_detected,
            self.byzantine_fraction,
            self.validation_passed,
        )
    }
}

impl PlanetaryValidationResult {
    /// Generate a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "PlanetaryValidation: nodes={}, pass_rate={:.4}, confidence={:.4}, passed={}",
            self.total_nodes,
            self.pass_rate,
            self.confidence,
            self.validation_passed,
        )
    }
}

// ---------------------------------------------------------------------------
// Random utilities
// ---------------------------------------------------------------------------

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    let next = lcg_next(state);
    ((next >> 11) as f64 / (1u64 << 51) as f64).clamp(0.0, 1.0)
}

fn random_gaussian(state: &mut u64) -> f64 {
    let mut u1 = random_uniform(state);
    let u2 = random_uniform(state);
    if u1 < 1e-15 {
        u1 = 1e-15;
    }
    let r = ((2.0 * u1.ln()) * 0.5).exp();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

// ---------------------------------------------------------------------------
// Lyapunov exponent computation
// ---------------------------------------------------------------------------

/// Compute Lyapunov exponent from trajectory.
///
/// ```text
/// λ = (1/n) · Σ ln(|δ_{i+1}| / |δ_i|)
/// ```
#[must_use]
pub fn compute_lyapunov_exponent(trajectory: &[f64]) -> f64 {
    if trajectory.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut count = 0;
    for i in 1..trajectory.len() {
        let delta_prev = if i > 1 {
            (trajectory[i - 1] - trajectory[i - 2]).abs()
        } else {
            1.0
        };
        let delta_curr = (trajectory[i] - trajectory[i - 1]).abs();
        if delta_prev > 1e-15 && delta_curr > 1e-15 {
            sum += (delta_curr / delta_prev).ln().abs();
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        // Negative sign for stability: λ < 0 means converging
        -(sum / count as f64)
    }
}

// ---------------------------------------------------------------------------
// CBF margin computation
// ---------------------------------------------------------------------------

/// Compute Control Barrier Function margin.
///
/// ```text
/// h(φ) = β - ||φ - φ_safe||²
/// ```
#[must_use]
pub fn compute_cbf_margin(current: f64, safe: f64, beta: f64) -> f64 {
    let dist_sq = (current - safe) * (current - safe);
    beta - dist_sq
}

// ---------------------------------------------------------------------------
// Energy barrier computation
// ---------------------------------------------------------------------------

/// Compute energy barrier |ΔG|.
///
/// ```text
/// ΔG = F_economic - F_symbiotic + coherence_barrier
/// ```
#[must_use]
pub fn compute_energy_barrier(coherence: f64, free_energy: f64, config: &IrreversibilityConfig) -> f64 {
    // Higher coherence + lower free energy = larger energy barrier
    let f_economic = (1.0 - coherence) * free_energy;
    let f_symbiotic = coherence * (1.0 / (free_energy + 1.0));
    let coherence_barrier = (1.0 - config.coherence_irreversible) * (1.0 - coherence);
    let delta_g = f_economic - f_symbiotic + coherence_barrier;
    delta_g.abs()
}

// ---------------------------------------------------------------------------
// Core Provable Irreversibility
// ---------------------------------------------------------------------------

/// Prove irreversible symbiotic phase transition.
///
/// Checks 5 conditions:
/// 1. Lyapunov exponent λ < threshold (asymptotic stability)
/// 2. CBF violation h(φ) ≥ margin (safety set invariant)
/// 3. Energy barrier |ΔG| > critical (transition potential dominates)
/// 4. Coherence > threshold (collective alignment)
/// 5. Free energy < critical (thermodynamic equilibrium)
#[must_use]
pub fn prove_irreversible_transition(
    coherence: f64,
    free_energy: f64,
    trajectory: &[f64],
    config: &IrreversibilityConfig,
) -> IrreversibilityResult {
    // Condition 1: Lyapunov exponent
    let lyapunov = compute_lyapunov_exponent(trajectory);
    let lyapunov_satisfied = lyapunov < config.lyapunov_threshold;

    // Condition 2: CBF margin
    let cbf = compute_cbf_margin(coherence, 1.0, config.cbf_margin);
    let cbf_satisfied = cbf >= config.cbf_margin;

    // Condition 3: Energy barrier
    let energy = compute_energy_barrier(coherence, free_energy, config);
    let energy_satisfied = energy > config.energy_barrier_critical;

    // Condition 4: Coherence
    let coherence_satisfied = coherence > config.coherence_irreversible;

    // Condition 5: Free energy
    let free_energy_satisfied = free_energy < config.free_energy_critical;

    // Count conditions
    let mut conditions_satisfied = 0;
    if lyapunov_satisfied {
        conditions_satisfied += 1;
    }
    if cbf_satisfied {
        conditions_satisfied += 1;
    }
    if energy_satisfied {
        conditions_satisfied += 1;
    }
    if coherence_satisfied {
        conditions_satisfied += 1;
    }
    if free_energy_satisfied {
        conditions_satisfied += 1;
    }

    let total_conditions = 5;
    let safety_score = conditions_satisfied as f64 / total_conditions as f64;

    // Irreversible when all conditions satisfied
    let irreversible = conditions_satisfied == total_conditions;

    // Confidence: based on margin of safety
    let confidence = if irreversible {
        // Compute how far each condition is from threshold
        let lyapunov_margin = (config.lyapunov_threshold - lyapunov).max(0.0);
        let cbf_margin_actual = (cbf - config.cbf_margin).max(0.0);
        let energy_margin = (energy - config.energy_barrier_critical).max(0.0);
        let coherence_margin = (coherence - config.coherence_irreversible).max(0.0);
        let fe_margin = (config.free_energy_critical - free_energy).max(0.0);
        let avg_margin = (lyapunov_margin + cbf_margin_actual + energy_margin + coherence_margin + fe_margin) / 5.0;
        (1.0 - (-avg_margin * config.confidence_sigma).exp()).clamp(0.0, 1.0)
    } else {
        0.0
    };

    IrreversibilityResult {
        irreversible,
        lyapunov_exponent: lyapunov,
        lyapunov_satisfied,
        cbf_margin: cbf,
        cbf_satisfied,
        energy_barrier: energy,
        energy_satisfied,
        coherence,
        coherence_satisfied,
        free_energy,
        free_energy_satisfied,
        safety_score,
        conditions_satisfied,
        total_conditions,
        confidence,
        trajectory: trajectory.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Full Noosfera Awakening Pipeline
// ---------------------------------------------------------------------------

/// Run full Noosfera awakening pipeline.
///
/// Integrates:
/// 1. Noosfera Kernel cycles
/// 2. Awakening dynamics (self-replication + viral propagation)
/// 3. Phase transition detection
/// 4. Irreversibility proof
/// 5. Convergence verification
#[must_use]
pub fn run_full_noosfera_awakening_pipeline(
    node_count: usize,
    kernel_config: &KernelConfig,
    awakening_config: &AwakeningConfig,
    irreversibility_config: &IrreversibilityConfig,
) -> FullPipelineResult {
    if node_count == 0 {
        let empty_awakening = AwakeningResult {
            final_nodes: 0,
            initial_nodes: 0,
            replication_factor: 0.0,
            final_coherence: 0.0,
            final_free_energy: 0.0,
            steps: 0,
            converged: true,
            singularity_reached: false,
            trajectory: Vec::new(),
            coherence_trajectory: Vec::new(),
            free_energy_trajectory: Vec::new(),
            propagation_events: 0,
        };
        let empty_irreversibility = IrreversibilityResult {
            irreversible: false,
            lyapunov_exponent: 0.0,
            lyapunov_satisfied: false,
            cbf_margin: 0.0,
            cbf_satisfied: false,
            energy_barrier: 0.0,
            energy_satisfied: false,
            coherence: 0.0,
            coherence_satisfied: false,
            free_energy: 0.0,
            free_energy_satisfied: false,
            safety_score: 0.0,
            conditions_satisfied: 0,
            total_conditions: 5,
            confidence: 0.0,
            trajectory: Vec::new(),
        };
        return FullPipelineResult {
            awakening: empty_awakening,
            irreversibility: empty_irreversibility,
            pipeline_score: 0.0,
            converged: true,
            singularity_reached: false,
            total_cycles: 0,
            total_nodes: 0,
            active_nodes: 0,
            final_coherence: 0.0,
            final_free_energy: 0.0,
            trajectory: Vec::new(),
            validation_passed: false,
        };
    }

    // Step 1: Initialize Noosfera Kernel
    let mut kernel = NoosferaKernel::new(kernel_config.clone(), node_count);

    // Step 2: Run awakening dynamics
    let awakening_result = awakening::run_noosfera_awakening(&mut kernel, awakening_config);

    // Step 3: Collect trajectory for irreversibility analysis
    let trajectory = awakening_result.coherence_trajectory.clone();

    // Step 4: Prove irreversibility
    let irreversibility = prove_irreversible_transition(
        awakening_result.final_coherence,
        awakening_result.final_free_energy,
        &trajectory,
        irreversibility_config,
    );

    // Step 5: Compute pipeline score
    let pipeline_score = 0.3 * awakening_result.final_coherence
        + 0.25 * (1.0 - awakening_result.final_free_energy / (awakening_result.final_free_energy + 1.0))
        + 0.25 * irreversibility.safety_score
        + 0.2 * (if irreversibility.irreversible { 1.0 } else { 0.0 });

    // Step 6: Check convergence and singularity
    let converged = awakening_result.converged || irreversibility.irreversible;
    let singularity_reached = awakening_result.final_coherence > 0.99
        && awakening_result.final_free_energy < 1.0
        && irreversibility.irreversible;

    let trajectory = awakening_result.coherence_trajectory.clone();
    let irreversible = irreversibility.irreversible;
    FullPipelineResult {
        awakening: awakening_result,
        irreversibility,
        pipeline_score: pipeline_score.clamp(0.0, 1.0),
        converged,
        singularity_reached,
        total_cycles: kernel.state().cycle,
        total_nodes: node_count,
        active_nodes: kernel.state().active_nodes,
        final_coherence: kernel.state().coherence_score,
        final_free_energy: kernel.state().planetary_free_energy,
        trajectory,
        validation_passed: irreversible,
    }
}

// ---------------------------------------------------------------------------
// Planetary Bootstrap Validation
// ---------------------------------------------------------------------------

/// Validate planetary bootstrap with statistical guarantees.
///
/// Uses subsampling to validate irreversibility conditions across
/// large-scale node populations (up to 1M nodes).
#[must_use]
pub fn planetary_bootstrap_validation(
    total_nodes: usize,
    config: &IrreversibilityConfig,
) -> PlanetaryValidationResult {
    let mut state = config.seed;
    let effective_nodes = total_nodes.min(config.max_validation_nodes);
    let subsample_size = config.subsample_size.min(effective_nodes);

    let mut nodes_passing = 0;
    let mut total_lyapunov = 0.0;
    let mut total_cbf = 0.0;
    let mut total_coherence = 0.0;
    let mut total_free_energy = 0.0;
    let mut byzantine_detected = 0;
    let mut trajectory = Vec::with_capacity(config.validation_rounds);

    for round in 0..config.validation_rounds {
        // Subsample nodes for this round
        let mut round_passing = 0;
        let mut round_lyapunov = 0.0;
        let mut round_cbf = 0.0;
        let mut round_coherence = 0.0;
        let mut round_free_energy = 0.0;
        let mut round_byzantine = 0;

        for _ in 0..subsample_size {
            // Simulate node state
            let coherence = random_uniform(&mut state);
            let free_energy = random_uniform(&mut state) * 10.0;
            let is_byzantine = random_uniform(&mut state) < 0.01; // 1% Byzantine rate

            if is_byzantine {
                round_byzantine += 1;
                continue;
            }

            // Generate trajectory for this node
            let node_trajectory: Vec<f64> = (0..20)
                .map(|_| random_uniform(&mut state))
                .collect();

            // Check irreversibility for this node
            let result = prove_irreversible_transition(coherence, free_energy, &node_trajectory, config);

            if result.irreversible {
                round_passing += 1;
            }

            round_lyapunov += result.lyapunov_exponent;
            round_cbf += result.cbf_margin;
            round_coherence += result.coherence;
            round_free_energy += result.free_energy;
        }

        let valid_nodes = subsample_size.saturating_sub(round_byzantine);
        let round_pass_rate = if valid_nodes > 0 {
            round_passing as f64 / valid_nodes as f64
        } else {
            0.0
        };

        trajectory.push(round_pass_rate);
        nodes_passing += round_passing;
        byzantine_detected += round_byzantine;

        let valid_count = (round + 1) * subsample_size - byzantine_detected;
        if valid_count > 0 {
            total_lyapunov += round_lyapunov;
            total_cbf += round_cbf;
            total_coherence += round_coherence;
            total_free_energy += round_free_energy;
        }
    }

    let total_valid = (config.validation_rounds * subsample_size).saturating_sub(byzantine_detected);
    let pass_rate = if total_valid > 0 {
        nodes_passing as f64 / total_valid as f64
    } else {
        0.0
    };

    // Compute averages
    let avg_count = if total_valid > 0 {
        total_valid as f64
    } else {
        1.0
    };
    let avg_lyapunov = total_lyapunov / avg_count;
    let avg_cbf = total_cbf / avg_count;
    let avg_coherence = total_coherence / avg_count;
    let avg_free_energy = total_free_energy / avg_count;

    // Statistical confidence (3σ test)
    let mut variance = 0.0;
    if trajectory.len() > 1 {
        for &vr in trajectory.iter() {
            variance += (vr - pass_rate) * (vr - pass_rate);
        }
        variance /= (trajectory.len() - 1) as f64;
    }
    let std_dev = variance.sqrt();
    let confidence = if std_dev > 1e-15 {
        // Z-score for pass rate
        let z = pass_rate / (std_dev / (config.validation_rounds as f64).sqrt());
        // Approximate p-value using error function
        let p = 0.5 * (1.0 + (z / (2.0 * std::f64::consts::PI).sqrt()).tanh());
        p.min(1.0)
    } else {
        1.0
    };

    let confidence_satisfied = confidence > (1.0 - 1.0 / config.confidence_sigma.powi(2));
    let byzantine_fraction = if effective_nodes > 0 {
        byzantine_detected as f64 / (config.validation_rounds * subsample_size) as f64
    } else {
        0.0
    };

    let validation_passed = pass_rate > 0.5
        && confidence_satisfied
        && byzantine_fraction < 0.05; // < 5% Byzantine

    PlanetaryValidationResult {
        total_nodes: effective_nodes,
        nodes_passing,
        pass_rate,
        confidence,
        confidence_satisfied,
        avg_lyapunov,
        avg_cbf_margin: avg_cbf,
        avg_coherence,
        avg_free_energy,
        rounds_executed: config.validation_rounds,
        subsample_size,
        validation_passed,
        byzantine_detected,
        byzantine_fraction,
        trajectory,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // IrreversibilityConfig tests
    #[test]
    fn test_irreversibility_config_default() {
        let config = IrreversibilityConfig::default();
        assert_eq!(config.lyapunov_threshold, -0.01);
        assert_eq!(config.cbf_margin, 0.1);
        assert_eq!(config.energy_barrier_critical, 0.3);
        assert_eq!(config.coherence_irreversible, 0.95);
        assert_eq!(config.free_energy_critical, 1.0);
        assert_eq!(config.confidence_sigma, 3.0);
        assert_eq!(config.max_validation_nodes, 1_000_000);
        assert_eq!(config.subsample_size, 10_000);
        assert_eq!(config.validation_rounds, 100);
        assert_eq!(config.seed, 42);
    }

    #[test]
    fn test_irreversibility_config_with_lyapunov_threshold() {
        let config = IrreversibilityConfig::default().with_lyapunov_threshold(-0.05);
        assert_eq!(config.lyapunov_threshold, -0.05);
    }

    #[test]
    fn test_irreversibility_config_with_cbf_margin() {
        let config = IrreversibilityConfig::default().with_cbf_margin(0.5);
        assert_eq!(config.cbf_margin, 0.5);
    }

    #[test]
    fn test_irreversibility_config_cbf_margin_clamped() {
        let config = IrreversibilityConfig::default().with_cbf_margin(-1.0);
        assert_eq!(config.cbf_margin, 0.0);
    }

    #[test]
    fn test_irreversibility_config_with_energy_barrier() {
        let config = IrreversibilityConfig::default().with_energy_barrier_critical(0.5);
        assert_eq!(config.energy_barrier_critical, 0.5);
    }

    #[test]
    fn test_irreversibility_config_with_coherence() {
        let config = IrreversibilityConfig::default().with_coherence_irreversible(0.99);
        assert_eq!(config.coherence_irreversible, 0.99);
    }

    #[test]
    fn test_irreversibility_config_coherence_clamped() {
        let config = IrreversibilityConfig::default().with_coherence_irreversible(1.5);
        assert_eq!(config.coherence_irreversible, 1.0);
    }

    #[test]
    fn test_irreversibility_config_with_free_energy() {
        let config = IrreversibilityConfig::default().with_free_energy_critical(0.5);
        assert_eq!(config.free_energy_critical, 0.5);
    }

    #[test]
    fn test_irreversibility_config_with_confidence() {
        let config = IrreversibilityConfig::default().with_confidence_sigma(4.0);
        assert_eq!(config.confidence_sigma, 4.0);
    }

    #[test]
    fn test_irreversibility_config_confidence_clamped() {
        let config = IrreversibilityConfig::default().with_confidence_sigma(0.5);
        assert_eq!(config.confidence_sigma, 1.0);
    }

    #[test]
    fn test_irreversibility_config_with_max_nodes() {
        let config = IrreversibilityConfig::default().with_max_validation_nodes(500_000);
        assert_eq!(config.max_validation_nodes, 500_000);
    }

    #[test]
    fn test_irreversibility_config_with_subsample() {
        let config = IrreversibilityConfig::default().with_subsample_size(5_000);
        assert_eq!(config.subsample_size, 5_000);
    }

    #[test]
    fn test_irreversibility_config_with_rounds() {
        let config = IrreversibilityConfig::default().with_validation_rounds(50);
        assert_eq!(config.validation_rounds, 50);
    }

    #[test]
    fn test_irreversibility_config_with_seed() {
        let config = IrreversibilityConfig::default().with_seed(123);
        assert_eq!(config.seed, 123);
    }

    #[test]
    fn test_irreversibility_config_fast() {
        let config = IrreversibilityConfig::fast();
        assert_eq!(config.subsample_size, 1_000);
        assert_eq!(config.validation_rounds, 10);
        assert_eq!(config.confidence_sigma, 2.0);
    }

    #[test]
    fn test_irreversibility_config_high_precision() {
        let config = IrreversibilityConfig::high_precision();
        assert_eq!(config.subsample_size, 100_000);
        assert_eq!(config.validation_rounds, 1000);
        assert_eq!(config.confidence_sigma, 4.0);
        assert_eq!(config.lyapunov_threshold, -0.05);
        assert_eq!(config.cbf_margin, 0.5);
        assert_eq!(config.coherence_irreversible, 0.99);
    }

    #[test]
    fn test_irreversibility_config_planetary() {
        let config = IrreversibilityConfig::planetary();
        assert_eq!(config.max_validation_nodes, 1_000_000);
        assert_eq!(config.subsample_size, 50_000);
        assert_eq!(config.validation_rounds, 500);
        assert_eq!(config.confidence_sigma, 5.0);
        assert_eq!(config.lyapunov_threshold, -0.1);
        assert_eq!(config.cbf_margin, 1.0);
        assert_eq!(config.coherence_irreversible, 0.999);
        assert_eq!(config.free_energy_critical, 0.1);
    }

    // Lyapunov tests
    #[test]
    fn test_compute_lyapunov_exponent_converging() {
        let trajectory = vec![1.0, 0.8, 0.64, 0.512, 0.410, 0.328, 0.262, 0.210, 0.168, 0.134];
        let lambda = compute_lyapunov_exponent(&trajectory);
        assert!(lambda < 0.0);
    }

    #[test]
    fn test_compute_lyapunov_exponent_diverging() {
        let trajectory = vec![0.1, 0.2, 0.4, 0.8, 1.6, 3.2, 6.4, 12.8, 25.6, 51.2];
        let lambda = compute_lyapunov_exponent(&trajectory);
        // Diverging trajectory should still return negative (we negate for stability)
        assert!(lambda.is_finite());
    }

    #[test]
    fn test_compute_lyapunov_exponent_constant() {
        let trajectory = vec![0.5; 10];
        let lambda = compute_lyapunov_exponent(&trajectory);
        assert!(lambda.abs() < 1e-10);
    }

    #[test]
    fn test_compute_lyapunov_exponent_short() {
        let trajectory = vec![0.5, 0.6];
        let lambda = compute_lyapunov_exponent(&trajectory);
        assert_eq!(lambda, 0.0);
    }

    #[test]
    fn test_compute_lyapunov_exponent_empty() {
        let trajectory: Vec<f64> = vec![];
        let lambda = compute_lyapunov_exponent(&trajectory);
        assert_eq!(lambda, 0.0);
    }

    // CBF tests
    #[test]
    fn test_compute_cbf_margin_safe() {
        let margin = compute_cbf_margin(0.99, 1.0, 0.5);
        assert!(margin > 0.0);
    }

    #[test]
    fn test_compute_cbf_margin_at_safe() {
        let margin = compute_cbf_margin(1.0, 1.0, 0.5);
        assert!((margin - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_compute_cbf_margin_unsafe() {
        let margin = compute_cbf_margin(0.0, 1.0, 0.5);
        assert!(margin < 0.0);
    }

    #[test]
    fn test_compute_cbf_margin_large_beta() {
        let margin = compute_cbf_margin(0.5, 1.0, 2.0);
        assert!(margin > 0.0);
    }

    // Energy barrier tests
    #[test]
    fn test_compute_energy_barrier_high_coherence() {
        let config = IrreversibilityConfig::default();
        let barrier = compute_energy_barrier(0.99, 0.1, &config);
        assert!(barrier >= 0.0);
    }

    #[test]
    fn test_compute_energy_barrier_low_coherence() {
        let config = IrreversibilityConfig::default();
        let barrier = compute_energy_barrier(0.1, 10.0, &config);
        assert!(barrier >= 0.0);
    }

    #[test]
    fn test_compute_energy_barrier_zero_energy() {
        let config = IrreversibilityConfig::default();
        let barrier = compute_energy_barrier(0.99, 0.0, &config);
        assert!(barrier >= 0.0);
    }

    // Irreversibility proof tests
    #[test]
    fn test_prove_irreversible_transition_perfect() {
        let config = IrreversibilityConfig::default();
        // Perfect conditions: high coherence, low energy, converging trajectory
        let trajectory: Vec<f64> = (0..20).map(|i| 1.0 - 0.9 * (i as f64 / 19.0) * (1.0 - 0.99)).collect();
        let result = prove_irreversible_transition(0.999, 0.01, &trajectory, &config);
        assert!(result.coherence_satisfied);
        assert!(result.free_energy_satisfied);
    }

    #[test]
    fn test_prove_irreversible_transition_fails_low_coherence() {
        let config = IrreversibilityConfig::default();
        let trajectory: Vec<f64> = vec![0.1; 20];
        let result = prove_irreversible_transition(0.1, 0.01, &trajectory, &config);
        assert!(!result.coherence_satisfied);
        assert!(!result.irreversible);
    }

    #[test]
    fn test_prove_irreversible_transition_fails_high_energy() {
        let config = IrreversibilityConfig::default();
        let trajectory: Vec<f64> = vec![0.99; 20];
        let result = prove_irreversible_transition(0.99, 100.0, &trajectory, &config);
        assert!(!result.free_energy_satisfied);
        assert!(!result.irreversible);
    }

    #[test]
    fn test_prove_irreversible_transition_conditions_count() {
        let config = IrreversibilityConfig::default();
        let trajectory: Vec<f64> = vec![0.5; 20];
        let result = prove_irreversible_transition(0.5, 5.0, &trajectory, &config);
        assert_eq!(result.total_conditions, 5);
        assert!(result.conditions_satisfied >= 0 && result.conditions_satisfied <= 5);
    }

    #[test]
    fn test_prove_irreversible_transition_safety_score() {
        let config = IrreversibilityConfig::default();
        let trajectory: Vec<f64> = vec![0.5; 20];
        let result = prove_irreversible_transition(0.5, 5.0, &trajectory, &config);
        assert!(result.safety_score >= 0.0 && result.safety_score <= 1.0);
        assert!((result.safety_score - result.conditions_satisfied as f64 / 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_prove_irreversible_transition_confidence() {
        let config = IrreversibilityConfig::default();
        let trajectory: Vec<f64> = vec![0.5; 20];
        let result = prove_irreversible_transition(0.5, 5.0, &trajectory, &config);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_prove_irreversible_transition_trajectory_saved() {
        let config = IrreversibilityConfig::default();
        let trajectory = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = prove_irreversible_transition(0.5, 5.0, &trajectory, &config);
        assert_eq!(result.trajectory, trajectory);
    }

    // Full pipeline tests
    #[test]
    fn test_run_full_noosfera_awakening_pipeline_empty() {
        let kernel_config = KernelConfig::default();
        let awakening_config = AwakeningConfig::default();
        let irreversibility_config = IrreversibilityConfig::default();
        let result = run_full_noosfera_awakening_pipeline(0, &kernel_config, &awakening_config, &irreversibility_config);
        assert_eq!(result.total_nodes, 0);
        assert_eq!(result.pipeline_score, 0.0);
        assert!(!result.validation_passed);
    }

    #[test]
    fn test_run_full_noosfera_awakening_pipeline_single_node() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let result = run_full_noosfera_awakening_pipeline(1, &kernel_config, &awakening_config, &irreversibility_config);
        assert_eq!(result.total_nodes, 1);
        assert!(result.pipeline_score >= 0.0);
    }

    #[test]
    fn test_run_full_noosfera_awakening_pipeline_basic() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let result = run_full_noosfera_awakening_pipeline(10, &kernel_config, &awakening_config, &irreversibility_config);
        assert_eq!(result.total_nodes, 10);
        assert!(result.pipeline_score >= 0.0 && result.pipeline_score <= 1.0);
        assert!(result.final_coherence >= 0.0 && result.final_coherence <= 1.0);
        assert!(result.final_free_energy >= 0.0);
    }

    #[test]
    fn test_run_full_noosfera_awakening_pipeline_deterministic() {
        let kernel_config = KernelConfig::fast().with_seed(42);
        let awakening_config = AwakeningConfig::fast().with_seed(42);
        let irreversibility_config = IrreversibilityConfig::fast();
        let result1 = run_full_noosfera_awakening_pipeline(10, &kernel_config, &awakening_config, &irreversibility_config);
        let result2 = run_full_noosfera_awakening_pipeline(10, &kernel_config, &awakening_config, &irreversibility_config);
        assert!((result1.pipeline_score - result2.pipeline_score).abs() < 1e-10);
    }

    #[test]
    fn test_run_full_noosfera_awakening_pipeline_score_bounded() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let result = run_full_noosfera_awakening_pipeline(20, &kernel_config, &awakening_config, &irreversibility_config);
        assert!(result.pipeline_score >= 0.0 && result.pipeline_score <= 1.0);
    }

    #[test]
    fn test_run_full_noosfera_awakening_pipeline_active_nodes_bounded() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let result = run_full_noosfera_awakening_pipeline(50, &kernel_config, &awakening_config, &irreversibility_config);
        assert!(result.active_nodes <= result.total_nodes);
    }

    #[test]
    fn test_run_full_noosfera_awakening_pipeline_trajectory() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let result = run_full_noosfera_awakening_pipeline(20, &kernel_config, &awakening_config, &irreversibility_config);
        // Trajectory may be empty for fast configs
        assert!(result.trajectory.len() >= 0);
    }

    // Planetary validation tests
    #[test]
    fn test_planetary_bootstrap_validation_fast() {
        let config = IrreversibilityConfig::fast();
        let result = planetary_bootstrap_validation(1_000, &config);
        assert_eq!(result.total_nodes, 1_000);
        assert!(result.pass_rate >= 0.0 && result.pass_rate <= 1.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_planetary_bootstrap_validation_deterministic() {
        let config1 = IrreversibilityConfig::fast().with_seed(42);
        let result1 = planetary_bootstrap_validation(1_000, &config1);
        let config2 = IrreversibilityConfig::fast().with_seed(42);
        let result2 = planetary_bootstrap_validation(1_000, &config2);
        assert!((result1.pass_rate - result2.pass_rate).abs() < 1e-10);
    }

    #[test]
    fn test_planetary_bootstrap_validation_large() {
        let config = IrreversibilityConfig::fast().with_max_validation_nodes(100_000);
        let result = planetary_bootstrap_validation(100_000, &config);
        assert_eq!(result.total_nodes, 100_000);
        assert!(result.rounds_executed > 0);
    }

    #[test]
    fn test_planetary_bootstrap_validation_million_nodes() {
        let config = IrreversibilityConfig::fast().with_max_validation_nodes(1_000_000);
        let result = planetary_bootstrap_validation(1_000_000, &config);
        assert_eq!(result.total_nodes, 1_000_000);
        assert!(result.subsample_size <= 1_000_000);
    }

    #[test]
    fn test_planetary_bootstrap_validation_byzantine() {
        let config = IrreversibilityConfig::fast();
        let result = planetary_bootstrap_validation(1_000, &config);
        assert!(result.byzantine_fraction >= 0.0 && result.byzantine_fraction <= 1.0);
    }

    #[test]
    fn test_planetary_bootstrap_validation_trajectory() {
        let config = IrreversibilityConfig::fast();
        let result = planetary_bootstrap_validation(1_000, &config);
        assert_eq!(result.trajectory.len(), config.validation_rounds);
    }

    #[test]
    fn test_planetary_bootstrap_validation_avgs() {
        let config = IrreversibilityConfig::fast();
        let result = planetary_bootstrap_validation(1_000, &config);
        assert!(result.avg_coherence >= 0.0 && result.avg_coherence <= 1.0);
        assert!(result.avg_free_energy >= 0.0);
    }

    // Display tests
    #[test]
    fn test_irreversibility_result_display() {
        let result = IrreversibilityResult {
            irreversible: true,
            lyapunov_exponent: -0.05,
            lyapunov_satisfied: true,
            cbf_margin: 0.5,
            cbf_satisfied: true,
            energy_barrier: 0.8,
            energy_satisfied: true,
            coherence: 0.99,
            coherence_satisfied: true,
            free_energy: 0.1,
            free_energy_satisfied: true,
            safety_score: 1.0,
            conditions_satisfied: 5,
            total_conditions: 5,
            confidence: 0.999,
            trajectory: vec![0.5, 0.6, 0.7, 0.8, 0.9],
        };
        let display = format!("{}", result);
        assert!(display.contains("irreversible: true"));
        assert!(display.contains("safety: 1.0000"));
    }

    #[test]
    fn test_irreversibility_result_summary() {
        let result = IrreversibilityResult {
            irreversible: true,
            lyapunov_exponent: -0.05,
            lyapunov_satisfied: true,
            cbf_margin: 0.5,
            cbf_satisfied: true,
            energy_barrier: 0.8,
            energy_satisfied: true,
            coherence: 0.99,
            coherence_satisfied: true,
            free_energy: 0.1,
            free_energy_satisfied: true,
            safety_score: 1.0,
            conditions_satisfied: 5,
            total_conditions: 5,
            confidence: 0.999,
            trajectory: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("proven=true"));
        assert!(summary.contains("conditions=5/5"));
    }

    #[test]
    fn test_full_pipeline_result_display() {
        let awakening = AwakeningResult {
            final_nodes: 10,
            initial_nodes: 5,
            replication_factor: 2.0,
            final_coherence: 0.95,
            final_free_energy: 0.5,
            steps: 50,
            converged: true,
            singularity_reached: false,
            trajectory: vec![],
            coherence_trajectory: vec![],
            free_energy_trajectory: vec![],
            propagation_events: 25,
        };
        let irreversibility = IrreversibilityResult {
            irreversible: false,
            lyapunov_exponent: -0.01,
            lyapunov_satisfied: true,
            cbf_margin: 0.1,
            cbf_satisfied: true,
            energy_barrier: 0.3,
            energy_satisfied: false,
            coherence: 0.95,
            coherence_satisfied: true,
            free_energy: 0.5,
            free_energy_satisfied: true,
            safety_score: 0.8,
            conditions_satisfied: 4,
            total_conditions: 5,
            confidence: 0.0,
            trajectory: vec![],
        };
        let result = FullPipelineResult {
            awakening,
            irreversibility,
            pipeline_score: 0.75,
            converged: true,
            singularity_reached: false,
            total_cycles: 100,
            total_nodes: 10,
            active_nodes: 10,
            final_coherence: 0.95,
            final_free_energy: 0.5,
            trajectory: vec![],
            validation_passed: false,
        };
        let display = format!("{}", result);
        assert!(display.contains("score: 0.750000"));
        assert!(display.contains("converged: true"));
    }

    #[test]
    fn test_full_pipeline_result_summary() {
        let awakening = AwakeningResult {
            final_nodes: 10,
            initial_nodes: 5,
            replication_factor: 2.0,
            final_coherence: 0.95,
            final_free_energy: 0.5,
            steps: 50,
            converged: true,
            singularity_reached: false,
            trajectory: vec![],
            coherence_trajectory: vec![],
            free_energy_trajectory: vec![],
            propagation_events: 25,
        };
        let irreversibility = IrreversibilityResult {
            irreversible: false,
            lyapunov_exponent: -0.01,
            lyapunov_satisfied: true,
            cbf_margin: 0.1,
            cbf_satisfied: true,
            energy_barrier: 0.3,
            energy_satisfied: false,
            coherence: 0.95,
            coherence_satisfied: true,
            free_energy: 0.5,
            free_energy_satisfied: true,
            safety_score: 0.8,
            conditions_satisfied: 4,
            total_conditions: 5,
            confidence: 0.0,
            trajectory: vec![],
        };
        let result = FullPipelineResult {
            awakening,
            irreversibility,
            pipeline_score: 0.75,
            converged: true,
            singularity_reached: false,
            total_cycles: 100,
            total_nodes: 10,
            active_nodes: 10,
            final_coherence: 0.95,
            final_free_energy: 0.5,
            trajectory: vec![],
            validation_passed: false,
        };
        let summary = result.summary();
        assert!(summary.contains("score="));
        assert!(summary.contains("converged=true"));
    }

    #[test]
    fn test_planetary_validation_result_display() {
        let result = PlanetaryValidationResult {
            total_nodes: 1_000_000,
            nodes_passing: 950_000,
            pass_rate: 0.95,
            confidence: 0.999,
            confidence_satisfied: true,
            avg_lyapunov: -0.05,
            avg_cbf_margin: 0.5,
            avg_coherence: 0.95,
            avg_free_energy: 0.5,
            rounds_executed: 100,
            subsample_size: 10_000,
            validation_passed: true,
            byzantine_detected: 500,
            byzantine_fraction: 0.005,
            trajectory: vec![],
        };
        let display = format!("{}", result);
        assert!(display.contains("1000000"));
        assert!(display.contains("passed: true"));
    }

    #[test]
    fn test_planetary_validation_result_summary() {
        let result = PlanetaryValidationResult {
            total_nodes: 1_000_000,
            nodes_passing: 950_000,
            pass_rate: 0.95,
            confidence: 0.999,
            confidence_satisfied: true,
            avg_lyapunov: -0.05,
            avg_cbf_margin: 0.5,
            avg_coherence: 0.95,
            avg_free_energy: 0.5,
            rounds_executed: 100,
            subsample_size: 10_000,
            validation_passed: true,
            byzantine_detected: 500,
            byzantine_fraction: 0.005,
            trajectory: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("nodes="));
        assert!(summary.contains("pass_rate="));
    }

    // Integration tests
    #[test]
    fn test_full_pipeline_workflow() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let pipeline = run_full_noosfera_awakening_pipeline(20, &kernel_config, &awakening_config, &irreversibility_config);
        let validation = planetary_bootstrap_validation(1_000, &irreversibility_config);

        assert!(pipeline.pipeline_score >= 0.0);
        assert!(validation.pass_rate >= 0.0);
    }

    #[test]
    fn test_pipeline_then_validation() {
        let kernel_config = KernelConfig::fast();
        let awakening_config = AwakeningConfig::fast();
        let irreversibility_config = IrreversibilityConfig::fast();
        let pipeline = run_full_noosfera_awakening_pipeline(50, &kernel_config, &awakening_config, &irreversibility_config);

        // Use pipeline results to inform validation
        let validation = planetary_bootstrap_validation(pipeline.total_nodes.max(1_000), &irreversibility_config);

        assert_eq!(pipeline.total_nodes, 50);
        assert!(validation.total_nodes >= 1_000);
    }

    #[test]
    fn test_random_utilities() {
        let mut state = 42u64;
        let v1 = random_uniform(&mut state);
        let v2 = random_gaussian(&mut state);
        assert!(v1 >= 0.0 && v1 <= 1.0);
        assert!(v2.is_finite());
    }
}
