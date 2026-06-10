//! Thermodynamic Planetary Free Energy & Active Inference Closure.
//!
//! **Sprint 131 C:** Thermodynamic Planetary Free Energy & Active Inference Closure.
//!
//! Implements the thermodynamic closure layer for the Noospheric mesh:
//! 1. **Planetary Variational Free Energy (VFE):**
//!    F_planet = Σ x_i · VFE_i + λ · energy_distribution_entropy - symbiosis_bonus
//! 2. **Active Inference Planetary Step:**
//!    Gradient flow optimization over the planetary mesh topology.
//! 3. **Thermodynamic Resilience Score:**
//!    Measures system resilience against adversarial perturbations.
//!
//! **Key Formula — Planetary Free Energy:**
//! ```text
//! F_planet = Σ_i x_i · VFE_i + λ · H(energy_dist) - γ · symbiosis_bonus
//! where:
//!   x_i = influence share of node i
//!   VFE_i = Variational Free Energy of node i
//!   H(energy_dist) = Shannon entropy of energy distribution
//!   symbiosis_bonus = Σ_i<j sim(belief_i, belief_j) · cooperation_ij
//! ```
//!
//! **Active Inference Update:**
//! ```text
//! φ(t+1) = φ(t) - lr · ∇_φ F_planet(φ)
//! Convergence when |F(t) - F(t-1)| < tolerance
//! ```
//!
//! **Thermodynamic Resilience:**
//! ```text
//! R_thermo = 1 / (1 + F_planet / F_baseline)
//! Higher resilience = lower free energy relative to baseline
//! ```

/// Configuration for thermodynamic planetary operations.
#[derive(Debug, Clone)]
pub struct ThermoConfig {
    /// Learning rate for active inference updates.
    pub lr: f64,
    /// Number of active inference iterations.
    pub iterations: usize,
    /// Energy distribution entropy weight (λ).
    pub energy_entropy_weight: f64,
    /// Symbiosis bonus weight (γ).
    pub symbiosis_weight: f64,
    /// Convergence tolerance for free energy.
    pub convergence_tolerance: f64,
    /// Baseline free energy for resilience normalization.
    pub baseline_vfe: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for ThermoConfig {
    fn default() -> Self {
        Self {
            lr: 0.01,
            iterations: 50,
            energy_entropy_weight: 0.3,
            symbiosis_weight: 0.5,
            convergence_tolerance: 1e-6,
            baseline_vfe: 1.0,
            seed: 42,
        }
    }
}

impl ThermoConfig {
    /// Create config with custom learning rate.
    pub fn with_lr(mut self, lr: f64) -> Self {
        self.lr = lr.clamp(1e-6, 1.0);
        self
    }

    /// Create config with custom iterations.
    pub fn with_iterations(mut self, n: usize) -> Self {
        self.iterations = n.max(1);
        self
    }

    /// Create config with custom energy entropy weight.
    pub fn with_energy_entropy_weight(mut self, w: f64) -> Self {
        self.energy_entropy_weight = w.max(0.0);
        self
    }

    /// Create config with custom symbiosis weight.
    pub fn with_symbiosis_weight(mut self, w: f64) -> Self {
        self.symbiosis_weight = w.max(0.0);
        self
    }

    /// Create config with custom convergence tolerance.
    pub fn with_convergence_tolerance(mut self, tol: f64) -> Self {
        self.convergence_tolerance = tol.max(1e-12);
        self
    }

    /// Create config with custom baseline VFE.
    pub fn with_baseline_vfe(mut self, vfe: f64) -> Self {
        self.baseline_vfe = vfe.max(1e-12);
        self
    }

    /// Create config with custom seed.
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

/// Result of planetary free energy computation.
#[derive(Debug, Clone)]
pub struct PlanetaryFreeEnergyResult {
    /// Total planetary free energy.
    pub total_f_planet: f64,
    /// Per-node VFE contributions.
    pub node_vfe_contributions: Vec<f64>,
    /// Energy distribution entropy.
    pub energy_entropy: f64,
    /// Symbiosis bonus.
    pub symbiosis_bonus: f64,
    /// Influence shares used in computation.
    pub influence_shares: Vec<f64>,
    /// Convergence indicator.
    pub converged: bool,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Free energy trajectory over iterations.
    pub f_trajectory: Vec<f64>,
}

impl PlanetaryFreeEnergyResult {
    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "PlanetaryFreeEnergy {{ f_planet={:.6}, entropy={:.4}, symbiosis={:.4}, converged={}, iterations={} }}",
            self.total_f_planet, self.energy_entropy, self.symbiosis_bonus,
            self.converged, self.iterations
        )
    }
}

/// Result of thermodynamic resilience evaluation.
#[derive(Debug, Clone)]
pub struct ResilienceResult {
    /// Thermodynamic resilience score in [0, 1].
    pub resilience_score: f64,
    /// Current planetary free energy.
    pub current_f_planet: f64,
    /// Baseline free energy for comparison.
    pub baseline_f_planet: f64,
    /// Resilience category.
    pub category: String,
    /// Perturbation resistance (higher = more resistant).
    pub perturbation_resistance: f64,
    /// Recovery time estimate (in simulation steps).
    pub recovery_time_estimate: f64,
}

impl ResilienceResult {
    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "Resilience {{ score={:.4}, category={}, perturbation_resist={:.4}, recovery_time={:.1} }}",
            self.resilience_score, self.category, self.perturbation_resistance,
            self.recovery_time_estimate
        )
    }
}

/// Result of active inference planetary step.
#[derive(Debug, Clone)]
pub struct ActiveInferenceResult {
    /// Updated belief states for all nodes.
    pub updated_beliefs: Vec<Vec<f64>>,
    /// Final planetary free energy.
    pub final_f_planet: f64,
    /// Initial planetary free energy.
    pub initial_f_planet: f64,
    /// Free energy reduction achieved.
    pub f_reduction: f64,
    /// Convergence indicator.
    pub converged: bool,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Free energy trajectory.
    pub f_trajectory: Vec<f64>,
    /// Per-node gradient norms.
    pub gradient_norms: Vec<f64>,
}

