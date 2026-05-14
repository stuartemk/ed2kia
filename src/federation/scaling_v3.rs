//! Federation Scaling v3 — Escalado adaptativo de federación con sharding dinámico
//!
//! Motor de escalado que ajusta la topología de federación basado en capacidad
//! de nodo, latencia histórica y carga de trabajo. Integra con marketplace_v3
//! para descubrimiento de recursos y reputation_matcher para scoring ponderado.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use tracing::info;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for federation scaling v3.
#[derive(Debug, Error)]
pub enum ScalingV3Error {
    /// Node not found in federation.
    #[error("Node {0} not found")]
    NodeNotFound(String),
    /// Invalid capacity value.
    #[error("Invalid capacity: {0}")]
    InvalidCapacity(String),
    /// Shard assignment failed.
    #[error("Shard assignment failed: {0}")]
    ShardAssignmentFailed(String),
    /// Scaling threshold not met.
    #[error("Scaling threshold not met: current={current}, threshold={threshold}")]
    ThresholdNotMet { current: f64, threshold: f64 },
    /// Federation overloaded.
    #[error("Federation overloaded: load_factor={0:.3}")]
    Overloaded(f64),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Node capability profile for scaling decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilityV3 {
    /// Unique node identifier.
    pub node_id: String,
    /// Available compute capacity (FLOPS).
    pub compute_capacity: f64,
    /// Available memory (GB).
    pub memory_gb: f64,
    /// Available VRAM (GB).
    pub vram_gb: f64,
    /// Network bandwidth (Mbps).
    pub bandwidth_mbps: f64,
    /// Current load factor (0.0-1.0).
    pub load_factor: f64,
    /// Average latency to peers (ms).
    pub avg_latency_ms: f64,
    /// Historical uptime (0.0-1.0).
    pub uptime: f64,
    /// Reputation score from matcher.
    pub reputation: f64,
    /// Last heartbeat timestamp (ms).
    pub last_heartbeat_ms: u64,
    /// Assigned shard IDs.
    pub assigned_shards: Vec<String>,
}

impl NodeCapabilityV3 {
    pub fn new(
        node_id: String,
        compute_capacity: f64,
        memory_gb: f64,
        vram_gb: f64,
        bandwidth_mbps: f64,
    ) -> Self {
        Self {
            node_id,
            compute_capacity,
            memory_gb,
            vram_gb,
            bandwidth_mbps,
            load_factor: 0.0,
            avg_latency_ms: 0.0,
            uptime: 1.0,
            reputation: 0.5,
            last_heartbeat_ms: current_timestamp_ms(),
            assigned_shards: Vec::new(),
        }
    }

    /// Computes a composite scoring for node suitability.
    pub fn scaling_score(&self) -> f64 {
        let capacity_score = self.compute_capacity / (1.0 + self.load_factor);
        let latency_penalty = 1.0 / (1.0 + self.avg_latency_ms / 100.0);
        let reputation_factor = self.reputation;
        let uptime_factor = self.uptime;

        (capacity_score * latency_penalty * reputation_factor * uptime_factor).min(1.0)
    }

    /// Checks if node can accept more work.
    pub fn can_accept_work(&self, min_available: f64) -> bool {
        (1.0 - self.load_factor) >= min_available
    }

    /// Checks if node is stale.
    pub fn is_stale(&self, max_stale_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_heartbeat_ms) > max_stale_ms
    }

    /// Updates heartbeat.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat_ms = current_timestamp_ms();
    }
}

/// Shard configuration for distributed workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfigV3 {
    /// Shard identifier.
    pub shard_id: String,
    /// Assigned nodes.
    pub nodes: Vec<String>,
    /// Shard capacity (total compute).
    pub capacity: f64,
    /// Shard load factor.
    pub load_factor: f64,
    /// Shard health score (0.0-1.0).
    pub health_score: f64,
    /// Created timestamp (ms).
    pub created_at_ms: u64,
    /// Last rebalance timestamp (ms).
    pub last_rebalance_ms: u64,
}

impl ShardConfigV3 {
    pub fn new(shard_id: String) -> Self {
        Self {
            shard_id,
            nodes: Vec::new(),
            capacity: 0.0,
            load_factor: 0.0,
            health_score: 1.0,
            created_at_ms: current_timestamp_ms(),
            last_rebalance_ms: current_timestamp_ms(),
        }
    }

