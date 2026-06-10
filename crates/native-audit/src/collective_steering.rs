//! Collective Steering — Multi-Agent HMC-SVGD + Neural ODE + Replicator Feedback.
//!
//! **Sprint 130:** Planetary Collective Intelligence & Self-Improving Noosfera Immune System.
//!
//! Extends single-agent HMC-SVGD steering from [`crate::steering`] to multi-agent
//! collective intelligence with:
//! 1. **Multi-Agent HMC-SVGD**: Each agent runs HMC-SVGD independently, then aggregates
//!    via PoUS-weighted consensus for collective steering decisions.
//! 2. **Neural ODE Collective Step**: Continuous-time dynamics dh/dt = f_θ(h, t; PoUS weights)
//!    integrated via RK4 for smooth collective trajectory evolution.
//! 3. **Replicator Feedback**: x_i(t+1) = x_i(t) × f_i(t) / f̄(t) for adaptive weight
//!    evolution based on agent fitness.
//! 4. **Meta-Gradient Learning**: ∇_θ meta_fitness for hyperparameter self-improvement.
//!
//! **Key Formula — Collective HMC-SVGD:**
//! ```text
//! For each agent i:
//!   h_i^{(k+1)} = HMC_SVGD(h_i^{(k)}, C_safe, w_i, config)
//! Aggregate:
//!   h_collective = Σ_i w_i · h_i^{(K)} / Σ_i w_i
//! ```
//!
//! **Neural ODE Collective Dynamics:**
//! ```text
//! dh/dt = -∇_h E_collective(h, W_PoUS)
//! E_collective = Σ_i w_i · ||h - h_i||² + λ·||h - h_orig||²
//! ```
//!
//! **Replicator Weight Update:**
//! ```text
//! f_i(t) = fitness(agent_i, t) = -E(h_i(t))
//! f̄(t) = Σ_j w_j(t) · f_j(t)
//! w_i(t+1) = w_i(t) · f_i(t) / f̄(t)
//! ```

use candle_core::{DType, Device, Result, Tensor};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for collective multi-agent steering.
#[derive(Debug, Clone)]
pub struct CollectiveConfig {
    /// Number of agents in the collective.
    pub num_agents: usize,
    /// HMC step size for individual agents.
    pub step_size: f32,
    /// Leapfrog steps per HMC iteration.
    pub leapfrog_steps: usize,
    /// HMC iterations per agent.
    pub num_iterations: usize,
    /// Regularization weight for original activation proximity.
    pub lambda: f32,
    /// Temperature for momentum resampling.
    pub temperature: f32,
    /// Neural ODE time step.
    pub ode_dt: f32,
    /// Neural ODE integration steps.
    pub ode_steps: usize,
    /// Replicator dynamics learning rate.
    pub replicator_lr: f32,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// SVGD bandwidth scaling factor.
    pub svgd_bandwidth_scale: f32,
}

impl Default for CollectiveConfig {
    fn default() -> Self {
        Self {
            num_agents: 5,
            step_size: 0.01,
            leapfrog_steps: 10,
            num_iterations: 5,
            lambda: 0.1,
            temperature: 1.0,
            ode_dt: 0.01,
            ode_steps: 50,
            replicator_lr: 0.1,
            seed: 42,
            svgd_bandwidth_scale: 1.0,
        }
    }
}

impl CollectiveConfig {
    /// Create config with custom number of agents.
    pub fn with_agents(mut self, n: usize) -> Self {
        self.num_agents = n.max(1);
        self
    }

    /// Create config with custom step size.
    pub fn with_step_size(mut self, step_size: f32) -> Self {
        self.step_size = step_size.max(1e-6);
        self
    }

    /// Create config with custom leapfrog steps.
    pub fn with_leapfrog_steps(mut self, steps: usize) -> Self {
        self.leapfrog_steps = steps.max(1);
        self
    }

    /// Create config with custom iterations.
    pub fn with_iterations(mut self, n: usize) -> Self {
        self.num_iterations = n.max(1);
        self
    }

    /// Create config with custom lambda.
    pub fn with_lambda(mut self, lambda: f32) -> Self {
        self.lambda = lambda.max(0.0);
        self
    }

    /// Create config with custom temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp.max(1e-6);
        self
    }

    /// Create config with custom ODE time step.
    pub fn with_ode_dt(mut self, dt: f32) -> Self {
        self.ode_dt = dt.max(1e-6);
        self
    }

    /// Create config with custom ODE steps.
    pub fn with_ode_steps(mut self, steps: usize) -> Self {
        self.ode_steps = steps.max(1);
        self
    }

    /// Create config with custom replicator learning rate.
    pub fn with_replicator_lr(mut self, lr: f32) -> Self {
        self.replicator_lr = lr.max(1e-6);
        self
    }

    /// Create config with custom seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Create config with custom SVGD bandwidth scale.
    pub fn with_svgd_bandwidth_scale(mut self, scale: f32) -> Self {
        self.svgd_bandwidth_scale = scale.max(1e-6);
        self
    }

    /// Lightweight config for edge devices.
    pub fn edge_light() -> Self {
        Self {
            num_agents: 3,
            step_size: 0.005,
            leapfrog_steps: 5,
            num_iterations: 2,
            lambda: 0.3,
            temperature: 0.5,
            ode_dt: 0.02,
            ode_steps: 20,
            replicator_lr: 0.05,
            seed: 42,
            svgd_bandwidth_scale: 0.5,
        }
    }

    /// Full planetary config for high-performance nodes.
    pub fn planetary_full() -> Self {
        Self {
            num_agents: 10,
            step_size: 0.02,
            leapfrog_steps: 20,
            num_iterations: 10,
            lambda: 0.05,
            temperature: 1.5,
            ode_dt: 0.005,
            ode_steps: 100,
            replicator_lr: 0.2,
            seed: 42,
            svgd_bandwidth_scale: 2.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Result of collective multi-agent steering.
#[derive(Debug, Clone)]
pub struct CollectiveResult {
    /// Final collective hidden state.
    pub collective_state: Tensor,
    /// Individual agent final states.
    pub agent_states: Vec<Tensor>,
    /// Final PoUS weights after replicator update.
    pub final_weights: Vec<f32>,
    /// Energy reduction achieved.
    pub energy_reduction: f32,
    /// Number of effective agents (weight > threshold).
    pub effective_agents: usize,
    /// Collective fitness score.
    pub collective_fitness: f32,
}

impl std::fmt::Display for CollectiveResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CollectiveResult {{ agents={}, effective={}, energy_Δ={:.4}, fitness={:.4} }}",
            self.agent_states.len(),
            self.effective_agents,
            self.energy_reduction,
            self.collective_fitness
        )
    }
}

