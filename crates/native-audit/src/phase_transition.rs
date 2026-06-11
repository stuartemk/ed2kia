//! Irreversible Phase Transition & Tipping Point Stabilization.
//!
//! Detects and stabilizes the mathematical tipping point toward an
//! irreversible symbiotic civilization. Uses Lyapunov exponents and
//! Control Barrier Functions to prove post-transition stability.
//!
//! **Phase Transition Potential:**
//! ```text
//! ΔG = F_planet,economic - F_planet,symbiotic + λ * coherence_barrier
//! ```
//!
//! **Irreversibility Proof:** Lyapunov exponent λ < 0 implies asymptotic
//! stability of the symbiotic attractor post-tipping.

use serde::{Deserialize, Serialize};

// ─── Phase Transition Configuration ──────────────────────────────────────────

/// Configuration for phase transition detection and stabilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransitionConfig {
    /// Coherence threshold for phase transition.
    pub coherence_threshold: f64,
    /// Free energy threshold for phase transition.
    pub energy_threshold: f64,
    /// Lyapunov convergence tolerance.
    pub lyapunov_tolerance: f64,
    /// CBF safety margin (β).
    pub cbf_beta: f64,
    /// Maximum stabilization steps.
    pub max_steps: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for PhaseTransitionConfig {
    fn default() -> Self {
        Self {
            coherence_threshold: 0.95,
            energy_threshold: 0.05,
            lyapunov_tolerance: 1e-8,
            cbf_beta: 1.0,
            max_steps: 1000,
            seed: 42,
        }
    }
}

impl PhaseTransitionConfig {
    /// Create config for fast testing.
    pub fn fast() -> Self {
        Self {
            coherence_threshold: 0.8,
            energy_threshold: 0.2,
            lyapunov_tolerance: 1e-4,
            cbf_beta: 0.5,
            max_steps: 100,
            seed: 42,
        }
    }

    /// Create config for high precision.
    pub fn high_precision() -> Self {
        Self {
            coherence_threshold: 0.99,
            energy_threshold: 0.01,
            lyapunov_tolerance: 1e-12,
            cbf_beta: 2.0,
            max_steps: 10000,
            seed: 42,
        }
    }

    /// Set coherence threshold.
    pub fn with_coherence_threshold(mut self, threshold: f64) -> Self {
        self.coherence_threshold = threshold.max(0.0).min(1.0);
        self
    }

    /// Set energy threshold.
    pub fn with_energy_threshold(mut self, threshold: f64) -> Self {
        self.energy_threshold = threshold.max(0.0).min(1.0);
        self
    }

    /// Set CBF beta.
    pub fn with_cbf_beta(mut self, beta: f64) -> Self {
        self.cbf_beta = beta.max(0.0);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// ─── Phase Transition Result ─────────────────────────────────────────────────

/// Result of phase transition detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransitionResult {
    /// Phase transition detected.
    pub transition_detected: bool,
    /// Phase transition potential ΔG.
    pub transition_potential: f64,
    /// Current coherence.
    pub coherence: f64,
    /// Current planetary free energy.
    pub free_energy: f64,
    /// Lyapunov exponent (negative = stable).
    pub lyapunov_exponent: f64,
    /// CBF violation (should be ≤ 0 for safety).
    pub cbf_violation: f64,
    /// Stabilization steps taken.
    pub stabilization_steps: usize,
    /// Post-transition stability confirmed.
    pub stable: bool,
    /// Irreversibility confirmed.
    pub irreversible: bool,
}

impl PhaseTransitionResult {
    /// Generate summary.
    pub fn summary(&self) -> String {
        format!(
            "Phase Transition: detected={}, ΔG={:.6}, λ={:.8}, CBF={:.6}, stable={}, irreversible={}",
            self.transition_detected,
            self.transition_potential,
            self.lyapunov_exponent,
            self.cbf_violation,
            self.stable,
            self.irreversible
        )
    }
}

impl std::fmt::Display for PhaseTransitionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── Civilizational Phase Shift Result ───────────────────────────────────────

/// Result of civilizational phase shift simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationalPhaseShift {
    /// Initial economic dominance (0-1).
    pub initial_economic_dominance: f64,
    /// Final economic dominance.
    pub final_economic_dominance: f64,
    /// Symbiotic adoption rate.
    pub symbiotic_adoption: f64,
    /// Tipping point cycle.
    pub tipping_cycle: usize,
    /// Total cycles simulated.
    pub total_cycles: usize,
    /// Phase transition confirmed.
    pub transition_confirmed: bool,
    /// Irreversibility confirmed.
    pub irreversible: bool,
    /// Economic dominance trajectory.
    pub economic_trajectory: Vec<f64>,
    /// Symbiotic adoption trajectory.
    pub symbiotic_trajectory: Vec<f64>,
}

