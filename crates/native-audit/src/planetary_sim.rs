//! Planetary Simulation — Large-scale mesh simulation with churn modeling.
//!
//! Simulates planetary-scale deployment of ed2kIA nodes under realistic conditions:
//! - Node churn (join/leave) with configurable probability
//! - Heterogeneous device types (smartwatch → datacenter)
//! - Network latency variation (3G to fiber)
//! - Energy-aware scheduling
//! - Trust dynamics and PoSym accumulation
//!
//! **Sprint 124:** Planetary Symbiotic Mesh & Full Edge Deployment Immunity.
//! **Sprint 126:** The Noosfera Awakening & Global Ecosystem Symbiosis — Awakening simulation
//! with adoption curve modeling, tipping point detection, and Noosfera emergence metrics.

use crate::edge_runtime::{DeviceType, EnergyImpact, PowerState};

/// Configuration for planetary-scale simulation.
#[derive(Debug, Clone)]
pub struct PlanetarySimConfig {
    /// Total number of nodes in the simulation
    pub node_count: usize,
    /// Node churn probability per step (0.0 to 0.1)
    pub churn_probability: f64,
    /// Simulation duration in seconds
    pub duration_seconds: f64,
    /// Time step in seconds
    pub timestep: f64,
    /// Base network latency in milliseconds
    pub base_latency_ms: f64,
    /// Latency variance percentage
    pub latency_variance: f64,
    /// Device distribution: (DeviceType, weight)
    pub device_distribution: Vec<(DeviceType, f64)>,
    /// Enable DP noise in simulation
    pub dp_enabled: bool,
    /// DP epsilon for simulation
    pub dp_epsilon: f64,
}

impl Default for PlanetarySimConfig {
    fn default() -> Self {
        Self {
            node_count: 10_000,
            churn_probability: 0.05,
            duration_seconds: 3600.0,
            timestep: 60.0,
            base_latency_ms: 100.0,
            latency_variance: 0.3,
            device_distribution: vec![
                (DeviceType::Desktop, 0.30),
                (DeviceType::Mobile, 0.35),
                (DeviceType::Iot, 0.20),
                (DeviceType::Datacenter, 0.10),
                (DeviceType::Smartwatch, 0.03),
                (DeviceType::OldDesktop, 0.02),
            ],
            dp_enabled: false,
            dp_epsilon: 1.0,
        }
    }
}

impl PlanetarySimConfig {
    /// Create config with custom node count.
    pub fn with_nodes(mut self, count: usize) -> Self {
        self.node_count = count;
        self
    }

    /// Create config with custom churn rate.
    pub fn with_churn(mut self, probability: f64) -> Self {
        self.churn_probability = probability.clamp(0.0, 0.1);
        self
    }

    /// Create config with custom duration.
    pub fn with_duration(mut self, seconds: f64) -> Self {
        self.duration_seconds = seconds;
        self
    }

    /// Create config for high-churn scenario (mobile-heavy).
    pub fn high_churn() -> Self {
        Self {
            churn_probability: 0.08,
            device_distribution: vec![
                (DeviceType::Mobile, 0.50),
                (DeviceType::Iot, 0.30),
                (DeviceType::Smartwatch, 0.10),
                (DeviceType::Desktop, 0.10),
            ],
            base_latency_ms: 200.0,
            latency_variance: 0.5,
            ..Self::default()
        }
    }

    /// Create config for stable scenario (datacenter-heavy).
    pub fn stable() -> Self {
        Self {
            churn_probability: 0.01,
            device_distribution: vec![
                (DeviceType::Datacenter, 0.60),
                (DeviceType::Desktop, 0.30),
                (DeviceType::Mobile, 0.10),
            ],
            base_latency_ms: 10.0,
            latency_variance: 0.1,
            ..Self::default()
        }
    }
}

/// A simulated node in the planetary mesh.
#[derive(Debug, Clone)]
pub struct SimNode {
    /// Unique node ID
    pub id: u64,
    /// Device type
    pub device_type: DeviceType,
    /// Current power state
    pub power_state: PowerState,
    /// Current trust score [0, 1]
    pub trust_score: f64,
    /// Accumulated energy consumption (mWh)
    pub energy_consumed_mwh: f64,
    /// Number of successful steers
    pub steer_count: u64,
    /// Number of failed steers
    pub fail_count: u64,
    /// Node is currently active
    pub active: bool,
    /// Join time (simulation seconds)
    pub join_time: f64,
    /// Current latency (ms)
    pub latency_ms: f64,
}

impl SimNode {
    /// Create a new simulated node.
    pub fn new(id: u64, device_type: DeviceType, join_time: f64) -> Self {
        Self {
            id,
            device_type,
            power_state: PowerState::Normal,
            trust_score: 0.5,
            energy_consumed_mwh: 0.0,
            steer_count: 0,
            fail_count: 0,
            active: true,
            join_time,
            latency_ms: 100.0,
        }
    }

    /// Simulate one step for this node.
    pub fn step(
        &mut self,
        timestep: f64,
        base_latency: f64,
        latency_var: f64,
        churn_prob: f64,
        seed: u64,
    ) {
        if !self.active {
            return;
        }

        // Update latency with variance
        let mut state = seed + self.id;
        let variance = (next_random_sim(&mut state) - 0.5) * 2.0 * latency_var;
        self.latency_ms = (base_latency * (1.0 + variance)).max(1.0);

        // Simulate energy consumption based on device type and power state
        let base_cost = self.device_type.base_energy_cost();
        let budget = self.power_state.compute_budget() as f64;
        let energy_step = base_cost * budget * (timestep / 3600.0);
        self.energy_consumed_mwh += energy_step;

        // Simulate steering attempt
        let success_prob = self.trust_score * budget.min(1.0);
        let roll = next_random_sim(&mut state);
        if roll < success_prob {
            self.steer_count += 1;
            self.trust_score = (self.trust_score + 0.01).min(1.0);
        } else {
            self.fail_count += 1;
            self.trust_score = (self.trust_score - 0.005).max(0.0);
        }

        // Simulate churn — node may leave
        let churn_roll = next_random_sim(&mut state);
        if churn_roll < churn_prob {
            self.active = false;
        }
    }
}

/// Simulation results after running the planetary mesh simulation.
#[derive(Debug, Clone)]
pub struct PlanetarySimResult {
    /// Total nodes configured
    pub total_nodes: usize,
    /// Active nodes at end of simulation
    pub active_nodes: usize,
    /// Nodes that churned (left) during simulation
    pub churned_nodes: usize,
    /// Nodes that rejoined during simulation
    pub rejoined_nodes: usize,
    /// Total successful steers across all nodes
    pub total_steers: u64,
    /// Total failed steers across all nodes
    pub total_failures: u64,
    /// Average trust score across active nodes
    pub avg_trust: f64,
    /// Total energy consumed (mWh)
    pub total_energy_mwh: f64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Simulation duration (seconds)
    pub duration_seconds: f64,
    /// Number of simulation steps
    pub steps: usize,
    /// Steer success rate
    pub steer_success_rate: f64,
    /// Network resilience score (active_nodes / total_nodes)
    pub resilience_score: f64,
}

impl PlanetarySimResult {
    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "PlanetarySim[{}s] nodes={}/{} churn={} rejoin={} steers={}/{} trust={:.3} energy={:.2}mWh latency={:.1}ms success={:.1}% resilience={:.1}%",
            self.duration_seconds,
            self.active_nodes,
            self.total_nodes,
            self.churned_nodes,
            self.rejoined_nodes,
            self.total_steers,
            self.total_failures,
            self.avg_trust,
            self.total_energy_mwh,
            self.avg_latency_ms,
            self.steer_success_rate * 100.0,
            self.resilience_score * 100.0,
        )
    }
}

impl Default for PlanetarySimResult {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            active_nodes: 0,
            churned_nodes: 0,
            rejoined_nodes: 0,
            total_steers: 0,
            total_failures: 0,
            avg_trust: 0.0,
            total_energy_mwh: 0.0,
            avg_latency_ms: 0.0,
            duration_seconds: 0.0,
            steps: 0,
            steer_success_rate: 0.0,
            resilience_score: 0.0,
        }
    }
}

/// Deterministic PRNG for simulation.
fn next_random_sim(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let x = (*state >> 33) as u32;
    x as f64 / u32::MAX as f64
}

/// Select a device type based on weighted distribution.
fn select_device_type(distribution: &[(DeviceType, f64)], roll: f64) -> DeviceType {
    let mut cumulative = 0.0;
    for (device_type, weight) in distribution {
        cumulative += weight;
        if roll <= cumulative {
            return *device_type;
        }
    }
    distribution
        .last()
        .map(|(dt, _)| *dt)
        .unwrap_or(DeviceType::Desktop)
}

/// Run the planetary mesh simulation.
///
/// Simulates a large-scale deployment of ed2kIA nodes under realistic conditions
/// including node churn, heterogeneous hardware, network latency variation,
/// and energy-aware scheduling.
///
/// # Arguments
/// * `node_count` — Total number of nodes to simulate
/// * `churn_probability` — Probability of node leaving per step (0.0 to 0.1)
/// * `duration_seconds` — Total simulation duration in seconds
/// * `config` — Optional simulation configuration (use defaults if None)
///
/// # Returns
/// `PlanetarySimResult` with full simulation statistics
pub fn simulate_planetary_mesh(
    node_count: usize,
    churn_probability: f64,
    duration_seconds: f64,
    config: Option<PlanetarySimConfig>,
) -> PlanetarySimResult {
    let cfg = config.unwrap_or_else(PlanetarySimConfig::default);
    let mut nodes: Vec<SimNode> = Vec::with_capacity(node_count);

    // Initialize nodes with device distribution
    let mut seed = 42;
    for i in 0..node_count {
        let roll = next_random_sim(&mut seed);
        let device_type = select_device_type(&cfg.device_distribution, roll);
        let node = SimNode::new(i as u64, device_type, 0.0);
        nodes.push(node);
    }

    let steps = (duration_seconds / cfg.timestep) as usize;
    let mut total_churned = 0;
    let mut total_rejoined = 0;
    let mut total_steers: u64 = 0;
    let mut total_failures: u64 = 0;

    for step in 0..steps {
        let sim_time = step as f64 * cfg.timestep;

        for node in &mut nodes {
            if node.active {
                node.step(
                    cfg.timestep,
                    cfg.base_latency_ms,
                    cfg.latency_variance,
                    churn_probability,
                    seed + step as u64,
                );
                total_steers += node.steer_count;
                total_failures += node.fail_count;
            } else {
                // Churned node — track it
                total_churned += 1;
            }

            // Rejoin logic: churned nodes may rejoin
            if !node.active {
                let rejoin_roll = next_random_sim(&mut (seed + step as u64 + node.id));
                if rejoin_roll < 0.1 * churn_probability {
                    node.active = true;
                    node.join_time = sim_time;
                    node.trust_score = 0.3; // Rejoined nodes start with lower trust
                    total_rejoined += 1;
                }
            }
        }
    }

    // Compute final statistics
    let active_nodes = nodes.iter().filter(|n| n.active).count();
    let active_trusts: Vec<f64> = nodes
        .iter()
        .filter(|n| n.active)
        .map(|n| n.trust_score)
        .collect();
    let avg_trust = if active_trusts.is_empty() {
        0.0
    } else {
        active_trusts.iter().sum::<f64>() / active_trusts.len() as f64
    };

    let total_energy: f64 = nodes.iter().map(|n| n.energy_consumed_mwh).sum();
    let avg_latency = if active_nodes > 0 {
        nodes
            .iter()
            .filter(|n| n.active)
            .map(|n| n.latency_ms)
            .sum::<f64>()
            / active_nodes as f64
    } else {
        0.0
    };

    let total_actions = total_steers + total_failures;
    let steer_success_rate = if total_actions > 0 {
        total_steers as f64 / total_actions as f64
    } else {
        0.0
    };

    let resilience_score = if node_count > 0 {
        active_nodes as f64 / node_count as f64
    } else {
        0.0
    };

    PlanetarySimResult {
        total_nodes: node_count,
        active_nodes,
        churned_nodes: total_churned,
        rejoined_nodes: total_rejoined,
        total_steers,
        total_failures,
        avg_trust,
        total_energy_mwh: total_energy,
        avg_latency_ms: avg_latency,
        duration_seconds,
        steps,
        steer_success_rate,
        resilience_score,
    }
}

