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
        let contributions = vec![3.0, 5.0, 2.0];
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
}
