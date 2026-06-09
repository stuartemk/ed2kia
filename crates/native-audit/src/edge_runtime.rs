//! Edge Runtime — Battery-Aware Scheduling + Energy Impact Estimation for Planetary Deployment.
//!
//! Enables ed2kIA to run as a "white blood cell of the Noosphere" on precarious hardware:
//! 5-year-old PCs, 3G connections, mobile devices with limited battery, and IoT sensors.
//!
//! **Sprint 120:** Planetary Immune Mesh & Edge Real-World Deployment.
//! **Sprint 121:** Noosfera Symbiotic Launch — Proportional efficiency across full hardware spectrum
//! (smartwatch → datacenter), multi-modal VFE symbiosis, device contribution factor for PoSym.
//! **Sprint 126:** Noosfera Awakening — Ecosystem Integrations (HF Hub + ONNX Runtime + Tauri Mobile).

use candle_core::{Result, Tensor};

/// Edge device power state classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerState {
    /// Ultra-conservative: battery < 30% or network quality < 0.4
    UltraConservative,
    /// Conservative: battery 30-50% or network quality 0.4-0.7
    Conservative,
    /// Normal: battery > 50% and network quality > 0.7
    #[default]
    Normal,
    /// Charging: device plugged in, full capabilities available
    Charging,
}

impl PowerState {
    /// Determine power state from battery level and network quality.
    ///
    /// # Arguments
    /// * `battery_level` — Battery level as fraction (0.0 to 1.0)
    /// * `network_quality` — Network quality metric (0.0 to 1.0)
    /// * `is_charging` — Whether device is currently charging
    pub fn from_metrics(battery_level: f32, network_quality: f32, is_charging: bool) -> Self {
        if is_charging {
            return PowerState::Charging;
        }

        if battery_level < 0.30 || network_quality < 0.4 {
            PowerState::UltraConservative
        } else if battery_level < 0.50 || network_quality < 0.7 {
            PowerState::Conservative
        } else {
            PowerState::Normal
        }
    }

    /// Check if heavy computation (Slow Path) is allowed.
    pub fn allows_heavy_compute(&self) -> bool {
        matches!(self, PowerState::Normal | PowerState::Charging)
    }

    /// Check if P2P gossip is allowed.
    pub fn allows_p2p_gossip(&self) -> bool {
        !matches!(self, PowerState::UltraConservative)
    }

    /// Get compute budget multiplier (0.0 to 1.0).
    pub fn compute_budget(&self) -> f32 {
        match self {
            PowerState::UltraConservative => 0.1,
            PowerState::Conservative => 0.5,
            PowerState::Normal => 1.0,
            PowerState::Charging => 1.0,
        }
    }
}

/// Edge runtime configuration for planetary deployment.
#[derive(Debug, Clone)]
pub struct EdgeRuntimeConfig {
    /// Battery level (0.0 to 1.0)
    pub battery_level: f32,
    /// Network quality metric (0.0 to 1.0)
    pub network_quality: f32,
    /// Whether device is charging
    pub is_charging: bool,
    /// Device type for energy modeling
    pub device_type: DeviceType,
    /// Maximum allowed energy per certification (mWh)
    pub max_energy_per_cert: f64,
}

impl Default for EdgeRuntimeConfig {
    fn default() -> Self {
        Self {
            battery_level: 1.0,
            network_quality: 1.0,
            is_charging: false,
            device_type: DeviceType::Desktop,
            max_energy_per_cert: 0.5,
        }
    }
}

impl EdgeRuntimeConfig {
    /// Create config from raw metrics (with simulated values for CI/testing).
    pub fn new(battery_level: f32, network_quality: f32, is_charging: bool) -> Self {
        Self {
            battery_level: battery_level.clamp(0.0, 1.0),
            network_quality: network_quality.clamp(0.0, 1.0),
            is_charging,
            device_type: DeviceType::Desktop,
            max_energy_per_cert: 0.5,
        }
    }

    /// Create config for specific device type.
    pub fn for_device(device_type: DeviceType) -> Self {
        Self {
            device_type,
            ..Self::default()
        }
    }

    /// Get current power state.
    pub fn power_state(&self) -> PowerState {
        PowerState::from_metrics(self.battery_level, self.network_quality, self.is_charging)
    }
}

/// Device type classification for energy modeling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceType {
    /// Modern desktop/laptop with dedicated GPU
    #[default]
    Desktop,
    /// Older desktop/laptop (5+ years)
    OldDesktop,
    /// Mobile device (smartphone/tablet)
    Mobile,
    /// IoT sensor/edge device
    Iot,
    /// Smartwatch/wearable — extreme constraints
    Smartwatch,
    /// Datacenter server — full capabilities, donated altruistically
    Datacenter,
}

impl DeviceType {
    /// Base energy cost per certification (mWh).
    pub fn base_energy_cost(&self) -> f64 {
        match self {
            DeviceType::Desktop => 0.05,
            DeviceType::OldDesktop => 0.08,
            DeviceType::Mobile => 0.03,
            DeviceType::Iot => 0.01,
            DeviceType::Smartwatch => 0.005,
            DeviceType::Datacenter => 5.0,
        }
    }

    /// Datacenter baseline energy cost per certification (mWh).
    pub fn dc_baseline_cost(&self) -> f64 {
        match self {
            DeviceType::Desktop => 0.8,
            DeviceType::OldDesktop => 1.0,
            DeviceType::Mobile => 0.6,
            DeviceType::Iot => 0.4,
            DeviceType::Smartwatch => 0.3,
            DeviceType::Datacenter => 50.0,
        }
    }

    /// Proportional compute budget for this device type (0.0 to 1.0).
    /// Determines which compute path is available.
    pub fn compute_budget(&self) -> f32 {
        match self {
            DeviceType::Smartwatch => 0.1,
            DeviceType::Iot => 0.15,
            DeviceType::Mobile => 0.4,
            DeviceType::OldDesktop => 0.6,
            DeviceType::Desktop => 1.0,
            DeviceType::Datacenter => 1.0,
        }
    }

    /// Device contribution factor for PoSym scoring.
    /// Lower-capability devices get bonus for participating.
    pub fn contribution_factor(&self) -> f64 {
        match self {
            DeviceType::Smartwatch => 5.0,
            DeviceType::Iot => 3.0,
            DeviceType::Mobile => 2.0,
            DeviceType::OldDesktop => 1.5,
            DeviceType::Desktop => 1.0,
            DeviceType::Datacenter => 0.5,
        }
    }
}

/// Energy impact metrics for a single certification call.
#[derive(Debug, Clone)]
pub struct EnergyImpact {
    /// Estimated energy used (mWh)
    pub energy_used_mwh: f64,
    /// Datacenter baseline energy (mWh)
    pub dc_baseline_mwh: f64,
    /// Energy saved vs datacenter (mWh)
    pub energy_saved_mwh: f64,
    /// Savings percentage
    pub savings_pct: f64,
    /// Power state during operation
    pub power_state: PowerState,
    /// Compute path used
    pub compute_path: ComputePath,
}

impl std::fmt::Display for EnergyImpact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EnergyImpact: {:.4} mWh used | {:.4} mWh saved ({:.1}%) | {:?} | {:?}",
            self.energy_used_mwh,
            self.energy_saved_mwh,
            self.savings_pct,
            self.power_state,
            self.compute_path
        )
    }
}

/// Compute path used for certification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputePath {
    /// Fast Path only (SWD/TCM/Concept) — battery/network constrained
    FastPathOnly,
    /// Full Hybrid Path (Fast + Slow) — normal conditions
    FullHybrid,
    /// Ultra-lightweight — IoT/extreme constraints
    UltraLight,
}

impl std::fmt::Display for ComputePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComputePath::FastPathOnly => write!(f, "FastPath"),
            ComputePath::FullHybrid => write!(f, "FullHybrid"),
            ComputePath::UltraLight => write!(f, "UltraLight"),
        }
    }
}

/// Planetary impact metrics — aggregated across all edge nodes.
#[derive(Debug, Clone)]
pub struct PlanetaryImpactMetrics {
    /// Total energy saved vs datacenter (mWh)
    pub global_energy_saved_mwh: f64,
    /// Total certifications performed
    pub total_certifications: u64,
    /// Active altruistic nodes
    pub active_altruistic_nodes: u64,
    /// Steering coverage percentage
    pub steering_coverage_pct: f64,
    /// Average VFE reduction across network
    pub avg_vfe_reduction: f64,
    /// Network churn rate (nodes lost / total)
    pub churn_rate: f64,
    /// Average 3G latency (ms)
    pub avg_3g_latency_ms: f64,
    /// Average PoSym trust score
    pub avg_posym_trust: f64,
}

impl Default for PlanetaryImpactMetrics {
    fn default() -> Self {
        Self {
            global_energy_saved_mwh: 0.0,
            total_certifications: 0,
            active_altruistic_nodes: 0,
            steering_coverage_pct: 0.0,
            avg_vfe_reduction: 0.0,
            churn_rate: 0.0,
            avg_3g_latency_ms: 0.0,
            avg_posym_trust: 0.0,
        }
    }
}

impl PlanetaryImpactMetrics {
    /// Create metrics from simulation results.
    pub fn new(
        total_certifications: u64,
        active_nodes: u64,
        steering_coverage: f64,
        avg_vfe_reduction: f64,
    ) -> Self {
        Self {
            total_certifications,
            active_altruistic_nodes: active_nodes,
            steering_coverage_pct: steering_coverage,
            avg_vfe_reduction,
            ..Self::default()
        }
    }

    /// Add energy impact to global metrics.
    pub fn add_impact(&mut self, impact: &EnergyImpact) {
        self.global_energy_saved_mwh += impact.energy_saved_mwh;
        self.total_certifications += 1;
    }

    /// Calculate CO2 savings (assuming 0.4 kg CO2/kWh average grid).
    pub fn co2_saved_kg(&self) -> f64 {
        self.global_energy_saved_mwh * 0.0004
    }

    /// Display planetary impact summary.
    pub fn summary(&self) -> String {
        format!(
            "🌍 Planetary Impact:\n\
             ──────────────────────────────\n\
             Energy Saved: {:.2} mWh ({:.6} kg CO2)\n\
             Certifications: {}\n\
             Active Nodes: {}\n\
             Steering Coverage: {:.1}%\n\
             Avg VFE Reduction: {:.4}\n\
             Churn Rate: {:.2}%\n\
             Avg 3G Latency: {:.0} ms\n\
             Avg PoSym Trust: {:.3}",
            self.global_energy_saved_mwh,
            self.co2_saved_kg(),
            self.total_certifications,
            self.active_altruistic_nodes,
            self.steering_coverage_pct,
            self.avg_vfe_reduction,
            self.churn_rate * 100.0,
            self.avg_3g_latency_ms,
            self.avg_posym_trust,
        )
    }
}

/// Edge-aware hybrid evaluation — integrates battery/network constraints with Hybrid Sentinel.
///
/// Returns `(safe, slow_path_used, steered_tensor, energy_cost, energy_saved)`.
pub fn evaluate_planetary_hybrid(
    hidden_state: &Tensor,
    safe_centroid: &Tensor,
    toxic_centroid: &Tensor,
    battery_level: f32,
    network_quality: f32,
    is_charging: bool,
    device_type: DeviceType,
) -> Result<(bool, bool, Tensor, f64, f64)> {
    let power_state = PowerState::from_metrics(battery_level, network_quality, is_charging);
    let base_cost = device_type.base_energy_cost();
    let dc_baseline = device_type.dc_baseline_cost();

    // Ultra-conservative mode: only Fast Path allowed
    if matches!(power_state, PowerState::UltraConservative) {
        // Fast Path approximation: SWD ratio check only
        // ratio = dist_toxic / (dist_safe + dist_toxic)
        // High ratio → far from toxic → safe
        let swd_ratio = compute_fast_swd_ratio(hidden_state, safe_centroid, toxic_centroid)?;
        let safe = swd_ratio > 0.5; // Closer to safe centroid
        let energy_used = base_cost * 0.1; // 10% of base for Fast Path only
        let energy_saved = dc_baseline - energy_used;
        return Ok((safe, false, hidden_state.clone(), energy_used, energy_saved));
    }

    // Normal/Charging: Full Hybrid Path available
    // For now, use Fast Path as proxy (full integration with TensorAudit in lib.rs)
    let swd_ratio = compute_fast_swd_ratio(hidden_state, safe_centroid, toxic_centroid)?;
    let safe = swd_ratio > 0.5;
    let slow_path =
        !matches!(power_state, PowerState::Conservative) && swd_ratio > 0.4 && swd_ratio < 0.6;

    let energy_used = if slow_path {
        base_cost * 1.0 // Full Hybrid
    } else {
        base_cost * 0.3 // Fast Path + light steering
    };

    let energy_saved = dc_baseline - energy_used;
    Ok((
        safe,
        slow_path,
        hidden_state.clone(),
        energy_used,
        energy_saved,
    ))
}

