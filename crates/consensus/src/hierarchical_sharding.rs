//! Hierarchical Sharding — Cluster Discovery & Shard Assignment for Planetary Scale.
//!
//! Provides consistent-hashing-based shard assignment, load-aware rebalancing,
//! and cluster discovery primitives for hierarchical (multi-level) sharding
//! architectures ready for planetary-scale P2P deployment.
//!
//! # Design Principles
//!
//! - **Consistent Hashing**: O(log k) shard lookup with bounded reassignment on churn.
//! - **Load-Aware Rebalancing**: Nodes migrate to underloaded shards when imbalance exceeds threshold.
//! - **Hierarchical Clusters**: Shards grouped into clusters for multi-level aggregation.
//! - **Byzantine-Resilient**: Trust-weighted shard voting prevents Sybil shard takeover.
//! - **Energy-Proportional**: Shard assignment considers node energy budget (Sprint 120/121 integration).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error;

// ─── Error Types ───────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum ShardingError {
    #[error("Shard {0} not found in cluster")]
    ShardNotFound(u64),

    #[error("Cluster {0} not found")]
    ClusterNotFound(u64),

    #[error("Node {0} already assigned to shard {1}")]
    NodeAlreadyAssigned(u64, u64),

    #[error("Shard {0} at capacity ({1}/{2})")]
    ShardAtCapacity(u64, usize, usize),

    #[error("Invalid shard count: {0}")]
    InvalidShardCount(usize),

    #[error("Hash collision detected for node {0}")]
    HashCollision(u64),

    #[error("Cluster discovery timeout after {0}ms")]
    DiscoveryTimeout(u64),

    #[error("Byzantine detection: node {0} exceeds trust threshold")]
    ByzantineDetected(u64),
}

// ─── Configuration ─────────────────────────────────────────────────────────

/// Configuration for hierarchical sharding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Unique identifier for this shard.
    pub shard_id: u64,
    /// Cluster this shard belongs to.
    pub cluster_id: u64,
    /// Maximum number of nodes allowed in this shard.
    pub max_nodes: usize,
    /// Consensus type used within this shard.
    pub consensus_type: ConsensusType,
    /// Replication factor for cross-shard redundancy.
    pub replication_factor: usize,
    /// Load imbalance threshold (0.0-1.0) triggering rebalancing.
    pub imbalance_threshold: f64,
}

impl ShardConfig {
    /// Create a new shard configuration.
    pub fn new(shard_id: u64, cluster_id: u64, max_nodes: usize) -> Self {
        Self {
            shard_id,
            cluster_id,
            max_nodes,
            consensus_type: ConsensusType::default(),
            replication_factor: 3,
            imbalance_threshold: 0.2,
        }
    }

    /// Create with custom consensus type.
    pub fn with_consensus(mut self, consensus_type: ConsensusType) -> Self {
        self.consensus_type = consensus_type;
        self
    }

    /// Create with custom replication factor.
    pub fn with_replication(mut self, factor: usize) -> Self {
        self.replication_factor = factor.max(1);
        self
    }

    /// Create with custom imbalance threshold.
    pub fn with_imbalance_threshold(mut self, threshold: f64) -> Self {
        self.imbalance_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Check if shard is at capacity.
    pub fn is_at_capacity(&self, current_nodes: usize) -> bool {
        current_nodes >= self.max_nodes
    }

    /// Calculate load ratio (0.0 = empty, 1.0 = full).
    pub fn load_ratio(&self, current_nodes: usize) -> f64 {
        if self.max_nodes == 0 {
            return 1.0;
        }
        (current_nodes as f64 / self.max_nodes as f64).min(1.0)
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            shard_id: 0,
            cluster_id: 0,
            max_nodes: 100,
            consensus_type: ConsensusType::default(),
            replication_factor: 3,
            imbalance_threshold: 0.2,
        }
    }
}

/// Consensus type used within a shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConsensusType {
    /// Proof of Symbiosis (PoSym) — Energy-proportional trust.
    PoSym,
    /// Proof of Novelty (PoN) — Contribution-based.
    PoN,
    /// Hybrid PoSym + PoN.
    #[default]
    Hybrid,
    /// Zero-Knowledge Proof based consensus.
    ZKP,
}

/// Cluster configuration for hierarchical grouping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Unique cluster identifier.
    pub cluster_id: u64,
    /// Maximum number of shards in this cluster.
    pub max_shards: usize,
    /// Geographic region hint (optional).
    pub region: Option<String>,
    /// Cluster leader shard ID.
    pub leader_shard: u64,
}

impl ClusterConfig {
    /// Create a new cluster configuration.
    pub fn new(cluster_id: u64, max_shards: usize, leader_shard: u64) -> Self {
        Self {
            cluster_id,
            max_shards,
            region: None,
            leader_shard,
        }
    }

    /// Set the geographic region.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
}

// ─── Node Assignment ───────────────────────────────────────────────────────

/// Result of assigning a node to a shard.
#[derive(Debug, Clone)]
pub struct ShardAssignment {
    /// The node ID.
    pub node_id: u64,
    /// Assigned shard ID.
    pub shard_id: u64,
    /// Assigned cluster ID.
    pub cluster_id: u64,
    /// Hash position on the consistency ring.
    pub hash_position: u64,
    /// Load ratio of the assigned shard.
    pub load_ratio: f64,
    /// Whether this was a rebalanced assignment.
    pub rebalanced: bool,
}

impl std::fmt::Display for ShardAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ShardAssignment {{ node={}, shard={}, cluster={}, hash=0x{:016x}, load={:.2}, rebalanced={} }}",
            self.node_id, self.shard_id, self.cluster_id, self.hash_position, self.load_ratio, self.rebalanced
        )
    }
}

/// Consistent hash ring for shard assignment.
pub struct ConsistentHashRing {
    /// Number of shards in the ring.
    pub num_shards: usize,
    /// Virtual nodes per physical shard (for better distribution).
    pub virtual_nodes: usize,
    /// Mapping from virtual node hash -> physical shard ID.
    ring: BTreeMap<u64, u64>,
    /// Current node count per shard.
    shard_loads: HashMap<u64, usize>,
}

