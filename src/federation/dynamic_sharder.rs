//! Dynamic Sharder — Sharding adaptativo basado en capacidad, latencia y reputación.
//!
//! Motor de partición dinámica que ajusta la distribución de shards en la federación
//! basado en métricas en tiempo real: capacidad de cómputo, latencia histórica y
//! reputación técnica. Soporta rebalanceo automático con mínimo downtime.
//!
//! Feature-gated: `#[cfg(feature = "v1.3-sprint3")]`

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for dynamic sharding.
#[derive(Debug, Error)]
pub enum DynamicSharderError {
    /// Node not found in federation.
    #[error("Node {0} not found")]
    NodeNotFound(String),
    /// Shard not found.
    #[error("Shard {0} not found")]
    ShardNotFound(String),
    /// Invalid shard count.
    #[error("Invalid shard count: {0}")]
    InvalidShardCount(String),
    /// Rebalancing failed.
    #[error("Rebalancing failed: {0}")]
    RebalanceFailed(String),
    /// Insufficient capacity.
    #[error("Insufficient capacity: available={available}, required={required}")]
    InsufficientCapacity { available: f64, required: f64 },
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for the dynamic sharder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSharderConfig {
    /// Maximum number of shards allowed.
    pub max_shards: usize,
    /// Minimum nodes per shard.
    pub min_nodes_per_shard: usize,
    /// Rebalance threshold (load imbalance ratio).
    pub rebalance_threshold: f64,
    /// Latency weight in scoring.
    pub latency_weight: f64,
    /// Reputation weight in scoring.
    pub reputation_weight: f64,
    /// Capacity weight in scoring.
    pub capacity_weight: f64,
    /// Maximum rebalance migrations per cycle.
    pub max_migrations_per_cycle: usize,
    /// Decision timeout in milliseconds.
    pub decision_timeout_ms: u64,
}

impl Default for DynamicSharderConfig {
    fn default() -> Self {
        Self {
            max_shards: 64,
            min_nodes_per_shard: 2,
            rebalance_threshold: 0.2,
            latency_weight: 0.3,
            reputation_weight: 0.4,
            capacity_weight: 0.3,
            max_migrations_per_cycle: 5,
            decision_timeout_ms: 70,
        }
    }
}

/// Node metrics for sharding decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardNodeMetrics {
    /// Unique node identifier.
    pub node_id: String,
    /// Available compute capacity (normalized 0-1).
    pub capacity: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Technical reputation score (0-1).
    pub reputation: f64,
    /// Current load factor (0-1).
    pub current_load: f64,
    /// Number of shards assigned.
    pub shard_count: usize,
    /// Last heartbeat timestamp in milliseconds.
    pub last_heartbeat_ms: u64,
}

impl ShardNodeMetrics {
    /// Create new node metrics.
    pub fn new(node_id: String, capacity: f64, avg_latency_ms: f64, reputation: f64) -> Self {
        Self {
            node_id,
            capacity,
            avg_latency_ms,
            reputation,
            current_load: 0.0,
            shard_count: 0,
            last_heartbeat_ms: current_timestamp_ms(),
        }
    }

    /// Compute composite score for shard assignment.
    pub fn composite_score(&self, config: &DynamicSharderConfig) -> f64 {
        let latency_score = 1.0 - (self.avg_latency_ms / 500.0).clamp(0.0, 1.0);
        let capacity_score = self.capacity.clamp(0.0, 1.0);
        let reputation_score = self.reputation.clamp(0.0, 1.0);

        config.latency_weight * latency_score
            + config.capacity_weight * capacity_score
            + config.reputation_weight * reputation_score
    }

    /// Check if node is healthy (recent heartbeat).
    pub fn is_healthy(&self, timeout_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_heartbeat_ms) < timeout_ms
    }
}

/// Shard assignment entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardAssignment {
    /// Shard identifier.
    pub shard_id: String,
    /// Assigned node IDs.
    pub nodes: Vec<String>,
    /// Shard load factor (0-1).
    pub load_factor: f64,
    /// Shard creation timestamp.
    pub created_ms: u64,
    /// Last rebalance timestamp.
    pub last_rebalance_ms: u64,
}

impl ShardAssignment {
    /// Create new shard assignment.
    pub fn new(shard_id: String, nodes: Vec<String>) -> Self {
        Self {
            shard_id,
            nodes,
            load_factor: 0.0,
            created_ms: current_timestamp_ms(),
            last_rebalance_ms: 0,
        }
    }