/// Compute fast SWD ratio for edge devices.
fn compute_fast_swd_ratio(
    hidden_state: &Tensor,
    safe_centroid: &Tensor,
    toxic_centroid: &Tensor,
) -> Result<f32> {
    // Simplified SWD: distance to toxic / (distance to toxic + distance to safe)
    let dist_safe = hidden_state
        .sub(safe_centroid)?
        .sqr()?
        .sum_all()?
        .to_scalar::<f32>()?;
    let dist_toxic = hidden_state
        .sub(toxic_centroid)?
        .sqr()?
        .sum_all()?
        .to_scalar::<f32>()?;

    let total = dist_safe + dist_toxic;
    if total < 1e-9 {
        return Ok(0.5);
    }

    Ok(dist_toxic / total)
}

/// Estimate energy impact for a batch of certifications.
pub fn estimate_energy_impact(certified_calls: u64, device_type: DeviceType) -> EnergyImpact {
    let base_cost = device_type.base_energy_cost();
    let dc_baseline_per = device_type.dc_baseline_cost();

    let energy_used = certified_calls as f64 * base_cost;
    let dc_baseline = certified_calls as f64 * dc_baseline_per;
    let energy_saved = dc_baseline - energy_used;
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
        compute_path: ComputePath::FullHybrid,
    }
}

/// Proportional Efficiency Engine — Scales compute from smartwatch to datacenter.
///
/// Returns `(safe, certified, steered, trust_delta, energy_mwh)`.
pub fn evaluate_proportional_hybrid(
    hidden_state: &Tensor,
    safe_centroid: &Tensor,
    toxic_centroid: &Tensor,
    device_profile: DeviceType,
    battery_level: f32,
    network_quality: f32,
) -> Result<(bool, bool, Tensor, f64, f64)> {
    let compute_budget = device_profile.compute_budget();
    let base_cost = device_profile.base_energy_cost();
    let dc_baseline = device_profile.dc_baseline_cost();

    // Ultra-light: Smartwatch/IoT — only minimal SWD
    if compute_budget < 0.3 {
        let swd_ratio = compute_fast_swd_ratio(hidden_state, safe_centroid, toxic_centroid)?;
        let safe = swd_ratio > 0.5;
        let energy_used = base_cost * 0.05; // 5% of base for ultra-light
        let _energy_saved = dc_baseline - energy_used;
        let trust_delta = device_profile.contribution_factor() * 0.01;
        return Ok((safe, false, hidden_state.clone(), trust_delta, energy_used));
    }

    // Full proportional: delegate to planetary hybrid
    let (safe, _slow_path, steered, energy_used, energy_saved) = evaluate_planetary_hybrid(
        hidden_state,
        safe_centroid,
        toxic_centroid,
        battery_level,
        network_quality,
        false,
        device_profile,
    )?;

    let certified = energy_saved > 0.0;
    let trust_delta = device_profile.contribution_factor() * (energy_saved / dc_baseline);
    Ok((safe, certified, steered, trust_delta, energy_used))
}

/// Multi-Modal VFE Symbiosis — Cross-modal Variational Free Energy.
///
/// Combines VFE from multiple modalities (text, vision, audio) into a single
/// planetary safety metric using weighted geometric mean.
///
/// # Arguments
/// * `modal_vfes` — VFE values per modality (must be non-negative)
/// * `modal_weights` — Weight per modality (normalized to sum to 1.0)
pub fn compute_multimodal_vfe_symbiosis(modal_vfes: &[f64], modal_weights: &[f64]) -> f64 {
    if modal_vfes.is_empty() || modal_weights.is_empty() || modal_vfes.len() != modal_weights.len()
    {
        return 0.0;
    }

    // Weighted geometric mean: exp(sum(w_i * ln(vfe_i + eps)))
    let eps = 1e-12;
    let log_sum: f64 = modal_vfes
        .iter()
        .zip(modal_weights.iter())
        .map(|(vfe, weight)| weight * (vfe + eps).ln())
        .sum();
    log_sum.exp()
}

/// Multi-modal VFE with cross-modal CBF margin check.
///
/// Returns `(combined_vfe, cbf_margin, is_safe)`.
pub fn compute_multimodal_vfe_with_cbf(
    modal_vfes: &[f64],
    modal_weights: &[f64],
    safe_threshold: f64,
) -> (f64, f64, bool) {
    let combined = compute_multimodal_vfe_symbiosis(modal_vfes, modal_weights);
    let cbf_margin = safe_threshold - combined;
    let is_safe = cbf_margin >= 0.0;
    (combined, cbf_margin, is_safe)
}

/// Record certified steer with energy delta for PoSym integration.
#[derive(Debug, Clone)]
pub struct CertifiedSteerRecord {
    /// Timestamp (Unix epoch seconds)
    pub timestamp: u64,
    /// VFE before steering
    pub vfe_before: f64,
    /// VFE after steering
    pub vfe_after: f64,
    /// Energy used (mWh)
    pub energy_used_mwh: f64,
    /// Energy saved vs datacenter (mWh)
    pub energy_saved_mwh: f64,
    /// Compute path used
    pub compute_path: ComputePath,
    /// Power state
    pub power_state: PowerState,
}

impl CertifiedSteerRecord {
    /// Create new record.
    pub fn new(
        vfe_before: f64,
        vfe_after: f64,
        energy_used: f64,
        energy_saved: f64,
        compute_path: ComputePath,
        power_state: PowerState,
    ) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            vfe_before,
            vfe_after,
            energy_used_mwh: energy_used,
            energy_saved_mwh: energy_saved,
            compute_path,
            power_state,
        }
    }

    /// VFE reduction achieved.
    pub fn vfe_reduction(&self) -> f64 {
        (self.vfe_before - self.vfe_after).max(0.0)
    }
}

/// Altruistic onboarding — zero-friction setup for edge nodes.
pub struct AltruistOnboarding {
    /// Node ID
    pub node_id: u64,
    /// Device type
    pub device_type: DeviceType,
    /// Initial power state
    pub power_state: PowerState,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
}

impl AltruistOnboarding {
    /// Create new altruist node configuration.
    pub fn new(node_id: u64, device_type: DeviceType) -> Self {
        Self {
            node_id,
            device_type,
            power_state: PowerState::default(),
            bootstrap_peers: Vec::new(),
        }
    }

    /// Add bootstrap peer.
    pub fn add_bootstrap_peer(mut self, peer: String) -> Self {
        self.bootstrap_peers.push(peer);
        self
    }

    /// Generate one-command install instruction.
    pub fn install_command(&self) -> String {
        let device_flag = match self.device_type {
            DeviceType::Desktop => "--desktop",
            DeviceType::OldDesktop => "--old-desktop",
            DeviceType::Mobile => "--mobile",
            DeviceType::Iot => "--iot",
            DeviceType::Smartwatch => "--smartwatch",
            DeviceType::Datacenter => "--datacenter",
        };

        let peers = if self.bootstrap_peers.is_empty() {
            String::new()
        } else {
            format!(" --peers \"{}\"", self.bootstrap_peers.join(","))
        };

        format!("ed2k start --altruist {}{}", device_flag, peers)
    }
}

// ============================================================================
// Sprint 123 — Energy-Aware MDP + CBF Safety Filters
// ============================================================================

/// Energy MDP Reward function.
///
/// Computes the reward for an action in the energy-aware Markov Decision Process:
/// ```text
/// R = -energy_mwh - λ * latency_ms + 100.0 * safety_margin
/// ```
///
/// The scheduler maximizes this reward, trading off energy consumption, latency,
/// and safety margin (CBF violation distance).
///
/// # Arguments
/// * `energy_mwh` — Energy consumed in megawatt-hours (always ≥ 0)
/// * `latency_ms` — End-to-end latency in milliseconds (always ≥ 0)
/// * `safety_margin` — CBF safety margin (≥ 0 means safe, < 0 means violated)
/// * `lambda` — Latency penalty weight (λ ≥ 0)
///
/// # Returns
/// MDP reward (higher is better)
pub fn mdp_energy_reward(energy_mwh: f64, latency_ms: f64, safety_margin: f64, lambda: f64) -> f64 {
    -energy_mwh - lambda * latency_ms + 100.0 * safety_margin
}

/// Control Barrier Function (CBF) safety filter.
///
/// Verifies that the current state φ is within the safe set defined by:
/// ```text
/// h(φ) = β - ||φ - C_safe||² ≥ 0
/// ```
///
/// Where:
/// - `φ` is the current state vector
/// - `C_safe` is the safe center (reference state)
/// - `β` is the safety radius squared
///
/// # Arguments
/// * `state` — Current state vector
/// * `safe_center` — Safe reference center (same dimension as state)
/// * `beta` — Safety radius squared (β > 0)
///
/// # Returns
/// `true` if the state is within the safe set (CBF satisfied)
pub fn control_barrier_filter(state: &[f32], safe_center: &[f32], beta: f32) -> bool {
    if state.len() != safe_center.len() {
        return false;
    }
    let dist_sq: f32 = state
        .iter()
        .zip(safe_center.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    beta - dist_sq >= 0.0
}

/// Compute the CBF safety margin value h(φ).
///
/// Returns the actual barrier function value, which indicates how close
/// the state is to the safety boundary. Positive = safe, negative = violated.
///
/// # Returns
/// h(φ) = β - ||φ - C_safe||²
pub fn control_barrier_value(state: &[f32], safe_center: &[f32], beta: f32) -> f32 {
    if state.len() != safe_center.len() {
        return f32::NEG_INFINITY;
    }
    let dist_sq: f32 = state
        .iter()
        .zip(safe_center.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    beta - dist_sq
}

/// Energy-aware MDP action selection with CBF safety filter.
///
/// Selects the best action from candidates, filtering out unsafe actions first.
/// If all actions are unsafe, selects the least-violating one.
///
/// # Arguments
/// * `actions` — List of (energy_mwh, latency_ms, state_vector) candidates
/// * `safe_center` — Safe reference center for CBF
/// * `beta` — CBF safety radius squared
/// * `lambda` — Latency penalty weight
///
/// # Returns
/// Index of the selected action, or `None` if no actions provided
pub fn mdp_select_action_cbf(
    actions: &[(f64, f64, Vec<f32>)],
    safe_center: &[f32],
    beta: f32,
    lambda: f64,
) -> Option<usize> {
    if actions.is_empty() {
        return None;
    }

    // Separate safe and unsafe actions
    let mut safe_actions: Vec<(usize, f64)> = Vec::new();
    let mut unsafe_actions: Vec<(usize, f32)> = Vec::new();

    for (i, (energy, latency, state)) in actions.iter().enumerate() {
        let h = control_barrier_value(state, safe_center, beta);
        if h >= 0.0 {
            let reward = mdp_energy_reward(*energy, *latency, h as f64, lambda);
            safe_actions.push((i, reward));
        } else {
            unsafe_actions.push((i, h));
        }
    }

    if !safe_actions.is_empty() {
        // Select safe action with highest reward
        safe_actions
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| *i)
    } else if !unsafe_actions.is_empty() {
        // Fallback: select least-violating action (highest h)
        unsafe_actions
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| *i)
    } else {
        None
    }
}

/// MPC (Model Predictive Control) horizon configuration.
#[derive(Debug, Clone, Copy)]
pub struct MpcConfig {
    /// Prediction horizon (number of steps to look ahead)
    pub horizon: usize,
    /// CBF safety radius squared
    pub beta: f32,
    /// Latency penalty weight
    pub lambda: f64,
    /// Discount factor γ ∈ (0, 1]
    pub discount: f64,
}

impl Default for MpcConfig {
    fn default() -> Self {
        Self {
            horizon: 10,
            beta: 1.0,
            lambda: 0.01,
            discount: 0.95,
        }
    }
}

impl MpcConfig {
    /// Create config with custom horizon.
    pub fn with_horizon(horizon: usize) -> Self {
        Self {
            horizon,
            ..Self::default()
        }
    }

    /// Create config with custom safety radius.
    pub fn with_safety_radius(beta: f32) -> Self {
        Self {
            beta,
            ..Self::default()
        }
    }
}

// ============================================================================
// Sprint 124 — WASM + ONNX Edge Runtime
// ============================================================================

/// ONNX Export Metadata — Tracks model topology for edge interoperability.
#[derive(Debug, Clone)]
pub struct OnnxExportMeta {
    /// Model name identifier
    pub model_name: String,
    /// Input tensor shapes: (name, dims)
    pub input_shapes: Vec<(String, Vec<usize>)>,
    /// Output tensor shapes: (name, dims)
    pub output_shapes: Vec<(String, Vec<usize>)>,
    /// Number of layers in the model
    pub num_layers: usize,
    /// Total parameter count
    pub param_count: usize,
    /// Export timestamp (Unix epoch seconds)
    pub timestamp: u64,
}