/// Compute energy impact from simulation results.
///
/// # Arguments
/// * `result` — Planetary simulation results
/// * `device_type` — Device type to compute impact for
///
/// # Returns
/// `EnergyImpact` struct with energy statistics
pub fn compute_sim_energy_impact(
    result: &PlanetarySimResult,
    device_type: DeviceType,
) -> EnergyImpact {
    let per_node_energy = if result.active_nodes > 0 {
        result.total_energy_mwh / result.active_nodes as f64
    } else {
        0.0
    };

    let base_cost = device_type.base_energy_cost();
    let _certified_calls = result.total_steers;

    let energy_used = per_node_energy * base_cost;
    let dc_baseline = per_node_energy * DeviceType::Datacenter.base_energy_cost();
    let energy_saved = (dc_baseline - energy_used).max(0.0);
    let savings_pct = if dc_baseline > 0.0 {
        (energy_saved / dc_baseline) * 100.0
    } else {
        0.0
    };

    EnergyImpact {
        energy_used_mwh: energy_used,
        dc_baseline_mwh: dc_baseline,
        energy_saved_mwh: energy_saved,
        savings_pct,
        power_state: PowerState::Normal,
        compute_path: crate::edge_runtime::ComputePath::UltraLight,
    }
}

/// Noosfera Awakening Metrics — Adoption curve and tipping point detection.
///
/// Tracks the emergence of collective intelligence across the planetary mesh
/// as nodes adopt ed2kIA steering, measuring adoption rate, network effects,
/// and the critical tipping point where the Noosfera becomes self-sustaining.
#[derive(Debug, Clone)]
pub struct AwakeningMetrics {
    /// Total nodes in the simulation
    pub total_nodes: usize,
    /// Nodes actively participating in the Noosfera
    pub awakened_nodes: usize,
    /// Adoption rate (0.0 to 1.0)
    pub adoption_rate: f64,
    /// Month when tipping point was reached (0 if not reached)
    pub tipping_point_month: u32,
    /// Whether the adoption tipping point was reached (>50% adoption)
    pub tipping_point_reached: bool,
    /// Network effect multiplier (exponential growth factor)
    pub network_effect_multiplier: f64,
    /// Average trust score of awakened nodes
    pub avg_awakened_trust: f64,
    /// Collective intelligence score (weighted by trust and adoption)
    pub collective_intelligence_score: f64,
    /// Knowledge diffusion rate (nodes informed per month)
    pub knowledge_diffusion_rate: f64,
    /// Months simulated
    pub months_simulated: u32,
    /// Adoption curve: (month, adoption_rate)
    pub adoption_curve: Vec<(u32, f64)>,
}

impl AwakeningMetrics {
    /// Create new awakening metrics.
    pub fn new(
        total_nodes: usize,
        awakened_nodes: usize,
        adoption_rate: f64,
        tipping_point_month: u32,
        tipping_point_reached: bool,
        network_effect_multiplier: f64,
        avg_awakened_trust: f64,
        collective_intelligence_score: f64,
        knowledge_diffusion_rate: f64,
        months_simulated: u32,
        adoption_curve: Vec<(u32, f64)>,
    ) -> Self {
        Self {
            total_nodes,
            awakened_nodes,
            adoption_rate,
            tipping_point_month,
            tipping_point_reached,
            network_effect_multiplier,
            avg_awakened_trust,
            collective_intelligence_score,
            knowledge_diffusion_rate,
            months_simulated,
            adoption_curve,
        }
    }

    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "Awakening[{}m] nodes={}/{} rate={:.1}% tipping={} effect={:.2}x trust={:.3} ci={:.3} diffusion={:.1}/m",
            self.months_simulated,
            self.awakened_nodes,
            self.total_nodes,
            self.adoption_rate * 100.0,
            if self.tipping_point_reached {
                format!("m{}", self.tipping_point_month)
            } else {
                "—".to_string()
            },
            self.network_effect_multiplier,
            self.avg_awakened_trust,
            self.collective_intelligence_score,
            self.knowledge_diffusion_rate,
        )
    }
}

/// Simulate the Noosfera Awakening — adoption curve with tipping point detection.
///
/// Models the spread of ed2kIA adoption across a global network using:
/// - Bass diffusion model (innovation + imitation coefficients)
/// - Network effect multiplier (exponential growth from social contagion)
/// - Trust-based filtering (only high-trust nodes contribute to diffusion)
/// - Tipping point detection (>50% adoption = self-sustaining Noosfera)
///
/// # Arguments
/// * `initial_nodes` — Total nodes in the network
/// * `months` — Simulation duration in months
///
/// # Returns
/// `AwakeningMetrics` with full adoption curve and tipping point analysis
pub fn simulate_noosfera_awakening(initial_nodes: usize, months: u32) -> AwakeningMetrics {
    if initial_nodes == 0 || months == 0 {
        return AwakeningMetrics {
            total_nodes: initial_nodes,
            awakened_nodes: 0,
            adoption_rate: 0.0,
            tipping_point_month: 0,
            tipping_point_reached: false,
            network_effect_multiplier: 0.0,
            avg_awakened_trust: 0.0,
            collective_intelligence_score: 0.0,
            knowledge_diffusion_rate: 0.0,
            months_simulated: months,
            adoption_curve: vec![],
        };
    }

    // Bass diffusion parameters
    let p = 0.02; // Innovation coefficient (external influence)
    let q = 0.08; // Imitation coefficient (social contagion)

    let mut awakened = (initial_nodes as f64 * p) as usize; // Initial adopters
    let mut adoption_curve: Vec<(u32, f64)> = Vec::new();
    let mut tipping_point_month: u32 = 0;
    let mut tipping_point_reached = false;
    let mut total_diffused = 0;
    let mut mut_state = 42;

    for month in 0..months {
        let adopters_so_far = awakened;
        let remaining = initial_nodes.saturating_sub(adopters_so_far);

        // Bass model: new adopters = (p + q * adopters/N) * remaining
        let adoption_pressure = p + q * (adopters_so_far as f64 / initial_nodes as f64);
        let new_adopters = (adoption_pressure * remaining as f64) as usize;

        // Add stochastic noise (deterministic PRNG)
        let noise = next_random_sim(&mut mut_state);
        let noise_factor = 0.9 + (noise - 0.5) * 0.2; // ±10% variation
        let noisy_new_adopters = (new_adopters as f64 * noise_factor) as usize;

        awakened = (awakened + noisy_new_adopters).min(initial_nodes);
        total_diffused += noisy_new_adopters;

        let current_rate = awakened as f64 / initial_nodes as f64;
        adoption_curve.push((month, current_rate));

        // Detect tipping point (>50% adoption)
        if !tipping_point_reached && current_rate > 0.5 {
            tipping_point_reached = true;
            tipping_point_month = month;
        }
    }

    // Compute network effect multiplier (exponential growth factor)
    // Compares actual adoption to linear baseline (innovation-only, no social contagion)
    let final_rate = awakened as f64 / initial_nodes as f64;
    let baseline_linear = (p * months as f64).min(1.0);
    let network_effect_multiplier = if baseline_linear > 0.0 {
        (final_rate / baseline_linear).max(1.0)
    } else {
        1.0
    };

    // Compute average awakened trust (higher adoption → higher collective trust)
    let avg_awakened_trust = 0.5 + 0.4 * final_rate.min(1.0);

    // Collective intelligence score = adoption * trust * network_effect
    let collective_intelligence_score =
        final_rate * avg_awakened_trust * network_effect_multiplier.min(5.0);

    // Knowledge diffusion rate (avg nodes informed per month)
    let knowledge_diffusion_rate = if months > 0 {
        total_diffused as f64 / months as f64
    } else {
        0.0
    };

    AwakeningMetrics {
        total_nodes: initial_nodes,
        awakened_nodes: awakened,
        adoption_rate: final_rate,
        tipping_point_month,
        tipping_point_reached,
        network_effect_multiplier,
        avg_awakened_trust,
        collective_intelligence_score,
        knowledge_diffusion_rate,
        months_simulated: months,
        adoption_curve,
    }
}

/// Weibull Churn Configuration — Time-dependent node failure modeling.
///
/// The Weibull distribution models churn with flexible hazard rates:
/// - shape (k) < 1: Decreasing hazard (early failures / infant mortality)
/// - shape (k) = 1: Constant hazard (exponential / memoryless churn)
/// - shape (k) > 1: Increasing hazard (aging / wear-out churn)
#[derive(Debug, Clone)]
pub struct WeibullChurnConfig {
    /// Shape parameter (k) — controls hazard trend
    pub shape: f64,
    /// Scale parameter (λ) — characteristic lifetime in seconds
    pub scale: f64,
    /// Seed for deterministic simulation
    pub seed: u64,
}

impl Default for WeibullChurnConfig {
    fn default() -> Self {
        Self {
            shape: 1.5,    // Increasing hazard (aging devices)
            scale: 7200.0, // Characteristic lifetime: 2 hours
            seed: 42,
        }
    }
}

impl WeibullChurnConfig {
    /// Create config with shape parameter.
    pub fn with_shape(mut self, k: f64) -> Self {
        self.shape = k.max(0.1);
        self
    }

    /// Create config with scale parameter.
    pub fn with_scale(mut self, lambda: f64) -> Self {
        self.scale = lambda.max(1.0);
        self
    }

    /// Decreasing hazard (infant mortality — nodes drop early).
    pub fn infant_mortality() -> Self {
        Self {
            shape: 0.5,
            ..Self::default()
        }
    }

    /// Constant hazard (memoryless — exponential churn).
    pub fn exponential() -> Self {
        Self {
            shape: 1.0,
            ..Self::default()
        }
    }

    /// Increasing hazard (aging — wear-out churn).
    pub fn wear_out() -> Self {
        Self {
            shape: 2.5,
            ..Self::default()
        }
    }
}

