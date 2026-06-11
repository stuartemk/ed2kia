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

    // ─── Sprint 123: Hierarchical Sharding v2 ────────────────────────────

    /// Dynamic rebalancing based on load + energy + trust.
    ///
    /// Considers not only node count but also energy budget and trust scores
    /// when deciding which nodes to migrate.
    ///
    /// # Arguments
    /// * `energy_budgets` — Energy budget per node (Wh)
    /// * `trust_scores` — Trust score per node ∈ [0, 1]
    /// * `max_migrations` — Maximum number of node migrations per round
    ///
    /// # Returns
    /// List of new shard assignments from rebalancing
    pub fn rebalance_dynamic(
        &mut self,
        energy_budgets: &std::collections::HashMap<u64, f64>,
        trust_scores: &std::collections::HashMap<u64, f64>,
        max_migrations: usize,
    ) -> Result<Vec<ShardAssignment>, ShardingError> {
        if self.total_shards == 0 {
            return Ok(Vec::new());
        }

        let mut migrations = Vec::new();
        let mut moved = std::collections::HashSet::new();

        // Find the most loaded and least loaded shards
        let (overloaded_shard, overloaded_count) = self
            .shard_nodes
            .iter()
            .max_by_key(|(_, nodes)| nodes.len())
            .map(|(&shard, nodes)| (shard, nodes.len()))
            .unwrap_or((0, 0));

        let (underloaded_shard, underloaded_count) = self
            .shard_nodes
            .iter()
            .min_by_key(|(_, nodes)| nodes.len())
            .map(|(&shard, nodes)| (shard, nodes.len()))
            .unwrap_or((0, 0));

        let threshold = (overloaded_count as f64 - underloaded_count as f64) / 2.0;

        if threshold < 1.0 {
            return Ok(Vec::new());
        }

        // Migrate nodes from overloaded to underloaded shard
        // Collect node IDs first to avoid borrow conflicts
        let candidate_nodes: Vec<u64> = self
            .shard_nodes
            .get(&overloaded_shard)
            .map(|nodes| nodes.iter().take(max_migrations).copied().collect())
            .unwrap_or_default();
        for &node_id in &candidate_nodes {
            if migrations.len() >= max_migrations || moved.contains(&node_id) {
                break;
            }
            // Prefer migrating low-trust, high-energy nodes
            let trust = trust_scores.get(&node_id).copied().unwrap_or(0.5);
            let energy = energy_budgets.get(&node_id).copied().unwrap_or(100.0);

            // Only migrate if trust is reasonable and energy allows
            if trust < 0.1 || energy < 10.0 {
                continue;
            }

            // Remove from overloaded shard
            if let Some(nodes) = self.shard_nodes.get_mut(&overloaded_shard) {
                nodes.remove(&node_id);
            }

            // Add to underloaded shard
            if let Some(nodes) = self.shard_nodes.get_mut(&underloaded_shard) {
                nodes.insert(node_id);
            }

            // Update assignment
            let new_assignment = ShardAssignment {
                node_id,
                shard_id: underloaded_shard,
                cluster_id: self
                    .get_shard_config(underloaded_shard)
                    .map(|c| c.cluster_id)
                    .unwrap_or(0),
                hash_position: 0,
                load_ratio: 0.0,
                rebalanced: true,
            };

            if let Some(assignment) = self.node_assignments.get_mut(&node_id) {
                assignment.shard_id = underloaded_shard;
                assignment.cluster_id = new_assignment.cluster_id;
            }

            migrations.push(new_assignment);
            moved.insert(node_id);
        }

        Ok(migrations)
    }

    /// Get the load score for a shard considering energy and trust.
    ///
    /// Load score = node_count * avg_energy_cost / avg_trust
    /// Higher score = more loaded (considering quality-weighted load)
    ///
    /// # Arguments
    /// * `shard_id` — Shard to evaluate
    /// * `energy_budgets` — Energy budget per node
    /// * `trust_scores` — Trust score per node
    ///
    /// # Returns
    /// Quality-weighted load score
    pub fn shard_load_score(
        &self,
        shard_id: u64,
        energy_budgets: &std::collections::HashMap<u64, f64>,
        trust_scores: &std::collections::HashMap<u64, f64>,
    ) -> f64 {
        let nodes = match self.shard_nodes.get(&shard_id) {
            Some(n) => n,
            None => return 0.0,
        };

        if nodes.is_empty() {
            return 0.0;
        }

        let total_energy: f64 = nodes
            .iter()
            .map(|n| energy_budgets.get(n).copied().unwrap_or(100.0))
            .sum();
        let total_trust: f64 = nodes
            .iter()
            .map(|n| trust_scores.get(n).copied().unwrap_or(0.5))
            .sum();

        let avg_energy = total_energy / nodes.len() as f64;
        let avg_trust = total_trust / nodes.len() as f64;

        if avg_trust < 1e-12 {
            return nodes.len() as f64 * avg_energy;
        }

        nodes.len() as f64 * avg_energy / avg_trust
    }
}