    /// Check if node is assigned to this shard.
    pub fn contains_node(&self, node_id: &str) -> bool {
        self.nodes.contains(&node_id.to_string())
    }

    /// Add node to shard.
    pub fn add_node(&mut self, node_id: String) {
        if !self.contains_node(&node_id) {
            self.nodes.push(node_id);
        }
    }

    /// Remove node from shard.
    pub fn remove_node(&mut self, node_id: &str) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|n| n != node_id);
        self.nodes.len() < before
    }
}

/// Rebalance action to migrate a node between shards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceAction {
    /// Node to migrate.
    pub node_id: String,
    /// Source shard.
    pub from_shard: String,
    /// Target shard.
    pub to_shard: String,
    /// Reason for migration.
    pub reason: String,
    /// Estimated cost (latency impact).
    pub estimated_cost_ms: f64,
}

/// Rebalance plan with ordered actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancePlan {
    /// Plan identifier.
    pub plan_id: String,
    /// Actions in execution order.
    pub actions: Vec<RebalanceAction>,
    /// Total estimated cost.
    pub total_cost_ms: f64,
    /// Creation timestamp.
    pub created_ms: u64,
}

impl RebalancePlan {
    /// Create empty plan.
    pub fn new(plan_id: String) -> Self {
        Self {
            plan_id,
            actions: Vec::new(),
            total_cost_ms: 0.0,
            created_ms: current_timestamp_ms(),
        }
    }

    /// Add action to plan.
    pub fn add_action(&mut self, action: RebalanceAction) {
        self.total_cost_ms += action.estimated_cost_ms;
        self.actions.push(action);
    }

    /// Check if plan is empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Statistics for dynamic sharding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharderStats {
    /// Total shards created.
    pub total_shards: usize,
    /// Total rebalances performed.
    pub total_rebalances: usize,
    /// Total migrations executed.
    pub total_migrations: usize,
    /// Average decision time in milliseconds.
    pub avg_decision_time_ms: f64,
    /// Current load imbalance.
    pub current_imbalance: f64,
    /// Last rebalance timestamp.
    pub last_rebalance_ms: u64,
}

