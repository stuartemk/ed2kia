//! Global Bootstrap — Planetary-scale altruist mesh bootstrap for Noosfera Symbiotic Launch.
//!
//! Enables ed2kIA to bootstrap a global mesh of altruistic nodes spanning the full hardware
//! spectrum (smartwatch → datacenter), with proportional contribution tracking via PoSym
//! and energy-aware onboarding.
//!
//! **Sprint 121 (v12.1.0):** Noosfera Symbiotic Launch & Verifiable Global Defense.

use crate::edge_runtime::{AltruistOnboarding, DeviceType};
use crate::live_testnet::{MeshStats, PlanetaryMeshNode};

/// Device distribution profile for planetary bootstrap simulation.
///
/// Represents the expected hardware mix in a global deployment:
/// - 40% Smartwatch/IoT (ultra-light edge)
/// - 30% Mobile/OldDesktop (mid-tier)
/// - 20% Desktop (standard)
/// - 10% Datacenter (heavy infrastructure)
#[derive(Debug, Clone, Copy)]
pub struct DeviceDistribution {
    pub smartwatch_pct: f64,
    pub iot_pct: f64,
    pub mobile_pct: f64,
    pub old_desktop_pct: f64,
    pub desktop_pct: f64,
    pub datacenter_pct: f64,
}

impl Default for DeviceDistribution {
    fn default() -> Self {
        Self {
            smartwatch_pct: 0.20,
            iot_pct: 0.20,
            mobile_pct: 0.20,
            old_desktop_pct: 0.10,
            desktop_pct: 0.20,
            datacenter_pct: 0.10,
        }
    }
}

impl DeviceDistribution {
    /// Select a device type based on the distribution profile.
    pub fn select(&self, index: usize) -> DeviceType {
        let r = (index % 100) as f64 / 100.0;
        let mut cumulative = 0.0;

        cumulative += self.smartwatch_pct;
        if r < cumulative {
            return DeviceType::Smartwatch;
        }
        cumulative += self.iot_pct;
        if r < cumulative {
            return DeviceType::Iot;
        }
        cumulative += self.mobile_pct;
        if r < cumulative {
            return DeviceType::Mobile;
        }
        cumulative += self.old_desktop_pct;
        if r < cumulative {
            return DeviceType::OldDesktop;
        }
        cumulative += self.desktop_pct;
        if r < cumulative {
            return DeviceType::Desktop;
        }
        DeviceType::Datacenter
    }

    /// Verify distribution sums to 1.0.
    pub fn is_valid(&self) -> bool {
        let total = self.smartwatch_pct
            + self.iot_pct
            + self.mobile_pct
            + self.old_desktop_pct
            + self.desktop_pct
            + self.datacenter_pct;
        (total - 1.0).abs() < 1e-9
    }
}

/// Bootstrap configuration for the global altruist mesh.
#[derive(Debug, Clone)]
pub struct GlobalBootstrapConfig {
    /// Total number of nodes in the planetary mesh.
    pub node_count: usize,
    /// Device distribution profile.
    pub distribution: DeviceDistribution,
    /// Minimum trust threshold for node acceptance.
    pub trust_threshold: f64,
    /// Churn probability per simulation step.
    pub churn_probability: f64,
    /// Number of simulation steps.
    pub simulation_steps: usize,
    /// Base 3G latency in milliseconds.
    pub base_latency_ms: u64,
    /// Latency variance in milliseconds.
    pub latency_variance_ms: u64,
}

impl Default for GlobalBootstrapConfig {
    fn default() -> Self {
        Self {
            node_count: 5000,
            distribution: DeviceDistribution::default(),
            trust_threshold: 0.6,
            churn_probability: 0.05,
            simulation_steps: 10,
            base_latency_ms: 300,
            latency_variance_ms: 100,
        }
    }
}