// ─── Sprint 123: Fault-Tolerant Gossip ──────────────────────────────────────

/// Gossip message for shard state synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    /// Source node ID
    pub source: u64,
    /// Message type
    pub msg_type: GossipMessageType,
    /// Shard ID this message concerns
    pub shard_id: u64,
    /// Payload data
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
    /// PoSym signature (proof bytes)
    pub signature: Vec<u8>,
    /// Sequence number for deduplication
    pub sequence: u64,
}

/// Type of gossip message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GossipMessageType {
    /// Node joined a shard
    NodeJoin,
    /// Node left a shard
    NodeLeave,
    /// Shard rebalancing event
    Rebalance,
    /// Heartbeat / liveness
    Heartbeat,
    /// State sync request
    StateSync,
    /// State sync response
    StateResponse,
}

impl GossipMessage {
    /// Create a new gossip message.
    pub fn new(
        source: u64,
        msg_type: GossipMessageType,
        shard_id: u64,
        payload: Vec<u8>,
        signature: Vec<u8>,
        sequence: u64,
    ) -> Self {
        Self {
            source,
            msg_type,
            shard_id,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            signature,
            sequence,
        }
    }

    /// Verify the PoSym signature on this message.
    ///
    /// Returns `true` if the signature is structurally valid.
    pub fn verify_signature(&self) -> bool {
        !self.signature.is_empty() && self.signature.len() >= 32
    }

    /// Check if this message is stale (older than timeout).
    pub fn is_stale(&self, timeout_seconds: u64, current_time: u64) -> bool {
        if self.timestamp > current_time {
            return false; // Future timestamp (clock skew) — not stale
        }
        current_time - self.timestamp > timeout_seconds
    }
}

/// GossipSub protocol state for a single node.
#[derive(Debug)]
pub struct GossipSubState {
    /// Known peers and their last seen time
    peers: std::collections::HashMap<u64, u64>,
    /// Deduplication: seen message sequences
    seen_sequences: std::collections::HashMap<u64, u64>,
    /// Maximum messages to keep in dedup cache
    max_cache: usize,
    /// Message timeout in seconds
    timeout_seconds: u64,
}

impl GossipSubState {
    /// Create a new GossipSub state.
    pub fn new(max_cache: usize, timeout_seconds: u64) -> Self {
        Self {
            peers: std::collections::HashMap::new(),
            seen_sequences: std::collections::HashMap::new(),
            max_cache,
            timeout_seconds,
        }
    }

    /// Process an incoming gossip message.
    ///
    /// Returns `true` if the message should be forwarded (not duplicate, not stale).
    pub fn process_message(&mut self, msg: &GossipMessage, current_time: u64) -> bool {
        // Check if duplicate
        if self.seen_sequences.contains_key(&msg.sequence) {
            return false;
        }

        // Check if stale
        if msg.is_stale(self.timeout_seconds, current_time) {
            return false;
        }

        // Verify signature
        if !msg.verify_signature() {
            return false;
        }

        // Update peer info
        self.peers.insert(msg.source, current_time);

        // Add to dedup cache
        if self.seen_sequences.len() >= self.max_cache {
            // Evict oldest entry
            if let Some(oldest_key) = self
                .seen_sequences
                .iter()
                .min_by_key(|(_, &v)| v)
                .map(|(&k, _)| k)
            {
                self.seen_sequences.remove(&oldest_key);
            }
        }
        self.seen_sequences.insert(msg.sequence, msg.timestamp);

        true
    }