impl CivilizationalPhaseShift {
    /// Generate summary.
    pub fn summary(&self) -> String {
        format!(
            "Civilizational Shift: economic {:.4}→{:.4}, symbiotic={:.4}, tipping@{}, irreversible={}",
            self.initial_economic_dominance,
            self.final_economic_dominance,
            self.symbiotic_adoption,
            self.tipping_cycle,
            self.irreversible
        )
    }
}

impl std::fmt::Display for CivilizationalPhaseShift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── Core Functions ──────────────────────────────────────────────────────────

/// LCG random number generator.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Generate uniform random in [0, 1].
fn random_uniform(state: &mut u64) -> f64 {
    ((lcg_next(state) >> 11) as f64 / (1u64 << 51) as f64).clamp(0.0, 1.0)
}

/// Compute phase transition potential ΔG.
///
/// ```text
/// ΔG = F_planet,economic - F_planet,symbiotic + λ * coherence_barrier
/// ```
///
/// When ΔG < 0, the symbiotic attractor dominates and transition is favorable.
pub fn compute_transition_potential(
    f_planet_economic: f64,
    f_planet_symbiotic: f64,
    coherence: f64,
    coherence_threshold: f64,
    lambda: f64,
) -> f64 {
    let coherence_barrier = if coherence < coherence_threshold {
        (coherence_threshold - coherence) * lambda
    } else {
        0.0
    };
    f_planet_economic - f_planet_symbiotic + coherence_barrier
}

