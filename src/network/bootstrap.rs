//! Global Bootstrap Protocol — Seed Node Discovery for Planetary Mesh.
//!
//! Provides `BootstrapProtocol`, a discovery and onboarding system that enables
//! new nodes to find and connect to the ed2kIA network within <3 seconds using:
//!
//! - **Seed Nodes**: Pre-configured, high-availability bootstrap servers.
//! - **WebRTC-Star Discovery**: NAT traversal via centralized signaling.
//! - **Circuit Relay v2**: Indirect peer discovery through relay nodes.
//! - **DNS-SD Fallback**: DNS-based service discovery for local networks.
//!
//! **Architecture Principles:**
//! - Distribución: Decentralized seed node selection.
//! - Evolución: Adaptive bootstrap strategy based on network conditions.
//! - Cooperación: Seed nodes cooperate to distribute discovery load.
//! - Preservación: Bootstrap data preserved across network partitions.
//!
//! **Feature Gate:** `v3.7-symbiotic-portal`

use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::time::{Duration, Instant};

/// A seed node entry in the bootstrap configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedNode {
    /// Unique node identifier (128-bit).
    pub node_id: u128,
    /// Hostname or IP address.
    pub address: String,
    /// Port for P2P communication.
    pub port: u16,
    /// Supported transport protocols.
    pub transports: Vec<TransportType>,
    /// Geographic region for latency optimization.
    pub region: String,
    /// Last known heartbeat timestamp (Unix epoch seconds).
    pub last_heartbeat: u64,
    /// Node is currently active and accepting connections.
    pub active: bool,
}

impl SeedNode {
    /// Create a new SeedNode.
    pub fn new(node_id: u128, address: String, port: u16, region: String) -> Self {
        Self {
            node_id,
            address,
            port,
            transports: vec![TransportType::WebRTC, TransportType::Tcp],
            region,
            last_heartbeat: current_unix_time(),
            active: true,
        }
    }

    /// Check if this seed node is considered alive (heartbeat within timeout).
    pub fn is_alive(&self, timeout_secs: u64) -> bool {
        self.active && (current_unix_time().saturating_sub(self.last_heartbeat) <= timeout_secs)
    }

    /// Generate the connection endpoint string.
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

impl fmt::Display for SeedNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SeedNode(id={}, addr={}, region={}, active={})",
            self.node_id,
            self.endpoint(),
            self.region,
            self.active
        )
    }
}

/// Supported transport protocols for bootstrap discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    /// WebRTC with STUN/TURN fallback.
    WebRTC,
    /// TCP with TLS encryption.
    Tcp,
    /// QUIC for low-latency connections.
    Quic,
    /// WebSocket for browser-based nodes.
    WebSocket,
}

impl fmt::Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportType::WebRTC => write!(f, "webrtc"),
            TransportType::Tcp => write!(f, "tcp"),
            TransportType::Quic => write!(f, "quic"),
            TransportType::WebSocket => write!(f, "ws"),
        }
    }
}

/// Bootstrap strategy for network discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapStrategy {
    /// Use WebRTC-Star signaling for direct peer discovery.
    WebRTCStar,
    /// Use Circuit Relay v2 for indirect discovery.
    CircuitRelay,
    /// Use DNS-based service discovery.
    DnsSd,
    /// Use hardcoded seed node list.
    StaticSeeds,
    /// Automatic selection based on network conditions.
    Auto,
}

impl fmt::Display for BootstrapStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapStrategy::WebRTCStar => write!(f, "webrtc-star"),
            BootstrapStrategy::CircuitRelay => write!(f, "circuit-relay"),
            BootstrapStrategy::DnsSd => write!(f, "dns-sd"),
            BootstrapStrategy::StaticSeeds => write!(f, "static-seeds"),
            BootstrapStrategy::Auto => write!(f, "auto"),
        }
    }
}

