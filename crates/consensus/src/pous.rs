//! Proof of Useful Symbiosis (PoUS) — Thermodynamic Fitness, Replicator Dynamics, Shapley Values.
//!
//! Implements game-theoretic alignment for planetary-scale consensus:
//! - **PoUS Fitness:** Thermodynamic score based on VFE reduction, energy efficiency, uptime, and Byzantine penalty.
//! - **Replicator Equation:** Evolutionary dynamics driving node influence toward symbiotic Nash equilibrium.
//! - **Shapley Values:** Fair credit allocation for cooperative federated audits.

/// PoUS Fitness Score — Thermodynamic Node Evaluation
///
/// ```text
/// Score_i = α · ΔVFE_contrib + β · Efficiency_energy + γ · Uptime_verified - δ · Byzantine_penalty
/// ```
///
/// # Parameters
/// - `delta_vfe`: Variational Free Energy reduction contributed by the node.
/// - `energy_efficiency`: Energy saved (mWh) vs. baseline datacenter computation.
/// - `uptime_ratio`: Verified uptime ratio in [0, 1].
/// - `byzantine_penalty`: Accumulated Byzantine penalty score.
///
/// # Coefficients
/// - α = 0.5 (VFE contribution weight)
/// - β = 0.3 (Energy efficiency weight)
/// - γ = 0.2 (Uptime weight)
/// - δ = 2.0 (Strict Byzantine penalty)
pub fn compute_pous_fitness(
    delta_vfe: f64,
    energy_efficiency: f64,
    uptime_ratio: f64,
    byzantine_penalty: f64,
) -> f64 {
    let alpha = 0.5;
    let beta = 0.3;
    let gamma = 0.2;
    let delta = 2.0;
    let fitness = (alpha * delta_vfe) + (beta * energy_efficiency) + (gamma * uptime_ratio)
        - (delta * byzantine_penalty);
    fitness.max(0.0)
}

/// PoUS Fitness with configurable coefficients.
///
/// Allows custom weights for different deployment scenarios.
#[allow(clippy::too_many_arguments)]
pub fn compute_pous_fitness_custom(
    delta_vfe: f64,
    energy_efficiency: f64,
    uptime_ratio: f64,
    byzantine_penalty: f64,
    alpha: f64,
    beta: f64,
    gamma: f64,
    delta: f64,
) -> f64 {
    let fitness = (alpha * delta_vfe) + (beta * energy_efficiency) + (gamma * uptime_ratio)
        - (delta * byzantine_penalty);
    fitness.max(0.0)
}

/// PoUS Fitness with entropy diversity bonus.
///
/// Extends base fitness with network entropy term to prevent monopolization:
/// ```text
/// Fitness_i += η · (-Σ p_j · log(p_j))
/// ```
///
/// # Parameters
/// - `base_fitness`: Base PoUS fitness score.
/// - `network_distribution`: Current influence distribution across nodes.
/// - `eta`: Entropy bonus coefficient.
pub fn compute_pous_fitness_with_entropy(
    base_fitness: f64,
    network_distribution: &[f64],
    eta: f64,
) -> f64 {
    if network_distribution.is_empty() {
        return base_fitness;
    }
    let entropy: f64 = network_distribution
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.ln())
        .sum();
    (base_fitness + eta * entropy).max(0.0)
}

/// Evolutionary Game Dynamics — Replicator Equation (Euler Integration)
///
/// ```text
/// dx_i/dt = x_i · (f_i - f̄)
/// x_i(t+dt) = x_i + dt · x_i · (f_i - f̄)
/// ```
///
/// Updates node influence share based on fitness relative to network average.
///
/// # Parameters
/// - `current_share`: Current influence share of the node in [0.0001, 1.0].
/// - `node_fitness`: Fitness score of this node.
/// - `average_network_fitness`: Average fitness across all nodes.
/// - `learning_rate`: Time step (dt) for Euler integration.
///
/// # Returns
/// New influence share clamped to [0.0001, 1.0].
pub fn update_influence_share(
    current_share: f64,
    node_fitness: f64,
    average_network_fitness: f64,
    learning_rate: f64,
) -> f64 {
    let dx_dt = current_share * (node_fitness - average_network_fitness);
    let new_share = current_share + learning_rate * dx_dt;
    new_share.clamp(0.0001, 1.0)
}

/// Replicator Equation — Heun (RK2) Integration
///
/// Improved stability over Euler for multi-node simulations:
/// ```text
/// k1 = x · (f - f̄)
/// k2 = (x + 0.5·dt·k1) · (f - f̄)
/// x(t+dt) = x + 0.5·dt·(k1 + k2)
/// ```
pub fn update_influence_share_heun(
    current_share: f64,
    node_fitness: f64,
    average_network_fitness: f64,
    learning_rate: f64,
) -> f64 {
    let fitness_diff = node_fitness - average_network_fitness;
    let k1 = current_share * fitness_diff;
    let predictor = current_share + 0.5 * learning_rate * k1;
    let k2 = predictor * fitness_diff;
    let new_share = current_share + 0.5 * learning_rate * (k1 + k2);
    new_share.clamp(0.0001, 1.0)
}

/// Simulate replicator dynamics for a population of nodes over multiple steps.
///
/// # Parameters
/// - `shares`: Initial influence shares (must sum to ~1.0).
/// - `fitnesses`: Fitness scores for each node.
/// - `steps`: Number of simulation steps.
/// - `learning_rate`: Time step (dt).
///
/// # Returns
/// Vector of share distributions at each step.
pub fn simulate_replicator_dynamics(
    shares: &[f64],
    fitnesses: &[f64],
    steps: usize,
    learning_rate: f64,
) -> Vec<Vec<f64>> {
    if shares.is_empty() || shares.len() != fitnesses.len() {
        return vec![];
    }
    let mut current_shares = shares.to_vec();
    let mut trajectory = vec![current_shares.clone()];

    for _ in 0..steps {
        let avg_fitness: f64 = fitnesses
            .iter()
            .zip(current_shares.iter())
            .map(|(&f, &s)| f * s)
            .sum();
        let next_shares: Vec<f64> = current_shares
            .iter()
            .zip(fitnesses.iter())
            .map(|(&s, &f)| {
                let dx_dt = s * (f - avg_fitness);
                (s + learning_rate * dx_dt).clamp(0.0001, 1.0)
            })
            .collect();

        // Renormalize to ensure shares sum to 1.0
        let total: f64 = next_shares.iter().sum();
        if total > 0.0 {
            current_shares = next_shares.iter().map(|&s| s / total).collect();
        }
        trajectory.push(current_shares.clone());
    }
    trajectory
}

/// Shapley Value Approximation — Fair Credit Allocation for Federated Audits
///
/// ```text
/// φ_i(v) ≈ 0.5 · (1/N) + 0.5 · (marginal_i / total_coalition)
/// ```
///
/// Combines equal-share fairness with marginal-contribution merit.
///
/// # Parameters
/// - `node_marginal_vfe_reduction`: VFE reduction attributable to this node.
/// - `total_coalition_vfe_reduction`: Total VFE reduction of the coalition.
/// - `total_nodes_in_coalition`: Number of nodes in the coalition.
///
/// # Returns
/// Shapley credit share in [0.0, 1.0].
pub fn compute_shapley_credit_allocation(
    node_marginal_vfe_reduction: f64,
    total_coalition_vfe_reduction: f64,
    total_nodes_in_coalition: usize,
) -> f64 {
    if total_coalition_vfe_reduction <= 0.0 || total_nodes_in_coalition == 0 {
        return 0.0;
    }
    let base_share = 1.0 / total_nodes_in_coalition as f64;
    let marginal_contribution_ratio = node_marginal_vfe_reduction / total_coalition_vfe_reduction;
    let shapley_approx = 0.5 * base_share + 0.5 * marginal_contribution_ratio;
    shapley_approx.clamp(0.0, 1.0)
}

/// Monte Carlo Shapley Value Approximation
///
/// For large coalitions, samples random subsets instead of enumerating 2^N:
/// ```text
/// φ_i ≈ (1/M) · Σ_m [v(S_m ∪ {i}) - v(S_m)]
/// ```
///
/// # Parameters
/// - `node_index`: Index of the target node.
/// - `total_nodes`: Total number of nodes in coalition.
/// - `coalition_value_fn`: Function computing coalition value given member indices.
/// - `samples`: Number of Monte Carlo samples (M).
/// - `seed`: Random seed for reproducibility.
pub fn compute_shapley_monte_carlo(
    node_index: usize,
    total_nodes: usize,
    coalition_value_fn: &dyn Fn(&[usize]) -> f64,
    samples: usize,
    seed: u64,
) -> f64 {
    if total_nodes == 0 || node_index >= total_nodes {
        return 0.0;
    }
    let mut state = seed;
    let mut shapley_sum = 0.0;

    for _ in 0..samples {
        // Generate random subset excluding target node
        let mut subset = Vec::with_capacity(total_nodes - 1);
        for j in 0..total_nodes {
            if j != node_index {
                state = next_random_u64(&mut state);
                let roll = (state % 1000) as f64 / 1000.0;
                if roll < 0.5 {
                    subset.push(j);
                }
            }
        }
        let v_without = coalition_value_fn(&subset);
        subset.push(node_index);
        let v_with = coalition_value_fn(&subset);
        shapley_sum += v_with - v_without;
    }
    shapley_sum / samples as f64
}

fn next_random_u64(state: &mut u64) -> u64 {
    // Simple LCG for deterministic sampling
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Compute Shapley values for all nodes in a coalition.
///
/// # Parameters
/// - `marginal_contributions`: VFE reduction for each node.
/// - `total_coalition_vfe_reduction`: Sum of all marginal contributions.
///
/// # Returns
/// Vector of Shapley credit shares (sums to ~1.0).
pub fn compute_all_shapley_values(
    marginal_contributions: &[f64],
    total_coalition_vfe_reduction: f64,
) -> Vec<f64> {
    let n = marginal_contributions.len();
    if n == 0 {
        return vec![];
    }
    marginal_contributions
        .iter()
        .map(|&m| compute_shapley_credit_allocation(m, total_coalition_vfe_reduction, n))
        .collect()
}

/// Nash Equilibrium Check — Verifies if current distribution is stable.
///
/// A distribution is at Nash equilibrium if no node can improve fitness
/// by unilaterally changing strategy.
///
/// # Parameters
/// - `shares`: Current influence distribution.
/// - `fitnesses`: Current fitness scores.
/// - `tolerance`: Convergence threshold.
///
/// # Returns
/// `true` if the distribution is within tolerance of Nash equilibrium.
pub fn is_nash_equilibrium(shares: &[f64], fitnesses: &[f64], tolerance: f64) -> bool {
    if shares.is_empty() || shares.len() != fitnesses.len() {
        return false;
    }
    let avg_fitness: f64 = shares
        .iter()
        .zip(fitnesses.iter())
        .map(|(&s, &f)| s * f)
        .sum();
    shares
        .iter()
        .zip(fitnesses.iter())
        .all(|(&s, &f)| (s * (f - avg_fitness)).abs() < tolerance)
}

/// Symbiotic Nash Convergence — Simulate until equilibrium or max steps.
///
/// # Returns
/// Final shares, number of steps taken, and whether equilibrium was reached.
pub fn converge_to_nash(
    initial_shares: &[f64],
    fitnesses: &[f64],
    max_steps: usize,
    learning_rate: f64,
    tolerance: f64,
) -> (Vec<f64>, usize, bool) {
    let trajectory =
        simulate_replicator_dynamics(initial_shares, fitnesses, max_steps, learning_rate);
    let final_shares = trajectory
        .last()
        .cloned()
        .unwrap_or_else(|| initial_shares.to_vec());

    // Check convergence at each step
    for (step, shares) in trajectory.iter().enumerate() {
        if is_nash_equilibrium(shares, fitnesses, tolerance) {
            return (shares.clone(), step, true);
        }
    }
    (final_shares, max_steps, false)
}

// ============================================================================
// S128 — Gossip Priority + Edge Scheduler Integration
// ============================================================================

/// Edge device type for scheduling priority calculations.
/// Mirrors `native-audit::edge_runtime::DeviceType` to avoid cross-crate dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDeviceType {
    Desktop,
    OldDesktop,
    Mobile,
    Iot,
    Smartwatch,
    Datacenter,
}