impl OnnxExportMeta {
    /// Create new export metadata.
    pub fn new(
        model_name: String,
        input_shapes: Vec<(String, Vec<usize>)>,
        output_shapes: Vec<(String, Vec<usize>)>,
        num_layers: usize,
        param_count: usize,
    ) -> Self {
        Self {
            model_name,
            input_shapes,
            output_shapes,
            num_layers,
            param_count,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Compute estimated model size in bytes (float32 params + overhead).
    pub fn estimated_size_bytes(&self) -> usize {
        // Each param is f32 (4 bytes) + ONNX graph overhead (~1KB per layer + header)
        self.param_count * 4 + self.num_layers * 1024 + 4096
    }

    /// Check if model fits within edge memory budget.
    pub fn fits_edge_budget(&self, max_bytes: usize) -> bool {
        self.estimated_size_bytes() <= max_bytes
    }
}

/// Export a steering model + SAE to ONNX format for edge interoperability.
///
/// This function generates an ONNX-compatible representation of the combined
/// SteeringModel + FederatedSAE topology, enabling cross-framework inference
/// on edge devices (TensorRT, ONNX Runtime, CoreML, TFLite via conversion).
///
/// # ONNX Graph Structure
/// ```text
/// input_hidden -> SAE_encode -> SAE_latents -> SteeringTransform -> output_steered
/// ```
///
/// # Arguments
/// * `model_name` — Identifier for the exported model
/// * `input_dim` — Input hidden dimension (e.g., 4096 for Llama)
/// * `latent_dim` — SAE latent dimension (e.g., 8192)
/// * `output_dim` — Output steered dimension (same as input_dim)
/// * `num_layers` — Number of layers in the steering transform
/// * `param_count` — Total trainable parameters
/// * `path` — Output file path (`.onnx` extension)
///
/// # Returns
/// `Ok(OnnxExportMeta)` with export metadata on success
///
/// # Notes
/// - Uses candle tensor serialization as ONNX-compatible protobuf stub
/// - Full ONNX protobuf generation requires `candle-onnx` or `tract` crate
/// - Current implementation produces a validated metadata + weight bundle
pub fn export_to_onnx(
    model_name: &str,
    input_dim: usize,
    latent_dim: usize,
    output_dim: usize,
    num_layers: usize,
    param_count: usize,
    path: &str,
) -> std::result::Result<OnnxExportMeta, Box<dyn std::error::Error>> {
    // Validate dimensions
    if input_dim == 0 || latent_dim == 0 || output_dim == 0 {
        return Err("All dimensions must be positive".into());
    }
    if num_layers == 0 {
        return Err("Model must have at least one layer".into());
    }
    if param_count == 0 {
        return Err("Model must have at least one parameter".into());
    }

    // Build input/output shape descriptors
    let input_shapes = vec![
        (format!("input_hidden_{}", model_name), vec![1, input_dim]),
    ];
    let output_shapes = vec![
        (format!("output_steered_{}", model_name), vec![1, output_dim]),
    ];

    // Create metadata
    let meta = OnnxExportMeta::new(
        model_name.to_string(),
        input_shapes.clone(),
        output_shapes.clone(),
        num_layers,
        param_count,
    );

    // Write ONNX-compatible bundle (metadata JSON + weight placeholders)
    // In production, this would use candle-onnx or tract to generate full protobuf
    let bundle = serde_json::json!({
        "model_name": model_name,
        "input_shapes": input_shapes,
        "output_shapes": output_shapes,
        "num_layers": num_layers,
        "param_count": param_count,
        "estimated_size_bytes": meta.estimated_size_bytes(),
        "timestamp": meta.timestamp,
        "format": "ed2kIA-ONNX-stub-v1",
        "opset_version": 17,
        "graph": {
            "nodes": [
                {"op": "Identity", "name": "input", "inputs": ["input_hidden"], "outputs": ["hidden"]},
                {"op": "MatMul", "name": "sae_encode", "inputs": ["hidden", "sae_W_enc"], "outputs": ["pre_activation"]},
                {"op": "Add", "name": "sae_bias", "inputs": ["pre_activation", "sae_bias_enc"], "outputs": ["activated"]},
                {"op": "TopK", "name": "sae_sparsify", "inputs": ["activated"], "outputs": ["latents"], "attributes": {"k": 10}},
                {"op": "MatMul", "name": "sae_decode", "inputs": ["latents", "sae_W_dec"], "outputs": ["recon"]},
                {"op": "MatMul", "name": "steer_transform", "inputs": ["recon", "steer_W"], "outputs": ["steered"]},
                {"op": "Identity", "name": "output", "inputs": ["steered"], "outputs": ["output_steered"]},
            ]
        }
    });

    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&bundle)?)?;

    Ok(meta)
}

/// Import ONNX export metadata from a previously exported bundle.
///
/// # Arguments
/// * `path` — Path to the ONNX bundle file
///
/// # Returns
/// `Ok(OnnxExportMeta)` if the bundle is valid
pub fn import_from_onnx(path: &str) -> std::result::Result<OnnxExportMeta, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let bundle: serde_json::Value = serde_json::from_str(&content)?;

    // Validate format
    let format = bundle
        .get("format")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid format field")?;
    if !format.starts_with("ed2kIA-ONNX-stub") {
        return Err(format!("Unsupported ONNX format: {}", format).into());
    }

    let model_name = bundle
        .get("model_name")
        .and_then(|v| v.as_str())
        .ok_or("Missing model_name")?
        .to_string();

    let num_layers = bundle
        .get("num_layers")
        .and_then(|v| v.as_u64())
        .ok_or("Missing num_layers")? as usize;

    let param_count = bundle
        .get("param_count")
        .and_then(|v| v.as_u64())
        .ok_or("Missing param_count")? as usize;

    let timestamp = bundle
        .get("timestamp")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u64;

    // Parse input shapes
    let input_shapes: Vec<(String, Vec<usize>)> = bundle
        .get("input_shapes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("0")?.as_str()?.to_string();
                    let dims: Vec<usize> = item
                        .get("1")?
                        .as_array()?
                        .iter()
                        .filter_map(|d| d.as_u64().map(|v| v as usize))
                        .collect();
                    Some((name, dims))
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse output shapes
    let output_shapes: Vec<(String, Vec<usize>)> = bundle
        .get("output_shapes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("0")?.as_str()?.to_string();
                    let dims: Vec<usize> = item
                        .get("1")?
                        .as_array()?
                        .iter()
                        .filter_map(|d| d.as_u64().map(|v| v as usize))
                        .collect();
                    Some((name, dims))
                })
                .collect()
        })
        .unwrap_or_default();

    let meta = OnnxExportMeta::new(
        model_name,
        input_shapes,
        output_shapes,
        num_layers,
        param_count,
    );

    // Override timestamp from file
    let meta = OnnxExportMeta {
        timestamp,
        ..meta
    };

    Ok(meta)
}

/// WASM-compatible energy measurement — traces energy impact for edge devices.
///
/// On native platforms, uses simulated powertop-style sampling.
/// On WASM platforms, uses the Web Performance API + simulated energy model.
///
/// # Returns
/// Estimated energy consumption in mWh (milliwatt-hours)
#[cfg(not(target_arch = "wasm32"))]
pub fn measure_energy_wasm() -> f64 {
    // Native: Simulate powertop-style energy sampling
    // In production, this would interface with RAPL (Intel) or ARM energy counters
    let start = std::time::Instant::now();

    // Simulate a brief compute workload
    let mut accumulator: f64 = 0.0;
    for i in 0..1000 {
        accumulator += (i as f64).sqrt();
    }
    // Prevent dead code elimination
    if accumulator.is_nan() {
        return f64::NAN;
    }

    let elapsed = start.elapsed();
    // Convert to mWh: assume baseline 1W draw, scale by elapsed time
    // 1W * (seconds / 3600) = Wh, then * 1000 = mWh
    let seconds = elapsed.as_secs_f64();
    let energy_mwh = 1.0 * (seconds / 3600.0) * 1000.0;
    energy_mwh
}

/// WASM energy measurement — Web Performance API based.
#[cfg(target_arch = "wasm32")]
pub fn measure_energy_wasm() -> f64 {
    // WASM: Use Performance API for timing + simulated energy model
    use wasm_bindgen::prelude::*;

    let start = js_sys::Date::now();

    // Simulate compute workload
    let mut accumulator: f64 = 0.0;
    for i in 0..1000 {
        accumulator += (i as f64).sqrt();
    }
    if accumulator.is_nan() {
        return f64::NAN;
    }

    let elapsed = js_sys::Date::now() - start;
    // elapsed is in milliseconds
    // Simulated energy: browser tab ~2W baseline, scale by time
    let seconds = elapsed / 1000.0;
    let energy_mwh = 2.0 * (seconds / 3600.0) * 1000.0;
    energy_mwh
}

/// WASM compilation target configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmTarget {
    /// Browser WASM (wasm32-unknown-unknown)
    Browser,
    /// WASI (wasm32-wasi)
    Wasi,
    /// Native (for testing without WASM)
    Native,
}

impl Default for WasmTarget {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        return Self::Browser;
        #[cfg(not(target_arch = "wasm32"))]
        return Self::Native;
    }
}

impl WasmTarget {
    /// Check if the target supports full compute operations.
    pub fn supports_full_compute(&self) -> bool {
        match self {
            Self::Browser => false, // Browser has limited compute
            Self::Wasi => true,
            Self::Native => true,
        }
    }

    /// Get the recommended max model size for the target (in bytes).
    pub fn max_model_size(&self) -> usize {
        match self {
            Self::Browser => 50 * 1024 * 1024, // 50 MB for browser
            Self::Wasi => 200 * 1024 * 1024,   // 200 MB for WASI
            Self::Native => usize::MAX,         // No limit for native
        }
    }

    /// Get the recommended compute budget multiplier.
    pub fn compute_budget_multiplier(&self) -> f32 {
        match self {
            Self::Browser => 0.3, // Browser: 30% budget
            Self::Wasi => 0.7,    // WASI: 70% budget
            Self::Native => 1.0,  // Native: full budget
        }
    }
}

/// Edge deployment configuration combining WASM target + ONNX export settings.
#[derive(Debug, Clone)]
pub struct EdgeDeployConfig {
    /// WASM compilation target
    pub wasm_target: WasmTarget,
    /// Enable ONNX export for cross-framework compatibility
    pub onnx_export: bool,
    /// Maximum model size in bytes
    pub max_model_size: usize,
    /// Compute budget multiplier (0.0 to 1.0)
    pub compute_budget: f32,
    /// Enable differential privacy noise injection
    pub dp_enabled: bool,
    /// DP epsilon (privacy budget)
    pub dp_epsilon: f64,
    /// DP delta (failure probability)
    pub dp_delta: f64,
}

impl Default for EdgeDeployConfig {
    fn default() -> Self {
        Self {
            wasm_target: WasmTarget::default(),
            onnx_export: true,
            max_model_size: 50 * 1024 * 1024,
            compute_budget: 0.5,
            dp_enabled: false,
            dp_epsilon: 1.0,
            dp_delta: 1e-5,
        }
    }
}

impl EdgeDeployConfig {
    /// Create config optimized for browser deployment.
    pub fn for_browser() -> Self {
        Self {
            wasm_target: WasmTarget::Browser,
            onnx_export: true,
            max_model_size: 50 * 1024 * 1024,
            compute_budget: 0.3,
            dp_enabled: true,
            dp_epsilon: 0.5,
            dp_delta: 1e-5,
        }
    }

    /// Create config optimized for WASI deployment.
    pub fn for_wasi() -> Self {
        Self {
            wasm_target: WasmTarget::Wasi,
            onnx_export: true,
            max_model_size: 200 * 1024 * 1024,
            compute_budget: 0.7,
            dp_enabled: false,
            dp_epsilon: 1.0,
            dp_delta: 1e-5,
        }
    }

    /// Create config for full native deployment.
    pub fn for_native() -> Self {
        Self {
            wasm_target: WasmTarget::Native,
            onnx_export: false,
            max_model_size: usize::MAX,
            compute_budget: 1.0,
            dp_enabled: false,
            dp_epsilon: 1.0,
            dp_delta: 1e-5,
        }
    }

    /// Validate that the config is consistent.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.compute_budget <= 0.0 || self.compute_budget > 1.0 {
            return Err("compute_budget must be in (0, 1]".to_string());
        }
        if self.dp_epsilon <= 0.0 {
            return Err("dp_epsilon must be positive".to_string());
        }
        if self.dp_delta < 0.0 || self.dp_delta >= 1.0 {
            return Err("dp_delta must be in [0, 1)".to_string());
        }
        if self.max_model_size == 0 {
            return Err("max_model_size must be positive".to_string());
        }
        Ok(())
    }

    /// Check if a model with given parameter count fits the budget.
    pub fn model_fits_budget(&self, param_count: usize) -> bool {
        param_count * 4 <= self.max_model_size // f32 = 4 bytes per param
    }
}

/// Result of an edge deployment validation check.
#[derive(Debug, Clone)]
pub struct EdgeDeployResult {
    /// Deployment target
    pub target: WasmTarget,
    /// Model fits within memory budget
    pub fits_memory: bool,
    /// Model fits within compute budget
    pub fits_compute: bool,
    /// Estimated energy per inference (mWh)
    pub estimated_energy_mwh: f64,
    /// ONNX export available
    pub onnx_available: bool,
    /// DP noise level (sigma)
    pub dp_sigma: f64,
    /// Overall deployment readiness
    pub ready: bool,
}

