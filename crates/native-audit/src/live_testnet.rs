//! Live Testnet Mesh — Planetary P2P mesh with PoSym enforcement for real-world deployment.
//!
//! Extends simulated testnet (`testnet_sim`) with real-world constraints:
//! 3G latency, battery-aware scheduling, churn resilience, energy tracking,
//! and PoSym-based trust enforcement for the Planetary Immune Mesh.
//!
//! **Sprint 120:** Planetary Immune Mesh & Edge Real-World Deployment.

use crate::edge_runtime::{ComputePath, DeviceType, PlanetaryImpactMetrics, PowerState};
use rand::Rng;
use sha2::{Digest, Sha256};

/// Cryptographic hash digest (SHA-256).
pub type Hash = [u8; 32];

/// Minimal PoSym engine for mesh nodes (self-contained, no external consensus dep).
#[derive(Debug, Clone)]
pub struct ProofOfSymbiosis {
    pub node_id: u64,
    pub contributions: Vec<SteerContribution>,
    pub uptime: u64,
    pub weight_steers: f64,
    pub weight_vfe: f64,
    pub weight_uptime: f64,
}

impl ProofOfSymbiosis {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            contributions: Vec::new(),
            uptime: 0,
            weight_steers: 0.6,
            weight_vfe: 0.3,
            weight_uptime: 0.1,
        }
    }

    pub fn add_contribution(&mut self, c: SteerContribution) {
        self.contributions.push(c);
    }

    pub fn score(&self) -> f64 {
        let steers = self.contributions.len() as f64;
        let total_vfe: f64 = self.contributions.iter().map(|c| c.vfe_reduction()).sum();
        let uptime = self.uptime as f64;
        self.weight_steers * (steers + 1.0).ln()
            + self.weight_vfe * (1.0 + total_vfe).ln()
            + self.weight_uptime * (uptime + 1.0).ln()
    }
}

/// A single certified steering contribution.
#[derive(Debug, Clone)]
pub struct SteerContribution {
    pub node_id: u64,
    pub timestamp: u64,
    pub vfe_before: f64,
    pub vfe_after: f64,
    pub hash: Hash,
}

impl SteerContribution {
    pub fn new(node_id: u64, vfe_before: f64, vfe_after: f64) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut hasher = Sha256::new();
        hasher.update(node_id.to_le_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.update(vfe_before.to_le_bytes());
        hasher.update(vfe_after.to_le_bytes());
        let hash = hasher.finalize().into();
        Self {
            node_id,
            timestamp,
            vfe_before,
            vfe_after,
            hash,
        }
    }

    pub fn vfe_reduction(&self) -> f64 {
        (self.vfe_before - self.vfe_after).max(0.0)
    }
}

/// Planetary mesh node — real-world edge node with PoSym enforcement.
pub struct PlanetaryMeshNode {
    /// Node ID
    pub node_id: u64,
    /// Proof of Symbiosis state
    pub posym: ProofOfSymbiosis,
    /// Current trust score (0.0 to 1.0)
    pub trust_score: f64,
    /// Connected peer count
    pub peer_count: usize,
    /// Total certifications performed
    pub total_certs: u64,
    /// Total energy saved (mWh)
    pub energy_saved_mwh: f64,
    /// VFE reduction achieved
    pub vfe_reduction: f64,
    /// Node uptime (seconds)
    pub uptime: u64,
    /// Is node currently active
    pub active: bool,
    /// Simulated 3G latency (ms)
    pub latency_ms: u64,
    /// Device type
    pub device_type: DeviceType,
    /// Power state
    pub power_state: PowerState,
}

impl PlanetaryMeshNode {
    /// Create new mesh node.
    pub fn new(node_id: u64, device_type: DeviceType) -> Self {
        Self {
            node_id,
            posym: ProofOfSymbiosis::new(node_id),
            trust_score: 0.5,
            peer_count: 0,
            total_certs: 0,
            energy_saved_mwh: 0.0,
            vfe_reduction: 0.0,
            uptime: 0,
            active: true,
            latency_ms: 100,
            device_type,
            power_state: PowerState::Normal,
        }
    }