    /// Adds a node to this shard.
    pub fn add_node(&mut self, node_id: String, capacity: f64) {
        self.nodes.push(node_id);
        self.capacity += capacity;
    }

    /// Removes a node from this shard.
    pub fn remove_node(&mut self, node_id: &str, capacity: f64) {
        self.nodes.retain(|n| n != node_id);
        self.capacity = (self.capacity - capacity).max(0.0);
    }

    /// Updates load factor.
    pub fn update_load(&mut self, load_factor: f64) {
        self.load_factor = load_factor.clamp(0.0, 1.0);
        self.health_score = (1.0 - self.load_factor).max(0.0);
    }

    /// Checks if shard needs rebalancing.
    pub fn needs_rebalance(&self, threshold: f64) -> bool {
        self.load_factor > threshold
    }
}

/// Scaling decision produced by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingDecisionV3 {
    /// Decision type.
    pub decision_type: ScalingDecisionType,
    /// Target node or shard.
    pub target: String,
    /// Reason for decision.
    pub reason: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
    /// Estimated impact on latency (ms).
    pub estimated_latency_impact_ms: f64,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

impl ScalingDecisionV3 {
    pub fn new(decision_type: ScalingDecisionType, target: String, reason: String, confidence: f64) -> Self {
        Self {
            decision_type,
            target,
            reason,
            confidence,
            estimated_latency_impact_ms: 0.0,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

/// Types of scaling decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingDecisionType {
    /// Add new shard.
    AddShard,
    /// Remove shard.
    RemoveShard,
    /// Rebalance nodes across shards.
    Rebalance,
    /// Scale up node capacity.
    ScaleUp,
    /// Scale down node capacity.
    ScaleDown,
    /// No action needed.
    NoOp,
}

impl std::fmt::Display for ScalingDecisionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalingDecisionType::AddShard => write!(f, "AddShard"),
            ScalingDecisionType::RemoveShard => write!(f, "RemoveShard"),
            ScalingDecisionType::Rebalance => write!(f, "Rebalance"),
            ScalingDecisionType::ScaleUp => write!(f, "ScaleUp"),
            ScalingDecisionType::ScaleDown => write!(f, "ScaleDown"),
            ScalingDecisionType::NoOp => write!(f, "NoOp"),
        }
    }
}

/// Statistics for the scaling engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingStatsV3 {
    /// Total scaling decisions made.
    pub total_decisions: usize,
    /// Total shards created.
    pub total_shards_created: usize,
    /// Total shards removed.
    pub total_shards_removed: usize,
    /// Total rebalances.
    pub total_rebalances: usize,
    /// Average decision time (ms).
    pub avg_decision_ms: f64,
    /// Current active shards.
    pub active_shards: usize,
    /// Current active nodes.
    pub active_nodes: usize,
    /// Federation load factor.
    pub federation_load_factor: f64,
}

impl Default for ScalingStatsV3 {
    fn default() -> Self {
        Self {
            total_decisions: 0,
            total_shards_created: 0,
            total_shards_removed: 0,
            total_rebalances: 0,
            avg_decision_ms: 0.0,
            active_shards: 0,
            active_nodes: 0,
            federation_load_factor: 0.0,
        }
    }
}

/// Configuration for federation scaling v3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingV3Config {
    /// Maximum shards allowed.
    pub max_shards: usize,
    /// Minimum nodes per shard.
    pub min_nodes_per_shard: usize,
    /// Load threshold for scaling up.
    pub scale_up_threshold: f64,
    /// Load threshold for scaling down.
    pub scale_down_threshold: f64,
    /// Rebalance threshold.
    pub rebalance_threshold: f64,
    /// Maximum node staleness (ms).
    pub max_stale_ms: u64,
    /// Decision history size.
    pub decision_history_size: usize,
    /// Target load factor.
    pub target_load_factor: f64,
}