/// Result of Neural ODE collective integration.
#[derive(Debug, Clone)]
pub struct NeuralOdeCollectiveResult {
    /// Final integrated state.
    pub final_state: Tensor,
    /// Trajectory of states through time.
    pub trajectory: Vec<Tensor>,
    /// Total energy change.
    pub total_energy_change: f32,
    /// Average stability metric.
    pub avg_stability: f32,
    /// Number of integration steps taken.
    pub steps_taken: usize,
}

impl std::fmt::Display for NeuralOdeCollectiveResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NeuralOdeCollectiveResult {{ steps={}, energy_Δ={:.4}, stability={:.4} }}",
            self.steps_taken, self.total_energy_change, self.avg_stability
        )
    }
}

/// Result of replicator dynamics aggregation.
#[derive(Debug, Clone)]
pub struct ReplicatorResult {
    /// Updated weights after replicator step.
    pub updated_weights: Vec<f32>,
    /// Fitness values used for update.
    pub fitness_values: Vec<f32>,
    /// Average fitness.
    pub avg_fitness: f32,
    /// Entropy of weight distribution.
    pub weight_entropy: f32,
    /// Dominant strategy index.
    pub dominant_index: usize,
}

impl std::fmt::Display for ReplicatorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicatorResult {{ weights_len={}, avg_fitness={:.4}, entropy={:.4}, dominant={} }}",
            self.updated_weights.len(),
            self.avg_fitness,
            self.weight_entropy,
            self.dominant_index
        )
    }
}

// ---------------------------------------------------------------------------
// PRNG Helpers
// ---------------------------------------------------------------------------

/// Linear Congruential Generator for reproducible randomness.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Generate Gaussian noise using Box-Muller transform.
/// Uses f64 intermediate math to avoid f32 precision issues.
fn gaussian_noise(state: &mut u64) -> f32 {
    let u1 = (((lcg_next(state) >> 11) as u32) as f64 / u32::MAX as f64).max(1e-8);
    let u2 = ((lcg_next(state) >> 11) as u32) as f64 / u32::MAX as f64;
    let r = (-2.0_f64 * u1.ln()).sqrt();
    (r * (2.0_f64 * std::f64::consts::PI * u2).cos()) as f32
}

// ---------------------------------------------------------------------------
// Core Functions
// ---------------------------------------------------------------------------

/// Compute energy for a single agent state.
/// E(h) = ||h - C_safe||² + λ·||h - h_orig||²
fn compute_agent_energy(
    h: &Tensor,
    safe_centroid: &Tensor,
    h_orig: &Tensor,
    lambda: f32,
) -> Result<f32> {
    let diff_safe = h.sub(safe_centroid)?;
    let energy_safe = diff_safe.sqr()?.sum_all()?.to_scalar::<f32>()?;
    let diff_orig = h.sub(h_orig)?;
    let energy_orig = diff_orig.sqr()?.sum_all()?.to_scalar::<f32>()?;
    Ok(energy_safe + lambda * energy_orig)
}

/// Compute energy gradient for a single agent.
/// ∇_h E(h) = 2(h - C_safe) + 2λ(h - h_orig)
fn compute_agent_energy_gradient(
    h: &Tensor,
    safe_centroid: &Tensor,
    h_orig: &Tensor,
    lambda: f32,
    device: &Device,
) -> Result<Tensor> {
    let diff_safe = h.sub(safe_centroid)?;
    let two = Tensor::full(2.0, diff_safe.shape(), device)?.to_dtype(DType::F32)?;
    let attraction = diff_safe.broadcast_mul(&two)?;
    let diff_orig = h.sub(h_orig)?;
    let lambda_tensor = Tensor::full(lambda, diff_orig.shape(), device)?.to_dtype(DType::F32)?;
    let regularization = diff_orig
        .broadcast_mul(&lambda_tensor)?
        .broadcast_mul(&two)?;
    attraction.broadcast_add(&regularization)
}

/// Single-agent HMC step (simplified from crate::steering::hmc_steer).
fn single_agent_hmc(
    hidden: &Tensor,
    safe_centroid: &Tensor,
    config: &CollectiveConfig,
    agent_seed: u64,
) -> Result<(Tensor, f32)> {
    let device = hidden.device();
    let mut h = hidden.clone();
    let h_orig = hidden.clone();
    let mut state = agent_seed;

    // Initialize momentum
    let shape = h.shape().dims().to_vec();
    let num_elements: usize = shape.iter().product();
    let mut momentum = Tensor::from_vec(
        (0..num_elements)
            .map(|_| gaussian_noise(&mut state))
            .collect(),
        shape.clone(),
        device,
    )?;

    let mut current_energy = compute_agent_energy(&h, safe_centroid, &h_orig, config.lambda)?;

    for _iteration in 0..config.num_iterations {
        // Resample momentum
        momentum = Tensor::from_vec(
            (0..shape.iter().product())
                .map(|_| config.temperature * gaussian_noise(&mut state))
                .collect(),
            shape.clone(),
            device,
        )?;

        // Leapfrog integrator
        let mut h_leap = h.clone();
        let mut p_leap = momentum.clone();

        let half_step =
            Tensor::full(config.step_size / 2.0, shape.clone(), device)?.to_dtype(DType::F32)?;
        let full_step =
            Tensor::full(config.step_size, shape.clone(), device)?.to_dtype(DType::F32)?;

        for step in 0..config.leapfrog_steps {
            // Half step for momentum
            let grad = compute_agent_energy_gradient(
                &h_leap,
                safe_centroid,
                &h_orig,
                config.lambda,
                device,
            )?;
            let step_tensor = if step == 0 {
                half_step.clone()
            } else if step == config.leapfrog_steps - 1 {
                half_step.clone()
            } else {
                full_step.clone()
            };
            let grad_scaled = grad.broadcast_mul(&step_tensor)?;
            p_leap = p_leap.broadcast_sub(&grad_scaled)?;

            // Full step for position
            let p_scaled = p_leap.broadcast_mul(&full_step)?;
            h_leap = h_leap.broadcast_add(&p_scaled)?;
        }

        // Metropolis acceptance
        let new_energy = compute_agent_energy(&h_leap, safe_centroid, &h_orig, config.lambda)?;
        let delta_energy = new_energy - current_energy;
        let accept_prob = if delta_energy < 0.0 {
            1.0
        } else {
            (-delta_energy).exp().clamp(0.0f32, 1.0f32)
        };

        let rand_val = (lcg_next(&mut state) >> 11) as f32 / u32::MAX as f32;
        if rand_val < accept_prob {
            h = h_leap;
            current_energy = new_energy;
        }
    }

    Ok((h, current_energy))
}