    /// Record certified steer with energy tracking.
    pub fn record_certified_steer(
        &mut self,
        vfe_before: f64,
        vfe_after: f64,
        energy_saved: f64,
        compute_path: ComputePath,
    ) {
        self.total_certs += 1;
        self.energy_saved_mwh += energy_saved;
        self.vfe_reduction += (vfe_before - vfe_after).max(0.0);

        let contribution = SteerContribution::new(self.node_id, vfe_before, vfe_after);
        self.posym.add_contribution(contribution);

        // Update power state based on compute path
        if matches!(compute_path, ComputePath::UltraLight) {
            self.power_state = PowerState::UltraConservative;
        } else if matches!(compute_path, ComputePath::FastPathOnly) {
            self.power_state = PowerState::Conservative;
        } else {
            self.power_state = PowerState::Normal;
        }
    }

    /// Update trust score based on PoSym score and peer feedback.
    pub fn update_trust(&mut self, peer_trust_scores: &[f64]) {
        let posym_score = self.posym.score();
        let peer_avg = if peer_trust_scores.is_empty() {
            0.5
        } else {
            peer_trust_scores.iter().sum::<f64>() / peer_trust_scores.len() as f64
        };

        // Trust = 60% PoSym + 40% peer consensus (normalized)
        let raw = 0.6 * posym_score + 0.4 * peer_avg;
        self.trust_score = (raw / 10.0).clamp(0.0, 1.0);
    }

    /// Check if node meets minimum trust threshold for mesh participation.
    pub fn meets_trust_threshold(&self, threshold: f64) -> bool {
        self.trust_score >= threshold && self.active
    }

    /// Simulate node churn (disconnect/reconnect).
    pub fn simulate_churn(&mut self, churn_probability: f64) {
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < churn_probability {
            self.active = false;
            self.peer_count = 0;
        } else {
            self.active = true;
            self.uptime += 1;
            self.posym.uptime += 1;
        }
    }

    /// Simulate 3G latency variation.
    pub fn simulate_3g_latency(&mut self, base_latency: u64, variance: u64) {
        let mut rng = rand::thread_rng();
        let delta = rng.gen_range(-(variance as i64)..=(variance as i64));
        self.latency_ms = (base_latency as i64 + delta).max(10) as u64;
    }

    /// Simulate one round of edge operation.
    pub fn simulate_round(&mut self, churn_prob: f64, base_latency: u64, latency_var: u64) {
        self.simulate_churn(churn_prob);
        self.simulate_3g_latency(base_latency, latency_var);

        if self.active {
            let mut rng = rand::thread_rng();
            let vfe_before: f64 = rng.gen_range(0.5..1.0);
            let vfe_after: f64 = rng.gen_range(0.1..vfe_before.min(0.5));
            let energy_saved = rng.gen_range(0.5..0.8);

            // Choose compute path based on power state
            let compute_path = match self.power_state {
                PowerState::UltraConservative => ComputePath::UltraLight,
                PowerState::Conservative => ComputePath::FastPathOnly,
                PowerState::Normal | PowerState::Charging => ComputePath::FullHybrid,
            };

            self.record_certified_steer(vfe_before, vfe_after, energy_saved, compute_path);
        }
    }
}

/// Mesh statistics for real-world deployment monitoring.
#[derive(Debug, Clone)]
pub struct MeshStats {
    /// Total nodes in mesh
    pub total_nodes: usize,
    /// Active nodes
    pub active_nodes: usize,
    /// Average trust score
    pub avg_trust_score: f64,
    /// Average 3G latency (ms)
    pub avg_latency_ms: u64,
    /// Total certifications across mesh
    pub total_certifications: u64,
    /// Total energy saved (mWh)
    pub total_energy_saved_mwh: f64,
    /// Total VFE reduction
    pub total_vfe_reduction: f64,
    /// Churn rate (nodes lost / total)
    pub churn_rate: f64,
    /// Nodes evicted for low trust
    pub evicted_nodes: usize,
    /// Steering coverage percentage
    pub steering_coverage_pct: f64,
}

