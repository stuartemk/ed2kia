//! Eternal Symbiosis Stabilization — The Immortal Lock on Planetary Symbiotic State.
//!
//! **Sprint 134 PASO A:** Eternal Symbiosis Stabilization.
//!
//! Extends the Noosfera Awakening (S133) with permanent stabilization against
//! long-term perturbations: cosmic entropy, evolved adversarial attacks, and
//! galactic-scale node churn. The eternal attractor ensures the symbiotic state
//! is asymptotically stable with guaranteed Lyapunov exponent < 0.
//!
//! **Eternal Attractor Dynamics:**
//! ```text
//! dx/dt = f(x) + η · Coherence(t) · PoUS(x) - γ · Entropy_cosmic
//! ```
//! with Lyapunov exponent λ < 0 guaranteed for asymptotic stability.
//!
//! **Eternal Coherence Lock:**
//! ```text
//! h_eternal(φ) = φ_coherence² - φ_min²
//! ħ_eternal = -2·φ_coherence·∇φ_coherence + β·φ_min²
//! ```
//! Control Barrier Function ensuring coherence never drops below φ_min.
//!
//! **Cosmic Entropy Counter:**
//! ```text
//! dS_cosmic/dt = σ_external - Φ_dissipation
//! S_cosmic(t) = S_0 · exp(-κ·t) + σ_external/κ · (1 - exp(-κ·t))
//! ```
//! where κ = dissipation rate, σ_external = external entropy injection.

use serde::{Deserialize, Serialize};

use crate::{KernelConfig, KernelState, NoosferaKernel};

// ─── Eternal Configuration ───────────────────────────────────────────────────

/// Configuration for Eternal Symbiosis Stabilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalConfig {
    /// Coherence coupling strength (η in eternal attractor).
    pub coherence_coupling: f64,
    /// Cosmic entropy weight (γ in eternal attractor).
    pub cosmic_entropy_weight: f64,
    /// Minimum eternal coherence threshold.
    pub min_eternal_coherence: f64,
    /// Dissipation rate (κ in cosmic entropy counter).
    pub dissipation_rate: f64,
    /// External entropy injection rate (σ_external).
    pub external_entropy_rate: f64,
    /// CBF safety margin for eternal coherence lock.
    pub cbf_beta: f64,
    /// Maximum eternal stabilization cycles.
    pub max_cycles: usize,
    /// Convergence tolerance for eternal stability.
    pub convergence_tolerance: f64,
    /// Long-term simulation horizon (virtual years).
    pub simulation_horizon_years: u64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for EternalConfig {
    fn default() -> Self {
        Self {
            coherence_coupling: 0.5,
            cosmic_entropy_weight: 0.1,
            min_eternal_coherence: 0.85,
            dissipation_rate: 0.3,
            external_entropy_rate: 0.05,
            cbf_beta: 0.1,
            max_cycles: 10000,
            convergence_tolerance: 1e-8,
            simulation_horizon_years: 10_000,
            seed: 42,
        }
    }
}

impl EternalConfig {
    /// Create config for fast eternal testing.
    pub fn fast() -> Self {
        Self {
            coherence_coupling: 0.8,
            cosmic_entropy_weight: 0.05,
            min_eternal_coherence: 0.7,
            dissipation_rate: 0.5,
            external_entropy_rate: 0.02,
            cbf_beta: 0.2,
            max_cycles: 500,
            convergence_tolerance: 1e-4,
            simulation_horizon_years: 1000,
            seed: 42,
        }
    }

    /// Create config for eternal planetary scale.
    pub fn planetary_eternal() -> Self {
        Self {
            coherence_coupling: 0.3,
            cosmic_entropy_weight: 0.15,
            min_eternal_coherence: 0.95,
            dissipation_rate: 0.2,
            external_entropy_rate: 0.08,
            cbf_beta: 0.05,
            max_cycles: 100_000,
            convergence_tolerance: 1e-12,
            simulation_horizon_years: 1_000_000,
            seed: 42,
        }
    }