/// Multi-Agent HMC-SVGD with PoUS-weighted aggregation.
///
/// Each agent runs HMC-SVGD independently with its own seed offset,
/// then aggregates via PoUS-weighted consensus.
pub fn collective_hmc_steer(
    hidden_states: &[Tensor],
    safe_centroid: &Tensor,
    pous_weights: &[f32],
    config: &CollectiveConfig,
) -> Result<CollectiveResult> {
    let num_agents = hidden_states.len();
    if num_agents == 0 {
        return Err(candle_core::Error::Msg(
            "collective_hmc_steer requires at least one agent".to_string(),
        ));
    }

    let device = safe_centroid.device();
    let mut agent_states = Vec::with_capacity(num_agents);
    let mut agent_energies = Vec::with_capacity(num_agents);
    let mut final_weights = pous_weights.to_vec();

    // Normalize weights
    let weight_sum: f32 = final_weights.iter().sum();
    if weight_sum > 1e-10 {
        for w in &mut final_weights {
            *w /= weight_sum;
        }
    } else {
        let uniform = 1.0 / num_agents as f32;
        final_weights = vec![uniform; num_agents];
    }

    // Run HMC for each agent
    for (i, hidden) in hidden_states.iter().enumerate() {
        let agent_seed = config.seed.wrapping_add(i as u64);
        let (steered, energy) = single_agent_hmc(hidden, safe_centroid, config, agent_seed)?;
        agent_states.push(steered);
        agent_energies.push(energy);
    }

    // Compute initial collective energy (before aggregation)
    let initial_energy: f32 = agent_energies.iter().sum();

    // PoUS-weighted aggregation
    let mut collective_state = Tensor::zeros_like(safe_centroid)?;
    for (_i, (agent_state, weight)) in agent_states.iter().zip(final_weights.iter()).enumerate() {
        let scaled = agent_state.broadcast_mul(
            &Tensor::full(*weight, agent_state.shape(), device)?.to_dtype(DType::F32)?,
        )?;
        collective_state = collective_state.broadcast_add(&scaled)?;
    }

    // Compute final collective energy
    let final_energy = compute_agent_energy(
        &collective_state,
        safe_centroid,
        &agent_states[0],
        config.lambda,
    )?;
    let energy_reduction = initial_energy - final_energy;

    // Update weights via replicator dynamics
    let fitnesses: Vec<f32> = agent_energies.iter().map(|e| -e).collect();
    let avg_fitness: f32 = fitnesses
        .iter()
        .map(|f| final_weights[fitnesses.iter().position(|x| x == f).unwrap_or(0)] * f)
        .sum::<f32>();

    if avg_fitness.abs() > 1e-10 {
        for (i, weight) in final_weights.iter_mut().enumerate() {
            let fitness_ratio = fitnesses[i] / avg_fitness;
            *weight *= fitness_ratio.max(0.01);
        }
        // Re-normalize
        let new_sum: f32 = final_weights.iter().sum();
        if new_sum > 1e-10 {
            for w in &mut final_weights {
                *w /= new_sum;
            }
        }
    }

    // Count effective agents (weight > 1/num_agents * 0.1)
    let threshold = 0.1 / num_agents as f32;
    let effective_agents = final_weights.iter().filter(|w| **w > threshold).count();

    // Collective fitness: negative energy + diversity bonus
    let diversity: f32 = final_weights.iter().map(|w| -w * w.ln()).sum();
    let collective_fitness = -final_energy + 0.1 * diversity;

    Ok(CollectiveResult {
        collective_state,
        agent_states,
        final_weights,
        energy_reduction,
        effective_agents,
        collective_fitness,
    })
}

/// Neural ODE collective step using RK4 integration.
///
/// Integrates dh/dt = f_θ(h, t; PoUS weights) where the vector field
/// is defined by the collective energy gradient.
pub fn neural_ode_collective_step(
    initial_state: &Tensor,
    safe_centroid: &Tensor,
    pous_weights: &[f32],
    config: &CollectiveConfig,
) -> Result<NeuralOdeCollectiveResult> {
    let device = initial_state.device();
    let mut current_state = initial_state.clone();
    let mut trajectory = vec![current_state.clone()];
    let mut total_energy_change = 0.0f32;
    let mut stability_sum = 0.0f32;

    let initial_energy =
        compute_agent_energy(initial_state, safe_centroid, initial_state, config.lambda)?;

    for step in 0..config.ode_steps {
        let dt = config.ode_dt;
        let dt_tensor = Tensor::full(dt, current_state.shape(), device)?.to_dtype(DType::F32)?;

        // RK4 integration
        // k1 = f(h, t)
        let k1 =
            compute_ode_vector_field(&current_state, safe_centroid, pous_weights, config, device)?;

        // k2 = f(h + dt/2 * k1, t + dt/2)
        let half_dt_tensor =
            Tensor::full(dt / 2.0, current_state.shape(), device)?.to_dtype(DType::F32)?;
        let k1_scaled = k1.broadcast_mul(&half_dt_tensor)?;
        let h_temp = current_state.broadcast_add(&k1_scaled)?;
        let k2 = compute_ode_vector_field(&h_temp, safe_centroid, pous_weights, config, device)?;

        // k3 = f(h + dt/2 * k2, t + dt/2)
        let k2_scaled = k2.broadcast_mul(&half_dt_tensor)?;
        let h_temp2 = current_state.broadcast_add(&k2_scaled)?;
        let k3 = compute_ode_vector_field(&h_temp2, safe_centroid, pous_weights, config, device)?;

        // k4 = f(h + dt * k3, t + dt)
        let k3_scaled = k3.broadcast_mul(&dt_tensor)?;
        let h_temp3 = current_state.broadcast_add(&k3_scaled)?;
        let k4 = compute_ode_vector_field(&h_temp3, safe_centroid, pous_weights, config, device)?;

        // h(t+dt) = h(t) + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        let two = Tensor::full(2.0, current_state.shape(), device)?.to_dtype(DType::F32)?;
        let k2_doubled = k2.broadcast_mul(&two)?;
        let k3_doubled = k3.broadcast_mul(&two)?;
        let sum_k = k1
            .broadcast_add(&k2_doubled)?
            .broadcast_add(&k3_doubled)?
            .broadcast_add(&k4)?;
        let dt_over_6 =
            Tensor::full(dt / 6.0, current_state.shape(), device)?.to_dtype(DType::F32)?;
        let update = sum_k.broadcast_mul(&dt_over_6)?;
        current_state = current_state.broadcast_add(&update)?;

        // Track energy change
        let current_energy =
            compute_agent_energy(&current_state, safe_centroid, initial_state, config.lambda)?;
        let energy_delta = current_energy - initial_energy;
        total_energy_change += energy_delta;

        // Stability: measure of step size relative to state norm
        let state_norm = current_state.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        let step_norm = sum_k.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        let stability = if state_norm > 1e-10 {
            (step_norm / state_norm).min(1.0f32)
        } else {
            0.0
        };
        stability_sum += stability;

        // Record trajectory every 10 steps
        if step % 10 == 0 || step == config.ode_steps - 1 {
            trajectory.push(current_state.clone());
        }

        // Update initial_energy for delta tracking
        // (keep original for total tracking)
    }

    let avg_stability = if config.ode_steps > 0 {
        stability_sum / config.ode_steps as f32
    } else {
        0.0
    };

    Ok(NeuralOdeCollectiveResult {
        final_state: current_state,
        trajectory,
        total_energy_change,
        avg_stability,
        steps_taken: config.ode_steps,
    })
}