impl Default for ScalingV3Config {
    fn default() -> Self {
        Self {
            max_shards: 16,
            min_nodes_per_shard: 3,
            scale_up_threshold: 0.75,
            scale_down_threshold: 0.25,
            rebalance_threshold: 0.6,
            max_stale_ms: 30_000,
            decision_history_size: 500,
            target_load_factor: 0.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Federation scaling engine v3.
pub struct FederationScalingV3 {
    /// Configuration.
    pub config: ScalingV3Config,
    /// Registered nodes.
    pub nodes: HashMap<String, NodeCapabilityV3>,
    /// Active shards.
    pub shards: HashMap<String, ShardConfigV3>,
    /// Scaling statistics.
    pub stats: ScalingStatsV3,
    /// Decision history.
    pub decision_history: VecDeque<ScalingDecisionV3>,
    /// Next shard sequence number.
    pub next_shard_seq: u64,
}

impl FederationScalingV3 {
    /// Creates a new scaling engine with default config.
    pub fn new() -> Self {
        Self::with_config(ScalingV3Config::default())
    }

    /// Creates a new scaling engine with custom config.
    pub fn with_config(config: ScalingV3Config) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            shards: HashMap::new(),
            stats: ScalingStatsV3::default(),
            decision_history: VecDeque::with_capacity(500),
            next_shard_seq: 0,
        }
    }

    /// Registers a node in the federation.
    pub fn register_node(&mut self, node: NodeCapabilityV3) {
        let node_id = node.node_id.clone();
        self.nodes.insert(node_id, node);
        self.stats.active_nodes = self.nodes.len();
    }

    /// Removes a node from the federation.
    pub fn unregister_node(&mut self, node_id: &str) -> Result<(), ScalingV3Error> {
        let node = self.nodes.get(node_id).ok_or(ScalingV3Error::NodeNotFound(node_id.to_string()))?;
        let capacity = node.compute_capacity;

        // Remove from assigned shards
        for shard in self.shards.values_mut() {
            shard.remove_node(node_id, capacity);
        }

        self.nodes.remove(node_id);
        self.stats.active_nodes = self.nodes.len();
        Ok(())
    }

    /// Updates node heartbeat and metrics.
    pub fn update_node(&mut self, node_id: &str, load_factor: f64, latency_ms: f64) -> Result<(), ScalingV3Error> {
        let node = self.nodes.get_mut(node_id).ok_or(ScalingV3Error::NodeNotFound(node_id.to_string()))?;
        node.heartbeat();
        node.load_factor = load_factor.clamp(0.0, 1.0);
        node.avg_latency_ms = latency_ms;
        Ok(())
    }

    /// Updates node reputation.
    pub fn update_reputation(&mut self, node_id: &str, reputation: f64) -> Result<(), ScalingV3Error> {
        let node = self.nodes.get_mut(node_id).ok_or(ScalingV3Error::NodeNotFound(node_id.to_string()))?;
        node.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    /// Runs the scaling evaluation cycle.
    pub fn evaluate(&mut self) -> Vec<ScalingDecisionV3> {
        let start = std::time::Instant::now();
        let mut decisions = Vec::new();

        // Remove stale nodes
        let stale_nodes: Vec<String> = self.nodes.values()
            .filter(|n| n.is_stale(self.config.max_stale_ms))
            .map(|n| n.node_id.clone())
            .collect();
        for node_id in stale_nodes {
            let _ = self.unregister_node(&node_id);
            decisions.push(ScalingDecisionV3::new(
                ScalingDecisionType::ScaleDown,
                node_id,
                "Node stale".to_string(),
                1.0,
            ));
        }

        // Compute federation load
        let federation_load = self.compute_federation_load();
        self.stats.federation_load_factor = federation_load;

        // Check if we need to add shards
        if federation_load > self.config.scale_up_threshold
            && self.shards.len() < self.config.max_shards {
            let decision = self.create_shard();
            decisions.push(decision);
        }

        // Check if we need to remove empty shards
        let empty_shards: Vec<String> = self.shards.iter()
            .filter(|(_, s)| s.nodes.is_empty())
            .map(|(id, _)| id.clone())
            .collect();
        for shard_id in empty_shards {
            self.shards.remove(&shard_id);
            self.stats.total_shards_removed += 1;
            decisions.push(ScalingDecisionV3::new(
                ScalingDecisionType::RemoveShard,
                shard_id,
                "Empty shard".to_string(),
                1.0,
            ));
        }

        // Check for rebalancing
        for (shard_id, shard) in &self.shards {
            if shard.needs_rebalance(self.config.rebalance_threshold) {
                decisions.push(ScalingDecisionV3::new(
                    ScalingDecisionType::Rebalance,
                    shard_id.clone(),
                    format!("Load {:.2} exceeds threshold {:.2}", shard.load_factor, self.config.rebalance_threshold),
                    0.8,
                ));
            }
        }

        // Check if we can scale down
        if federation_load < self.config.scale_down_threshold && self.shards.len() > 1 {
            decisions.push(ScalingDecisionV3::new(
                ScalingDecisionType::ScaleDown,
                "federation".to_string(),
                format!("Load {:.3} below threshold {:.2}", federation_load, self.config.scale_down_threshold),
                0.7,
            ));
        }

        // If no decisions needed
        if decisions.is_empty() {
            decisions.push(ScalingDecisionV3::new(
                ScalingDecisionType::NoOp,
                "federation".to_string(),
                format!("Load {:.3} within acceptable range", federation_load),
                1.0,
            ));
        }

        // Record decisions
        let elapsed_ms = start.elapsed().as_micros() as f64 / 1000.0;
        self.stats.total_decisions += decisions.len();
        let alpha = 0.1;
        self.stats.avg_decision_ms = alpha * elapsed_ms + (1.0 - alpha) * self.stats.avg_decision_ms;
        self.stats.active_shards = self.shards.len();

        for decision in &decisions {
            match &decision.decision_type {
                ScalingDecisionType::AddShard => self.stats.total_shards_created += 1,
                ScalingDecisionType::RemoveShard => self.stats.total_shards_removed += 1,
                ScalingDecisionType::Rebalance => self.stats.total_rebalances += 1,
                _ => {}
            }
            self.decision_history.push_back(decision.clone());
        }

        // Trim history
        while self.decision_history.len() > self.config.decision_history_size {
            self.decision_history.pop_front();
        }

        info!("FederationScalingV3: evaluation complete (decisions={}, load={:.3})", decisions.len(), federation_load);
        decisions
    }

    /// Assigns a node to the best shard.
    pub fn assign_node_to_shard(&mut self, node_id: &str) -> Result<String, ScalingV3Error> {
        // Extract node capacity before mutable borrows
        let node_capacity = self.nodes.get(node_id)
            .ok_or(ScalingV3Error::NodeNotFound(node_id.to_string()))?
            .compute_capacity;

        // Find shard with lowest load
        let best_shard = self.shards.iter()
            .min_by(|(_, a), (_, b)| a.load_factor.partial_cmp(&b.load_factor).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id.clone());

        match best_shard {
            Some(shard_id) => {
                if let Some(shard) = self.shards.get_mut(&shard_id) {
                    shard.add_node(node_id.to_string(), node_capacity);
                }
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.assigned_shards.push(shard_id.clone());
                }
                Ok(shard_id)
            }
            None => {
                // Create new shard
                let decision = self.create_shard();
                let shard_id = decision.target.clone();
                if let Some(shard) = self.shards.get_mut(&shard_id) {
                    shard.add_node(node_id.to_string(), node_capacity);
                }
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.assigned_shards.push(shard_id.clone());
                }
                Ok(shard_id)
            }
        }
    }