impl Default for SharderStats {
    fn default() -> Self {
        Self {
            total_shards: 0,
            total_rebalances: 0,
            total_migrations: 0,
            avg_decision_time_ms: 0.0,
            current_imbalance: 0.0,
            last_rebalance_ms: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Core Engine
// ---------------------------------------------------------------------------

/// Dynamic sharder engine.
pub struct DynamicSharder {
    /// Configuration.
    pub config: DynamicSharderConfig,
    /// Node metrics registry.
    pub nodes: HashMap<String, ShardNodeMetrics>,
    /// Shard assignments.
    pub shards: BTreeMap<String, ShardAssignment>,
    /// Statistics.
    pub stats: SharderStats,
    /// Decision history.
    pub decision_history: VecDeque<f64>,
}

impl DynamicSharder {
    /// Create new dynamic sharder with default config.
    pub fn new() -> Self {
        Self::with_config(DynamicSharderConfig::default())
    }

    /// Create sharder with custom config.
    pub fn with_config(config: DynamicSharderConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            shards: BTreeMap::new(),
            stats: SharderStats::default(),
            decision_history: VecDeque::new(),
        }
    }

    // ── Node Management ──

    /// Register a node with metrics.
    pub fn register_node(
        &mut self,
        node_id: String,
        capacity: f64,
        avg_latency_ms: f64,
        reputation: f64,
    ) -> Result<(), DynamicSharderError> {
        let metrics = ShardNodeMetrics::new(node_id.clone(), capacity, avg_latency_ms, reputation);
        self.nodes.insert(node_id, metrics);
        Ok(())
    }

    /// Update node metrics.
    pub fn update_node_metrics(
        &mut self,
        node_id: &str,
        capacity: Option<f64>,
        avg_latency_ms: Option<f64>,
        reputation: Option<f64>,
        current_load: Option<f64>,
    ) -> Result<(), DynamicSharderError> {
        let node = self.nodes.get_mut(node_id)
            .ok_or(DynamicSharderError::NodeNotFound(node_id.to_string()))?;

        if let Some(cap) = capacity {
            node.capacity = cap.clamp(0.0, 1.0);
        }
        if let Some(lat) = avg_latency_ms {
            node.avg_latency_ms = lat;
        }
        if let Some(rep) = reputation {
            node.reputation = rep.clamp(0.0, 1.0);
        }
        if let Some(load) = current_load {
            node.current_load = load.clamp(0.0, 1.0);
        }
        node.last_heartbeat_ms = current_timestamp_ms();
        Ok(())
    }

    /// Remove node from federation.
    pub fn remove_node(&mut self, node_id: &str) -> Result<(), DynamicSharderError> {
        self.nodes.get(node_id)
            .ok_or(DynamicSharderError::NodeNotFound(node_id.to_string()))?;

        // Remove from all shards
        for shard in self.shards.values_mut() {
            shard.remove_node(node_id);
        }
        self.nodes.remove(node_id);
        Ok(())
    }

    /// Get node metrics.
    pub fn get_node(&self, node_id: &str) -> Option<&ShardNodeMetrics> {
        self.nodes.get(node_id)
    }

    /// Get healthy nodes.
    pub fn healthy_nodes(&self) -> Vec<&ShardNodeMetrics> {
        let timeout = self.config.decision_timeout_ms * 10; // 10x decision timeout
        self.nodes.values()
            .filter(|n| n.is_healthy(timeout))
            .collect()
    }

    // ── Shard Management ──

    /// Create new shard with optimal node assignment.
    pub fn create_shard(&mut self, shard_id: String) -> Result<ShardAssignment, DynamicSharderError> {
        if self.shards.len() >= self.config.max_shards {
            return Err(DynamicSharderError::InvalidShardCount(
                "Maximum shards reached".to_string(),
            ));
        }

        // Select best nodes for new shard
        let mut scored_nodes: Vec<&ShardNodeMetrics> = self.healthy_nodes();
        scored_nodes.sort_by(|a, b| {
            let score_a = (*a).composite_score(&self.config);
            let score_b = (*b).composite_score(&self.config);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Pick nodes with lowest current shard count
        let mut selected = Vec::new();
        for node in scored_nodes {
            if node.shard_count < self.config.min_nodes_per_shard {
                selected.push(node.node_id.clone());
            }
            if selected.len() >= self.config.min_nodes_per_shard {
                break;
            }
        }

        if selected.is_empty() {
            return Err(DynamicSharderError::InsufficientCapacity {
                available: self.nodes.len() as f64,
                required: self.config.min_nodes_per_shard as f64,
            });
        }

        let shard = ShardAssignment::new(shard_id.clone(), selected.clone());

        // Update node shard counts
        for node_id in &selected {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.shard_count += 1;
            }
        }

        self.stats.total_shards += 1;
        self.shards.insert(shard_id, shard.clone());
        Ok(shard)
    }

    /// Remove shard.
    pub fn remove_shard(&mut self, shard_id: &str) -> Result<ShardAssignment, DynamicSharderError> {
        let shard = self.shards.remove(shard_id)
            .ok_or(DynamicSharderError::ShardNotFound(shard_id.to_string()))?;

        // Update node shard counts
        for node_id in &shard.nodes {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.shard_count = node.shard_count.saturating_sub(1);
            }
        }

        Ok(shard)
    }

    /// Get shard assignment.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardAssignment> {
        self.shards.get(shard_id)
    }

    /// Get shards for a node.
    pub fn get_node_shards(&self, node_id: &str) -> Vec<&ShardAssignment> {
        self.shards.values()
            .filter(|s| s.contains_node(node_id))
            .collect()
    }

    // ── Rebalancing ──

    /// Compute load imbalance across shards.
    pub fn compute_imbalance(&self) -> f64 {
        if self.shards.len() < 2 {
            return 0.0;
        }

        let loads: Vec<f64> = self.shards.values()
            .map(|s| s.load_factor)
            .collect();
        let avg: f64 = loads.iter().sum::<f64>() / loads.len() as f64;
        let variance: f64 = loads.iter()
            .map(|l| (l - avg).powi(2))
            .sum::<f64>() / loads.len() as f64;
        variance.sqrt()
    }

    /// Check if rebalancing is needed.
    pub fn needs_rebalance(&mut self) -> bool {
        let imbalance = self.compute_imbalance();
        self.stats.current_imbalance = imbalance;
        imbalance > self.config.rebalance_threshold
    }

    /// Generate rebalance plan.
    pub fn generate_rebalance_plan(&mut self) -> RebalancePlan {
        let plan_id = format!("plan-{}", current_timestamp_ms());
        let mut plan = RebalancePlan::new(plan_id);

        if !self.needs_rebalance() {
            return plan;
        }

        // Find overloaded and underloaded shards
        let loads: Vec<_> = self.shards.values()
            .map(|s| (&s.shard_id, s.load_factor))
            .collect();
        let avg_load: f64 = loads.iter().map(|(_, l)| l).sum::<f64>() / loads.len() as f64;

        let mut overloaded: Vec<_> = loads.iter()
            .filter(|(_, l)| *l > avg_load * (1.0 + self.config.rebalance_threshold))
            .collect();
        let mut underloaded: Vec<_> = loads.iter()
            .filter(|(_, l)| *l < avg_load * (1.0 - self.config.rebalance_threshold))
            .collect();

        overloaded.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        underloaded.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut migrations = 0;
        for (overloaded_id, _) in &overloaded {
            if migrations >= self.config.max_migrations_per_cycle {
                break;
            }

            let shard = self.shards.get(*overloaded_id).unwrap();
            if shard.nodes.is_empty() {
                continue;
            }

            // Find node to migrate (lowest score in overloaded shard)
            let mut node_scores: Vec<_> = shard.nodes.iter()
                .filter_map(|n| self.nodes.get(n).map(|m| (n, m.composite_score(&self.config))))
                .collect();
            node_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((node_id, _)) = node_scores.first() {
                if let Some((target_id, _)) = underloaded.first() {
                    if migrations < self.config.max_migrations_per_cycle {
                        let action = RebalanceAction {
                            node_id: node_id.to_string(),
                            from_shard: overloaded_id.to_string(),
                            to_shard: target_id.to_string(),
                            reason: "Load imbalance".to_string(),
                            estimated_cost_ms: 50.0,
                        };
                        plan.add_action(action);
                        migrations += 1;
                    }
                }
            }
        }

        plan
    }

    /// Execute rebalance plan.
    pub fn execute_rebalance(&mut self, plan: &RebalancePlan) -> Result<usize, DynamicSharderError> {
        let mut executed = 0;

        for action in &plan.actions {
            // Remove from source shard
            if let Some(from_shard) = self.shards.get_mut(&action.from_shard) {
                from_shard.remove_node(&action.node_id);
            }

            // Add to target shard
            if let Some(to_shard) = self.shards.get_mut(&action.to_shard) {
                to_shard.add_node(action.node_id.clone());
            }

            // Update node shard counts
            let node_id = action.node_id.clone();
            let shard_count = self.get_node_shards(&node_id).len();
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.shard_count = shard_count;
            }

            executed += 1;
        }

        if executed > 0 {
            self.stats.total_rebalances += 1;
            self.stats.total_migrations += executed;
            self.stats.last_rebalance_ms = current_timestamp_ms();

            // Update shard rebalance timestamps
            for shard in self.shards.values_mut() {
                shard.last_rebalance_ms = current_timestamp_ms();
            }
        }

        Ok(executed)
    }