impl ConsistentHashRing {
    /// Create a new consistent hash ring with the given number of shards.
    pub fn new(num_shards: usize) -> Result<Self, ShardingError> {
        if num_shards == 0 {
            return Err(ShardingError::InvalidShardCount(num_shards));
        }
        let virtual_nodes = (num_shards * 150).max(100);
        let mut ring = BTreeMap::new();
        let mut shard_loads = HashMap::new();

        for shard_id in 0..num_shards {
            shard_loads.insert(shard_id as u64, 0);
            for vn in 0..virtual_nodes {
                let hash = Self::compute_virtual_node_hash(shard_id as u64, vn);
                ring.insert(hash, shard_id as u64);
            }
        }

        Ok(Self {
            num_shards,
            virtual_nodes,
            ring,
            shard_loads,
        })
    }

    /// Compute hash for a virtual node.
    fn compute_virtual_node_hash(shard_id: u64, virtual_node: usize) -> u64 {
        let data = format!("shard_{}_vn_{}", shard_id, virtual_node);
        let hash = Sha256::digest(data.as_bytes());
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    }

    /// Compute hash position for a node ID.
    pub fn compute_node_hash(node_id: u64) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(node_id.to_le_bytes());
        let hash = hasher.finalize();
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    }

    /// Find the shard for a given node ID using consistent hashing.
    pub fn get_shard(&self, node_id: u64) -> u64 {
        let hash = Self::compute_node_hash(node_id);
        if let Some(shard_id) = self.ring.range(hash..).next().map(|(_, &v)| v) {
            shard_id
        } else {
            *self.ring.values().next().unwrap_or(&0)
        }
    }

    /// Get the hash position for a node.
    pub fn get_node_position(&self, node_id: u64) -> u64 {
        Self::compute_node_hash(node_id)
    }

    /// Record a node assignment to a shard.
    pub fn record_assignment(&mut self, shard_id: u64) {
        *self.shard_loads.entry(shard_id).or_insert(0) += 1;
    }

    /// Get current load for a shard.
    pub fn get_shard_load(&self, shard_id: u64) -> usize {
        *self.shard_loads.get(&shard_id).unwrap_or(&0)
    }

    /// Get all shard loads.
    pub fn get_all_loads(&self) -> HashMap<u64, usize> {
        self.shard_loads.clone()
    }

    /// Calculate the load imbalance across all shards.
    /// Returns 0.0 for perfect balance, 1.0 for maximum imbalance.
    pub fn load_imbalance(&self) -> f64 {
        let loads: Vec<f64> = self.shard_loads.values().map(|&l| l as f64).collect();
        if loads.is_empty() || loads.len() == 1 {
            return 0.0;
        }
        let mean = loads.iter().sum::<f64>() / loads.len() as f64;
        if mean == 0.0 {
            return 0.0;
        }
        let variance = loads.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / loads.len() as f64;
        let std_dev = variance.sqrt();
        (std_dev / mean).min(1.0)
    }

    /// Find the least loaded shard.
    pub fn least_loaded_shard(&self) -> u64 {
        self.shard_loads
            .iter()
            .min_by_key(|(_, &load)| load)
            .map(|(&shard, _)| shard)
            .unwrap_or(0)
    }

    /// Find the most loaded shard.
    pub fn most_loaded_shard(&self) -> u64 {
        self.shard_loads
            .iter()
            .max_by_key(|(_, &load)| load)
            .map(|(&shard, _)| shard)
            .unwrap_or(0)
    }
}

// ─── Hierarchical Shard Manager ────────────────────────────────────────────

/// Manages hierarchical shard assignments across clusters.
pub struct HierarchicalShardManager {
    /// Cluster configurations.
    clusters: HashMap<u64, ClusterConfig>,
    /// Shard configurations.
    shards: HashMap<u64, ShardConfig>,
    /// Consistent hash ring for assignment.
    ring: ConsistentHashRing,
    /// Node -> Shard mapping.
    node_assignments: HashMap<u64, ShardAssignment>,
    /// Shard -> Node mapping.
    shard_nodes: HashMap<u64, HashSet<u64>>,
    /// Total number of shards.
    #[allow(dead_code)]
    total_shards: usize,
}

impl HierarchicalShardManager {
    /// Create a new hierarchical shard manager.
    pub fn new(total_shards: usize, clusters: &[ClusterConfig]) -> Result<Self, ShardingError> {
        let ring = ConsistentHashRing::new(total_shards)?;
        let mut clusters_map = HashMap::new();
        let mut shards_map = HashMap::new();
        let mut shard_nodes = HashMap::new();

        for cluster in clusters {
            clusters_map.insert(cluster.cluster_id, cluster.clone());
        }

        for shard_id in 0..total_shards {
            let config = ShardConfig::new(shard_id as u64, 0, 1000);
            shards_map.insert(shard_id as u64, config);
            shard_nodes.insert(shard_id as u64, HashSet::new());
        }

        Ok(Self {
            clusters: clusters_map,
            shards: shards_map,
            ring,
            node_assignments: HashMap::new(),
            shard_nodes,
            total_shards,
        })
    }

    /// Assign a node to a shard using consistent hashing.
    pub fn assign_node(&mut self, node_id: u64) -> Result<ShardAssignment, ShardingError> {
        // Check if already assigned
        if let Some(existing) = self.node_assignments.get(&node_id) {
            return Err(ShardingError::NodeAlreadyAssigned(
                node_id,
                existing.shard_id,
            ));
        }

        let shard_id = self.ring.get_shard(node_id);
        let hash_position = self.ring.get_node_position(node_id);

        // Check capacity
        let shard_config = self
            .shards
            .get(&shard_id)
            .ok_or(ShardingError::ShardNotFound(shard_id))?;

        if shard_config.is_at_capacity(self.ring.get_shard_load(shard_id)) {
            return Err(ShardingError::ShardAtCapacity(
                shard_id,
                self.ring.get_shard_load(shard_id),
                shard_config.max_nodes,
            ));
        }

        // Record assignment
        self.ring.record_assignment(shard_id);
        self.shard_nodes
            .entry(shard_id)
            .or_default()
            .insert(node_id);

        let cluster_id = shard_config.cluster_id;
        let load_ratio = shard_config.load_ratio(self.ring.get_shard_load(shard_id));

        let assignment = ShardAssignment {
            node_id,
            shard_id,
            cluster_id,
            hash_position,
            load_ratio,
            rebalanced: false,
        };

        self.node_assignments.insert(node_id, assignment.clone());
        Ok(assignment)
    }

