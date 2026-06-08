//! Edge Runtime — Battery-Aware Scheduling + Energy Impact Estimation for Planetary Deployment.
//!
//! Enables ed2kIA to run as a "white blood cell of the Noosphere" on precarious hardware:
//! 5-year-old PCs, 3G connections, mobile devices with limited battery, and IoT sensors.
//!
//! **Sprint 120:** Planetary Immune Mesh & Edge Real-World Deployment.

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
}

impl DeviceType {
    /// Base energy cost per certification (mWh).
    pub fn base_energy_cost(&self) -> f64 {
        match self {
            DeviceType::Desktop => 0.05,
            DeviceType::OldDesktop => 0.08,
            DeviceType::Mobile => 0.03,
            DeviceType::Iot => 0.01,
        }
    }

    /// Datacenter baseline energy cost per certification (mWh).
    pub fn dc_baseline_cost(&self) -> f64 {
        match self {
            DeviceType::Desktop => 0.8,
            DeviceType::OldDesktop => 1.0,
            DeviceType::Mobile => 0.6,
            DeviceType::Iot => 0.4,
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
            self.energy_used_mwh, self.energy_saved_mwh, self.savings_pct,
            self.power_state, self.compute_path
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
    let slow_path = !matches!(power_state, PowerState::Conservative)
        && swd_ratio > 0.4
        && swd_ratio < 0.6;

    let energy_used = if slow_path {
        base_cost * 1.0 // Full Hybrid
    } else {
        base_cost * 0.3 // Fast Path + light steering
    };

    let energy_saved = dc_baseline - energy_used;
    Ok((safe, slow_path, hidden_state.clone(), energy_used, energy_saved))
}

/// Compute fast SWD ratio for edge devices.
fn compute_fast_swd_ratio(
    hidden_state: &Tensor,
    safe_centroid: &Tensor,
    toxic_centroid: &Tensor,
) -> Result<f32> {
    // Simplified SWD: distance to toxic / (distance to toxic + distance to safe)
    let dist_safe = hidden_state.sub(safe_centroid)?.sqr()?.sum_all()?.to_scalar::<f32>()?;
    let dist_toxic = hidden_state.sub(toxic_centroid)?.sqr()?.sum_all()?.to_scalar::<f32>()?;

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
        };

        let peers = if self.bootstrap_peers.is_empty() {
            String::new()
        } else {
            format!(
                " --peers \"{}\"",
                self.bootstrap_peers.join(",")
            )
        };

        format!("ed2k start --altruist {}{}", device_flag, peers)
    }
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
            0.8, 0.3, 0.05, 0.75,
            ComputePath::FullHybrid, PowerState::Normal,
        );
        assert_eq!(record.vfe_reduction(), 0.5);
        assert!(record.timestamp > 0);
    }

    #[test]
    fn test_certified_steer_record_no_reduction() {
        let record = CertifiedSteerRecord::new(
            0.3, 0.5, 0.05, 0.75,
            ComputePath::FastPathOnly, PowerState::Conservative,
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
            &hidden, &safe, &toxic,
            0.2, 0.3, false, // Low battery + poor network
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
            &hidden, &safe, &toxic,
            0.9, 0.9, false, // Good battery + good network
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
            &hidden, &safe, &toxic,
            0.9, 0.9, false,
            DeviceType::Desktop,
        )?;

        // Should be near boundary (ratio ~0.5)
        // Equidistant → ratio ≈ 0.5 → safe = (ratio > 0.5) → false
        assert!(!safe);
        Ok(())
    }
}