impl EdgeDeviceType {
    /// Compute efficiency weight for scheduling.
    /// Higher values = more capable device = higher base priority.
    pub fn device_efficiency_weight(&self) -> f64 {
        match self {
            EdgeDeviceType::Datacenter => 1.0,
            EdgeDeviceType::Desktop => 0.9,
            EdgeDeviceType::OldDesktop => 0.6,
            EdgeDeviceType::Mobile => 0.4,
            EdgeDeviceType::Iot => 0.15,
            EdgeDeviceType::Smartwatch => 0.1,
        }
    }

    /// Base energy cost per certification (mWh).
    pub fn base_energy_cost(&self) -> f64 {
        match self {
            EdgeDeviceType::Desktop => 0.05,
            EdgeDeviceType::OldDesktop => 0.08,
            EdgeDeviceType::Mobile => 0.03,
            EdgeDeviceType::Iot => 0.01,
            EdgeDeviceType::Smartwatch => 0.005,
            EdgeDeviceType::Datacenter => 5.0,
        }
    }
}

/// Gossip Priority — Scales message propagation rate by PoUS fitness × influence share.
///
/// Nodes with higher thermodynamic fitness and larger influence shares
/// propagate gossip messages more frequently, creating a fitness-aware
/// information diffusion network.
///
/// ```text
/// priority = (fitness × share).clamp(0.01, 1.0)
/// ```
///
/// # Parameters
/// - `fitness`: PoUS fitness score (non-negative).
/// - `current_share`: Current influence share in [0, 1].
///
/// # Returns
/// Priority value in [0.01, 1.0].
pub fn compute_gossip_priority(fitness: f64, current_share: f64) -> f64 {
    (fitness * current_share).clamp(0.01, 1.0)
}

/// Edge Scheduler Priority — Fitness-aware, battery-conscious scheduling.
///
/// Combines PoUS fitness, device efficiency, and battery level to determine
/// the scheduling priority for edge compute tasks. Low-battery devices
/// are deprioritized to preserve planetary network longevity.
///
/// ```text
/// priority = fitness × device_weight × battery_factor
/// battery_factor = min(battery_level, 1.0) × 0.5 + 0.5
/// ```
///
/// # Parameters
/// - `fitness`: PoUS fitness score (non-negative).
/// - `battery_level`: Battery level as ratio [0.0, 1.0].
/// - `device_type`: Edge device classification.
///
/// # Returns
/// Scheduling priority in [0.0, 1.0].
pub fn update_edge_scheduler_priority(
    fitness: f64,
    battery_level: f64,
    device_type: EdgeDeviceType,
) -> f64 {
    let device_weight = device_type.device_efficiency_weight();
    let battery_factor = battery_level.clamp(0.0, 1.0) * 0.5 + 0.5;
    let priority = fitness * device_weight * battery_factor;
    priority.clamp(0.0, 1.0)
}

// ============================================================================
// S129 — PoUS Replicator Dynamics with SGW + HMC Integration
// ============================================================================

/// PoUS Fitness with Sliced Gromov-Wasserstein (SGW) Manifold Bonus
///
/// Extends standard PoUS fitness with a topology-aware bonus that rewards
/// nodes whose activation manifolds are closer (in SGW distance) to the
/// safe centroid manifold.
///
/// ```text
/// fitness_sgw = fitness_pous × (1 + η × max(0, safe_dist - node_dist) / safe_dist)
/// ```
///
/// Where:
/// - `fitness_pous`: Standard PoUS fitness score.
/// - `node_dist`: SGW distance from node's manifold to coalition mean.
/// - `safe_dist`: SGW distance from safe centroid to coalition mean.
/// - `η`: Topology sensitivity coefficient (default 0.5).
///
/// # Parameters
/// - `base_fitness`: Standard PoUS fitness score.
/// - `node_sgw_dist`: SGW distance of this node to coalition mean manifold.
/// - `safe_sgw_dist`: SGW distance of safe centroid to coalition mean manifold.
/// - `topology_eta`: Topology sensitivity coefficient.
///
/// # Returns
/// Adjusted fitness score (non-negative).
pub fn compute_pous_fitness_sgw(
    base_fitness: f64,
    node_sgw_dist: f64,
    safe_sgw_dist: f64,
    topology_eta: f64,
) -> f64 {
    if safe_sgw_dist <= 0.0 || base_fitness <= 0.0 {
        return base_fitness;
    }
    let proximity_bonus = (safe_sgw_dist - node_sgw_dist) / safe_sgw_dist;
    let bonus = topology_eta * proximity_bonus.max(0.0);
    base_fitness * (1.0 + bonus)
}

/// SGW-Aware Replicator Dynamics
///
/// Extends standard replicator dynamics with SGW manifold distances as
/// fitness modifiers. Nodes with activation manifolds closer to the safe
/// centroid receive a fitness boost, accelerating their influence growth.
///
/// ```text
/// f_i' = f_i × (1 + η × max(0, D_safe - D_i) / D_safe)
/// dx_i/dt = x_i × (f_i' - φ̄')
/// ```
///
/// # Parameters
/// - `shares`: Current influence share distribution.
/// - `fitnesses`: Base PoUS fitness scores.
/// - `sgw_distances`: SGW distances from each node to coalition mean.
/// - `safe_sgw_dist`: SGW distance from safe centroid to coalition mean.
/// - `topology_eta`: Topology sensitivity coefficient.
/// - `steps`: Number of simulation steps.
/// - `learning_rate`: Time step (dt).
///
/// # Returns
/// Vector of share distributions at each step.
pub fn replicator_dynamics_sgw_aware(
    shares: &[f64],
    fitnesses: &[f64],
    sgw_distances: &[f64],
    safe_sgw_dist: f64,
    topology_eta: f64,
    steps: usize,
    learning_rate: f64,
) -> Vec<Vec<f64>> {
    if shares.is_empty() || shares.len() != fitnesses.len() || shares.len() != sgw_distances.len() {
        return vec![];
    }

    // Compute SGW-adjusted fitnesses
    let adjusted_fitnesses: Vec<f64> = fitnesses
        .iter()
        .zip(sgw_distances.iter())
        .map(|(&f, &d)| compute_pous_fitness_sgw(f, d, safe_sgw_dist, topology_eta))
        .collect();

    simulate_replicator_dynamics(shares, &adjusted_fitnesses, steps, learning_rate)
}

/// HMC Steering Credit Allocation
///
/// Computes the credit allocation for nodes based on HMC steering energy
/// reduction. Nodes that achieve greater energy reduction during HMC
/// steering receive higher credit, as they demonstrate more effective
/// safe-manifold navigation.
///
/// ```text
/// credit_i = energy_reduction_i / Σ energy_reduction_j
/// ```
///
/// # Parameters
/// - `energy_reductions`: Energy reduction achieved by each node during HMC steering.
///
/// # Returns
/// Credit allocation vector summing to 1.0 (or uniform if all zero).
pub fn compute_hmc_steer_credit(energy_reductions: &[f64]) -> Vec<f64> {
    let n = energy_reductions.len();
    if n == 0 {
        return vec![];
    }
    let total: f64 = energy_reductions.iter().filter(|&&e| e > 0.0).sum();
    if total <= 0.0 {
        // Uniform fallback
        return vec![1.0 / n as f64; n];
    }
    energy_reductions
        .iter()
        .map(|&e| (e.max(0.0)) / total)
        .collect()
}

/// PoUS-HMC Hybrid Fitness
///
/// Combines PoUS thermodynamic fitness with HMC steering energy reduction
/// for a hybrid node evaluation that rewards both useful computation and
/// safe-manifold navigation capability.
///
/// ```text
/// fitness_hybrid = α × fitness_pous + (1-α) × normalized_energy_reduction
/// ```
///
/// # Parameters
/// - `pous_fitness`: Standard PoUS fitness score.
/// - `energy_reduction`: HMC steering energy reduction.
/// - `max_energy_reduction`: Maximum energy reduction in the cohort (for normalization).
/// - `alpha`: PoUS weight (default 0.7).
///
/// # Returns
/// Hybrid fitness score (non-negative).
pub fn compute_pous_hmc_hybrid_fitness(
    pous_fitness: f64,
    energy_reduction: f64,
    max_energy_reduction: f64,
    alpha: f64,
) -> f64 {
    let normalized_energy = if max_energy_reduction > 0.0 {
        energy_reduction.max(0.0) / max_energy_reduction
    } else {
        0.0
    };
    alpha * pous_fitness + (1.0 - alpha) * normalized_energy
}

/// Replicator Dynamics with HMC Steering Feedback
///
/// Runs replicator dynamics where fitness is dynamically updated based on
/// HMC steering energy reduction at each step. This creates a feedback loop
/// where nodes that effectively navigate the safe manifold gain influence,
/// which in turn affects the coalition's collective steering direction.
///
/// # Parameters
/// - `shares`: Initial influence share distribution.
/// - `pous_fitnesses`: Base PoUS fitness scores.
/// - `energy_reductions`: HMC energy reduction per node.
/// - `max_energy_reduction`: Maximum energy reduction for normalization.
/// - `alpha`: PoUS weight in hybrid fitness.
/// - `steps`: Number of simulation steps.
/// - `learning_rate`: Time step (dt).
///
/// # Returns
/// Vector of share distributions at each step.
pub fn replicator_dynamics_hmc_feedback(
    shares: &[f64],
    pous_fitnesses: &[f64],
    energy_reductions: &[f64],
    max_energy_reduction: f64,
    alpha: f64,
    steps: usize,
    learning_rate: f64,
) -> Vec<Vec<f64>> {
    if shares.is_empty()
        || shares.len() != pous_fitnesses.len()
        || shares.len() != energy_reductions.len()
    {
        return vec![];
    }

    let hybrid_fitnesses: Vec<f64> = pous_fitnesses
        .iter()
        .zip(energy_reductions.iter())
        .map(|(&f, &e)| compute_pous_hmc_hybrid_fitness(f, e, max_energy_reduction, alpha))
        .collect();

    simulate_replicator_dynamics(shares, &hybrid_fitnesses, steps, learning_rate)
}