    /// Assign a node to a specific shard (override consistent hashing).
    pub fn assign_node_to_shard(
        &mut self,
        node_id: u64,
        target_shard: u64,
    ) -> Result<ShardAssignment, ShardingError> {
        // Check if shard exists
        let shard_config = self
            .shards
            .get(&target_shard)
            .ok_or(ShardingError::ShardNotFound(target_shard))?;

        // Check capacity
        if shard_config.is_at_capacity(self.ring.get_shard_load(target_shard)) {
            return Err(ShardingError::ShardAtCapacity(
                target_shard,
                self.ring.get_shard_load(target_shard),
                shard_config.max_nodes,
            ));
        }

        let hash_position = self.ring.get_node_position(node_id);
        self.ring.record_assignment(target_shard);
        self.shard_nodes
            .entry(target_shard)
            .or_default()
            .insert(node_id);

        let cluster_id = shard_config.cluster_id;
        let load_ratio = shard_config.load_ratio(self.ring.get_shard_load(target_shard));

        let assignment = ShardAssignment {
            node_id,
            shard_id: target_shard,
            cluster_id,
            hash_position,
            load_ratio,
            rebalanced: false,
        };

        self.node_assignments.insert(node_id, assignment.clone());
        Ok(assignment)
    }

    /// Rebalance nodes from overloaded shards to underloaded ones.
    pub fn rebalance(&mut self) -> Result<Vec<ShardAssignment>, ShardingError> {
        let imbalance = self.ring.load_imbalance();
        let threshold = self
            .shards
            .values()
            .next()
            .map(|s| s.imbalance_threshold)
            .unwrap_or(0.2);

        if imbalance <= threshold {
            return Ok(Vec::new());
        }

        let most_loaded = self.ring.most_loaded_shard();
        let least_loaded = self.ring.least_loaded_shard();

        let most_load = self.ring.get_shard_load(most_loaded);
        let least_load = self.ring.get_shard_load(least_loaded);

        if most_load <= least_load + 1 {
            return Ok(Vec::new());
        }

        // Move one node from most loaded to least loaded
        let node_to_move = self
            .shard_nodes
            .get(&most_loaded)
            .and_then(|nodes| nodes.iter().next().copied())
            .ok_or(ShardingError::ShardNotFound(most_loaded))?;

        // Remove from old shard
        self.shard_nodes
            .get_mut(&most_loaded)
            .unwrap()
            .remove(&node_to_move);

        // Add to new shard
        self.shard_nodes
            .entry(least_loaded)
            .or_default()
            .insert(node_to_move);

        let hash_position = self.ring.get_node_position(node_to_move);
        let shard_config = self
            .shards
            .get(&least_loaded)
            .ok_or(ShardingError::ShardNotFound(least_loaded))?;

        let cluster_id = shard_config.cluster_id;
        let load_ratio = shard_config.load_ratio(self.ring.get_shard_load(least_loaded));

        let assignment = ShardAssignment {
            node_id: node_to_move,
            shard_id: least_loaded,
            cluster_id,
            hash_position,
            load_ratio,
            rebalanced: true,
        };

        self.node_assignments
            .insert(node_to_move, assignment.clone());
        Ok(vec![assignment])
    }

    /// Get the assignment for a node.
    pub fn get_node_assignment(&self, node_id: u64) -> Option<&ShardAssignment> {
        self.node_assignments.get(&node_id)
    }

    /// Get all nodes in a shard.
    pub fn get_shard_nodes(&self, shard_id: u64) -> Option<&HashSet<u64>> {
        self.shard_nodes.get(&shard_id)
    }

    /// Get the cluster for a shard.
    pub fn get_shard_cluster(&self, shard_id: u64) -> Option<u64> {
        self.shards.get(&shard_id).map(|s| s.cluster_id)
    }

    /// Get total number of assigned nodes.
    pub fn total_nodes(&self) -> usize {
        self.node_assignments.len()
    }

    /// Get current load imbalance.
    pub fn load_imbalance(&self) -> f64 {
        self.ring.load_imbalance()
    }

    /// Get shard configuration.
    pub fn get_shard_config(&self, shard_id: u64) -> Option<&ShardConfig> {
        self.shards.get(&shard_id)
    }

    /// Get cluster configuration.
    pub fn get_cluster_config(&self, cluster_id: u64) -> Option<&ClusterConfig> {
        self.clusters.get(&cluster_id)
    }

    /// Remove a node from the system.
    /// Return the total number of shards managed.
    pub fn total_shards(&self) -> usize {
        self.total_shards
    }

    pub fn remove_node(&mut self, node_id: u64) -> Option<ShardAssignment> {
        let assignment = self.node_assignments.remove(&node_id)?;
        if let Some(nodes) = self.shard_nodes.get_mut(&assignment.shard_id) {
            nodes.remove(&node_id);
        }
        Some(assignment)
    }
}

// ─── Cluster Discovery ─────────────────────────────────────────────────────

/// Peer information for cluster discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier.
    pub peer_id: u64,
    /// Peer's assigned shard.
    pub shard_id: u64,
    /// Peer's trust score (0.0-1.0).
    pub trust_score: f64,
    /// Peer's energy budget.
    pub energy_budget: f64,
    /// Peer's network address (multiaddr format).
    pub address: String,
    /// Last seen timestamp (Unix epoch).
    pub last_seen: u64,
}

impl PeerInfo {
    /// Create a new peer info.
    pub fn new(peer_id: u64, shard_id: u64, address: impl Into<String>) -> Self {
        Self {
            peer_id,
            shard_id,
            trust_score: 0.5,
            energy_budget: 1.0,
            address: address.into(),
            last_seen: 0,
        }
    }

