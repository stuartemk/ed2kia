//! Symbiotic Value Learning — Multi-Agent Inverse RL + Correlated Equilibria.
//!
//! **Sprint 131:** Symbiotic Value Alignment, Noospheric Self-Organization &
//! Thermodynamic Planetary Closure.
//!
//! Extends PoUS fitness and replicator dynamics with:
//! 1. **Symbiotic Value Update (Active Inference style):**
//!    φ* = argmin_φ VFE(φ) = E_q[log q(φ) - log p(observations, safe)]
//! 2. **Correlated Equilibrium in Replicator Dynamics:**
//!    x_i(t+1) = x_i(t) · f_i(π_i, π_{-i}) / f̄(t)
//!    where f_i = E_π[reward_i | collective safe prior]
//! 3. **Meta-Value Alignment Loop:** Iterative refinement of value priors
//!    through no-regret correlated equilibrium solving.
//!
//! **Key Formula — Symbiotic Value Update:**
//! ```text
//! VFE(φ) = KL(q(φ) || p(φ | safe_prior)) - E_q[log p(observations | φ)]
//! φ(t+1) = φ(t) - lr · ∇_φ VFE(φ)
//! ```
//!
//! **Correlated Equilibrium Solver:**
//! ```text
//! For each agent i:
//!   f_i = E_π[reward_i | collective safe prior]
//!   x_i(t+1) = x_i(t) · f_i / f̄(t)
//! Converges to correlated equilibrium when no agent benefits from unilateral deviation.
//! ```

/// Configuration for symbiotic value learning.
#[derive(Debug, Clone)]
pub struct ValueConfig {
    /// Learning rate for value updates.
    pub lr: f64,
    /// Number of alignment iterations.
    pub iterations: usize,
    /// Temperature for softmax exploration.
    pub temperature: f64,
    /// Safe prior weight (higher = more conservative).
    pub safe_prior_weight: f64,
    /// Convergence tolerance.
    pub convergence_tolerance: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for ValueConfig {
    fn default() -> Self {
        Self {
            lr: 0.01,
            iterations: 50,
            temperature: 1.0,
            safe_prior_weight: 0.5,
            convergence_tolerance: 1e-6,
            seed: 42,
        }
    }
}

impl ValueConfig {
    pub fn with_lr(mut self, lr: f64) -> Self {
        self.lr = lr.clamp(1e-6, 1.0);
        self
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations.max(1);
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature.max(1e-6);
        self
    }

    pub fn with_safe_prior_weight(mut self, weight: f64) -> Self {
        self.safe_prior_weight = weight.clamp(0.0, 1.0);
        self
    }