/// Discovery result from a bootstrap attempt.
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// List of discovered peer addresses.
    pub peers: Vec<String>,
    /// Bootstrap strategy used.
    pub strategy: BootstrapStrategy,
    /// Time taken for discovery (milliseconds).
    pub discovery_time_ms: u64,
    /// Number of seed nodes contacted.
    pub seeds_contacted: usize,
    /// Discovery was successful (found at least one peer).
    pub success: bool,
}

impl DiscoveryResult {
    /// Create a successful discovery result.
    pub fn success(
        peers: Vec<String>,
        strategy: BootstrapStrategy,
        time_ms: u64,
        seeds: usize,
    ) -> Self {
        Self {
            peers,
            strategy,
            discovery_time_ms: time_ms,
            seeds_contacted: seeds,
            success: true,
        }
    }

    /// Create a failed discovery result.
    pub fn failed(strategy: BootstrapStrategy, time_ms: u64) -> Self {
        Self {
            peers: vec![],
            strategy,
            discovery_time_ms: time_ms,
            seeds_contacted: 0,
            success: false,
        }
    }
}

/// Errors in the bootstrap protocol.
#[derive(Debug, Clone)]
pub enum BootstrapError {
    /// No seed nodes configured.
    NoSeedNodes,
    /// All seed nodes are unreachable.
    AllSeedsUnreachable,
    /// Discovery timeout exceeded.
    Timeout,
    /// Invalid seed node configuration.
    InvalidConfig(String),
    /// Network unreachable.
    NetworkUnreachable,
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapError::NoSeedNodes => write!(f, "No seed nodes configured"),
            BootstrapError::AllSeedsUnreachable => write!(f, "All seed nodes are unreachable"),
            BootstrapError::Timeout => write!(f, "Discovery timeout exceeded"),
            BootstrapError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            BootstrapError::NetworkUnreachable => write!(f, "Network unreachable"),
        }
    }
}

impl std::error::Error for BootstrapError {}

/// Configuration for the BootstrapProtocol.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// List of seed nodes for initial discovery.
    pub seed_nodes: Vec<SeedNode>,
    /// Primary bootstrap strategy.
    pub strategy: BootstrapStrategy,
    /// Discovery timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum number of peers to discover per round.
    pub max_peers: usize,
    /// Seed node heartbeat timeout in seconds.
    pub heartbeat_timeout_secs: u64,
    /// Enable WebRTC-Star signaling.
    pub enable_webrtc: bool,
    /// Enable Circuit Relay v2.
    pub enable_relay: bool,
    /// Enable DNS-SD fallback.
    pub enable_dns_sd: bool,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            seed_nodes: default_seed_nodes(),
            strategy: BootstrapStrategy::Auto,
            timeout_ms: 3000, // 3 second target
            max_peers: 20,
            heartbeat_timeout_secs: 30,
            enable_webrtc: true,
            enable_relay: true,
            enable_dns_sd: true,
        }
    }
}

impl BootstrapConfig {
    /// Validate the bootstrap configuration.
    pub fn validate(&self) -> Result<(), BootstrapError> {
        if self.seed_nodes.is_empty() {
            return Err(BootstrapError::NoSeedNodes);
        }
        if self.timeout_ms == 0 {
            return Err(BootstrapError::InvalidConfig(
                "timeout_ms must be > 0".to_string(),
            ));
        }
        if self.max_peers == 0 {
            return Err(BootstrapError::InvalidConfig(
                "max_peers must be > 0".to_string(),
            ));
        }
        for seed in &self.seed_nodes {
            if seed.transports.is_empty() {
                return Err(BootstrapError::InvalidConfig(format!(
                    "Seed node {} has no transports configured",
                    seed.node_id
                )));
            }
        }
        Ok(())
    }
}

/// BootstrapProtocol — Global network discovery and onboarding coordinator.
pub struct BootstrapProtocol {
    /// Configuration.
    config: BootstrapConfig,
    /// Cache of discovered peers.
    discovered_peers: HashSet<String>,
    /// Discovery history for metrics.
    discovery_history: VecDeque<DiscoveryResult>,
    /// Maximum history entries to retain.
    max_history: usize,
    /// Timestamp of the last successful discovery.
    last_discovery: Option<Instant>,
}