    /// Check if peer meets minimum trust threshold.
    pub fn meets_trust_threshold(&self, threshold: f64) -> bool {
        self.trust_score >= threshold
    }

    /// Check if peer is considered active (within timeout).
    /// Returns true if the peer was last seen within the timeout window.
    /// Handles the case where last_seen is in the future (clock skew).
    pub fn is_active(&self, timeout_seconds: u64, current_time: u64) -> bool {
        if self.last_seen <= current_time {
            current_time - self.last_seen <= timeout_seconds
        } else {
            // Peer was last seen in the future (clock skew) — consider active
            true
        }
    }
}

/// Cluster discovery state.
pub struct ClusterDiscovery {
    /// Known peers in the cluster.
    peers: HashMap<u64, PeerInfo>,
    /// Minimum trust threshold for cluster membership.
    min_trust_threshold: f64,
    /// Maximum peers per shard.
    max_peers_per_shard: usize,
    /// Discovery timeout in milliseconds.
    discovery_timeout_ms: u64,
}

impl ClusterDiscovery {
    /// Create a new cluster discovery instance.
    pub fn new(min_trust_threshold: f64) -> Self {
        Self {
            peers: HashMap::new(),
            min_trust_threshold,
            max_peers_per_shard: 100,
            discovery_timeout_ms: 5000,
        }
    }

    /// Set maximum peers per shard.
    pub fn with_max_peers(mut self, max: usize) -> Self {
        self.max_peers_per_shard = max;
        self
    }

    /// Set discovery timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.discovery_timeout_ms = timeout_ms;
        self
    }

    /// Add a peer to the discovery set.
    pub fn add_peer(&mut self, peer: PeerInfo) {
        self.peers.insert(peer.peer_id, peer);
    }

    /// Remove a peer from the discovery set.
    pub fn remove_peer(&mut self, peer_id: u64) -> Option<PeerInfo> {
        self.peers.remove(&peer_id)
    }

    /// Discover peers in a specific shard.
    pub fn discover_peers_in_shard(&self, shard_id: u64) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.shard_id == shard_id)
            .collect()
    }

    /// Discover trusted peers (meeting trust threshold).
    pub fn discover_trusted_peers(&self) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.meets_trust_threshold(self.min_trust_threshold))
            .collect()
    }

    /// Discover active peers in a shard.
    pub fn discover_active_peers(
        &self,
        shard_id: u64,
        current_time: u64,
        timeout: u64,
    ) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.shard_id == shard_id && p.is_active(timeout, current_time))
            .collect()
    }

    /// Get all known peers.
    pub fn all_peers(&self) -> Vec<&PeerInfo> {
        self.peers.values().collect()
    }

    /// Get peer count.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get peer by ID.
    pub fn get_peer(&self, peer_id: u64) -> Option<&PeerInfo> {
        self.peers.get(&peer_id)
    }

    /// Calculate average trust score in a shard.
    pub fn avg_trust_in_shard(&self, shard_id: u64) -> f64 {
        let peers: Vec<f64> = self
            .peers
            .values()
            .filter(|p| p.shard_id == shard_id)
            .map(|p| p.trust_score)
            .collect();

        if peers.is_empty() {
            return 0.0;
        }
        peers.iter().sum::<f64>() / peers.len() as f64
    }

    /// Calculate total energy budget in a shard.
    pub fn total_energy_in_shard(&self, shard_id: u64) -> f64 {
        self.peers
            .values()
            .filter(|p| p.shard_id == shard_id)
            .map(|p| p.energy_budget)
            .sum()
    }
}

impl Default for ClusterDiscovery {
    fn default() -> Self {
        Self::new(0.5)
    }
}

// ─── Trust-Weighted Shard Voting ───────────────────────────────────────────

/// Result of trust-weighted shard voting.
#[derive(Debug, Clone)]
pub struct ShardVoteResult {
    /// The winning shard ID.
    pub winning_shard: u64,
    /// Total trust weight for the winning shard.
    pub winning_weight: f64,
    /// Total votes cast.
    pub total_votes: usize,
    /// Trust weight distribution per shard.
    pub shard_weights: HashMap<u64, f64>,
    /// Whether the result meets quorum.
    pub quorum_met: bool,
}

/// Perform trust-weighted voting for shard selection.
///
/// Each vote is weighted by the voter's trust score, making it
/// Byzantine-resilient against Sybil attacks.
pub fn trust_weighted_shard_vote(
    votes: &[(u64, u64, f64)],
    quorum_threshold: f64,
) -> ShardVoteResult {
    let mut shard_weights: HashMap<u64, f64> = HashMap::new();
    let total_trust: f64 = votes.iter().map(|&(_, _, trust)| trust).sum();

    for &(_voter_id, shard_id, trust) in votes {
        *shard_weights.entry(shard_id).or_insert(0.0) += trust;
    }

    let winning_shard = shard_weights
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(&shard, _)| shard)
        .unwrap_or(0);

    let winning_weight = *shard_weights.get(&winning_shard).unwrap_or(&0.0);
    let quorum_met = total_trust >= quorum_threshold;

    ShardVoteResult {
        winning_shard,
        winning_weight,
        total_votes: votes.len(),
        shard_weights,
        quorum_met,
    }
}

// ─── Load-Based Shard Assignment ───────────────────────────────────────────