impl Default for MeshStats {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            active_nodes: 0,
            avg_trust_score: 0.0,
            avg_latency_ms: 0,
            total_certifications: 0,
            total_energy_saved_mwh: 0.0,
            total_vfe_reduction: 0.0,
            churn_rate: 0.0,
            evicted_nodes: 0,
            steering_coverage_pct: 0.0,
        }
    }
}

impl MeshStats {
    /// Calculate stats from mesh nodes.
    pub fn from_nodes(nodes: &[PlanetaryMeshNode], trust_threshold: f64) -> Self {
        let total_nodes = nodes.len();
        if total_nodes == 0 {
            return Self::default();
        }

        let active_nodes = nodes.iter().filter(|n| n.active).count();
        let trusted_nodes: Vec<&PlanetaryMeshNode> = nodes
            .iter()
            .filter(|n| n.meets_trust_threshold(trust_threshold))
            .collect();

        let avg_trust = nodes.iter().map(|n| n.trust_score).sum::<f64>() / total_nodes as f64;
        let avg_latency: u64 = nodes.iter().map(|n| n.latency_ms).sum::<u64>() / total_nodes as u64;
        let total_certs: u64 = nodes.iter().map(|n| n.total_certs).sum();
        let total_energy: f64 = nodes.iter().map(|n| n.energy_saved_mwh).sum();
        let total_vfe: f64 = nodes.iter().map(|n| n.vfe_reduction).sum();
        let evicted = nodes
            .iter()
            .filter(|n| !n.meets_trust_threshold(trust_threshold))
            .count();
        let churned = nodes.iter().filter(|n| !n.active).count();

        Self {
            total_nodes,
            active_nodes,
            avg_trust_score: avg_trust,
            avg_latency_ms: avg_latency,
            total_certifications: total_certs,
            total_energy_saved_mwh: total_energy,
            total_vfe_reduction: total_vfe,
            churn_rate: churned as f64 / total_nodes as f64,
            evicted_nodes: evicted,
            steering_coverage_pct: trusted_nodes.len() as f64 / total_nodes as f64 * 100.0,
        }
    }

    /// Display mesh stats summary.
    pub fn summary(&self) -> String {
        format!(
            "Mesh Stats:\n\
             ──────────────────────────────\n\
             Total Nodes: {} | Active: {}\n\
             Avg Trust: {:.3} | Evicted: {}\n\
             Avg 3G Latency: {} ms\n\
             Certifications: {}\n\
             Energy Saved: {:.2} mWh\n\
             VFE Reduction: {:.4}\n\
             Churn Rate: {:.2}%\n\
             Steering Coverage: {:.1}%",
            self.total_nodes,
            self.active_nodes,
            self.avg_trust_score,
            self.evicted_nodes,
            self.avg_latency_ms,
            self.total_certifications,
            self.total_energy_saved_mwh,
            self.total_vfe_reduction,
            self.churn_rate * 100.0,
            self.steering_coverage_pct,
        )
    }
}

/// Live mesh simulation — simulates real-world planetary mesh with churn, 3G latency, and PoSym enforcement.
pub struct LiveMeshSimulation {
    /// Mesh nodes
    pub nodes: Vec<PlanetaryMeshNode>,
    /// Trust threshold for mesh participation
    pub trust_threshold: f64,
    /// Churn probability per round
    pub churn_probability: f64,
    /// Base 3G latency (ms)
    pub base_3g_latency: u64,
    /// Latency variance (ms)
    pub latency_variance: u64,
    /// Simulation round
    pub current_round: u64,
    /// Device type distribution
    pub device_mix: Vec<DeviceType>,
}

impl LiveMeshSimulation {
    /// Create new simulation with specified node count and realistic device mix.
    pub fn new(node_count: usize) -> Self {
        // Realistic device mix: 20% desktop, 30% old desktop, 30% mobile, 20% IoT
        let device_mix: Vec<DeviceType> = (0..node_count)
            .map(|i| match i % 10 {
                0..=1 => DeviceType::Desktop,
                2..=4 => DeviceType::OldDesktop,
                5..=7 => DeviceType::Mobile,
                8..=9 => DeviceType::Iot,
                _ => unreachable!(),
            })
            .collect();

        let nodes: Vec<PlanetaryMeshNode> = device_mix
            .iter()
            .enumerate()
            .map(|(i, dt)| PlanetaryMeshNode::new(i as u64, *dt))
            .collect();

        Self {
            nodes,
            trust_threshold: 0.3,
            churn_probability: 0.05,
            base_3g_latency: 150,
            latency_variance: 100,
            current_round: 0,
            device_mix,
        }
    }