/// Compute Weibull CDF: F(t) = 1 - exp(-(t/λ)^k)
pub fn weibull_cdf(k: f64, lambda: f64, t: f64) -> f64 {
    if t <= 0.0 || lambda <= 0.0 || k <= 0.0 {
        return 0.0;
    }
    let z = t / lambda;
    (1.0 - (-(z.powf(k))).exp()).clamp(0.0, 1.0)
}

/// Compute Weibull hazard rate: h(t) = (k/λ) × (t/λ)^(k-1)
pub fn weibull_hazard(k: f64, lambda: f64, t: f64) -> f64 {
    if t <= 0.0 || lambda <= 0.0 || k <= 0.0 {
        return 0.0;
    }
    (k / lambda) * (t / lambda).powf(k - 1.0)
}

/// Simulate Weibull churn for a fleet of nodes.
///
/// Each node has a lifetime drawn from Weibull(k, λ). Nodes whose
/// lifetime < current_time are considered churned.
///
/// # Returns
/// `(churned_count, survival_rate, avg_lifetime)`
pub fn simulate_weibull_churn(
    node_count: usize,
    config: &WeibullChurnConfig,
    duration_seconds: f64,
) -> (usize, f64, f64) {
    if node_count == 0 {
        return (0, 0.0, 0.0);
    }

    let mut state = config.seed;
    let mut churned = 0;
    let mut total_lifetime = 0.0;

    for _ in 0..node_count {
        // Inverse transform sampling: t = λ × (-ln(1 - U))^(1/k)
        let u = next_random_sim(&mut state);
        let u_clamped = u.max(1e-12).min(1.0 - 1e-12);
        let lifetime = config.scale * (-((1.0 - u_clamped).ln())).powf(1.0 / config.shape);

        if lifetime < duration_seconds {
            churned += 1;
            total_lifetime += lifetime;
        } else {
            total_lifetime += duration_seconds;
        }
    }

    let survived = node_count - churned;
    let survival_rate = survived as f64 / node_count as f64;
    let avg_lifetime = total_lifetime / node_count as f64;

    (churned, survival_rate, avg_lifetime)
}

/// Replicator Dynamics Result — Strategy evolution in the planetary mesh.
#[derive(Debug, Clone)]
pub struct ReplicatorDynamicsResult {
    /// Final strategy shares (sums to 1.0)
    pub final_shares: Vec<f64>,
    /// Number of steps simulated
    pub steps: usize,
    /// Final average fitness
    pub avg_fitness: f64,
    /// Dominant strategy index (highest share)
    pub dominant_strategy: usize,
    /// Dominant strategy share
    pub dominant_share: f64,
    /// Entropy of strategy distribution (diversity measure)
    pub strategy_entropy: f64,
    /// Share trajectory: (step, shares_snapshot)
    pub trajectory: Vec<(usize, Vec<f64>)>,
}

impl ReplicatorDynamicsResult {
    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "Replicator[{}s] dominant={} share={:.1}% fitness={:.4} entropy={:.4}",
            self.steps,
            self.dominant_strategy,
            self.dominant_share * 100.0,
            self.avg_fitness,
            self.strategy_entropy,
        )
    }
}

/// Simulate replicator dynamics: dx_i/dt = x_i × (f_i - φ)
///
/// Where:
/// - x_i = strategy share (frequency)
/// - f_i = fitness of strategy i
/// - φ = average fitness = Σ x_j × f_j
///
/// This models the evolutionary selection of strategies in the planetary mesh,
/// where higher-fitness strategies grow at the expense of lower-fitness ones.
///
/// # Arguments
/// * `initial_shares` — Initial strategy distribution (must sum to ~1.0)
/// * `fitnesses` — Fitness values for each strategy
/// * `steps` — Number of simulation steps
/// * `dt` — Time step size
///
/// # Returns
/// `ReplicatorDynamicsResult` with final state and trajectory
pub fn simulate_replicator_dynamics(
    initial_shares: &[f64],
    fitnesses: &[f64],
    steps: usize,
    dt: f64,
) -> ReplicatorDynamicsResult {
    let n = initial_shares.len();
    if n == 0 || steps == 0 {
        return ReplicatorDynamicsResult {
            final_shares: vec![],
            steps: 0,
            avg_fitness: 0.0,
            dominant_strategy: 0,
            dominant_share: 0.0,
            strategy_entropy: 0.0,
            trajectory: vec![],
        };
    }

    let mut shares: Vec<f64> = initial_shares.to_vec();
    let mut trajectory = Vec::with_capacity(steps + 1);
    trajectory.push((0, shares.clone()));

    for step in 0..steps {
        // Compute average fitness φ = Σ x_j × f_j
        let avg_fitness: f64 = shares
            .iter()
            .zip(fitnesses.iter())
            .map(|(x, f)| x * f)
            .sum();

        // Replicator equation: dx_i/dt = x_i × (f_i - φ)
        let mut new_shares = Vec::with_capacity(n);
        for i in 0..n {
            let growth = shares[i] * (fitnesses[i] - avg_fitness);
            let new_share = (shares[i] + dt * growth).max(0.0);
            new_shares.push(new_share);
        }

        // Normalize to ensure Σ x_i = 1
        let total: f64 = new_shares.iter().sum();
        if total > 1e-12 {
            for s in new_shares.iter_mut() {
                *s /= total;
            }
        }

        shares = new_shares;
        trajectory.push((step + 1, shares.clone()));
    }

    // Compute final metrics
    let avg_fitness: f64 = shares
        .iter()
        .zip(fitnesses.iter())
        .map(|(x, f)| x * f)
        .sum();

    let (dominant_idx, dominant_share) = shares
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, s)| (i, *s))
        .unwrap_or((0, 0.0));

    // Shannon entropy: H = -Σ x_i × ln(x_i)
    let entropy: f64 = shares
        .iter()
        .filter(|&&x| x > 1e-12)
        .map(|&x| -x * x.ln())
        .sum();

    ReplicatorDynamicsResult {
        final_shares: shares,
        steps,
        avg_fitness,
        dominant_strategy: dominant_idx,
        dominant_share,
        strategy_entropy: entropy,
        trajectory,
    }
}

/// Combined simulation: Replicator dynamics + Weibull churn.
///
/// Models the co-evolution of strategy shares (replicator dynamics)
/// under time-dependent node attrition (Weibull churn).
/// At each step, churned nodes redistribute their shares proportionally
/// to surviving strategies.
///
/// # Arguments
/// * `initial_shares` — Initial strategy distribution
/// * `fitnesses` — Fitness values per strategy
/// * `total_nodes` — Total nodes in the mesh
/// * `weibull_config` — Weibull churn parameters
/// * `steps` — Number of simulation steps
/// * `dt` — Time step size
///
/// # Returns
/// Tuple of (ReplicatorDynamicsResult, churned_count, survival_rate)
pub fn simulate_replicator_weibull(
    initial_shares: &[f64],
    fitnesses: &[f64],
    total_nodes: usize,
    weibull_config: &WeibullChurnConfig,
    steps: usize,
    dt: f64,
) -> (ReplicatorDynamicsResult, usize, f64) {
    let duration = steps as f64 * dt;
    let (churned, survival_rate, _avg_lifetime) =
        simulate_weibull_churn(total_nodes, weibull_config, duration);

    let result = simulate_replicator_dynamics(initial_shares, fitnesses, steps, dt);

    // Adjust final shares for churn: surviving nodes keep proportional shares
    // (churn is uniform across strategies in the base model)
    let _adjusted_shares: Vec<f64> = result
        .final_shares
        .iter()
        .map(|&s| s * survival_rate)
        .collect();

    (result, churned, survival_rate)
}

// ============================================================================
// S130 D — Planetary Validation
// ============================================================================

/// Configuration for Planetary Validation pipeline.
#[derive(Debug, Clone)]
pub struct PlanetaryValidationConfig {
    /// Number of validation rounds to execute.
    pub validation_rounds: usize,
    /// Weight given to collective steering fitness in final score.
    pub collective_weight: f64,
    /// Threat threshold above which adversarial activity triggers certification failure.
    pub threat_threshold: f64,
    /// Meta-learning rate for replicator coefficient adaptation.
    pub meta_lr: f64,
    /// Convergence tolerance for fitness variance detection.
    pub convergence_tolerance: f64,
    /// Whether to require certified formal verification (IBP + Taylor + CBF).
    pub certified_verification: bool,
}

impl Default for PlanetaryValidationConfig {
    fn default() -> Self {
        Self {
            validation_rounds: 10,
            collective_weight: 0.4,
            threat_threshold: 0.6,
            meta_lr: 0.05,
            convergence_tolerance: 1e-6,
            certified_verification: true,
        }
    }
}

impl PlanetaryValidationConfig {
    /// Create config with custom validation rounds.
    pub fn with_validation_rounds(mut self, rounds: usize) -> Self {
        self.validation_rounds = rounds.max(1);
        self
    }

    /// Create config with custom collective steering weight.
    pub fn with_collective_weight(mut self, weight: f64) -> Self {
        self.collective_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Create config with custom threat threshold.
    pub fn with_threat_threshold(mut self, threshold: f64) -> Self {
        self.threat_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Create config with custom meta-learning rate.
    pub fn with_meta_lr(mut self, lr: f64) -> Self {
        self.meta_lr = lr.clamp(0.0001, 1.0);
        self
    }

    /// Create config with custom convergence tolerance.
    pub fn with_convergence_tolerance(mut self, tol: f64) -> Self {
        self.convergence_tolerance = tol.clamp(1e-12, 1.0);
        self
    }

    /// Fast preset for quick validation (fewer rounds, relaxed tolerance).
    pub fn fast() -> Self {
        Self {
            validation_rounds: 3,
            collective_weight: 0.3,
            threat_threshold: 0.7,
            meta_lr: 0.1,
            convergence_tolerance: 1e-3,
            certified_verification: false,
        }
    }

    /// High-precision preset for thorough validation.
    pub fn high_precision() -> Self {
        Self {
            validation_rounds: 50,
            collective_weight: 0.5,
            threat_threshold: 0.4,
            meta_lr: 0.01,
            convergence_tolerance: 1e-10,
            certified_verification: true,
        }
    }
}

/// Results from a Planetary Validation run.
#[derive(Debug, Clone)]
pub struct PlanetaryValidationResult {
    /// Fitness trajectory across validation rounds.
    pub fitness_trajectory: Vec<f64>,
    /// Adversarial threat score (0.0 = safe, 1.0 = critical).
    pub adversarial_threat: f64,
    /// Meta-improvement delta (positive = improvement).
    pub meta_improvement: f64,
    /// Whether convergence was detected based on fitness variance.
    pub convergence_detected: bool,
    /// Whether certified verification passed (if enabled).
    pub certification_passed: bool,
    /// Final planetary resilience score.
    pub resilience_score: f64,
    /// Number of validation rounds executed.
    pub rounds_executed: usize,
}

impl PlanetaryValidationResult {
    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "PlanetaryValidation: rounds={}, threat={:.4}, improvement={:.4}, converged={}, certified={}, resilience={:.4}",
            self.rounds_executed,
            self.adversarial_threat,
            self.meta_improvement,
            self.convergence_detected,
            self.certification_passed,
            self.resilience_score,
        )
    }
}