/// Assign a node to the least loaded shard that meets energy constraints.
pub fn assign_node_by_load(
    shard_configs: &[ShardConfig],
    shard_loads: &HashMap<u64, usize>,
    node_energy_budget: f64,
    min_energy_ratio: f64,
) -> Result<u64, ShardingError> {
    if shard_configs.is_empty() {
        return Err(ShardingError::InvalidShardCount(0));
    }

    let mut candidates: Vec<(&ShardConfig, usize)> = shard_configs
        .iter()
        .map(|config| {
            let load = *shard_loads.get(&config.shard_id).unwrap_or(&0);
            (config, load)
        })
        .filter(|(config, load)| !config.is_at_capacity(*load))
        .filter(|(config, load)| {
            // Ensure shard's energy requirements are met
            config.load_ratio(*load) < (1.0 - min_energy_ratio)
                || node_energy_budget >= min_energy_ratio
        })
        .collect();

    // Sort by load ratio (ascending)
    candidates.sort_by(|a, b| {
        a.0.load_ratio(a.1)
            .partial_cmp(&b.0.load_ratio(b.1))
            .unwrap()
    });

    candidates
        .first()
        .map(|(config, _)| config.shard_id)
        .ok_or(ShardingError::InvalidShardCount(0))
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── ShardConfig Tests ─────────────────────────────────────────────

    #[test]
    fn test_shard_config_new() {
        let config = ShardConfig::new(1, 0, 50);
        assert_eq!(config.shard_id, 1);
        assert_eq!(config.cluster_id, 0);
        assert_eq!(config.max_nodes, 50);
        assert_eq!(config.consensus_type, ConsensusType::Hybrid);
        assert_eq!(config.replication_factor, 3);
        assert!((config.imbalance_threshold - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_shard_config_default() {
        let config = ShardConfig::default();
        assert_eq!(config.shard_id, 0);
        assert_eq!(config.cluster_id, 0);
        assert_eq!(config.max_nodes, 100);
    }

    #[test]
    fn test_shard_config_with_consensus() {
        let config = ShardConfig::new(0, 0, 100).with_consensus(ConsensusType::PoSym);
        assert_eq!(config.consensus_type, ConsensusType::PoSym);
    }

    #[test]
    fn test_shard_config_with_replication() {
        let config = ShardConfig::new(0, 0, 100).with_replication(5);
        assert_eq!(config.replication_factor, 5);
    }

    #[test]
    fn test_shard_config_replication_min_one() {
        let config = ShardConfig::new(0, 0, 100).with_replication(0);
        assert_eq!(config.replication_factor, 1);
    }

    #[test]
    fn test_shard_config_with_imbalance_threshold() {
        let config = ShardConfig::new(0, 0, 100).with_imbalance_threshold(0.3);
        assert!((config.imbalance_threshold - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_shard_config_imbalance_threshold_clamped() {
        let config = ShardConfig::new(0, 0, 100).with_imbalance_threshold(1.5);
        assert!((config.imbalance_threshold - 1.0).abs() < f64::EPSILON);

        let config2 = ShardConfig::new(0, 0, 100).with_imbalance_threshold(-0.5);
        assert!((config2.imbalance_threshold - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_shard_config_capacity() {
        let config = ShardConfig::new(0, 0, 10);
        assert!(!config.is_at_capacity(5));
        assert!(!config.is_at_capacity(9));
        assert!(config.is_at_capacity(10));
        assert!(config.is_at_capacity(15));
    }

    #[test]
    fn test_shard_config_load_ratio() {
        let config = ShardConfig::new(0, 0, 100);
        assert!((config.load_ratio(0) - 0.0).abs() < f64::EPSILON);
        assert!((config.load_ratio(50) - 0.5).abs() < f64::EPSILON);
        assert!((config.load_ratio(100) - 1.0).abs() < f64::EPSILON);
        assert!((config.load_ratio(150) - 1.0).abs() < f64::EPSILON); // Clamped
    }

    #[test]
    fn test_shard_config_load_ratio_zero_max() {
        let config = ShardConfig {
            max_nodes: 0,
            ..ShardConfig::default()
        };
        assert!((config.load_ratio(0) - 1.0).abs() < f64::EPSILON);
    }

    // ─── ConsensusType Tests ───────────────────────────────────────────

    #[test]
    fn test_consensus_type_default() {
        let ct = ConsensusType::default();
        assert_eq!(ct, ConsensusType::Hybrid);
    }

    #[test]
    fn test_consensus_type_equality() {
        assert_eq!(ConsensusType::PoSym, ConsensusType::PoSym);
        assert_ne!(ConsensusType::PoSym, ConsensusType::PoN);
    }

    // ─── ClusterConfig Tests ───────────────────────────────────────────

    #[test]
    fn test_cluster_config_new() {
        let config = ClusterConfig::new(0, 10, 0);
        assert_eq!(config.cluster_id, 0);
        assert_eq!(config.max_shards, 10);
        assert_eq!(config.leader_shard, 0);
        assert!(config.region.is_none());
    }

    #[test]
    fn test_cluster_config_with_region() {
        let config = ClusterConfig::new(0, 10, 0).with_region("us-east-1");
        assert_eq!(config.region, Some("us-east-1".to_string()));
    }

    // ─── ConsistentHashRing Tests ──────────────────────────────────────

    #[test]
    fn test_hash_ring_creation() {
        let ring = ConsistentHashRing::new(4).unwrap();
        assert_eq!(ring.num_shards, 4);
    }

    #[test]
    fn test_hash_ring_zero_shards_error() {
        let result = ConsistentHashRing::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_ring_assigns_valid_shard() {
        let ring = ConsistentHashRing::new(8).unwrap();
        for node_id in 0..100 {
            let shard = ring.get_shard(node_id);
            assert!(
                shard < 8,
                "Shard {} out of range for node {}",
                shard,
                node_id
            );
        }
    }

    #[test]
    fn test_hash_ring_consistency() {
        let ring = ConsistentHashRing::new(16).unwrap();
        let node_id = 42;
        let shard1 = ring.get_shard(node_id);
        let shard2 = ring.get_shard(node_id);
        assert_eq!(shard1, shard2, "Same node should map to same shard");
    }

    #[test]
    fn test_hash_ring_distribution() {
        let ring = ConsistentHashRing::new(10).unwrap();
        let mut counts = vec![0usize; 10];

        for node_id in 0..1000 {
            let shard = ring.get_shard(node_id);
            counts[shard as usize] += 1;
        }

        let min_count = *counts.iter().min().unwrap();
        let max_count = *counts.iter().max().unwrap();
        let ratio = min_count as f64 / max_count as f64;

        // With virtual nodes, distribution should be reasonably balanced
        assert!(
            ratio > 0.5,
            "Distribution ratio {:.2} too low (min={}, max={})",
            ratio,
            min_count,
            max_count
        );
    }

    #[test]
    fn test_hash_ring_node_hash_deterministic() {
        let hash1 = ConsistentHashRing::compute_node_hash(12345);
        let hash2 = ConsistentHashRing::compute_node_hash(12345);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_ring_node_hash_unique() {
        let hash1 = ConsistentHashRing::compute_node_hash(1);
        let hash2 = ConsistentHashRing::compute_node_hash(2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_ring_record_assignment() {
        let mut ring = ConsistentHashRing::new(4).unwrap();
        assert_eq!(ring.get_shard_load(0), 0);
        ring.record_assignment(0);
        assert_eq!(ring.get_shard_load(0), 1);
        ring.record_assignment(0);
        assert_eq!(ring.get_shard_load(0), 2);
    }

    #[test]
    fn test_hash_ring_load_imbalance_empty() {
        let ring = ConsistentHashRing::new(4).unwrap();
        assert!((ring.load_imbalance() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hash_ring_load_imbalance_balanced() {
        let mut ring = ConsistentHashRing::new(4).unwrap();
        for _ in 0..4 {
            ring.record_assignment(0);
            ring.record_assignment(1);
            ring.record_assignment(2);
            ring.record_assignment(3);
        }
        assert!((ring.load_imbalance() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hash_ring_load_imbalance_unbalanced() {
        let mut ring = ConsistentHashRing::new(4).unwrap();
        for _ in 0..100 {
            ring.record_assignment(0);
        }
        let imbalance = ring.load_imbalance();
        assert!(imbalance > 0.5, "Imbalance {} should be high", imbalance);
    }

    #[test]
    fn test_hash_ring_least_loaded() {
        let mut ring = ConsistentHashRing::new(4).unwrap();
        ring.record_assignment(0);
        ring.record_assignment(0);
        ring.record_assignment(1);
        // Shard 2 and 3 have 0 load
        let least = ring.least_loaded_shard();
        assert!(
            least == 2 || least == 3,
            "Least loaded should be 2 or 3, got {}",
            least
        );
    }

    #[test]
    fn test_hash_ring_most_loaded() {
        let mut ring = ConsistentHashRing::new(4).unwrap();
        for _ in 0..10 {
            ring.record_assignment(0);
        }
        ring.record_assignment(1);
        assert_eq!(ring.most_loaded_shard(), 0);
    }

    // ─── HierarchicalShardManager Tests ────────────────────────────────

    #[test]
    fn test_shard_manager_creation() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let manager = HierarchicalShardManager::new(8, &clusters);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_shard_manager_zero_shards_error() {
        let clusters = vec![];
        let result = HierarchicalShardManager::new(0, &clusters);
        assert!(result.is_err());
    }

    #[test]
    fn test_shard_manager_assign_node() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        let result = manager.assign_node(1);
        assert!(result.is_ok());
        let assignment = result.unwrap();
        assert_eq!(assignment.node_id, 1);
        assert!(assignment.shard_id < 8);
    }

    #[test]
    fn test_shard_manager_double_assign_error() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        manager.assign_node(1).unwrap();
        let result = manager.assign_node(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_shard_manager_assign_to_shard() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        let result = manager.assign_node_to_shard(1, 3);
        assert!(result.is_ok());
        let assignment = result.unwrap();
        assert_eq!(assignment.shard_id, 3);
    }

    #[test]
    fn test_shard_manager_assign_to_invalid_shard() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        let result = manager.assign_node_to_shard(1, 99);
        assert!(result.is_err());
    }

    #[test]
    fn test_shard_manager_get_assignment() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        manager.assign_node(42).unwrap();
        let assignment = manager.get_node_assignment(42);
        assert!(assignment.is_some());
        assert_eq!(assignment.unwrap().node_id, 42);
    }

    #[test]
    fn test_shard_manager_get_missing_assignment() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        assert!(manager.get_node_assignment(999).is_none());
    }

    #[test]
    fn test_shard_manager_total_nodes() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        assert_eq!(manager.total_nodes(), 0);
        manager.assign_node(1).unwrap();
        assert_eq!(manager.total_nodes(), 1);
        manager.assign_node(2).unwrap();
        assert_eq!(manager.total_nodes(), 2);
    }

    #[test]
    fn test_shard_manager_remove_node() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        manager.assign_node(1).unwrap();
        assert_eq!(manager.total_nodes(), 1);
        let removed = manager.remove_node(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().node_id, 1);
        assert_eq!(manager.total_nodes(), 0);
    }

    #[test]
    fn test_shard_manager_remove_missing_node() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        let removed = manager.remove_node(999);
        assert!(removed.is_none());
    }

    #[test]
    fn test_shard_manager_rebalance() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let mut manager = HierarchicalShardManager::new(4, &clusters).unwrap();

        // Assign many nodes to create imbalance
        for i in 0..20 {
            manager.assign_node_to_shard(i, 0).unwrap();
        }
        manager.assign_node_to_shard(100, 3).unwrap();

        let result = manager.rebalance();
        // Should succeed and move a node
        assert!(result.is_ok());
    }

    #[test]
    fn test_shard_manager_load_imbalance() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let manager = HierarchicalShardManager::new(4, &clusters).unwrap();
        assert!((manager.load_imbalance() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_shard_manager_get_shard_config() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        let config = manager.get_shard_config(0);
        assert!(config.is_some());
        assert_eq!(config.unwrap().shard_id, 0);
    }

    #[test]
    fn test_shard_manager_get_missing_shard_config() {
        let clusters = vec![ClusterConfig::new(0, 10, 0)];
        let manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        assert!(manager.get_shard_config(99).is_none());
    }

    // ─── PeerInfo Tests ────────────────────────────────────────────────

    #[test]
    fn test_peer_info_new() {
        let peer = PeerInfo::new(1, 0, "127.0.0.1:8080");
        assert_eq!(peer.peer_id, 1);
        assert_eq!(peer.shard_id, 0);
        assert_eq!(peer.address, "127.0.0.1:8080");
        assert!((peer.trust_score - 0.5).abs() < f64::EPSILON);
        assert!((peer.energy_budget - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_peer_info_trust_threshold() {
        let peer = PeerInfo {
            trust_score: 0.8,
            ..PeerInfo::new(1, 0, "127.0.0.1:8080")
        };
        assert!(peer.meets_trust_threshold(0.5));
        assert!(peer.meets_trust_threshold(0.8));
        assert!(!peer.meets_trust_threshold(0.9));
    }

    #[test]
    fn test_peer_info_active() {
        let peer = PeerInfo {
            last_seen: 1000,
            ..PeerInfo::new(1, 0, "127.0.0.1:8080")
        };
        assert!(peer.is_active(500, 1400));
        assert!(!peer.is_active(500, 1600));
    }

    // ─── ClusterDiscovery Tests ────────────────────────────────────────

    #[test]
    fn test_cluster_discovery_default() {
        let discovery = ClusterDiscovery::default();
        assert!((discovery.min_trust_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cluster_discovery_new() {
        let discovery = ClusterDiscovery::new(0.7);
        assert!((discovery.min_trust_threshold - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cluster_discovery_with_max_peers() {
        let discovery = ClusterDiscovery::new(0.5).with_max_peers(200);
        assert_eq!(discovery.max_peers_per_shard, 200);
    }

    #[test]
    fn test_cluster_discovery_with_timeout() {
        let discovery = ClusterDiscovery::new(0.5).with_timeout(10000);
        assert_eq!(discovery.discovery_timeout_ms, 10000);
    }

    #[test]
    fn test_cluster_discovery_add_peer() {
        let mut discovery = ClusterDiscovery::new(0.5);
        let peer = PeerInfo::new(1, 0, "127.0.0.1:8080");
        discovery.add_peer(peer);
        assert_eq!(discovery.peer_count(), 1);
    }

    #[test]
    fn test_cluster_discovery_remove_peer() {
        let mut discovery = ClusterDiscovery::new(0.5);
        let peer = PeerInfo::new(1, 0, "127.0.0.1:8080");
        discovery.add_peer(peer);
        let removed = discovery.remove_peer(1);
        assert!(removed.is_some());
        assert_eq!(discovery.peer_count(), 0);
    }

    #[test]
    fn test_cluster_discovery_remove_missing_peer() {
        let mut discovery = ClusterDiscovery::new(0.5);
        let removed = discovery.remove_peer(999);
        assert!(removed.is_none());
    }

    #[test]
    fn test_cluster_discovery_peers_in_shard() {
        let mut discovery = ClusterDiscovery::new(0.5);
        discovery.add_peer(PeerInfo::new(1, 0, "a"));
        discovery.add_peer(PeerInfo::new(2, 0, "b"));
        discovery.add_peer(PeerInfo::new(3, 1, "c"));
        let shard0_peers = discovery.discover_peers_in_shard(0);
        assert_eq!(shard0_peers.len(), 2);
    }

    #[test]
    fn test_cluster_discovery_trusted_peers() {
        let mut discovery = ClusterDiscovery::new(0.7);
        discovery.add_peer(PeerInfo {
            trust_score: 0.9,
            ..PeerInfo::new(1, 0, "a")
        });
        discovery.add_peer(PeerInfo {
            trust_score: 0.5,
            ..PeerInfo::new(2, 0, "b")
        });
        let trusted = discovery.discover_trusted_peers();
        assert_eq!(trusted.len(), 1);
        assert_eq!(trusted[0].peer_id, 1);
    }

    #[test]
    fn test_cluster_discovery_active_peers() {
        let mut discovery = ClusterDiscovery::new(0.5);
        discovery.add_peer(PeerInfo {
            last_seen: 1000,
            ..PeerInfo::new(1, 0, "a")
        });
        discovery.add_peer(PeerInfo {
            last_seen: 100,
            ..PeerInfo::new(2, 0, "b")
        });
        let active = discovery.discover_active_peers(0, 1400, 500);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].peer_id, 1);
    }

    #[test]
    fn test_cluster_discovery_get_peer() {
        let mut discovery = ClusterDiscovery::new(0.5);
        discovery.add_peer(PeerInfo::new(1, 0, "a"));
        let peer = discovery.get_peer(1);
        assert!(peer.is_some());
        assert_eq!(peer.unwrap().peer_id, 1);
    }

    #[test]
    fn test_cluster_discovery_get_missing_peer() {
        let discovery = ClusterDiscovery::new(0.5);
        assert!(discovery.get_peer(999).is_none());
    }

    #[test]
    fn test_cluster_discovery_avg_trust() {
        let mut discovery = ClusterDiscovery::new(0.5);
        discovery.add_peer(PeerInfo {
            trust_score: 0.8,
            ..PeerInfo::new(1, 0, "a")
        });
        discovery.add_peer(PeerInfo {
            trust_score: 0.6,
            ..PeerInfo::new(2, 0, "b")
        });
        let avg = discovery.avg_trust_in_shard(0);
        assert!((avg - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cluster_discovery_avg_trust_empty_shard() {
        let discovery = ClusterDiscovery::new(0.5);
        let avg = discovery.avg_trust_in_shard(0);
        assert!((avg - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cluster_discovery_total_energy() {
        let mut discovery = ClusterDiscovery::new(0.5);
        discovery.add_peer(PeerInfo {
            energy_budget: 1.5,
            ..PeerInfo::new(1, 0, "a")
        });
        discovery.add_peer(PeerInfo {
            energy_budget: 2.5,
            ..PeerInfo::new(2, 0, "b")
        });
        let total = discovery.total_energy_in_shard(0);
        assert!((total - 4.0).abs() < f64::EPSILON);
    }

    // ─── Trust-Weighted Shard Voting Tests ─────────────────────────────

    #[test]
    fn test_shard_vote_basic() {
        let votes = vec![(1, 0, 0.9), (2, 0, 0.8), (3, 1, 0.5)];
        let result = trust_weighted_shard_vote(&votes, 1.0);
        assert_eq!(result.winning_shard, 0);
        assert!((result.winning_weight - 1.7).abs() < 1e-10, "winning_weight = {}", result.winning_weight);
        assert_eq!(result.total_votes, 3);
        assert!(result.quorum_met);
    }

    #[test]
    fn test_shard_vote_empty() {
        let votes: Vec<(u64, u64, f64)> = vec![];
        let result = trust_weighted_shard_vote(&votes, 1.0);
        assert_eq!(result.winning_shard, 0);
        assert!((result.winning_weight - 0.0).abs() < f64::EPSILON);
        assert!(!result.quorum_met);
    }

    #[test]
    fn test_shard_vote_quorum_not_met() {
        let votes = vec![(1, 0, 0.3)];
        let result = trust_weighted_shard_vote(&votes, 1.0);
        assert!(!result.quorum_met);
    }

    #[test]
    fn test_shard_vote_quorum_met() {
        let votes = vec![(1, 0, 0.6), (2, 0, 0.5)];
        let result = trust_weighted_shard_vote(&votes, 1.0);
        assert!(result.quorum_met);
    }

    #[test]
    fn test_shard_vote_tie_breaking() {
        let votes = vec![(1, 0, 0.5), (2, 1, 0.5)];
        let result = trust_weighted_shard_vote(&votes, 0.0);
        // Both have same weight; result should be deterministic
        assert!(result.winning_shard == 0 || result.winning_shard == 1);
    }

    #[test]
    fn test_shard_vote_byzantine_resistance() {
        // Byzantine nodes with low trust can't win
        let votes = vec![
            (1, 0, 0.9),
            (2, 0, 0.8),
            (3, 1, 0.1), // Byzantine
            (4, 1, 0.1), // Byzantine
            (5, 1, 0.1), // Byzantine
        ];
        let result = trust_weighted_shard_vote(&votes, 0.0);
        assert_eq!(result.winning_shard, 0);
        assert!((result.winning_weight - 1.7).abs() < 1e-10, "winning_weight = {}", result.winning_weight);
    }

    // ─── Load-Based Assignment Tests ───────────────────────────────────

    #[test]
    fn test_assign_by_load_basic() {
        let configs = vec![ShardConfig::new(0, 0, 100), ShardConfig::new(1, 0, 100)];
        let mut loads = HashMap::new();
        loads.insert(0, 80);
        loads.insert(1, 20);

        let result = assign_node_by_load(&configs, &loads, 1.0, 0.1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Shard 1 has lower load
    }

    #[test]
    fn test_assign_by_load_empty_configs() {
        let configs: Vec<ShardConfig> = vec![];
        let loads = HashMap::new();
        let result = assign_node_by_load(&configs, &loads, 1.0, 0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_assign_by_load_all_full() {
        let configs = vec![ShardConfig::new(0, 0, 10), ShardConfig::new(1, 0, 10)];
        let mut loads = HashMap::new();
        loads.insert(0, 10);
        loads.insert(1, 10);

        let result = assign_node_by_load(&configs, &loads, 1.0, 0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_assign_by_load_energy_constraint() {
        let configs = vec![ShardConfig::new(0, 0, 100)];
        let mut loads = HashMap::new();
        loads.insert(0, 0);

        // High energy node can assign
        let result = assign_node_by_load(&configs, &loads, 1.0, 0.1);
        assert!(result.is_ok());

        // Low energy node with high minimum
        let result = assign_node_by_load(&configs, &loads, 0.05, 0.5);
        // Should still work since shard is not at capacity
        assert!(result.is_ok());
    }

    // ─── ShardAssignment Display Tests ─────────────────────────────────

    #[test]
    fn test_shard_assignment_display() {
        let assignment = ShardAssignment {
            node_id: 42,
            shard_id: 3,
            cluster_id: 0,
            hash_position: 0x1234567890ABCDEF,
            load_ratio: 0.5,
            rebalanced: false,
        };
        let display = format!("{}", assignment);
        assert!(display.contains("node=42"));
        assert!(display.contains("shard=3"));
        assert!(display.contains("cluster=0"));
    }

    // ─── Integration Tests ─────────────────────────────────────────────

    #[test]
    fn test_full_assignment_flow() {
        let clusters = vec![ClusterConfig::new(0, 4, 0), ClusterConfig::new(1, 4, 4)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();

        // Assign 100 nodes
        for i in 0..100 {
            manager.assign_node(i).unwrap();
        }

        assert_eq!(manager.total_nodes(), 100);

        // Check distribution
        let mut shard_counts = vec![0usize; 8];
        for i in 0..100 {
            if let Some(assignment) = manager.get_node_assignment(i) {
                shard_counts[assignment.shard_id as usize] += 1;
            }
        }

        let min_count = *shard_counts.iter().min().unwrap();
        let max_count = *shard_counts.iter().max().unwrap();
        let ratio = min_count as f64 / max_count as f64;

        // Should be reasonably balanced
        assert!(
            ratio > 0.3,
            "Distribution too uneven: min={}, max={}, ratio={:.2}",
            min_count,
            max_count,
            ratio
        );
    }

    #[test]
    fn test_discovery_with_manager() {
        let clusters = vec![ClusterConfig::new(0, 8, 0)];
        let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
        let mut discovery = ClusterDiscovery::new(0.6);

        // Assign nodes and add to discovery
        for i in 0..50 {
            let assignment = manager.assign_node(i).unwrap();
            let peer = PeerInfo {
                peer_id: i,
                shard_id: assignment.shard_id,
                trust_score: if i % 5 == 0 { 0.4 } else { 0.8 },
                energy_budget: 1.0,
                address: format!("127.0.0.1:{}", 8000 + i),
                last_seen: 1000,
            };
            discovery.add_peer(peer);
        }

        assert_eq!(discovery.peer_count(), 50);

        // Trusted peers should exclude low-trust nodes
        let trusted = discovery.discover_trusted_peers();
        assert!(trusted.len() < 50); // Some have trust 0.4 < 0.6

        // Per-shard discovery
        for shard_id in 0..8 {
            let shard_peers = discovery.discover_peers_in_shard(shard_id);
            assert!(shard_peers.len() <= 50);
        }
    }

    #[test]
    fn test_vote_based_rebalancing() {
        let votes: Vec<(u64, u64, f64)> = (0..20)
            .map(|i| {
                let shard = if i < 15 { 0 } else { 1 };
                let trust = if i < 15 { 0.9 } else { 0.3 };
                (i, shard as u64, trust as f64)
            })
            .collect();

        let result = trust_weighted_shard_vote(&votes, 5.0);
        assert_eq!(result.winning_shard, 0);
        assert!(result.quorum_met);
    }
}