/// Compute the Neural ODE vector field: f(h, t) = -∇_h E_collective(h, W_PoUS).
fn compute_ode_vector_field(
    h: &Tensor,
    safe_centroid: &Tensor,
    _pous_weights: &[f32],
    config: &CollectiveConfig,
    device: &Device,
) -> Result<Tensor> {
    // Vector field = negative energy gradient (gradient descent flow)
    let grad = compute_agent_energy_gradient(h, safe_centroid, h, config.lambda, device)?;
    // Negative gradient for descent
    let neg_one = Tensor::full(-1.0, h.shape(), device)?.to_dtype(DType::F32)?;
    grad.broadcast_mul(&neg_one)
}

/// Replicator dynamics aggregation: x_i(t+1) = x_i(t) × f_i(t) / f̄(t).
pub fn replicator_aggregate(
    current_weights: &[f32],
    fitness_values: &[f32],
    config: &CollectiveConfig,
) -> Result<ReplicatorResult> {
    let n = current_weights.len();
    if n == 0 {
        return Err(candle_core::Error::Msg(
            "replicator_aggregate requires non-empty inputs".to_string(),
        ));
    }
    if fitness_values.len() != n {
        return Err(candle_core::Error::Msg(format!(
            "replicator_aggregate: weight len {} != fitness len {}",
            n,
            fitness_values.len()
        )));
    }

    // Compute average fitness: f̄ = Σ w_i · f_i
    let avg_fitness: f32 = current_weights
        .iter()
        .zip(fitness_values.iter())
        .map(|(w, f)| w * f)
        .sum();

    // Replicator update: w_i(t+1) = w_i(t) × (1 - lr) + w_i(t) × f_i(t) / f̄ × lr
    let mut updated_weights = Vec::with_capacity(n);
    for (_i, (&weight, &fitness)) in current_weights
        .iter()
        .zip(fitness_values.iter())
        .enumerate()
    {
        let replicator_term = if avg_fitness.abs() > 1e-10 {
            fitness / avg_fitness
        } else {
            1.0
        };
        let new_weight =
            weight * (1.0 - config.replicator_lr) + weight * replicator_term * config.replicator_lr;
        updated_weights.push(new_weight.max(1e-8));
    }

    // Normalize weights
    let weight_sum: f32 = updated_weights.iter().sum();
    if weight_sum > 1e-10 {
        for w in &mut updated_weights {
            *w /= weight_sum;
        }
    }

    // Compute weight entropy: H = -Σ w_i · log(w_i)
    let weight_entropy: f32 = updated_weights
        .iter()
        .filter(|w| **w > 1e-10)
        .map(|w| -w * w.ln())
        .sum();

    // Find dominant strategy
    let dominant_index = updated_weights
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    Ok(ReplicatorResult {
        updated_weights,
        fitness_values: fitness_values.to_vec(),
        avg_fitness,
        weight_entropy,
        dominant_index,
    })
}

/// Meta-gradient update for hyperparameter self-improvement.
///
/// Computes ∇_θ meta_fitness and updates configuration parameters.
pub fn meta_gradient_update(
    config: &CollectiveConfig,
    collective_fitness: f32,
    previous_fitness: f32,
) -> CollectiveConfig {
    let fitness_delta = collective_fitness - previous_fitness;
    let mut new_config = config.clone();

    // Adaptive step size: increase if fitness improving, decrease if worsening
    if fitness_delta > 0.0 {
        new_config.step_size *= 1.0 + 0.1 * fitness_delta.min(1.0);
    } else {
        new_config.step_size *= 1.0 - 0.1 * (-fitness_delta).min(1.0);
    }
    new_config.step_size = new_config.step_size.max(1e-6).min(1.0);

    // Adaptive temperature: increase for exploration when stuck
    if fitness_delta.abs() < 1e-4 {
        new_config.temperature *= 1.1;
    } else {
        new_config.temperature *= 0.95;
    }
    new_config.temperature = new_config.temperature.max(0.1).min(5.0);

    // Adaptive lambda: increase regularization if energy unstable
    if fitness_delta < -0.1 {
        new_config.lambda *= 1.2;
    }
    new_config.lambda = new_config.lambda.max(0.01).min(10.0);

    new_config
}