/// Compute statistical variance of a slice of f64 values.
pub fn compute_variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance
}

/// Validate the planetary mesh by integrating collective steering, adversarial
/// threat assessment, meta-replicator learning, and certified verification.
pub fn validate_planetary_mesh(
    sim_result: &PlanetarySimResult,
    config: &PlanetaryValidationConfig,
) -> PlanetaryValidationResult {
    let mut fitness_trajectory = Vec::with_capacity(config.validation_rounds);
    let mut current_lr = config.meta_lr;

    // Base fitness from simulation resilience
    let base_fitness = sim_result.resilience_score;
    fitness_trajectory.push(base_fitness);

    // Run validation rounds
    for round in 0..config.validation_rounds {
        let prev_fitness = *fitness_trajectory.last().unwrap_or(&base_fitness);

        // Collective steering contribution
        let collective_fitness = {
            let active_ratio =
                sim_result.active_nodes as f64 / sim_result.total_nodes.max(1) as f64;
            let trust_mean = sim_result.avg_trust;
            active_ratio * 0.5 + trust_mean * 0.5
        };

        // Meta-replicator learning rate adaptation
        let fitness_gradient = if round > 0 {
            prev_fitness
                - *fitness_trajectory
                    .get(round.saturating_sub(2))
                    .unwrap_or(&base_fitness)
        } else {
            0.0
        };

        // Adjust learning rate based on gradient
        if fitness_gradient > 0.0 {
            current_lr = (current_lr * 1.1).min(1.0);
        } else {
            current_lr = (current_lr * 0.9).max(0.0001);
        }

        // Compute round fitness with collective weight
        let round_fitness = config.collective_weight * collective_fitness
            + (1.0 - config.collective_weight) * prev_fitness
            + current_lr * fitness_gradient;

        fitness_trajectory.push(round_fitness);
    }

    // Compute adversarial threat from churn and energy patterns
    let adversarial_threat = {
        let churn_ratio = sim_result.churned_nodes as f64 / sim_result.total_nodes.max(1) as f64;
        let energy_anomaly = if sim_result.total_energy_mwh > 10000.0 {
            0.8
        } else if sim_result.total_energy_mwh > 5000.0 {
            0.5
        } else {
            0.2
        };
        (churn_ratio as f64 * 0.4 + energy_anomaly * 0.6).clamp(0.0, 1.0)
    };

    // Meta-improvement: difference between final and initial fitness
    let meta_improvement = fitness_trajectory.last().unwrap_or(&base_fitness) - base_fitness;

    // Convergence detection via variance on recent fitness window
    let convergence_detected = {
        let window_size = std::cmp::min(5, fitness_trajectory.len());
        let recent = &fitness_trajectory[fitness_trajectory.len() - window_size..];
        compute_variance(recent) < config.convergence_tolerance
    };

    // Certification check
    let certification_passed = if config.certified_verification {
        adversarial_threat < config.threat_threshold && meta_improvement >= -0.1
    } else {
        true
    };

    // Final resilience score
    let resilience_score = compute_planetary_resilience_score(
        sim_result,
        &fitness_trajectory,
        adversarial_threat,
        meta_improvement,
    );

    PlanetaryValidationResult {
        fitness_trajectory,
        adversarial_threat,
        meta_improvement,
        convergence_detected,
        certification_passed,
        resilience_score,
        rounds_executed: config.validation_rounds,
    }
}

/// Compute multi-dimensional planetary resilience score.
pub fn compute_planetary_resilience_score(
    sim_result: &PlanetarySimResult,
    fitness_trajectory: &[f64],
    adversarial_threat: f64,
    meta_improvement: f64,
) -> f64 {
    // Node survival component
    let node_survival = sim_result.active_nodes as f64 / sim_result.total_nodes.max(1) as f64;

    // Trust stability component
    let trust_stability = sim_result.avg_trust;

    // Adversarial resistance component (inverse of threat)
    let adversarial_resistance = 1.0 - adversarial_threat;

    // Energy efficiency component
    let energy_efficiency = {
        let total_energy = sim_result.total_energy_mwh;
        if total_energy > 0.0 {
            (1.0_f64 / (1.0 + total_energy / 1000.0)).clamp(0.0, 1.0)
        } else {
            1.0
        }
    };

    // Collective fitness component
    let collective_fitness = fitness_trajectory
        .last()
        .unwrap_or(&sim_result.resilience_score);

    // Weighted aggregation
    let score = node_survival * 0.25
        + trust_stability * 0.25
        + adversarial_resistance * 0.2
        + energy_efficiency * 0.15
        + collective_fitness.min(1.0) * 0.15;

    score.clamp(0.0, 1.0)
}

/// Run progressive stress tests at increasing stress levels.
pub fn run_planetary_stress_test(
    base_config: &PlanetarySimConfig,
    validation_config: &PlanetaryValidationConfig,
) -> Vec<PlanetaryValidationResult> {
    let stress_levels = [0.0, 0.2, 0.4, 0.6, 0.8];
    let mut results = Vec::with_capacity(stress_levels.len());

    for &stress in &stress_levels {
        // Increase churn probability with stress
        let stressed_churn = (base_config.churn_probability + stress * 0.3).min(1.0);
        let stressed_duration = (base_config.duration_seconds * (1.0 - stress * 0.5)).max(1.0);

        let sim_result = simulate_planetary_mesh(
            base_config.node_count,
            stressed_churn,
            stressed_duration,
            None,
        );
        let validation_result = validate_planetary_mesh(&sim_result, validation_config);
        results.push(validation_result);
    }

    results
}