impl ActiveInferenceResult {
    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "ActiveInference {{ f_initial={:.6}, f_final={:.6}, reduction={:.6}, converged={}, iterations={} }}",
            self.initial_f_planet, self.final_f_planet, self.f_reduction,
            self.converged, self.iterations
        )
    }
}

// ---------------------------------------------------------------------------
// Core Thermodynamic Functions
// ---------------------------------------------------------------------------

/// LCG random number generator for reproducibility.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

/// Generate uniform random in [0, 1).
fn random_uniform(state: &mut u64) -> f64 {
    let raw = lcg_next(state);
    let masked = (raw >> 11) & ((1u64 << 53) - 1);
    masked as f64 / (1u64 << 53) as f64
}

/// Compute Shannon entropy of a distribution.
///
/// H(p) = -Σ p_i · log(p_i)
pub fn shannon_entropy(dist: &[f64]) -> f64 {
    dist.iter()
        .map(|&p| {
            let p = p.max(1e-12).min(1.0 - 1e-12);
            -p * p.ln()
        })
        .sum()
}

/// Compute KL divergence between two distributions.
///
/// KL(p || q) = Σ p_i · log(p_i / q_i)
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

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(&ai, &bi)| ai * bi).sum();
    let norm_a: f64 = a.iter().map(|&ai| ai * ai).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|&bi| bi * bi).sum::<f64>().sqrt();
    let denom = norm_a * norm_b;
    if denom < 1e-12 {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0)
    }
}