impl BootstrapProtocol {
    /// Create a new BootstrapProtocol with default configuration.
    pub fn new() -> Self {
        Self::with_config(BootstrapConfig::default())
    }

    /// Create a new BootstrapProtocol with custom configuration.
    pub fn with_config(config: BootstrapConfig) -> Self {
        Self {
            config,
            discovered_peers: HashSet::new(),
            discovery_history: VecDeque::with_capacity(50),
            max_history: 50,
            last_discovery: None,
        }
    }

    /// Add a seed node to the configuration.
    pub fn add_seed_node(&mut self, seed: SeedNode) {
        self.config.seed_nodes.push(seed);
    }

    /// Remove a seed node by ID.
    pub fn remove_seed_node(&mut self, node_id: u128) -> bool {
        let len_before = self.config.seed_nodes.len();
        self.config.seed_nodes.retain(|s| s.node_id != node_id);
        self.config.seed_nodes.len() < len_before
    }

    /// Get the list of active seed nodes.
    pub fn active_seeds(&self) -> Vec<&SeedNode> {
        self.config
            .seed_nodes
            .iter()
            .filter(|s| s.is_alive(self.config.heartbeat_timeout_secs))
            .collect()
    }

    /// Select the best seed node based on region and transport compatibility.
    pub fn select_best_seed(
        &self,
        preferred_region: Option<&str>,
        preferred_transport: TransportType,
    ) -> Option<&SeedNode> {
        let active = self.active_seeds();
        if active.is_empty() {
            return None;
        }

        // Try to find a seed in the preferred region with the preferred transport
        if let Some(region) = preferred_region {
            if let Some(seed) = active
                .iter()
                .find(|s| s.region == region && s.transports.contains(&preferred_transport))
            {
                return Some(seed);
            }
            // Fallback to region match without transport
            if let Some(seed) = active.iter().find(|s| s.region == region) {
                return Some(seed);
            }
        }

        // Fallback to any active seed with preferred transport
        if let Some(seed) = active
            .iter()
            .find(|s| s.transports.contains(&preferred_transport))
        {
            return Some(seed);
        }

        // Fallback to first active seed
        active.first().copied()
    }

    /// Determine the bootstrap strategy to use.
    fn select_strategy(&self) -> BootstrapStrategy {
        match self.config.strategy {
            BootstrapStrategy::Auto => {
                // Prefer WebRTC if enabled and seeds support it
                if self.config.enable_webrtc
                    && self
                        .config
                        .seed_nodes
                        .iter()
                        .any(|s| s.transports.contains(&TransportType::WebRTC))
                {
                    return BootstrapStrategy::WebRTCStar;
                }
                // Prefer Circuit Relay if enabled
                if self.config.enable_relay {
                    return BootstrapStrategy::CircuitRelay;
                }
                // Fallback to static seeds
                BootstrapStrategy::StaticSeeds
            }
            strategy => strategy,
        }
    }