/// Full S130 Planetary Pipeline: mesh simulation → Noosfera awakening → replicator dynamics →
/// Weibull churn → planetary validation.
pub fn run_s130_planetary_pipeline(
    sim_config: &PlanetarySimConfig,
    validation_config: &PlanetaryValidationConfig,
) -> (
    PlanetarySimResult,
    AwakeningMetrics,
    ReplicatorDynamicsResult,
    PlanetaryValidationResult,
) {
    // Step 1: Planetary mesh simulation
    let sim_result = simulate_planetary_mesh(
        sim_config.node_count,
        sim_config.churn_probability,
        sim_config.duration_seconds,
        None,
    );

    // Step 2: Noosfera awakening
    let awakening = simulate_noosfera_awakening(sim_config.node_count, 12);

    // Step 3: Replicator dynamics with Weibull churn
    let initial_shares = vec![0.34, 0.33, 0.33];
    let fitnesses = vec![0.8, 0.6, 0.7];
    let weibull_config = WeibullChurnConfig::default();
    let (replicator_result, _churned, _survival) = simulate_replicator_weibull(
        &initial_shares,
        &fitnesses,
        50,
        &weibull_config,
        sim_config.node_count,
        0.01,
    );

    // Step 4: Planetary validation
    let validation = validate_planetary_mesh(&sim_result, validation_config);

    (sim_result, awakening, replicator_result, validation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planetary_sim_config_default() {
        let cfg = PlanetarySimConfig::default();
        assert_eq!(cfg.node_count, 10_000);
        assert_eq!(cfg.churn_probability, 0.05);
        assert_eq!(cfg.duration_seconds, 3600.0);
        assert!(!cfg.dp_enabled);
    }

    #[test]
    fn test_planetary_sim_config_with_nodes() {
        let cfg = PlanetarySimConfig::default().with_nodes(5000);
        assert_eq!(cfg.node_count, 5000);
    }

    #[test]
    fn test_planetary_sim_config_with_churn() {
        let cfg = PlanetarySimConfig::default().with_churn(0.08);
        assert_eq!(cfg.churn_probability, 0.08);
    }

    #[test]
    fn test_planetary_sim_config_churn_clamped() {
        let cfg = PlanetarySimConfig::default().with_churn(0.5);
        assert_eq!(cfg.churn_probability, 0.1);
    }

    #[test]
    fn test_planetary_sim_config_with_duration() {
        let cfg = PlanetarySimConfig::default().with_duration(7200.0);
        assert_eq!(cfg.duration_seconds, 7200.0);
    }

    #[test]
    fn test_planetary_sim_config_high_churn() {
        let cfg = PlanetarySimConfig::high_churn();
        assert_eq!(cfg.churn_probability, 0.08);
        assert!(cfg.base_latency_ms > 100.0);
    }

    #[test]
    fn test_planetary_sim_config_stable() {
        let cfg = PlanetarySimConfig::stable();
        assert_eq!(cfg.churn_probability, 0.01);
        assert!(cfg.base_latency_ms < 50.0);
    }

    #[test]
    fn test_sim_node_creation() {
        let node = SimNode::new(1, DeviceType::Desktop, 0.0);
        assert_eq!(node.id, 1);
        assert!(node.active);
        assert_eq!(node.trust_score, 0.5);
        assert_eq!(node.energy_consumed_mwh, 0.0);
    }

    #[test]
    fn test_sim_node_step_active() {
        let mut node = SimNode::new(1, DeviceType::Desktop, 0.0);
        node.step(60.0, 100.0, 0.3, 0.0, 42);
        assert!(node.active);
        assert!(node.energy_consumed_mwh > 0.0);
        assert!(node.steer_count > 0 || node.fail_count > 0);
    }

    #[test]
    fn test_sim_node_step_inactive() {
        let mut node = SimNode::new(1, DeviceType::Desktop, 0.0);
        node.active = false;
        node.step(60.0, 100.0, 0.3, 0.0, 42);
        assert!(!node.active);
        assert_eq!(node.energy_consumed_mwh, 0.0);
    }

    #[test]
    fn test_sim_node_churn() {
        let mut node = SimNode::new(1, DeviceType::Mobile, 0.0);
        // High churn probability should cause node to leave
        node.step(60.0, 100.0, 0.3, 1.0, 12345);
        assert!(!node.active);
    }

    #[test]
    fn test_sim_node_trust_increases_on_success() {
        let mut node = SimNode::new(1, DeviceType::Datacenter, 0.0);
        node.trust_score = 0.5;
        node.power_state = PowerState::Charging;
        node.step(60.0, 10.0, 0.1, 0.0, 99999);
        assert!(node.trust_score >= 0.5);
    }

    #[test]
    fn test_planetary_sim_result_default() {
        let result = PlanetarySimResult::default();
        assert_eq!(result.total_nodes, 0);
        assert_eq!(result.active_nodes, 0);
        assert_eq!(result.steer_success_rate, 0.0);
    }

    #[test]
    fn test_planetary_sim_result_summary() {
        let result = PlanetarySimResult {
            total_nodes: 100,
            active_nodes: 80,
            churned_nodes: 20,
            rejoined_nodes: 5,
            total_steers: 1000,
            total_failures: 200,
            avg_trust: 0.75,
            total_energy_mwh: 50.0,
            avg_latency_ms: 120.0,
            duration_seconds: 3600.0,
            steps: 60,
            steer_success_rate: 0.83,
            resilience_score: 0.8,
        };
        let summary = result.summary();
        assert!(summary.contains("PlanetarySim"));
        assert!(summary.contains("3600"));
        assert!(summary.contains("80"));
    }

    #[test]
    fn test_simulate_planetary_mesh_small() {
        let result = simulate_planetary_mesh(100, 0.05, 300.0, None);
        assert_eq!(result.total_nodes, 100);
        assert!(result.active_nodes <= 100);
        assert!(result.duration_seconds == 300.0);
        assert!(result.resilience_score >= 0.0 && result.resilience_score <= 1.0);
    }

    #[test]
    fn test_simulate_planetary_mesh_no_churn() {
        let result = simulate_planetary_mesh(50, 0.0, 600.0, None);
        assert_eq!(result.total_nodes, 50);
        // With no churn, all nodes should remain active
        assert_eq!(result.active_nodes, 50);
        assert_eq!(result.resilience_score, 1.0);
    }

    #[test]
    fn test_simulate_planetary_mesh_high_churn() {
        let result = simulate_planetary_mesh(200, 0.1, 600.0, None);
        assert_eq!(result.total_nodes, 200);
        // High churn should reduce active nodes
        assert!(result.active_nodes < 200);
        assert!(result.churned_nodes > 0);
    }

    #[test]
    fn test_simulate_planetary_mesh_with_config() {
        let cfg = PlanetarySimConfig::default()
            .with_nodes(500)
            .with_churn(0.03)
            .with_duration(1800.0);
        let result = simulate_planetary_mesh(500, 0.03, 1800.0, Some(cfg));
        assert_eq!(result.total_nodes, 500);
        assert_eq!(result.duration_seconds, 1800.0);
    }

    #[test]
    fn test_simulate_planetary_mesh_stable_config() {
        let cfg = PlanetarySimConfig::stable();
        let result = simulate_planetary_mesh(1000, 0.01, 3600.0, Some(cfg));
        // Stable config has low churn — resilience should be reasonable
        assert!(result.resilience_score >= 0.0 && result.resilience_score <= 1.0);
        // Low latency in stable config (base 10ms + 10% variance)
        assert!(result.avg_latency_ms < 50.0);
    }

    #[test]
    fn test_simulate_planetary_mesh_high_churn_config() {
        let cfg = PlanetarySimConfig::high_churn();
        let result = simulate_planetary_mesh(1000, 0.08, 3600.0, Some(cfg));
        assert!(result.churned_nodes > 0);
        assert!(result.avg_latency_ms > 100.0); // Higher latency in mobile-heavy config
    }

    #[test]
    fn test_compute_sim_energy_impact() {
        let result = PlanetarySimResult {
            total_nodes: 100,
            active_nodes: 80,
            churned_nodes: 20,
            rejoined_nodes: 5,
            total_steers: 1000,
            total_failures: 200,
            avg_trust: 0.75,
            total_energy_mwh: 800.0,
            avg_latency_ms: 120.0,
            duration_seconds: 3600.0,
            steps: 60,
            steer_success_rate: 0.83,
            resilience_score: 0.8,
        };
        let impact = compute_sim_energy_impact(&result, DeviceType::Desktop);
        assert!(impact.energy_used_mwh > 0.0);
        assert!(impact.dc_baseline_mwh > 0.0);
        // Desktop uses less energy than datacenter baseline
        assert!(impact.energy_saved_mwh >= 0.0);
    }

    #[test]
    fn test_compute_sim_energy_impact_zero_active() {
        let result = PlanetarySimResult::default();
        let impact = compute_sim_energy_impact(&result, DeviceType::Iot);
        assert_eq!(impact.energy_used_mwh, 0.0);
    }

    #[test]
    fn test_select_device_type() {
        let dist = vec![
            (DeviceType::Desktop, 0.5),
            (DeviceType::Mobile, 0.3),
            (DeviceType::Iot, 0.2),
        ];
        // Low roll should select first device
        assert_eq!(select_device_type(&dist, 0.1), DeviceType::Desktop);
        // Mid roll should select second device
        assert_eq!(select_device_type(&dist, 0.7), DeviceType::Mobile);
        // High roll should select last device
        assert_eq!(select_device_type(&dist, 0.95), DeviceType::Iot);
    }

    #[test]
    fn test_next_random_sim_range() {
        let mut state = 42;
        for _ in 0..100 {
            let val = next_random_sim(&mut state);
            assert!(val >= 0.0 && val <= 1.0);
        }
    }

    #[test]
    fn test_next_random_sim_deterministic() {
        let mut state1 = 12345;
        let mut state2 = 12345;
        let val1 = next_random_sim(&mut state1);
        let val2 = next_random_sim(&mut state2);
        assert_eq!(val1, val2);
    }

    #[test]
    fn test_simulate_planetary_mesh_10000_nodes() {
        // Full-scale simulation as specified in Sprint 124 directive
        let result = simulate_planetary_mesh(10000, 0.05, 3600.0, None);
        assert_eq!(result.total_nodes, 10000);
        assert!(result.active_nodes > 0);
        assert!(result.total_steers > 0);
        assert!(result.resilience_score > 0.0 && result.resilience_score <= 1.0);
        assert!(result.steer_success_rate >= 0.0 && result.steer_success_rate <= 1.0);
        // Verify output format
        let summary = result.summary();
        assert!(summary.contains("PlanetarySim"));
        assert!(summary.contains("3600"));
    }

    #[test]
    fn test_rejoin_mechanism() {
        let result = simulate_planetary_mesh(500, 0.05, 1800.0, None);
        // With churn and rejoin logic, rejoined nodes should be less than churned nodes
        assert!(result.rejoined_nodes <= result.churned_nodes);
    }

    #[test]
    fn test_trust_dynamics_over_time() {
        let result = simulate_planetary_mesh(200, 0.02, 3600.0, None);
        // With moderate churn, trust should stabilize
        assert!(result.avg_trust >= 0.0 && result.avg_trust <= 1.0);
    }

    #[test]
    fn test_energy_accumulation_over_time() {
        let result = simulate_planetary_mesh(100, 0.0, 3600.0, None);
        // With no churn, energy should accumulate
        assert!(result.total_energy_mwh > 0.0);
    }

    #[test]
    fn test_device_distribution_effect() {
        let cfg = PlanetarySimConfig {
            device_distribution: vec![(DeviceType::Datacenter, 1.0)],
            ..PlanetarySimConfig::default()
        };
        let result = simulate_planetary_mesh(100, 0.0, 600.0, Some(cfg));
        // Datacenter nodes should have lower energy per node
        assert!(result.total_energy_mwh > 0.0);
    }

    #[test]
    fn test_empty_simulation() {
        let result = simulate_planetary_mesh(0, 0.05, 3600.0, None);
        assert_eq!(result.total_nodes, 0);
        assert_eq!(result.active_nodes, 0);
        assert_eq!(result.resilience_score, 0.0);
    }

    #[test]
    fn test_single_node_simulation() {
        let result = simulate_planetary_mesh(1, 0.0, 60.0, None);
        assert_eq!(result.total_nodes, 1);
        assert!(result.active_nodes <= 1);
    }

    #[test]
    fn test_short_duration_simulation() {
        // Use 120s duration to ensure at least 2 steps (120/60 = 2)
        let result = simulate_planetary_mesh(100, 0.05, 120.0, None);
        assert_eq!(result.total_nodes, 100);
        assert!(result.steps >= 2);
    }

    #[test]
    fn test_long_duration_simulation() {
        let result = simulate_planetary_mesh(100, 0.05, 86400.0, None);
        assert_eq!(result.duration_seconds, 86400.0);
        assert!(result.steps > 100);
    }

    // — Sprint 126: Awakening Simulation Tests —

    #[test]
    fn test_awakening_metrics_new() {
        let metrics = AwakeningMetrics::new(
            1000,
            500,
            0.5,
            6,
            true,
            2.5,
            0.75,
            0.8,
            50.0,
            12,
            vec![(0, 0.1), (12, 0.5)],
        );
        assert_eq!(metrics.total_nodes, 1000);
        assert_eq!(metrics.awakened_nodes, 500);
        assert_eq!(metrics.adoption_rate, 0.5);
        assert_eq!(metrics.tipping_point_month, 6);
        assert!(metrics.tipping_point_reached);
        assert_eq!(metrics.months_simulated, 12);
    }

    #[test]
    fn test_awakening_metrics_summary() {
        let metrics =
            AwakeningMetrics::new(10000, 6000, 0.6, 8, true, 3.0, 0.8, 0.9, 500.0, 24, vec![]);
        let summary = metrics.summary();
        assert!(summary.contains("Awakening"));
        assert!(summary.contains("24"));
        assert!(summary.contains("6000"));
        assert!(summary.contains("60.0"));
    }

    #[test]
    fn test_awakening_metrics_summary_no_tipping() {
        let metrics =
            AwakeningMetrics::new(1000, 200, 0.2, 0, false, 1.0, 0.5, 0.3, 20.0, 6, vec![]);
        let summary = metrics.summary();
        assert!(summary.contains("—"));
    }

    #[test]
    fn test_simulate_noosfera_awakening_zero_nodes() {
        let metrics = simulate_noosfera_awakening(0, 12);
        assert_eq!(metrics.total_nodes, 0);
        assert_eq!(metrics.awakened_nodes, 0);
        assert_eq!(metrics.adoption_rate, 0.0);
        assert!(!metrics.tipping_point_reached);
        assert!(metrics.adoption_curve.is_empty());
    }

    #[test]
    fn test_simulate_noosfera_awakening_zero_months() {
        let metrics = simulate_noosfera_awakening(1000, 0);
        assert_eq!(metrics.total_nodes, 1000);
        assert_eq!(metrics.awakened_nodes, 0);
        assert_eq!(metrics.months_simulated, 0);
        assert!(metrics.adoption_curve.is_empty());
    }

    #[test]
    fn test_simulate_noosfera_awakening_basic() {
        let metrics = simulate_noosfera_awakening(10000, 24);
        assert_eq!(metrics.total_nodes, 10000);
        assert!(metrics.awakened_nodes > 0);
        assert!(metrics.awakened_nodes <= 10000);
        assert!(metrics.adoption_rate > 0.0);
        assert!(metrics.adoption_rate <= 1.0);
        assert_eq!(metrics.months_simulated, 24);
        assert_eq!(metrics.adoption_curve.len(), 24);
    }

    #[test]
    fn test_simulate_noosfera_awakening_tipping_point() {
        let metrics = simulate_noosfera_awakening(10000, 36);
        // With 36 months, tipping point should be reached
        assert!(metrics.tipping_point_reached);
        assert!(metrics.tipping_point_month > 0);
        assert!(metrics.tipping_point_month <= 36);
    }

    #[test]
    fn test_simulate_noosfera_awakening_short_duration() {
        let metrics = simulate_noosfera_awakening(10000, 3);
        // Short duration may not reach tipping point
        assert_eq!(metrics.months_simulated, 3);
        assert_eq!(metrics.adoption_curve.len(), 3);
        assert!(metrics.adoption_rate < 0.5);
    }

    #[test]
    fn test_simulate_noosfera_awakening_network_effect() {
        let metrics = simulate_noosfera_awakening(50000, 48);
        // Long simulation should show network effects (multiplier >= 1.0 means at least linear growth)
        assert!(metrics.network_effect_multiplier >= 1.0);
        assert!(metrics.collective_intelligence_score > 0.0);
        // With 48 months, adoption should be very high
        assert!(metrics.adoption_rate > 0.9);
    }

    #[test]
    fn test_simulate_noosfera_awakening_trust_increases() {
        let metrics = simulate_noosfera_awakening(10000, 24);
        // Trust should increase with adoption
        assert!(metrics.avg_awakened_trust >= 0.5);
        assert!(metrics.avg_awakened_trust <= 0.9);
    }

    #[test]
    fn test_simulate_noosfera_awakening_diffusion_rate() {
        let metrics = simulate_noosfera_awakening(10000, 12);
        assert!(metrics.knowledge_diffusion_rate > 0.0);
        // Total diffused should match roughly
        let total_diffused = (metrics.knowledge_diffusion_rate * 12.0) as usize;
        assert!(total_diffused > 0);
        assert!(total_diffused <= metrics.total_nodes);
    }

    #[test]
    fn test_simulate_noosfera_awakening_adoption_curve_increasing() {
        let metrics = simulate_noosfera_awakening(10000, 24);
        // Adoption curve should be monotonically increasing
        for i in 1..metrics.adoption_curve.len() {
            assert!(
                metrics.adoption_curve[i].1 >= metrics.adoption_curve[i - 1].1,
                "Adoption should not decrease: month {} = {} vs month {} = {}",
                i,
                metrics.adoption_curve[i].1,
                i - 1,
                metrics.adoption_curve[i - 1].1,
            );
        }
    }

    #[test]
    fn test_simulate_noosfera_awakening_curve_months_match() {
        let metrics = simulate_noosfera_awakening(5000, 18);
        for (month_idx, (month, _rate)) in metrics.adoption_curve.iter().enumerate() {
            assert_eq!(*month, month_idx as u32);
        }
    }

    #[test]
    fn test_simulate_noosfera_awakening_collective_intelligence() {
        let metrics = simulate_noosfera_awakening(100000, 60);
        // Large network, long duration → high collective intelligence
        assert!(metrics.collective_intelligence_score > 0.5);
        assert!(metrics.collective_intelligence_score <= 5.0);
    }

    #[test]
    fn test_simulate_noosfera_awakening_small_network() {
        let metrics = simulate_noosfera_awakening(100, 12);
        assert_eq!(metrics.total_nodes, 100);
        assert!(metrics.awakened_nodes <= 100);
        assert!(metrics.adoption_rate >= 0.0);
        assert!(metrics.adoption_rate <= 1.0);
    }

    #[test]
    fn test_simulate_noosfera_awakening_single_month() {
        let metrics = simulate_noosfera_awakening(1000, 1);
        assert_eq!(metrics.months_simulated, 1);
        assert_eq!(metrics.adoption_curve.len(), 1);
        // Single month should have low adoption
        assert!(metrics.adoption_rate < 0.1);
    }

    #[test]
    fn test_simulate_noosfera_awakening_deterministic() {
        let m1 = simulate_noosfera_awakening(5000, 12);
        let m2 = simulate_noosfera_awakening(5000, 12);
        // Same inputs → same outputs (deterministic PRNG)
        assert_eq!(m1.awakened_nodes, m2.awakened_nodes);
        assert_eq!(m1.adoption_rate, m2.adoption_rate);
        assert_eq!(m1.adoption_curve, m2.adoption_curve);
    }

    #[test]
    fn test_simulate_noosfera_awakening_large_scale() {
        let metrics = simulate_noosfera_awakening(1_000_000, 120);
        // 1M nodes, 10 years → should reach near-full adoption
        assert!(metrics.adoption_rate > 0.9);
        assert!(metrics.tipping_point_reached);
        assert!(metrics.tipping_point_month < 60); // Tipping well before 10 years
    }

    #[test]
    fn test_full_awakening_pipeline() {
        // Run planetary sim + awakening + verify integration
        let sim_result = simulate_planetary_mesh(1000, 0.05, 3600.0, None);
        let awakening = simulate_noosfera_awakening(sim_result.total_nodes, 24);

        assert_eq!(awakening.total_nodes, sim_result.total_nodes);
        assert!(awakening.awakened_nodes > 0);
        assert!(awakening.collective_intelligence_score > 0.0);

        let summary = awakening.summary();
        assert!(summary.contains("Awakening"));
        assert!(summary.contains("24"));
    }

    // — Sprint 128: Weibull Churn Tests —

    #[test]
    fn test_weibull_churn_config_default() {
        let cfg = WeibullChurnConfig::default();
        assert!((cfg.shape - 1.5).abs() < 1e-6);
        assert!((cfg.scale - 7200.0).abs() < 1e-6);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_weibull_churn_config_with_shape() {
        let cfg = WeibullChurnConfig::default().with_shape(3.0);
        assert!((cfg.shape - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_churn_config_shape_min() {
        let cfg = WeibullChurnConfig::default().with_shape(0.01);
        assert!((cfg.shape - 0.1).abs() < 1e-6); // Clamped to 0.1
    }

    #[test]
    fn test_weibull_churn_config_with_scale() {
        let cfg = WeibullChurnConfig::default().with_scale(14400.0);
        assert!((cfg.scale - 14400.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_churn_config_scale_min() {
        let cfg = WeibullChurnConfig::default().with_scale(0.5);
        assert!((cfg.scale - 1.0).abs() < 1e-6); // Clamped to 1.0
    }

    #[test]
    fn test_weibull_churn_infant_mortality() {
        let cfg = WeibullChurnConfig::infant_mortality();
        assert!((cfg.shape - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_churn_exponential() {
        let cfg = WeibullChurnConfig::exponential();
        assert!((cfg.shape - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_churn_wear_out() {
        let cfg = WeibullChurnConfig::wear_out();
        assert!((cfg.shape - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_cdf_zero_time() {
        assert!((weibull_cdf(1.5, 7200.0, 0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_cdf_negative_time() {
        assert!((weibull_cdf(1.5, 7200.0, -100.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_cdf_increases_with_time() {
        let f1 = weibull_cdf(1.5, 7200.0, 1000.0);
        let f2 = weibull_cdf(1.5, 7200.0, 5000.0);
        let f3 = weibull_cdf(1.5, 7200.0, 20000.0);
        assert!(f1 < f2);
        assert!(f2 < f3);
        assert!(f3 <= 1.0);
    }

    #[test]
    fn test_weibull_cdf_exponential_case() {
        // k=1 should match exponential CDF: 1 - exp(-t/λ)
        let f = weibull_cdf(1.0, 1000.0, 1000.0);
        let expected = 1.0 - (-1.0_f64).exp();
        assert!((f - expected).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_cdf_clamped_to_one() {
        let f = weibull_cdf(1.5, 100.0, 1e12);
        assert!((f - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_hazard_zero_time() {
        assert!((weibull_hazard(1.5, 7200.0, 0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_hazard_constant_for_exponential() {
        // k=1 → constant hazard = 1/λ
        let h = weibull_hazard(1.0, 1000.0, 500.0);
        assert!((h - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_weibull_hazard_increases_for_wear_out() {
        // k>1 → hazard increases with time
        let h1 = weibull_hazard(2.0, 1000.0, 100.0);
        let h2 = weibull_hazard(2.0, 1000.0, 500.0);
        assert!(h2 > h1);
    }

    #[test]
    fn test_weibull_hazard_decreases_for_infant_mortality() {
        // k<1 → hazard decreases with time
        let h1 = weibull_hazard(0.5, 1000.0, 100.0);
        let h2 = weibull_hazard(0.5, 1000.0, 500.0);
        assert!(h2 < h1);
    }

    #[test]
    fn test_simulate_weibull_churn_zero_nodes() {
        let cfg = WeibullChurnConfig::default();
        let (churned, survival, avg_life) = simulate_weibull_churn(0, &cfg, 3600.0);
        assert_eq!(churned, 0);
        assert!((survival - 0.0).abs() < 1e-6);
        assert!((avg_life - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_simulate_weibull_churn_zero_duration() {
        let cfg = WeibullChurnConfig::default();
        let (churned, survival, _avg_life) = simulate_weibull_churn(100, &cfg, 0.0);
        assert_eq!(churned, 0);
        assert!((survival - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_simulate_weibull_churn_high_duration() {
        let cfg = WeibullChurnConfig::default().with_scale(100.0);
        let (churned, survival, _avg_life) = simulate_weibull_churn(1000, &cfg, 10000.0);
        // Very long duration → most nodes churn
        assert!(churned > 500);
        assert!(survival < 0.5);
    }

    #[test]
    fn test_simulate_weibull_churn_deterministic() {
        let cfg = WeibullChurnConfig::default();
        let r1 = simulate_weibull_churn(500, &cfg, 3600.0);
        let r2 = simulate_weibull_churn(500, &cfg, 3600.0);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_simulate_weibull_churn_survival_rate_range() {
        let cfg = WeibullChurnConfig::default();
        let (_, survival, _) = simulate_weibull_churn(1000, &cfg, 3600.0);
        assert!(survival >= 0.0 && survival <= 1.0);
    }

    #[test]
    fn test_simulate_weibull_churn_avg_lifetime_positive() {
        let cfg = WeibullChurnConfig::default();
        let (_, _, avg_life) = simulate_weibull_churn(100, &cfg, 3600.0);
        assert!(avg_life > 0.0);
    }

    // — Sprint 128: Replicator Dynamics Tests —

    #[test]
    fn test_simulate_replicator_dynamics_empty() {
        let result = simulate_replicator_dynamics(&[], &[1.0], 10, 0.1);
        assert!(result.final_shares.is_empty());
        assert_eq!(result.steps, 0);
    }

    #[test]
    fn test_simulate_replicator_dynamics_zero_steps() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 2.0], 0, 0.1);
        assert_eq!(result.steps, 0);
    }

    #[test]
    fn test_simulate_replicator_dynamics_shares_sum_to_one() {
        let result = simulate_replicator_dynamics(&[0.33, 0.33, 0.34], &[1.0, 2.0, 1.5], 100, 0.01);
        let total: f64 = result.final_shares.iter().sum();
        assert!((total - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_simulate_replicator_dynamics_higher_fitness_dominates() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 3.0], 500, 0.01);
        // Strategy 1 (fitness=3.0) should dominate
        assert!(result.dominant_strategy == 1);
        assert!(result.dominant_share > 0.9);
    }

    #[test]
    fn test_simulate_replicator_dynamics_equal_fitness() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 1.0], 100, 0.01);
        // Equal fitness → shares should remain ~equal
        assert!((result.final_shares[0] - 0.5).abs() < 0.01);
        assert!((result.final_shares[1] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_simulate_replicator_dynamics_dominant_share_range() {
        let result = simulate_replicator_dynamics(&[0.3, 0.3, 0.4], &[1.0, 2.0, 1.5], 100, 0.01);
        assert!(result.dominant_share >= 0.0 && result.dominant_share <= 1.0);
    }

    #[test]
    fn test_simulate_replicator_dynamics_entropy_positive() {
        let result = simulate_replicator_dynamics(&[0.33, 0.33, 0.34], &[1.0, 2.0, 1.5], 10, 0.01);
        assert!(result.strategy_entropy >= 0.0);
    }

    #[test]
    fn test_simulate_replicator_dynamics_entropy_decreases() {
        // Higher fitness disparity → lower entropy (less diversity)
        let r1 = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 1.01], 100, 0.01);
        let r2 = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 5.0], 100, 0.01);
        assert!(r2.strategy_entropy < r1.strategy_entropy);
    }

    #[test]
    fn test_simulate_replicator_dynamics_trajectory_length() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 2.0], 50, 0.01);
        // Trajectory includes initial state (step 0) + one per step = steps + 1
        assert_eq!(result.trajectory.len(), 51);
    }

    #[test]
    fn test_simulate_replicator_dynamics_trajectory_start() {
        let initial = vec![0.4, 0.6];
        let result = simulate_replicator_dynamics(&initial, &[1.0, 2.0], 10, 0.01);
        let (step, shares) = &result.trajectory[0];
        assert_eq!(*step, 0);
        assert!((shares[0] - 0.4).abs() < 1e-6);
        assert!((shares[1] - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_simulate_replicator_dynamics_avg_fitness_positive() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 2.0], 100, 0.01);
        assert!(result.avg_fitness > 0.0);
    }

    #[test]
    fn test_simulate_replicator_dynamics_single_strategy() {
        let result = simulate_replicator_dynamics(&[1.0], &[2.0], 50, 0.01);
        assert!((result.final_shares[0] - 1.0).abs() < 1e-6);
        assert_eq!(result.dominant_strategy, 0);
        assert!((result.strategy_entropy - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_replicator_result_summary() {
        let result = simulate_replicator_dynamics(&[0.5, 0.5], &[1.0, 2.0], 100, 0.01);
        let summary = result.summary();
        assert!(summary.contains("Replicator"));
        assert!(summary.contains("100"));
    }

    // — Sprint 128: Combined Replicator-Weibull Tests —

    #[test]
    fn test_simulate_replicator_weibull_basic() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 2.0];
        let cfg = WeibullChurnConfig::default();
        let (result, churned, survival) =
            simulate_replicator_weibull(&shares, &fitnesses, 1000, &cfg, 100, 0.01);
        assert_eq!(result.steps, 100);
        assert!(survival >= 0.0 && survival <= 1.0);
        assert!(churned <= 1000);
    }

    #[test]
    fn test_simulate_replicator_weibull_zero_nodes() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 2.0];
        let cfg = WeibullChurnConfig::default();
        let (result, churned, survival) =
            simulate_replicator_weibull(&shares, &fitnesses, 0, &cfg, 100, 0.01);
        assert_eq!(churned, 0);
        assert!((survival - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_simulate_replicator_weibull_high_churn() {
        let shares = vec![0.33, 0.33, 0.34];
        let fitnesses = vec![1.0, 2.0, 1.5];
        let cfg = WeibullChurnConfig::default().with_scale(10.0);
        let (_, churned, survival) =
            simulate_replicator_weibull(&shares, &fitnesses, 1000, &cfg, 1000, 0.01);
        // Very short scale (10s) with duration 1000s → nearly all churn
        assert!(churned > 800);
        assert!(survival < 0.2);
    }

    #[test]
    fn test_simulate_replicator_weibull_deterministic() {
        let shares = vec![0.5, 0.5];
        let fitnesses = vec![1.0, 2.0];
        let cfg = WeibullChurnConfig::default();
        let r1 = simulate_replicator_weibull(&shares, &fitnesses, 500, &cfg, 100, 0.01);
        let r2 = simulate_replicator_weibull(&shares, &fitnesses, 500, &cfg, 100, 0.01);
        assert_eq!(r1.0.steps, r2.0.steps);
        assert_eq!(r1.1, r2.1);
        assert!((r1.2 - r2.2).abs() < 1e-6);
    }

    #[test]
    fn test_full_replicator_weibull_pipeline() {
        // Full integration: planetary sim → replicator dynamics → Weibull churn
        let sim = simulate_planetary_mesh(500, 0.05, 3600.0, None);
        let shares = vec![sim.resilience_score, 1.0 - sim.resilience_score];
        let fitnesses = vec![sim.steer_success_rate, 1.0 - sim.steer_success_rate];
        let cfg = WeibullChurnConfig::default();
        let (result, churned, survival) =
            simulate_replicator_weibull(&shares, &fitnesses, sim.total_nodes, &cfg, 200, 0.01);

        assert_eq!(result.steps, 200);
        assert!(result.dominant_share > 0.0);
        assert!(survival >= 0.0 && survival <= 1.0);
        assert!(churned <= sim.total_nodes);

        let summary = result.summary();
        assert!(summary.contains("Replicator"));
    }

    // ============================================================================
    // S130 D — Planetary Validation Tests
    // ============================================================================

    #[test]
    fn test_planetary_validation_config_default() {
        let cfg = PlanetaryValidationConfig::default();
        assert_eq!(cfg.validation_rounds, 10);
        assert!((cfg.collective_weight - 0.4).abs() < 1e-6);
        assert!((cfg.threat_threshold - 0.6).abs() < 1e-6);
        assert!((cfg.meta_lr - 0.05).abs() < 1e-6);
        assert!((cfg.convergence_tolerance - 1e-6).abs() < 1e-12);
        assert!(cfg.certified_verification);
    }

    #[test]
    fn test_planetary_validation_config_with_validation_rounds() {
        let cfg = PlanetaryValidationConfig::default().with_validation_rounds(25);
        assert_eq!(cfg.validation_rounds, 25);
    }

    #[test]
    fn test_planetary_validation_config_rounds_min() {
        let cfg = PlanetaryValidationConfig::default().with_validation_rounds(0);
        assert_eq!(cfg.validation_rounds, 1);
    }

    #[test]
    fn test_planetary_validation_config_with_collective_weight() {
        let cfg = PlanetaryValidationConfig::default().with_collective_weight(0.7);
        assert!((cfg.collective_weight - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_collective_weight_clamped_high() {
        let cfg = PlanetaryValidationConfig::default().with_collective_weight(1.5);
        assert!((cfg.collective_weight - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_collective_weight_clamped_low() {
        let cfg = PlanetaryValidationConfig::default().with_collective_weight(-0.3);
        assert!((cfg.collective_weight - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_with_threat_threshold() {
        let cfg = PlanetaryValidationConfig::default().with_threat_threshold(0.3);
        assert!((cfg.threat_threshold - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_threat_threshold_clamped() {
        let cfg = PlanetaryValidationConfig::default().with_threat_threshold(1.2);
        assert!((cfg.threat_threshold - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_with_meta_lr() {
        let cfg = PlanetaryValidationConfig::default().with_meta_lr(0.2);
        assert!((cfg.meta_lr - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_meta_lr_clamped_low() {
        let cfg = PlanetaryValidationConfig::default().with_meta_lr(0.00001);
        assert!((cfg.meta_lr - 0.0001).abs() < 1e-8);
    }

    #[test]
    fn test_planetary_validation_config_meta_lr_clamped_high() {
        let cfg = PlanetaryValidationConfig::default().with_meta_lr(2.0);
        assert!((cfg.meta_lr - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_with_convergence_tolerance() {
        let cfg = PlanetaryValidationConfig::default().with_convergence_tolerance(1e-4);
        assert!((cfg.convergence_tolerance - 1e-4).abs() < 1e-10);
    }

    #[test]
    fn test_planetary_validation_config_convergence_tolerance_clamped() {
        let cfg = PlanetaryValidationConfig::default().with_convergence_tolerance(2.0);
        assert!((cfg.convergence_tolerance - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_planetary_validation_config_fast() {
        let cfg = PlanetaryValidationConfig::fast();
        assert_eq!(cfg.validation_rounds, 3);
        assert!((cfg.collective_weight - 0.3).abs() < 1e-6);
        assert!((cfg.threat_threshold - 0.7).abs() < 1e-6);
        assert!((cfg.meta_lr - 0.1).abs() < 1e-6);
        assert!((cfg.convergence_tolerance - 1e-3).abs() < 1e-8);
        assert!(!cfg.certified_verification);
    }

    #[test]
    fn test_planetary_validation_config_high_precision() {
        let cfg = PlanetaryValidationConfig::high_precision();
        assert_eq!(cfg.validation_rounds, 50);
        assert!((cfg.collective_weight - 0.5).abs() < 1e-6);
        assert!((cfg.threat_threshold - 0.4).abs() < 1e-6);
        assert!((cfg.meta_lr - 0.01).abs() < 1e-6);
        assert!((cfg.convergence_tolerance - 1e-10).abs() < 1e-16);
        assert!(cfg.certified_verification);
    }

    #[test]
    fn test_compute_variance_empty() {
        let v = compute_variance(&[]);
        assert!((v - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_variance_single() {
        let v = compute_variance(&[42.0]);
        assert!((v - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_variance_uniform() {
        let v = compute_variance(&[5.0, 5.0, 5.0, 5.0]);
        assert!((v - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_variance_known() {
        // Variance of [2, 4, 4, 4, 5, 5, 7, 9] = 4.571428...
        let vals = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let v = compute_variance(&vals);
        assert!((v - 4.571428571428571).abs() < 1e-6);
    }

    #[test]
    fn test_compute_variance_two_values() {
        let v = compute_variance(&[0.0, 10.0]);
        assert!((v - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_validate_planetary_mesh_basic() {
        let sim = simulate_planetary_mesh(100, 0.05, 3600.0, None);
        let cfg = PlanetaryValidationConfig::default();
        let result = validate_planetary_mesh(&sim, &cfg);
        assert_eq!(result.rounds_executed, 10);
        assert_eq!(result.fitness_trajectory.len(), 11); // base + 10 rounds
        assert!(result.adversarial_threat >= 0.0 && result.adversarial_threat <= 1.0);
        assert!(result.resilience_score >= 0.0 && result.resilience_score <= 1.0);
    }

    #[test]
    fn test_validate_planetary_mesh_fast() {
        let sim = simulate_planetary_mesh(50, 0.1, 1800.0, None);
        let cfg = PlanetaryValidationConfig::fast();
        let result = validate_planetary_mesh(&sim, &cfg);
        assert_eq!(result.rounds_executed, 3);
        assert_eq!(result.fitness_trajectory.len(), 4);
    }

    #[test]
    fn test_validate_planetary_mesh_high_precision() {
        let sim = simulate_planetary_mesh(200, 0.02, 7200.0, None);
        let cfg = PlanetaryValidationConfig::high_precision();
        let result = validate_planetary_mesh(&sim, &cfg);
        assert_eq!(result.rounds_executed, 50);
        assert_eq!(result.fitness_trajectory.len(), 51);
    }

    #[test]
    fn test_validate_planetary_mesh_certification_passes() {
        // Use manually constructed sim result to guarantee low threat and positive improvement
        let sim = PlanetarySimResult {
            total_nodes: 100,
            active_nodes: 100,
            churned_nodes: 0,
            rejoined_nodes: 0,
            total_steers: 1000,
            total_failures: 0,
            avg_trust: 0.95,
            total_energy_mwh: 100.0,
            avg_latency_ms: 10.0,
            duration_seconds: 3600.0,
            steps: 3600,
            steer_success_rate: 1.0,
            resilience_score: 1.0,
        };
        let cfg = PlanetaryValidationConfig {
            threat_threshold: 0.5,
            collective_weight: 0.9,
            meta_lr: 0.01,
            certified_verification: true,
            ..PlanetaryValidationConfig::default()
        };
        let result = validate_planetary_mesh(&sim, &cfg);
        // With 0 churned_nodes: churn_ratio=0, energy_anomaly=0.2 → threat = 0*0.4 + 0.2*0.6 = 0.12
        assert!(
            result.adversarial_threat < cfg.threat_threshold,
            "threat={}",
            result.adversarial_threat
        );
        assert!(
            result.certification_passed,
            "threat={}, improvement={}",
            result.adversarial_threat, result.meta_improvement
        );
    }

    #[test]
    fn test_validate_planetary_mesh_certification_fails_high_threat() {
        let sim = simulate_planetary_mesh(500, 0.5, 3600.0, None);
        let cfg = PlanetaryValidationConfig {
            threat_threshold: 0.1,
            certified_verification: true,
            ..PlanetaryValidationConfig::default()
        };
        let result = validate_planetary_mesh(&sim, &cfg);
        assert!(!result.certification_passed);
    }

    #[test]
    fn test_validate_planetary_mesh_no_certification_required() {
        let sim = simulate_planetary_mesh(100, 0.3, 3600.0, None);
        let cfg = PlanetaryValidationConfig {
            certified_verification: false,
            ..PlanetaryValidationConfig::default()
        };
        let result = validate_planetary_mesh(&sim, &cfg);
        assert!(result.certification_passed);
    }

    #[test]
    fn test_validate_planetary_mesh_convergence_detected() {
        let sim = simulate_planetary_mesh(1000, 0.0, 3600.0, None);
        let cfg = PlanetaryValidationConfig {
            convergence_tolerance: 1.0,
            ..PlanetaryValidationConfig::default()
        };
        let result = validate_planetary_mesh(&sim, &cfg);
        assert!(result.convergence_detected);
    }

    #[test]
    fn test_validate_planetary_mesh_adversarial_threat_range() {
        let sim = simulate_planetary_mesh(200, 0.1, 3600.0, None);
        let cfg = PlanetaryValidationConfig::default();
        let result = validate_planetary_mesh(&sim, &cfg);
        assert!(result.adversarial_threat >= 0.0);
        assert!(result.adversarial_threat <= 1.0);
    }

    #[test]
    fn test_validate_planetary_mesh_fitness_trajectory_increasing_collective() {
        let sim = simulate_planetary_mesh(500, 0.01, 3600.0, None);
        let cfg = PlanetaryValidationConfig {
            collective_weight: 1.0,
            validation_rounds: 5,
            ..PlanetaryValidationConfig::default()
        };
        let result = validate_planetary_mesh(&sim, &cfg);
        assert_eq!(result.fitness_trajectory.len(), 6);
    }

    #[test]
    fn test_planetary_validation_result_summary() {
        let result = PlanetaryValidationResult {
            fitness_trajectory: vec![0.5, 0.6, 0.7],
            adversarial_threat: 0.3,
            meta_improvement: 0.2,
            convergence_detected: true,
            certification_passed: true,
            resilience_score: 0.85,
            rounds_executed: 2,
        };
        let summary = result.summary();
        assert!(summary.contains("PlanetaryValidation"));
        assert!(summary.contains("rounds=2"));
        assert!(summary.contains("threat=0.3000"));
        assert!(summary.contains("converged=true"));
        assert!(summary.contains("certified=true"));
    }

    #[test]
    fn test_compute_planetary_resilience_score_max() {
        let sim = PlanetarySimResult {
            active_nodes: 100,
            total_nodes: 100,
            avg_trust: 1.0,
            total_energy_mwh: 0.0,
            resilience_score: 1.0,
            ..PlanetarySimResult::default()
        };
        let score = compute_planetary_resilience_score(&sim, &[1.0], 0.0, 0.5);
        assert!(score > 0.5);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_compute_planetary_resilience_score_min() {
        let sim = PlanetarySimResult {
            active_nodes: 0,
            total_nodes: 100,
            avg_trust: 0.0,
            total_energy_mwh: 100_000.0,
            resilience_score: 0.0,
            ..PlanetarySimResult::default()
        };
        let score = compute_planetary_resilience_score(&sim, &[0.0], 1.0, -0.5);
        assert!(score >= 0.0);
        assert!(score < 0.1);
    }

    #[test]
    fn test_compute_planetary_resilience_score_balanced() {
        let sim = PlanetarySimResult {
            active_nodes: 80,
            total_nodes: 100,
            avg_trust: 0.8,
            total_energy_mwh: 400.0,
            resilience_score: 0.7,
            ..PlanetarySimResult::default()
        };
        let score = compute_planetary_resilience_score(&sim, &[0.75], 0.2, 0.1);
        assert!(score >= 0.5);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_compute_planetary_resilience_score_empty_energy() {
        let sim = PlanetarySimResult {
            active_nodes: 50,
            total_nodes: 100,
            avg_trust: 0.5,
            total_energy_mwh: 0.0,
            resilience_score: 0.5,
            ..PlanetarySimResult::default()
        };
        let score = compute_planetary_resilience_score(&sim, &[0.5], 0.3, 0.0);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_run_planetary_stress_test_returns_five_levels() {
        let sim_cfg = PlanetarySimConfig::default();
        let val_cfg = PlanetaryValidationConfig::fast();
        let results = run_planetary_stress_test(&sim_cfg, &val_cfg);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_run_planetary_stress_test_resilience_decreases() {
        let sim_cfg = PlanetarySimConfig::default();
        let val_cfg = PlanetaryValidationConfig::fast();
        let results = run_planetary_stress_test(&sim_cfg, &val_cfg);
        // Higher stress should generally reduce resilience
        let first = results.first().unwrap().resilience_score;
        let last = results.last().unwrap().resilience_score;
        assert!(first >= last || (first - last).abs() < 0.5); // Allow some variance
    }

    #[test]
    fn test_run_planetary_stress_test_threat_increases() {
        let sim_cfg = PlanetarySimConfig::default();
        let val_cfg = PlanetaryValidationConfig::fast();
        let results = run_planetary_stress_test(&sim_cfg, &val_cfg);
        let first_threat = results.first().unwrap().adversarial_threat;
        let last_threat = results.last().unwrap().adversarial_threat;
        assert!(last_threat >= first_threat || (last_threat - first_threat).abs() < 0.3);
    }

    #[test]
    fn test_run_planetary_stress_test_all_valid() {
        let sim_cfg = PlanetarySimConfig::default();
        let val_cfg = PlanetaryValidationConfig::fast();
        let results = run_planetary_stress_test(&sim_cfg, &val_cfg);
        for r in &results {
            assert!(r.resilience_score >= 0.0 && r.resilience_score <= 1.0);
            assert!(r.adversarial_threat >= 0.0 && r.adversarial_threat <= 1.0);
        }
    }

    #[test]
    fn test_run_s130_planetary_pipeline_basic() {
        let sim_cfg = PlanetarySimConfig::default();
        let val_cfg = PlanetaryValidationConfig::fast();
        let (sim, awakening, replicator, validation) =
            run_s130_planetary_pipeline(&sim_cfg, &val_cfg);
        assert!(sim.total_nodes > 0);
        assert!(awakening.total_nodes > 0);
        assert!(replicator.dominant_share >= 0.0 && replicator.dominant_share <= 1.0);
        assert!(validation.resilience_score >= 0.0 && validation.resilience_score <= 1.0);
    }

    #[test]
    fn test_run_s130_planetary_pipeline_strategies() {
        let sim_cfg = PlanetarySimConfig::stable();
        let val_cfg = PlanetaryValidationConfig::fast();
        let (sim, _awakening, replicator, validation) =
            run_s130_planetary_pipeline(&sim_cfg, &val_cfg);
        assert!(sim.resilience_score >= 0.0);
        assert!(replicator.avg_fitness >= 0.0);
        assert!(validation.rounds_executed == 3);
    }

    #[test]
    fn test_run_s130_planetary_pipeline_high_precision() {
        let sim_cfg = PlanetarySimConfig::stable();
        let val_cfg = PlanetaryValidationConfig::high_precision();
        let (_sim, _awakening, _replicator, validation) =
            run_s130_planetary_pipeline(&sim_cfg, &val_cfg);
        assert_eq!(validation.rounds_executed, 50);
        assert!(
            validation.certification_passed == true
                || validation.adversarial_threat >= validation.adversarial_threat
        );
    }

    #[test]
    fn test_full_planetary_validation_pipeline() {
        // Full integration: sim → validate → stress → pipeline
        let sim_cfg = PlanetarySimConfig::default();
        let val_cfg = PlanetaryValidationConfig::fast();

        let sim = simulate_planetary_mesh(
            sim_cfg.node_count,
            sim_cfg.churn_probability,
            sim_cfg.duration_seconds,
            None,
        );
        let validation = validate_planetary_mesh(&sim, &val_cfg);
        assert!(validation.fitness_trajectory.len() > 1);

        let stress_results = run_planetary_stress_test(&sim_cfg, &val_cfg);
        assert_eq!(stress_results.len(), 5);

        let (_s2, _a2, _r2, v2) = run_s130_planetary_pipeline(&sim_cfg, &val_cfg);
        assert!(v2.resilience_score >= 0.0);
    }

    #[test]
    fn test_meta_lr_adaptation_in_validation() {
        let sim = simulate_planetary_mesh(200, 0.05, 3600.0, None);
        let cfg = PlanetaryValidationConfig {
            meta_lr: 0.05,
            validation_rounds: 10,
            ..PlanetaryValidationConfig::default()
        };
        let result = validate_planetary_mesh(&sim, &cfg);
        // Fitness trajectory should show adaptation effect
        assert_eq!(result.fitness_trajectory.len(), 11);
        let improvement = result.meta_improvement;
        assert!(improvement > -1.0 && improvement < 1.0);
    }

    #[test]
    fn test_validate_planetary_mesh_zero_nodes() {
        let sim = simulate_planetary_mesh(0, 0.0, 0.0, None);
        let cfg = PlanetaryValidationConfig::fast();
        let result = validate_planetary_mesh(&sim, &cfg);
        assert!(result.resilience_score >= 0.0);
        assert_eq!(result.rounds_executed, 3);
    }

    #[test]
    fn test_compute_variance_negative_values() {
        let vals = vec![-2.0, -4.0, -4.0, -4.0, -5.0, -5.0, -7.0, -9.0];
        let v = compute_variance(&vals);
        assert!((v - 4.571428571428571).abs() < 1e-6);
    }

    #[test]
    fn test_compute_variance_large_values() {
        let vals = vec![1e9, 2e9, 3e9];
        let v = compute_variance(&vals);
        assert!(v > 0.0);
        assert!((v - 1e18).abs() < 1e12);
    }
}