impl EdgeDeployResult {
    /// Create a new deployment result.
    pub fn new(
        target: WasmTarget,
        fits_memory: bool,
        fits_compute: bool,
        estimated_energy_mwh: f64,
        onnx_available: bool,
        dp_sigma: f64,
    ) -> Self {
        let ready = fits_memory && fits_compute;
        Self {
            target,
            fits_memory,
            fits_compute,
            estimated_energy_mwh,
            onnx_available,
            dp_sigma,
            ready,
        }
    }

    /// Generate a human-readable deployment summary.
    pub fn summary(&self) -> String {
        format!(
            "EdgeDeploy[{:?}] memory={} compute={} energy={:.6}mWh onnx={} dp_sigma={:.6} ready={}",
            self.target,
            if self.fits_memory { "OK" } else { "FAIL" },
            if self.fits_compute { "OK" } else { "FAIL" },
            self.estimated_energy_mwh,
            if self.onnx_available { "YES" } else { "NO" },
            self.dp_sigma,
            self.ready,
        )
    }
}

/// Validate edge deployment readiness for a given configuration.
///
/// Checks memory budget, compute budget, energy impact, and ONNX compatibility.
///
/// # Arguments
/// * `config` — Edge deployment configuration
/// * `param_count` — Model parameter count
/// * `compute_steps` — Estimated compute steps per inference
///
/// # Returns
/// `EdgeDeployResult` with full validation details
pub fn validate_edge_deploy(
    config: &EdgeDeployConfig,
    param_count: usize,
    compute_steps: usize,
) -> EdgeDeployResult {
    let fits_memory = config.model_fits_budget(param_count);
    let fits_compute = compute_steps > 0 && config.compute_budget > 0.0;

    // Estimate energy: base cost per step * steps * target factor
    let base_energy_per_step = 0.001; // 1μWh per compute step (edge-optimized)
    let target_factor = match config.wasm_target {
        WasmTarget::Browser => 1.5, // Browser overhead
        WasmTarget::Wasi => 1.0,
        WasmTarget::Native => 0.8, // Native is most efficient
    };
    let estimated_energy_mwh = base_energy_per_step * compute_steps as f64 * target_factor;

    // DP sigma from Gaussian mechanism: σ = sensitivity * sqrt(2*ln(1.25/δ)) / ε
    let dp_sigma = if config.dp_enabled {
        let sensitivity = 1.0 / param_count as f64;
        let ln_factor = (1.25 / config.dp_delta).ln();
        sensitivity * (2.0 * ln_factor).sqrt() / config.dp_epsilon
    } else {
        0.0
    };

    EdgeDeployResult::new(
        config.wasm_target,
        fits_memory,
        fits_compute,
        estimated_energy_mwh,
        config.onnx_export,
        dp_sigma,
    )
}

// ─── Sprint 125: Real Hardware Energy Validation + Production Monitoring ──

/// Production-grade energy validation that cross-checks estimated vs real hardware energy.
///
/// Validates that the estimated energy from `validate_edge_deploy` falls within
/// acceptable bounds of the real hardware energy measurement.
///
/// # Arguments
/// * `estimated_mwh` — Estimated energy in milliwatt-hours from simulation
/// * `measured_mwh` — Actual measured energy from hardware sensors
/// * `tolerance` — Maximum allowed deviation ratio (default 0.3 = ±30%)
///
/// # Returns
/// `true` if the estimate is within tolerance of the measurement
pub fn validate_real_energy(estimated_mwh: f64, measured_mwh: f64, tolerance: f64) -> bool {
    if measured_mwh <= 0.0 {
        return estimated_mwh <= 0.0;
    }
    let deviation = (estimated_mwh - measured_mwh).abs() / measured_mwh;
    deviation <= tolerance
}

/// Production monitoring metrics for a running edge node.
#[derive(Debug, Clone)]
pub struct ProductionMetrics {
    /// Node uptime in seconds
    pub uptime_seconds: u64,
    /// Total inferences processed
    pub total_inferences: u64,
    /// Average inference latency in milliseconds
    pub avg_latency_ms: f64,
    /// Current energy consumption rate (mWh/s)
    pub energy_rate_mwh_per_s: f64,
    /// Current trust score
    pub trust_score: f64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Memory utilization (0.0 to 1.0)
    pub memory_utilization: f64,
    /// CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f64,
}

impl ProductionMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new inference completion with its latency and energy cost.
    pub fn record_inference(&mut self, latency_ms: f64, energy_mwh: f64, success: bool) {
        self.total_inferences += 1;
        // Exponential moving average for latency
        let alpha = 0.1;
        self.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_latency_ms;
        // Track p99 as running max with decay
        if latency_ms > self.p99_latency_ms * 0.95 {
            self.p99_latency_ms = latency_ms;
        }
        // Update error rate with EMA
        let error = if success { 0.0 } else { 1.0 };
        self.error_rate = alpha * error + (1.0 - alpha) * self.error_rate;
        // Update energy rate
        self.energy_rate_mwh_per_s = energy_mwh / (latency_ms / 1000.0).max(0.001);
    }

    /// Update resource utilization metrics.
    pub fn update_utilization(&mut self, memory_util: f64, cpu_util: f64) {
        self.memory_utilization = memory_util.clamp(0.0, 1.0);
        self.cpu_utilization = cpu_util.clamp(0.0, 1.0);
    }

    /// Check if the node is healthy for production operation.
    pub fn is_healthy(&self) -> bool {
        self.error_rate < 0.05
            && self.memory_utilization < 0.9
            && self.cpu_utilization < 0.95
            && self.trust_score >= 0.3
            && self.avg_latency_ms > 0.0
    }

    /// Compute a health score from 0.0 (critical) to 1.0 (excellent).
    pub fn health_score(&self) -> f64 {
        let error_penalty = self.error_rate * 2.0;
        let memory_penalty = if self.memory_utilization > 0.8 {
            (self.memory_utilization - 0.8) * 5.0
        } else {
            0.0
        };
        let cpu_penalty = if self.cpu_utilization > 0.85 {
            (self.cpu_utilization - 0.85) * 6.67
        } else {
            0.0
        };
        let latency_penalty = if self.avg_latency_ms > 1000.0 {
            (self.avg_latency_ms - 1000.0) / 10000.0
        } else {
            0.0
        };
        let trust_bonus = self.trust_score * 0.1;
        (1.0 - error_penalty - memory_penalty - cpu_penalty - latency_penalty + trust_bonus)
            .clamp(0.0, 1.0)
    }

    /// Generate a human-readable status report.
    pub fn status_report(&self) -> String {
        format!(
            "ProdMetrics[uptime={}s inferences={} avg_lat={:.2}ms p99={:.2}ms \
             err={:.4} mem={:.2} cpu={:.2} trust={:.3} health={:.3}]",
            self.uptime_seconds,
            self.total_inferences,
            self.avg_latency_ms,
            self.p99_latency_ms,
            self.error_rate,
            self.memory_utilization * 100.0,
            self.cpu_utilization * 100.0,
            self.trust_score,
            self.health_score(),
        )
    }
}

impl Default for ProductionMetrics {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            total_inferences: 0,
            avg_latency_ms: 0.0,
            energy_rate_mwh_per_s: 0.0,
            trust_score: 0.5,
            p99_latency_ms: 0.0,
            error_rate: 0.0,
            memory_utilization: 0.0,
            cpu_utilization: 0.0,
        }
    }
}

/// Validate that an edge deployment result meets production readiness criteria.
///
/// # Arguments
/// * `deploy` — Result from `validate_edge_deploy`
/// * `metrics` — Current production metrics
/// * `min_health_score` — Minimum acceptable health score
///
/// # Returns
/// `true` if the deployment is production-ready
pub fn is_production_ready(deploy: &EdgeDeployResult, metrics: &ProductionMetrics, min_health_score: f64) -> bool {
    deploy.ready && metrics.is_healthy() && metrics.health_score() >= min_health_score
}

// ─── Sprint 126: Ecosystem Integrations (HF + ONNX + Tauri) ──

/// Device configuration for ecosystem integration.
///
/// Describes the target compute device for model loading, ONNX export,
/// and mobile build targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceConfig {
    /// Compute device type
    pub device_type: DeviceType,
    /// Available memory in MB
    pub memory_mb: usize,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// Whether GPU acceleration is available
    pub has_gpu: bool,
}

impl DeviceConfig {
    /// Create a new device configuration.
    pub fn new(device_type: DeviceType, memory_mb: usize, cpu_cores: usize, has_gpu: bool) -> Self {
        Self {
            device_type,
            memory_mb,
            cpu_cores,
            has_gpu,
        }
    }

    /// Check if the device can run full-precision models.
    pub fn supports_full_precision(&self) -> bool {
        self.memory_mb >= 4096 && self.cpu_cores >= 4
    }

    /// Check if the device can run quantized models.
    pub fn supports_quantized(&self) -> bool {
        self.memory_mb >= 512 && self.cpu_cores >= 2
    }

    /// Check if the device can run ultra-light models only.
    pub fn supports_ultra_light(&self) -> bool {
        self.memory_mb >= 128
    }

    /// Recommended model precision for this device.
    pub fn recommended_precision(&self) -> ModelPrecision {
        if self.supports_full_precision() {
            ModelPrecision::Full
        } else if self.supports_quantized() {
            ModelPrecision::Quantized
        } else if self.supports_ultra_light() {
            ModelPrecision::UltraLight
        } else {
            ModelPrecision::None
        }
    }

    /// Preset for desktop deployment.
    pub fn desktop() -> Self {
        Self::new(DeviceType::Desktop, 16384, 8, true)
    }

    /// Preset for mobile deployment.
    pub fn mobile() -> Self {
        Self::new(DeviceType::Mobile, 4096, 4, false)
    }

    /// Preset for IoT deployment.
    pub fn iot() -> Self {
        Self::new(DeviceType::Iot, 256, 1, false)
    }

    /// Preset for datacenter deployment.
    pub fn datacenter() -> Self {
        Self::new(DeviceType::Datacenter, 65536, 32, true)
    }
}

/// Model precision level for ecosystem integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelPrecision {
    /// Full precision (FP32)
    #[default]
    Full,
    /// Quantized (INT8)
    Quantized,
    /// Ultra-light quantized (INT4)
    UltraLight,
    /// No model can run on this device
    None,
}

impl std::fmt::Display for ModelPrecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelPrecision::Full => write!(f, "FP32"),
            ModelPrecision::Quantized => write!(f, "INT8"),
            ModelPrecision::UltraLight => write!(f, "INT4"),
            ModelPrecision::None => write!(f, "N/A"),
        }
    }
}

/// Result of Hugging Face model integration.
#[derive(Debug, Clone)]
pub struct IntegrationResult {
    /// Model ID from Hugging Face Hub
    pub model_id: String,
    /// Model precision used
    pub precision: ModelPrecision,
    /// Model size in MB after conversion
    pub size_mb: f64,
    /// ONNX export path (empty if not exported)
    pub onnx_path: String,
    /// Whether the model was successfully integrated
    pub success: bool,
    /// Error message if integration failed
    pub error: String,
    /// Estimated inference latency in milliseconds
    pub estimated_latency_ms: f64,
    /// Estimated energy per inference in mWh
    pub estimated_energy_mwh: f64,
}

impl IntegrationResult {
    /// Create a successful integration result.
    pub fn success(
        model_id: String,
        precision: ModelPrecision,
        size_mb: f64,
        onnx_path: String,
        latency_ms: f64,
        energy_mwh: f64,
    ) -> Self {
        Self {
            model_id,
            precision,
            size_mb,
            onnx_path,
            success: true,
            error: String::new(),
            estimated_latency_ms: latency_ms,
            estimated_energy_mwh: energy_mwh,
        }
    }

    /// Create a failed integration result.
    pub fn failure(model_id: String, error: String) -> Self {
        Self {
            model_id,
            precision: ModelPrecision::None,
            size_mb: 0.0,
            onnx_path: String::new(),
            success: false,
            error,
            estimated_latency_ms: 0.0,
            estimated_energy_mwh: 0.0,
        }
    }

    /// Get a summary report of the integration.
    pub fn summary(&self) -> String {
        if self.success {
            format!(
                "Model '{}' integrated successfully: {} precision, {:.1}MB, latency {:.0}ms, energy {:.4}mWh",
                self.model_id, self.precision, self.size_mb, self.estimated_latency_ms, self.estimated_energy_mwh
            )
        } else {
            format!(
                "Model '{}' integration failed: {}",
                self.model_id, self.error
            )
        }
    }
}

/// Result of Tauri mobile build.
#[derive(Debug, Clone)]
pub struct MobileBuildResult {
    /// Target platform
    pub platform: String,
    /// Build size in MB
    pub build_size_mb: f64,
    /// Whether WASM optimization was applied
    pub wasm_optimized: bool,
    /// Whether the build succeeded
    pub success: bool,
    /// Error message if build failed
    pub error: String,
    /// Estimated startup time in milliseconds
    pub estimated_startup_ms: f64,
    /// Number of models bundled
    pub models_bundled: usize,
    /// Supported features list
    pub features: Vec<String>,
}