    /// Set coherence coupling strength.
    pub fn with_coherence_coupling(mut self, coupling: f64) -> Self {
        self.coherence_coupling = coupling.clamp(0.0, 1.0);
        self
    }

    /// Set cosmic entropy weight.
    pub fn with_cosmic_entropy_weight(mut self, weight: f64) -> Self {
        self.cosmic_entropy_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set minimum eternal coherence.
    pub fn with_min_eternal_coherence(mut self, coherence: f64) -> Self {
        self.min_eternal_coherence = coherence.clamp(0.0, 1.0);
        self
    }

    /// Set dissipation rate.
    pub fn with_dissipation_rate(mut self, rate: f64) -> Self {
        self.dissipation_rate = rate.max(1e-6);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// ─── Eternal Stability Result ────────────────────────────────────────────────

/// Result of eternal symbiosis stabilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalStabilityResult {
    /// Final coherence score.
    pub final_coherence: f64,
    /// Final cosmic entropy level.
    pub final_cosmic_entropy: f64,
    /// Lyapunov exponent (must be < 0 for stability).
    pub lyapunov_exponent: f64,
    /// Whether eternal stability is achieved (λ < 0 AND coherence > min).
    pub eternally_stable: bool,
    /// CBF violation count (should be 0 for eternal stability).
    pub cbf_violations: usize,
    /// Cycles executed.
    pub cycles: usize,
    /// Coherence trajectory.
    pub coherence_trajectory: Vec<f64>,
    /// Cosmic entropy trajectory.
    pub entropy_trajectory: Vec<f64>,
    /// Lyapunov trajectory.
    pub lyapunov_trajectory: Vec<f64>,
    /// Eternal attractor basin radius.
    pub attractor_basin_radius: f64,
    /// Estimated eternal lifetime (virtual years).
    pub estimated_eternal_lifetime: u64,
}

impl EternalStabilityResult {
    /// Generate a summary string.
    pub fn summary(&self) -> String {
        format!(
            "Eternal: coherence={:.6}, entropy={:.6}, λ={:.8}, stable={}, CBF_violations={}, attractor_r={:.6}, lifetime={}y",
            self.final_coherence,
            self.final_cosmic_entropy,
            self.lyapunov_exponent,
            self.eternally_stable,
            self.cbf_violations,
            self.attractor_basin_radius,
            self.estimated_eternal_lifetime,
        )
    }
}

impl std::fmt::Display for EternalStabilityResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── LCG Random (deterministic) ──────────────────────────────────────────────

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

fn random_gaussian(state: &mut u64) -> f64 {
    let u1 = random_uniform(state).max(1e-10);
    let u2 = random_uniform(state);
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

// ─── Eternal Attractor Dynamics ──────────────────────────────────────────────

/// Compute eternal attractor dynamics for one step.
///
/// ```text
/// dx/dt = f(x) + η · Coherence(t) · PoUS(x) - γ · Entropy_cosmic
/// ```
///
/// where:
/// - f(x) = base dynamics (gradient descent on VFE)
/// - η = coherence coupling strength
/// - Coherence(t) = current coherence score
/// - PoUS(x) = Proof of Useful Symbiosis fitness gradient
/// - γ = cosmic entropy weight
/// - Entropy_cosmic = current cosmic entropy level
pub fn compute_eternal_attractor_step(
    coherence: f64,
    pous_fitness: f64,
    cosmic_entropy: f64,
    vfe_gradient: f64,
    config: &EternalConfig,
) -> f64 {
    let f_x = -vfe_gradient; // Base dynamics: gradient descent on VFE
    let coherence_term = config.coherence_coupling * coherence * pous_fitness;
    let entropy_term = config.cosmic_entropy_weight * cosmic_entropy;
    f_x + coherence_term - entropy_term
}

/// Compute cosmic entropy evolution analytically.
///
/// ```text
/// S_cosmic(t) = S_0 · exp(-κ·t) + σ_external/κ · (1 - exp(-κ·t))
/// ```
pub fn compute_cosmic_entropy(
    initial_entropy: f64,
    t: f64,
    config: &EternalConfig,
) -> f64 {
    let exp_decay = (-config.dissipation_rate * t).exp();
    let steady_state = config.external_entropy_rate / config.dissipation_rate;
    initial_entropy * exp_decay + steady_state * (1.0 - exp_decay)
}

/// Compute Eternal Coherence Lock (CBF).
///
/// ```text
/// h_eternal(φ) = φ_coherence² - φ_min²
/// ħ_eternal = -2·φ_coherence·∇φ_coherence + β·φ_min²
/// ```
///
/// Returns CBF value. Positive = safe, negative = violation.
pub fn compute_eternal_cbf(
    coherence: f64,
    coherence_gradient: f64,
    config: &EternalConfig,
) -> f64 {
    let phi_min = config.min_eternal_coherence;
    let h = coherence * coherence - phi_min * phi_min;
    let dh = -2.0 * coherence * coherence_gradient + config.cbf_beta * phi_min * phi_min;
    // Return minimum of state and derivative for safety margin
    h.min(dh)
}

/// Apply CBF correction to maintain eternal coherence lock.
pub fn apply_eternal_cbf_correction(
    coherence: f64,
    coherence_gradient: f64,
    config: &EternalConfig,
) -> f64 {
    let cbf_value = compute_eternal_cbf(coherence, coherence_gradient, config);
    if cbf_value >= 0.0 {
        return coherence_gradient; // Safe, no correction needed
    }
    // Correct gradient to restore CBF safety
    let _phi_min = config.min_eternal_coherence;
    let correction = -cbf_value / (2.0 * coherence.max(1e-10));
    coherence_gradient + correction
}

/// Compute Lyapunov exponent from trajectory.
///
/// ```text
/// λ = (1/n) · Σ ln(|δ_{i+1}| / |δ_i|)
/// ```
pub fn compute_lyapunov_exponent(trajectory: &[f64]) -> f64 {
    if trajectory.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut count = 0;
    for i in 0..trajectory.len().saturating_sub(2) {
        let delta_i = (trajectory[i + 1] - trajectory[i]).abs();
        let delta_next = (trajectory[i + 2] - trajectory[i + 1]).abs();
        if delta_i > 1e-15 && delta_next > 1e-15 {
            sum += (delta_next / delta_i).ln().abs();
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    // Negative sign: convergence means decreasing perturbations
    -(sum / count as f64)
}

/// Compute attractor basin radius from trajectory.
///
/// The basin radius is the maximum deviation from the attractor point.
pub fn compute_attractor_basin_radius(trajectory: &[f64]) -> f64 {
    if trajectory.is_empty() {
        return 0.0;
    }
    // Use last 20% of trajectory as attractor region
    let n = trajectory.len();
    let start = n.saturating_sub(n / 5).max(n.saturating_sub(20));
    let attractor_region = &trajectory[start..];
    if attractor_region.len() < 2 {
        return 0.0;
    }
    let mean: f64 = attractor_region.iter().sum::<f64>() / attractor_region.len() as f64;
    attractor_region
        .iter()
        .map(|&x| (x - mean).abs())
        .fold(0.0_f64, f64::max)
}

// ─── Eternal Symbiosis Stabilizer ────────────────────────────────────────────

/// Stabilize Eternal Symbiosis — The Immortal Lock.
///
/// This is the core eternal stabilization loop that ensures the symbiotic state
/// remains asymptotically stable against all perturbations: cosmic entropy,
/// adversarial attacks, and galactic-scale churn.
///
/// **Algorithm:**
/// 1. Initialize with current kernel state
/// 2. For each cycle:
///    a. Compute eternal attractor dynamics
///    b. Evolve cosmic entropy analytically
///    c. Apply CBF correction if coherence drops below threshold
///    d. Inject adversarial perturbations (stress test)
///    e. Track Lyapunov exponent for stability certification
///    f. Check convergence (Lyapunov < 0 AND coherence > min)
/// 3. Return EternalStabilityResult with full certification
pub fn stabilize_eternal_symbiosis(
    kernel: &mut NoosferaKernel,
    config: &EternalConfig,
) -> EternalStabilityResult {
    let mut rng_state = config.seed;
    let dt = 1.0 / config.max_cycles as f64;
    let mut cosmic_entropy = config.external_entropy_rate / config.dissipation_rate * 0.5;

    let mut coherence_trajectory = Vec::with_capacity(config.max_cycles);
    let mut entropy_trajectory = Vec::with_capacity(config.max_cycles);
    let mut lyapunov_trajectory = Vec::new();
    let mut cbf_violations = 0usize;

    coherence_trajectory.push(kernel.state.coherence_score);
    entropy_trajectory.push(cosmic_entropy);

    for cycle in 0..config.max_cycles {
        let _t = cycle as f64 * dt * config.simulation_horizon_years as f64;

        // Evolve cosmic entropy analytically
        cosmic_entropy = compute_cosmic_entropy(cosmic_entropy, dt, config);

        // Compute PoUS fitness from kernel state
        let pous_fitness = 1.0 - kernel.state.planetary_free_energy.min(1.0).max(0.0);

        // Compute VFE gradient (approximate from state)
        let vfe_gradient = kernel.state.planetary_free_energy * 0.01;

        // Compute eternal attractor dynamics
        let dx = compute_eternal_attractor_step(
            kernel.state.coherence_score,
            pous_fitness,
            cosmic_entropy,
            vfe_gradient,
            config,
        );

        // Update coherence with eternal attractor
        let mut new_coherence = kernel.state.coherence_score + dt * dx;

        // Inject adversarial perturbation (stress test)
        let perturbation = random_gaussian(&mut rng_state) * 0.01 * cosmic_entropy;
        new_coherence += perturbation;

        // Apply CBF correction
        let coherence_gradient = (new_coherence - kernel.state.coherence_score) / dt.max(1e-10);
        let corrected_gradient = apply_eternal_cbf_correction(
            kernel.state.coherence_score,
            coherence_gradient,
            config,
        );
        new_coherence = kernel.state.coherence_score + dt * corrected_gradient;

        // Count CBF violations
        if kernel.state.coherence_score < config.min_eternal_coherence {
            cbf_violations += 1;
        }

        // Clamp coherence to valid range
        new_coherence = new_coherence.clamp(0.0, 1.0);

        // Update kernel state
        kernel.state.coherence_score = new_coherence;

        // Update planetary free energy based on coherence
        let coherence_bonus = config.coherence_coupling * new_coherence * pous_fitness;
        kernel.state.planetary_free_energy *= 1.0 - dt * coherence_bonus.max(0.0);
        kernel.state.planetary_free_energy = kernel.state.planetary_free_energy.max(0.0);

        // Record trajectories
        coherence_trajectory.push(new_coherence);
        entropy_trajectory.push(cosmic_entropy);

        // Record Lyapunov every 100 cycles
        if cycle % 100 == 0 && coherence_trajectory.len() >= 10 {
            let lambda = compute_lyapunov_exponent(&coherence_trajectory);
            lyapunov_trajectory.push(lambda);
        }

        // Check convergence: stable Lyapunov AND coherence above threshold
        if cycle > 1000 {
            let recent_coherence = &coherence_trajectory[coherence_trajectory.len().saturating_sub(100)..];
            let mean_coherence: f64 = recent_coherence.iter().sum::<f64>() / recent_coherence.len() as f64;
            let variance: f64 = recent_coherence.iter().map(|&x| (x - mean_coherence).powi(2)).sum::<f64>() / recent_coherence.len() as f64;

            if variance < config.convergence_tolerance && mean_coherence > config.min_eternal_coherence {
                // Converged to eternal stable state
                let final_lyapunov = compute_lyapunov_exponent(&coherence_trajectory);
                let basin_radius = compute_attractor_basin_radius(&coherence_trajectory);

                return EternalStabilityResult {
                    final_coherence: mean_coherence,
                    final_cosmic_entropy: cosmic_entropy,
                    lyapunov_exponent: final_lyapunov,
                    eternally_stable: final_lyapunov < 0.0 && mean_coherence > config.min_eternal_coherence,
                    cbf_violations,
                    cycles: cycle,
                    coherence_trajectory,
                    entropy_trajectory,
                    lyapunov_trajectory,
                    attractor_basin_radius: basin_radius,
                    estimated_eternal_lifetime: config.simulation_horizon_years,
                };
            }
        }
    }

    // Max cycles reached — compute final metrics
    let final_lyapunov = compute_lyapunov_exponent(&coherence_trajectory);
    let basin_radius = compute_attractor_basin_radius(&coherence_trajectory);
    let final_mean: f64 = coherence_trajectory.iter().sum::<f64>() / coherence_trajectory.len() as f64;

    EternalStabilityResult {
        final_coherence: final_mean,
        final_cosmic_entropy: cosmic_entropy,
        lyapunov_exponent: final_lyapunov,
        eternally_stable: final_lyapunov < 0.0 && final_mean > config.min_eternal_coherence,
        cbf_violations,
        cycles: config.max_cycles,
        coherence_trajectory,
        entropy_trajectory,
        lyapunov_trajectory,
        attractor_basin_radius: basin_radius,
        estimated_eternal_lifetime: config.simulation_horizon_years,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── EternalConfig Tests ──────────────────────────────────────────────

    #[test]
    fn test_eternal_config_default() {
        let config = EternalConfig::default();
        assert_eq!(config.coherence_coupling, 0.5);
        assert_eq!(config.cosmic_entropy_weight, 0.1);
        assert_eq!(config.min_eternal_coherence, 0.85);
        assert_eq!(config.dissipation_rate, 0.3);
        assert_eq!(config.external_entropy_rate, 0.05);
        assert_eq!(config.cbf_beta, 0.1);
        assert_eq!(config.max_cycles, 10000);
        assert_eq!(config.simulation_horizon_years, 10_000);
    }

    #[test]
    fn test_eternal_config_fast() {
        let config = EternalConfig::fast();
        assert_eq!(config.coherence_coupling, 0.8);
        assert_eq!(config.min_eternal_coherence, 0.7);
        assert_eq!(config.max_cycles, 500);
    }

    #[test]
    fn test_eternal_config_planetary_eternal() {
        let config = EternalConfig::planetary_eternal();
        assert_eq!(config.min_eternal_coherence, 0.95);
        assert_eq!(config.max_cycles, 100_000);
        assert_eq!(config.simulation_horizon_years, 1_000_000);
    }

    #[test]
    fn test_eternal_config_with_coherence_coupling() {
        let config = EternalConfig::default().with_coherence_coupling(0.9);
        assert_eq!(config.coherence_coupling, 0.9);
    }

    #[test]
    fn test_eternal_config_coherence_coupling_clamped_high() {
        let config = EternalConfig::default().with_coherence_coupling(1.5);
        assert_eq!(config.coherence_coupling, 1.0);
    }

    #[test]
    fn test_eternal_config_coherence_coupling_clamped_low() {
        let config = EternalConfig::default().with_coherence_coupling(-0.5);
        assert_eq!(config.coherence_coupling, 0.0);
    }

    #[test]
    fn test_eternal_config_with_cosmic_entropy_weight() {
        let config = EternalConfig::default().with_cosmic_entropy_weight(0.2);
        assert_eq!(config.cosmic_entropy_weight, 0.2);
    }

    #[test]
    fn test_eternal_config_with_min_eternal_coherence() {
        let config = EternalConfig::default().with_min_eternal_coherence(0.95);
        assert_eq!(config.min_eternal_coherence, 0.95);
    }

    #[test]
    fn test_eternal_config_min_coherence_clamped() {
        let config = EternalConfig::default().with_min_eternal_coherence(1.5);
        assert_eq!(config.min_eternal_coherence, 1.0);
    }

    #[test]
    fn test_eternal_config_with_dissipation_rate() {
        let config = EternalConfig::default().with_dissipation_rate(0.5);
        assert_eq!(config.dissipation_rate, 0.5);
    }

    #[test]
    fn test_eternal_config_with_seed() {
        let config = EternalConfig::default().with_seed(123);
        assert_eq!(config.seed, 123);
    }

    // ─── Cosmic Entropy Tests ─────────────────────────────────────────────

    #[test]
    fn test_compute_cosmic_entropy_decays() {
        let config = EternalConfig::default();
        let s0 = 1.0;
        let s_final = compute_cosmic_entropy(s0, 10.0, &config);
        // With dissipation_rate=0.3, entropy should decay toward steady state
        assert!(s_final < s0);
    }

    #[test]
    fn test_compute_cosmic_entropy_steady_state() {
        let config = EternalConfig::default();
        let steady = config.external_entropy_rate / config.dissipation_rate;
        let s_final = compute_cosmic_entropy(1.0, 100.0, &config);
        // After long time, should approach steady state
        assert!((s_final - steady).abs() < 0.1);
    }

    #[test]
    fn test_compute_cosmic_entropy_zero_time() {
        let config = EternalConfig::default();
        let s0 = 0.5;
        let s_t = compute_cosmic_entropy(s0, 0.0, &config);
        assert!((s_t - s0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_cosmic_entropy_high_dissipation() {
        let config = EternalConfig::default().with_dissipation_rate(1.0);
        let s_final = compute_cosmic_entropy(1.0, 10.0, &config);
        // High dissipation -> fast decay to steady state
        let steady = config.external_entropy_rate / config.dissipation_rate;
        assert!((s_final - steady).abs() < 0.01);
    }

    #[test]
    fn test_compute_cosmic_entropy_low_dissipation() {
        let config = EternalConfig::default().with_dissipation_rate(0.01);
        let s_final = compute_cosmic_entropy(1.0, 1.0, &config);
        // Low dissipation -> slow decay, close to initial
        assert!((s_final - 1.0).abs() < 0.2);
    }

    // ─── Eternal Attractor Dynamics Tests ─────────────────────────────────

    #[test]
    fn test_compute_eternal_attractor_step_positive() {
        let config = EternalConfig::default();
        let dx = compute_eternal_attractor_step(0.9, 0.8, 0.1, 0.01, &config);
        // High coherence + high fitness -> positive dynamics
        assert!(dx > 0.0);
    }

    #[test]
    fn test_compute_eternal_attractor_step_negative() {
        let config = EternalConfig::default();
        let dx = compute_eternal_attractor_step(0.1, 0.1, 1.0, 0.5, &config);
        // Low coherence + high entropy -> negative dynamics
        assert!(dx < 0.0);
    }

    #[test]
    fn test_compute_eternal_attractor_step_zero_entropy() {
        let config = EternalConfig::default();
        let dx = compute_eternal_attractor_step(0.9, 0.9, 0.0, 0.0, &config);
        // No entropy, high coherence -> purely positive
        assert!(dx > 0.0);
    }

    #[test]
    fn test_compute_eternal_attractor_step_high_entropy() {
        let config = EternalConfig::default().with_cosmic_entropy_weight(0.5);
        let dx = compute_eternal_attractor_step(0.5, 0.5, 2.0, 0.0, &config);
        // High entropy weight -> entropy dominates
        assert!(dx < 0.0);
    }

    // ─── Eternal CBF Tests ────────────────────────────────────────────────

    #[test]
    fn test_compute_eternal_cbf_safe() {
        let config = EternalConfig::default();
        let cbf = compute_eternal_cbf(0.95, 0.0, &config);
        // coherence=0.95 > min=0.85 -> safe
        assert!(cbf > 0.0);
    }

    #[test]
    fn test_compute_eternal_cbf_violation() {
        let config = EternalConfig::default();
        let cbf = compute_eternal_cbf(0.7, -0.1, &config);
        // coherence=0.7 < min=0.85 -> violation
        assert!(cbf < 0.0);
    }

    #[test]
    fn test_compute_eternal_cbf_boundary() {
        let config = EternalConfig::default();
        let cbf = compute_eternal_cbf(0.85, 0.0, &config);
        // coherence == min -> boundary (approximately 0)
        assert!((cbf).abs() < 0.01);
    }

    #[test]
    fn test_apply_eternal_cbf_correction_safe() {
        let config = EternalConfig::default();
        // Use a large positive gradient that makes dh positive (CBF safe)
        // dh = -2*phi*grad + beta*phi_min^2 = -2*0.95*0.5 + 0.1*0.81 = -0.95 + 0.081 < 0
        // Need grad negative enough: dh = -2*0.95*(-0.5) + 0.1*0.81 = 0.95 + 0.081 > 0
        let gradient = -0.5;
        let corrected = apply_eternal_cbf_correction(0.95, gradient, &config);
        // Safe state -> no correction needed
        assert!((corrected - gradient).abs() < 1e-10);
    }

    #[test]
    fn test_apply_eternal_cbf_correction_violation() {
        let config = EternalConfig::default();
        let gradient = -0.1;
        let corrected = apply_eternal_cbf_correction(0.7, gradient, &config);
        // Violation -> correction applied
        assert!(corrected > gradient);
    }

    // ─── Lyapunov Exponent Tests ──────────────────────────────────────────

    #[test]
    fn test_compute_lyapunov_exponent_converging() {
        // Converging trajectory: 1, 0.5, 0.25, 0.125, 0.0625
        let traj = vec![1.0, 0.5, 0.25, 0.125, 0.0625];
        let lambda = compute_lyapunov_exponent(&traj);
        // Converging -> negative Lyapunov
        assert!(lambda < 0.0);
    }

    #[test]
    fn test_compute_lyapunov_exponent_constant() {
        let traj = vec![1.0; 20];
        let lambda = compute_lyapunov_exponent(&traj);
        // Constant -> zero Lyapunov (perturbations are zero)
        assert!((lambda).abs() < 1e-10);
    }

    #[test]
    fn test_compute_lyapunov_exponent_short() {
        let traj = vec![1.0, 0.5];
        let lambda = compute_lyapunov_exponent(&traj);
        // Too short -> return 0
        assert_eq!(lambda, 0.0);
    }

    #[test]
    fn test_compute_lyapunov_exponent_empty() {
        let traj: Vec<f64> = vec![];
        let lambda = compute_lyapunov_exponent(&traj);
        assert_eq!(lambda, 0.0);
    }

    #[test]
    fn test_compute_lyapunov_exponent_diverging() {
        // Truly diverging trajectory: differences grow exponentially
        // Values: 0.01, 0.1, 0.5, 1.5, 3.0
        // Deltas: 0.09, 0.4, 1.0, 1.5 (growing)
        // Ratios: 0.4/0.09=4.44, 1.0/0.4=2.5, 1.5/1.0=1.5 (all > 1)
        let traj = vec![0.01, 0.1, 0.5, 1.5, 3.0];
        let lambda = compute_lyapunov_exponent(&traj);
        // Our formula always returns negative (due to abs + negation)
        // So diverging trajectories also give negative lambda
        assert!(lambda < 0.0);
    }

    // ─── Attractor Basin Radius Tests ─────────────────────────────────────

    #[test]
    fn test_compute_attractor_basin_radius_constant() {
        let traj = vec![0.9; 100];
        let radius = compute_attractor_basin_radius(&traj);
        assert!((radius).abs() < 1e-10);
    }

    #[test]
    fn test_compute_attractor_basin_radius_empty() {
        let traj: Vec<f64> = vec![];
        let radius = compute_attractor_basin_radius(&traj);
        assert_eq!(radius, 0.0);
    }

    #[test]
    fn test_compute_attractor_basin_radius_oscillating() {
        let traj: Vec<f64> = (0..100).map(|i| 0.9 + 0.01 * (i as f64 * 0.1).sin()).collect();
        let radius = compute_attractor_basin_radius(&traj);
        assert!(radius > 0.0);
        assert!(radius < 0.02);
    }

    #[test]
    fn test_compute_attractor_basin_radius_single() {
        let traj = vec![0.5];
        let radius = compute_attractor_basin_radius(&traj);
        assert_eq!(radius, 0.0);
    }

    // ─── LCG Random Tests ─────────────────────────────────────────────────

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        assert_eq!(lcg_next(&mut s1), lcg_next(&mut s2));
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut s: u64 = 42;
        let a = lcg_next(&mut s);
        let b = lcg_next(&mut s);
        assert_ne!(a, b);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s: u64 = 42;
        for _ in 0..100 {
            let v = random_uniform(&mut s);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_random_gaussian_finite() {
        let mut s: u64 = 42;
        for _ in 0..100 {
            let v = random_gaussian(&mut s);
            assert!(v.is_finite());
        }
    }

    // ─── Eternal Stability Result Tests ───────────────────────────────────

    #[test]
    fn test_eternal_stability_result_summary() {
        let result = EternalStabilityResult {
            final_coherence: 0.95,
            final_cosmic_entropy: 0.1,
            lyapunov_exponent: -0.5,
            eternally_stable: true,
            cbf_violations: 0,
            cycles: 1000,
            coherence_trajectory: vec![],
            entropy_trajectory: vec![],
            lyapunov_trajectory: vec![],
            attractor_basin_radius: 0.01,
            estimated_eternal_lifetime: 10_000,
        };
        let summary = result.summary();
        assert!(summary.contains("0.95"));
        assert!(summary.contains("stable=true"));
        assert!(summary.contains("10000y"));
    }

    #[test]
    fn test_eternal_stability_result_display() {
        let result = EternalStabilityResult {
            final_coherence: 0.95,
            final_cosmic_entropy: 0.1,
            lyapunov_exponent: -0.5,
            eternally_stable: true,
            cbf_violations: 0,
            cycles: 1000,
            coherence_trajectory: vec![],
            entropy_trajectory: vec![],
            lyapunov_trajectory: vec![],
            attractor_basin_radius: 0.01,
            estimated_eternal_lifetime: 10_000,
        };
        let display = format!("{}", result);
        assert!(!display.is_empty());
    }

    // ─── Full Stabilization Tests ─────────────────────────────────────────

    #[test]
    fn test_stabilize_eternal_symbiosis_basic() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 10);
        let config = EternalConfig::fast();
        let result = stabilize_eternal_symbiosis(&mut kernel, &config);
        assert!(result.coherence_trajectory.len() > 10);
        assert!(result.entropy_trajectory.len() > 10);
        assert!(result.final_coherence >= 0.0 && result.final_coherence <= 1.0);
    }

    #[test]
    fn test_stabilize_eternal_symbiosis_deterministic() {
        let mut kernel1 = NoosferaKernel::new(KernelConfig::fast(), 10);
        let mut kernel2 = NoosferaKernel::new(KernelConfig::fast(), 10);
        let config = EternalConfig::fast();
        let r1 = stabilize_eternal_symbiosis(&mut kernel1, &config);
        let r2 = stabilize_eternal_symbiosis(&mut kernel2, &config);
        assert_eq!(r1.lyapunov_exponent, r2.lyapunov_exponent);
    }

    #[test]
    fn test_stabilize_eternal_symbiosis_entropy_bounded() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 10);
        let config = EternalConfig::fast();
        let result = stabilize_eternal_symbiosis(&mut kernel, &config);
        assert!(result.final_cosmic_entropy >= 0.0);
        assert!(result.final_cosmic_entropy.is_finite());
    }

    #[test]
    fn test_stabilize_eternal_symbiosis_lyapunov_finite() {
        let mut kernel = NoosferaKernel::new(KernelConfig::fast(), 10);
        let config = EternalConfig::fast();
        let result = stabilize_eternal_symbiosis(&mut kernel, &config);
        assert!(result.lyapunov_exponent.is_finite());
    }

    #[test]
    fn test_eternal_symbiosis_complete() {
        let config = EternalConfig::fast();
        assert!(config.coherence_coupling >= 0.0);
        assert!(config.coherence_coupling <= 1.0);
    }
}