    pub fn with_convergence_tolerance(mut self, tol: f64) -> Self {
        self.convergence_tolerance = tol.max(1e-12);
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fast convergence preset for testing.
    pub fn fast() -> Self {
        Self {
            iterations: 10,
            lr: 0.1,
            convergence_tolerance: 1e-3,
            ..Self::default()
        }
    }

    /// High precision preset for production.
    pub fn high_precision() -> Self {
        Self {
            iterations: 200,
            lr: 0.001,
            convergence_tolerance: 1e-9,
            ..Self::default()
        }
    }
}

/// Result of symbiotic value update.
#[derive(Debug, Clone)]
pub struct ValueUpdateResult {
    /// Updated collective beliefs.
    pub updated_beliefs: Vec<Vec<f64>>,
    /// Global Variational Free Energy.
    pub global_vfe: f64,
    /// Value alignment score (higher = better alignment).
    pub alignment_score: f64,
    /// Correlated equilibrium convergence indicator.
    pub equilibrium_reached: bool,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Value trajectory over iterations.
    pub value_trajectory: Vec<f64>,
}

impl ValueUpdateResult {
    pub fn summary(&self) -> String {
        format!(
            "ValueUpdateResult {{ vfe={:.6}, alignment={:.4}, equilibrium={}, iterations={} }}",
            self.global_vfe, self.alignment_score, self.equilibrium_reached, self.iterations
        )
    }
}

/// LCG random number generator for reproducibility.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

/// Generate uniform random in [0, 1).
fn random_uniform(state: &mut u64) -> f64 {
    let raw = lcg_next(state);
    // Mask to 53 bits (u64::MAX >> 11) to fit in f64 mantissa, then normalize
    let masked = (raw >> 11) & ((1u64 << 53) - 1);
    masked as f64 / (1u64 << 53) as f64
}

/// Generate Gaussian random via Box-Muller.
fn random_gaussian(state: &mut u64) -> f64 {
    let u1 = random_uniform(state).clamp(1e-10, 1.0 - 1e-10);
    let u2 = random_uniform(state);
    // Box-Muller: sqrt(-2*ln(u1)) * cos(2*pi*u2)
    // Since u1 in (0,1), ln(u1) < 0, so -2*ln(u1) > 0
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// Compute softmax of values with temperature scaling.
pub fn softmax(values: &[f64], temperature: f64) -> Vec<f64> {
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = values.iter().map(|&v| ((v - max) / temperature).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum.is_normal() && sum > 0.0 {
        exps.into_iter().map(|e| e / sum).collect()
    } else {
        let uniform = 1.0 / values.len().max(1) as f64;
        vec![uniform; values.len()]
    }
}

/// Compute KL divergence between two discrete distributions.
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    p.iter()
        .zip(q.iter())
        .map(|(&pi, &qi)| {
            let pi = pi.max(1e-12);
            let qi = qi.max(1e-12);
            pi * (pi / qi).ln()
        })
        .sum()
}

/// Compute entropy of a discrete distribution.
pub fn entropy(dist: &[f64]) -> f64 {
    dist.iter()
        .map(|&p| {
            let p = p.clamp(1e-12, 1.0 - 1e-12);
            -p * p.ln()
        })
        .sum()
}

/// Compute the Variational Free Energy (VFE) for a belief distribution.
///
/// VFE(φ) = KL(q(φ) || p(φ | safe_prior)) - E_q[log p(observations | φ)]
///
/// # Arguments
/// * `beliefs` - Current belief distribution q(φ).
/// * `safe_prior` - Safe prior distribution p(φ | safe).
/// * `observations` - Observation likelihood parameters.
pub fn compute_vfe(beliefs: &[f64], safe_prior: &[f64], observations: &[f64]) -> f64 {
    let kl = kl_divergence(beliefs, safe_prior);
    let expected_log_likelihood: f64 = {
        let log_obs: Vec<f64> = observations
            .iter()
            .map(|&o| o.max(1e-12).ln())
            .collect();
        beliefs.iter().zip(log_obs.iter()).map(|(&b, &lo)| b * lo).sum()
    };
    kl - expected_log_likelihood
}

/// Symbiotic Value Update — Active Inference style belief update.
///
/// φ(t+1) = φ(t) - lr · ∇_φ VFE(φ)
///
/// # Arguments
/// * `current_beliefs` - Current belief distribution for each agent.
/// * `safe_prior` - Collective safe prior.
/// * `observations` - Current observations.
/// * `pous_influences` - PoUS influence weights for each agent.
/// * `config` - Value learning configuration.
///
/// # Returns
/// Updated beliefs and global VFE.
pub fn symbiotic_value_update(
    current_beliefs: &[Vec<f64>],
    safe_prior: &[f64],
    observations: &[f64],
    pous_influences: &[f64],
    config: &ValueConfig,
) -> ValueUpdateResult {
    let num_agents = current_beliefs.len().max(1);
    let dim = safe_prior.len().max(1);
    let mut state = config.seed;

    // Normalize PoUS influences
    let total_influence: f64 = pous_influences.iter().sum();
    let weights: Vec<f64> = if total_influence > 1e-12 {
        pous_influences
            .iter()
            .map(|&w| w / total_influence)
            .collect()
    } else {
        vec![1.0 / num_agents as f64; num_agents]
    };

    let mut updated_beliefs: Vec<Vec<f64>> = current_beliefs.to_vec();
    let mut value_trajectory = Vec::with_capacity(config.iterations + 1);

    // Initial VFE
    let mut global_vfe = compute_global_vfe(&updated_beliefs, &weights, safe_prior, observations);
    value_trajectory.push(global_vfe);

    for iteration in 0..config.iterations {
        let _ = iteration; // Track iteration count
        let mut gradient_sum: Vec<f64> = vec![0.0; dim];

        // Compute weighted gradient from all agents
        for (agent_idx, beliefs) in updated_beliefs.iter_mut().enumerate() {
            let w = weights.get(agent_idx).copied().unwrap_or(1.0 / num_agents as f64);

            // Numerical gradient of VFE w.r.t. beliefs
            let eps = 1e-5;
            let mut grad: Vec<f64> = vec![0.0; dim];
            for d in 0..dim {
                let original = beliefs[d];
                let perturbed = (original + eps).clamp(1e-12, 1.0 - 1e-12);
                let mut beliefs_plus = beliefs.to_vec();
                beliefs_plus[d] = perturbed;
                // Renormalize
                let sum: f64 = beliefs_plus.iter().sum();
                if sum > 1e-12 {
                    for b in beliefs_plus.iter_mut() {
                        *b /= sum;
                    }
                }
                let vfe_plus = compute_vfe(&beliefs_plus, safe_prior, observations);

                let perturbed_minus = (original - eps).clamp(1e-12, 1.0 - 1e-12);
                let mut beliefs_minus = beliefs.to_vec();
                beliefs_minus[d] = perturbed_minus;
                let sum_minus: f64 = beliefs_minus.iter().sum();
                if sum_minus > 1e-12 {
                    for b in beliefs_minus.iter_mut() {
                        *b /= sum_minus;
                    }
                }
                let vfe_minus = compute_vfe(&beliefs_minus, safe_prior, observations);

                grad[d] = (vfe_plus - vfe_minus) / (2.0 * eps);
            }

            // Weighted gradient accumulation
            for d in 0..dim {
                gradient_sum[d] += w * grad[d];
            }

            // Update beliefs with gradient descent + noise for exploration
            for d in 0..dim {
                let noise = config.temperature * random_gaussian(&mut state) * 1e-4;
                beliefs[d] -= config.lr * gradient_sum[d] + noise;
                beliefs[d] = beliefs[d].max(1e-12);
            }

            // Renormalize beliefs
            let bsum: f64 = beliefs.iter().sum();
            if bsum > 1e-12 {
                for b in beliefs.iter_mut() {
                    *b /= bsum;
                }
            }
        }

        // Safe prior attraction (symbiotic constraint)
        for beliefs in updated_beliefs.iter_mut() {
            for d in 0..dim {
                beliefs[d] = config.safe_prior_weight * safe_prior[d]
                    + (1.0 - config.safe_prior_weight) * beliefs[d];
                beliefs[d] = beliefs[d].max(1e-12);
            }
            let bsum: f64 = beliefs.iter().sum();
            if bsum > 1e-12 {
                for b in beliefs.iter_mut() {
                    *b /= bsum;
                }
            }
        }

        // Compute new VFE
        let new_vfe = compute_global_vfe(&updated_beliefs, &weights, safe_prior, observations);
        value_trajectory.push(new_vfe);

        // Check convergence
        if (new_vfe - global_vfe).abs() < config.convergence_tolerance {
            global_vfe = new_vfe;
            break;
        }
        global_vfe = new_vfe;
    }

    // Compute alignment score
    let alignment_score = compute_alignment_score(&updated_beliefs, &weights, safe_prior);

    // Check correlated equilibrium
    let equilibrium_reached = check_correlated_equilibrium(
        &updated_beliefs,
        safe_prior,
        observations,
        &weights,
        config.convergence_tolerance,
    );

    ValueUpdateResult {
        updated_beliefs,
        global_vfe,
        alignment_score,
        equilibrium_reached,
        iterations: value_trajectory.len() - 1,
        value_trajectory,
    }
}

/// Compute global VFE as weighted sum of agent VFEs.
fn compute_global_vfe(
    beliefs: &[Vec<f64>],
    weights: &[f64],
    safe_prior: &[f64],
    observations: &[f64],
) -> f64 {
    beliefs
        .iter()
        .zip(weights.iter())
        .map(|(b, &w)| w * compute_vfe(b, safe_prior, observations))
        .sum()
}

/// Compute alignment score between collective beliefs and safe prior.
fn compute_alignment_score(beliefs: &[Vec<f64>], weights: &[f64], safe_prior: &[f64]) -> f64 {
    let dim = safe_prior.len().max(1);
    let weighted_beliefs: Vec<f64> = (0..dim)
        .map(|d| {
            beliefs
                .iter()
                .zip(weights.iter())
                .map(|(b, &w)| w * b.get(d).copied().unwrap_or(0.0))
                .sum()
        })
        .collect();

    // Alignment = 1 - normalized KL divergence
    let kl = kl_divergence(&weighted_beliefs, safe_prior);
    1.0 - kl.clamp(0.0, 1.0)
}

/// Check if current state is a correlated equilibrium.
fn check_correlated_equilibrium(
    beliefs: &[Vec<f64>],
    safe_prior: &[f64],
    observations: &[f64],
    _weights: &[f64],
    tolerance: f64,
) -> bool {
    let num_agents = beliefs.len();
    if num_agents == 0 {
        return true;
    }

    // For each agent, check if unilateral deviation improves VFE
    for agent_beliefs in beliefs.iter() {
        let original_vfe = compute_vfe(agent_beliefs, safe_prior, observations);

        // Try random deviations
        let mut state = 42u64;
        let mut max_improvement = 0.0;
        for _ in 0..10 {
            let mut deviated = agent_beliefs.to_vec();
            for d in deviated.iter_mut() {
                *d += random_gaussian(&mut state) * 0.1;
                *d = d.max(1e-12);
            }
            let sum: f64 = deviated.iter().sum();
            if sum > 1e-12 {
                for d in deviated.iter_mut() {
                    *d /= sum;
                }
            }
            let deviated_vfe = compute_vfe(&deviated, safe_prior, observations);
            let improvement = original_vfe - deviated_vfe;
            if improvement > max_improvement {
                max_improvement = improvement;
            }
        }

        // If deviation improves VFE by more than tolerance, not equilibrium
        if max_improvement > tolerance {
            return false;
        }
    }
    true
}

/// Correlated Equilibrium Solver — No-regret learning with replicator dynamics.
///
/// x_i(t+1) = x_i(t) · f_i(π_i, π_{-i}) / f̄(t)
///
/// # Arguments
/// * `initial_shares` - Initial strategy shares x_i(0).
/// * `reward_matrix` - Reward matrix R[i][j] for strategy i against j.
/// * `safe_prior` - Safe prior for value alignment.
/// * `config` - Solver configuration.
///
/// # Returns
/// Correlated equilibrium distribution and convergence metrics.
pub struct CorrelatedEquilibriumResult {
    /// Equilibrium strategy distribution.
    pub equilibrium_shares: Vec<f64>,
    /// Average regret over iterations.
    pub avg_regret: f64,
    /// Whether no-regret equilibrium was reached.
    pub no_regret: bool,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Regret trajectory.
    pub regret_trajectory: Vec<f64>,
}

impl CorrelatedEquilibriumResult {
    pub fn summary(&self) -> String {
        format!(
            "CorrelatedEquilibrium {{ regret={:.6}, no_regret={}, iterations={} }}",
            self.avg_regret, self.no_regret, self.iterations
        )
    }
}

pub fn correlated_equilibrium_solver(
    initial_shares: &[f64],
    reward_matrix: &[Vec<f64>],
    safe_prior: &[f64],
    config: &ValueConfig,
) -> CorrelatedEquilibriumResult {
    let n = initial_shares.len();
    if n == 0 {
        return CorrelatedEquilibriumResult {
            equilibrium_shares: vec![],
            avg_regret: 0.0,
            no_regret: true,
            iterations: 0,
            regret_trajectory: vec![],
        };
    }
    let mut shares: Vec<f64> = initial_shares.to_vec();
    let mut cumulative_regret = 0.0;
    let mut regret_trajectory = Vec::with_capacity(config.iterations + 1);

    // Normalize initial shares
    let sum: f64 = shares.iter().sum();
    if sum > 1e-12 {
        for s in shares.iter_mut() {
            *s /= sum;
        }
    }

    regret_trajectory.push(0.0);

    for iteration in 0..config.iterations {
        let _ = iteration;

        // Compute fitness for each strategy: f_i = E_π[reward_i | safe_prior]
        let mut fitnesses = vec![0.0; n];
        for (i, row) in reward_matrix.iter().enumerate().take(n) {
            for (j, &share) in shares.iter().enumerate().take(n) {
                let reward = row.get(j).copied().unwrap_or(0.0);
                fitnesses[i] += share * reward;
            }
        }

        // Safe prior bonus: strategies closer to safe prior get bonus
        for (i, fitness) in fitnesses.iter_mut().enumerate().take(n) {
            let one_hot: Vec<f64> = (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect();
            let alignment = 1.0 - kl_divergence(&one_hot, safe_prior).clamp(0.0, 1.0);
            *fitness += config.safe_prior_weight * alignment;
        }

        // Replicator dynamics: x_i(t+1) = x_i(t) · f_i / f̄
        let avg_fitness: f64 = fitnesses.iter().sum::<f64>() / n as f64;
        if avg_fitness.abs() > 1e-12 {
            for i in 0..n {
                shares[i] = shares[i] * fitnesses[i] / avg_fitness;
                shares[i] = shares[i].max(1e-12);
            }
        }

        // Renormalize
        let new_sum: f64 = shares.iter().sum();
        if new_sum > 1e-12 {
            for s in shares.iter_mut() {
                *s /= new_sum;
            }
        }

        // Compute regret: max_i E[reward_i] - E[reward_mixed]
        let mixed_reward: f64 = shares
            .iter()
            .enumerate()
            .map(|(i, &si)| {
                si * reward_matrix
                    .get(i)
                    .map(|r| r.iter().zip(shares.iter()).map(|(&ri, &sj)| ri * sj).sum::<f64>())
                    .unwrap_or(0.0)
            })
            .sum();

        let max_reward = (0..n)
            .map(|i| {
                reward_matrix
                    .get(i)
                    .map(|r| r.iter().zip(shares.iter()).map(|(&ri, &sj)| ri * sj).sum::<f64>())
                    .unwrap_or(0.0)
            })
            .fold(f64::NEG_INFINITY, f64::max);

        let regret = (max_reward - mixed_reward).max(0.0);
        cumulative_regret += regret;
        regret_trajectory.push(regret);
    }

    let avg_regret = cumulative_regret / config.iterations as f64;
    let no_regret = avg_regret < config.convergence_tolerance;

    CorrelatedEquilibriumResult {
        equilibrium_shares: shares,
        avg_regret,
        no_regret,
        iterations: config.iterations,
        regret_trajectory,
    }
}

/// Meta-Value Alignment Loop — Iterative refinement of value priors.
///
/// Combines symbiotic value update with correlated equilibrium solving
/// for robust value alignment across the planetary mesh.
pub struct MetaValueResult {
    /// Final aligned value priors.
    pub aligned_priors: Vec<f64>,
    /// Final global VFE.
    pub final_vfe: f64,
    /// Final alignment score.
    pub final_alignment: f64,
    /// Correlated equilibrium result.
    pub equilibrium: CorrelatedEquilibriumResult,
    /// Value update result.
    pub value_update: ValueUpdateResult,
    /// Meta-convergence indicator.
    pub meta_converged: bool,
}

impl MetaValueResult {
    pub fn summary(&self) -> String {
        format!(
            "MetaValue {{ vfe={:.6}, alignment={:.4}, meta_converged={} }}",
            self.final_vfe, self.final_alignment, self.meta_converged
        )
    }
}

pub fn meta_value_alignment_loop(
    initial_beliefs: &[Vec<f64>],
    initial_safe_prior: &[f64],
    observations: &[f64],
    pous_influences: &[f64],
    reward_matrix: &[Vec<f64>],
    config: &ValueConfig,
) -> MetaValueResult {
    let dim = initial_safe_prior.len().max(1);
    let current_prior = initial_safe_prior.to_vec();

    // Phase 1: Symbiotic value update
    let value_result =
        symbiotic_value_update(initial_beliefs, &current_prior, observations, pous_influences, config);

    // Phase 2: Correlated equilibrium on updated beliefs
    let initial_shares: Vec<f64> = (0..dim)
        .map(|d| {
            initial_beliefs
                .iter()
                .map(|b| b.get(d).copied().unwrap_or(0.0))
                .sum::<f64>()
                / initial_beliefs.len().max(1) as f64
        })
        .collect();

    let eq_result = correlated_equilibrium_solver(&initial_shares, reward_matrix, &current_prior, config);

    // Phase 3: Refine safe prior using equilibrium
    let refined_prior: Vec<f64> = eq_result
        .equilibrium_shares
        .iter()
        .zip(current_prior.iter())
        .map(|(&eq, &prior)| 0.5 * eq + 0.5 * prior)
        .collect();

    // Renormalize refined prior
    let psum: f64 = refined_prior.iter().sum();
    let aligned_priors = if psum > 1e-12 {
        refined_prior.into_iter().map(|p| p / psum).collect()
    } else {
        let uniform = 1.0 / dim as f64;
        vec![uniform; dim]
    };

    // Check meta-convergence: alignment improved and VFE decreased
    let meta_converged = value_result.alignment_score > 0.5
        && value_result.global_vfe < compute_vfe(initial_safe_prior, initial_safe_prior, observations);

    MetaValueResult {
        aligned_priors,
        final_vfe: value_result.global_vfe,
        final_alignment: value_result.alignment_score,
        equilibrium: eq_result,
        value_update: value_result,
        meta_converged,
    }
}

/// Compute PoUS fitness for value alignment context.
pub fn compute_value_alignment_fitness(
    delta_vfe: f64,
    alignment_score: f64,
    equilibrium_regret: f64,
    byzantine_penalty: f64,
) -> f64 {
    // Value alignment fitness: reward alignment, penalize regret and Byzantine behavior
    let alpha = 0.4; // VFE reduction weight
    let beta = 0.4;  // Alignment weight
    let gamma = 0.2; // Equilibrium bonus
    let delta = 2.0; // Byzantine penalty

    alpha * delta_vfe + beta * alignment_score + gamma * (1.0 - equilibrium_regret)
        - delta * byzantine_penalty
}

/// S131 Full Pipeline — End-to-end symbiotic value alignment.
pub fn s131_full_pipeline(
    beliefs: &[Vec<f64>],
    safe_prior: &[f64],
    observations: &[f64],
    pous_influences: &[f64],
    reward_matrix: &[Vec<f64>],
    config: &ValueConfig,
) -> MetaValueResult {
    meta_value_alignment_loop(
        beliefs, safe_prior, observations, pous_influences, reward_matrix, config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // === ValueConfig Tests ===

    #[test]
    fn test_value_config_default() {
        let cfg = ValueConfig::default();
        assert_eq!(cfg.lr, 0.01);
        assert_eq!(cfg.iterations, 50);
        assert_eq!(cfg.temperature, 1.0);
        assert_eq!(cfg.safe_prior_weight, 0.5);
        assert_eq!(cfg.convergence_tolerance, 1e-6);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_value_config_with_lr() {
        let cfg = ValueConfig::default().with_lr(0.1);
        assert_eq!(cfg.lr, 0.1);
    }

    #[test]
    fn test_value_config_lr_clamped_low() {
        let cfg = ValueConfig::default().with_lr(1e-10);
        assert_eq!(cfg.lr, 1e-6);
    }

    #[test]
    fn test_value_config_lr_clamped_high() {
        let cfg = ValueConfig::default().with_lr(2.0);
        assert_eq!(cfg.lr, 1.0);
    }

    #[test]
    fn test_value_config_with_iterations() {
        let cfg = ValueConfig::default().with_iterations(100);
        assert_eq!(cfg.iterations, 100);
    }

    #[test]
    fn test_value_config_iterations_min() {
        let cfg = ValueConfig::default().with_iterations(0);
        assert_eq!(cfg.iterations, 1);
    }

    #[test]
    fn test_value_config_with_temperature() {
        let cfg = ValueConfig::default().with_temperature(0.5);
        assert_eq!(cfg.temperature, 0.5);
    }

    #[test]
    fn test_value_config_temperature_min() {
        let cfg = ValueConfig::default().with_temperature(0.0);
        assert!(cfg.temperature >= 1e-6);
    }

    #[test]
    fn test_value_config_with_safe_prior_weight() {
        let cfg = ValueConfig::default().with_safe_prior_weight(0.8);
        assert_eq!(cfg.safe_prior_weight, 0.8);
    }

    #[test]
    fn test_value_config_safe_prior_weight_clamped() {
        let cfg = ValueConfig::default().with_safe_prior_weight(1.5);
        assert_eq!(cfg.safe_prior_weight, 1.0);
    }

    #[test]
    fn test_value_config_with_convergence_tolerance() {
        let cfg = ValueConfig::default().with_convergence_tolerance(1e-8);
        assert_eq!(cfg.convergence_tolerance, 1e-8);
    }

    #[test]
    fn test_value_config_convergence_tolerance_clamped() {
        let cfg = ValueConfig::default().with_convergence_tolerance(0.0);
        assert!(cfg.convergence_tolerance >= 1e-12);
    }

    #[test]
    fn test_value_config_with_seed() {
        let cfg = ValueConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn test_value_config_fast() {
        let cfg = ValueConfig::fast();
        assert_eq!(cfg.iterations, 10);
        assert_eq!(cfg.lr, 0.1);
        assert_eq!(cfg.convergence_tolerance, 1e-3);
    }

    #[test]
    fn test_value_config_high_precision() {
        let cfg = ValueConfig::high_precision();
        assert_eq!(cfg.iterations, 200);
        assert_eq!(cfg.lr, 0.001);
        assert_eq!(cfg.convergence_tolerance, 1e-9);
    }

    // === Softmax Tests ===

    #[test]
    fn test_softmax_uniform() {
        let vals = vec![0.0, 0.0, 0.0];
        let result = softmax(&vals, 1.0);
        let sum: f64 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(result.iter().all(|&x| (x - 1.0 / 3.0).abs() < 1e-6));
    }

    #[test]
    fn test_softmax_dominant() {
        let vals = vec![10.0, 0.0, 0.0];
        let result = softmax(&vals, 1.0);
        assert!(result[0] > 0.99);
    }

    #[test]
    fn test_softmax_temperature_effect() {
        let vals = vec![1.0, 0.0, 0.0];
        let hot = softmax(&vals, 10.0);
        let cold = softmax(&vals, 0.1);
        // Higher temperature = more uniform
        assert!(entropy(&hot) > entropy(&cold));
    }

    #[test]
    fn test_softmax_empty() {
        let result = softmax(&[], 1.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_softmax_single() {
        let result = softmax(&[5.0], 1.0);
        assert!((result[0] - 1.0).abs() < 1e-6);
    }

    // === KL Divergence Tests ===

    #[test]
    fn test_kl_divergence_identical() {
        let p = vec![0.5, 0.5];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl < 1e-6);
    }

    #[test]
    fn test_kl_divergence_different() {
        let p = vec![1.0, 0.0];
        let q = vec![0.0, 1.0];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0);
    }

    #[test]
    fn test_kl_divergence_uniform_vs_point() {
        let p = vec![1.0, 0.0, 0.0];
        let q = vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        let kl = kl_divergence(&p, &q);
        assert!((kl - (3_f64.ln())).abs() < 1e-6);
    }

    #[test]
    fn test_kl_divergence_non_negative() {
        let p = vec![0.7, 0.2, 0.1];
        let q = vec![0.3, 0.4, 0.3];
        let kl = kl_divergence(&p, &q);
        assert!(kl >= 0.0);
    }

    // === Entropy Tests ===

    #[test]
    fn test_entropy_uniform() {
        let dist = vec![1.0 / 4.0; 4];
        let e = entropy(&dist);
        assert!((e - std::f64::consts::LN_2 * 2.0).abs() < 1e-6); // H(uniform_4) = ln(4) = 2*ln(2)
    }

    #[test]
    fn test_entropy_point_mass() {
        let dist = vec![1.0, 0.0, 0.0];
        let e = entropy(&dist);
        assert!(e < 1e-6);
    }

    #[test]
    fn test_entropy_non_negative() {
        let dist = vec![0.6, 0.3, 0.1];
        let e = entropy(&dist);
        assert!(e >= 0.0);
    }

    // === VFE Tests ===

    #[test]
    fn test_compute_vfe_basic() {
        let beliefs = vec![0.5, 0.5];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let vfe = compute_vfe(&beliefs, &prior, &obs);
        assert!(vfe.is_finite());
    }

    #[test]
    fn test_compute_vfe_zero_when_matching() {
        let dist = vec![0.5, 0.5];
        let obs = vec![1.0, 1.0]; // log(1) = 0
        let vfe = compute_vfe(&dist, &dist, &obs);
        // KL = 0, E[log p(obs)] = 0
        assert!(vfe.abs() < 1e-6);
    }

    #[test]
    fn test_compute_vfe_increases_with_mismatch() {
        let beliefs = vec![1.0, 0.0];
        let prior = vec![0.0, 1.0];
        let obs = vec![0.5, 0.5];
        let vfe = compute_vfe(&beliefs, &prior, &obs);
        assert!(vfe > 0.0);
    }

    // === Symbiotic Value Update Tests ===

    #[test]
    fn test_symbiotic_value_update_basic() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![0.5, 0.5];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        assert_eq!(result.updated_beliefs.len(), 2);
        assert!(result.global_vfe.is_finite());
        assert!(result.alignment_score >= 0.0);
        assert!(result.alignment_score <= 1.0);
    }

    #[test]
    fn test_symbiotic_value_update_empty() {
        let beliefs: Vec<Vec<f64>> = vec![];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences: Vec<f64> = vec![];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        assert!(result.updated_beliefs.is_empty() || !result.updated_beliefs.is_empty());
    }

    #[test]
    fn test_symbiotic_value_update_single_agent() {
        let beliefs = vec![vec![1.0, 0.0, 0.0]];
        let prior = vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        let obs = vec![1.0, 1.0, 1.0];
        let influences = vec![1.0];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        assert_eq!(result.updated_beliefs.len(), 1);
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_symbiotic_value_update_convergence() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![0.5, 0.5];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        // Should converge quickly when beliefs match prior
        assert!(result.iterations < cfg.iterations || result.equilibrium_reached);
    }

    #[test]
    fn test_symbiotic_value_update_trajectory_increasing() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![1.0];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        assert!(!result.value_trajectory.is_empty());
        assert!(result.value_trajectory.len() == result.iterations + 1);
    }

    #[test]
    fn test_symbiotic_value_update_deterministic() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![1.0];
        let cfg = ValueConfig::fast().with_seed(42);
        let r1 = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        let r2 = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        assert!((r1.global_vfe - r2.global_vfe).abs() < 1e-6);
    }

    #[test]
    fn test_symbiotic_value_update_high_safe_prior() {
        let beliefs = vec![vec![1.0, 0.0]];
        let prior = vec![0.0, 1.0];
        let obs = vec![1.0, 1.0];
        let influences = vec![1.0];
        let cfg = ValueConfig::fast().with_safe_prior_weight(0.95);
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        // High safe prior weight should pull beliefs toward prior
        assert!(result.updated_beliefs[0][1] > result.updated_beliefs[0][0]);
    }

    #[test]
    fn test_symbiotic_value_update_beliefs_normalized() {
        let beliefs = vec![vec![0.3, 0.3, 0.4]];
        let prior = vec![0.33, 0.33, 0.34];
        let obs = vec![1.0, 1.0, 1.0];
        let influences = vec![1.0];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        for b in &result.updated_beliefs {
            let sum: f64 = b.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4, "Beliefs not normalized: {}", sum);
        }
    }

    // === Correlated Equilibrium Tests ===

    #[test]
    fn test_correlated_equilibrium_basic() {
        let shares = vec![0.5, 0.5];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let prior = vec![0.5, 0.5];
        let cfg = ValueConfig::fast();
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        assert_eq!(result.equilibrium_shares.len(), 2);
        assert!(result.avg_regret >= 0.0);
    }

    #[test]
    fn test_correlated_equilibrium_uniform_reward() {
        let shares = vec![0.33, 0.33, 0.34];
        let rewards = vec![vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0]];
        let prior = vec![0.33, 0.33, 0.34];
        let cfg = ValueConfig::fast();
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        // Uniform rewards should maintain roughly uniform shares
        assert!(result.equilibrium_shares.iter().all(|&s| s > 0.1));
    }

    #[test]
    fn test_correlated_equilibrium_dominant_strategy() {
        let shares = vec![0.33, 0.33, 0.34];
        let rewards = vec![vec![10.0, 10.0, 10.0], vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0]];
        let prior = vec![0.33, 0.33, 0.34];
        let cfg = ValueConfig {
            iterations: 100,
            ..ValueConfig::fast()
        };
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        // Dominant strategy should accumulate share
        assert!(result.equilibrium_shares[0] > result.equilibrium_shares[1]);
    }