    /// Gets the best node for a workload.
    pub fn select_best_node(&self, min_available: f64) -> Option<&NodeCapabilityV3> {
        self.nodes.values()
            .filter(|n| n.can_accept_work(min_available))
            .max_by(|a, b| a.scaling_score().partial_cmp(&b.scaling_score()).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Computes federation-wide load factor.
    fn compute_federation_load(&self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        let total_load: f64 = self.nodes.values().map(|n| n.load_factor).sum();
        total_load / self.nodes.len() as f64
    }

    /// Creates a new shard.
    fn create_shard(&mut self) -> ScalingDecisionV3 {
        let shard_id = format!("shard-{}", self.next_shard_seq);
        self.next_shard_seq += 1;
        self.shards.insert(shard_id.clone(), ShardConfigV3::new(shard_id.clone()));
        self.stats.total_shards_created += 1;
        ScalingDecisionV3::new(
            ScalingDecisionType::AddShard,
            shard_id,
            format!("Federation load {:.3} exceeds threshold {:.2}", self.stats.federation_load_factor, self.config.scale_up_threshold),
            0.9,
        )
    }

    /// Gets scaling statistics.
    pub fn get_stats(&self) -> ScalingStatsV3 {
        self.stats.clone()
    }

    /// Gets recent decisions.
    pub fn get_recent_decisions(&self, limit: usize) -> Vec<&ScalingDecisionV3> {
        self.decision_history.iter().rev().take(limit).collect()
    }

    /// Gets a node by ID.
    pub fn get_node(&self, node_id: &str) -> Option<&NodeCapabilityV3> {
        self.nodes.get(node_id)
    }

    /// Gets a shard by ID.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardConfigV3> {
        self.shards.get(shard_id)
    }

    /// Gets all active shards.
    pub fn active_shards(&self) -> Vec<&ShardConfigV3> {
        self.shards.values().collect()
    }

    /// Gets all active nodes.
    pub fn active_nodes(&self) -> Vec<&NodeCapabilityV3> {
        self.nodes.values().collect()
    }
}

impl Default for FederationScalingV3 {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes a deterministic shard assignment hash.
pub fn compute_shard_hash(node_id: &str, shard_count: usize) -> usize {
    let mut hasher = Sha256::new();
    hasher.update(node_id.as_bytes());
    let result = hasher.finalize();
    let mut hash_u64 = [0u8; 8];
    hash_u64.copy_from_slice(&result[..8]);
    let value = u64::from_le_bytes(hash_u64);
    (value % shard_count as u64) as usize
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, capacity: f64) -> NodeCapabilityV3 {
        NodeCapabilityV3::new(id.to_string(), capacity, 16.0, 8.0, 1000.0)
    }