/// Compute the Variational Free Energy (VFE) for a single node.
///
/// VFE_i = KL(q(φ_i) || p(φ_i | safe_prior)) - E_q[log p(obs | φ_i)]
pub fn compute_node_vfe(
    beliefs: &[f64],
    safe_prior: &[f64],
    observations: &[f64],
) -> f64 {
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

/// Compute the Planetary Free Energy.
///
/// F_planet = Σ_i x_i · VFE_i + λ · H(energy_dist) - γ · symbiosis_bonus
///
/// # Arguments
/// * `influence_shares` - Influence share x_i for each node.
/// * `node_vfes` - VFE for each node.
/// * `energy_distribution` - Energy distribution across nodes.
/// * `beliefs` - Belief distributions for symbiosis computation.
/// * `cooperation_matrix` - Cooperation scores between node pairs.
/// * `config` - Thermodynamic configuration.
///
/// # Returns
/// The total planetary free energy.
pub fn compute_planetary_free_energy(
    influence_shares: &[f64],
    node_vfes: &[f64],
    energy_distribution: &[f64],
    beliefs: &[Vec<f64>],
    cooperation_matrix: &[Vec<f64>],
    config: &ThermoConfig,
) -> f64 {
    let num_nodes = influence_shares.len().max(1);

    // Weighted VFE sum: Σ x_i · VFE_i
    let weighted_vfe: f64 = influence_shares
        .iter()
        .zip(node_vfes.iter())
        .map(|(&x, &vfe)| x * vfe)
        .sum();

    // Energy distribution entropy: H(energy_dist)
    let energy_entropy = shannon_entropy(energy_distribution);

    // Symbiosis bonus: Σ_i<j sim(belief_i, belief_j) · cooperation_ij
    let mut symbiosis_bonus = 0.0;
    for i in 0..num_nodes {
        for j in (i + 1)..num_nodes {
            let sim = if i < beliefs.len() && j < beliefs.len() {
                cosine_similarity(&beliefs[i], &beliefs[j])
            } else {
                0.0
            };
            let coop = if i < cooperation_matrix.len() && j < cooperation_matrix[i].len() {
                cooperation_matrix[i][j]
            } else {
                0.0
            };
            symbiosis_bonus += sim * coop;
        }
    }

    // F_planet = weighted_vfe + λ · energy_entropy - γ · symbiosis_bonus
    weighted_vfe
        + config.energy_entropy_weight * energy_entropy
        - config.symbiosis_weight * symbiosis_bonus
}

/// Compute the full Planetary Free Energy with detailed results.
///
/// # Returns
/// Detailed result with per-node contributions, entropy, symbiosis, and trajectory.
pub fn compute_planetary_free_energy_detailed(
    influence_shares: &[f64],
    node_vfes: &[f64],
    energy_distribution: &[f64],
    beliefs: &[Vec<f64>],
    cooperation_matrix: &[Vec<f64>],
    config: &ThermoConfig,
) -> PlanetaryFreeEnergyResult {
    let num_nodes = influence_shares.len().max(1);

    // Per-node VFE contributions
    let node_vfe_contributions: Vec<f64> = influence_shares
        .iter()
        .zip(node_vfes.iter())
        .map(|(&x, &vfe)| x * vfe)
        .collect();

    // Energy distribution entropy
    let energy_entropy = shannon_entropy(energy_distribution);

    // Symbiosis bonus
    let mut symbiosis_bonus = 0.0;
    for i in 0..num_nodes {
        for j in (i + 1)..num_nodes {
            let sim = if i < beliefs.len() && j < beliefs.len() {
                cosine_similarity(&beliefs[i], &beliefs[j])
            } else {
                0.0
            };
            let coop = if i < cooperation_matrix.len() && j < cooperation_matrix[i].len() {
                cooperation_matrix[i][j]
            } else {
                0.0
            };
            symbiosis_bonus += sim * coop;
        }
    }

    // Total F_planet
    let total_f_planet = node_vfe_contributions.iter().sum::<f64>()
        + config.energy_entropy_weight * energy_entropy
        - config.symbiosis_weight * symbiosis_bonus;

    PlanetaryFreeEnergyResult {
        total_f_planet,
        node_vfe_contributions,
        energy_entropy,
        symbiosis_bonus,
        influence_shares: influence_shares.to_vec(),
        converged: true,
        iterations: 1,
        f_trajectory: vec![total_f_planet],
    }
}

/// Active Inference Planetary Step — Gradient flow optimization.
///
/// φ(t+1) = φ(t) - lr · ∇_φ F_planet(φ)
///
/// Optimizes belief states across the planetary mesh to minimize free energy.
///
/// # Arguments
/// * `current_beliefs` - Current belief distribution for each node.
/// * `safe_prior` - Collective safe prior.
/// * `observations` - Current observations.
/// * `influence_shares` - Influence share for each node.
/// * `energy_distribution` - Energy distribution across nodes.
/// * `cooperation_matrix` - Cooperation scores between node pairs.
/// * `config` - Thermodynamic configuration.
///
/// # Returns
/// Detailed result with updated beliefs, free energy trajectory, and convergence info.
pub fn active_inference_planetary_step(
    current_beliefs: &[Vec<f64>],
    safe_prior: &[f64],
    observations: &[f64],
    influence_shares: &[f64],
    energy_distribution: &[f64],
    cooperation_matrix: &[Vec<f64>],
    config: &ThermoConfig,
) -> ActiveInferenceResult {
    let num_nodes = current_beliefs.len().max(1);
    let dim = safe_prior.len().max(1);
    let mut state = config.seed;

    // Initialize cooperation matrix if empty
    let coop = if cooperation_matrix.is_empty() {
        vec![vec![0.0; num_nodes]; num_nodes]
    } else {
        cooperation_matrix.to_vec()
    };

    // Compute initial VFEs
    let mut node_vfes: Vec<f64> = current_beliefs
        .iter()
        .map(|b| compute_node_vfe(b, safe_prior, observations))
        .collect();

    // Initial F_planet
    let initial_f_planet = compute_planetary_free_energy(
        influence_shares,
        &node_vfes,
        energy_distribution,
        current_beliefs,
        &coop,
        config,
    );

    let mut beliefs: Vec<Vec<f64>> = current_beliefs.to_vec();
    let mut f_trajectory = vec![initial_f_planet];
    let mut gradient_norms: Vec<f64> = vec![0.0; num_nodes];
    let mut converged = false;

    for iteration in 0..config.iterations {
        let _ = iteration; // Track iteration

        // Compute numerical gradients for each node
        let eps = 1e-5;
        let mut total_gradient_norm: f64 = 0.0;

        for node_idx in 0..num_nodes {
            if node_idx >= beliefs.len() {
                break;
            }

            let mut node_grad: Vec<f64> = vec![0.0; dim];

            for d in 0..dim {
                let original = beliefs[node_idx][d];

                // Forward perturbation
                let mut beliefs_plus = beliefs.to_vec();
                beliefs_plus[node_idx][d] = (original + eps).clamp(1e-12, 1.0 - 1e-12);
                // Renormalize
                let sum_plus: f64 = beliefs_plus[node_idx].iter().sum();
                if sum_plus > 1e-12 {
                    for b in beliefs_plus[node_idx].iter_mut() {
                        *b /= sum_plus;
                    }
                }
                let vfes_plus: Vec<f64> = beliefs_plus
                    .iter()
                    .map(|b| compute_node_vfe(b, safe_prior, observations))
                    .collect();
                let f_plus = compute_planetary_free_energy(
                    influence_shares,
                    &vfes_plus,
                    energy_distribution,
                    &beliefs_plus,
                    &coop,
                    config,
                );

                // Backward perturbation
                let mut beliefs_minus = beliefs.to_vec();
                beliefs_minus[node_idx][d] = (original - eps).clamp(1e-12, 1.0 - 1e-12);
                let sum_minus: f64 = beliefs_minus[node_idx].iter().sum();
                if sum_minus > 1e-12 {
                    for b in beliefs_minus[node_idx].iter_mut() {
                        *b /= sum_minus;
                    }
                }
                let vfes_minus: Vec<f64> = beliefs_minus
                    .iter()
                    .map(|b| compute_node_vfe(b, safe_prior, observations))
                    .collect();
                let f_minus = compute_planetary_free_energy(
                    influence_shares,
                    &vfes_minus,
                    energy_distribution,
                    &beliefs_minus,
                    &coop,
                    config,
                );

                node_grad[d] = (f_plus - f_minus) / (2.0 * eps);
            }

            // Compute gradient norm for this node
            let norm: f64 = node_grad.iter().map(|g| g * g).sum::<f64>().sqrt();
            gradient_norms[node_idx] = norm;
            total_gradient_norm += norm;

            // Gradient descent update with influence weighting
            let lr = config.lr * influence_shares.get(node_idx).copied().unwrap_or(1.0 / num_nodes as f64);
            for d in 0..dim {
                beliefs[node_idx][d] -= lr * node_grad[d];
            }

            // Project back to simplex (softmax projection)
            let max_val = beliefs[node_idx].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let exps: Vec<f64> = beliefs[node_idx]
                .iter()
                .map(|&v| (v - max_val).exp())
                .collect();
            let sum_exp: f64 = exps.iter().sum();
            if sum_exp > 1e-12 {
                for d in 0..dim {
                    beliefs[node_idx][d] = exps[d] / sum_exp;
                }
            }
        }

        // Update VFEs
        node_vfes = beliefs
            .iter()
            .map(|b| compute_node_vfe(b, safe_prior, observations))
            .collect();

        // Compute current F_planet
        let current_f = compute_planetary_free_energy(
            influence_shares,
            &node_vfes,
            energy_distribution,
            &beliefs,
            &coop,
            config,
        );
        f_trajectory.push(current_f);

        // Check convergence
        if f_trajectory.len() >= 2 {
            let delta = (f_trajectory[f_trajectory.len() - 1] - f_trajectory[f_trajectory.len() - 2]).abs();
            if delta < config.convergence_tolerance {
                converged = true;
                break;
            }
        }

        // Add small noise for exploration
        for node_idx in 0..num_nodes {
            if node_idx < beliefs.len() {
                for d in 0..dim {
                    beliefs[node_idx][d] += 1e-8 * (random_uniform(&mut state) - 0.5);
                }
                // Renormalize
                let sum: f64 = beliefs[node_idx].iter().sum();
                if sum > 1e-12 {
                    for b in beliefs[node_idx].iter_mut() {
                        *b /= sum;
                    }
                }
            }
        }
    }

    let final_f_planet = *f_trajectory.last().unwrap_or(&initial_f_planet);
    let f_reduction = initial_f_planet - final_f_planet;

    ActiveInferenceResult {
        updated_beliefs: beliefs,
        final_f_planet,
        initial_f_planet,
        f_reduction,
        converged,
        iterations: f_trajectory.len().saturating_sub(1),
        f_trajectory,
        gradient_norms,
    }
}

/// Compute the Thermodynamic Resilience Score.
///
/// R_thermo = 1 / (1 + F_planet / F_baseline)
///
/// Higher resilience = lower free energy relative to baseline.
///
/// # Arguments
/// * `current_f_planet` - Current planetary free energy.
/// * `baseline_f_planet` - Baseline free energy for normalization.
/// * `perturbation_magnitude` - Magnitude of adversarial perturbation tested.
///
/// # Returns
/// Detailed resilience result with score, category, and estimates.
pub fn thermodynamic_resilience_score(
    current_f_planet: f64,
    baseline_f_planet: f64,
    perturbation_magnitude: f64,
) -> ResilienceResult {
    let baseline = baseline_f_planet.max(1e-12);
    let ratio = current_f_planet / baseline;
    let resilience_score = (1.0 / (1.0 + ratio)).clamp(0.0, 1.0);

    // Perturbation resistance: how much perturbation can be absorbed
    let perturbation_resistance = (1.0 / (1.0 + perturbation_magnitude * ratio)).clamp(0.0, 1.0);

    // Recovery time estimate (in simulation steps)
    let recovery_time_estimate = if resilience_score > 0.5 {
        perturbation_magnitude * 10.0
    } else {
        perturbation_magnitude * 50.0
    };

    // Categorize resilience
    let category = if resilience_score >= 0.9 {
        "NOOSPHERIC_IMMUNE".to_string()
    } else if resilience_score >= 0.7 {
        "SYMBIOTIC_STABLE".to_string()
    } else if resilience_score >= 0.5 {
        "TRANSITIONING".to_string()
    } else if resilience_score >= 0.3 {
        "VULNERABLE".to_string()
    } else {
        "CRITICAL".to_string()
    };

    ResilienceResult {
        resilience_score,
        current_f_planet,
        baseline_f_planet,
        category,
        perturbation_resistance,
        recovery_time_estimate,
    }
}

/// Run the full thermodynamic self-organization loop.
///
/// Iteratively applies active inference steps while monitoring resilience.
///
/// # Arguments
/// * `initial_beliefs` - Initial belief distribution for each node.
/// * `safe_prior` - Collective safe prior.
/// * `observations` - Current observations.
/// * `influence_shares` - Influence share for each node.
/// * `energy_distribution` - Energy distribution across nodes.
/// * `cooperation_matrix` - Cooperation scores between node pairs.
/// * `config` - Thermodynamic configuration.
///
/// # Returns
/// Tuple of (active_inference_result, resilience_result).
pub fn run_thermodynamic_self_organization(
    initial_beliefs: &[Vec<f64>],
    safe_prior: &[f64],
    observations: &[f64],
    influence_shares: &[f64],
    energy_distribution: &[f64],
    cooperation_matrix: &[Vec<f64>],
    config: &ThermoConfig,
) -> (ActiveInferenceResult, ResilienceResult) {
    // Run active inference
    let ai_result = active_inference_planetary_step(
        initial_beliefs,
        safe_prior,
        observations,
        influence_shares,
        energy_distribution,
        cooperation_matrix,
        config,
    );

    // Compute resilience
    let resilience = thermodynamic_resilience_score(
        ai_result.final_f_planet,
        config.baseline_vfe,
        0.1, // Default perturbation magnitude
    );

    (ai_result, resilience)
}

/// Simulate the civilizational transition from economy to symbiosis.
///
/// Models the adoption curve of Noospheric nodes over time, tracking:
/// - Economic incentive nodes (declining)
/// - Symbiotic nodes (growing)
/// - Transition tipping point
///
/// # Arguments
/// * `initial_economic_ratio` - Initial ratio of economy-driven nodes.
/// * `symbiosis_attractor` - Strength of symbiotic attraction (0-1).
/// * `steps` - Number of simulation steps.
/// * `seed` - Random seed.
///
/// # Returns
/// Vector of (step, economic_ratio, symbiotic_ratio, tipping_point_reached).
pub fn simulate_civilizational_transition(
    initial_economic_ratio: f64,
    symbiosis_attractor: f64,
    steps: usize,
    seed: u64,
) -> Vec<(usize, f64, f64, bool)> {
    let mut state = seed;
    let mut economic_ratio = initial_economic_ratio.clamp(0.0, 1.0);
    let mut trajectory = Vec::with_capacity(steps);

    for step in 0..steps {
        let symbiotic_ratio = 1.0 - economic_ratio;

        // Transition dynamics:
        // Economic nodes convert to symbiotic based on:
        // 1. Symbiosis attractor strength
        // 2. Network effect (more symbiotic = faster conversion)
        // 3. Stochastic noise
        let network_effect = symbiotic_ratio.powi(2);
        let conversion_rate = symbiosis_attractor * network_effect * 0.1;
        let noise = (random_uniform(&mut state) - 0.5) * 0.02;
        let delta = conversion_rate + noise;

        economic_ratio = (economic_ratio - delta).clamp(0.0, 1.0);

        // Tipping point: when symbiotic > economic
        let tipping_point_reached = symbiotic_ratio > economic_ratio;

        trajectory.push((
            step,
            economic_ratio,
            1.0 - economic_ratio,
            tipping_point_reached,
        ));
    }

    trajectory
}

/// Compute the noospheric aggregation via colimit.
///
/// Aggregates multiple manifold states into a unified noospheric state
/// using category-theoretic colimit construction.
///
/// # Arguments
/// * `manifold_states` - Vector of manifold activation states.
/// * `transition_weights` - Weights for transitions between manifolds.
///
/// # Returns
/// Aggregated noospheric state vector.
pub fn colimit_noospheric_aggregation(
    manifold_states: &[Vec<f64>],
    transition_weights: &[Vec<f64>],
) -> Vec<f64> {
    if manifold_states.is_empty() {
        return vec![];
    }

    let dim = manifold_states[0].len();
    let num_manifolds = manifold_states.len();

    // Weighted aggregation with transition smoothing
    let mut aggregated = vec![0.0; dim];
    let mut total_weight = 0.0;

    for (i, state) in manifold_states.iter().enumerate() {
        // Compute effective weight from transitions
        let mut eff_weight = 1.0 / num_manifolds as f64;
        if i < transition_weights.len() {
            for &w in &transition_weights[i] {
                eff_weight += w * 0.1;
            }
        }

        for (d, &val) in state.iter().enumerate() {
            if d < dim {
                aggregated[d] += eff_weight * val;
            }
        }
        total_weight += eff_weight;
    }

    // Normalize
    if total_weight > 1e-12 {
        for v in aggregated.iter_mut() {
            *v /= total_weight;
        }
    }

    aggregated
}

/// Compute the functorial safety margin between two manifold compositions.
///
/// Measures how much safety is preserved through functorial composition.
///
/// # Arguments
/// * `source_safety` - Safety margin of source manifold.
/// * `target_safety` - Safety margin of target manifold.
/// * `composition_fidelity` - Fidelity of the functorial composition (0-1).
///
/// # Returns
/// Composite safety margin.
pub fn functorial_safety_margin(
    source_safety: f64,
    target_safety: f64,
    composition_fidelity: f64,
) -> f64 {
    let fidelity = composition_fidelity.clamp(0.0, 1.0);
    // Safety is preserved proportionally to composition fidelity
    // and bounded by the minimum of source and target
    let min_safety = source_safety.min(target_safety);
    let blended = (source_safety + target_safety) / 2.0;
    (min_safety * fidelity + blended * (1.0 - fidelity)).clamp(0.0, 1.0)
}

/// Run the S131 Noospheric Closure pipeline.
///
/// Full integration: thermodynamic VFE → active inference → resilience → governance.
///
/// # Arguments
/// * `beliefs` - Initial belief distributions.
/// * `safe_prior` - Safe prior.
/// * `observations` - Observations.
/// * `influence_shares` - Node influence shares.
/// * `energy_distribution` - Energy distribution.
/// * `cooperation_matrix` - Cooperation matrix.
/// * `config` - Thermodynamic configuration.
///
/// # Returns
/// Tuple of (free_energy_result, active_inference_result, resilience_result).
pub fn s131_noosfera_closure(
    beliefs: &[Vec<f64>],
    safe_prior: &[f64],
    observations: &[f64],
    influence_shares: &[f64],
    energy_distribution: &[f64],
    cooperation_matrix: &[Vec<f64>],
    config: &ThermoConfig,
) -> (
    PlanetaryFreeEnergyResult,
    ActiveInferenceResult,
    ResilienceResult,
) {
    // Compute initial VFEs
    let node_vfes: Vec<f64> = beliefs
        .iter()
        .map(|b| compute_node_vfe(b, safe_prior, observations))
        .collect();

    // Compute detailed free energy
    let fe_result = compute_planetary_free_energy_detailed(
        influence_shares,
        &node_vfes,
        energy_distribution,
        beliefs,
        cooperation_matrix,
        config,
    );

    // Run active inference
    let ai_result = active_inference_planetary_step(
        beliefs,
        safe_prior,
        observations,
        influence_shares,
        energy_distribution,
        cooperation_matrix,
        config,
    );

    // Compute resilience
    let resilience = thermodynamic_resilience_score(
        ai_result.final_f_planet,
        config.baseline_vfe,
        0.1,
    );

    (fe_result, ai_result, resilience)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // === Shannon Entropy Tests ===

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let h = shannon_entropy(&dist);
        assert!((h - std::f64::consts::LN_2 * 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        let dist = vec![1.0, 0.0, 0.0];
        let h = shannon_entropy(&dist);
        assert!(h.abs() < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_binary() {
        let dist = vec![0.5, 0.5];
        let h = shannon_entropy(&dist);
        assert!((h - std::f64::consts::LN_2).abs() < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_empty() {
        let dist: Vec<f64> = vec![];
        let h = shannon_entropy(&dist);
        assert!((h - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_positive() {
        let dist = vec![0.1, 0.2, 0.3, 0.4];
        let h = shannon_entropy(&dist);
        assert!(h > 0.0);
    }

    // === KL Divergence Tests ===

    #[test]
    fn test_kl_divergence_identical() {
        let p = vec![0.5, 0.5];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl.abs() < 1e-10);
    }

    #[test]
    fn test_kl_divergence_positive() {
        let p = vec![0.9, 0.1];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0);
    }

    #[test]
    fn test_kl_divergence_asymmetric() {
        let p = vec![0.9, 0.1];
        let q = vec![0.5, 0.5];
        let kl_pq = kl_divergence(&p, &q);
        let kl_qp = kl_divergence(&q, &p);
        assert!((kl_pq - kl_qp).abs() > 1e-6);
    }

    // === Cosine Similarity Tests ===

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f64> = vec![];
        let b: Vec<f64> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-10);
    }

    // === Node VFE Tests ===

    #[test]
    fn test_compute_node_vfe_basic() {
        let beliefs = vec![0.5, 0.5];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let vfe = compute_node_vfe(&beliefs, &prior, &obs);
        assert!(vfe.is_finite());
    }

    #[test]
    fn test_compute_node_vfe_matches_prior() {
        let beliefs = vec![0.5, 0.5];
        let prior = vec![0.5, 0.5];
        let obs = vec![1.0, 1.0];
        let vfe = compute_node_vfe(&beliefs, &prior, &obs);
        // KL should be 0, log-likelihood should be 0
        assert!(vfe.abs() < 1e-10);
    }

    #[test]
    fn test_compute_node_vfe_diverges_from_prior() {
        let beliefs = vec![0.9, 0.1];
        let prior = vec![0.5, 0.5];
        let obs = vec![1.0, 1.0];
        let vfe = compute_node_vfe(&beliefs, &prior, &obs);
        assert!(vfe > 0.0);
    }

    // === Planetary Free Energy Tests ===

    #[test]
    fn test_compute_planetary_free_energy_basic() {
        let shares = vec![0.5, 0.5];
        let vfes = vec![1.0, 2.0];
        let energy = vec![0.5, 0.5];
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let coop = vec![vec![0.0, 0.5], vec![0.5, 0.0]];
        let config = ThermoConfig::default();
        let f = compute_planetary_free_energy(&shares, &vfes, &energy, &beliefs, &coop, &config);
        assert!(f.is_finite());
    }

    #[test]
    fn test_compute_planetary_free_energy_zero_vfe() {
        let shares = vec![0.5, 0.5];
        let vfes = vec![0.0, 0.0];
        let energy = vec![0.5, 0.5];
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let coop = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let config = ThermoConfig::default();
        let f = compute_planetary_free_energy(&shares, &vfes, &energy, &beliefs, &coop, &config);
        // Should be positive from entropy term
        assert!(f >= 0.0);
    }

    #[test]
    fn test_compute_planetary_free_energy_high_symbiosis() {
        let shares = vec![0.5, 0.5];
        let vfes = vec![1.0, 1.0];
        let energy = vec![0.5, 0.5];
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let coop = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let config = ThermoConfig::default();
        let f = compute_planetary_free_energy(&shares, &vfes, &energy, &beliefs, &coop, &config);
        // High symbiosis should reduce F
        assert!(f < 2.0);
    }

    #[test]
    fn test_compute_planetary_free_energy_empty() {
        let shares: Vec<f64> = vec![];
        let vfes: Vec<f64> = vec![];
        let energy: Vec<f64> = vec![];
        let beliefs: Vec<Vec<f64>> = vec![];
        let coop: Vec<Vec<f64>> = vec![];
        let config = ThermoConfig::default();
        let f = compute_planetary_free_energy(&shares, &vfes, &energy, &beliefs, &coop, &config);
        assert!(f.is_finite());
    }

    #[test]
    fn test_compute_planetary_free_energy_detailed() {
        let shares = vec![0.5, 0.5];
        let vfes = vec![1.0, 2.0];
        let energy = vec![0.5, 0.5];
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let coop = vec![vec![0.0, 0.5], vec![0.5, 0.0]];
        let config = ThermoConfig::default();
        let result = compute_planetary_free_energy_detailed(
            &shares, &vfes, &energy, &beliefs, &coop, &config,
        );
        assert!(result.total_f_planet.is_finite());
        assert_eq!(result.node_vfe_contributions.len(), 2);
        assert!(result.energy_entropy > 0.0);
        assert!(result.f_trajectory.len() == 1);
    }

    // === Active Inference Tests ===

    #[test]
    fn test_active_inference_planetary_step_basic() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![0.5, 0.5];
        let energy = vec![0.5, 0.5];
        let coop = vec![vec![0.0, 0.5], vec![0.5, 0.0]];
        let config = ThermoConfig::fast();
        let result = active_inference_planetary_step(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert!(result.final_f_planet.is_finite());
        assert!(result.iterations > 0);
        assert_eq!(result.updated_beliefs.len(), 2);
    }

    #[test]
    fn test_active_inference_planetary_step_convergence() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![1.0];
        let energy = vec![1.0];
        let coop = vec![vec![0.0]];
        let config = ThermoConfig::fast();
        let result = active_inference_planetary_step(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        // Should converge quickly when beliefs match prior
        assert!(result.converged || result.f_reduction >= 0.0);
    }

    #[test]
    fn test_active_inference_planetary_step_reduction() {
        let beliefs = vec![vec![0.9, 0.1]];
        let prior = vec![0.5, 0.5];
        let obs = vec![1.0, 1.0];
        let shares = vec![1.0];
        let energy = vec![1.0];
        let coop = vec![vec![0.0]];
        let config = ThermoConfig::fast();
        let result = active_inference_planetary_step(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        // VFE should decrease
        assert!(result.f_reduction >= 0.0);
    }

    #[test]
    fn test_active_inference_gradient_norms() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![0.5, 0.5];
        let energy = vec![0.5, 0.5];
        let coop = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let config = ThermoConfig::fast();
        let result = active_inference_planetary_step(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert_eq!(result.gradient_norms.len(), 2);
        for &norm in &result.gradient_norms {
            assert!(norm >= 0.0);
        }
    }

    #[test]
    fn test_active_inference_empty_coop() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![1.0];
        let energy = vec![1.0];
        let coop: Vec<Vec<f64>> = vec![];
        let config = ThermoConfig::fast();
        let result = active_inference_planetary_step(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert!(result.final_f_planet.is_finite());
    }

    // === Resilience Tests ===

    #[test]
    fn test_thermodynamic_resilience_perfect() {
        let result = thermodynamic_resilience_score(0.0, 1.0, 0.0);
        assert!((result.resilience_score - 1.0).abs() < 1e-10);
        assert_eq!(result.category, "NOOSPHERIC_IMMUNE");
    }

    #[test]
    fn test_thermodynamic_resilience_critical() {
        let result = thermodynamic_resilience_score(10.0, 1.0, 1.0);
        assert!(result.resilience_score < 0.3);
        assert_eq!(result.category, "CRITICAL");
    }

    #[test]
    fn test_thermodynamic_resilience_symbiotic() {
        let result = thermodynamic_resilience_score(0.3, 1.0, 0.1);
        assert!(result.resilience_score >= 0.7);
        assert!(result.category == "SYMBIOTIC_STABLE" || result.category == "NOOSPHERIC_IMMUNE");
    }

    #[test]
    fn test_thermodynamic_resilience_transitioning() {
        let result = thermodynamic_resilience_score(0.5, 1.0, 0.1);
        assert!(result.resilience_score >= 0.5 && result.resilience_score < 0.7);
        assert_eq!(result.category, "TRANSITIONING");
    }

    #[test]
    fn test_thermodynamic_resilience_perturbation() {
        let r_low = thermodynamic_resilience_score(0.5, 1.0, 0.01);
        let r_high = thermodynamic_resilience_score(0.5, 1.0, 1.0);
        assert!(r_low.perturbation_resistance > r_high.perturbation_resistance);
    }

    #[test]
    fn test_thermodynamic_resilience_recovery_time() {
        let r_high = thermodynamic_resilience_score(0.1, 1.0, 0.1);
        let r_low = thermodynamic_resilience_score(2.0, 1.0, 0.1);
        assert!(r_high.recovery_time_estimate < r_low.recovery_time_estimate);
    }

    // === Self-Organization Tests ===

    #[test]
    fn test_run_thermodynamic_self_organization() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![0.5, 0.5];
        let energy = vec![0.5, 0.5];
        let coop = vec![vec![0.0, 0.5], vec![0.5, 0.0]];
        let config = ThermoConfig::fast();
        let (ai, resilience) = run_thermodynamic_self_organization(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert!(ai.final_f_planet.is_finite());
        assert!(resilience.resilience_score >= 0.0 && resilience.resilience_score <= 1.0);
    }

    // === Civilizational Transition Tests ===

    #[test]
    fn test_simulate_civilizational_transition_basic() {
        let trajectory = simulate_civilizational_transition(0.8, 0.5, 100, 42);
        assert_eq!(trajectory.len(), 100);
        assert!(trajectory[0].1 > 0.5); // Start with high economic ratio
    }

    #[test]
    fn test_simulate_civilizational_transition_tipping_point() {
        let trajectory = simulate_civilizational_transition(0.6, 0.8, 200, 42);
        let tipping_reached = trajectory.iter().any(|t| t.3);
        assert!(tipping_reached);
    }

    #[test]
    fn test_simulate_civilizational_transition_declining_economic() {
        let trajectory = simulate_civilizational_transition(0.9, 0.7, 500, 42);
        let first_econ = trajectory.first().unwrap().1;
        let last_econ = trajectory.last().unwrap().1;
        assert!(last_econ < first_econ);
    }

    #[test]
    fn test_simulate_civilizational_transition_strong_attractor() {
        let trajectory = simulate_civilizational_transition(0.5, 1.0, 100, 42);
        let last = trajectory.last().unwrap();
        assert!(last.2 > last.1); // Symbiotic > economic
    }

    #[test]
    fn test_simulate_civilizational_transition_weak_attractor() {
        let trajectory = simulate_civilizational_transition(0.9, 0.1, 100, 42);
        let last = trajectory.last().unwrap();
        assert!(last.1 > last.2); // Economic still dominant
    }

    #[test]
    fn test_simulate_civilizational_transition_deterministic() {
        let t1 = simulate_civilizational_transition(0.8, 0.5, 50, 42);
        let t2 = simulate_civilizational_transition(0.8, 0.5, 50, 42);
        assert_eq!(t1.len(), t2.len());
        for (a, b) in t1.iter().zip(t2.iter()) {
            assert!((a.1 - b.1).abs() < 1e-10);
            assert!((a.2 - b.2).abs() < 1e-10);
        }
    }

    // === Colimit Aggregation Tests ===

    #[test]
    fn test_colimit_noospheric_aggregation_basic() {
        let states = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let weights = vec![vec![0.5], vec![0.5]];
        let result = colimit_noospheric_aggregation(&states, &weights);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_colimit_noospheric_aggregation_empty() {
        let states: Vec<Vec<f64>> = vec![];
        let weights: Vec<Vec<f64>> = vec![];
        let result = colimit_noospheric_aggregation(&states, &weights);
        assert!(result.is_empty());
    }

    #[test]
    fn test_colimit_noospheric_aggregation_single() {
        let states = vec![vec![1.0, 2.0, 3.0]];
        let weights: Vec<Vec<f64>> = vec![];
        let result = colimit_noospheric_aggregation(&states, &weights);
        assert_eq!(result.len(), 3);
    }

    // === Functorial Safety Tests ===

    #[test]
    fn test_functorial_safety_margin_perfect() {
        let s = functorial_safety_margin(1.0, 1.0, 1.0);
        assert!((s - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_functorial_safety_margin_zero_fidelity() {
        let s = functorial_safety_margin(0.8, 0.6, 0.0);
        assert!((s - 0.7).abs() < 1e-10); // Average when fidelity=0
    }

    #[test]
    fn test_functorial_safety_margin_bounded() {
        let s = functorial_safety_margin(0.5, 0.3, 0.5);
        assert!(s >= 0.0 && s <= 1.0);
    }

    #[test]
    fn test_functorial_safety_margin_min_bound() {
        let s = functorial_safety_margin(0.3, 0.8, 1.0);
        assert!(s <= 0.3 + 1e-10); // Bounded by min
    }

    // === S131 Noospheric Closure Tests ===

    #[test]
    fn test_s131_noosfera_closure_basic() {
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![0.5, 0.5];
        let energy = vec![0.5, 0.5];
        let coop = vec![vec![0.0, 0.5], vec![0.5, 0.0]];
        let config = ThermoConfig::fast();
        let (fe, ai, resilience) = s131_noosfera_closure(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert!(fe.total_f_planet.is_finite());
        assert!(ai.final_f_planet.is_finite());
        assert!(resilience.resilience_score >= 0.0);
    }

    #[test]
    fn test_s131_noosfera_closure_large() {
        let dim = 5;
        let num_nodes = 10;
        let beliefs: Vec<Vec<f64>> = (0..num_nodes)
            .map(|_| vec![1.0 / dim as f64; dim])
            .collect();
        let prior = vec![1.0 / dim as f64; dim];
        let obs = vec![1.0; dim];
        let shares = vec![1.0 / num_nodes as f64; num_nodes];
        let energy = vec![1.0 / num_nodes as f64; num_nodes];
        let coop = vec![vec![0.0; num_nodes]; num_nodes];
        let config = ThermoConfig::fast();
        let (fe, ai, resilience) = s131_noosfera_closure(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert_eq!(fe.node_vfe_contributions.len(), num_nodes);
        assert_eq!(ai.updated_beliefs.len(), num_nodes);
        assert!(resilience.resilience_score >= 0.0);
    }

    #[test]
    fn test_s131_noosfera_closure_high_precision() {
        let beliefs = vec![vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![1.0];
        let energy = vec![1.0];
        let coop = vec![vec![0.0]];
        let config = ThermoConfig::high_precision();
        let (_fe, ai, _resilience) = s131_noosfera_closure(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );
        assert!(ai.iterations <= 200);
    }

    // === Config Tests ===

    #[test]
    fn test_thermo_config_default() {
        let config = ThermoConfig::default();
        assert_eq!(config.lr, 0.01);
        assert_eq!(config.iterations, 50);
        assert_eq!(config.seed, 42);
    }

    #[test]
    fn test_thermo_config_fast() {
        let config = ThermoConfig::fast();
        assert_eq!(config.iterations, 10);
        assert_eq!(config.lr, 0.1);
    }

    #[test]
    fn test_thermo_config_high_precision() {
        let config = ThermoConfig::high_precision();
        assert_eq!(config.iterations, 200);
        assert_eq!(config.lr, 0.001);
    }

    #[test]
    fn test_thermo_config_with_lr() {
        let config = ThermoConfig::default().with_lr(0.5);
        assert_eq!(config.lr, 0.5);
    }

    #[test]
    fn test_thermo_config_lr_clamped() {
        let config = ThermoConfig::default().with_lr(2.0);
        assert_eq!(config.lr, 1.0);
    }

    // === Summary Tests ===

    #[test]
    fn test_planetary_free_energy_result_summary() {
        let result = PlanetaryFreeEnergyResult {
            total_f_planet: 1.5,
            node_vfe_contributions: vec![0.5, 1.0],
            energy_entropy: 0.693,
            symbiosis_bonus: 0.3,
            influence_shares: vec![0.5, 0.5],
            converged: true,
            iterations: 10,
            f_trajectory: vec![2.0, 1.5],
        };
        let summary = result.summary();
        assert!(summary.contains("PlanetaryFreeEnergy"));
        assert!(summary.contains("f_planet=1.500000"));
        assert!(summary.contains("converged=true"));
    }

    #[test]
    fn test_resilience_result_summary() {
        let result = ResilienceResult {
            resilience_score: 0.85,
            current_f_planet: 0.3,
            baseline_f_planet: 1.0,
            category: "SYMBIOTIC_STABLE".to_string(),
            perturbation_resistance: 0.9,
            recovery_time_estimate: 15.0,
        };
        let summary = result.summary();
        assert!(summary.contains("Resilience"));
        assert!(summary.contains("SYMBIOTIC_STABLE"));
    }

    #[test]
    fn test_active_inference_result_summary() {
        let result = ActiveInferenceResult {
            updated_beliefs: vec![vec![0.5, 0.5]],
            final_f_planet: 0.5,
            initial_f_planet: 1.0,
            f_reduction: 0.5,
            converged: true,
            iterations: 10,
            f_trajectory: vec![1.0, 0.5],
            gradient_norms: vec![0.1],
        };
        let summary = result.summary();
        assert!(summary.contains("ActiveInference"));
        assert!(summary.contains("reduction=0.500000"));
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
            assert!(r >= 0.0);
            assert!(r < 1.0);
        }
    }

    // === Integration Tests ===

    #[test]
    fn test_full_thermodynamic_pipeline() {
        // Full integration: VFE → active inference → resilience
        let beliefs = vec![vec![0.5, 0.5], vec![0.6, 0.4], vec![0.4, 0.6]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![0.4, 0.3, 0.3];
        let energy = vec![0.4, 0.3, 0.3];
        let coop = vec![
            vec![0.0, 0.7, 0.5],
            vec![0.7, 0.0, 0.6],
            vec![0.5, 0.6, 0.0],
        ];
        let config = ThermoConfig::fast();

        let (fe, ai, resilience) = s131_noosfera_closure(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );

        assert!(fe.total_f_planet.is_finite());
        assert!(ai.f_reduction >= 0.0);
        assert!(resilience.resilience_score >= 0.0 && resilience.resilience_score <= 1.0);
        assert_eq!(fe.node_vfe_contributions.len(), 3);
        assert_eq!(ai.updated_beliefs.len(), 3);
    }

    #[test]
    fn test_thermodynamic_with_value_alignment() {
        // Integration with value alignment concepts
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let prior = vec![0.5, 0.5];
        let obs = vec![0.5, 0.5];
        let shares = vec![0.5, 0.5];
        let energy = vec![0.5, 0.5];
        let coop = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let config = ThermoConfig::fast();

        let (_fe, ai, resilience) = s131_noosfera_closure(
            &beliefs, &prior, &obs, &shares, &energy, &coop, &config,
        );

        // High cooperation should lead to better resilience
        assert!(resilience.resilience_score > 0.0);
        assert!(ai.final_f_planet.is_finite());
    }

    #[test]
    fn test_civilizational_transition_full_simulation() {
        // Simulate full civilizational transition
        let trajectory = simulate_civilizational_transition(0.95, 0.6, 1000, 42);

        // Check that economic ratio declines over time
        let first_econ = trajectory.first().unwrap().1;
        let last_econ = trajectory.last().unwrap().1;
        assert!(last_econ < first_econ);

        // Check that tipping point is eventually reached
        let tipping_steps: Vec<_> =
            trajectory.iter().filter(|t| t.3).collect();
        assert!(!tipping_steps.is_empty());

        // First tipping point should be before last
        let first_tipping = tipping_steps.first().unwrap();
        assert!(first_tipping.0 < 1000);
    }

    #[test]
    fn test_energy_entropy_affects_planetary_f() {
        let shares = vec![0.5, 0.5];
        let vfes = vec![1.0, 1.0];
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let coop = vec![vec![0.0, 0.0], vec![0.0, 0.0]];

        // Uniform energy distribution (high entropy)
        let energy_uniform = vec![0.5, 0.5];
        let config = ThermoConfig::default();
        let f_uniform = compute_planetary_free_energy(
            &shares, &vfes, &energy_uniform, &beliefs, &coop, &config,
        );

        // Skewed energy distribution (low entropy)
        let energy_skewed = vec![0.9, 0.1];
        let f_skewed = compute_planetary_free_energy(
            &shares, &vfes, &energy_skewed, &beliefs, &coop, &config,
        );

        // Higher entropy should increase F (due to positive λ)
        assert!(f_uniform > f_skewed);
    }

    #[test]
    fn test_symbiosis_bonus_reduces_f() {
        let shares = vec![0.5, 0.5];
        let vfes = vec![1.0, 1.0];
        let energy = vec![0.5, 0.5];
        let beliefs = vec![vec![0.5, 0.5], vec![0.5, 0.5]];

        // High cooperation
        let coop_high = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let config = ThermoConfig::default();
        let f_high = compute_planetary_free_energy(
            &shares, &vfes, &energy, &beliefs, &coop_high, &config,
        );

        // No cooperation
        let coop_none = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let f_none = compute_planetary_free_energy(
            &shares, &vfes, &energy, &beliefs, &coop_none, &config,
        );

        // Higher cooperation should reduce F
        assert!(f_high < f_none);
    }
}