/// Result of a planetary bootstrap simulation.
#[derive(Debug)]
pub struct BootstrapResult {
    /// Final mesh statistics.
    pub mesh_stats: MeshStats,
    /// Total energy saved across all nodes (mWh).
    pub total_energy_saved_mwh: f64,
    /// Average VFE reduction across the mesh.
    pub avg_vfe_reduction: f64,
    /// Number of active (trusted) nodes.
    pub active_nodes: usize,
    /// Number of ultra-light nodes (smartwatch + IoT).
    pub ultra_light_nodes: usize,
    /// Number of mid-tier nodes (mobile + old desktop).
    pub mid_tier_nodes: usize,
    /// Number of standard nodes (desktop).
    pub standard_nodes: usize,
    /// Number of heavy infrastructure nodes (datacenter).
    pub heavy_nodes: usize,
    /// Average contribution factor across all nodes.
    pub avg_contribution_factor: f64,
    /// PoSym participation rate (nodes with trust > threshold).
    pub posym_participation_rate: f64,
}

impl std::fmt::Display for BootstrapResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BootstrapResult {{\n  \
             active_nodes={}, ultra_light={}, mid_tier={}, standard={}, heavy={}\n  \
             total_energy_saved={:.2}mWh, avg_vfe_reduction={:.4}\n  \
             avg_contribution_factor={:.2}x, posym_participation={:.1}%\n  \
             trusted={}, untrusted={}\n  \
             }}",
            self.active_nodes,
            self.ultra_light_nodes,
            self.mid_tier_nodes,
            self.standard_nodes,
            self.heavy_nodes,
            self.total_energy_saved_mwh,
            self.avg_vfe_reduction,
            self.avg_contribution_factor,
            self.posym_participation_rate * 100.0,
            self.mesh_stats.active_nodes,
            self.mesh_stats.evicted_nodes,
        )
    }
}

/// Global Altruist Mesh — Coordinates planetary-scale bootstrap and simulation.
pub struct GlobalAltruistMesh {
    /// Configuration for the bootstrap.
    config: GlobalBootstrapConfig,
    /// Mesh nodes.
    nodes: Vec<PlanetaryMeshNode>,
}

impl GlobalAltruistMesh {
    /// Create a new global altruist mesh with the given configuration.
    pub fn new(config: GlobalBootstrapConfig) -> Self {
        let mut nodes = Vec::with_capacity(config.node_count);
        for i in 0..config.node_count {
            let device_type = config.distribution.select(i);
            let node = PlanetaryMeshNode::new(i as u64, device_type);
            nodes.push(node);
        }
        Self { config, nodes }
    }

    /// Create a mesh with default configuration.
    pub fn default_planetary() -> Self {
        Self::new(GlobalBootstrapConfig::default())
    }

    /// Execute the planetary bootstrap simulation.
    pub fn bootstrap_planetary(&mut self) -> BootstrapResult {
        for _step in 0..self.config.simulation_steps {
            for node in &mut self.nodes {
                node.simulate_round(
                    self.config.churn_probability,
                    self.config.base_latency_ms,
                    self.config.latency_variance_ms,
                );
            }
        }

        let stats = MeshStats::from_nodes(&self.nodes, self.config.trust_threshold);

        let mut total_energy = 0.0;
        let mut total_vfe = 0.0;
        let mut total_cf = 0.0;
        let mut ultra_light = 0usize;
        let mut mid_tier = 0usize;
        let mut standard = 0usize;
        let mut heavy = 0usize;
        let mut active = 0usize;

        for node in &self.nodes {
            total_energy += node.energy_saved_mwh;
            total_vfe += node.vfe_reduction;
            total_cf += node.device_type.contribution_factor();

            match node.device_type {
                DeviceType::Smartwatch | DeviceType::Iot => ultra_light += 1,
                DeviceType::Mobile | DeviceType::OldDesktop => mid_tier += 1,
                DeviceType::Desktop => standard += 1,
                DeviceType::Datacenter => heavy += 1,
            }

            if node.active && node.trust_score >= self.config.trust_threshold {
                active += 1;
            }
        }

        let node_count = self.nodes.len() as f64;
        BootstrapResult {
            mesh_stats: stats,
            total_energy_saved_mwh: total_energy,
            avg_vfe_reduction: if node_count > 0.0 {
                total_vfe / node_count
            } else {
                0.0
            },
            active_nodes: active,
            ultra_light_nodes: ultra_light,
            mid_tier_nodes: mid_tier,
            standard_nodes: standard,
            heavy_nodes: heavy,
            avg_contribution_factor: if node_count > 0.0 {
                total_cf / node_count
            } else {
                0.0
            },
            posym_participation_rate: if node_count > 0.0 {
                active as f64 / node_count
            } else {
                0.0
            },
        }
    }