    #[test]
    fn test_scaling_creation() {
        let scaling = FederationScalingV3::new();
        assert_eq!(scaling.stats.active_nodes, 0);
        assert_eq!(scaling.stats.active_shards, 0);
    }

    #[test]
    fn test_register_node() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        assert_eq!(scaling.stats.active_nodes, 1);
    }

    #[test]
    fn test_unregister_node() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        assert!(scaling.unregister_node("node-1").is_ok());
        assert_eq!(scaling.stats.active_nodes, 0);
    }

    #[test]
    fn test_unregister_missing_node() {
        let mut scaling = FederationScalingV3::new();
        assert!(scaling.unregister_node("missing").is_err());
    }

    #[test]
    fn test_update_node() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        assert!(scaling.update_node("node-1", 0.5, 10.0).is_ok());
        let node = scaling.get_node("node-1").unwrap();
        assert!((node.load_factor - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_update_reputation() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        assert!(scaling.update_reputation("node-1", 0.95).is_ok());
        let node = scaling.get_node("node-1").unwrap();
        assert!((node.reputation - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_empty() {
        let mut scaling = FederationScalingV3::new();
        let decisions = scaling.evaluate();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision_type, ScalingDecisionType::NoOp);
    }

    #[test]
    fn test_evaluate_scale_up() {
        let config = ScalingV3Config {
            scale_up_threshold: 0.5,
            ..Default::default()
        };
        let mut scaling = FederationScalingV3::with_config(config);
        scaling.register_node(make_node("node-1", 100.0));
        scaling.update_node("node-1", 0.8, 10.0).unwrap();

        let decisions = scaling.evaluate();
        let has_add_shard = decisions.iter().any(|d| d.decision_type == ScalingDecisionType::AddShard);
        assert!(has_add_shard);
    }

    #[test]
    fn test_assign_node_to_shard() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        let shard_id = scaling.assign_node_to_shard("node-1").unwrap();
        assert!(!shard_id.is_empty());
    }

    #[test]
    fn test_select_best_node() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        scaling.register_node(make_node("node-2", 200.0));
        scaling.update_node("node-1", 0.9, 50.0).unwrap();
        scaling.update_node("node-2", 0.3, 10.0).unwrap();

        let best = scaling.select_best_node(0.3);
        assert!(best.is_some());
        assert_eq!(best.unwrap().node_id, "node-2");
    }

    #[test]
    fn test_scaling_score() {
        let mut node = make_node("node-1", 200.0);
        node.load_factor = 0.5;
        node.avg_latency_ms = 20.0;
        node.reputation = 0.9;
        let score = node.scaling_score();
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_can_accept_work() {
        let mut node = make_node("node-1", 100.0);
        node.load_factor = 0.3;
        assert!(node.can_accept_work(0.4));
        assert!(!node.can_accept_work(0.8));
    }

    #[test]
    fn test_shard_config() {
        let mut shard = ShardConfigV3::new("shard-0".to_string());
        shard.add_node("node-1".to_string(), 100.0);
        assert_eq!(shard.nodes.len(), 1);
        assert!((shard.capacity - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_shard_remove_node() {
        let mut shard = ShardConfigV3::new("shard-0".to_string());
        shard.add_node("node-1".to_string(), 100.0);
        shard.remove_node("node-1", 50.0);
        assert_eq!(shard.nodes.len(), 0);
        assert!((shard.capacity - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_shard_needs_rebalance() {
        let mut shard = ShardConfigV3::new("shard-0".to_string());
        shard.update_load(0.7);
        assert!(shard.needs_rebalance(0.6));
        assert!(!shard.needs_rebalance(0.8));
    }

    #[test]
    fn test_shard_health_score() {
        let mut shard = ShardConfigV3::new("shard-0".to_string());
        shard.update_load(0.3);
        assert!((shard.health_score - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_decision_type_display() {
        assert_eq!(format!("{}", ScalingDecisionType::AddShard), "AddShard");
        assert_eq!(format!("{}", ScalingDecisionType::NoOp), "NoOp");
    }

    #[test]
    fn test_stats_default() {
        let stats = ScalingStatsV3::default();
        assert_eq!(stats.total_decisions, 0);
        assert_eq!(stats.active_shards, 0);
    }

    #[test]
    fn test_config_default() {
        let config = ScalingV3Config::default();
        assert_eq!(config.max_shards, 16);
        assert!((config.scale_up_threshold - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_scaling_default() {
        let scaling = FederationScalingV3::default();
        assert_eq!(scaling.stats.active_nodes, 0);
    }

    #[test]
    fn test_compute_shard_hash() {
        let hash1 = compute_shard_hash("node-1", 4);
        let hash2 = compute_shard_hash("node-1", 4);
        assert_eq!(hash1, hash2);
        assert!(hash1 < 4);
    }

    #[test]
    fn test_shard_hash_distribution() {
        let mut counts = vec![0usize; 8];
        for i in 0..100 {
            let shard = compute_shard_hash(&format!("node-{}", i), 8);
            counts[shard] += 1;
        }
        // Check no shard is completely empty
        assert!(counts.iter().all(|&c| c > 0));
    }

    #[test]
    fn test_get_stats() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        let stats = scaling.get_stats();
        assert_eq!(stats.active_nodes, 1);
    }

    #[test]
    fn test_get_recent_decisions() {
        let mut scaling = FederationScalingV3::new();
        scaling.evaluate();
        let decisions = scaling.get_recent_decisions(10);
        assert_eq!(decisions.len(), 1);
    }

    #[test]
    fn test_active_shards() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        scaling.assign_node_to_shard("node-1").unwrap();
        let shards = scaling.active_shards();
        assert_eq!(shards.len(), 1);
    }

    #[test]
    fn test_active_nodes() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        scaling.register_node(make_node("node-2", 200.0));
        let nodes = scaling.active_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_node_heartbeat() {
        let mut node = make_node("node-1", 100.0);
        let old_ts = node.last_heartbeat_ms;
        std::thread::sleep(std::time::Duration::from_millis(10));
        node.heartbeat();
        assert!(node.last_heartbeat_ms >= old_ts);
    }

    #[test]
    fn test_node_not_stale() {
        let node = make_node("node-1", 100.0);
        assert!(!node.is_stale(30_000));
    }

    #[test]
    fn test_multiple_nodes_in_shard() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        scaling.register_node(make_node("node-2", 200.0));
        let shard_id = scaling.assign_node_to_shard("node-1").unwrap();
        scaling.assign_node_to_shard("node-2").unwrap();
        let shard = scaling.get_shard(&shard_id).unwrap();
        assert_eq!(shard.nodes.len(), 2);
    }

    #[test]
    fn test_error_display() {
        let err = ScalingV3Error::NodeNotFound("node-1".to_string());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_threshold_not_met_error() {
        let err = ScalingV3Error::ThresholdNotMet { current: 0.3, threshold: 0.5 };
        assert!(format!("{}", err).contains("0.3"));
    }

    #[test]
    fn test_overloaded_error() {
        let err = ScalingV3Error::Overloaded(0.95);
        assert!(format!("{}", err).contains("0.950"));
    }

    #[test]
    fn test_reputation_clamping() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        scaling.update_reputation("node-1", 1.5).unwrap();
        let node = scaling.get_node("node-1").unwrap();
        assert!((node.reputation - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_load_clamping() {
        let mut scaling = FederationScalingV3::new();
        scaling.register_node(make_node("node-1", 100.0));
        scaling.update_node("node-1", 1.5, 10.0).unwrap();
        let node = scaling.get_node("node-1").unwrap();
        assert!((node.load_factor - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_scaling_decision_creation() {
        let decision = ScalingDecisionV3::new(
            ScalingDecisionType::AddShard,
            "shard-0".to_string(),
            "Test".to_string(),
            0.9,
        );
        assert_eq!(decision.decision_type, ScalingDecisionType::AddShard);
        assert!((decision.confidence - 0.9).abs() < 1e-10);
    }
}