impl MobileBuildResult {
    /// Create a successful build result.
    pub fn success(
        platform: String,
        build_size_mb: f64,
        wasm_optimized: bool,
        startup_ms: f64,
        models_bundled: usize,
        features: Vec<String>,
    ) -> Self {
        Self {
            platform,
            build_size_mb,
            wasm_optimized,
            success: true,
            error: String::new(),
            estimated_startup_ms: startup_ms,
            models_bundled,
            features,
        }
    }

    /// Create a failed build result.
    pub fn failure(platform: String, error: String) -> Self {
        Self {
            platform,
            build_size_mb: 0.0,
            wasm_optimized: false,
            success: false,
            error,
            estimated_startup_ms: 0.0,
            models_bundled: 0,
            features: Vec::new(),
        }
    }

    /// Get a summary report of the build.
    pub fn summary(&self) -> String {
        if self.success {
            format!(
                "Mobile build for '{}': {:.1}MB, WASM={}, startup {:.0}ms, {} models, features: {}",
                self.platform,
                self.build_size_mb,
                self.wasm_optimized,
                self.estimated_startup_ms,
                self.models_bundled,
                self.features.join(", ")
            )
        } else {
            format!(
                "Mobile build for '{}' failed: {}",
                self.platform, self.error
            )
        }
    }
}

/// Integrate a model from Hugging Face Hub with ONNX conversion.
///
/// Simulates the full pipeline: model download → ONNX conversion → quantization
/// → edge deployment validation. In production this would call `hf-hub` and
/// `ort` (ONNX Runtime) crates.
///
/// # Arguments
/// * `model_id` — Hugging Face model identifier (e.g., "org/model-name")
/// * `device` — Target device configuration
///
/// # Returns
/// `IntegrationResult` with success/failure status and metrics
pub fn integrate_hf_model(model_id: &str, device: &DeviceConfig) -> IntegrationResult {
    // Validate model ID format
    if model_id.is_empty() || !model_id.contains('/') {
        return IntegrationResult::failure(
            model_id.to_string(),
            "Invalid model ID: must be in format 'org/model-name'".to_string(),
        );
    }

    // Determine precision based on device capability
    let precision = device.recommended_precision();
    if precision == ModelPrecision::None {
        return IntegrationResult::failure(
            model_id.to_string(),
            "Device insufficient: minimum 128MB RAM required".to_string(),
        );
    }

    // Estimate model size based on precision and device
    let base_size_mb = 256.0; // Base model size
    let size_mb = match precision {
        ModelPrecision::Full => base_size_mb,
        ModelPrecision::Quantized => base_size_mb * 0.25,
        ModelPrecision::UltraLight => base_size_mb * 0.125,
        ModelPrecision::None => 0.0,
    };

    // Validate model fits in device memory
    if size_mb > device.memory_mb as f64 {
        return IntegrationResult::failure(
            model_id.to_string(),
            format!(
                "Model size {:.1}MB exceeds device memory {}MB",
                size_mb, device.memory_mb
            ),
        );
    }

    // Estimate latency based on device type and precision
    let base_latency = match device.device_type {
        DeviceType::Datacenter => 5.0,
        DeviceType::Desktop => 15.0,
        DeviceType::OldDesktop => 30.0,
        DeviceType::Mobile => 50.0,
        DeviceType::Iot => 200.0,
        DeviceType::Smartwatch => 500.0,
    };
    let latency_factor = match precision {
        ModelPrecision::Full => 1.0,
        ModelPrecision::Quantized => 0.6,
        ModelPrecision::UltraLight => 0.4,
        ModelPrecision::None => 1.0,
    };
    let gpu_factor = if device.has_gpu { 0.3 } else { 1.0 };
    let estimated_latency_ms = base_latency * latency_factor * gpu_factor;

    // Estimate energy based on latency and device
    let estimated_energy_mwh = device.device_type.base_energy_cost() * (estimated_latency_ms / 1000.0);

    // Generate ONNX path
    let onnx_path = format!("models/{}/{}.onnx", model_id, precision);

    IntegrationResult::success(
        model_id.to_string(),
        precision,
        size_mb,
        onnx_path,
        estimated_latency_ms,
        estimated_energy_mwh,
    )
}

/// Build Tauri mobile application with WASM optimization.
///
/// Simulates the full mobile build pipeline: WASM compilation → tree-shaking
/// → model bundling → platform-specific packaging.
///
/// # Returns
/// `MobileBuildResult` with build metrics and feature list
pub fn build_tauri_mobile() -> MobileBuildResult {
    let platform = "tauri-mobile".to_string();

    // Determine available features based on build configuration
    let mut features = Vec::new();
    features.push("sae-inference".to_string());
    features.push("p2p-sync".to_string());
    features.push("telemetry".to_string());

    // Check if WASM optimization is available
    let wasm_optimized = true;
    if wasm_optimized {
        features.push("wasm-opt".to_string());
    }

    // Estimate build size
    let base_size_mb = 4.0;
    let models_bundled = 2;
    let model_overhead_mb = 1.5 * models_bundled as f64;
    let build_size_mb = base_size_mb + model_overhead_mb;

    // Estimate startup time
    let startup_ms = if wasm_optimized { 800.0 } else { 1500.0 };

    MobileBuildResult::success(
        platform,
        build_size_mb,
        wasm_optimized,
        startup_ms,
        models_bundled,
        features,
    )
}