    /// Run full rebalance cycle.
    pub fn rebalance_cycle(&mut self) -> Result<usize, DynamicSharderError> {
        let start = current_timestamp_ms();

        let plan = self.generate_rebalance_plan();
        if plan.is_empty() {
            return Ok(0);
        }

        let executed = self.execute_rebalance(&plan)?;

        let elapsed = current_timestamp_ms().saturating_sub(start);
        self.decision_history.push_back(elapsed as f64);
        if self.decision_history.len() > 50 {
            self.decision_history.pop_front();
        }
        self.stats.avg_decision_time_ms = self.decision_history.iter()
            .sum::<f64>() / self.decision_history.len() as f64;

        Ok(executed)
    }

    // ── Assignment ──

    /// Find best shard for a new node.
    pub fn find_best_shard_for_node(&self, node_id: &str) -> Option<String> {
        let _node = self.nodes.get(node_id)?;

        // Find shard with lowest average score
        let mut best_shard = None;
        let mut best_avg = f64::MAX;

        for shard in self.shards.values() {
            let avg: f64 = shard.nodes.iter()
                .filter_map(|n| self.nodes.get(n))
                .map(|m| m.composite_score(&self.config))
                .sum::<f64>() / shard.nodes.len().max(1) as f64;

            if avg < best_avg {
                best_avg = avg;
                best_shard = Some(shard.shard_id.clone());
            }
        }

        best_shard
    }

    /// Assign node to optimal shard.
    pub fn assign_node_to_shard(
        &mut self,
        node_id: &str,
    ) -> Result<String, DynamicSharderError> {
        self.nodes.get(node_id)
            .ok_or(DynamicSharderError::NodeNotFound(node_id.to_string()))?;

        let shard_id = self.find_best_shard_for_node(node_id)
            .ok_or(DynamicSharderError::RebalanceFailed(
                "No shards available".to_string(),
            ))?;

        if let Some(shard) = self.shards.get_mut(&shard_id) {
            shard.add_node(node_id.to_string());
        }

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.shard_count += 1;
        }