    /// Generate install commands for all nodes in the mesh.
    pub fn generate_install_commands(&self) -> Vec<String> {
        self.nodes
            .iter()
            .map(|node| {
                let onboarding = AltruistOnboarding::new(node.node_id, node.device_type);
                onboarding.install_command()
            })
            .collect()
    }

    /// Get the number of nodes in the mesh.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get a reference to the nodes.
    pub fn nodes(&self) -> &[PlanetaryMeshNode] {
        &self.nodes
    }

    /// Get a mutable reference to the nodes.
    pub fn nodes_mut(&mut self) -> &mut Vec<PlanetaryMeshNode> {
        &mut self.nodes
    }

    /// Get the configuration.
    pub fn config(&self) -> &GlobalBootstrapConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_distribution_default() {
        let dist = DeviceDistribution::default();
        assert!(dist.is_valid());
        assert_eq!(dist.smartwatch_pct, 0.20);
        assert_eq!(dist.iot_pct, 0.20);
        assert_eq!(dist.mobile_pct, 0.20);
        assert_eq!(dist.old_desktop_pct, 0.10);
        assert_eq!(dist.desktop_pct, 0.20);
        assert_eq!(dist.datacenter_pct, 0.10);
    }

    #[test]
    fn test_device_distribution_select_covers_all() {
        let dist = DeviceDistribution::default();
        let mut found = [false; 6]; // Smartwatch, Desktop, Mobile, Iot, Smartwatch, Datacenter
        for i in 0..100 {
            let dt = dist.select(i);
            match dt {
                DeviceType::Smartwatch => found[0] = true,
                DeviceType::Desktop => found[1] = true,
                DeviceType::Mobile => found[2] = true,
                DeviceType::Iot => found[3] = true,
                DeviceType::OldDesktop => found[4] = true,
                DeviceType::Datacenter => found[5] = true,
            }
        }
        assert!(
            found.iter().all(|&b| b),
            "Not all device types selected: {:?}",
            found
        );
    }

    #[test]
    fn test_bootstrap_config_default() {
        let config = GlobalBootstrapConfig::default();
        assert_eq!(config.node_count, 5000);
        assert_eq!(config.trust_threshold, 0.6);
        assert_eq!(config.churn_probability, 0.05);
        assert_eq!(config.simulation_steps, 10);
    }