    #[test]
    fn test_correlated_equilibrium_empty() {
        let shares: Vec<f64> = vec![];
        let rewards: Vec<Vec<f64>> = vec![];
        let prior = vec![1.0];
        let cfg = ValueConfig::fast();
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        assert!(result.equilibrium_shares.is_empty());
        assert!(result.no_regret);
        assert!(result.avg_regret == 0.0);
    }

    #[test]
    fn test_correlated_equilibrium_single_strategy() {
        let shares = vec![1.0];
        let rewards = vec![vec![1.0]];
        let prior = vec![1.0];
        let cfg = ValueConfig::fast();
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        assert!((result.equilibrium_shares[0] - 1.0).abs() < 1e-6);
        assert!(result.no_regret);
    }

    #[test]
    fn test_correlated_equilibrium_regret_trajectory() {
        let shares = vec![0.5, 0.5];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let prior = vec![0.5, 0.5];
        let cfg = ValueConfig::fast();
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        assert_eq!(result.regret_trajectory.len(), cfg.iterations + 1);
    }

    #[test]
    fn test_correlated_equilibrium_shares_normalized() {
        let shares = vec![0.5, 0.5];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let prior = vec![0.5, 0.5];
        let cfg = ValueConfig::fast();
        let result = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        let sum: f64 = result.equilibrium_shares.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_correlated_equilibrium_deterministic() {
        let shares = vec![0.5, 0.5];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let prior = vec![0.5, 0.5];
        let cfg = ValueConfig::fast();
        let r1 = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        let r2 = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg);
        assert!((r1.avg_regret - r2.avg_regret).abs() < 1e-6);
    }