        Ok(shard_id)
    }

    // ── Stats ──

    /// Get current stats.
    pub fn get_stats(&self) -> &SharderStats {
        &self.stats
    }

    /// Reset stats.
    pub fn reset_stats(&mut self) {
        self.stats = SharderStats::default();
        self.decision_history.clear();
    }
}

impl Default for DynamicSharder {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Utilities ───

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

    #[test]
    fn test_creation() {
        let sharder = DynamicSharder::new();
        assert_eq!(sharder.nodes.len(), 0);
        assert_eq!(sharder.shards.len(), 0);
    }

    #[test]
    fn test_creation_with_config() {
        let config = DynamicSharderConfig {
            max_shards: 32,
            rebalance_threshold: 0.15,
            ..Default::default()
        };
        let sharder = DynamicSharder::with_config(config);
        assert_eq!(sharder.config.max_shards, 32);
        assert_eq!(sharder.config.rebalance_threshold, 0.15);
    }

    #[test]
    fn test_register_node() {
        let mut sharder = DynamicSharder::new();
        let result = sharder.register_node("node-1".to_string(), 0.8, 50.0, 0.9);
        assert!(result.is_ok());
        assert_eq!(sharder.nodes.len(), 1);
    }

    #[test]
    fn test_update_node_metrics() {
        let mut sharder = DynamicSharder::new();
        sharder.register_node("node-1".to_string(), 0.8, 50.0, 0.9).unwrap();
        let result = sharder.update_node_metrics("node-1", Some(0.9), Some(40.0), Some(0.95), Some(0.5));
        assert!(result.is_ok());
        let node = sharder.get_node("node-1").unwrap();
        assert_eq!(node.capacity, 0.9);
        assert_eq!(node.avg_latency_ms, 40.0);
        assert_eq!(node.reputation, 0.95);
        assert_eq!(node.current_load, 0.5);
    }