    /// Configure trust threshold.
    pub fn with_trust_threshold(mut self, threshold: f64) -> Self {
        self.trust_threshold = threshold;
        self
    }

    /// Configure churn probability.
    pub fn with_churn_probability(mut self, probability: f64) -> Self {
        self.churn_probability = probability;
        self
    }

    /// Configure 3G latency.
    pub fn with_3g_latency(mut self, base: u64, variance: u64) -> Self {
        self.base_3g_latency = base;
        self.latency_variance = variance;
        self
    }

    /// Run one simulation round.
    pub fn step(&mut self) -> MeshStats {
        self.current_round += 1;

        // Simulate each node
        for node in &mut self.nodes {
            node.simulate_round(
                self.churn_probability,
                self.base_3g_latency,
                self.latency_variance,
            );
        }

        // Update trust scores
        let trust_scores: Vec<f64> = self.nodes.iter().map(|n| n.trust_score).collect();
        for node in &mut self.nodes {
            let peer_scores: Vec<f64> = trust_scores
                .iter()
                .filter(|&&s| s != node.trust_score)
                .copied()
                .collect();
            node.update_trust(&peer_scores);
        }

        MeshStats::from_nodes(&self.nodes, self.trust_threshold)
    }

    /// Run simulation for specified number of rounds.
    pub fn run(&mut self, rounds: u64) -> Vec<MeshStats> {
        (0..rounds).map(|_| self.step()).collect()
    }

    /// Get final mesh stats.
    pub fn final_stats(&self) -> MeshStats {
        MeshStats::from_nodes(&self.nodes, self.trust_threshold)
    }

    /// Calculate planetary impact metrics from simulation.
    pub fn planetary_impact(&self) -> PlanetaryImpactMetrics {
        let stats = self.final_stats();
        let mut metrics = PlanetaryImpactMetrics::new(
            stats.total_certifications,
            stats.active_nodes as u64,
            stats.steering_coverage_pct,
            if stats.total_certifications > 0 {
                stats.total_vfe_reduction / stats.total_certifications as f64
            } else {
                0.0
            },
        );
        metrics.global_energy_saved_mwh = stats.total_energy_saved_mwh;
        metrics.churn_rate = stats.churn_rate;
        metrics.avg_3g_latency_ms = stats.avg_latency_ms as f64;
        metrics.avg_posym_trust = stats.avg_trust_score;
        metrics
    }
}

/// Bootstrap peers for live mesh connection.
#[derive(Debug, Clone)]
pub struct BootstrapPeer {
    /// Multiaddr string
    pub multiaddr: String,
    /// Node ID
    pub node_id: u64,
    /// Trust score
    pub trust_score: f64,
}

impl BootstrapPeer {
    pub fn new(multiaddr: String, node_id: u64, trust_score: f64) -> Self {
        Self {
            multiaddr,
            node_id,
            trust_score,
        }
    }
}

/// Live mesh bootstrap configuration.
pub struct LiveMeshBootstrap {
    /// Bootstrap peers
    pub peers: Vec<BootstrapPeer>,
    /// Minimum trust threshold
    pub min_trust: f64,
    /// Maximum peer count
    pub max_peers: usize,
}

impl Default for LiveMeshBootstrap {
    fn default() -> Self {
        Self {
            peers: Vec::new(),
            min_trust: 0.3,
            max_peers: 20,
        }
    }
}