    /// Run the bootstrap discovery process.
    ///
    /// Attempts to discover peers using the configured strategy and seed nodes.
    /// Returns a DiscoveryResult with the discovered peers.
    pub fn discover(&mut self) -> DiscoveryResult {
        let start = Instant::now();

        // Validate configuration
        if let Err(_e) = self.config.validate() {
            return DiscoveryResult::failed(
                self.select_strategy(),
                start.elapsed().as_millis() as u64,
            );
        }

        let strategy = self.select_strategy();
        let active_seeds = self.active_seeds();

        if active_seeds.is_empty() {
            return DiscoveryResult::failed(strategy, start.elapsed().as_millis() as u64);
        }

        // Simulate discovery based on strategy
        let mut peers = Vec::new();
        let mut seeds_contacted = 0;

        // Collect discovered peers locally first to avoid borrow conflicts
        let mut new_peers = Vec::new();
        for seed in &active_seeds {
            if peers.len() + new_peers.len() >= self.config.max_peers {
                break;
            }
            seeds_contacted += 1;

            // Simulate peer discovery from this seed
            // In production, this would perform actual network discovery
            let discovered = self.simulate_discovery_from_seed(seed, &strategy);
            for peer in discovered {
                if peers.len() + new_peers.len() >= self.config.max_peers {
                    break;
                }
                new_peers.push(peer);
            }
        }

        // Insert collected peers into the discovered set
        for peer in new_peers {
            if self.discovered_peers.insert(peer.clone()) {
                peers.push(peer);
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Check timeout
        if elapsed_ms > self.config.timeout_ms {
            self.last_discovery = Some(start);
            self.record_result(DiscoveryResult::failed(strategy, elapsed_ms));
            return DiscoveryResult::failed(strategy, elapsed_ms);
        }

        let result = DiscoveryResult::success(peers, strategy, elapsed_ms, seeds_contacted);
        self.last_discovery = Some(start);
        self.record_result(result.clone());
        result
    }

    /// Simulate peer discovery from a seed node.
    fn simulate_discovery_from_seed(
        &self,
        seed: &SeedNode,
        strategy: &BootstrapStrategy,
    ) -> Vec<String> {
        // In production, this performs actual network discovery via:
        // - WebRTC-Star signaling
        // - Circuit Relay v2 queries
        // - DNS-SD lookups
        // - Static peer lists from seed nodes
        let mut peers = Vec::new();

        // Generate simulated peers based on seed node properties
        let peer_count = match strategy {
            BootstrapStrategy::WebRTCStar => 5,
            BootstrapStrategy::CircuitRelay => 3,
            BootstrapStrategy::DnsSd => 2,
            BootstrapStrategy::StaticSeeds => 4,
            BootstrapStrategy::Auto => 4,
        };

        for i in 0..peer_count {
            let peer = format!("{}:{}", seed.address, seed.port + 1000 + i as u16);
            peers.push(peer);
        }

        peers
    }

    /// Record a discovery result in history.
    fn record_result(&mut self, result: DiscoveryResult) {
        self.discovery_history.push_back(result);
        if self.discovery_history.len() > self.max_history {
            self.discovery_history.pop_front();
        }
    }

    /// Get the list of all discovered peers.
    pub fn get_discovered_peers(&self) -> Vec<String> {
        self.discovered_peers.iter().cloned().collect()
    }

    /// Clear the discovered peer cache.
    pub fn clear_peers(&mut self) {
        self.discovered_peers.clear();
    }

    /// Get discovery statistics.
    pub fn get_stats(&self) -> BootstrapStats {
        let total_discoveries = self.discovery_history.len();
        let successful: usize = self.discovery_history.iter().filter(|r| r.success).count();
        let avg_time = if total_discoveries > 0 {
            self.discovery_history
                .iter()
                .map(|r| r.discovery_time_ms)
                .sum::<u64>()
                / total_discoveries as u64
        } else {
            0
        };

        BootstrapStats {
            total_discoveries,
            successful_discoveries: successful,
            success_rate: if total_discoveries > 0 {
                successful as f64 / total_discoveries as f64
            } else {
                0.0
            },
            avg_discovery_time_ms: avg_time,
            cached_peers: self.discovered_peers.len(),
            active_seeds: self.active_seeds().len(),
        }
    }

    /// Get the time since the last discovery.
    pub fn time_since_last_discovery(&self) -> Option<Duration> {
        self.last_discovery.map(|t| t.elapsed())
    }

    /// Update the bootstrap configuration.
    pub fn update_config(&mut self, new_config: BootstrapConfig) {
        self.config = new_config;
    }

    /// Reset the bootstrap protocol to initial state.
    pub fn reset(&mut self) {
        self.discovered_peers.clear();
        self.discovery_history.clear();
        self.last_discovery = None;
    }
}

impl Default for BootstrapProtocol {
    fn default() -> Self {
        Self::new()
    }
}

/// Bootstrap statistics for monitoring and metrics.
#[derive(Debug, Clone)]
pub struct BootstrapStats {
    /// Total number of discovery attempts.
    pub total_discoveries: usize,
    /// Number of successful discoveries.
    pub successful_discoveries: usize,
    /// Success rate (0.0 to 1.0).
    pub success_rate: f64,
    /// Average discovery time in milliseconds.
    pub avg_discovery_time_ms: u64,
    /// Number of cached peers.
    pub cached_peers: usize,
    /// Number of active seed nodes.
    pub active_seeds: usize,
}

impl BootstrapStats {
    /// Serialize to JSON for monitoring.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"totalDiscoveries":{},"successfulDiscoveries":{},"successRate":{:.4},"avgDiscoveryTimeMs":{},"cachedPeers":{},"activeSeeds":{}}}"#,
            self.total_discoveries,
            self.successful_discoveries,
            self.success_rate,
            self.avg_discovery_time_ms,
            self.cached_peers,
            self.active_seeds
        )
    }
}