    #[test]
    fn test_small_mesh_bootstrap() {
        let config = GlobalBootstrapConfig {
            node_count: 100,
            simulation_steps: 3,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let result = mesh.bootstrap_planetary();

        assert_eq!(
            result.ultra_light_nodes
                + result.mid_tier_nodes
                + result.standard_nodes
                + result.heavy_nodes,
            100
        );
        assert!(result.total_energy_saved_mwh >= 0.0);
        assert!(result.avg_vfe_reduction >= 0.0);
        assert!(result.avg_contribution_factor > 0.0);
        assert!(result.posym_participation_rate >= 0.0);
        assert!(result.posym_participation_rate <= 1.0);
    }

    #[test]
    fn test_bootstrap_result_display() {
        let config = GlobalBootstrapConfig {
            node_count: 10,
            simulation_steps: 1,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let result = mesh.bootstrap_planetary();
        let display = format!("{}", result);
        assert!(display.contains("active_nodes="));
        assert!(display.contains("total_energy_saved="));
        assert!(display.contains("posym_participation="));
    }

    #[test]
    fn test_generate_install_commands() {
        let config = GlobalBootstrapConfig {
            node_count: 10,
            ..GlobalBootstrapConfig::default()
        };
        let mesh = GlobalAltruistMesh::new(config);
        let commands = mesh.generate_install_commands();
        assert_eq!(commands.len(), 10);
        for cmd in &commands {
            assert!(cmd.contains("ed2k start --altruist"));
        }
    }

    #[test]
    fn test_mesh_node_count() {
        let config = GlobalBootstrapConfig {
            node_count: 250,
            ..GlobalBootstrapConfig::default()
        };
        let mesh = GlobalAltruistMesh::new(config);
        assert_eq!(mesh.node_count(), 250);
        assert_eq!(mesh.nodes().len(), 250);
    }

    #[test]
    fn test_default_planetary() {
        let mesh = GlobalAltruistMesh::default_planetary();
        assert_eq!(mesh.node_count(), 5000);
    }

    #[test]
    fn test_contribution_factor_weighting() {
        // Ultra-light devices should have higher average contribution factor
        let config = GlobalBootstrapConfig {
            node_count: 200,
            simulation_steps: 2,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let result = mesh.bootstrap_planetary();

        // Average contribution factor should be > 1.0 due to ultra-light bonus
        assert!(result.avg_contribution_factor > 1.0);
    }

    #[test]
    fn test_custom_distribution() {
        let dist = DeviceDistribution {
            smartwatch_pct: 0.40,
            iot_pct: 0.0,
            mobile_pct: 0.30,
            old_desktop_pct: 0.10,
            desktop_pct: 0.15,
            datacenter_pct: 0.05,
        };
        assert!(dist.is_valid());

        let config = GlobalBootstrapConfig {
            node_count: 100,
            distribution: dist,
            simulation_steps: 1,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let result = mesh.bootstrap_planetary();

        // With 40% smartwatch, ultra-light should be ~40
        assert!(result.ultra_light_nodes >= 30);
    }

    #[test]
    fn test_high_churn_resilience() {
        let config = GlobalBootstrapConfig {
            node_count: 200,
            churn_probability: 0.10,
            simulation_steps: 3,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let result = mesh.bootstrap_planetary();

        // Mesh should still have nodes tracked
        assert_eq!(
            result.ultra_light_nodes
                + result.mid_tier_nodes
                + result.standard_nodes
                + result.heavy_nodes,
            200
        );
        // With moderate churn, energy should still accumulate
        assert!(result.total_energy_saved_mwh >= 0.0);
    }

    #[test]
    fn test_energy_accumulation() {
        let config = GlobalBootstrapConfig {
            node_count: 50,
            simulation_steps: 5,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let result = mesh.bootstrap_planetary();

        assert!(result.total_energy_saved_mwh > 0.0);
    }

    #[test]
    fn test_nodes_mut_access() {
        let config = GlobalBootstrapConfig {
            node_count: 10,
            ..GlobalBootstrapConfig::default()
        };
        let mut mesh = GlobalAltruistMesh::new(config);
        let nodes = mesh.nodes_mut();
        assert_eq!(nodes.len(), 10);
    }

    #[test]
    fn test_config_reference() {
        let config = GlobalBootstrapConfig {
            node_count: 750,
            trust_threshold: 0.8,
            ..GlobalBootstrapConfig::default()
        };
        let mesh = GlobalAltruistMesh::new(config);
        assert_eq!(mesh.config().node_count, 750);
        assert_eq!(mesh.config().trust_threshold, 0.8);
    }
}