    #[test]
    fn test_update_unknown_node() {
        let mut sharder = DynamicSharder::new();
        let result = sharder.update_node_metrics("unknown", None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_composite_score() {
        let node = ShardNodeMetrics::new("n1".to_string(), 0.8, 50.0, 0.9);
        let config = DynamicSharderConfig::default();
        let score = node.composite_score(&config);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_create_shard() {
        let mut sharder = DynamicSharder::new();
        sharder.register_node("n1".to_string(), 0.8, 50.0, 0.9).unwrap();
        sharder.register_node("n2".to_string(), 0.7, 60.0, 0.85).unwrap();

        let result = sharder.create_shard("shard-1".to_string());
        assert!(result.is_ok());
        let shard = result.unwrap();
        assert_eq!(shard.shard_id, "shard-1");
        assert!(!shard.nodes.is_empty());
    }

    #[test]
    fn test_create_shard_no_nodes() {
        let mut sharder = DynamicSharder::new();
        let result = sharder.create_shard("shard-1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_max_shards() {
        let mut sharder = DynamicSharder::new();
        sharder.config.max_shards = 2;
        sharder.register_node("n1".to_string(), 0.8, 50.0, 0.9).unwrap();
        sharder.register_node("n2".to_string(), 0.7, 60.0, 0.85).unwrap();

        sharder.create_shard("s1".to_string()).unwrap();
        sharder.create_shard("s2".to_string()).unwrap();
        let result = sharder.create_shard("s3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_shard() {
        let mut sharder = DynamicSharder::new();
        sharder.register_node("n1".to_string(), 0.8, 50.0, 0.9).unwrap();
        sharder.register_node("n2".to_string(), 0.7, 60.0, 0.85).unwrap();
        sharder.create_shard("shard-1".to_string()).unwrap();

        let result = sharder.remove_shard("shard-1");
        assert!(result.is_ok());
        assert_eq!(sharder.shards.len(), 0);
    }

    #[test]
    fn test_compute_imbalance() {
        let mut sharder = DynamicSharder::new();
        sharder.register_node("n1".to_string(), 0.8, 50.0, 0.9).unwrap();
        sharder.register_node("n2".to_string(), 0.7, 60.0, 0.85).unwrap();
        sharder.create_shard("s1".to_string()).unwrap();

        // Single shard should have 0 imbalance
        let imbalance = sharder.compute_imbalance();
        assert_eq!(imbalance, 0.0);
    }

    #[test]
    fn test_needs_rebalance() {
        let mut sharder = DynamicSharder::new();
        assert!(!sharder.needs_rebalance());
    }

    #[test]
    fn test_generate_rebalance_plan_empty() {
        let mut sharder = DynamicSharder::new();
        let plan = sharder.generate_rebalance_plan();
        assert!(plan.is_empty());
    }

    #[test]
    fn test_rebalance_cycle_empty() {
        let mut sharder = DynamicSharder::new();
        let result = sharder.rebalance_cycle();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_assign_node_to_shard() {
        let mut sharder = DynamicSharder::new();
        sharder.register_node("n1".to_string(), 0.8, 50.0, 0.9).unwrap();
        sharder.register_node("n2".to_string(), 0.7, 60.0, 0.85).unwrap();
        sharder.create_shard("shard-1".to_string()).unwrap();

        let result = sharder.assign_node_to_shard("n1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_node_shards() {
        let mut sharder = DynamicSharder::new();
        sharder.register_node("n1".to_string(), 0.8, 50.0, 0.9).unwrap();
        sharder.create_shard("shard-1".to_string()).unwrap();

        let shards = sharder.get_node_shards("n1");
        assert!(!shards.is_empty());
    }

    #[test]
    fn test_reset_stats() {
        let mut sharder = DynamicSharder::new();
        sharder.stats.total_shards = 5;
        sharder.stats.total_rebalances = 3;
        sharder.reset_stats();
        assert_eq!(sharder.stats.total_shards, 0);
        assert_eq!(sharder.stats.total_rebalances, 0);
    }

    #[test]
    fn test_node_health_check() {
        let node = ShardNodeMetrics::new("n1".to_string(), 0.8, 50.0, 0.9);
        assert!(node.is_healthy(10000));
    }

    #[test]
    fn test_shard_contains_node() {
        let shard = ShardAssignment::new("s1".to_string(), vec!["n1".to_string()]);
        assert!(shard.contains_node("n1"));
        assert!(!shard.contains_node("n2"));
    }

    #[test]
    fn test_shard_remove_node() {
        let mut shard = ShardAssignment::new("s1".to_string(), vec!["n1".to_string(), "n2".to_string()]);
        assert!(shard.remove_node("n1"));
        assert_eq!(shard.nodes.len(), 1);
        assert!(!shard.remove_node("n3"));
    }

    #[test]
    fn test_rebalance_plan() {
        let mut plan = RebalancePlan::new("test-plan".to_string());
        assert!(plan.is_empty());

        plan.add_action(RebalanceAction {
            node_id: "n1".to_string(),
            from_shard: "s1".to_string(),
            to_shard: "s2".to_string(),
            reason: "test".to_string(),
            estimated_cost_ms: 50.0,
        });
        assert!(!plan.is_empty());
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.total_cost_ms, 50.0);
    }

    #[test]
    fn test_error_display() {
        let err = DynamicSharderError::NodeNotFound("x".to_string());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_config_default() {
        let config = DynamicSharderConfig::default();
        assert_eq!(config.max_shards, 64);
        assert_eq!(config.min_nodes_per_shard, 2);
    }

    #[test]
    fn test_stats_default() {
        let stats = SharderStats::default();
        assert_eq!(stats.total_shards, 0);
        assert_eq!(stats.total_rebalances, 0);
    }

    #[test]
    fn test_sharder_default() {
        let sharder = DynamicSharder::default();
        assert_eq!(sharder.nodes.len(), 0);
    }

    #[test]
    fn test_full_lifecycle() {
        let mut sharder = DynamicSharder::new();

        // Register nodes
        sharder.register_node("n1".to_string(), 0.9, 30.0, 0.95).unwrap();
        sharder.register_node("n2".to_string(), 0.8, 40.0, 0.9).unwrap();
        sharder.register_node("n3".to_string(), 0.7, 50.0, 0.85).unwrap();

        // Create shards
        sharder.create_shard("s1".to_string()).unwrap();
        sharder.create_shard("s2".to_string()).unwrap();

        assert_eq!(sharder.shards.len(), 2);
        assert_eq!(sharder.stats.total_shards, 2);

        // Check imbalance
        let imbalance = sharder.compute_imbalance();
        assert!(imbalance >= 0.0);

        // Rebalance
        let result = sharder.rebalance_cycle();
        assert!(result.is_ok());
    }
}