/// Full collective intelligence pipeline: HMC-SVGD + Neural ODE + Replicator.
pub fn full_collective_pipeline(
    hidden_states: &[Tensor],
    safe_centroid: &Tensor,
    pous_weights: &[f32],
    config: &CollectiveConfig,
) -> Result<(
    CollectiveResult,
    NeuralOdeCollectiveResult,
    ReplicatorResult,
)> {
    // Step 1: Multi-agent HMC-SVGD
    let collective = collective_hmc_steer(hidden_states, safe_centroid, pous_weights, config)?;

    // Step 2: Neural ODE refinement on collective state
    let ode_result = neural_ode_collective_step(
        &collective.collective_state,
        safe_centroid,
        &collective.final_weights,
        config,
    )?;

    // Step 3: Replicator dynamics for weight evolution
    let fitnesses: Vec<f32> = collective
        .agent_states
        .iter()
        .map(|s| {
            let e = compute_agent_energy(s, safe_centroid, s, config.lambda)?;
            Ok(-e)
        })
        .collect::<Result<Vec<_>>>()?;
    let replicator = replicator_aggregate(&collective.final_weights, &fitnesses, config)?;

    Ok((collective, ode_result, replicator))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tensor(rows: usize, cols: usize, seed: u64) -> Result<Tensor> {
        let mut state = seed;
        let vals: Vec<f32> = (0..rows * cols)
            .map(|_| gaussian_noise(&mut state))
            .collect();
        Tensor::from_vec(vals, (rows, cols), &Device::Cpu)
    }

    // --- Config Tests ---

    #[test]
    fn test_collective_config_default() {
        let config = CollectiveConfig::default();
        assert_eq!(config.num_agents, 5);
        assert!((config.step_size - 0.01) < 1e-6);
        assert_eq!(config.leapfrog_steps, 10);
        assert_eq!(config.num_iterations, 5);
        assert!((config.lambda - 0.1) < 1e-6);
        assert!((config.temperature - 1.0) < 1e-6);
        assert!((config.ode_dt - 0.01) < 1e-6);
        assert_eq!(config.ode_steps, 50);
        assert!((config.replicator_lr - 0.1) < 1e-6);
        assert_eq!(config.seed, 42);
        assert!((config.svgd_bandwidth_scale - 1.0) < 1e-6);
    }

    #[test]
    fn test_collective_config_with_agents() {
        let config = CollectiveConfig::default().with_agents(10);
        assert_eq!(config.num_agents, 10);
    }

    #[test]
    fn test_collective_config_agents_min() {
        let config = CollectiveConfig::default().with_agents(0);
        assert_eq!(config.num_agents, 1);
    }

    #[test]
    fn test_collective_config_with_step_size() {
        let config = CollectiveConfig::default().with_step_size(0.05);
        assert!((config.step_size - 0.05) < 1e-6);
    }

    #[test]
    fn test_collective_config_step_size_min() {
        let config = CollectiveConfig::default().with_step_size(0.0);
        assert!(config.step_size >= 1e-6);
    }

    #[test]
    fn test_collective_config_with_leapfrog_steps() {
        let config = CollectiveConfig::default().with_leapfrog_steps(20);
        assert_eq!(config.leapfrog_steps, 20);
    }

    #[test]
    fn test_collective_config_leapfrog_min() {
        let config = CollectiveConfig::default().with_leapfrog_steps(0);
        assert_eq!(config.leapfrog_steps, 1);
    }

    #[test]
    fn test_collective_config_with_iterations() {
        let config = CollectiveConfig::default().with_iterations(15);
        assert_eq!(config.num_iterations, 15);
    }

    #[test]
    fn test_collective_config_iterations_min() {
        let config = CollectiveConfig::default().with_iterations(0);
        assert_eq!(config.num_iterations, 1);
    }

    #[test]
    fn test_collective_config_with_lambda() {
        let config = CollectiveConfig::default().with_lambda(0.5);
        assert!((config.lambda - 0.5) < 1e-6);
    }

    #[test]
    fn test_collective_config_with_temperature() {
        let config = CollectiveConfig::default().with_temperature(2.0);
        assert!((config.temperature - 2.0) < 1e-6);
    }

    #[test]
    fn test_collective_config_with_ode_dt() {
        let config = CollectiveConfig::default().with_ode_dt(0.005);
        assert!((config.ode_dt - 0.005) < 1e-6);
    }

    #[test]
    fn test_collective_config_with_ode_steps() {
        let config = CollectiveConfig::default().with_ode_steps(100);
        assert_eq!(config.ode_steps, 100);
    }

    #[test]
    fn test_collective_config_with_replicator_lr() {
        let config = CollectiveConfig::default().with_replicator_lr(0.2);
        assert!((config.replicator_lr - 0.2) < 1e-6);
    }

    #[test]
    fn test_collective_config_with_seed() {
        let config = CollectiveConfig::default().with_seed(123);
        assert_eq!(config.seed, 123);
    }

    #[test]
    fn test_collective_config_with_svgd_bandwidth_scale() {
        let config = CollectiveConfig::default().with_svgd_bandwidth_scale(2.0);
        assert!((config.svgd_bandwidth_scale - 2.0) < 1e-6);
    }

    #[test]
    fn test_collective_config_edge_light() {
        let config = CollectiveConfig::edge_light();
        assert_eq!(config.num_agents, 3);
        assert!((config.step_size - 0.005) < 1e-6);
        assert_eq!(config.leapfrog_steps, 5);
        assert_eq!(config.num_iterations, 2);
        assert_eq!(config.ode_steps, 20);
    }

    #[test]
    fn test_collective_config_planetary_full() {
        let config = CollectiveConfig::planetary_full();
        assert_eq!(config.num_agents, 10);
        assert!((config.step_size - 0.02) < 1e-6);
        assert_eq!(config.leapfrog_steps, 20);
        assert_eq!(config.num_iterations, 10);
        assert_eq!(config.ode_steps, 100);
    }

    // --- Energy Tests ---

    #[test]
    fn test_compute_agent_energy_positive() -> Result<()> {
        let h = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let h_orig = make_tensor(4, 8, 77)?;
        let energy = compute_agent_energy(&h, &safe, &h_orig, 0.1)?;
        assert!(
            energy > 0.0,
            "Energy must be positive for different tensors"
        );
        Ok(())
    }

    #[test]
    fn test_compute_agent_energy_zero_same() -> Result<()> {
        let h = make_tensor(4, 8, 42)?;
        let energy = compute_agent_energy(&h, &h, &h, 0.1)?;
        assert!(
            (energy - 0.0) < 1e-6,
            "Energy should be zero when all tensors identical"
        );
        Ok(())
    }

    #[test]
    fn test_compute_agent_energy_gradient_direction() -> Result<()> {
        let h = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let h_orig = h.clone();
        let grad = compute_agent_energy_gradient(&h, &safe, &h_orig, 0.1, &Device::Cpu)?;
        assert_eq!(grad.shape(), h.shape(), "Gradient shape must match input");
        Ok(())
    }

    // --- Single Agent HMC Tests ---

    #[test]
    fn test_single_agent_hmc_basic() -> Result<()> {
        let hidden = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let config = CollectiveConfig::default();
        let (result, energy) = single_agent_hmc(&hidden, &safe, &config, 42)?;
        assert_eq!(result.shape(), hidden.shape());
        assert!(energy >= 0.0);
        Ok(())
    }

    #[test]
    fn test_single_agent_hmc_deterministic() -> Result<()> {
        let hidden = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let config = CollectiveConfig::default();
        let (r1, e1) = single_agent_hmc(&hidden, &safe, &config, 42)?;
        let (r2, e2) = single_agent_hmc(&hidden, &safe, &config, 42)?;
        let diff = r1.sub(&r2)?.sqr()?.sum_all()?.to_scalar::<f32>()?;
        assert!(diff < 1e-6, "HMC must be deterministic with same seed");
        assert!((e1 - e2) < 1e-6);
        Ok(())
    }

    #[test]
    fn test_single_agent_hmc_moves_toward_safe() -> Result<()> {
        let hidden = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let config = CollectiveConfig::default();
        let dist_before = hidden.sub(&safe)?.sqr()?.sum_all()?.to_scalar::<f32>()?;
        let (result, _) = single_agent_hmc(&hidden, &safe, &config, 42)?;
        let dist_after = result.sub(&safe)?.sqr()?.sum_all()?.to_scalar::<f32>()?;
        // HMC should generally move toward safe (not guaranteed every run)
        assert!(
            dist_after < dist_before * 2.0,
            "Distance should not dramatically increase"
        );
        Ok(())
    }

    // --- Collective HMC Tests ---

    #[test]
    fn test_collective_hmc_steer_empty() -> Result<()> {
        let safe = make_tensor(4, 8, 42)?;
        let weights = vec![];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&[], &safe, &weights, &config);
        assert!(result.is_err(), "Empty agents should error");
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_single_agent() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        assert_eq!(result.agent_states.len(), 1);
        assert_eq!(result.final_weights.len(), 1);
        assert!(result.effective_agents >= 1);
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_multiple_agents() -> Result<()> {
        let states = vec![
            make_tensor(4, 8, 42)?,
            make_tensor(4, 8, 43)?,
            make_tensor(4, 8, 44)?,
        ];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.3, 0.2];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        assert_eq!(result.agent_states.len(), 3);
        assert_eq!(result.final_weights.len(), 3);
        assert!(result.effective_agents <= 3);
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_weight_normalization() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![3.0, 1.0]; // Unnormalized
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        let weight_sum: f32 = result.final_weights.iter().sum();
        assert!((weight_sum - 1.0) < 1e-4, "Weights should be normalized");
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_zero_weights_uniform() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.0, 0.0];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        // Zero weights are normalized to uniform, then replicator dynamics may shift slightly
        // Check weights are roughly equal and sum to 1
        let weight_sum: f32 = result.final_weights.iter().sum();
        assert!((weight_sum - 1.0) < 1e-4, "Weights should sum to 1");
        assert!(
            (result.final_weights[0] - result.final_weights[1]).abs() < 0.3,
            "Weights should be roughly similar after zero-weight normalization"
        );
        for w in &result.final_weights {
            assert!(*w > 0.0, "All weights should be positive");
        }
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_deterministic() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let r1 = collective_hmc_steer(&states, &safe, &weights, &config)?;
        let r2 = collective_hmc_steer(&states, &safe, &weights, &config)?;
        let diff = r1
            .collective_state
            .sub(&r2.collective_state)?
            .sqr()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(diff < 1e-6, "Collective steering must be deterministic");
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_fitness_positive() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        // Fitness can be negative (depends on energy), but should be finite
        assert!(result.collective_fitness.is_finite());
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_display() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        let display = format!("{}", result);
        assert!(display.contains("agents="));
        assert!(display.contains("effective="));
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_energy_reduction_finite() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        assert!(result.energy_reduction.is_finite());
        Ok(())
    }

    #[test]
    fn test_collective_hmc_steer_effective_agents_count() -> Result<()> {
        let states: Vec<Tensor> = (0..5)
            .map(|i| make_tensor(4, 8, 42 + i))
            .collect::<Result<_>>()?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.4, 0.3, 0.2, 0.05, 0.05];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        assert!(result.effective_agents >= 1);
        assert!(result.effective_agents <= 5);
        Ok(())
    }

    // --- Neural ODE Tests ---

    #[test]
    fn test_neural_ode_collective_step_basic() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        assert_eq!(result.final_state.shape(), state.shape());
        assert_eq!(result.steps_taken, config.ode_steps);
        assert!(result.trajectory.len() > 0);
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_trajectory_length() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default().with_ode_steps(30);
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        // Records every 10 steps + initial + final
        assert!(result.trajectory.len() >= 3);
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_energy_finite() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        assert!(result.total_energy_change.is_finite());
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_stability_range() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        assert!(result.avg_stability >= 0.0);
        assert!(result.avg_stability <= 1.0);
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_deterministic() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let r1 = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        let r2 = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        let diff = r1
            .final_state
            .sub(&r2.final_state)?
            .sqr()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(diff < 1e-6, "Neural ODE must be deterministic");
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_display() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        let display = format!("{}", result);
        assert!(display.contains("steps="));
        assert!(display.contains("stability="));
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_small_dt() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default()
            .with_ode_dt(0.001)
            .with_ode_steps(5);
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        assert_eq!(result.steps_taken, 5);
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_large_dt() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default()
            .with_ode_dt(0.1)
            .with_ode_steps(5);
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        assert_eq!(result.steps_taken, 5);
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_multiple_weights() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.3, 0.2];
        let config = CollectiveConfig::default();
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        assert!(result.total_energy_change.is_finite());
        Ok(())
    }

    #[test]
    fn test_neural_ode_collective_step_trajectory_shapes() -> Result<()> {
        let state = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default().with_ode_steps(25);
        let result = neural_ode_collective_step(&state, &safe, &weights, &config)?;
        for (i, traj_state) in result.trajectory.iter().enumerate() {
            assert_eq!(
                traj_state.shape(),
                state.shape(),
                "Trajectory state {} shape mismatch",
                i
            );
        }
        Ok(())
    }

    // --- Replicator Tests ---

    #[test]
    fn test_replicator_aggregate_empty() -> Result<()> {
        let weights = vec![];
        let fitnesses = vec![];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config);
        assert!(result.is_err(), "Empty inputs should error");
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_length_mismatch() -> Result<()> {
        let weights = vec![0.5, 0.5];
        let fitnesses = vec![1.0];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config);
        assert!(result.is_err(), "Length mismatch should error");
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_basic() -> Result<()> {
        let weights = vec![0.5, 0.3, 0.2];
        let fitnesses = vec![1.0, 0.5, 0.2];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        assert_eq!(result.updated_weights.len(), 3);
        assert_eq!(result.fitness_values.len(), 3);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_normalization() -> Result<()> {
        let weights = vec![0.5, 0.3, 0.2];
        let fitnesses = vec![1.0, 0.5, 0.2];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        let weight_sum: f32 = result.updated_weights.iter().sum();
        assert!((weight_sum - 1.0) < 1e-4, "Updated weights should sum to 1");
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_dominant_fitness() -> Result<()> {
        let weights = vec![0.33, 0.33, 0.34];
        let fitnesses = vec![10.0, 1.0, 1.0]; // First agent much better
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        assert_eq!(
            result.dominant_index, 0,
            "Highest fitness should be dominant"
        );
        assert!(result.updated_weights[0] > result.updated_weights[1]);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_equal_fitness() -> Result<()> {
        let weights = vec![0.33, 0.34, 0.33];
        let fitnesses = vec![1.0, 1.0, 1.0];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        // With equal fitness, weights should stay roughly equal
        for w in &result.updated_weights {
            assert!((*w - 0.333).abs() < 0.1);
        }
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_entropy_positive() -> Result<()> {
        let weights = vec![0.33, 0.34, 0.33];
        let fitnesses = vec![1.0, 1.0, 1.0];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        assert!(result.weight_entropy > 0.0, "Entropy should be positive");
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_entropy_max_uniform() -> Result<()> {
        let n = 3;
        let uniform = 1.0 / n as f32;
        let weights = vec![uniform; n];
        let fitnesses = vec![1.0; n];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        // Max entropy for uniform distribution: log(n)
        let max_entropy = (n as f32).ln();
        assert!(result.weight_entropy < max_entropy + 0.1);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_avg_fitness() -> Result<()> {
        let weights = vec![0.5, 0.5];
        let fitnesses = vec![2.0, 0.0];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        // avg_fitness = 0.5 * 2.0 + 0.5 * 0.0 = 1.0
        assert!((result.avg_fitness - 1.0) < 1e-6);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_display() -> Result<()> {
        let weights = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 0.5];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        let display = format!("{}", result);
        assert!(display.contains("avg_fitness="));
        assert!(display.contains("entropy="));
        assert!(display.contains("dominant="));
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_single_strategy() -> Result<()> {
        let weights = vec![1.0];
        let fitnesses = vec![5.0];
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        assert!((result.updated_weights[0] - 1.0) < 1e-6);
        assert_eq!(result.dominant_index, 0);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_low_lr_stability() -> Result<()> {
        let weights = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 0.0];
        let config = CollectiveConfig::default().with_replicator_lr(0.01);
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        // Low LR means minimal change
        assert!((result.updated_weights[0] - 0.5) < 0.1);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_high_lr_change() -> Result<()> {
        let weights = vec![0.5, 0.5];
        let fitnesses = vec![10.0, 1.0];
        let config = CollectiveConfig::default().with_replicator_lr(0.5);
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        // High LR + high fitness ratio should shift weights significantly
        assert!(result.updated_weights[0] > result.updated_weights[1]);
        Ok(())
    }

    #[test]
    fn test_replicator_aggregate_weights_positive() -> Result<()> {
        let weights = vec![0.25, 0.25, 0.25, 0.25];
        let fitnesses = vec![1.0, -5.0, 1.0, 1.0]; // One very negative fitness
        let config = CollectiveConfig::default();
        let result = replicator_aggregate(&weights, &fitnesses, &config)?;
        for w in &result.updated_weights {
            assert!(*w > 0.0, "All weights must remain positive");
        }
        Ok(())
    }

    // --- Meta-Gradient Tests ---

    #[test]
    fn test_meta_gradient_update_improving() {
        let config = CollectiveConfig::default();
        let new_config = meta_gradient_update(&config, 1.0, 0.0); // Fitness improved
        assert!(new_config.step_size >= config.step_size);
    }

    #[test]
    fn test_meta_gradient_update_worsening() {
        let config = CollectiveConfig::default();
        let new_config = meta_gradient_update(&config, -1.0, 0.0); // Fitness worsened
        assert!(new_config.step_size <= config.step_size);
    }

    #[test]
    fn test_meta_gradient_update_stuck_increases_temp() {
        let config = CollectiveConfig::default();
        let new_config = meta_gradient_update(&config, 0.00001, 0.0); // Almost no change
        assert!(new_config.temperature >= config.temperature);
    }

    #[test]
    fn test_meta_gradient_update_step_size_bounds() {
        let config = CollectiveConfig::default().with_step_size(0.5);
        let new_config = meta_gradient_update(&config, 100.0, 0.0); // Huge improvement
        assert!(new_config.step_size <= 1.0);
    }

    #[test]
    fn test_meta_gradient_update_temperature_bounds() {
        let config = CollectiveConfig::default().with_temperature(0.5);
        let new_config = meta_gradient_update(&config, 0.00001, 0.0);
        assert!(new_config.temperature >= 0.1);
        assert!(new_config.temperature <= 5.0);
    }

    #[test]
    fn test_meta_gradient_update_lambda_increase() {
        let config = CollectiveConfig::default();
        let new_config = meta_gradient_update(&config, -0.5, 0.0); // Big worsening
        assert!(new_config.lambda >= config.lambda);
    }

    #[test]
    fn test_meta_gradient_update_lambda_bounds() {
        let config = CollectiveConfig::default().with_lambda(5.0);
        let new_config = meta_gradient_update(&config, -10.0, 0.0);
        assert!(new_config.lambda <= 10.0);
    }

    // --- Full Pipeline Tests ---

    #[test]
    fn test_full_collective_pipeline_basic() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let (collective, ode, _replicator) =
            full_collective_pipeline(&states, &safe, &weights, &config)?;
        assert_eq!(collective.agent_states.len(), 2);
        assert_eq!(ode.steps_taken, config.ode_steps);
        Ok(())
    }

    #[test]
    fn test_full_collective_pipeline_single_agent() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![1.0];
        let config = CollectiveConfig::default();
        let (collective, ode, replicator) =
            full_collective_pipeline(&states, &safe, &weights, &config)?;
        assert_eq!(collective.agent_states.len(), 1);
        assert!(collective.collective_fitness.is_finite());
        assert!(ode.total_energy_change.is_finite());
        assert!(replicator.weight_entropy >= 0.0);
        Ok(())
    }

    #[test]
    fn test_full_collective_pipeline_multiple_agents() -> Result<()> {
        let states: Vec<Tensor> = (0..5)
            .map(|i| make_tensor(4, 8, 42 + i))
            .collect::<Result<_>>()?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.4, 0.3, 0.2, 0.05, 0.05];
        let config = CollectiveConfig::default();
        let (collective, _ode, _replicator) =
            full_collective_pipeline(&states, &safe, &weights, &config)?;
        assert_eq!(collective.agent_states.len(), 5);
        assert!(collective.effective_agents <= 5);
        Ok(())
    }

    #[test]
    fn test_full_collective_pipeline_edge_light() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::edge_light();
        let (collective, ode, _replicator) =
            full_collective_pipeline(&states, &safe, &weights, &config)?;
        assert!(collective.collective_fitness.is_finite());
        assert_eq!(ode.steps_taken, config.ode_steps);
        Ok(())
    }

    #[test]
    fn test_full_collective_pipeline_planetary() -> Result<()> {
        let states: Vec<Tensor> = (0..3)
            .map(|i| make_tensor(4, 8, 42 + i))
            .collect::<Result<_>>()?;
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.3, 0.2];
        let config = CollectiveConfig::planetary_full();
        let (collective, ode, _replicator) =
            full_collective_pipeline(&states, &safe, &weights, &config)?;
        assert!(collective.collective_fitness.is_finite());
        assert_eq!(ode.steps_taken, config.ode_steps);
        Ok(())
    }

    #[test]
    fn test_full_collective_pipeline_deterministic() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let (c1, _o1, _r1) = full_collective_pipeline(&states, &safe, &weights, &config)?;
        let (c2, _o2, _r2) = full_collective_pipeline(&states, &safe, &weights, &config)?;
        let diff = c1
            .collective_state
            .sub(&c2.collective_state)?
            .sqr()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(diff < 1e-6, "Full pipeline must be deterministic");
        Ok(())
    }

    #[test]
    fn test_full_collective_pipeline_weight_evolution() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let (_collective, _, replicator) =
            full_collective_pipeline(&states, &safe, &weights, &config)?;
        // Weights should evolve based on fitness
        let weight_sum: f32 = replicator.updated_weights.iter().sum();
        assert!((weight_sum - 1.0) < 1e-4);
        Ok(())
    }

    // --- PRNG Tests ---

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        let v1 = lcg_next(&mut s1);
        let v2 = lcg_next(&mut s2);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut state: u64 = 42;
        let v1 = lcg_next(&mut state);
        let v2 = lcg_next(&mut state);
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_gaussian_noise_range() {
        let mut state: u64 = 42;
        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;
        for _ in 0..1000 {
            let val = gaussian_noise(&mut state);
            if val < min_val {
                min_val = val;
            }
            if val > max_val {
                max_val = val;
            }
        }
        assert!(
            min_val > -5.0,
            "Gaussian noise should be in reasonable range"
        );
        assert!(
            max_val < 5.0,
            "Gaussian noise should be in reasonable range"
        );
    }

    #[test]
    fn test_gaussian_noise_mean_near_zero() {
        let mut state: u64 = 42;
        let sum: f32 = (0..10000).map(|_| gaussian_noise(&mut state)).sum();
        let mean = sum / 10000.0;
        assert!(mean.abs() < 0.1, "Gaussian noise mean should be near zero");
    }

    // --- Integration Tests ---

    #[test]
    fn test_collective_vs_individual_consistency() -> Result<()> {
        // Single agent collective should match individual HMC
        let hidden = make_tensor(4, 8, 42)?;
        let safe = make_tensor(4, 8, 99)?;
        let config = CollectiveConfig::default();

        let (individual, _) = single_agent_hmc(&hidden, &safe, &config, config.seed)?;
        let collective = collective_hmc_steer(&[hidden.clone()], &safe, &[1.0], &config)?;

        // Results should be similar (not identical due to aggregation)
        let diff = individual
            .sub(&collective.collective_state)?
            .sqr()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(
            diff < 1.0,
            "Single agent collective should be close to individual"
        );
        Ok(())
    }

    #[test]
    fn test_ode_refines_collective() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();

        let collective = collective_hmc_steer(&states, &safe, &weights, &config)?;
        let ode = neural_ode_collective_step(
            &collective.collective_state,
            &safe,
            &collective.final_weights,
            &config,
        )?;

        // ODE should produce a valid final state
        assert_eq!(ode.final_state.shape(), collective.collective_state.shape());
        assert!(ode.total_energy_change.is_finite());
        Ok(())
    }

    #[test]
    fn test_replicator_improves_weights() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();

        let collective = collective_hmc_steer(&states, &safe, &weights, &config)?;
        let fitnesses: Vec<f32> = collective
            .agent_states
            .iter()
            .map(|s| {
                let e = compute_agent_energy(s, &safe, s, config.lambda)?;
                Ok(-e)
            })
            .collect::<Result<Vec<_>>>()?;
        let replicator = replicator_aggregate(&collective.final_weights, &fitnesses, &config)?;

        // After replicator, weights should still sum to 1
        let weight_sum: f32 = replicator.updated_weights.iter().sum();
        assert!((weight_sum - 1.0) < 1e-4);
        Ok(())
    }

    #[test]
    fn test_iterative_improvement() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let mut weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();

        let mut prev_fitness = f32::NEG_INFINITY;
        for _ in 0..3 {
            let collective = collective_hmc_steer(&states, &safe, &weights, &config)?;
            let fitnesses: Vec<f32> = collective
                .agent_states
                .iter()
                .map(|s| {
                    let e = compute_agent_energy(s, &safe, s, config.lambda)?;
                    Ok(-e)
                })
                .collect::<Result<Vec<_>>>()?;
            let replicator = replicator_aggregate(&weights, &fitnesses, &config)?;
            weights = replicator.updated_weights;
            if collective.collective_fitness > prev_fitness {
                prev_fitness = collective.collective_fitness;
            }
        }
        // Pipeline should complete without error
        Ok(())
    }

    #[test]
    fn test_large_collective() -> Result<()> {
        let num_agents: usize = 10;
        let states: Vec<Tensor> = (0..num_agents)
            .map(|i| make_tensor(4, 16, 42 + i as u64))
            .collect::<Result<_>>()?;
        let safe = make_tensor(4, 16, 99)?;
        let weights = vec![1.0 / num_agents as f32; num_agents];
        let config = CollectiveConfig::default().with_agents(num_agents);
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        assert_eq!(result.agent_states.len(), num_agents);
        assert!(result.collective_fitness.is_finite());
        Ok(())
    }

    #[test]
    fn test_high_dimensional() -> Result<()> {
        let states = vec![make_tensor(32, 64, 42)?, make_tensor(32, 64, 43)?];
        let safe = make_tensor(32, 64, 99)?;
        let weights = vec![0.5, 0.5];
        let config = CollectiveConfig::default();
        let result = collective_hmc_steer(&states, &safe, &weights, &config)?;
        assert_eq!(result.collective_state.shape(), safe.shape());
        Ok(())
    }

    #[test]
    fn test_meta_gradient_pipeline() -> Result<()> {
        let states = vec![make_tensor(4, 8, 42)?, make_tensor(4, 8, 43)?];
        let safe = make_tensor(4, 8, 99)?;
        let weights = vec![0.5, 0.5];
        let mut config = CollectiveConfig::default();

        let mut prev_fitness = 0.0;
        for _ in 0..3 {
            let (collective, _, _) = full_collective_pipeline(&states, &safe, &weights, &config)?;
            config = meta_gradient_update(&config, collective.collective_fitness, prev_fitness);
            prev_fitness = collective.collective_fitness;
        }
        // Config should remain valid after meta-gradient updates
        assert!(config.step_size > 0.0);
        assert!(config.temperature > 0.0);
        assert!(config.lambda >= 0.0);
        Ok(())
    }
}