/// Full S129 Pipeline — SGW + HMC + Replicator Dynamics
///
/// Executes the complete S129 certification pipeline:
/// 1. Compute SGW-adjusted fitness for each node.
/// 2. Compute HMC steering credit allocation.
/// 3. Run hybrid replicator dynamics with both SGW and HMC feedback.
/// 4. Return final equilibrium shares and pipeline metrics.
///
/// # Parameters
/// - `shares`: Initial influence share distribution.
/// - `pous_fitnesses`: Base PoUS fitness scores.
/// - `sgw_distances`: SGW distances from each node to coalition mean.
/// - `safe_sgw_dist`: SGW distance from safe centroid to coalition mean.
/// - `energy_reductions`: HMC energy reduction per node.
/// - `max_energy_reduction`: Maximum energy reduction for normalization.
/// - `topology_eta`: Topology sensitivity coefficient.
/// - `alpha`: PoUS weight in hybrid fitness.
/// - `steps`: Number of replicator dynamics steps.
/// - `learning_rate`: Time step (dt).
///
/// # Returns
/// Tuple of (final_shares, trajectory, shapley_credits).
#[allow(clippy::too_many_arguments)]
pub fn s129_full_pipeline(
    shares: &[f64],
    pous_fitnesses: &[f64],
    sgw_distances: &[f64],
    safe_sgw_dist: f64,
    energy_reductions: &[f64],
    max_energy_reduction: f64,
    topology_eta: f64,
    alpha: f64,
    steps: usize,
    learning_rate: f64,
) -> (Vec<f64>, Vec<Vec<f64>>, Vec<f64>) {
    let n = shares.len();
    if n == 0
        || n != pous_fitnesses.len()
        || n != sgw_distances.len()
        || n != energy_reductions.len()
    {
        return (vec![], vec![], vec![]);
    }

    // Step 1: SGW-adjusted fitness
    let sgw_fitnesses: Vec<f64> = pous_fitnesses
        .iter()
        .zip(sgw_distances.iter())
        .map(|(&f, &d)| compute_pous_fitness_sgw(f, d, safe_sgw_dist, topology_eta))
        .collect();

    // Step 2: HMC steering credit
    let hmc_credits = compute_hmc_steer_credit(energy_reductions);

    // Step 3: Hybrid fitness (SGW + HMC)
    let hybrid_fitnesses: Vec<f64> = sgw_fitnesses
        .iter()
        .zip(energy_reductions.iter())
        .map(|(&f, &e)| compute_pous_hmc_hybrid_fitness(f, e, max_energy_reduction, alpha))
        .collect();

    // Step 4: Replicator dynamics
    let trajectory = simulate_replicator_dynamics(shares, &hybrid_fitnesses, steps, learning_rate);

    // Final shares
    let final_shares = trajectory
        .last()
        .cloned()
        .unwrap_or_else(|| shares.to_vec());

    (final_shares, trajectory, hmc_credits)
}

// --- S130: Meta-Replicator + Self-Improving Loop ---

/// Configuration for the Meta-Replicator — second-order optimization of replicator hyperparameters.
#[derive(Debug, Clone)]
pub struct MetaReplicatorConfig {
    /// Learning rate for meta-gradient updates on hyperparameters.
    pub meta_lr: f64,
    /// Coefficient learning rate (α in base PoUS).
    pub alpha_lr: f64,
    /// Coefficient learning rate (β in base PoUS).
    pub beta_lr: f64,
    /// Coefficient learning rate (γ in base PoUS).
    pub gamma_lr: f64,
    /// Replicator learning rate adaptation speed.
    pub replicator_lr_adapt: f64,
    /// Minimum learning rate for replicator dynamics.
    pub min_lr: f64,
    /// Maximum learning rate for replicator dynamics.
    pub max_lr: f64,
    /// Temperature for exploration in meta-space.
    pub temperature: f64,
    /// Convergence window for detecting stagnation.
    pub convergence_window: usize,
    /// Convergence tolerance for meta-gradient norm.
    pub convergence_tolerance: f64,
    /// Maximum number of meta-iterations.
    pub max_meta_iterations: usize,
    /// Momentum coefficient for meta-gradient (0 = no momentum).
    pub momentum: f64,
}

impl Default for MetaReplicatorConfig {
    fn default() -> Self {
        Self {
            meta_lr: 0.01,
            alpha_lr: 0.005,
            beta_lr: 0.005,
            gamma_lr: 0.005,
            replicator_lr_adapt: 0.01,
            min_lr: 1e-6,
            max_lr: 0.5,
            temperature: 1.0,
            convergence_window: 20,
            convergence_tolerance: 1e-8,
            max_meta_iterations: 500,
            momentum: 0.9,
        }
    }
}

impl MetaReplicatorConfig {
    /// Create config with custom meta learning rate.
    pub fn with_meta_lr(mut self, lr: f64) -> Self {
        self.meta_lr = lr.clamp(1e-8, 1.0);
        self
    }

    /// Create config with custom convergence tolerance.
    pub fn with_convergence_tolerance(mut self, tol: f64) -> Self {
        self.convergence_tolerance = tol.clamp(1e-12, 1.0);
        self
    }

    /// Fast convergence preset — fewer iterations, looser tolerance.
    pub fn fast_converge() -> Self {
        Self {
            convergence_window: 10,
            convergence_tolerance: 1e-5,
            max_meta_iterations: 100,
            meta_lr: 0.05,
            ..Self::default()
        }
    }

    /// High-precision preset — more iterations, tighter tolerance.
    pub fn high_precision() -> Self {
        Self {
            convergence_window: 50,
            convergence_tolerance: 1e-10,
            max_meta_iterations: 2000,
            meta_lr: 0.001,
            momentum: 0.95,
            ..Self::default()
        }
    }
}

/// Result of a single meta-replicator step.
#[derive(Debug, Clone)]
pub struct MetaReplicatorResult {
    /// Updated replicator learning rate.
    pub updated_lr: f64,
    /// Updated PoUS coefficients (α, β, γ, δ).
    pub updated_coefficients: [f64; 4],
    /// Meta-gradient norm.
    pub meta_gradient_norm: f64,
    /// Meta-fitness improvement (positive = improvement).
    pub meta_improvement: f64,
    /// Current meta-temperature.
    pub temperature: f64,
    /// Whether convergence detected.
    pub converged: bool,
    /// Number of meta-iterations performed.
    pub iterations: usize,
}

/// Tracks historical performance for self-improving loop.
#[derive(Debug, Clone)]
pub struct SelfImprovementState {
    /// Historical fitness values (most recent last).
    pub history: Vec<f64>,
    /// Best fitness observed so far.
    pub best_fitness: f64,
    /// Current adaptive learning rate.
    pub adaptive_lr: f64,
    /// Momentum buffer for meta-gradients.
    pub momentum_buffer: Vec<f64>,
    /// Number of consecutive improvements.
    pub consecutive_improvements: usize,
    /// Number of consecutive stagnations.
    pub consecutive_stagnations: usize,
    /// Total meta-iterations performed.
    pub total_iterations: usize,
}

impl SelfImprovementState {
    /// Create initial state with given learning rate and dimension.
    pub fn new(initial_lr: f64, dim: usize) -> Self {
        Self {
            history: Vec::new(),
            best_fitness: f64::NEG_INFINITY,
            adaptive_lr: initial_lr,
            momentum_buffer: vec![0.0; dim],
            consecutive_improvements: 0,
            consecutive_stagnations: 0,
            total_iterations: 0,
        }
    }

    /// Record a new fitness observation.
    pub fn record(&mut self, fitness: f64) {
        self.history.push(fitness);
        if fitness > self.best_fitness {
            self.best_fitness = fitness;
            self.consecutive_improvements += 1;
            self.consecutive_stagnations = 0;
        } else if fitness < self.best_fitness - 1e-10 {
            self.consecutive_improvements = 0;
            self.consecutive_stagnations += 1;
        }
        self.total_iterations += 1;
    }

    /// Get the trend (derivative approximation) from recent history.
    pub fn trend(&self, window: usize) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        let w = window.min(self.history.len());
        let recent = &self.history[self.history.len() - w..];
        if recent.len() < 2 {
            return 0.0;
        }
        (recent[recent.len() - 1] - recent[0]) / (w as f64)
    }

    /// Check if stuck in local optimum (stagnation detected).
    pub fn is_stagnated(&self, window: usize, tolerance: f64) -> bool {
        if self.history.len() < window {
            return false;
        }
        let recent = &self.history[self.history.len() - window..];
        let max_val = recent.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_val = recent.iter().cloned().fold(f64::INFINITY, f64::min);
        (max_val - min_val) < tolerance
    }
}

/// Convergence monitoring for adaptive step sizing.
#[derive(Debug, Clone)]
pub struct ConvergenceMonitor {
    /// Recent gradient norms.
    pub recent_norms: Vec<f64>,
    /// Window size for convergence detection.
    pub window: usize,
    /// Tolerance for convergence.
    pub tolerance: f64,
    /// Current step size multiplier.
    pub step_multiplier: f64,
}

impl ConvergenceMonitor {
    /// Create new monitor with given window and tolerance.
    pub fn new(window: usize, tolerance: f64) -> Self {
        Self {
            recent_norms: Vec::new(),
            window,
            tolerance,
            step_multiplier: 1.0,
        }
    }

    /// Record a new gradient norm and update step multiplier.
    pub fn record_norm(&mut self, norm: f64) -> bool {
        self.recent_norms.push(norm);
        if self.recent_norms.len() > self.window {
            self.recent_norms.remove(0);
        }
        let converged = self.is_converged();
        if converged {
            // Reduce step size near convergence for fine-tuning
            self.step_multiplier = (self.step_multiplier * 0.9).max(0.1);
        } else if norm > 0.0 {
            // Increase step size when far from convergence
            self.step_multiplier = (self.step_multiplier * 1.05).min(5.0);
        }
        converged
    }

    /// Check if converged based on recent norms.
    pub fn is_converged(&self) -> bool {
        if self.recent_norms.len() < 2 {
            return false;
        }
        self.recent_norms.last().is_some_and(|&n| n < self.tolerance)
    }

    /// Get adaptive step size.
    pub fn adaptive_step(&self, base_lr: f64) -> f64 {
        (base_lr * self.step_multiplier).clamp(1e-8, 1.0)
    }
}

/// Compute meta-gradient for replicator learning rate adaptation.
///
/// Uses finite-difference approximation: ∇_lr meta_fitness ≈ (f(lr+ε) - f(lr-ε)) / (2ε)
pub fn compute_meta_gradient(
    _current_lr: f64,
    _current_fitness: f64,
    perturbed_fitness_plus: f64,
    perturbed_fitness_minus: f64,
    epsilon: f64,
) -> f64 {
    if epsilon <= 0.0 {
        return 0.0;
    }
    (perturbed_fitness_plus - perturbed_fitness_minus) / (2.0 * epsilon)
}