/// Get the current Unix timestamp in seconds.
fn current_unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generate the default seed node list for ed2kIA.
fn default_seed_nodes() -> Vec<SeedNode> {
    vec![
        SeedNode::new(
            0x0001_0000_0000_0000_0000_0000_0000_0001,
            "seed1.ed2kia.network".to_string(),
            9000,
            "us-east".to_string(),
        ),
        SeedNode::new(
            0x0001_0000_0000_0000_0000_0000_0000_0002,
            "seed2.ed2kia.network".to_string(),
            9000,
            "eu-west".to_string(),
        ),
        SeedNode::new(
            0x0001_0000_0000_0000_0000_0000_0000_0003,
            "seed3.ed2kia.network".to_string(),
            9000,
            "ap-southeast".to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── SeedNode Tests ───

    #[test]
    fn test_seed_node_creation() {
        let seed = SeedNode::new(1, "127.0.0.1".to_string(), 9000, "local".to_string());
        assert_eq!(seed.node_id, 1);
        assert_eq!(seed.address, "127.0.0.1");
        assert_eq!(seed.port, 9000);
        assert_eq!(seed.region, "local");
        assert!(seed.active);
    }

    #[test]
    fn test_seed_node_endpoint() {
        let seed = SeedNode::new(1, "127.0.0.1".to_string(), 9000, "local".to_string());
        assert_eq!(seed.endpoint(), "127.0.0.1:9000");
    }

    #[test]
    fn test_seed_node_is_alive() {
        let seed = SeedNode::new(1, "127.0.0.1".to_string(), 9000, "local".to_string());
        assert!(seed.is_alive(30));
    }

    #[test]
    fn test_seed_node_inactive() {
        let mut seed = SeedNode::new(1, "127.0.0.1".to_string(), 9000, "local".to_string());
        seed.active = false;
        assert!(!seed.is_alive(30));
    }

    #[test]
    fn test_seed_node_display() {
        let seed = SeedNode::new(1, "127.0.0.1".to_string(), 9000, "local".to_string());
        let display = format!("{}", seed);
        assert!(display.contains("127.0.0.1:9000"));
        assert!(display.contains("local"));
    }

    // ─── TransportType Tests ───

    #[test]
    fn test_transport_display() {
        assert_eq!(format!("{}", TransportType::WebRTC), "webrtc");
        assert_eq!(format!("{}", TransportType::Tcp), "tcp");
        assert_eq!(format!("{}", TransportType::Quic), "quic");
        assert_eq!(format!("{}", TransportType::WebSocket), "ws");
    }

    // ─── BootstrapStrategy Tests ───

    #[test]
    fn test_strategy_display() {
        assert_eq!(format!("{}", BootstrapStrategy::WebRTCStar), "webrtc-star");
        assert_eq!(
            format!("{}", BootstrapStrategy::CircuitRelay),
            "circuit-relay"
        );
        assert_eq!(format!("{}", BootstrapStrategy::DnsSd), "dns-sd");
        assert_eq!(
            format!("{}", BootstrapStrategy::StaticSeeds),
            "static-seeds"
        );
        assert_eq!(format!("{}", BootstrapStrategy::Auto), "auto");
    }

    // ─── BootstrapConfig Tests ───

    #[test]
    fn test_config_default() {
        let config = BootstrapConfig::default();
        assert!(!config.seed_nodes.is_empty());
        assert_eq!(config.strategy, BootstrapStrategy::Auto);
        assert_eq!(config.timeout_ms, 3000);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = BootstrapConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_no_seeds() {
        let config = BootstrapConfig {
            seed_nodes: vec![],
            ..BootstrapConfig::default()
        };
        match config.validate() {
            Err(BootstrapError::NoSeedNodes) => {}
            other => panic!("Expected NoSeedNodes, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_zero_timeout() {
        let mut config = BootstrapConfig::default();
        config.timeout_ms = 0;
        match config.validate() {
            Err(BootstrapError::InvalidConfig(_)) => {}
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_zero_max_peers() {
        let mut config = BootstrapConfig::default();
        config.max_peers = 0;
        match config.validate() {
            Err(BootstrapError::InvalidConfig(_)) => {}
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    // ─── BootstrapProtocol Tests ───

    #[test]
    fn test_protocol_creation() {
        let protocol = BootstrapProtocol::new();
        assert!(!protocol.config.seed_nodes.is_empty());
    }

    #[test]
    fn test_protocol_custom_config() {
        let config = BootstrapConfig {
            seed_nodes: vec![SeedNode::new(
                1,
                "test.local".to_string(),
                8000,
                "test".to_string(),
            )],
            strategy: BootstrapStrategy::StaticSeeds,
            ..BootstrapConfig::default()
        };
        let protocol = BootstrapProtocol::with_config(config);
        assert_eq!(protocol.config.seed_nodes.len(), 1);
        assert_eq!(protocol.config.strategy, BootstrapStrategy::StaticSeeds);
    }

    #[test]
    fn test_add_seed_node() {
        let mut protocol = BootstrapProtocol::new();
        let initial_count = protocol.config.seed_nodes.len();
        protocol.add_seed_node(SeedNode::new(
            999,
            "new.local".to_string(),
            7000,
            "new".to_string(),
        ));
        assert_eq!(protocol.config.seed_nodes.len(), initial_count + 1);
    }

    #[test]
    fn test_remove_seed_node() {
        let mut protocol = BootstrapProtocol::new();
        let target_id = protocol.config.seed_nodes[0].node_id;
        assert!(protocol.remove_seed_node(target_id));
        assert!(!protocol
            .config
            .seed_nodes
            .iter()
            .any(|s| s.node_id == target_id));
    }

    #[test]
    fn test_remove_nonexistent_seed() {
        let mut protocol = BootstrapProtocol::new();
        assert!(!protocol.remove_seed_node(u128::MAX));
    }

    #[test]
    fn test_active_seeds() {
        let protocol = BootstrapProtocol::new();
        let active = protocol.active_seeds();
        assert!(!active.is_empty());
    }

    #[test]
    fn test_select_best_seed() {
        let mut protocol = BootstrapProtocol::new();
        protocol.add_seed_node(SeedNode::new(
            100,
            "us.local".to_string(),
            8000,
            "us-east".to_string(),
        ));

        let seed = protocol.select_best_seed(Some("us-east"), TransportType::WebRTC);
        assert!(seed.is_some());
        assert_eq!(seed.unwrap().region, "us-east");
    }

    #[test]
    fn test_select_best_seed_no_match() {
        let protocol = BootstrapProtocol::new();
        let seed = protocol.select_best_seed(Some("nonexistent"), TransportType::Quic);
        // Should fallback to any active seed
        assert!(seed.is_some());
    }

    #[test]
    fn test_discover() {
        let mut protocol = BootstrapProtocol::new();
        let result = protocol.discover();
        assert!(result.success);
        assert!(!result.peers.is_empty());
        assert!(result.discovery_time_ms <= protocol.config.timeout_ms);
    }

    #[test]
    fn test_discover_peers_cached() {
        let mut protocol = BootstrapProtocol::new();
        let _ = protocol.discover();
        let peers = protocol.get_discovered_peers();
        assert!(!peers.is_empty());
    }

    #[test]
    fn test_clear_peers() {
        let mut protocol = BootstrapProtocol::new();
        let _ = protocol.discover();
        protocol.clear_peers();
        assert!(protocol.get_discovered_peers().is_empty());
    }

    #[test]
    fn test_get_stats() {
        let mut protocol = BootstrapProtocol::new();
        let _ = protocol.discover();
        let stats = protocol.get_stats();
        assert!(stats.total_discoveries > 0);
        assert!(stats.successful_discoveries > 0);
        assert!(stats.success_rate > 0.0);
    }

    #[test]
    fn test_stats_to_json() {
        let stats = BootstrapStats {
            total_discoveries: 10,
            successful_discoveries: 8,
            success_rate: 0.8,
            avg_discovery_time_ms: 150,
            cached_peers: 20,
            active_seeds: 3,
        };
        let json = stats.to_json();
        assert!(json.contains("\"totalDiscoveries\":10"));
        assert!(json.contains("\"successRate\":0.8000"));
    }

    #[test]
    fn test_reset() {
        let mut protocol = BootstrapProtocol::new();
        let _ = protocol.discover();
        protocol.reset();
        assert!(protocol.get_discovered_peers().is_empty());
        assert_eq!(protocol.get_stats().total_discoveries, 0);
    }

    #[test]
    fn test_update_config() {
        let mut protocol = BootstrapProtocol::new();
        let new_config = BootstrapConfig {
            strategy: BootstrapStrategy::WebRTCStar,
            ..BootstrapConfig::default()
        };
        protocol.update_config(new_config);
        assert_eq!(protocol.config.strategy, BootstrapStrategy::WebRTCStar);
    }

    #[test]
    fn test_time_since_last_discovery() {
        let mut protocol = BootstrapProtocol::new();
        assert!(protocol.time_since_last_discovery().is_none());
        let _ = protocol.discover();
        assert!(protocol.time_since_last_discovery().is_some());
    }

    #[test]
    fn test_default_seed_nodes() {
        let seeds = default_seed_nodes();
        assert_eq!(seeds.len(), 3);
        assert!(seeds.iter().any(|s| s.region == "us-east"));
        assert!(seeds.iter().any(|s| s.region == "eu-west"));
        assert!(seeds.iter().any(|s| s.region == "ap-southeast"));
    }

    #[test]
    fn test_bootstrap_error_display() {
        assert_eq!(
            format!("{}", BootstrapError::NoSeedNodes),
            "No seed nodes configured"
        );
        assert_eq!(
            format!("{}", BootstrapError::Timeout),
            "Discovery timeout exceeded"
        );
        assert_eq!(
            format!("{}", BootstrapError::NetworkUnreachable),
            "Network unreachable"
        );
    }

    #[test]
    fn test_discovery_result_success() {
        let result = DiscoveryResult::success(
            vec!["peer1:9001".to_string()],
            BootstrapStrategy::WebRTCStar,
            100,
            1,
        );
        assert!(result.success);
        assert_eq!(result.peers.len(), 1);
    }

    #[test]
    fn test_discovery_result_failed() {
        let result = DiscoveryResult::failed(BootstrapStrategy::WebRTCStar, 5000);
        assert!(!result.success);
        assert!(result.peers.is_empty());
    }

    #[test]
    fn test_protocol_default() {
        let protocol = BootstrapProtocol::default();
        assert!(!protocol.config.seed_nodes.is_empty());
    }
}