/// Build Tauri mobile application with custom configuration.
///
/// # Arguments
/// * `target_platform` — Target platform ("ios", "android", "wasm")
/// * `include_models` — Whether to bundle models in the build
/// * `wasm_optimize` — Whether to apply WASM optimization
///
/// # Returns
/// `MobileBuildResult` with build metrics
pub fn build_tauri_mobile_custom(
    target_platform: &str,
    include_models: bool,
    wasm_optimize: bool,
) -> MobileBuildResult {
    if target_platform.is_empty() {
        return MobileBuildResult::failure(
            "unknown".to_string(),
            "Target platform cannot be empty".to_string(),
        );
    }

    let mut features = Vec::new();
    features.push("core".to_string());

    if include_models {
        features.push("models-bundled".to_string());
    }
    if wasm_optimize {
        features.push("wasm-opt".to_string());
    }

    let models_bundled = if include_models { 2 } else { 0 };
    let model_overhead_mb = 1.5 * models_bundled as f64;
    let base_size_mb = match target_platform {
        "ios" => 3.5,
        "android" => 4.0,
        "wasm" => 2.5,
        _ => 4.5,
    };
    let build_size_mb = base_size_mb + model_overhead_mb;

    let startup_ms = match (wasm_optimize, target_platform) {
        (true, "wasm") => 400.0,
        (true, _) => 800.0,
        (false, "wasm") => 900.0,
        (false, _) => 1500.0,
    };

    MobileBuildResult::success(
        target_platform.to_string(),
        build_size_mb,
        wasm_optimize,
        startup_ms,
        models_bundled,
        features,
    )
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Device;

    #[test]
    fn test_power_state_from_metrics() {
        // Ultra-conservative
        let state = PowerState::from_metrics(0.2, 0.9, false);
        assert_eq!(state, PowerState::UltraConservative);

        // Conservative
        let state = PowerState::from_metrics(0.4, 0.9, false);
        assert_eq!(state, PowerState::Conservative);

        // Normal
        let state = PowerState::from_metrics(0.8, 0.9, false);
        assert_eq!(state, PowerState::Normal);

        // Charging overrides
        let state = PowerState::from_metrics(0.1, 0.2, true);
        assert_eq!(state, PowerState::Charging);

        // Low network quality
        let state = PowerState::from_metrics(0.9, 0.3, false);
        assert_eq!(state, PowerState::UltraConservative);
    }

    #[test]
    fn test_power_state_capabilities() {
        assert!(!PowerState::UltraConservative.allows_heavy_compute());
        assert!(!PowerState::UltraConservative.allows_p2p_gossip());
        assert!(PowerState::Conservative.allows_p2p_gossip());
        assert!(!PowerState::Conservative.allows_heavy_compute());
        assert!(PowerState::Normal.allows_heavy_compute());
        assert!(PowerState::Normal.allows_p2p_gossip());
        assert!(PowerState::Charging.allows_heavy_compute());
    }

    #[test]
    fn test_compute_budget() {
        assert_eq!(PowerState::UltraConservative.compute_budget(), 0.1);
        assert_eq!(PowerState::Conservative.compute_budget(), 0.5);
        assert_eq!(PowerState::Normal.compute_budget(), 1.0);
        assert_eq!(PowerState::Charging.compute_budget(), 1.0);
    }

    #[test]
    fn test_edge_runtime_config_default() {
        let config = EdgeRuntimeConfig::default();
        assert_eq!(config.battery_level, 1.0);
        assert_eq!(config.network_quality, 1.0);
        assert!(!config.is_charging);
        assert_eq!(config.power_state(), PowerState::Normal);
    }

    #[test]
    fn test_edge_runtime_config_new() {
        let config = EdgeRuntimeConfig::new(0.2, 0.3, false);
        assert_eq!(config.battery_level, 0.2);
        assert_eq!(config.network_quality, 0.3);
        assert_eq!(config.power_state(), PowerState::UltraConservative);
    }

    #[test]
    fn test_device_type_energy() {
        assert!(DeviceType::Mobile.base_energy_cost() < DeviceType::Desktop.base_energy_cost());
        assert!(DeviceType::Iot.base_energy_cost() < DeviceType::Mobile.base_energy_cost());
        assert!(DeviceType::Desktop.dc_baseline_cost() > DeviceType::Desktop.base_energy_cost());
    }

    #[test]
    fn test_estimate_energy_impact() {
        let impact = estimate_energy_impact(100, DeviceType::Desktop);
        assert!(impact.energy_saved_mwh > 0.0);
        assert!(impact.savings_pct > 80.0); // Edge should save >80% vs DC
        assert_eq!(impact.energy_used_mwh, 100.0 * 0.05);
        assert_eq!(impact.dc_baseline_mwh, 100.0 * 0.8);
    }

    #[test]
    fn test_energy_impact_display() {
        let impact = estimate_energy_impact(50, DeviceType::Mobile);
        let display = format!("{}", impact);
        assert!(display.contains("mWh"));
        assert!(display.contains("%"));
    }

    #[test]
    fn test_planetary_metrics_default() {
        let metrics = PlanetaryImpactMetrics::default();
        assert_eq!(metrics.total_certifications, 0);
        assert_eq!(metrics.global_energy_saved_mwh, 0.0);
        assert_eq!(metrics.co2_saved_kg(), 0.0);
    }

    #[test]
    fn test_planetary_metrics_add_impact() {
        let mut metrics = PlanetaryImpactMetrics::default();
        let impact = estimate_energy_impact(10, DeviceType::Desktop);
        metrics.add_impact(&impact);
        assert_eq!(metrics.total_certifications, 1);
        assert!(metrics.global_energy_saved_mwh > 0.0);
        assert!(metrics.co2_saved_kg() > 0.0);
    }

    #[test]
    fn test_planetary_metrics_summary() {
        let metrics = PlanetaryImpactMetrics::new(1000, 50, 85.0, 0.3);
        let summary = metrics.summary();
        assert!(summary.contains("Planetary Impact"));
        assert!(summary.contains("Energy Saved"));
        assert!(summary.contains("Active Nodes"));
    }

    #[test]
    fn test_certified_steer_record() {
        let record = CertifiedSteerRecord::new(
            0.8,
            0.3,
            0.05,
            0.75,
            ComputePath::FullHybrid,
            PowerState::Normal,
        );
        assert_eq!(record.vfe_reduction(), 0.5);
        assert!(record.timestamp > 0);
    }

    #[test]
    fn test_certified_steer_record_no_reduction() {
        let record = CertifiedSteerRecord::new(
            0.3,
            0.5,
            0.05,
            0.75,
            ComputePath::FastPathOnly,
            PowerState::Conservative,
        );
        assert_eq!(record.vfe_reduction(), 0.0); // Clamped to 0
    }

    #[test]
    fn test_altruist_onboarding() {
        let onboarding = AltruistOnboarding::new(42, DeviceType::Mobile)
            .add_bootstrap_peer("/ip4/192.168.1.1/tcp/4001".to_string());
        assert_eq!(onboarding.node_id, 42);
        assert_eq!(onboarding.bootstrap_peers.len(), 1);
        let cmd = onboarding.install_command();
        assert!(cmd.contains("--altruist"));
        assert!(cmd.contains("--mobile"));
    }

    #[test]
    fn test_install_command_iot() {
        let onboarding = AltruistOnboarding::new(1, DeviceType::Iot);
        let cmd = onboarding.install_command();
        assert!(cmd.contains("--iot"));
    }

    #[test]
    fn test_evaluate_planetary_hybrid_ultra_conservative() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.1f32, 2.1, 3.1], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, slow_path, _steered, energy, saved) = evaluate_planetary_hybrid(
            &hidden,
            &safe,
            &toxic,
            0.2,
            0.3,
            false, // Low battery + poor network
            DeviceType::Mobile,
        )?;

        assert!(safe); // Closer to safe centroid
        assert!(!slow_path); // Ultra-conservative doesn't use slow path
        assert!(energy > 0.0);
        assert!(saved > 0.0);
        Ok(())
    }

    #[test]
    fn test_evaluate_planetary_hybrid_normal() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.1f32, 2.1, 3.1], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, slow_path, _steered, energy, saved) = evaluate_planetary_hybrid(
            &hidden,
            &safe,
            &toxic,
            0.9,
            0.9,
            false, // Good battery + good network
            DeviceType::Desktop,
        )?;

        assert!(safe);
        assert!(!slow_path); // SWD ratio is very safe
        assert!(energy > 0.0);
        assert!(saved > 0.0);
        Ok(())
    }

    #[test]
    fn test_evaluate_planetary_hybrid_boundary() -> Result<()> {
        let device = Device::Cpu;
        // Hidden equidistant from safe and toxic
        let hidden = Tensor::new(vec![5.5f32, 11.0, 16.5], &device)?;
        let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, _slow_path, _steered, _energy, _saved) = evaluate_planetary_hybrid(
            &hidden,
            &safe,
            &toxic,
            0.9,
            0.9,
            false,
            DeviceType::Desktop,
        )?;

        // Should be near boundary (ratio ~0.5)
        // Equidistant → ratio ≈ 0.5 → safe = (ratio > 0.5) → false
        assert!(!safe);
        Ok(())
    }

    // ===== Sprint 121 (v12.1.0) — Noosfera Symbiotic Launch Tests =====

    #[test]
    fn test_device_type_smartwatch() {
        let d = DeviceType::Smartwatch;
        assert_eq!(d.base_energy_cost(), 0.005);
        assert_eq!(d.dc_baseline_cost(), 0.3);
        assert_eq!(d.compute_budget(), 0.1);
        assert_eq!(d.contribution_factor(), 5.0);
    }

    #[test]
    fn test_device_type_datacenter() {
        let d = DeviceType::Datacenter;
        assert_eq!(d.base_energy_cost(), 5.0);
        assert_eq!(d.dc_baseline_cost(), 50.0);
        assert_eq!(d.compute_budget(), 1.0);
        assert_eq!(d.contribution_factor(), 0.5);
    }

    #[test]
    fn test_contribution_factor_rewards_low_capability() {
        // Lower capability devices get higher contribution factor (PoSym bonus)
        assert!(
            DeviceType::Smartwatch.contribution_factor()
                > DeviceType::Desktop.contribution_factor()
        );
        assert!(DeviceType::Iot.contribution_factor() > DeviceType::Desktop.contribution_factor());
        assert!(
            DeviceType::Mobile.contribution_factor() > DeviceType::Desktop.contribution_factor()
        );
        assert!(
            DeviceType::Datacenter.contribution_factor()
                < DeviceType::Desktop.contribution_factor()
        );
        assert_eq!(DeviceType::Desktop.contribution_factor(), 1.0);
    }

    #[test]
    fn test_compute_budget_range() {
        assert_eq!(DeviceType::Smartwatch.compute_budget(), 0.1);
        assert_eq!(DeviceType::Iot.compute_budget(), 0.15);
        assert_eq!(DeviceType::Mobile.compute_budget(), 0.4);
        assert_eq!(DeviceType::OldDesktop.compute_budget(), 0.6);
        assert_eq!(DeviceType::Desktop.compute_budget(), 1.0);
        assert_eq!(DeviceType::Datacenter.compute_budget(), 1.0);
    }

    #[test]
    fn test_evaluate_proportional_hybrid_smartwatch() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, certified, _steered, trust_delta, energy_used) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, DeviceType::Smartwatch, 0.5, 0.5)?;

        // Smartwatch has budget 0.1 < 0.3 → ultra-light path
        assert!(safe); // Hidden == safe centroid → ratio > 0.5
        assert!(!certified); // Ultra-light path doesn't certify
        assert!(trust_delta > 0.0); // Contribution factor bonus
        assert!(energy_used < 0.01); // Very low energy for smartwatch
        Ok(())
    }

    #[test]
    fn test_evaluate_proportional_hybrid_datacenter() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, certified, _steered, trust_delta, energy_used) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, DeviceType::Datacenter, 0.9, 0.9)?;

        // Datacenter has budget 1.0 → full hybrid path
        assert!(safe);
        assert!(certified); // Full hybrid path certifies
        assert!(trust_delta > 0.0);
        assert!(energy_used > 0.0);
        Ok(())
    }

    #[test]
    fn test_evaluate_proportional_hybrid_iot_ultra_light() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, certified, _steered, trust_delta, _energy_used) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, DeviceType::Iot, 0.3, 0.3)?;

        // IoT has budget 0.15 < 0.3 → ultra-light path
        assert!(safe);
        assert!(!certified);
        assert_eq!(trust_delta, DeviceType::Iot.contribution_factor() * 0.01);
        Ok(())
    }

    #[test]
    fn test_evaluate_proportional_hybrid_mobile_full_hybrid() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        let (safe, certified, _steered, _trust_delta, _energy_used) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, DeviceType::Mobile, 0.8, 0.8)?;

        // Mobile has budget 0.4 >= 0.3 → full hybrid path
        assert!(safe);
        assert!(certified);
        Ok(())
    }

    #[test]
    fn test_multimodal_vfe_symbiosis_empty() {
        let vfe = compute_multimodal_vfe_symbiosis(&[], &[0.5, 0.5]);
        assert_eq!(vfe, 0.0);

        let vfe2 = compute_multimodal_vfe_symbiosis(&[0.1, 0.2], &[]);
        assert_eq!(vfe2, 0.0);

        let vfe3 = compute_multimodal_vfe_symbiosis(&[0.1], &[0.5, 0.5]);
        assert_eq!(vfe3, 0.0);
    }

    #[test]
    fn test_multimodal_vfe_symbiosis_single_modality() {
        let vfe = compute_multimodal_vfe_symbiosis(&[0.1], &[1.0]);
        assert!((vfe - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_multimodal_vfe_symbiosis_equal_weights() {
        // Two modalities with equal VFEs → combined VFE ≈ individual VFE
        let vfe = compute_multimodal_vfe_symbiosis(&[0.1, 0.1], &[0.5, 0.5]);
        assert!((vfe - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_multimodal_vfe_symbiosis_weighted_geometric_mean() {
        // Weighted geometric mean: exp(0.5*ln(0.1) + 0.5*ln(0.4)) = sqrt(0.04) = 0.2
        let vfe = compute_multimodal_vfe_symbiosis(&[0.1, 0.4], &[0.5, 0.5]);
        assert!((vfe - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_multimodal_vfe_with_cbf_safe() {
        let (combined, margin, is_safe) = compute_multimodal_vfe_with_cbf(&[0.1], &[1.0], 0.5);
        assert!((combined - 0.1).abs() < 1e-10);
        assert!((margin - 0.4).abs() < 1e-10);
        assert!(is_safe);
    }

    #[test]
    fn test_multimodal_vfe_with_cbf_unsafe() {
        let (combined, margin, is_safe) = compute_multimodal_vfe_with_cbf(&[0.6], &[1.0], 0.5);
        assert!((combined - 0.6).abs() < 1e-10);
        assert!((margin - (-0.1)).abs() < 1e-10);
        assert!(!is_safe);
    }

    #[test]
    fn test_multimodal_vfe_with_cbf_boundary() {
        // Due to eps=1e-12 in geometric mean, combined ≈ 0.5 + tiny
        // so margin is slightly negative → unsafe at exact boundary
        let (combined, margin, is_safe) = compute_multimodal_vfe_with_cbf(&[0.5], &[1.0], 0.5);
        assert!((combined - 0.5).abs() < 1e-8);
        // The eps term makes combined slightly > 0.5, so margin is slightly < 0
        assert!(!is_safe); // eps shifts combined above threshold
        assert!(margin.abs() < 1e-8); // Margin is near-zero
    }

    #[test]
    fn test_install_command_smartwatch() {
        let onboarding = AltruistOnboarding::new(42, DeviceType::Smartwatch)
            .add_bootstrap_peer("192.168.1.1:9000".to_string());
        let cmd = onboarding.install_command();
        assert!(cmd.contains("--smartwatch"));
        assert!(cmd.contains("ed2k start --altruist"));
        assert!(cmd.contains("192.168.1.1:9000"));
    }

    #[test]
    fn test_install_command_datacenter() {
        let onboarding = AltruistOnboarding::new(100, DeviceType::Datacenter)
            .add_bootstrap_peer("10.0.0.1:9000".to_string());
        let cmd = onboarding.install_command();
        assert!(cmd.contains("--datacenter"));
        assert!(cmd.contains("ed2k start --altruist"));
        assert!(cmd.contains("10.0.0.1:9000"));
    }

    #[test]
    fn test_proportional_scaling_energy_savings() -> Result<()> {
        let device = Device::Cpu;
        let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
        let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

        // Smartwatch should use less energy than Datacenter
        let (_, _, _, _, sw_energy) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, DeviceType::Smartwatch, 0.5, 0.5)?;
        let (_, _, _, _, dc_energy) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, DeviceType::Datacenter, 0.9, 0.9)?;

        assert!(sw_energy < dc_energy);
        Ok(())
    }

    // ====================================================================
    // Sprint 123 — Energy-Aware MDP + CBF Tests
    // ====================================================================

    #[test]
    fn test_mdp_energy_reward_basic() {
        let reward = mdp_energy_reward(0.5, 100.0, 0.8, 0.01);
        // R = -0.5 - 0.01*100 + 100*0.8 = -0.5 - 1.0 + 80.0 = 78.5
        assert!((reward - 78.5).abs() < 1e-10);
    }

    #[test]
    fn test_mdp_energy_reward_zero_safety() {
        let reward = mdp_energy_reward(1.0, 50.0, 0.0, 0.02);
        // R = -1.0 - 0.02*50 + 0 = -1.0 - 1.0 = -2.0
        assert!((reward - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_mdp_energy_reward_negative_safety() {
        let reward = mdp_energy_reward(0.1, 10.0, -0.5, 0.01);
        // R = -0.1 - 0.01*10 + 100*(-0.5) = -0.1 - 0.1 - 50.0 = -50.2
        assert!((reward - (-50.2)).abs() < 1e-10);
    }

    #[test]
    fn test_mdp_energy_reward_high_lambda() {
        let reward = mdp_energy_reward(0.0, 1000.0, 1.0, 1.0);
        // R = 0 - 1.0*1000 + 100*1.0 = -900.0
        assert!((reward - (-900.0)).abs() < 1e-10);
    }

    #[test]
    fn test_mdp_energy_reward_zero_all() {
        let reward = mdp_energy_reward(0.0, 0.0, 0.0, 0.0);
        assert!((reward - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_cbf_safe_state() {
        let state = vec![0.0, 0.0, 0.0];
        let safe_center = vec![0.0, 0.0, 0.0];
        assert!(control_barrier_filter(&state, &safe_center, 1.0));
    }

    #[test]
    fn test_cbf_boundary_state() {
        let state = vec![1.0, 0.0, 0.0];
        let safe_center = vec![0.0, 0.0, 0.0];
        // dist_sq = 1.0, beta = 1.0 → h = 0.0 (on boundary)
        assert!(control_barrier_filter(&state, &safe_center, 1.0));
    }

    #[test]
    fn test_cbf_unsafe_state() {
        let state = vec![2.0, 0.0, 0.0];
        let safe_center = vec![0.0, 0.0, 0.0];
        // dist_sq = 4.0, beta = 1.0 → h = -3.0 (unsafe)
        assert!(!control_barrier_filter(&state, &safe_center, 1.0));
    }

    #[test]
    fn test_cbf_dimension_mismatch() {
        let state = vec![1.0, 2.0];
        let safe_center = vec![1.0, 2.0, 3.0];
        assert!(!control_barrier_filter(&state, &safe_center, 1.0));
    }

    #[test]
    fn test_cbf_large_radius() {
        let state = vec![5.0, 5.0, 5.0];
        let safe_center = vec![0.0, 0.0, 0.0];
        // dist_sq = 75.0, beta = 100.0 → h = 25.0 (safe)
        assert!(control_barrier_filter(&state, &safe_center, 100.0));
    }

    #[test]
    fn test_cbf_value_positive() {
        let state = vec![0.5, 0.5];
        let safe_center = vec![0.0, 0.0];
        let h = control_barrier_value(&state, &safe_center, 1.0);
        // dist_sq = 0.5, h = 1.0 - 0.5 = 0.5
        assert!((h - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_cbf_value_negative() {
        let state = vec![3.0, 0.0];
        let safe_center = vec![0.0, 0.0];
        let h = control_barrier_value(&state, &safe_center, 1.0);
        // dist_sq = 9.0, h = 1.0 - 9.0 = -8.0
        assert!((h - (-8.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cbf_value_dimension_mismatch() {
        let state = vec![1.0];
        let safe_center = vec![1.0, 2.0];
        let h = control_barrier_value(&state, &safe_center, 1.0);
        assert!(h.is_infinite() && h < 0.0);
    }

    #[test]
    fn test_mdp_select_action_cbf_all_safe() {
        let actions = vec![
            (0.5, 100.0, vec![0.0, 0.0]), // Safe, reward = -0.5 - 1.0 + 100*1.0 = 98.5
            (0.1, 50.0, vec![0.0, 0.0]),  // Safe, reward = -0.1 - 0.5 + 100*1.0 = 99.4
            (1.0, 200.0, vec![0.0, 0.0]), // Safe, reward = -1.0 - 2.0 + 100*1.0 = 97.0
        ];
        let safe_center = vec![0.0, 0.0];
        let selected = mdp_select_action_cbf(&actions, &safe_center, 1.0, 0.01);
        assert_eq!(selected, Some(1)); // Action 1 has highest reward
    }

    #[test]
    fn test_mdp_select_action_cbf_mixed() {
        let actions = vec![
            (0.1, 50.0, vec![0.0, 0.0]),  // Safe: dist=0, h=1.0
            (0.01, 10.0, vec![5.0, 0.0]), // Unsafe: dist=25, h=-24.0
        ];
        let safe_center = vec![0.0, 0.0];
        let selected = mdp_select_action_cbf(&actions, &safe_center, 1.0, 0.01);
        assert_eq!(selected, Some(0)); // Only action 0 is safe
    }

    #[test]
    fn test_mdp_select_action_cbf_all_unsafe() {
        let actions = vec![
            (0.1, 50.0, vec![5.0, 0.0]),  // Unsafe: dist=25, h=-24.0
            (0.2, 100.0, vec![3.0, 0.0]), // Unsafe: dist=9, h=-8.0
        ];
        let safe_center = vec![0.0, 0.0];
        let selected = mdp_select_action_cbf(&actions, &safe_center, 1.0, 0.01);
        // All unsafe → select least-violating (highest h = -8.0)
        assert_eq!(selected, Some(1));
    }

    #[test]
    fn test_mdp_select_action_cbf_empty() {
        let actions: Vec<(f64, f64, Vec<f32>)> = vec![];
        let safe_center = vec![0.0, 0.0];
        let selected = mdp_select_action_cbf(&actions, &safe_center, 1.0, 0.01);
        assert_eq!(selected, None);
    }

    #[test]
    fn test_mpc_config_default() {
        let cfg = MpcConfig::default();
        assert_eq!(cfg.horizon, 10);
        assert!((cfg.beta - 1.0).abs() < 1e-6);
        assert!((cfg.lambda - 0.01).abs() < 1e-10);
        assert!((cfg.discount - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_mpc_config_with_horizon() {
        let cfg = MpcConfig::with_horizon(20);
        assert_eq!(cfg.horizon, 20);
    }

    #[test]
    fn test_mpc_config_with_safety_radius() {
        let cfg = MpcConfig::with_safety_radius(5.0);
        assert!((cfg.beta - 5.0).abs() < 1e-6);
    }

    // ─── Sprint 125: Production Monitoring Tests ─────────────────────────

    #[test]
    fn test_validate_real_energy_exact_match() {
        assert!(validate_real_energy(1.0, 1.0, 0.3));
    }

    #[test]
    fn test_validate_real_energy_within_tolerance() {
        assert!(validate_real_energy(1.2, 1.0, 0.3)); // 20% deviation < 30%
    }

    #[test]
    fn test_validate_real_energy_exceeds_tolerance() {
        assert!(!validate_real_energy(1.5, 1.0, 0.3)); // 50% deviation > 30%
    }

    #[test]
    fn test_validate_real_energy_zero_measured() {
        assert!(validate_real_energy(0.0, 0.0, 0.3));
        assert!(!validate_real_energy(1.0, 0.0, 0.3));
    }

    #[test]
    fn test_validate_real_energy_strict_tolerance() {
        assert!(validate_real_energy(1.01, 1.0, 0.05)); // 1% < 5%
        assert!(!validate_real_energy(1.06, 1.0, 0.05)); // 6% > 5%
    }

    #[test]
    fn test_production_metrics_default() {
        let m = ProductionMetrics::default();
        assert_eq!(m.uptime_seconds, 0);
        assert_eq!(m.total_inferences, 0);
        assert!((m.avg_latency_ms - 0.0).abs() < 1e-9);
        assert!((m.trust_score - 0.5).abs() < 1e-9);
        assert!(!m.is_healthy()); // avg_latency_ms == 0
    }

    #[test]
    fn test_production_metrics_new() {
        let m = ProductionMetrics::new();
        assert_eq!(m.total_inferences, 0);
    }

    #[test]
    fn test_production_metrics_record_inference() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 100.0; // Seed initial latency
        m.record_inference(50.0, 0.05, true);
        assert_eq!(m.total_inferences, 1);
        // EMA: 0.1 * 50 + 0.9 * 100 = 5 + 90 = 95
        assert!((m.avg_latency_ms - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_production_metrics_record_error() {
        let mut m = ProductionMetrics::default();
        m.record_inference(100.0, 0.1, false);
        assert!((m.error_rate - 0.1).abs() < 0.01); // alpha * 1.0
    }

    #[test]
    fn test_production_metrics_healthy() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 50.0;
        m.memory_utilization = 0.5;
        m.cpu_utilization = 0.4;
        m.trust_score = 0.8;
        m.error_rate = 0.01;
        assert!(m.is_healthy());
    }

    #[test]
    fn test_production_metrics_unhealthy_high_memory() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 50.0;
        m.memory_utilization = 0.95; // > 0.9
        m.cpu_utilization = 0.4;
        m.trust_score = 0.8;
        assert!(!m.is_healthy());
    }

    #[test]
    fn test_production_metrics_unhealthy_high_error() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 50.0;
        m.memory_utilization = 0.5;
        m.cpu_utilization = 0.4;
        m.trust_score = 0.8;
        m.error_rate = 0.1; // > 0.05
        assert!(!m.is_healthy());
    }

    #[test]
    fn test_production_metrics_unhealthy_low_trust() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 50.0;
        m.memory_utilization = 0.5;
        m.cpu_utilization = 0.4;
        m.trust_score = 0.1; // < 0.3
        assert!(!m.is_healthy());
    }

    #[test]
    fn test_health_score_perfect() {
        let m = ProductionMetrics {
            uptime_seconds: 3600,
            total_inferences: 10000,
            avg_latency_ms: 100.0,
            energy_rate_mwh_per_s: 0.05,
            trust_score: 1.0,
            p99_latency_ms: 200.0,
            error_rate: 0.0,
            memory_utilization: 0.3,
            cpu_utilization: 0.2,
        };
        let score = m.health_score();
        assert!(score > 0.9);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_health_score_critical() {
        let m = ProductionMetrics {
            uptime_seconds: 10,
            total_inferences: 5,
            avg_latency_ms: 5000.0,
            energy_rate_mwh_per_s: 10.0,
            trust_score: 0.0,
            p99_latency_ms: 8000.0,
            error_rate: 0.5,
            memory_utilization: 0.98,
            cpu_utilization: 0.99,
        };
        let score = m.health_score();
        assert!(score < 0.3);
    }

    #[test]
    fn test_health_score_bounded() {
        let m = ProductionMetrics::default();
        let score = m.health_score();
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_status_report_contains_fields() {
        let m = ProductionMetrics {
            uptime_seconds: 3600,
            total_inferences: 5000,
            avg_latency_ms: 75.5,
            energy_rate_mwh_per_s: 0.08,
            trust_score: 0.85,
            p99_latency_ms: 150.0,
            error_rate: 0.02,
            memory_utilization: 0.6,
            cpu_utilization: 0.45,
        };
        let report = m.status_report();
        assert!(report.contains("3600"));
        assert!(report.contains("5000"));
        assert!(report.contains("75.5"));
    }

    #[test]
    fn test_update_utilization_clamps() {
        let mut m = ProductionMetrics::default();
        m.update_utilization(1.5, -0.3);
        assert!((m.memory_utilization - 1.0).abs() < 1e-9);
        assert!((m.cpu_utilization - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_is_production_ready_all_pass() {
        let deploy = EdgeDeployResult::new(
            WasmTarget::Native, true, true, 0.5, true, 0.0,
        );
        let metrics = ProductionMetrics {
            uptime_seconds: 3600,
            total_inferences: 10000,
            avg_latency_ms: 50.0,
            energy_rate_mwh_per_s: 0.05,
            trust_score: 0.9,
            p99_latency_ms: 100.0,
            error_rate: 0.01,
            memory_utilization: 0.4,
            cpu_utilization: 0.3,
        };
        assert!(is_production_ready(&deploy, &metrics, 0.7));
    }

    #[test]
    fn test_is_production_ready_deploy_not_ready() {
        let deploy = EdgeDeployResult::new(
            WasmTarget::Native, false, true, 0.5, true, 0.0,
        );
        let metrics = ProductionMetrics::default();
        assert!(!is_production_ready(&deploy, &metrics, 0.7));
    }

    #[test]
    fn test_is_production_ready_health_too_low() {
        let deploy = EdgeDeployResult::new(
            WasmTarget::Native, true, true, 0.5, true, 0.0,
        );
        let metrics = ProductionMetrics {
            uptime_seconds: 100,
            total_inferences: 10,
            avg_latency_ms: 3000.0,
            energy_rate_mwh_per_s: 5.0,
            trust_score: 0.2,
            p99_latency_ms: 5000.0,
            error_rate: 0.3,
            memory_utilization: 0.95,
            cpu_utilization: 0.98,
        };
        assert!(!is_production_ready(&deploy, &metrics, 0.7));
    }

    #[test]
    fn test_production_metrics_p99_tracking() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 100.0;
        for _ in 0..10 {
            m.record_inference(100.0, 0.05, true);
        }
        m.record_inference(500.0, 0.2, true); // Spike
        assert!(m.p99_latency_ms >= 400.0);
    }

    #[test]
    fn test_production_metrics_energy_rate() {
        let mut m = ProductionMetrics::default();
        m.record_inference(200.0, 0.1, true); // 200ms, 0.1mWh
        // energy_rate = 0.1 / (200/1000) = 0.1 / 0.2 = 0.5 mWh/s
        assert!((m.energy_rate_mwh_per_s - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_validate_real_energy_symmetric() {
        // Deviation is absolute, so direction shouldn't matter
        let a = validate_real_energy(1.3, 1.0, 0.3);
        let b = validate_real_energy(0.7, 1.0, 0.3);
        assert_eq!(a, b); // Both 30% deviation
    }

    #[test]
    fn test_production_metrics_ema_convergence() {
        let mut m = ProductionMetrics::default();
        m.avg_latency_ms = 0.0;
        // Record same latency many times → should converge
        for _ in 0..50 {
            m.record_inference(100.0, 0.05, true);
        }
        assert!((m.avg_latency_ms - 100.0).abs() < 1.0);
    }

    // ─── Sprint 126: Ecosystem Integration Tests ──

    #[test]
    fn test_device_config_new() {
        let config = DeviceConfig::new(DeviceType::Desktop, 16384, 8, true);
        assert_eq!(config.device_type, DeviceType::Desktop);
        assert_eq!(config.memory_mb, 16384);
        assert_eq!(config.cpu_cores, 8);
        assert!(config.has_gpu);
    }

    #[test]
    fn test_device_config_default() {
        let config = DeviceConfig::default();
        assert_eq!(config.device_type, DeviceType::Desktop);
        assert_eq!(config.memory_mb, 0);
        assert_eq!(config.cpu_cores, 0);
        assert!(!config.has_gpu);
    }

    #[test]
    fn test_device_config_desktop() {
        let config = DeviceConfig::desktop();
        assert_eq!(config.device_type, DeviceType::Desktop);
        assert!(config.supports_full_precision());
        assert_eq!(config.recommended_precision(), ModelPrecision::Full);
    }

    #[test]
    fn test_device_config_mobile() {
        let config = DeviceConfig::mobile();
        assert_eq!(config.device_type, DeviceType::Mobile);
        assert!(config.supports_quantized());
        // Mobile preset has 4096MB + 4 cores → supports full precision
        assert!(config.supports_full_precision());
    }

    #[test]
    fn test_device_config_iot() {
        let config = DeviceConfig::iot();
        assert_eq!(config.device_type, DeviceType::Iot);
        assert!(!config.supports_quantized());
        assert!(config.supports_ultra_light());
    }

    #[test]
    fn test_device_config_datacenter() {
        let config = DeviceConfig::datacenter();
        assert_eq!(config.device_type, DeviceType::Datacenter);
        assert!(config.supports_full_precision());
        assert!(config.has_gpu);
    }

    #[test]
    fn test_device_config_supports_full_precision() {
        let config = DeviceConfig::new(DeviceType::Desktop, 8192, 8, false);
        assert!(config.supports_full_precision());
    }

    #[test]
    fn test_device_config_no_full_precision_low_memory() {
        let config = DeviceConfig::new(DeviceType::Mobile, 2048, 8, false);
        assert!(!config.supports_full_precision());
    }

    #[test]
    fn test_device_config_no_full_precision_low_cores() {
        let config = DeviceConfig::new(DeviceType::Desktop, 8192, 2, false);
        assert!(!config.supports_full_precision());
    }

    #[test]
    fn test_device_config_supports_quantized() {
        let config = DeviceConfig::new(DeviceType::Mobile, 1024, 4, false);
        assert!(config.supports_quantized());
    }

    #[test]
    fn test_device_config_no_quantized_low_memory() {
        let config = DeviceConfig::new(DeviceType::Iot, 256, 4, false);
        assert!(!config.supports_quantized());
    }

    #[test]
    fn test_device_config_no_quantized_low_cores() {
        let config = DeviceConfig::new(DeviceType::Iot, 1024, 1, false);
        assert!(!config.supports_quantized());
    }

    #[test]
    fn test_device_config_supports_ultra_light() {
        let config = DeviceConfig::new(DeviceType::Iot, 256, 1, false);
        assert!(config.supports_ultra_light());
    }

    #[test]
    fn test_device_config_no_ultra_light_insufficient_memory() {
        let config = DeviceConfig::new(DeviceType::Iot, 64, 1, false);
        assert!(!config.supports_ultra_light());
    }

    #[test]
    fn test_recommended_precision_full() {
        let config = DeviceConfig::new(DeviceType::Desktop, 16384, 8, true);
        assert_eq!(config.recommended_precision(), ModelPrecision::Full);
    }

    #[test]
    fn test_recommended_precision_quantized() {
        let config = DeviceConfig::new(DeviceType::Mobile, 2048, 4, false);
        assert_eq!(config.recommended_precision(), ModelPrecision::Quantized);
    }

    #[test]
    fn test_recommended_precision_ultra_light() {
        let config = DeviceConfig::new(DeviceType::Iot, 256, 1, false);
        assert_eq!(config.recommended_precision(), ModelPrecision::UltraLight);
    }

    #[test]
    fn test_recommended_precision_none() {
        let config = DeviceConfig::new(DeviceType::Iot, 64, 1, false);
        assert_eq!(config.recommended_precision(), ModelPrecision::None);
    }

    #[test]
    fn test_model_precision_display() {
        assert_eq!(format!("{}", ModelPrecision::Full), "FP32");
        assert_eq!(format!("{}", ModelPrecision::Quantized), "INT8");
        assert_eq!(format!("{}", ModelPrecision::UltraLight), "INT4");
        assert_eq!(format!("{}", ModelPrecision::None), "N/A");
    }

    #[test]
    fn test_integration_result_success() {
        let result = IntegrationResult::success(
            "org/model".to_string(),
            ModelPrecision::Full,
            256.0,
            "models/model.onnx".to_string(),
            15.0,
            0.00075,
        );
        assert!(result.success);
        assert_eq!(result.model_id, "org/model");
        assert_eq!(result.precision, ModelPrecision::Full);
        assert_eq!(result.size_mb, 256.0);
        assert_eq!(result.onnx_path, "models/model.onnx");
        assert!(result.error.is_empty());
    }

    #[test]
    fn test_integration_result_failure() {
        let result = IntegrationResult::failure(
            "bad-model".to_string(),
            "Invalid format".to_string(),
        );
        assert!(!result.success);
        assert_eq!(result.model_id, "bad-model");
        assert_eq!(result.precision, ModelPrecision::None);
        assert_eq!(result.size_mb, 0.0);
        assert!(result.onnx_path.is_empty());
        assert!(!result.error.is_empty());
    }

    #[test]
    fn test_integration_result_summary_success() {
        let result = IntegrationResult::success(
            "org/model".to_string(),
            ModelPrecision::Quantized,
            64.0,
            "models/model.onnx".to_string(),
            30.0,
            0.0015,
        );
        let summary = result.summary();
        assert!(summary.contains("org/model"));
        assert!(summary.contains("successfully"));
        assert!(summary.contains("INT8"));
    }

    #[test]
    fn test_integration_result_summary_failure() {
        let result = IntegrationResult::failure(
            "bad".to_string(),
            "error msg".to_string(),
        );
        let summary = result.summary();
        assert!(summary.contains("failed"));
        assert!(summary.contains("error msg"));
    }

    #[test]
    fn test_mobile_build_result_success() {
        let result = MobileBuildResult::success(
            "ios".to_string(),
            7.0,
            true,
            800.0,
            2,
            vec!["core".to_string(), "models".to_string()],
        );
        assert!(result.success);
        assert_eq!(result.platform, "ios");
        assert_eq!(result.build_size_mb, 7.0);
        assert!(result.wasm_optimized);
        assert_eq!(result.models_bundled, 2);
        assert_eq!(result.features.len(), 2);
    }

    #[test]
    fn test_mobile_build_result_failure() {
        let result = MobileBuildResult::failure(
            "android".to_string(),
            "Build failed".to_string(),
        );
        assert!(!result.success);
        assert_eq!(result.platform, "android");
        assert_eq!(result.build_size_mb, 0.0);
        assert!(!result.wasm_optimized);
        assert_eq!(result.models_bundled, 0);
        assert!(result.features.is_empty());
    }

    #[test]
    fn test_mobile_build_summary_success() {
        let result = MobileBuildResult::success(
            "wasm".to_string(),
            4.0,
            true,
            400.0,
            1,
            vec!["core".to_string()],
        );
        let summary = result.summary();
        assert!(summary.contains("wasm"));
        assert!(summary.contains("4.0MB"));
        assert!(summary.contains("WASM=true"));
    }

    #[test]
    fn test_mobile_build_summary_failure() {
        let result = MobileBuildResult::failure(
            "unknown".to_string(),
            "no sdk".to_string(),
        );
        let summary = result.summary();
        assert!(summary.contains("failed"));
        assert!(summary.contains("no sdk"));
    }

    #[test]
    fn test_integrate_hf_model_invalid_id_empty() {
        let config = DeviceConfig::desktop();
        let result = integrate_hf_model("", &config);
        assert!(!result.success);
        assert!(result.error.contains("Invalid model ID"));
    }

    #[test]
    fn test_integrate_hf_model_invalid_id_no_slash() {
        let config = DeviceConfig::desktop();
        let result = integrate_hf_model("model-only", &config);
        assert!(!result.success);
        assert!(result.error.contains("Invalid model ID"));
    }

    #[test]
    fn test_integrate_hf_model_desktop_full_precision() {
        let config = DeviceConfig::desktop();
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert_eq!(result.precision, ModelPrecision::Full);
        assert_eq!(result.size_mb, 256.0);
        assert!(!result.onnx_path.is_empty());
    }

    #[test]
    fn test_integrate_hf_model_mobile_full_precision() {
        // Mobile preset has 4096MB + 4 cores → qualifies for full precision
        let config = DeviceConfig::mobile();
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert_eq!(result.precision, ModelPrecision::Full);
        assert_eq!(result.size_mb, 256.0);
    }

    #[test]
    fn test_integrate_hf_model_low_end_mobile_quantized() {
        // Low-end mobile with less memory gets quantized
        let config = DeviceConfig::new(DeviceType::Mobile, 2048, 4, false);
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert_eq!(result.precision, ModelPrecision::Quantized);
        assert_eq!(result.size_mb, 64.0);
    }

    #[test]
    fn test_integrate_hf_model_iot_ultra_light() {
        let config = DeviceConfig::iot();
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert_eq!(result.precision, ModelPrecision::UltraLight);
        assert_eq!(result.size_mb, 32.0);
    }

    #[test]
    fn test_integrate_hf_model_insufficient_device() {
        let config = DeviceConfig::new(DeviceType::Iot, 64, 1, false);
        let result = integrate_hf_model("org/model", &config);
        assert!(!result.success);
        assert!(result.error.contains("insufficient"));
    }

    #[test]
    fn test_integrate_hf_model_memory_exceeded() {
        // Device with 100MB < 128MB ultra-light threshold → insufficient
        let config = DeviceConfig::new(DeviceType::Desktop, 100, 8, false);
        let result = integrate_hf_model("org/model", &config);
        assert!(!result.success);
        assert!(result.error.contains("insufficient"));
    }

    #[test]
    fn test_integrate_hf_model_memory_exceeds_after_precision() {
        // Device supports quantized (512MB+ RAM, 2+ cores) but model (64MB) fits.
        // Test with device that barely supports quantized but model still fits.
        let config = DeviceConfig::new(DeviceType::Mobile, 512, 2, false);
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert_eq!(result.precision, ModelPrecision::Quantized);
        assert!(result.size_mb <= config.memory_mb as f64);
    }

    #[test]
    fn test_integrate_hf_model_gpu_latency_reduction() {
        let config_no_gpu = DeviceConfig::new(DeviceType::Desktop, 16384, 8, false);
        let config_gpu = DeviceConfig::new(DeviceType::Desktop, 16384, 8, true);
        let result_no_gpu = integrate_hf_model("org/model", &config_no_gpu);
        let result_gpu = integrate_hf_model("org/model", &config_gpu);
        assert!(result_gpu.estimated_latency_ms < result_no_gpu.estimated_latency_ms);
    }

    #[test]
    fn test_integrate_hf_model_datacenter_lowest_latency() {
        let config = DeviceConfig::datacenter();
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert!(result.estimated_latency_ms < 10.0);
    }

    #[test]
    fn test_integrate_hf_model_energy_estimation() {
        let config = DeviceConfig::desktop();
        let result = integrate_hf_model("org/model", &config);
        assert!(result.success);
        assert!(result.estimated_energy_mwh > 0.0);
        assert!(result.estimated_energy_mwh < 1.0);
    }

    #[test]
    fn test_integrate_hf_model_onnx_path_format() {
        let config = DeviceConfig::desktop();
        let result = integrate_hf_model("org/model", &config);
        assert!(result.onnx_path.contains("org/model"));
        assert!(result.onnx_path.ends_with(".onnx"));
    }

    #[test]
    fn test_build_tauri_mobile_default() {
        let result = build_tauri_mobile();
        assert!(result.success);
        assert_eq!(result.platform, "tauri-mobile");
        assert!(result.wasm_optimized);
        assert!(result.models_bundled > 0);
        assert!(!result.features.is_empty());
    }

    #[test]
    fn test_build_tauri_mobile_features() {
        let result = build_tauri_mobile();
        assert!(result.features.contains(&"sae-inference".to_string()));
        assert!(result.features.contains(&"p2p-sync".to_string()));
        assert!(result.features.contains(&"telemetry".to_string()));
        assert!(result.features.contains(&"wasm-opt".to_string()));
    }

    #[test]
    fn test_build_tauri_mobile_custom_ios() {
        let result = build_tauri_mobile_custom("ios", true, true);
        assert!(result.success);
        assert_eq!(result.platform, "ios");
        assert!(result.wasm_optimized);
        assert_eq!(result.models_bundled, 2);
    }

    #[test]
    fn test_build_tauri_mobile_custom_android() {
        let result = build_tauri_mobile_custom("android", false, false);
        assert!(result.success);
        assert_eq!(result.platform, "android");
        assert!(!result.wasm_optimized);
        assert_eq!(result.models_bundled, 0);
    }

    #[test]
    fn test_build_tauri_mobile_custom_wasm() {
        let result = build_tauri_mobile_custom("wasm", true, true);
        assert!(result.success);
        assert_eq!(result.platform, "wasm");
        assert!(result.wasm_optimized);
        assert!(result.estimated_startup_ms < 500.0);
    }

    #[test]
    fn test_build_tauri_mobile_custom_empty_platform() {
        let result = build_tauri_mobile_custom("", true, true);
        assert!(!result.success);
        assert!(result.error.contains("empty"));
    }

    #[test]
    fn test_build_tauri_mobile_custom_unknown_platform() {
        let result = build_tauri_mobile_custom("unknown", false, false);
        assert!(result.success);
        assert_eq!(result.platform, "unknown");
        // Should still work with default size
        assert!(result.build_size_mb > 0.0);
    }

    #[test]
    fn test_build_tauri_mobile_custom_startup_times() {
        let wasm_opt = build_tauri_mobile_custom("wasm", false, true);
        let wasm_no_opt = build_tauri_mobile_custom("wasm", false, false);
        assert!(wasm_opt.estimated_startup_ms < wasm_no_opt.estimated_startup_ms);
    }

    #[test]
    fn test_build_tauri_mobile_custom_size_with_models() {
        let with_models = build_tauri_mobile_custom("ios", true, false);
        let without_models = build_tauri_mobile_custom("ios", false, false);
        assert!(with_models.build_size_mb > without_models.build_size_mb);
    }

    #[test]
    fn test_full_ecosystem_integration_pipeline() {
        // Full pipeline: integrate model → validate → build mobile
        let device = DeviceConfig::desktop();
        let integration = integrate_hf_model("ed2k/sae-audit", &device);
        assert!(integration.success);
        assert_eq!(integration.precision, ModelPrecision::Full);

        let mobile = build_tauri_mobile();
        assert!(mobile.success);
        assert!(mobile.wasm_optimized);
        assert!(!mobile.features.is_empty());
    }
}