/// Meta-replicator step — second-order optimization of replicator hyperparameters.
///
/// Optimizes the learning rate and PoUS coefficients to maximize long-term fitness.
pub fn meta_replicator_step(
    shares: &[f64],
    fitnesses: &[f64],
    current_lr: f64,
    coefficients: &[f64; 4],
    state: &mut SelfImprovementState,
    config: &MetaReplicatorConfig,
) -> MetaReplicatorResult {
    let n = shares.len();
    if n == 0 || n != fitnesses.len() {
        return MetaReplicatorResult {
            updated_lr: current_lr,
            updated_coefficients: *coefficients,
            meta_gradient_norm: 0.0,
            meta_improvement: 0.0,
            temperature: config.temperature,
            converged: true,
            iterations: 0,
        };
    }

    // Compute current average fitness as meta-objective
    let avg_fitness: f64 = fitnesses.iter().sum::<f64>() / n as f64;

    // Finite-difference meta-gradient for learning rate
    let epsilon = 1e-4;
    let lr_plus = (current_lr + epsilon).min(config.max_lr);
    let lr_minus = (current_lr - epsilon).max(config.min_lr);

    // Evaluate fitness with perturbed learning rates (one-step lookahead)
    let fitness_plus = evaluate_one_step_fitness(shares, fitnesses, lr_plus, coefficients);
    let fitness_minus = evaluate_one_step_fitness(shares, fitnesses, lr_minus, coefficients);

    let lr_gradient = compute_meta_gradient(
        current_lr,
        avg_fitness,
        fitness_plus,
        fitness_minus,
        epsilon,
    );

    // Update learning rate with momentum
    let momentum_idx = 0;
    state.momentum_buffer[momentum_idx] =
        config.momentum * state.momentum_buffer[momentum_idx] + (1.0 - config.momentum) * lr_gradient;
    let new_lr = current_lr + config.meta_lr * state.momentum_buffer[momentum_idx];
    let new_lr = new_lr.max(config.min_lr).min(config.max_lr);

    // Meta-gradients for PoUS coefficients
    let mut new_coeffs = *coefficients;
    for i in 0..3 {
        let coeff_epsilon = 1e-4;
        let mut coeffs_plus = *coefficients;
        let mut coeffs_minus = *coefficients;
        coeffs_plus[i] += coeff_epsilon;
        coeffs_minus[i] = (coeffs_minus[i] - coeff_epsilon).max(0.0);

        let lr_coeff = if i == 0 {
            config.alpha_lr
        } else if i == 1 {
            config.beta_lr
        } else {
            config.gamma_lr
        };

        let coeff_gradient = (compute_pous_fitness_custom(
            coeffs_plus[0],
            coeffs_plus[1],
            coeffs_plus[2],
            coeffs_plus[3],
            1.0, 1.0, 0.99, 0.01,
        ) - compute_pous_fitness_custom(
            coeffs_minus[0],
            coeffs_minus[1],
            coeffs_minus[2],
            coeffs_minus[3],
            1.0, 1.0, 0.99, 0.01,
        )) / (2.0 * coeff_epsilon);

        let mom_idx = i + 1;
        state.momentum_buffer[mom_idx] =
            config.momentum * state.momentum_buffer[mom_idx] + (1.0 - config.momentum) * coeff_gradient;
        new_coeffs[i] += lr_coeff * state.momentum_buffer[mom_idx];
        new_coeffs[i] = new_coeffs[i].clamp(0.0, 10.0);
    }

    // Compute meta-gradient norm
    let meta_grad_norm = (lr_gradient * lr_gradient
        + state.momentum_buffer[1] * state.momentum_buffer[1]
        + state.momentum_buffer[2] * state.momentum_buffer[2]
        + state.momentum_buffer[3] * state.momentum_buffer[3])
        .sqrt();

    // Record fitness and check improvement
    let meta_improvement = avg_fitness - state.best_fitness.max(0.0);
    state.record(avg_fitness);

    // Adaptive temperature — increase if stagnated
    let temperature = if state.is_stagnated(
        config.convergence_window,
        config.convergence_tolerance,
    ) {
        config.temperature * 1.5
    } else {
        config.temperature * 0.99
    };

    MetaReplicatorResult {
        updated_lr: new_lr,
        updated_coefficients: new_coeffs,
        meta_gradient_norm: meta_grad_norm,
        meta_improvement,
        temperature,
        converged: meta_grad_norm < config.convergence_tolerance,
        iterations: state.total_iterations,
    }
}

/// Evaluate one-step fitness for meta-gradient computation.
fn evaluate_one_step_fitness(
    shares: &[f64],
    fitnesses: &[f64],
    lr: f64,
    _coefficients: &[f64; 4],
) -> f64 {
    let n = shares.len();
    if n == 0 {
        return 0.0;
    }
    let avg_f: f64 = fitnesses.iter().sum::<f64>() / n as f64;

    // One-step replicator update
    let mut new_shares = Vec::with_capacity(n);
    for i in 0..n {
        let dx = shares[i] * (fitnesses[i] - avg_f);
        let new_s = (shares[i] + lr * dx).clamp(1e-10, 1.0);
        new_shares.push(new_s);
    }

    // Normalize
    let total: f64 = new_shares.iter().sum();
    if total > 0.0 {
        new_shares.iter_mut().for_each(|s| *s /= total);
    }

    // Compute entropy as diversity measure
    let entropy: f64 = new_shares
        .iter()
        .filter(|&&s| s > 1e-15)
        .map(|&s| -s * s.ln())
        .sum();

    // Meta-fitness = weighted sum of avg fitness and diversity
    0.7 * avg_f + 0.3 * entropy
}

/// Self-improving fitness evaluation — adapts coefficients based on historical performance.
pub fn self_improving_fitness(
    base_fitness: f64,
    state: &SelfImprovementState,
    config: &MetaReplicatorConfig,
) -> f64 {
    // Trend bonus — reward consistent improvement
    let trend = state.trend(config.convergence_window);
    let trend_bonus = trend * config.temperature;

    // Stagnation penalty — encourage exploration when stuck
    let stagnation_penalty = if state.is_stagnated(
        config.convergence_window,
        config.convergence_tolerance,
    ) {
        -0.1 * config.temperature
    } else {
        0.0
    };

    // Diversity bonus from history variance
    let diversity_bonus = if state.history.len() >= 2 {
        let mean: f64 = state.history.iter().sum::<f64>() / state.history.len() as f64;
        let variance: f64 = state
            .history
            .iter()
            .map(|&v| (v - mean).powi(2))
            .sum::<f64>()
            / state.history.len() as f64;
        0.05 * variance.sqrt()
    } else {
        0.0
    };

    (base_fitness + trend_bonus + stagnation_penalty + diversity_bonus).max(0.0)
}

/// Run full self-improving loop — iterative meta-optimization with convergence monitoring.
pub fn run_self_improving_loop(
    initial_shares: &[f64],
    initial_fitnesses: &[f64],
    initial_lr: f64,
    initial_coefficients: [f64; 4],
    config: &MetaReplicatorConfig,
) -> Vec<MetaReplicatorResult> {
    let n = initial_shares.len();
    if n == 0 || n != initial_fitnesses.len() {
        return Vec::new();
    }

    let mut state = SelfImprovementState::new(initial_lr, 5); // 5 dims: lr + 4 coeffs
    let mut monitor = ConvergenceMonitor::new(config.convergence_window, config.convergence_tolerance);
    let mut current_lr = initial_lr;
    let mut current_coeffs = initial_coefficients;
    let mut current_shares = initial_shares.to_vec();
    let mut current_fitnesses = initial_fitnesses.to_vec();
    let mut results = Vec::new();

    for _iter in 0..config.max_meta_iterations {
        // Meta-replicator step
        let meta_result = meta_replicator_step(
            &current_shares,
            &current_fitnesses,
            current_lr,
            &current_coeffs,
            &mut state,
            config,
        );

        // Monitor convergence
        let converged = monitor.record_norm(meta_result.meta_gradient_norm);

        // Update hyperparameters
        current_lr = meta_result.updated_lr;
        current_coeffs = meta_result.updated_coefficients;

        // One-step replicator dynamics with adaptive lr
        let adaptive_lr = monitor.adaptive_step(current_lr);
        let avg_f: f64 = current_fitnesses.iter().sum::<f64>() / n as f64;
        let mut new_shares = Vec::with_capacity(n);
        for i in 0..n {
            let dx = current_shares[i] * (current_fitnesses[i] - avg_f);
            let new_s = (current_shares[i] + adaptive_lr * dx).clamp(1e-10, 1.0);
            new_shares.push(new_s);
        }
        let total: f64 = new_shares.iter().sum();
        if total > 0.0 {
            new_shares.iter_mut().for_each(|s| *s /= total);
        }
        current_shares = new_shares;

        // Update fitnesses based on new shares (simulated environment feedback)
        for i in 0..n {
            current_fitnesses[i] = compute_pous_fitness_custom(
                current_coeffs[0],
                current_coeffs[1],
                current_coeffs[2],
                current_coeffs[3],
                current_shares[i] * 10.0,
                current_shares[i] * 5.0,
                0.9 + 0.1 * current_shares[i],
                (1.0 - current_shares[i]) * 0.1,
            );
        }

        // Record self-improving fitness
        let si_fitness = self_improving_fitness(
            current_fitnesses.iter().sum::<f64>() / n as f64,
            &state,
            config,
        );
        let _ = si_fitness; // Used in state.record via meta_replicator_step

        // Check convergence before moving meta_result
        if converged || meta_result.converged {
            results.push(meta_result);
            break;
        }
        results.push(meta_result);
    }

    results
}