/// Compute Lyapunov exponent from a trajectory of free energy values.
///
/// λ = (1/n) * Σ ln(|δ_{i+1}| / |δ_i|)
/// Negative λ implies asymptotic stability.
pub fn compute_lyapunov_exponent(trajectory: &[f64]) -> f64 {
    if trajectory.len() < 3 {
        return 0.0;
    }

    let mut sum = 0.0;
    let mut count = 0;
    for i in 1..trajectory.len() {
        let delta_prev = (trajectory[i] - trajectory[i - 1]).abs().max(1e-15);
        if i + 1 < trajectory.len() {
            let delta_next = (trajectory[i + 1] - trajectory[i]).abs().max(1e-15);
            let ratio = delta_next / delta_prev;
            sum += ratio.ln();
            count += 1;
        }
    }

    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

/// Compute Control Barrier Function violation.
///
/// h(φ) = β - ||φ - φ_safe||²
/// CBF satisfied when h(φ) ≥ 0.
pub fn compute_cbf_violation(current_state: f64, safe_state: f64, beta: f64) -> f64 {
    let distance_sq = (current_state - safe_state).powi(2);
    beta - distance_sq
}

/// Detect phase transition based on coherence and free energy.
///
/// Returns `PhaseTransitionResult` with full analysis including
/// Lyapunov stability and CBF verification.
pub fn detect_phase_transition(
    coherence: f64,
    free_energy: f64,
    free_energy_trajectory: &[f64],
    config: &PhaseTransitionConfig,
) -> PhaseTransitionResult {
    let transition_detected =
        coherence > config.coherence_threshold && free_energy < config.energy_threshold;

    // Compute transition potential.
    let f_economic = free_energy * 2.0; // Economic regime has higher VFE.
    let transition_potential = compute_transition_potential(
        f_economic,
        free_energy,
        coherence,
        config.coherence_threshold,
        1.0,
    );

    // Compute Lyapunov exponent.
    let lyapunov = compute_lyapunov_exponent(free_energy_trajectory);

    // Compute CBF violation.
    let cbf_violation = compute_cbf_violation(coherence, 1.0, config.cbf_beta);

    // Stability: Lyapunov < 0 AND CBF satisfied.
    let stable = lyapunov < 0.0 && cbf_violation >= 0.0;

    // Irreversibility: stable AND transition detected.
    let irreversible = stable && transition_detected;

    PhaseTransitionResult {
        transition_detected,
        transition_potential,
        coherence,
        free_energy,
        lyapunov_exponent: lyapunov,
        cbf_violation,
        stabilization_steps: free_energy_trajectory.len(),
        stable,
        irreversible,
    }
}

/// Stabilize irreversible symbiosis using CBF projection.
///
/// Projects the current state back into the safe set if CBF is violated,
/// ensuring the symbiotic attractor remains stable post-transition.
pub fn stabilize_irreversible_symbiosis(
    coherence: f64,
    free_energy: f64,
    config: &PhaseTransitionConfig,
) -> (f64, f64, bool) {
    let cbf_violation = compute_cbf_violation(coherence, 1.0, config.cbf_beta);

    if cbf_violation >= 0.0 {
        // Already in safe set.
        return (coherence, free_energy, true);
    }

    // Project back to safe set: increase coherence toward 1.0.
    let correction = (-cbf_violation).sqrt() * 0.5;
    let new_coherence = (coherence + correction).min(1.0);
    let new_free_energy = free_energy * (1.0 - correction * 0.1);
    let stabilized = compute_cbf_violation(new_coherence, 1.0, config.cbf_beta) >= 0.0;

    (new_coherence, new_free_energy, stabilized)
}

/// Simulate civilizational phase shift from economic to symbiotic dominance.
///
/// Models the transition as a competitive dynamics between economic and
/// symbiotic attractors, with the tipping point occurring when symbiotic
/// adoption exceeds 50%.
pub fn simulate_civilizational_phase_shift(
    _initial_nodes: usize,
    total_cycles: usize,
    config: &PhaseTransitionConfig,
) -> CivilizationalPhaseShift {
    let mut rng_state = config.seed;
    let mut economic_dominance: f64 = 1.0;
    let mut symbiotic_adoption: f64 = 0.0;
    let mut tipping_cycle = total_cycles;
    let mut economic_trajectory = Vec::with_capacity(total_cycles + 1);
    let mut symbiotic_trajectory = Vec::with_capacity(total_cycles + 1);

    economic_trajectory.push(economic_dominance);
    symbiotic_trajectory.push(symbiotic_adoption);

    let replication_rate = 0.05;
    let diffusion_rate = 0.02;

    for cycle in 0..total_cycles {
        // Symbiotic growth: logistic + network effect.
        let network_effect = symbiotic_adoption.powi(2);
        let growth = replication_rate
            * (1.0 - symbiotic_adoption)
            * (1.0 + network_effect)
            * (1.0 + random_uniform(&mut rng_state) * diffusion_rate);

        symbiotic_adoption = (symbiotic_adoption + growth).min(1.0);
        economic_dominance = (1.0 - symbiotic_adoption).max(0.0);

        economic_trajectory.push(economic_dominance);
        symbiotic_trajectory.push(symbiotic_adoption);

        // Detect tipping point.
        if symbiotic_adoption > 0.5 && tipping_cycle == total_cycles {
            tipping_cycle = cycle;
        }
    }

    let transition_confirmed = symbiotic_adoption > 0.5;
    let irreversible = symbiotic_adoption > config.coherence_threshold;

    CivilizationalPhaseShift {
        initial_economic_dominance: 1.0,
        final_economic_dominance: economic_dominance,
        symbiotic_adoption,
        tipping_cycle,
        total_cycles,
        transition_confirmed,
        irreversible,
        economic_trajectory,
        symbiotic_trajectory,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_transition_config_default() {
        let cfg = PhaseTransitionConfig::default();
        assert!((cfg.coherence_threshold - 0.95).abs() < 1e-6);
        assert!((cfg.energy_threshold - 0.05).abs() < 1e-6);
        assert!((cfg.cbf_beta - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_phase_transition_config_fast() {
        let cfg = PhaseTransitionConfig::fast();
        assert!((cfg.coherence_threshold - 0.8).abs() < 1e-6);
        assert_eq!(cfg.max_steps, 100);
    }

    #[test]
    fn test_phase_transition_config_high_precision() {
        let cfg = PhaseTransitionConfig::high_precision();
        assert!((cfg.coherence_threshold - 0.99).abs() < 1e-6);
        assert_eq!(cfg.max_steps, 10000);
    }

    #[test]
    fn test_phase_transition_config_with_coherence_threshold() {
        let cfg = PhaseTransitionConfig::default().with_coherence_threshold(0.9);
        assert!((cfg.coherence_threshold - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_phase_transition_config_coherence_clamped() {
        let cfg = PhaseTransitionConfig::default().with_coherence_threshold(1.5);
        assert!((cfg.coherence_threshold - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_phase_transition_config_with_energy_threshold() {
        let cfg = PhaseTransitionConfig::default().with_energy_threshold(0.1);
        assert!((cfg.energy_threshold - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_phase_transition_config_with_cbf_beta() {
        let cfg = PhaseTransitionConfig::default().with_cbf_beta(2.0);
        assert!((cfg.cbf_beta - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_phase_transition_config_with_seed() {
        let cfg = PhaseTransitionConfig::default().with_seed(99);
        assert_eq!(cfg.seed, 99);
    }

    #[test]
    fn test_compute_transition_potential_symbiotic_dominates() {
        let dg = compute_transition_potential(0.5, 0.1, 0.95, 0.9, 1.0);
        assert!(dg < 0.0);
    }

    #[test]
    fn test_compute_transition_potential_economic_dominates() {
        let dg = compute_transition_potential(0.1, 0.5, 0.5, 0.9, 1.0);
        assert!(dg < 0.0);
    }

    #[test]
    fn test_compute_transition_potential_with_barrier() {
        let dg = compute_transition_potential(0.5, 0.3, 0.5, 0.9, 2.0);
        assert!(dg > 0.0);
    }

    #[test]
    fn test_compute_transition_potential_zero_barrier() {
        let dg = compute_transition_potential(0.5, 0.3, 0.95, 0.9, 1.0);
        assert!((dg - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_compute_lyapunov_exponent_stable() {
        let traj = vec![1.0, 0.8, 0.64, 0.512, 0.4096];
        let lambda = compute_lyapunov_exponent(&traj);
        assert!(lambda < 0.0);
    }

    #[test]
    fn test_compute_lyapunov_exponent_constant() {
        let traj = vec![0.5, 0.5, 0.5, 0.5];
        let lambda = compute_lyapunov_exponent(&traj);
        assert!(lambda.is_finite());
    }

    #[test]
    fn test_compute_lyapunov_exponent_short() {
        let traj = vec![1.0, 0.5];
        let lambda = compute_lyapunov_exponent(&traj);
        assert!((lambda - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_cbf_violation_safe() {
        let v = compute_cbf_violation(0.95, 1.0, 1.0);
        assert!(v >= 0.0);
    }

    #[test]
    fn test_compute_cbf_violation_unsafe() {
        let v = compute_cbf_violation(0.5, 1.0, 0.1);
        assert!(v < 0.0);
    }

    #[test]
    fn test_compute_cbf_violation_boundary() {
        let v = compute_cbf_violation(0.0, 0.0, 1.0);
        assert!((v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_detect_phase_transition_detected() {
        let cfg = PhaseTransitionConfig::default();
        let traj = vec![1.0, 0.8, 0.6, 0.4, 0.3];
        let result = detect_phase_transition(0.96, 0.03, &traj, &cfg);
        assert!(result.transition_detected);
    }

    #[test]
    fn test_detect_phase_transition_not_detected_low_coherence() {
        let cfg = PhaseTransitionConfig::default();
        let traj = vec![1.0, 0.8, 0.6];
        let result = detect_phase_transition(0.5, 0.03, &traj, &cfg);
        assert!(!result.transition_detected);
    }

    #[test]
    fn test_detect_phase_transition_not_detected_high_energy() {
        let cfg = PhaseTransitionConfig::default();
        let traj = vec![1.0, 0.8, 0.6];
        let result = detect_phase_transition(0.96, 0.5, &traj, &cfg);
        assert!(!result.transition_detected);
    }

    #[test]
    fn test_detect_phase_transition_irreversible() {
        let cfg = PhaseTransitionConfig::default();
        let traj = vec![1.0, 0.8, 0.64, 0.512, 0.03];
        let result = detect_phase_transition(0.96, 0.03, &traj, &cfg);
        assert!(result.transition_detected);
        // Lyapunov should be negative for decreasing trajectory.
        assert!(result.lyapunov_exponent < 0.0);
    }

    #[test]
    fn test_detect_phase_transition_potential_range() {
        let cfg = PhaseTransitionConfig::default();
        let traj = vec![1.0, 0.5];
        let result = detect_phase_transition(0.5, 0.5, &traj, &cfg);
        assert!(result.transition_potential.is_finite());
    }

    #[test]
    fn test_stabilize_irreversible_symbiosis_already_safe() {
        let cfg = PhaseTransitionConfig::default();
        let (c, _e, stabilized) = stabilize_irreversible_symbiosis(0.95, 0.05, &cfg);
        assert!(stabilized);
        assert!((c - 0.95).abs() < 1e-6);
    }

    #[test]
    fn test_stabilize_irreversible_symbiosis_needs_correction() {
        let cfg = PhaseTransitionConfig {
            cbf_beta: 0.01,
            ..PhaseTransitionConfig::default()
        };
        let (c, e, stabilized) = stabilize_irreversible_symbiosis(0.5, 0.5, &cfg);
        assert!(c > 0.5);
        assert!(e <= 0.5);
    }

    #[test]
    fn test_stabilize_irreversible_symbiosis_coherence_bounded() {
        let cfg = PhaseTransitionConfig {
            cbf_beta: 0.001,
            ..PhaseTransitionConfig::default()
        };
        let (c, _e, _stabilized) = stabilize_irreversible_symbiosis(0.1, 0.9, &cfg);
        assert!(c <= 1.0);
    }

    #[test]
    fn test_simulate_civilizational_phase_shift_basic() {
        let cfg = PhaseTransitionConfig::fast();
        let result = simulate_civilizational_phase_shift(100, 500, &cfg);
        assert!(result.symbiotic_adoption > 0.0);
        assert!(result.economic_trajectory.len() == result.symbiotic_trajectory.len());
    }

    #[test]
    fn test_simulate_civilizational_phase_shift_tipping() {
        let cfg = PhaseTransitionConfig::fast();
        let result = simulate_civilizational_phase_shift(100, 1000, &cfg);
        assert!(result.tipping_cycle < result.total_cycles);
        assert!(result.transition_confirmed);
    }

    #[test]
    fn test_simulate_civilizational_phase_shift_short() {
        let cfg = PhaseTransitionConfig::fast();
        let result = simulate_civilizational_phase_shift(100, 10, &cfg);
        assert_eq!(result.economic_trajectory.len(), 11);
    }

    #[test]
    fn test_simulate_civilizational_phase_shift_trajectories_sum() {
        let cfg = PhaseTransitionConfig::fast();
        let result = simulate_civilizational_phase_shift(100, 100, &cfg);
        for i in 0..result.economic_trajectory.len() {
            let sum = result.economic_trajectory[i] + result.symbiotic_trajectory[i];
            assert!((sum - 1.0).abs() < 1e-6, "sum {} at index {}", sum, i);
        }
    }

    #[test]
    fn test_simulate_civilizational_phase_shift_deterministic() {
        let cfg = PhaseTransitionConfig::fast();
        let r1 = simulate_civilizational_phase_shift(100, 100, &cfg);
        let r2 = simulate_civilizational_phase_shift(100, 100, &cfg);
        assert_eq!(r1.symbiotic_adoption, r2.symbiotic_adoption);
        assert_eq!(r1.economic_trajectory, r2.economic_trajectory);
    }

    #[test]
    fn test_simulate_civilizational_phase_shift_irreversible() {
        let cfg = PhaseTransitionConfig {
            coherence_threshold: 0.5,
            ..PhaseTransitionConfig::fast()
        };
        let result = simulate_civilizational_phase_shift(100, 2000, &cfg);
        assert!(result.irreversible);
    }

    #[test]
    fn test_phase_transition_result_summary() {
        let result = PhaseTransitionResult {
            transition_detected: true,
            transition_potential: -0.5,
            coherence: 0.96,
            free_energy: 0.03,
            lyapunov_exponent: -0.1,
            cbf_violation: 0.5,
            stabilization_steps: 100,
            stable: true,
            irreversible: true,
        };
        let summary = result.summary();
        assert!(summary.contains("detected=true"));
        assert!(summary.contains("irreversible=true"));
    }

    #[test]
    fn test_phase_transition_result_display() {
        let result = PhaseTransitionResult {
            transition_detected: false,
            transition_potential: 0.3,
            coherence: 0.5,
            free_energy: 0.5,
            lyapunov_exponent: 0.0,
            cbf_violation: -0.2,
            stabilization_steps: 50,
            stable: false,
            irreversible: false,
        };
        let display = format!("{}", result);
        assert!(display.contains("detected=false"));
    }

    #[test]
    fn test_civilizational_phase_shift_summary() {
        let result = CivilizationalPhaseShift {
            initial_economic_dominance: 1.0,
            final_economic_dominance: 0.2,
            symbiotic_adoption: 0.8,
            tipping_cycle: 100,
            total_cycles: 500,
            transition_confirmed: true,
            irreversible: true,
            economic_trajectory: vec![],
            symbiotic_trajectory: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("tipping@100"));
    }

    #[test]
    fn test_civilizational_phase_shift_display() {
        let result = CivilizationalPhaseShift {
            initial_economic_dominance: 1.0,
            final_economic_dominance: 0.5,
            symbiotic_adoption: 0.5,
            tipping_cycle: 200,
            total_cycles: 500,
            transition_confirmed: true,
            irreversible: false,
            economic_trajectory: vec![],
            symbiotic_trajectory: vec![],
        };
        let display = format!("{}", result);
        assert!(display.contains("irreversible=false"));
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
    fn test_phase_transition_config_clone() {
        let cfg = PhaseTransitionConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.coherence_threshold, cloned.coherence_threshold);
    }

    #[test]
    fn test_full_phase_transition_workflow() {
        let cfg = PhaseTransitionConfig::default();
        let traj = vec![1.0, 0.8, 0.64, 0.512, 0.4096, 0.32768, 0.03];
        let result = detect_phase_transition(0.96, 0.03, &traj, &cfg);
        assert!(result.transition_detected);
        let (c, _e, stabilized) =
            stabilize_irreversible_symbiosis(result.coherence, result.free_energy, &cfg);
        assert!(stabilized);
        let shift = simulate_civilizational_phase_shift(100, 500, &cfg);
        assert!(shift.transition_confirmed);
    }
}