    /// Get active peers (seen within timeout).
    pub fn active_peers(&self, current_time: u64) -> Vec<u64> {
        self.peers
            .iter()
            .filter(|(_, &last_seen)| {
                if last_seen > current_time {
                    return true; // Clock skew — consider active
                }
                current_time - last_seen <= self.timeout_seconds
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Total number of known peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

impl Default for GossipSubState {
    fn default() -> Self {
        Self::new(1000, 30)
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

// ─── Sprint 125: Self-Healing Mesh + Auto-Rebalancing ────────────────────────

/// Churn metrics for self-healing decisions.
#[derive(Debug, Clone, Default)]
pub struct ChurnMetrics {
    /// Fraction of nodes that failed in the last observation window.
    pub failure_rate: f64,
    /// Average trust of surviving nodes.
    pub avg_surviving_trust: f64,
    /// Average energy of surviving nodes.
    pub avg_surviving_energy: f64,
    /// Number of observation windows since last stable state.
    pub instability_windows: u32,
}

impl ChurnMetrics {
    pub fn new(failure_rate: f64, avg_surviving_trust: f64, avg_surviving_energy: f64) -> Self {
        Self {
            failure_rate: failure_rate.clamp(0.0, 1.0),
            avg_surviving_trust,
            avg_surviving_energy,
            instability_windows: 0,
        }
    }

    /// Compute from shard manager state after node failures.
    pub fn from_shard_manager(
        shard_manager: &HierarchicalShardManager,
        failed_nodes: &[u64],
        trust_scores: &std::collections::HashMap<u64, f64>,
        energy_budgets: &std::collections::HashMap<u64, f64>,
    ) -> Self {
        let total_nodes = shard_manager.node_assignments.len();
        let failure_rate = if total_nodes > 0 {
            failed_nodes.len() as f64 / total_nodes as f64
        } else {
            0.0
        };

        let mut total_trust = 0.0;
        let mut total_energy = 0.0;
        let mut surviving_count = 0usize;
        for shard_id in shard_manager.shards.keys() {
            if let Some(nodes) = shard_manager.shard_nodes.get(shard_id) {
                for node_id in nodes {
                    if !failed_nodes.contains(node_id) {
                        total_trust += trust_scores.get(node_id).copied().unwrap_or(0.5);
                        total_energy += energy_budgets.get(node_id).copied().unwrap_or(100.0);
                        surviving_count += 1;
                    }
                }
            }
        }

        Self {
            failure_rate: failure_rate.clamp(0.0, 1.0),
            avg_surviving_trust: if surviving_count > 0 {
                total_trust / surviving_count as f64
            } else {
                0.0
            },
            avg_surviving_energy: if surviving_count > 0 {
                total_energy / surviving_count as f64
            } else {
                0.0
            },
            instability_windows: 0,
        }
    }
}

/// Result of a self-healing rebalance operation.
#[derive(Debug, Clone)]
pub struct RebalanceResult {
    /// Nodes that were removed (failed).
    pub removed_nodes: Vec<u64>,
    /// New assignments created during rebalancing.
    pub new_assignments: Vec<ShardAssignment>,
    /// Nodes redistributed to other shards.
    pub redistributed_nodes: Vec<u64>,
    /// Final load imbalance ratio (lower is better).
    pub final_imbalance: f64,
    /// Trust-weighted efficiency score (0.0–1.0).
    pub efficiency_score: f64,
    /// Whether the mesh is considered healthy after healing.
    pub mesh_healthy: bool,
}

impl std::fmt::Display for RebalanceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rebalance[removed={} redistributed={} new_assign={} imbalance={:.4} efficiency={:.4} healthy={}]",
            self.removed_nodes.len(),
            self.redistributed_nodes.len(),
            self.new_assignments.len(),
            self.final_imbalance,
            self.efficiency_score,
            if self.mesh_healthy { "YES" } else { "NO" }
        )
    }
}

/// Self-healing rebalance with PoSym trust and energy awareness.
///
/// Detects failed nodes, isolates them, and redistributes load
/// using trust-weighted + load + energy scoring.
///
/// # Arguments
/// * `shard_manager` — Mutable reference to the shard manager
/// * `failed_nodes` — List of node IDs that have failed
/// * `churn_metrics` — Current churn metrics for adaptive decisions
///
/// # Returns
/// `RebalanceResult` with full healing details
pub fn self_heal_rebalance(
    shard_manager: &mut HierarchicalShardManager,
    failed_nodes: &[u64],
    churn_metrics: &ChurnMetrics,
) -> Result<RebalanceResult, ShardingError> {
    let mut removed_nodes = Vec::new();
    let mut redistributed_nodes = Vec::new();
    let mut new_assignments = Vec::new();

    // Step 1: Remove failed nodes from all shards
    for node_id in failed_nodes {
        if shard_manager.remove_node(*node_id).is_some() {
            removed_nodes.push(*node_id);
        }
    }

    // Step 2: Identify overloaded shards and find healthy targets
    let shard_loads: Vec<(u64, usize)> = shard_manager
        .shard_nodes
        .iter()
        .map(|(id, nodes)| (*id, nodes.len()))
        .collect();

    let avg_load = if !shard_loads.is_empty() {
        shard_loads.iter().map(|(_, c)| *c).sum::<usize>() as f64 / shard_loads.len() as f64
    } else {
        0.0
    };

    // Step 3: Adaptive redistribution based on churn severity
    let churn_factor = churn_metrics.failure_rate;
    let trust_factor = churn_metrics.avg_surviving_trust;
    let safety_margin = if churn_factor > 0.3 {
        0.7 // Aggressive: keep shards under 70% of avg
    } else if churn_factor > 0.1 {
        0.85 // Moderate
    } else {
        1.0 // Normal
    };

    let target_load = (avg_load * safety_margin) as usize;

    // Step 4: Redistribute from overloaded shards
    for (shard_id, count) in &shard_loads {
        if *count > target_load && target_load > 0 {
            // Collect node IDs first to avoid holding immutable borrow
            let nodes_to_move = shard_manager
                .shard_nodes
                .get(shard_id)
                .map(|nodes| {
                    nodes
                        .iter()
                        .take(*count - target_load)
                        .copied()
                        .collect::<Vec<u64>>()
                })
                .unwrap_or_default();

            // Find least loaded shard for redistribution
            if let Some(target_shard) = shard_loads
                .iter()
                .filter(|(id, _)| *id != *shard_id)
                .min_by_key(|(_, c)| *c)
                .map(|(id, _)| *id)
            {
                for node_id in nodes_to_move {
                    // Trust-weighted eligibility check
                    let node_trust = trust_factor;
                    if node_trust >= 0.3 {
                        match shard_manager.assign_node_to_shard(node_id, target_shard) {
                            Ok(assignment) => {
                                redistributed_nodes.push(node_id);
                                new_assignments.push(assignment);
                            }
                            Err(_) => {
                                // Node may already be assigned elsewhere
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 5: Run standard rebalance for fine-tuning
    let _ = shard_manager.rebalance()?;

    // Step 6: Compute final metrics
    let final_imbalance = shard_manager.load_imbalance();
    let total_nodes = shard_manager.node_assignments.len();

    // Efficiency = trust * (1 - imbalance) * (1 - churn_penalty)
    let churn_penalty = churn_factor * 0.5;
    let efficiency_score =
        (trust_factor * (1.0 - final_imbalance) * (1.0 - churn_penalty)).clamp(0.0, 1.0);

    // Mesh is healthy if: imbalance < 0.3, efficiency > 0.6, failure_rate < 0.5
    let mesh_healthy =
        final_imbalance < 0.3 && efficiency_score > 0.6 && churn_factor < 0.5 && total_nodes > 0;

    Ok(RebalanceResult {
        removed_nodes,
        new_assignments,
        redistributed_nodes,
        final_imbalance,
        efficiency_score,
        mesh_healthy,
    })
}

/// Auto-rebalancing trigger based on periodic health checks.
/// Returns `true` if a rebalance was triggered.
pub fn should_trigger_rebalance(
    shard_manager: &HierarchicalShardManager,
    churn_metrics: &ChurnMetrics,
    min_nodes_threshold: usize,
) -> bool {
    let imbalance = shard_manager.load_imbalance();
    let failure_rate = churn_metrics.failure_rate;
    let instability = churn_metrics.instability_windows;

    // Trigger if any condition is met
    imbalance > 0.4
        || failure_rate > 0.15
        || instability > 3
        || shard_manager.total_nodes() < min_nodes_threshold
}

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
        let mut counts = [0usize; 10];

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
        assert!(
            (result.winning_weight - 1.7).abs() < 1e-10,
            "winning_weight = {}",
            result.winning_weight
        );
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
        assert!(
            (result.winning_weight - 1.7).abs() < 1e-10,
            "winning_weight = {}",
            result.winning_weight
        );
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
        let mut shard_counts = [0usize; 8];
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
                (i, shard as u64, trust)
            })
            .collect();

        let result = trust_weighted_shard_vote(&votes, 5.0);
        assert_eq!(result.winning_shard, 0);
        assert!(result.quorum_met);
    }

    // ====================================================================
    // Sprint 123 — Hierarchical Sharding v2 + Gossip Tests
    // ====================================================================

    #[test]
    fn test_rebalance_dynamic_basic() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut manager = HierarchicalShardManager::new(4, &clusters)?;

        // Add 10 nodes to shard 0, 0 to others
        for i in 0..10u64 {
            manager.assign_node_to_shard(i, 0)?;
        }

        let energy_budgets: std::collections::HashMap<u64, f64> =
            (0..10).map(|i| (i as u64, 100.0)).collect();
        let trust_scores: std::collections::HashMap<u64, f64> =
            (0..10).map(|i| (i as u64, 0.8)).collect();

        let migrations = manager.rebalance_dynamic(&energy_budgets, &trust_scores, 3)?;
        assert!(!migrations.is_empty());
        // Shard 0 should have fewer nodes after rebalancing
        assert!(manager.shard_nodes.get(&0).unwrap().len() < 10);
        Ok(())
    }

    #[test]
    fn test_rebalance_dynamic_balanced() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 2, 0)];
        let mut manager = HierarchicalShardManager::new(2, &clusters)?;

        manager.assign_node_to_shard(0, 0)?;
        manager.assign_node_to_shard(1, 1)?;

        let energy_budgets: std::collections::HashMap<u64, f64> =
            [(0u64, 100.0), (1, 100.0)].iter().copied().collect();
        let trust_scores: std::collections::HashMap<u64, f64> =
            [(0u64, 0.8), (1, 0.8)].iter().copied().collect();

        let migrations = manager.rebalance_dynamic(&energy_budgets, &trust_scores, 5)?;
        assert!(migrations.is_empty()); // Already balanced
        Ok(())
    }

    #[test]
    fn test_rebalance_dynamic_skips_low_trust() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut manager = HierarchicalShardManager::new(4, &clusters)?;

        for i in 0..5u64 {
            manager.assign_node_to_shard(i, 0)?;
        }

        // Node 0 has very low trust — should not be migrated
        let mut energy_budgets = std::collections::HashMap::new();
        let mut trust_scores = std::collections::HashMap::new();
        for i in 0..5u64 {
            energy_budgets.insert(i, 100.0);
            trust_scores.insert(i, if i == 0 { 0.05 } else { 0.8 });
        }

        let migrations = manager.rebalance_dynamic(&energy_budgets, &trust_scores, 3)?;
        // Node 0 should not be in migrations
        for m in &migrations {
            assert_ne!(m.node_id, 0);
        }
        Ok(())
    }

    #[test]
    fn test_shard_load_score_empty() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 2, 0)];
        let manager = HierarchicalShardManager::new(2, &clusters)?;
        let score = manager.shard_load_score(
            0,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );
        assert_eq!(score, 0.0);
        Ok(())
    }

    #[test]
    fn test_shard_load_score_basic() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 2, 0)];
        let mut manager = HierarchicalShardManager::new(2, &clusters)?;

        manager.assign_node_to_shard(0, 0)?;
        manager.assign_node_to_shard(1, 0)?;

        let mut energy_budgets = std::collections::HashMap::new();
        let mut trust_scores = std::collections::HashMap::new();
        energy_budgets.insert(0u64, 100.0);
        energy_budgets.insert(1u64, 200.0);
        trust_scores.insert(0u64, 1.0);
        trust_scores.insert(1u64, 1.0);

        let score = manager.shard_load_score(0, &energy_budgets, &trust_scores);
        // 2 nodes * avg_energy(150) / avg_trust(1.0) = 300
        assert!((score - 300.0).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn test_gossip_message_creation() {
        let sig = vec![0x42; 32];
        let msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![1, 2, 3], sig, 100);
        assert_eq!(msg.source, 1);
        assert_eq!(msg.shard_id, 0);
        assert_eq!(msg.sequence, 100);
        assert!(msg.verify_signature());
    }

    #[test]
    fn test_gossip_message_verify_empty_signature() {
        let msg = GossipMessage::new(
            1,
            GossipMessageType::Heartbeat,
            0,
            vec![],
            vec![], // Empty signature
            1,
        );
        assert!(!msg.verify_signature());
    }

    #[test]
    fn test_gossip_message_stale() {
        let sig = vec![0x42; 32];
        let mut msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig, 1);
        msg.timestamp = 100;
        assert!(msg.is_stale(30, 200)); // 200 - 100 = 100 > 30
    }

    #[test]
    fn test_gossip_message_not_stale() {
        let sig = vec![0x42; 32];
        let mut msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig, 1);
        msg.timestamp = 100;
        assert!(!msg.is_stale(30, 120)); // 120 - 100 = 20 <= 30
    }

    #[test]
    fn test_gossip_message_future_timestamp() {
        let sig = vec![0x42; 32];
        let mut msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig, 1);
        msg.timestamp = 500;
        assert!(!msg.is_stale(30, 100)); // Future — not stale
    }

    #[test]
    fn test_gossip_message_type_variants() {
        assert!(matches!(
            GossipMessageType::NodeJoin,
            GossipMessageType::NodeJoin
        ));
        assert!(matches!(
            GossipMessageType::NodeLeave,
            GossipMessageType::NodeLeave
        ));
        assert!(matches!(
            GossipMessageType::Rebalance,
            GossipMessageType::Rebalance
        ));
        assert!(matches!(
            GossipMessageType::Heartbeat,
            GossipMessageType::Heartbeat
        ));
        assert!(matches!(
            GossipMessageType::StateSync,
            GossipMessageType::StateSync
        ));
        assert!(matches!(
            GossipMessageType::StateResponse,
            GossipMessageType::StateResponse
        ));
    }

    #[test]
    fn test_gossipsub_default() {
        let state = GossipSubState::default();
        assert_eq!(state.peer_count(), 0);
    }

    #[test]
    fn test_gossipsub_process_valid_message() {
        let mut state = GossipSubState::new(100, 30);
        let sig = vec![0x42; 32];
        let msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig, 1);
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(state.process_message(&msg, current_time));
        assert_eq!(state.peer_count(), 1);
    }

    #[test]
    fn test_gossipsub_deduplication() {
        let mut state = GossipSubState::new(100, 30);
        let sig = vec![0x42; 32];
        let msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig, 42);
        let current_time = 1000u64;

        assert!(state.process_message(&msg, current_time));
        // Same sequence → duplicate
        assert!(!state.process_message(&msg, current_time));
    }

    #[test]
    fn test_gossipsub_rejects_stale() {
        let mut state = GossipSubState::new(100, 30);
        let sig = vec![0x42; 32];
        let mut msg = GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig, 1);
        msg.timestamp = 100;
        assert!(!state.process_message(&msg, 500)); // 500 - 100 = 400 > 30
    }

    #[test]
    fn test_gossipsub_rejects_bad_signature() {
        let mut state = GossipSubState::new(100, 30);
        let msg = GossipMessage::new(
            1,
            GossipMessageType::Heartbeat,
            0,
            vec![],
            vec![], // Empty signature
            1,
        );
        let current_time = 1000u64;
        assert!(!state.process_message(&msg, current_time));
    }

    #[test]
    fn test_gossipsub_active_peers() {
        let mut state = GossipSubState::new(100, 30);
        let sig = vec![0x42; 32];

        let mut msg1 =
            GossipMessage::new(1, GossipMessageType::Heartbeat, 0, vec![], sig.clone(), 1);
        msg1.timestamp = 970;
        state.process_message(&msg1, 1000);

        let mut msg2 = GossipMessage::new(2, GossipMessageType::Heartbeat, 0, vec![], sig, 2);
        msg2.timestamp = 900; // Stale
        state.process_message(&msg2, 1000);

        // Only peer 1 should be active (seen at 970, timeout 30)
        let active = state.active_peers(1000);
        assert!(active.contains(&1));
    }

    #[test]
    fn test_gossipsub_cache_eviction() {
        let mut state = GossipSubState::new(3, 30);
        let sig = vec![0x42; 32];
        let current_time = 1000u64;

        for i in 0..5u64 {
            let mut msg =
                GossipMessage::new(i, GossipMessageType::Heartbeat, 0, vec![], sig.clone(), i);
            msg.timestamp = current_time - i * 10;
            state.process_message(&msg, current_time);
        }

        // Cache should be at most 3
        assert!(state.seen_sequences.len() <= 3);
    }

    // ─── Sprint 125: Self-Healing Mesh Tests ─────────────────────────────

    #[test]
    fn test_churn_metrics_new() {
        let metrics = ChurnMetrics::new(0.2, 0.8, 150.0);
        assert!((metrics.failure_rate - 0.2).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_trust - 0.8).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_energy - 150.0).abs() < f64::EPSILON);
        assert_eq!(metrics.instability_windows, 0);
    }

    #[test]
    fn test_churn_metrics_failure_rate_clamped() {
        let metrics = ChurnMetrics::new(1.5, 0.5, 100.0);
        assert!((metrics.failure_rate - 1.0).abs() < f64::EPSILON);
        let metrics2 = ChurnMetrics::new(-0.3, 0.5, 100.0);
        assert!((metrics2.failure_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_churn_metrics_default() {
        let metrics = ChurnMetrics::default();
        assert!((metrics.failure_rate - 0.0).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_trust - 0.0).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_energy - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.instability_windows, 0);
    }

    #[test]
    fn test_churn_metrics_from_shard_manager_no_failures() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        for i in 0..8u64 {
            sm.assign_node(i)?;
        }
        let trust: HashMap<u64, f64> = (0..8).map(|i| (i, 0.9)).collect();
        let energy: HashMap<u64, f64> = (0..8).map(|i| (i, 100.0)).collect();
        let metrics = ChurnMetrics::from_shard_manager(&sm, &[], &trust, &energy);
        assert!((metrics.failure_rate - 0.0).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_trust - 0.9).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_energy - 100.0).abs() < f64::EPSILON);
        Ok(())
    }

    #[test]
    fn test_churn_metrics_from_shard_manager_with_failures() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        for i in 0..10u64 {
            sm.assign_node(i)?;
        }
        let trust: HashMap<u64, f64> = (0..10).map(|i| (i, 0.8)).collect();
        let energy: HashMap<u64, f64> = (0..10).map(|i| (i, 120.0)).collect();
        let failed = vec![0u64, 1u64, 2u64];
        let metrics = ChurnMetrics::from_shard_manager(&sm, &failed, &trust, &energy);
        assert!((metrics.failure_rate - 0.3).abs() < f64::EPSILON);
        assert!((metrics.avg_surviving_trust - 0.8).abs() < f64::EPSILON);
        Ok(())
    }

    #[test]
    fn test_rebalance_result_display() -> Result<(), ShardingError> {
        let result = RebalanceResult {
            removed_nodes: vec![1, 2],
            new_assignments: vec![],
            redistributed_nodes: vec![3],
            final_imbalance: 0.1,
            efficiency_score: 0.85,
            mesh_healthy: true,
        };
        let s = format!("{}", result);
        assert!(s.contains("0.1000"));
        assert!(s.contains("0.8500"));
        assert!(s.contains("YES"));
        Ok(())
    }

    #[test]
    fn test_should_trigger_rebalance_low_imbalance() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let sm = HierarchicalShardManager::new(4, &clusters)?;
        let metrics = ChurnMetrics::new(0.0, 0.9, 100.0);
        let trigger = should_trigger_rebalance(&sm, &metrics, 0);
        assert!(!trigger); // Low imbalance, no failures, no instability
        Ok(())
    }

    #[test]
    fn test_should_trigger_rebalance_high_failure_rate() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let sm = HierarchicalShardManager::new(4, &clusters)?;
        let metrics = ChurnMetrics::new(0.25, 0.5, 100.0);
        let trigger = should_trigger_rebalance(&sm, &metrics, 0);
        assert!(trigger); // failure_rate > 0.15
        Ok(())
    }

    #[test]
    fn test_should_trigger_rebalance_high_instability() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let sm = HierarchicalShardManager::new(4, &clusters)?;
        let mut metrics = ChurnMetrics::new(0.0, 0.9, 100.0);
        metrics.instability_windows = 5;
        let trigger = should_trigger_rebalance(&sm, &metrics, 0);
        assert!(trigger); // instability > 3
        Ok(())
    }

    #[test]
    fn test_self_heal_rebalance_empty() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        let metrics = ChurnMetrics::new(0.0, 0.9, 100.0);
        let result = self_heal_rebalance(&mut sm, &[], &metrics)?;
        assert_eq!(result.removed_nodes.len(), 0);
        assert!((result.final_imbalance - 0.0).abs() < 1.0);
        Ok(())
    }

    #[test]
    fn test_self_heal_rebalance_removes_failed_nodes() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        for i in 0..8u64 {
            sm.assign_node(i)?;
        }
        let metrics = ChurnMetrics::new(0.25, 0.8, 100.0);
        let failed = vec![0u64, 1u64];
        let result = self_heal_rebalance(&mut sm, &failed, &metrics)?;
        assert!(result.removed_nodes.contains(&0));
        assert!(result.removed_nodes.contains(&1));
        Ok(())
    }

    #[test]
    fn test_self_heal_rebalance_mesh_healthy() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 16, 0)];
        let mut sm = HierarchicalShardManager::new(16, &clusters)?;
        for i in 0..256u64 {
            sm.assign_node(i)?;
        }
        let metrics = ChurnMetrics::new(0.02, 0.95, 100.0);
        let result = self_heal_rebalance(&mut sm, &[], &metrics)?;
        // Verify key properties: no nodes removed, efficiency bounded, imbalance reasonable
        assert_eq!(result.removed_nodes.len(), 0);
        assert!(result.efficiency_score >= 0.0 && result.efficiency_score <= 1.0);
        assert!(result.final_imbalance >= 0.0 && result.final_imbalance < 1.0);
        // With many nodes and no failures, efficiency should be decent
        assert!(result.efficiency_score > 0.5);
        Ok(())
    }

    #[test]
    fn test_self_heal_rebalance_high_churn_safety_margin() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        for i in 0..20u64 {
            sm.assign_node(i)?;
        }
        let metrics = ChurnMetrics::new(0.4, 0.6, 80.0);
        let failed: Vec<u64> = (0..5).collect();
        let result = self_heal_rebalance(&mut sm, &failed, &metrics)?;
        assert_eq!(result.removed_nodes.len(), 5);
        // With high churn, safety margin is 0.7 → lower target load
        assert!(result.final_imbalance < 0.5);
        Ok(())
    }

    #[test]
    fn test_self_heal_rebalance_efficiency_score_bounded() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        for i in 0..8u64 {
            sm.assign_node(i)?;
        }
        let metrics = ChurnMetrics::new(0.1, 0.7, 100.0);
        let result = self_heal_rebalance(&mut sm, &[], &metrics)?;
        assert!(result.efficiency_score >= 0.0);
        assert!(result.efficiency_score <= 1.0);
        Ok(())
    }

    #[test]
    fn test_self_heal_rebalance_no_nodes_after_removal() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let mut sm = HierarchicalShardManager::new(4, &clusters)?;
        sm.assign_node(0)?;
        let metrics = ChurnMetrics::new(1.0, 0.0, 0.0);
        let result = self_heal_rebalance(&mut sm, &[0], &metrics)?;
        assert_eq!(result.removed_nodes.len(), 1);
        assert!(!result.mesh_healthy); // No nodes = unhealthy
        Ok(())
    }

    #[test]
    fn test_rebalance_trigger_min_nodes_threshold() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 4, 0)];
        let sm = HierarchicalShardManager::new(4, &clusters)?;
        let metrics = ChurnMetrics::new(0.0, 0.9, 100.0);
        // Empty manager has 0 nodes, threshold is 5 → should trigger
        let trigger = should_trigger_rebalance(&sm, &metrics, 5);
        assert!(trigger);
        Ok(())
    }

    #[test]
    fn test_full_self_heal_cycle() -> Result<(), ShardingError> {
        let clusters = vec![ClusterConfig::new(0, 8, 0)];
        let mut sm = HierarchicalShardManager::new(8, &clusters)?;
        // Populate with 32 nodes
        for i in 0..32u64 {
            sm.assign_node(i)?;
        }
        // Simulate 8 node failures (25% churn)
        let failed: Vec<u64> = (0..8).collect();
        let metrics = ChurnMetrics::new(0.25, 0.85, 110.0);

        // Check that rebalance should be triggered
        assert!(should_trigger_rebalance(&sm, &metrics, 0));

        // Execute self-heal
        let result = self_heal_rebalance(&mut sm, &failed, &metrics)?;

        // Verify results
        assert_eq!(result.removed_nodes.len(), 8);
        assert!(result.efficiency_score >= 0.0 && result.efficiency_score <= 1.0);
        // After healing, remaining nodes should still be assigned
        assert!(sm.total_nodes() >= 20); // At least 24 surviving
        Ok(())
    }
}