impl LiveMeshBootstrap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_peer(mut self, peer: BootstrapPeer) -> Self {
        self.peers.push(peer);
        self
    }

    pub fn with_min_trust(mut self, trust: f64) -> Self {
        self.min_trust = trust;
        self
    }

    pub fn with_max_peers(mut self, max: usize) -> Self {
        self.max_peers = max;
        self
    }

    pub fn trusted_peers(&self) -> Vec<&BootstrapPeer> {
        self.peers
            .iter()
            .filter(|p| p.trust_score >= self.min_trust)
            .take(self.max_peers)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posym_creation() {
        let posym = ProofOfSymbiosis::new(42);
        assert_eq!(posym.node_id, 42);
        assert_eq!(posym.score(), 0.0); // No contributions yet
    }

    #[test]
    fn test_posym_score_increases() {
        let mut posym = ProofOfSymbiosis::new(1);
        posym.add_contribution(SteerContribution::new(1, 0.8, 0.3));
        posym.uptime = 100;
        assert!(posym.score() > 0.0);
    }

    #[test]
    fn test_steer_contribution_hash() {
        let c = SteerContribution::new(1, 0.8, 0.3);
        assert!(c.hash != [0u8; 32]);
        assert_eq!(c.vfe_reduction(), 0.5);
    }

    #[test]
    fn test_mesh_node_creation() {
        let node = PlanetaryMeshNode::new(42, DeviceType::Mobile);
        assert_eq!(node.node_id, 42);
        assert_eq!(node.trust_score, 0.5);
        assert!(node.active);
        assert_eq!(node.device_type, DeviceType::Mobile);
    }

    #[test]
    fn test_record_certified_steer() {
        let mut node = PlanetaryMeshNode::new(1, DeviceType::Desktop);
        node.record_certified_steer(0.8, 0.3, 0.7, ComputePath::FullHybrid);
        assert_eq!(node.total_certs, 1);
        assert!(node.energy_saved_mwh > 0.0);
        assert!(node.vfe_reduction > 0.0);
    }

    #[test]
    fn test_trust_update() {
        let mut node = PlanetaryMeshNode::new(1, DeviceType::Desktop);
        node.record_certified_steer(0.8, 0.3, 0.7, ComputePath::FullHybrid);
        node.posym.uptime = 100;
        node.update_trust(&[0.6, 0.7, 0.8]);
        assert!(node.trust_score > 0.0);
        assert!(node.trust_score <= 1.0);
    }

    #[test]
    fn test_trust_threshold() {
        let node = PlanetaryMeshNode::new(1, DeviceType::Desktop);
        assert!(node.meets_trust_threshold(0.4));
        assert!(!node.meets_trust_threshold(0.6));
    }

    #[test]
    fn test_inactive_node_fails_trust() {
        let mut node = PlanetaryMeshNode::new(1, DeviceType::Desktop);
        node.active = false;
        assert!(!node.meets_trust_threshold(0.1));
    }

    #[test]
    fn test_mesh_stats_empty() {
        let stats = MeshStats::from_nodes(&[], 0.3);
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.active_nodes, 0);
    }

    #[test]
    fn test_mesh_stats_from_nodes() {
        let mut nodes = vec![
            PlanetaryMeshNode::new(1, DeviceType::Desktop),
            PlanetaryMeshNode::new(2, DeviceType::Mobile),
        ];
        nodes[0].record_certified_steer(0.8, 0.3, 0.7, ComputePath::FullHybrid);
        nodes[1].active = false;

        let stats = MeshStats::from_nodes(&nodes, 0.3);
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.active_nodes, 1);
        assert_eq!(stats.total_certifications, 1);
    }

    #[test]
    fn test_mesh_stats_summary() {
        let stats = MeshStats::from_nodes(&[PlanetaryMeshNode::new(1, DeviceType::Desktop)], 0.3);
        let summary = stats.summary();
        assert!(summary.contains("Mesh Stats"));
        assert!(summary.contains("Total Nodes"));
    }

    #[test]
    fn test_live_mesh_simulation() {
        let mut sim = LiveMeshSimulation::new(10);
        let stats = sim.step();
        assert_eq!(stats.total_nodes, 10);
        assert!(stats.active_nodes <= 10);
    }

    #[test]
    fn test_live_mesh_run() {
        let mut sim = LiveMeshSimulation::new(5).with_churn_probability(0.1);
        let history = sim.run(3);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].total_nodes, 5);
    }

    #[test]
    fn test_simulation_config() {
        let sim = LiveMeshSimulation::new(20)
            .with_trust_threshold(0.5)
            .with_churn_probability(0.1)
            .with_3g_latency(200, 150);
        assert_eq!(sim.trust_threshold, 0.5);
        assert_eq!(sim.churn_probability, 0.1);
        assert_eq!(sim.base_3g_latency, 200);
    }

    #[test]
    fn test_device_mix() {
        let sim = LiveMeshSimulation::new(20);
        assert_eq!(sim.device_mix.len(), 20);
        // Check distribution
        let desktops = sim
            .device_mix
            .iter()
            .filter(|d| matches!(d, DeviceType::Desktop))
            .count();
        let mobiles = sim
            .device_mix
            .iter()
            .filter(|d| matches!(d, DeviceType::Mobile))
            .count();
        assert!(desktops > 0);
        assert!(mobiles > 0);
    }

    #[test]
    fn test_bootstrap_peer() {
        let peer = BootstrapPeer::new("/ip4/192.168.1.1/tcp/4001".to_string(), 1, 0.8);
        assert_eq!(peer.node_id, 1);
        assert_eq!(peer.trust_score, 0.8);
    }

    #[test]
    fn test_bootstrap_trusted_peers() {
        let bootstrap = LiveMeshBootstrap::new()
            .add_peer(BootstrapPeer::new("peer1".to_string(), 1, 0.8))
            .add_peer(BootstrapPeer::new("peer2".to_string(), 2, 0.2))
            .with_min_trust(0.5);
        let trusted = bootstrap.trusted_peers();
        assert_eq!(trusted.len(), 1);
        assert_eq!(trusted[0].node_id, 1);
    }

    #[test]
    fn test_bootstrap_max_peers() {
        let bootstrap = LiveMeshBootstrap::new()
            .add_peer(BootstrapPeer::new("p1".to_string(), 1, 0.9))
            .add_peer(BootstrapPeer::new("p2".to_string(), 2, 0.8))
            .add_peer(BootstrapPeer::new("p3".to_string(), 3, 0.7))
            .with_max_peers(2);
        let trusted = bootstrap.trusted_peers();
        assert_eq!(trusted.len(), 2);
    }

    #[test]
    fn test_churn_effect() {
        let mut sim = LiveMeshSimulation::new(100).with_churn_probability(0.5);
        let stats = sim.step();
        assert!(stats.churn_rate > 0.1);
    }

    #[test]
    fn test_energy_accumulation() {
        let mut sim = LiveMeshSimulation::new(10);
        sim.run(5);
        let stats = sim.final_stats();
        assert!(stats.total_energy_saved_mwh > 0.0);
    }

    #[test]
    fn test_steering_coverage() {
        let mut sim = LiveMeshSimulation::new(20).with_trust_threshold(0.3);
        sim.run(3);
        let stats = sim.final_stats();
        assert!(stats.steering_coverage_pct >= 0.0);
        assert!(stats.steering_coverage_pct <= 100.0);
    }

    #[test]
    fn test_planetary_impact() {
        let mut sim = LiveMeshSimulation::new(10);
        sim.run(5);
        let impact = sim.planetary_impact();
        assert!(impact.global_energy_saved_mwh > 0.0);
        assert!(impact.total_certifications > 0);
        assert!(impact.co2_saved_kg() > 0.0);
    }

    #[test]
    fn test_node_simulate_round() {
        let mut node = PlanetaryMeshNode::new(1, DeviceType::Desktop);
        node.simulate_round(0.0, 100, 50); // 0% churn = always active
        assert!(node.active);
        assert_eq!(node.total_certs, 1);
    }

    #[test]
    fn test_power_state_from_compute_path() {
        let mut node = PlanetaryMeshNode::new(1, DeviceType::Desktop);
        node.record_certified_steer(0.8, 0.3, 0.7, ComputePath::UltraLight);
        assert_eq!(node.power_state, PowerState::UltraConservative);

        node.record_certified_steer(0.8, 0.3, 0.7, ComputePath::FastPathOnly);
        assert_eq!(node.power_state, PowerState::Conservative);

        node.record_certified_steer(0.8, 0.3, 0.7, ComputePath::FullHybrid);
        assert_eq!(node.power_state, PowerState::Normal);
    }
}