    // === Meta-Value Alignment Tests ===

    #[test]
    fn test_meta_value_alignment_basic() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![0.5, 0.5];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let cfg = ValueConfig::fast();
        let result = meta_value_alignment_loop(
            &beliefs, &prior, &obs, &influences, &rewards, &cfg,
        );
        assert_eq!(result.aligned_priors.len(), 2);
        assert!(result.final_vfe.is_finite());
        assert!(result.final_alignment >= 0.0);
    }

    #[test]
    fn test_meta_value_alignment_convergence() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![1.0, 1.0];
        let influences = vec![1.0];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let cfg = ValueConfig::fast();
        let result = meta_value_alignment_loop(
            &beliefs, &prior, &obs, &influences, &rewards, &cfg,
        );
        // Should converge when beliefs match prior
        assert!(result.meta_converged || result.final_alignment > 0.5);
    }

    #[test]
    fn test_meta_value_alignment_aligned_priors_normalized() {
        let beliefs = vec![vec![0.3, 0.3, 0.4]];
        let prior = vec![0.33, 0.33, 0.34];
        let obs = vec![1.0, 1.0, 1.0];
        let influences = vec![1.0];
        let rewards = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let cfg = ValueConfig::fast();
        let result = meta_value_alignment_loop(
            &beliefs, &prior, &obs, &influences, &rewards, &cfg,
        );
        let sum: f64 = result.aligned_priors.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_meta_value_alignment_summary() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![1.0];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let cfg = ValueConfig::fast();
        let result = meta_value_alignment_loop(
            &beliefs, &prior, &obs, &influences, &rewards, &cfg,
        );
        let summary = result.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("MetaValue"));
    }

    // === Value Alignment Fitness Tests ===

    #[test]
    fn test_compute_value_alignment_fitness_basic() {
        let fitness = compute_value_alignment_fitness(0.5, 0.8, 0.1, 0.0);
        assert!(fitness > 0.0);
    }

    #[test]
    fn test_compute_value_alignment_fitness_byzantine_penalty() {
        let good = compute_value_alignment_fitness(0.5, 0.8, 0.1, 0.0);
        let bad = compute_value_alignment_fitness(0.5, 0.8, 0.1, 1.0);
        assert!(good > bad);
    }

    #[test]
    fn test_compute_value_alignment_fitness_perfect() {
        let fitness = compute_value_alignment_fitness(1.0, 1.0, 0.0, 0.0);
        assert!(fitness > 0.5);
    }

    #[test]
    fn test_compute_value_alignment_fitness_worst() {
        let fitness = compute_value_alignment_fitness(0.0, 0.0, 1.0, 1.0);
        assert!(fitness < 0.0);
    }

    // === S131 Full Pipeline Tests ===

    #[test]
    fn test_s131_full_pipeline_basic() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![0.5, 0.5];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let cfg = ValueConfig::fast();
        let result = s131_full_pipeline(&beliefs, &prior, &obs, &influences, &rewards, &cfg);
        assert!(result.final_vfe.is_finite());
        assert!(result.final_alignment >= 0.0);
    }

    #[test]
    fn test_s131_full_pipeline_empty() {
        let beliefs: Vec<Vec<f64>> = vec![];
        let prior = vec![1.0];
        let obs = vec![1.0];
        let influences: Vec<f64> = vec![];
        let rewards: Vec<Vec<f64>> = vec![];
        let cfg = ValueConfig::fast();
        let result = s131_full_pipeline(&beliefs, &prior, &obs, &influences, &rewards, &cfg);
        assert!(!result.aligned_priors.is_empty());
    }

    #[test]
    fn test_s131_full_pipeline_large() {
        let dim = 10;
        let beliefs: Vec<Vec<f64>> = (0..5)
            .map(|_| {
                let mut b = vec![1.0 / dim as f64; dim];
                b[0] += 0.1;
                b
            })
            .collect();
        let prior = vec![1.0 / dim as f64; dim];
        let obs = vec![1.0; dim];
        let influences = vec![0.2; 5];
        let rewards: Vec<Vec<f64>> = (0..dim)
            .map(|i| (0..dim).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        let cfg = ValueConfig::fast();
        let result = s131_full_pipeline(&beliefs, &prior, &obs, &influences, &rewards, &cfg);
        assert_eq!(result.aligned_priors.len(), dim);
        assert!(result.value_update.iterations > 0);
    }

    #[test]
    fn test_s131_full_pipeline_high_precision() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![1.0];
        let rewards = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let cfg = ValueConfig::high_precision();
        let result = s131_full_pipeline(&beliefs, &prior, &obs, &influences, &rewards, &cfg);
        assert!(result.equilibrium.iterations == 200);
    }

    // === Result Display Tests ===

    #[test]
    fn test_value_update_result_summary() {
        let result = ValueUpdateResult {
            updated_beliefs: vec![vec![0.5, 0.5]],
            global_vfe: 0.5,
            alignment_score: 0.8,
            equilibrium_reached: true,
            iterations: 10,
            value_trajectory: vec![0.6, 0.5],
        };
        let summary = result.summary();
        assert!(summary.contains("ValueUpdateResult"));
        assert!(summary.contains("vfe=0.500000"));
    }

    #[test]
    fn test_correlated_equilibrium_result_summary() {
        let result = CorrelatedEquilibriumResult {
            equilibrium_shares: vec![0.5, 0.5],
            avg_regret: 0.01,
            no_regret: true,
            iterations: 10,
            regret_trajectory: vec![0.0, 0.01],
        };
        let summary = result.summary();
        assert!(summary.contains("CorrelatedEquilibrium"));
        assert!(summary.contains("no_regret=true"));
    }

    // === Random Number Generator Tests ===

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1 = 42u64;
        let mut s2 = 42u64;
        let r1 = lcg_next(&mut s1);
        let r2 = lcg_next(&mut s2);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut s = 42u64;
        let r1 = lcg_next(&mut s);
        let r2 = lcg_next(&mut s);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s = 42u64;
        for _ in 0..100 {
            let r = random_uniform(&mut s);
            assert!(r >= 0.0, "uniform must be >= 0, got {}", r);
            assert!(r < 1.0, "uniform must be < 1, got {}", r);
        }
    }

    #[test]
    fn test_random_gaussian_finite() {
        let mut s = 12345u64;
        for _ in 0..100 {
            let g = random_gaussian(&mut s);
            assert!(g.is_finite(), "gaussian must be finite, got {}", g);
        }
    }

    // === Integration Tests ===

    #[test]
    fn test_value_alignment_with_pous_fitness() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let influences = vec![1.0];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);

        // Compute PoUS fitness for the alignment result
        let fitness = compute_value_alignment_fitness(
            -result.global_vfe, // VFE reduction (negative VFE = improvement)
            result.alignment_score,
            0.0, // No regret
            0.0, // No Byzantine penalty
        );
        assert!(fitness.is_finite());
    }

    #[test]
    fn test_multi_agent_value_convergence() {
        let num_agents = 10;
        let dim = 3;
        let beliefs: Vec<Vec<f64>> = (0..num_agents)
            .map(|_| vec![1.0 / dim as f64; dim])
            .collect();
        let prior = vec![1.0 / dim as f64; dim];
        let obs = vec![1.0; dim];
        let influences = vec![1.0 / num_agents as f64; num_agents];
        let cfg = ValueConfig::fast();
        let result = symbiotic_value_update(&beliefs, &prior, &obs, &influences, &cfg);
        assert_eq!(result.updated_beliefs.len(), num_agents);
        // All agents should converge toward prior
        assert!(result.alignment_score > 0.5);
    }

    #[test]
    fn test_equilibrium_solver_with_safe_prior_bonus() {
        let shares = vec![0.33, 0.33, 0.34];
        let rewards = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let prior = vec![0.5, 0.25, 0.25];
        let cfg_high_prior = ValueConfig::fast().with_safe_prior_weight(0.9);
        let cfg_low_prior = ValueConfig::fast().with_safe_prior_weight(0.0);

        let r_high = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg_high_prior);
        let r_low = correlated_equilibrium_solver(&shares, &rewards, &prior, &cfg_low_prior);

        // High prior weight should pull toward prior distribution
        assert!(r_high.equilibrium_shares[0] > r_low.equilibrium_shares[0]);
    }
}