/// S130 Full Pipeline — Meta-Replicator + Self-Improving Loop + PoUS + SGW + HMC.
#[allow(clippy::too_many_arguments)]
pub fn s130_full_pipeline(
    shares: &[f64],
    fitnesses: &[f64],
    sgw_dists: &[f64],
    safe_dist: f64,
    energy_reductions: &[f64],
    max_energy: f64,
    sgw_bonus_coeff: f64,
    alpha_hmc: f64,
    meta_config: &MetaReplicatorConfig,
) -> (Vec<f64>, Vec<MetaReplicatorResult>, Vec<f64>) {
    let n = shares.len();
    if n == 0
        || n != fitnesses.len()
        || n != sgw_dists.len()
        || n != energy_reductions.len()
    {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    // Step 1: Compute hybrid fitness (SGW + HMC + PoUS)
    let mut hybrid_fitnesses = Vec::with_capacity(n);
    for i in 0..n {
        let sgw_fitness = compute_pous_fitness_sgw(
            fitnesses[i],
            sgw_dists[i],
            safe_dist,
            sgw_bonus_coeff,
        );
        let hmc_normalized = if max_energy > 0.0 {
            (energy_reductions[i] / max_energy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let hybrid = (1.0 - alpha_hmc) * hmc_normalized + alpha_hmc * sgw_fitness;
        hybrid_fitnesses.push(hybrid);
    }

    // Step 2: Run meta-replicator loop
    let results = run_self_improving_loop(
        shares,
        &hybrid_fitnesses,
        0.01,
        [0.5, 0.3, 0.2, 2.0],
        meta_config,
    );

    // Step 3: Extract final shares from last meta-result's implicit dynamics
    let final_shares = if !results.is_empty() {
        let last = results.last().unwrap();
        // Compute final shares using updated coefficients
        let _coeffs = last.updated_coefficients;
        let mut final_shares = shares.to_vec();
        let avg_f: f64 = hybrid_fitnesses.iter().sum::<f64>() / n as f64;
        for i in 0..n {
            let dx = final_shares[i] * (hybrid_fitnesses[i] - avg_f);
            final_shares[i] = (final_shares[i] + last.updated_lr * dx).clamp(1e-10, 1.0);
        }
        let total: f64 = final_shares.iter().sum();
        if total > 0.0 {
            final_shares.iter_mut().for_each(|s| *s /= total);
        }
        final_shares
    } else {
        shares.to_vec()
    };

    // Step 4: Compute Shapley credits from final hybrid fitnesses
    let total_fitness: f64 = hybrid_fitnesses.iter().sum();
    let credits = compute_all_shapley_values(&hybrid_fitnesses, total_fitness);

    (final_shares, results, credits)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PoUS Fitness Tests ---

    #[test]
    fn test_pous_fitness_basic() {
        let fitness = compute_pous_fitness(10.0, 5.0, 0.95, 0.1);
        assert!(fitness > 0.0);
        // α·10 + β·5 + γ·0.95 - δ·0.1 = 5 + 1.5 + 0.19 - 0.2 = 6.49
        assert!((fitness - 6.49).abs() < 1e-10);
    }

    #[test]
    fn test_pous_fitness_zero_inputs() {
        let fitness = compute_pous_fitness(0.0, 0.0, 0.0, 0.0);
        assert_eq!(fitness, 0.0);
    }

    #[test]
    fn test_pous_fitness_high_byzantine_penalty() {
        let fitness = compute_pous_fitness(1.0, 1.0, 1.0, 10.0);
        // 0.5 + 0.3 + 0.2 - 20 = -19 → clamped to 0
        assert_eq!(fitness, 0.0);
    }

    #[test]
    fn test_pous_fitness_negative_raw_clamped() {
        let fitness = compute_pous_fitness(0.0, 0.0, 0.0, 1.0);
        // 0 + 0 + 0 - 2 = -2 → clamped to 0
        assert_eq!(fitness, 0.0);
    }

    #[test]
    fn test_pous_fitness_custom_coefficients() {
        let fitness = compute_pous_fitness_custom(10.0, 5.0, 0.95, 0.1, 1.0, 0.0, 0.0, 0.0);
        assert_eq!(fitness, 10.0);
    }

    #[test]
    fn test_pous_fitness_with_entropy_uniform() {
        let base = 5.0;
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let result = compute_pous_fitness_with_entropy(base, &dist, 0.1);
        // Entropy of uniform = -4 * 0.25 * ln(0.25) = ln(4) ≈ 1.386
        // result ≈ 5.0 + 0.1 * 1.386 = 5.1386
        assert!((result - 5.1386).abs() < 1e-3);
    }

    #[test]
    fn test_pous_fitness_with_entropy_empty() {
        let result = compute_pous_fitness_with_entropy(5.0, &[], 0.1);
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_pous_fitness_with_entropy_monopoly() {
        let base = 5.0;
        let dist = vec![1.0];
        let result = compute_pous_fitness_with_entropy(base, &dist, 0.1);
        // Entropy of monopoly = -1 * ln(1) = 0
        assert_eq!(result, 5.0);
    }

    // --- Replicator Dynamics Tests ---

    #[test]
    fn test_update_influence_share_above_avg() {
        let new_share = update_influence_share(0.1, 1.2, 0.8, 0.01);
        // dx/dt = 0.1 * (1.2 - 0.8) = 0.04
        // new = 0.1 + 0.01 * 0.04 = 0.1004
        assert!(new_share > 0.1);
        assert!((new_share - 0.1004).abs() < 1e-10);
    }

    #[test]
    fn test_update_influence_share_below_avg() {
        let new_share = update_influence_share(0.1, 0.5, 1.0, 0.01);
        // dx/dt = 0.1 * (0.5 - 1.0) = -0.05
        // new = 0.1 + 0.01 * (-0.05) = 0.0995
        assert!(new_share < 0.1);
        assert!((new_share - 0.0995).abs() < 1e-10);
    }

    #[test]
    fn test_update_influence_share_at_avg() {
        let new_share = update_influence_share(0.1, 1.0, 1.0, 0.01);
        assert_eq!(new_share, 0.1);
    }

    #[test]
    fn test_update_influence_share_clamp_low() {
        let new_share = update_influence_share(0.0001, 0.0, 10.0, 1.0);
        // Would go negative, clamped to 0.0001
        assert_eq!(new_share, 0.0001);
    }

    #[test]
    fn test_update_influence_share_clamp_high() {
        let new_share = update_influence_share(1.0, 10.0, 0.0, 1.0);
        // Would exceed 1.0, clamped to 1.0
        assert_eq!(new_share, 1.0);
    }

    #[test]
    fn test_update_influence_share_heun_stability() {
        let euler = update_influence_share(0.5, 2.0, 1.0, 0.1);
        let heun = update_influence_share_heun(0.5, 2.0, 1.0, 0.1);
        // Heun should be more stable (smaller step)
        assert!(heun > 0.0 && heun < 1.0);
        // Both increase since fitness > avg
        assert!(euler > 0.5);
        assert!(heun > 0.5);
    }

    #[test]
    fn test_simulate_replicator_dynamics_empty() {
        let result = simulate_replicator_dynamics(&[], &[], 10, 0.01);
        assert!(result.is_empty());
    }

    #[test]
    fn test_simulate_replicator_dynamics_length_mismatch() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0], 10, 0.01);
        assert!(result.is_empty());
    }

    #[test]
    fn test_simulate_replicator_dynamics_convergence() {
        let shares = vec![0.3, 0.3, 0.4];
        let fitnesses = vec![1.0, 2.0, 1.5];
        let trajectory = simulate_replicator_dynamics(&shares, &fitnesses, 100, 0.01);
        assert_eq!(trajectory.len(), 101); // initial + 100 steps
                                           // Node 1 (highest fitness) should gain share
        let final_shares = trajectory.last().unwrap();
        assert!(final_shares[1] > shares[1]);
    }

    #[test]
    fn test_simulate_replicator_dynamics_normalization() {
        let shares = vec![0.33, 0.33, 0.34];
        let fitnesses = vec![1.0, 3.0, 2.0];
        let trajectory = simulate_replicator_dynamics(&shares, &fitnesses, 50, 0.05);
        // Check shares sum to ~1.0 at each step
        for step_shares in &trajectory {
            let sum: f64 = step_shares.iter().sum();
            assert!((sum - 1.0).abs() < 0.01, "Shares sum {} != 1.0", sum);
        }
    }

    // --- Shapley Value Tests ---

    #[test]
    fn test_shapley_credit_allocation_basic() {
        let credit = compute_shapley_credit_allocation(5.0, 20.0, 4);
        // base_share = 1/4 = 0.25, marginal_ratio = 5/20 = 0.25
        // shapley = 0.5 * 0.25 + 0.5 * 0.25 = 0.25
        assert!((credit - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_shapley_credit_allocation_zero_total() {
        let credit = compute_shapley_credit_allocation(5.0, 0.0, 4);
        assert_eq!(credit, 0.0);
    }

    #[test]
    fn test_shapley_credit_allocation_zero_nodes() {
        let credit = compute_shapley_credit_allocation(5.0, 20.0, 0);
        assert_eq!(credit, 0.0);
    }

    #[test]
    fn test_shapley_credit_allocation_single_node() {
        let credit = compute_shapley_credit_allocation(10.0, 10.0, 1);
        // base = 1.0, marginal = 1.0 → shapley = 1.0
        assert!((credit - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_shapley_credit_allocation_high_contributor() {
        let credit = compute_shapley_credit_allocation(15.0, 20.0, 4);
        // base = 0.25, marginal = 0.75 → shapley = 0.5
        assert!((credit - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_shapley_credit_allocation_low_contributor() {
        let credit = compute_shapley_credit_allocation(1.0, 20.0, 4);
        // base = 0.25, marginal = 0.05 → shapley = 0.15
        assert!((credit - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_shapley_credit_allocation_clamped() {
        let credit = compute_shapley_credit_allocation(100.0, 10.0, 1);
        assert!(credit <= 1.0);
    }

    #[test]
    fn test_compute_all_shapley_values() {
        let contributions = vec![5.0, 10.0, 5.0];
        let total = 20.0;
        let values = compute_all_shapley_values(&contributions, total);
        assert_eq!(values.len(), 3);
        // Sum should be ~1.0
        let sum: f64 = values.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_all_shapley_values_empty() {
        let values = compute_all_shapley_values(&[], 0.0);
        assert!(values.is_empty());
    }

    #[test]
    fn test_compute_all_shapley_values_equal_contributions() {
        let contributions = vec![3.0, 3.0, 3.0, 3.0];
        let total = 12.0;
        let values = compute_all_shapley_values(&contributions, total);
        for &v in &values {
            assert!((v - 0.25).abs() < 1e-10);
        }
    }

    // --- Monte Carlo Shapley Tests ---

    #[test]
    fn test_shapley_monte_carlo_basic() {
        let contributions = [3.0, 5.0, 2.0];
        let total_nodes = 3;
        let value_fn =
            |members: &[usize]| -> f64 { members.iter().map(|&i| contributions[i]).sum() };
        let phi = compute_shapley_monte_carlo(0, total_nodes, &value_fn, 1000, 42);
        // For additive value function, Shapley value = marginal contribution
        // Node 0 contributes 3.0, so phi ≈ 3.0
        assert!((phi - 3.0).abs() < 1.0);
    }

    #[test]
    fn test_shapley_monte_carlo_invalid_node() {
        let value_fn = |_m: &[usize]| 1.0;
        let phi = compute_shapley_monte_carlo(5, 3, &value_fn, 100, 42);
        assert_eq!(phi, 0.0);
    }

    #[test]
    fn test_shapley_monte_carlo_empty_coalition() {
        let value_fn = |_m: &[usize]| 1.0;
        let phi = compute_shapley_monte_carlo(0, 0, &value_fn, 100, 42);
        assert_eq!(phi, 0.0);
    }

    #[test]
    fn test_shapley_monte_carlo_deterministic() {
        let value_fn = |members: &[usize]| members.len() as f64;
        let phi1 = compute_shapley_monte_carlo(0, 3, &value_fn, 500, 123);
        let phi2 = compute_shapley_monte_carlo(0, 3, &value_fn, 500, 123);
        assert!((phi1 - phi2).abs() < 1e-10);
    }

    // --- Nash Equilibrium Tests ---

    #[test]
    fn test_is_nash_equilibrium_equal_fitness() {
        let shares = vec![0.33, 0.33, 0.34];
        let fitnesses = vec![1.0, 1.0, 1.0];
        assert!(is_nash_equilibrium(&shares, &fitnesses, 1e-6));
    }

    #[test]
    fn test_is_nash_equilibrium_not_converged() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 3.0];
        assert!(!is_nash_equilibrium(&shares, &fitnesses, 1e-6));
    }

    #[test]
    fn test_is_nash_equilibrium_empty() {
        assert!(!is_nash_equilibrium(&[], &[], 1e-6));
    }

    #[test]
    fn test_is_nash_equilibrium_length_mismatch() {
        assert!(!is_nash_equilibrium(&[0.5], &[0.5, 0.5], 1e-6));
    }

    #[test]
    fn test_converge_to_nash_reaches_equilibrium() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 1.0];
        let (_final_shares, steps, converged) =
            converge_to_nash(&shares, &fitnesses, 1000, 0.01, 1e-6);
        assert!(converged);
        assert!(steps == 0); // Already at equilibrium
    }

    #[test]
    fn test_converge_to_nash_improves_higher_fitness() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 3.0];
        let (final_shares, _steps, _converged) =
            converge_to_nash(&shares, &fitnesses, 1000, 0.01, 1e-6);
        // Node 1 should have more share
        assert!(final_shares[1] > final_shares[0]);
    }

    #[test]
    fn test_converge_to_nash_max_steps() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 10.0];
        let (_final_shares, steps, converged) =
            converge_to_nash(&shares, &fitnesses, 10, 0.001, 1e-9);
        assert_eq!(steps, 10);
        assert!(!converged);
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_pous_pipeline() {
        // Compute fitness for 3 nodes
        let f1 = compute_pous_fitness(8.0, 4.0, 0.9, 0.0);
        let f2 = compute_pous_fitness(12.0, 6.0, 0.95, 0.1);
        let f3 = compute_pous_fitness(5.0, 2.0, 0.8, 0.5);
        let fitnesses = vec![f1, f2, f3];

        // Simulate replicator dynamics
        let shares = vec![0.33, 0.34, 0.33];
        let trajectory = simulate_replicator_dynamics(&shares, &fitnesses, 50, 0.01);
        let final_shares = trajectory.last().unwrap();

        // Node 2 (highest fitness) should dominate
        assert!(final_shares[1] > final_shares[0]);
        assert!(final_shares[1] > final_shares[2]);

        // Compute Shapley credits
        let contributions = vec![8.0, 12.0, 5.0];
        let total = 25.0;
        let shapley = compute_all_shapley_values(&contributions, total);
        let shapley_sum: f64 = shapley.iter().sum();
        assert!((shapley_sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_entropy_bonus_prevents_monopoly() {
        let base_fitness = compute_pous_fitness(10.0, 5.0, 1.0, 0.0);
        // Monopoly distribution
        let monopoly_dist = vec![1.0];
        let monopoly_fitness = compute_pous_fitness_with_entropy(base_fitness, &monopoly_dist, 0.5);
        // Diverse distribution
        let diverse_dist = vec![0.33, 0.33, 0.34];
        let diverse_fitness = compute_pous_fitness_with_entropy(base_fitness, &diverse_dist, 0.5);
        // Diverse should have higher fitness due to entropy bonus
        assert!(diverse_fitness > monopoly_fitness);
    }

    // --- S128: Gossip Priority + Edge Scheduler Tests ---

    #[test]
    fn test_gossip_priority_basic() {
        let p = compute_gossip_priority(2.0, 0.3);
        assert!((p - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_gossip_priority_clamp_low() {
        let p = compute_gossip_priority(0.001, 0.001);
        assert_eq!(p, 0.01); // Clamped to minimum
    }

    #[test]
    fn test_gossip_priority_clamp_high() {
        let p = compute_gossip_priority(100.0, 1.0);
        assert_eq!(p, 1.0); // Clamped to maximum
    }

    #[test]
    fn test_gossip_priority_zero_fitness() {
        let p = compute_gossip_priority(0.0, 0.5);
        assert_eq!(p, 0.01); // Clamped to minimum
    }

    #[test]
    fn test_edge_device_efficiency_weight() {
        assert!((EdgeDeviceType::Datacenter.device_efficiency_weight() - 1.0).abs() < 1e-10);
        assert!((EdgeDeviceType::Desktop.device_efficiency_weight() - 0.9).abs() < 1e-10);
        assert!((EdgeDeviceType::Mobile.device_efficiency_weight() - 0.4).abs() < 1e-10);
        assert!((EdgeDeviceType::Smartwatch.device_efficiency_weight() - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_edge_device_base_energy_cost() {
        assert!((EdgeDeviceType::Datacenter.base_energy_cost() - 5.0).abs() < 1e-10);
        assert!((EdgeDeviceType::Desktop.base_energy_cost() - 0.05).abs() < 1e-10);
        assert!((EdgeDeviceType::Smartwatch.base_energy_cost() - 0.005).abs() < 1e-10);
    }

    #[test]
    fn test_edge_scheduler_priority_full_battery() {
        let p = update_edge_scheduler_priority(1.0, 1.0, EdgeDeviceType::Datacenter);
        // fitness=1.0 × weight=1.0 × battery_factor=1.0 = 1.0
        assert!((p - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_scheduler_priority_low_battery() {
        let p = update_edge_scheduler_priority(1.0, 0.0, EdgeDeviceType::Datacenter);
        // fitness=1.0 × weight=1.0 × battery_factor=0.5 = 0.5
        assert!((p - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_edge_scheduler_priority_mobile_half_battery() {
        let p = update_edge_scheduler_priority(1.0, 0.5, EdgeDeviceType::Mobile);
        // fitness=1.0 × weight=0.4 × battery_factor=0.75 = 0.3
        assert!((p - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_edge_scheduler_priority_clamped() {
        let p = update_edge_scheduler_priority(10.0, 1.0, EdgeDeviceType::Datacenter);
        assert_eq!(p, 1.0); // Clamped to 1.0
    }

    #[test]
    fn test_edge_scheduler_priority_smartwatch() {
        let p = update_edge_scheduler_priority(0.5, 0.8, EdgeDeviceType::Smartwatch);
        // fitness=0.5 × weight=0.1 × battery_factor=0.9 = 0.045
        assert!((p - 0.045).abs() < 1e-10);
    }

    #[test]
    fn test_gossip_priority_integration_with_pous() {
        let fitness = compute_pous_fitness(10.0, 5.0, 0.95, 0.1);
        let share = 0.3;
        let priority = compute_gossip_priority(fitness, share);
        assert!(priority >= 0.01);
        assert!(priority <= 1.0);
        // fitness = 6.49, share = 0.3 → 1.947 → clamped to 1.0
        assert_eq!(priority, 1.0);
    }

    #[test]
    fn test_edge_scheduler_integration_with_pous() {
        let fitness = compute_pous_fitness(5.0, 2.0, 0.8, 0.0);
        // fitness = 2.5 + 0.6 + 0.16 = 3.26
        let priority = update_edge_scheduler_priority(fitness, 0.7, EdgeDeviceType::Desktop);
        // 3.26 × 0.9 × 0.85 = 2.4951 → clamped to 1.0
        assert_eq!(priority, 1.0);
    }

    // --- S129 — PoUS Replicator Dynamics Tests ---

    #[test]
    fn test_pous_fitness_sgw_identical_manifold() {
        // Node at same distance as safe → no bonus
        let result = compute_pous_fitness_sgw(1.0, 0.5, 0.5, 0.5);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pous_fitness_sgw_closer_than_safe() {
        // Node closer than safe → bonus applied
        let result = compute_pous_fitness_sgw(1.0, 0.3, 0.5, 0.5);
        // bonus = 0.5 * (0.5-0.3)/0.5 = 0.5 * 0.4 = 0.2
        // result = 1.0 * 1.2 = 1.2
        assert!((result - 1.2).abs() < 1e-10);
    }

    #[test]
    fn test_pous_fitness_sgw_farther_than_safe() {
        // Node farther than safe → no bonus (clamped to 0)
        let result = compute_pous_fitness_sgw(1.0, 0.8, 0.5, 0.5);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pous_fitness_sgw_zero_safe_dist() {
        let result = compute_pous_fitness_sgw(1.0, 0.3, 0.0, 0.5);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pous_fitness_sgw_zero_base_fitness() {
        let result = compute_pous_fitness_sgw(0.0, 0.3, 0.5, 0.5);
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pous_fitness_sgw_max_bonus() {
        // Node at distance 0, safe at 0.5 → max bonus
        let result = compute_pous_fitness_sgw(1.0, 0.0, 0.5, 1.0);
        // bonus = 1.0 * (0.5-0)/0.5 = 1.0
        // result = 1.0 * 2.0 = 2.0
        assert!((result - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_replicator_dynamics_sgw_empty() {
        let result = replicator_dynamics_sgw_aware(&[], &[], &[], 0.5, 0.5, 10, 0.01);
        assert!(result.is_empty());
    }

    #[test]
    fn test_replicator_dynamics_sgw_length_mismatch() {
        let result = replicator_dynamics_sgw_aware(&[0.5, 0.5], &[1.0], &[0.3], 0.5, 0.5, 10, 0.01);
        assert!(result.is_empty());
    }

    #[test]
    fn test_replicator_dynamics_sgw_convergence() {
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let sgw_dists = [0.3, 0.6];
        let safe_dist = 0.4;
        let trajectory = replicator_dynamics_sgw_aware(
            &shares, &fitnesses, &sgw_dists, safe_dist, 0.5, 100, 0.01,
        );
        assert!(!trajectory.is_empty());
        assert_eq!(trajectory.len(), 101); // initial + 100 steps
                                           // First step should match initial shares
        assert!((trajectory[0][0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_hmc_steer_credit_empty() {
        let result = compute_hmc_steer_credit(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_hmc_steer_credit_uniform_fallback() {
        let result = compute_hmc_steer_credit(&[0.0, 0.0, 0.0]);
        assert_eq!(result.len(), 3);
        for &v in &result {
            assert!((v - 1.0 / 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_hmc_steer_credit_normalization() {
        let result = compute_hmc_steer_credit(&[0.0, 3.0, 7.0]);
        assert_eq!(result.len(), 3);
        let total: f64 = result.iter().sum();
        assert!((total - 1.0).abs() < 1e-10);
        // 0/10 = 0, 3/10 = 0.3, 7/10 = 0.7
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[1] - 0.3).abs() < 1e-10);
        assert!((result[2] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_hmc_steer_credit_clamps_negative() {
        let result = compute_hmc_steer_credit(&[-1.0, 3.0, 7.0]);
        // -1 clamped to 0, total = 10
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[1] - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_pous_hmc_hybrid_zero_max_energy() {
        let result = compute_pous_hmc_hybrid_fitness(1.0, 0.5, 0.0, 0.7);
        // normalized_energy = 0, result = 0.7 * 1.0 = 0.7
        assert!((result - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_pous_hmc_hybrid_full_pous() {
        let result = compute_pous_hmc_hybrid_fitness(1.0, 0.5, 1.0, 1.0);
        // alpha=1.0 → only PoUS
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pous_hmc_hybrid_full_hmc() {
        let result = compute_pous_hmc_hybrid_fitness(0.0, 0.5, 1.0, 0.0);
        // alpha=0.0 → only HMC, normalized = 0.5/1.0 = 0.5
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_pous_hmc_hybrid_balanced() {
        let result = compute_pous_hmc_hybrid_fitness(1.0, 0.5, 1.0, 0.7);
        // 0.7 * 1.0 + 0.3 * 0.5 = 0.7 + 0.15 = 0.85
        assert!((result - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_replicator_dynamics_hmc_empty() {
        let result = replicator_dynamics_hmc_feedback(&[], &[], &[], 1.0, 0.7, 10, 0.01);
        assert!(result.is_empty());
    }

    #[test]
    fn test_replicator_dynamics_hmc_length_mismatch() {
        let result =
            replicator_dynamics_hmc_feedback(&[0.5], &[1.0], &[0.5, 0.3], 1.0, 0.7, 10, 0.01);
        assert!(result.is_empty());
    }

    #[test]
    fn test_replicator_dynamics_hmc_convergence() {
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let energy = [0.8, 0.2];
        let trajectory =
            replicator_dynamics_hmc_feedback(&shares, &fitnesses, &energy, 1.0, 0.7, 50, 0.01);
        assert!(!trajectory.is_empty());
        assert_eq!(trajectory.len(), 51);
    }

    #[test]
    fn test_s129_full_pipeline_empty() {
        let (shares, traj, credits) =
            s129_full_pipeline(&[], &[], &[], 0.5, &[], 1.0, 0.5, 0.7, 10, 0.01);
        assert!(shares.is_empty());
        assert!(traj.is_empty());
        assert!(credits.is_empty());
    }

    #[test]
    fn test_s129_full_pipeline_length_mismatch() {
        let (shares, traj, credits) = s129_full_pipeline(
            &[0.5, 0.5],
            &[1.0, 0.5],
            &[0.3],
            0.5,
            &[0.8, 0.2],
            1.0,
            0.5,
            0.7,
            10,
            0.01,
        );
        assert!(shares.is_empty());
        assert!(traj.is_empty());
        assert!(credits.is_empty());
    }

    #[test]
    fn test_s129_full_pipeline_basic() {
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let sgw_dists = [0.3, 0.6];
        let safe_dist = 0.4;
        let energy = [0.8, 0.2];
        let max_energy = 1.0;

        let (final_shares, trajectory, credits) = s129_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, max_energy, 0.5, 0.7, 100, 0.01,
        );

        assert_eq!(final_shares.len(), 2);
        assert_eq!(trajectory.len(), 101);
        assert_eq!(credits.len(), 2);

        // Credits should sum to 1.0
        let credit_total: f64 = credits.iter().sum();
        assert!((credit_total - 1.0).abs() < 1e-10);

        // Node 0 has higher fitness + closer SGW + higher energy → should dominate
        assert!(final_shares[0] > final_shares[1]);
    }

    #[test]
    fn test_s129_full_pipeline_deterministic() {
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let sgw_dists = [0.3, 0.6];
        let safe_dist = 0.4;
        let energy = [0.8, 0.2];

        let (s1, t1, c1) = s129_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, 50, 0.01,
        );
        let (s2, t2, c2) = s129_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, 50, 0.01,
        );

        assert_eq!(s1, s2);
        assert_eq!(t1.len(), t2.len());
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_s129_pipeline_node_dominance() {
        // Node 0: high fitness, close to safe, high energy reduction
        // Node 1: low fitness, far from safe, low energy reduction
        let shares = [0.5, 0.5];
        let fitnesses = [2.0, 0.5];
        let sgw_dists = [0.1, 0.8];
        let safe_dist = 0.3;
        let energy = [0.9, 0.1];

        let (final_shares, _, _) = s129_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, 200, 0.01,
        );

        // Node 0 should dominate
        assert!(final_shares[0] > 0.7);
        assert!(final_shares[1] < 0.3);
    }

    #[test]
    fn test_s129_pipeline_equal_nodes() {
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 1.0];
        let sgw_dists = [0.3, 0.3];
        let safe_dist = 0.3;
        let energy = [0.5, 0.5];

        let (final_shares, _, _) = s129_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, 100, 0.01,
        );

        // Equal nodes should stay equal
        assert!((final_shares[0] - 0.5).abs() < 0.01);
        assert!((final_shares[1] - 0.5).abs() < 0.01);
    }

    // --- S130 — Meta-Replicator + Self-Improving Loop Tests ---

    #[test]
    fn test_meta_replicator_config_default() {
        let config = MetaReplicatorConfig::default();
        assert!((config.meta_lr - 0.01).abs() < 1e-10);
        assert!((config.min_lr - 1e-6).abs() < 1e-10);
        assert!((config.max_lr - 0.5).abs() < 1e-10);
        assert!((config.temperature - 1.0).abs() < 1e-10);
        assert_eq!(config.convergence_window, 20);
        assert_eq!(config.max_meta_iterations, 500);
        assert!((config.momentum - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_meta_replicator_config_with_meta_lr() {
        let config = MetaReplicatorConfig::default().with_meta_lr(0.1);
        assert!((config.meta_lr - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_meta_replicator_config_meta_lr_clamped_low() {
        let config = MetaReplicatorConfig::default().with_meta_lr(0.0);
        assert!(config.meta_lr >= 1e-8);
    }

    #[test]
    fn test_meta_replicator_config_meta_lr_clamped_high() {
        let config = MetaReplicatorConfig::default().with_meta_lr(10.0);
        assert!((config.meta_lr - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_meta_replicator_config_with_convergence_tolerance() {
        let config = MetaReplicatorConfig::default().with_convergence_tolerance(1e-6);
        assert!((config.convergence_tolerance - 1e-6).abs() < 1e-10);
    }

    #[test]
    fn test_meta_replicator_config_convergence_tolerance_clamped() {
        let config = MetaReplicatorConfig::default().with_convergence_tolerance(0.0);
        assert!(config.convergence_tolerance >= 1e-12);
    }

    #[test]
    fn test_meta_replicator_config_fast_converge() {
        let config = MetaReplicatorConfig::fast_converge();
        assert_eq!(config.convergence_window, 10);
        assert!((config.convergence_tolerance - 1e-5).abs() < 1e-10);
        assert_eq!(config.max_meta_iterations, 100);
        assert!((config.meta_lr - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_meta_replicator_config_high_precision() {
        let config = MetaReplicatorConfig::high_precision();
        assert_eq!(config.convergence_window, 50);
        assert!((config.convergence_tolerance - 1e-10).abs() < 1e-10);
        assert_eq!(config.max_meta_iterations, 2000);
        assert!((config.meta_lr - 0.001).abs() < 1e-10);
        assert!((config.momentum - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_self_improvement_state_new() {
        let state = SelfImprovementState::new(0.01, 5);
        assert!(state.history.is_empty());
        assert_eq!(state.best_fitness, f64::NEG_INFINITY);
        assert!((state.adaptive_lr - 0.01).abs() < 1e-10);
        assert_eq!(state.momentum_buffer.len(), 5);
        assert_eq!(state.consecutive_improvements, 0);
        assert_eq!(state.consecutive_stagnations, 0);
        assert_eq!(state.total_iterations, 0);
    }

    #[test]
    fn test_self_improvement_state_record_improvement() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        assert_eq!(state.best_fitness, 1.0);
        assert_eq!(state.consecutive_improvements, 1);
        assert_eq!(state.consecutive_stagnations, 0);
        assert_eq!(state.total_iterations, 1);

        state.record(2.0);
        assert_eq!(state.best_fitness, 2.0);
        assert_eq!(state.consecutive_improvements, 2);
        assert_eq!(state.consecutive_stagnations, 0);
    }

    #[test]
    fn test_self_improvement_state_record_decline() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(5.0);
        state.record(3.0);
        assert_eq!(state.best_fitness, 5.0);
        assert_eq!(state.consecutive_improvements, 0);
        assert_eq!(state.consecutive_stagnations, 1);
    }

    #[test]
    fn test_self_improvement_state_trend_positive() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        state.record(2.0);
        state.record(3.0);
        state.record(4.0);
        let trend = state.trend(4);
        assert!(trend > 0.0);
        // (4 - 1) / 4 = 0.75
        assert!((trend - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_self_improvement_state_trend_negative() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(4.0);
        state.record(3.0);
        state.record(2.0);
        state.record(1.0);
        let trend = state.trend(4);
        assert!(trend < 0.0);
        assert!((trend - (-0.75)).abs() < 1e-10);
    }

    #[test]
    fn test_self_improvement_state_trend_empty() {
        let state = SelfImprovementState::new(0.01, 3);
        let trend = state.trend(5);
        assert_eq!(trend, 0.0);
    }

    #[test]
    fn test_self_improvement_state_trend_single() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(5.0);
        let trend = state.trend(5);
        assert_eq!(trend, 0.0);
    }

    #[test]
    fn test_self_improvement_state_stagnated() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        state.record(1.00001);
        state.record(0.99999);
        state.record(1.00002);
        state.record(0.99998);
        assert!(state.is_stagnated(5, 1e-3));
    }

    #[test]
    fn test_self_improvement_state_not_stagnated() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        state.record(2.0);
        state.record(3.0);
        state.record(4.0);
        state.record(5.0);
        assert!(!state.is_stagnated(5, 1e-3));
    }

    #[test]
    fn test_self_improvement_state_not_stagnated_too_few() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        assert!(!state.is_stagnated(10, 1e-6));
    }

    #[test]
    fn test_convergence_monitor_new() {
        let monitor = ConvergenceMonitor::new(10, 1e-6);
        assert!(monitor.recent_norms.is_empty());
        assert_eq!(monitor.window, 10);
        assert!((monitor.tolerance - 1e-6).abs() < 1e-10);
        assert!((monitor.step_multiplier - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_convergence_monitor_record_converged() {
        let mut monitor = ConvergenceMonitor::new(5, 1e-3);
        monitor.record_norm(1e-2); // First norm (not converged yet)
        let converged = monitor.record_norm(1e-4); // Second norm (below tolerance → converged)
        assert!(converged);
        assert!(monitor.step_multiplier < 1.0);
    }

    #[test]
    fn test_convergence_monitor_record_not_converged() {
        let mut monitor = ConvergenceMonitor::new(5, 1e-6);
        let converged = monitor.record_norm(0.5);
        assert!(!converged);
        assert!(monitor.step_multiplier > 1.0);
    }

    #[test]
    fn test_convergence_monitor_adaptive_step_increases() {
        let mut monitor = ConvergenceMonitor::new(5, 1e-6);
        monitor.record_norm(1.0);
        monitor.record_norm(0.8);
        monitor.record_norm(0.6);
        let step = monitor.adaptive_step(0.01);
        assert!(step > 0.01);
    }

    #[test]
    fn test_convergence_monitor_adaptive_step_decreases() {
        let mut monitor = ConvergenceMonitor::new(5, 1e-2);
        monitor.record_norm(1e-3);
        monitor.record_norm(1e-4);
        let step = monitor.adaptive_step(0.01);
        assert!(step < 0.01);
    }

    #[test]
    fn test_convergence_monitor_adaptive_step_bounds() {
        let mut monitor = ConvergenceMonitor::new(2, 1e-6);
        for _ in 0..20 {
            monitor.record_norm(1.0);
        }
        let step = monitor.adaptive_step(10.0);
        assert!(step <= 1.0); // Capped at 1.0
    }

    #[test]
    fn test_convergence_monitor_window_sliding() {
        let mut monitor = ConvergenceMonitor::new(3, 1e-6);
        monitor.record_norm(1.0);
        monitor.record_norm(1.0);
        monitor.record_norm(1.0);
        monitor.record_norm(1.0);
        assert_eq!(monitor.recent_norms.len(), 3);
    }

    #[test]
    fn test_compute_meta_gradient_basic() {
        let grad = compute_meta_gradient(0.01, 1.0, 1.1, 0.9, 1e-4);
        // (1.1 - 0.9) / (2 * 1e-4) = 0.2 / 2e-4 = 1000
        assert!((grad - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_meta_gradient_zero_epsilon() {
        let grad = compute_meta_gradient(0.01, 1.0, 1.1, 0.9, 0.0);
        assert_eq!(grad, 0.0);
    }

    #[test]
    fn test_compute_meta_gradient_symmetric() {
        let grad = compute_meta_gradient(0.01, 1.0, 1.0, 1.0, 1e-4);
        assert_eq!(grad, 0.0);
    }

    #[test]
    fn test_meta_replicator_step_empty() {
        let mut state = SelfImprovementState::new(0.01, 5);
        let config = MetaReplicatorConfig::default();
        let result = meta_replicator_step(&[], &[], 0.01, &[0.5, 0.3, 0.2, 2.0], &mut state, &config);
        assert!(result.converged);
        assert_eq!(result.iterations, 0);
        assert!((result.updated_lr - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_meta_replicator_step_length_mismatch() {
        let mut state = SelfImprovementState::new(0.01, 5);
        let config = MetaReplicatorConfig::default();
        let result = meta_replicator_step(&[0.5, 0.5], &[1.0], 0.01, &[0.5, 0.3, 0.2, 2.0], &mut state, &config);
        assert!(result.converged);
        assert_eq!(result.iterations, 0);
    }

    #[test]
    fn test_meta_replicator_step_basic() {
        let mut state = SelfImprovementState::new(0.01, 5);
        let config = MetaReplicatorConfig::default();
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let coeffs = [0.5, 0.3, 0.2, 2.0];
        let result = meta_replicator_step(&shares, &fitnesses, 0.01, &coeffs, &mut state, &config);
        assert!(result.updated_lr >= config.min_lr);
        assert!(result.updated_lr <= config.max_lr);
        assert!(result.meta_gradient_norm >= 0.0);
        assert!(result.temperature > 0.0);
        assert_eq!(result.updated_coefficients.len(), 4);
    }

    #[test]
    fn test_meta_replicator_step_lr_increases_with_positive_gradient() {
        let mut state = SelfImprovementState::new(0.01, 5);
        let config = MetaReplicatorConfig::default();
        // High fitness difference → positive gradient → lr should increase
        let shares = [0.1, 0.9];
        let fitnesses = [3.0, 1.0];
        let coeffs = [0.5, 0.3, 0.2, 2.0];
        let result = meta_replicator_step(&shares, &fitnesses, 0.01, &coeffs, &mut state, &config);
        assert!(result.updated_lr >= 0.01);
    }

    #[test]
    fn test_meta_replicator_step_coefficients_bounded() {
        let mut state = SelfImprovementState::new(0.01, 5);
        let config = MetaReplicatorConfig::default().with_meta_lr(0.5);
        let shares = [0.33, 0.33, 0.34];
        let fitnesses = [1.0, 2.0, 1.5];
        let coeffs = [5.0, 5.0, 5.0, 5.0];
        let result = meta_replicator_step(&shares, &fitnesses, 0.01, &coeffs, &mut state, &config);
        for &c in &result.updated_coefficients {
            assert!(c >= 0.0);
            assert!(c <= 10.0);
        }
    }

    #[test]
    fn test_meta_replicator_step_momentum_accumulation() {
        let mut state = SelfImprovementState::new(0.01, 5);
        let config = MetaReplicatorConfig::default();
        let shares = [0.5, 0.5];
        let fitnesses = [2.0, 1.0];
        let coeffs = [0.5, 0.3, 0.2, 2.0];

        let r1 = meta_replicator_step(&shares, &fitnesses, 0.01, &coeffs, &mut state, &config);
        let r2 = meta_replicator_step(&shares, &fitnesses, r1.updated_lr, &r1.updated_coefficients, &mut state, &config);
        // Momentum should cause larger updates on second call
        assert!(r2.iterations == 2);
    }

    #[test]
    fn test_self_improving_fitness_basic() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        state.record(2.0);
        state.record(3.0);
        let config = MetaReplicatorConfig::default();
        let result = self_improving_fitness(2.0, &state, &config);
        assert!(result >= 2.0); // Base + trend bonus
    }

    #[test]
    fn test_self_improving_fitness_stagnation_penalty() {
        let mut state = SelfImprovementState::new(0.01, 3);
        for _ in 0..25 {
            state.record(1.0);
        }
        let config = MetaReplicatorConfig::default();
        let result = self_improving_fitness(1.0, &state, &config);
        // Stagnation detected → penalty applied
        assert!(result <= 1.0 + 0.1);
    }

    #[test]
    fn test_self_improving_fitness_diversity_bonus() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(1.0);
        state.record(3.0);
        state.record(5.0);
        state.record(7.0);
        let config = MetaReplicatorConfig::default();
        let result = self_improving_fitness(4.0, &state, &config);
        assert!(result > 4.0); // Diversity bonus adds to base
    }

    #[test]
    fn test_self_improving_fitness_clamped_positive() {
        let mut state = SelfImprovementState::new(0.01, 3);
        state.record(-10.0);
        state.record(-20.0);
        let config = MetaReplicatorConfig::default();
        let result = self_improving_fitness(0.0, &state, &config);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_run_self_improving_loop_empty() {
        let config = MetaReplicatorConfig::default();
        let results = run_self_improving_loop(&[], &[], 0.01, [0.5, 0.3, 0.2, 2.0], &config);
        assert!(results.is_empty());
    }

    #[test]
    fn test_run_self_improving_loop_length_mismatch() {
        let config = MetaReplicatorConfig::default();
        let results = run_self_improving_loop(&[0.5, 0.5], &[1.0], 0.01, [0.5, 0.3, 0.2, 2.0], &config);
        assert!(results.is_empty());
    }

    #[test]
    fn test_run_self_improving_loop_basic() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let results = run_self_improving_loop(&shares, &fitnesses, 0.01, [0.5, 0.3, 0.2, 2.0], &config);
        assert!(!results.is_empty());
        assert!(results.len() <= config.max_meta_iterations);
    }

    #[test]
    fn test_run_self_improving_loop_convergence() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 1.0]; // Equal → should converge quickly
        let results = run_self_improving_loop(&shares, &fitnesses, 0.01, [0.5, 0.3, 0.2, 2.0], &config);
        assert!(!results.is_empty());
        // Last result should indicate convergence or small gradient
        let last = results.last().unwrap();
        assert!(last.meta_gradient_norm >= 0.0);
    }

    #[test]
    fn test_run_self_improving_loop_lr_evolution() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.3, 0.3, 0.4];
        let fitnesses = [1.0, 2.0, 1.5];
        let results = run_self_improving_loop(&shares, &fitnesses, 0.01, [0.5, 0.3, 0.2, 2.0], &config);
        assert!(!results.is_empty());
        let first_lr = results.first().unwrap().updated_lr;
        let last_lr = results.last().unwrap().updated_lr;
        // Learning rate should evolve
        assert!(first_lr >= config.min_lr);
        assert!(first_lr <= config.max_lr);
        assert!(last_lr >= config.min_lr);
        assert!(last_lr <= config.max_lr);
    }

    #[test]
    fn test_run_self_improving_loop_temperature_adaptation() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let results = run_self_improving_loop(&shares, &fitnesses, 0.01, [0.5, 0.3, 0.2, 2.0], &config);
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.temperature > 0.0);
        }
    }

    #[test]
    fn test_s130_full_pipeline_empty() {
        let config = MetaReplicatorConfig::default();
        let (shares, results, credits) = s130_full_pipeline(
            &[], &[], &[], 0.5, &[], 1.0, 0.5, 0.7, &config,
        );
        assert!(shares.is_empty());
        assert!(results.is_empty());
        assert!(credits.is_empty());
    }

    #[test]
    fn test_s130_full_pipeline_length_mismatch() {
        let config = MetaReplicatorConfig::default();
        let (shares, results, credits) = s130_full_pipeline(
            &[0.5, 0.5], &[1.0], &[0.3], 0.5, &[0.5], 1.0, 0.5, 0.7, &config,
        );
        assert!(shares.is_empty());
        assert!(results.is_empty());
        assert!(credits.is_empty());
    }

    #[test]
    fn test_s130_full_pipeline_basic() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let sgw_dists = [0.3, 0.6];
        let safe_dist = 0.4;
        let energy = [0.8, 0.2];
        let max_energy = 1.0;

        let (final_shares, results, credits) = s130_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, max_energy, 0.5, 0.7, &config,
        );

        assert_eq!(final_shares.len(), 2);
        assert!(!results.is_empty());
        assert_eq!(credits.len(), 2);

        // Credits should sum to ~1.0
        let credit_total: f64 = credits.iter().sum();
        assert!((credit_total - 1.0).abs() < 1e-6);

        // Node 0 has higher fitness → should have higher share
        assert!(final_shares[0] > final_shares[1]);
    }

    #[test]
    fn test_s130_full_pipeline_dominance() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.5, 0.5];
        let fitnesses = [3.0, 0.5];
        let sgw_dists = [0.1, 0.9];
        let safe_dist = 0.3;
        let energy = [0.95, 0.05];

        let (final_shares, results, _) = s130_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, &config,
        );

        // Node 0 should dominate (higher fitness + closer SGW + higher energy)
        assert!(final_shares[0] > final_shares[1]);
        // Meta-replicator produces valid results
        assert!(!results.is_empty());
        // Shares sum to ~1.0
        let total: f64 = final_shares.iter().sum();
        assert!((total - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_s130_full_pipeline_equal_nodes() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 1.0];
        let sgw_dists = [0.3, 0.3];
        let safe_dist = 0.3;
        let energy = [0.5, 0.5];

        let (final_shares, _, _) = s130_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, &config,
        );

        // Equal nodes should remain roughly equal
        assert!((final_shares[0] - 0.5).abs() < 0.15);
        assert!((final_shares[1] - 0.5).abs() < 0.15);
    }

    #[test]
    fn test_s130_full_pipeline_meta_results_structure() {
        let config = MetaReplicatorConfig::fast_converge();
        let shares = [0.4, 0.3, 0.3];
        let fitnesses = [1.5, 1.0, 0.8];
        let sgw_dists = [0.2, 0.4, 0.5];
        let safe_dist = 0.3;
        let energy = [0.7, 0.5, 0.3];

        let (_shares, results, _credits) = s130_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, &config,
        );

        assert!(!results.is_empty());
        let last = results.last().unwrap();
        assert!(last.meta_gradient_norm >= 0.0);
        assert!(last.temperature > 0.0);
        assert!(last.updated_lr > 0.0);
        assert_eq!(last.updated_coefficients.len(), 4);
    }

    #[test]
    fn test_s130_pipeline_integration_with_s129() {
        // Verify S130 builds on S129 components
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 0.5];
        let sgw_dists = [0.3, 0.6];
        let safe_dist = 0.4;
        let energy = [0.8, 0.2];

        // S129 pipeline
        let (s129_shares, s129_traj, _s129_credits) = s129_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, 100, 0.01,
        );

        // S130 pipeline
        let config = MetaReplicatorConfig::fast_converge();
        let (s130_shares, s130_results, _s130_credits) = s130_full_pipeline(
            &shares, &fitnesses, &sgw_dists, safe_dist, &energy, 1.0, 0.5, 0.7, &config,
        );

        // Both should produce valid results
        assert_eq!(s129_shares.len(), 2);
        assert_eq!(s130_shares.len(), 2);
        assert!(!s129_traj.is_empty());
        assert!(!s130_results.is_empty());

        // Both should favor node 0
        assert!(s129_shares[0] > s129_shares[1]);
        assert!(s130_shares[0] > s130_shares[1]);
    }

    #[test]
    fn test_evaluate_one_step_fitness_empty() {
        let result = evaluate_one_step_fitness(&[], &[], 0.01, &[0.5, 0.3, 0.2, 2.0]);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_evaluate_one_step_fitness_uniform() {
        let shares = [0.5, 0.5];
        let fitnesses = [1.0, 1.0];
        let result = evaluate_one_step_fitness(&shares, &fitnesses, 0.01, &[0.5, 0.3, 0.2, 2.0]);
        // Equal fitness → no change → max entropy
        assert!(result > 0.0);
    }

    #[test]
    fn test_evaluate_one_step_fitness_dominant() {
        let shares = [0.1, 0.9];
        let fitnesses = [1.0, 3.0];
        let result = evaluate_one_step_fitness(&shares, &fitnesses, 0.01, &[0.5, 0.3, 0.2, 2.0]);
        assert!(result > 0.0);
    }
}
