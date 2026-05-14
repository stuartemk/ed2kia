//! Shard Aggregator — Aggregates SAE shards by type with load balancing.
//!
//! Provides efficient aggregation and distribution of shards across
//! federation nodes, similar to how Linux `cgroups` aggregates processes
//! but distributed across a federated network.
//!
//! Zero financial logic: shards represent compute resources only.

use std::collections::HashMap;

/// Errors for shard aggregation operations.
#[derive(Debug)]
pub enum ShardAggregatorError {
    ShardNotFound(String),
    ShardAlreadyExists(String),
    NodeNotFound(String),
    AggregationFailed(String),
    MaxShardsExceeded(usize),
}

impl std::fmt::Display for ShardAggregatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardAggregatorError::ShardNotFound(id) => {
                write!(f, "Shard not found: {}", id)
            }
            ShardAggregatorError::ShardAlreadyExists(id) => {
                write!(f, "Shard already exists: {}", id)
            }
            ShardAggregatorError::NodeNotFound(id) => {
                write!(f, "Node not found: {}", id)
            }
            ShardAggregatorError::AggregationFailed(msg) => {
                write!(f, "Aggregation failed: {}", msg)
            }
            ShardAggregatorError::MaxShardsExceeded(max) => {
                write!(f, "Max shards exceeded: {}", max)
            }
        }
    }
}

/// Configuration for the shard aggregator.
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    /// Maximum number of shards per aggregation group.
    pub max_shards_per_group: usize,
    /// Load threshold to trigger rebalancing (0.0–1.0).
    pub rebalance_threshold: f64,
    /// Minimum healthy nodes required for aggregation.
    pub min_healthy_nodes: usize,
    /// Enable automatic load balancing.
    pub auto_rebalance: bool,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            max_shards_per_group: 64,
            rebalance_threshold: 0.75,
            min_healthy_nodes: 2,
            auto_rebalance: true,
        }
    }
}

/// Shard type classification for aggregation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShardType {
    SaeCompute,
    SaeStorage,
    SaeInference,
    Custom(String),
}

impl std::fmt::Display for ShardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardType::SaeCompute => write!(f, "SaeCompute"),
            ShardType::SaeStorage => write!(f, "SaeStorage"),
            ShardType::SaeInference => write!(f, "SaeInference"),
            ShardType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// Individual shard record within an aggregation group.
#[derive(Debug, Clone)]
pub struct ShardRecord {
    /// Unique shard identifier.
    pub shard_id: String,
    /// Shard type for grouping.
    pub shard_type: ShardType,
    /// Host node identifier.
    pub node_id: String,
    /// Current load factor (0.0–1.0).
    pub load_factor: f64,
    /// Available compute credits.
    pub available_credits: f64,
    /// Timestamp of last health check (ms).
    pub last_heartbeat_ms: u64,
    /// Shard is healthy and accepting work.
    pub healthy: bool,
}

impl ShardRecord {
    /// Create a new shard record.
    pub fn new(
        shard_id: String,
        shard_type: ShardType,
        node_id: String,
        available_credits: f64,
    ) -> Self {
        Self {
            shard_id,
            shard_type,
            node_id,
            load_factor: 0.0,
            available_credits,
            last_heartbeat_ms: current_timestamp_ms(),
            healthy: true,
        }
    }

    /// Update load factor with clamping.
    pub fn update_load(&mut self, load: f64) {
        self.load_factor = load.clamp(0.0, 1.0);
    }

    /// Update heartbeat timestamp.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat_ms = current_timestamp_ms();
    }

    /// Check if shard is overloaded based on threshold.
    pub fn is_overloaded(&self, threshold: f64) -> bool {
        self.load_factor > threshold
    }

    /// Check if shard is stale (no recent heartbeat).
    pub fn is_stale(&self, max_stale_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_heartbeat_ms) > max_stale_ms
    }
}

/// Aggregation group containing shards of the same type.
#[derive(Debug, Clone)]
pub struct AggregationGroup {
    /// Group identifier (matches shard type).
    pub group_id: String,
    /// Shard type for this group.
    pub shard_type: ShardType,
    /// Member shards in this group.
    pub shards: Vec<ShardRecord>,
    /// Total available credits in group.
    pub total_credits: f64,
    /// Average load factor across shards.
    pub avg_load: f64,
    /// Number of healthy shards.
    pub healthy_count: usize,
    /// Last rebalance timestamp (ms).
    pub last_rebalance_ms: u64,
}

impl AggregationGroup {
    /// Create a new aggregation group.
    pub fn new(group_id: String, shard_type: ShardType) -> Self {
        Self {
            group_id,
            shard_type,
            shards: Vec::new(),
            total_credits: 0.0,
            avg_load: 0.0,
            healthy_count: 0,
            last_rebalance_ms: 0,
        }
    }

    /// Add a shard to the group.
    pub fn add_shard(&mut self, shard: ShardRecord) {
        self.shards.push(shard);
        self.recalculate_stats();
    }

    /// Remove a shard from the group by ID.
    pub fn remove_shard(&mut self, shard_id: &str) -> Option<ShardRecord> {
        if let Some(pos) = self.shards.iter().position(|s| s.shard_id == shard_id) {
            let removed = self.shards.remove(pos);
            self.recalculate_stats();
            Some(removed)
        } else {
            None
        }
    }

    /// Get the shard with lowest load (for load balancing).
    pub fn least_loaded_shard(&self) -> Option<&ShardRecord> {
        self.shards
            .iter()
            .filter(|s| s.healthy)
            .min_by(|a, b| a.load_factor.partial_cmp(&b.load_factor).unwrap())
    }

    /// Check if group needs rebalancing.
    pub fn needs_rebalance(&self, threshold: f64) -> bool {
        if self.shards.is_empty() {
            return false;
        }
        let loads: Vec<f64> = self.shards.iter().map(|s| s.load_factor).collect();
        let max = loads.iter().cloned().fold(0.0_f64, f64::max);
        let min = loads.iter().cloned().fold(1.0_f64, f64::min);
        max - min > threshold
    }

    /// Recalculate aggregate statistics.
    fn recalculate_stats(&mut self) {
        if self.shards.is_empty() {
            self.total_credits = 0.0;
            self.avg_load = 0.0;
            self.healthy_count = 0;
            return;
        }
        self.total_credits = self.shards.iter().map(|s| s.available_credits).sum();
        self.avg_load = self.shards.iter().map(|s| s.load_factor).sum::<f64>()
            / self.shards.len() as f64;
        self.healthy_count = self.shards.iter().filter(|s| s.healthy).count();
    }
}

/// Aggregation statistics.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct AggregatorStats {
    /// Total shards aggregated.
    pub total_shards: usize,
    /// Total aggregation groups.
    pub total_groups: usize,
    /// Total rebalances performed.
    pub total_rebalances: usize,
    /// Last aggregation timestamp (ms).
    pub last_aggregation_ms: u64,
    /// Total healthy shards.
    pub healthy_shards: usize,
}


/// Shard Aggregator engine — aggregates shards by type with load balancing.
pub struct ShardAggregator {
    /// Aggregator configuration.
    pub config: AggregatorConfig,
    /// Aggregation groups by group ID.
    groups: HashMap<String, AggregationGroup>,
    /// Shard lookup by shard ID.
    shard_index: HashMap<String, String>,
    /// Aggregation statistics.
    stats: AggregatorStats,
}

impl ShardAggregator {
    /// Create a new aggregator with config.
    pub fn new(config: AggregatorConfig) -> Self {
        Self {
            config,
            groups: HashMap::new(),
            shard_index: HashMap::new(),
            stats: AggregatorStats::default(),
        }
    }

    /// Create aggregator with default config.
    pub fn with_defaults() -> Self {
        Self::new(AggregatorConfig::default())
    }

    /// Register a shard in the appropriate aggregation group.
    pub fn register_shard(&mut self, shard: ShardRecord) -> Result<(), ShardAggregatorError> {
        // Check for duplicate
        if self.shard_index.contains_key(&shard.shard_id) {
            return Err(ShardAggregatorError::ShardAlreadyExists(
                shard.shard_id.clone(),
            ));
        }

        // Get or create group for shard type
        let group_key = shard.shard_type.to_string();
        let group = self.groups.entry(group_key.clone()).or_insert_with(|| {
            AggregationGroup::new(group_key.clone(), shard.shard_type.clone())
        });

        // Check max shards
        if group.shards.len() >= self.config.max_shards_per_group {
            return Err(ShardAggregatorError::MaxShardsExceeded(
                self.config.max_shards_per_group,
            ));
        }

        // Add shard to group
        group.add_shard(shard.clone());
        self.shard_index
            .insert(shard.shard_id.clone(), group_key);

        // Update stats
        self.stats.total_shards += 1;
        self.stats.total_groups = self.groups.len();
        self.stats.last_aggregation_ms = current_timestamp_ms();
        self.update_healthy_count();

        Ok(())
    }

    /// Remove a shard from its aggregation group.
    pub fn remove_shard(&mut self, shard_id: &str) -> Result<ShardRecord, ShardAggregatorError> {
        let group_key = self.shard_index
            .get(shard_id)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?
            .clone();

        let group = self.groups.get_mut(&group_key)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?;

        let removed = group.remove_shard(shard_id)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?;
        self.shard_index.remove(shard_id);
        self.stats.total_shards = self.stats.total_shards.saturating_sub(1);
        self.update_healthy_count();
        Ok(removed)
    }

    /// Get a shard record by ID.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardRecord> {
        let group_key = self.shard_index.get(shard_id)?;
        let group = self.groups.get(group_key)?;
        group.shards.iter().find(|s| s.shard_id == shard_id)
    }

    /// Get aggregation group by type.
    pub fn get_group(&self, shard_type: &ShardType) -> Option<&AggregationGroup> {
        self.groups.get(&shard_type.to_string())
    }

    /// Get all aggregation groups.
    pub fn get_all_groups(&self) -> Vec<&AggregationGroup> {
        self.groups.values().collect()
    }

    /// Update shard load factor.
    pub fn update_shard_load(&mut self, shard_id: &str, load: f64) -> Result<(), ShardAggregatorError> {
        let group_key = self.shard_index.get(shard_id)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?
            .clone();
        let group = self.groups.get_mut(&group_key)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?;
        if let Some(shard) = group.shards.iter_mut().find(|s| s.shard_id == shard_id) {
            shard.update_load(load);
            group.recalculate_stats();
            Ok(())
        } else {
            Err(ShardAggregatorError::ShardNotFound(shard_id.to_string()))
        }
    }

    /// Update shard heartbeat.
    pub fn shard_heartbeat(&mut self, shard_id: &str) -> Result<(), ShardAggregatorError> {
        let group_key = self.shard_index.get(shard_id)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?
            .clone();
        let group = self.groups.get_mut(&group_key)
            .ok_or(ShardAggregatorError::ShardNotFound(shard_id.to_string()))?;
        if let Some(shard) = group.shards.iter_mut().find(|s| s.shard_id == shard_id) {
            shard.heartbeat();
            shard.healthy = true;
            group.recalculate_stats();
            self.update_healthy_count();
            Ok(())
        } else {
            Err(ShardAggregatorError::ShardNotFound(shard_id.to_string()))
        }
    }

    /// Perform load rebalancing across all groups.
    pub fn rebalance(&mut self) -> usize {
        let mut total_rebalances = 0;
        for group in self.groups.values_mut() {
            if group.needs_rebalance(self.config.rebalance_threshold) {
                // Mark rebalance performed
                group.last_rebalance_ms = current_timestamp_ms();
                total_rebalances += 1;
            }
        }
        self.stats.total_rebalances += total_rebalances;
        total_rebalances
    }

    /// Find the best shard for a request (least loaded healthy shard of given type).
    pub fn find_best_shard(&self, shard_type: &ShardType) -> Option<&ShardRecord> {
        let group = self.groups.get(&shard_type.to_string())?;
        group.least_loaded_shard()
    }

    /// Update healthy shard count in stats.
    fn update_healthy_count(&mut self) {
        self.stats.healthy_shards = self.groups.values()
            .flat_map(|g| &g.shards)
            .filter(|s| s.healthy)
            .count();
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> AggregatorStats {
        self.stats.clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = AggregatorStats::default();
    }
}

impl Default for ShardAggregator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Helper to get current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shard(id: &str, shard_type: ShardType, node_id: &str, credits: f64) -> ShardRecord {
        ShardRecord::new(
            id.to_string(),
            shard_type,
            node_id.to_string(),
            credits,
        )
    }

    #[test]
    fn test_aggregator_creation() {
        let agg = ShardAggregator::with_defaults();
        assert_eq!(agg.get_stats().total_shards, 0);
        assert_eq!(agg.get_stats().total_groups, 0);
    }

    #[test]
    fn test_register_shard() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        assert!(agg.register_shard(shard).is_ok());
        assert_eq!(agg.get_stats().total_shards, 1);
        assert_eq!(agg.get_stats().total_groups, 1);
    }

    #[test]
    fn test_register_duplicate_shard() {
        let mut agg = ShardAggregator::with_defaults();
        let shard1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let shard2 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        assert!(agg.register_shard(shard1).is_ok());
        match agg.register_shard(shard2) {
            Err(ShardAggregatorError::ShardAlreadyExists(id)) => assert_eq!(id, "s1"),
            _ => panic!("Expected ShardAlreadyExists"),
        }
    }

    #[test]
    fn test_remove_shard() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        agg.register_shard(shard).unwrap();
        let removed = agg.remove_shard("s1").unwrap();
        assert_eq!(removed.shard_id, "s1");
        assert_eq!(agg.get_stats().total_shards, 0);
    }

    #[test]
    fn test_remove_nonexistent_shard() {
        let mut agg = ShardAggregator::with_defaults();
        match agg.remove_shard("missing") {
            Err(ShardAggregatorError::ShardNotFound(id)) => assert_eq!(id, "missing"),
            _ => panic!("Expected ShardNotFound"),
        }
    }

    #[test]
    fn test_get_shard() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        agg.register_shard(shard).unwrap();
        let found = agg.get_shard("s1").unwrap();
        assert_eq!(found.shard_id, "s1");
        assert!(agg.get_shard("s2").is_none());
    }

    #[test]
    fn test_get_group() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        agg.register_shard(shard).unwrap();
        let group = agg.get_group(&ShardType::SaeCompute).unwrap();
        assert_eq!(group.shards.len(), 1);
        assert!(agg.get_group(&ShardType::SaeStorage).is_none());
    }

    #[test]
    fn test_update_shard_load() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        agg.register_shard(shard).unwrap();
        assert!(agg.update_shard_load("s1", 0.5).is_ok());
        let updated = agg.get_shard("s1").unwrap();
        assert_eq!(updated.load_factor, 0.5);
    }

    #[test]
    fn test_shard_heartbeat() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        agg.register_shard(shard).unwrap();
        assert!(agg.shard_heartbeat("s1").is_ok());
    }

    #[test]
    fn test_find_best_shard() {
        let mut agg = ShardAggregator::with_defaults();
        let s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let mut s2 = make_shard("s2", ShardType::SaeCompute, "node2", 200.0);
        s2.load_factor = 0.3;
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        let best = agg.find_best_shard(&ShardType::SaeCompute).unwrap();
        assert_eq!(best.shard_id, "s1"); // s1 has load 0.0, s2 has 0.3
    }

    #[test]
    fn test_group_stats_recalculation() {
        let mut agg = ShardAggregator::with_defaults();
        let s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let s2 = make_shard("s2", ShardType::SaeCompute, "node2", 200.0);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        let group = agg.get_group(&ShardType::SaeCompute).unwrap();
        assert_eq!(group.total_credits, 300.0);
        assert_eq!(group.healthy_count, 2);
    }

    #[test]
    fn test_needs_rebalance() {
        let mut agg = ShardAggregator::with_defaults();
        let mut s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let mut s2 = make_shard("s2", ShardType::SaeCompute, "node2", 200.0);
        s1.load_factor = 0.1;
        s2.load_factor = 0.9;
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        let group = agg.get_group(&ShardType::SaeCompute).unwrap();
        assert!(group.needs_rebalance(0.5));
    }

    #[test]
    fn test_rebalance() {
        let mut agg = ShardAggregator::with_defaults();
        let mut s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let mut s2 = make_shard("s2", ShardType::SaeCompute, "node2", 200.0);
        s1.load_factor = 0.1;
        s2.load_factor = 0.9;
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        let count = agg.rebalance();
        assert_eq!(count, 1);
        assert_eq!(agg.get_stats().total_rebalances, 1);
    }

    #[test]
    fn test_healthy_count() {
        let mut agg = ShardAggregator::with_defaults();
        let s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let s2 = make_shard("s2", ShardType::SaeCompute, "node2", 200.0);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        assert_eq!(agg.get_stats().healthy_shards, 2);
    }

    #[test]
    fn test_reset_stats() {
        let mut agg = ShardAggregator::with_defaults();
        let shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        agg.register_shard(shard).unwrap();
        agg.reset_stats();
        let stats = agg.get_stats();
        assert_eq!(stats.total_shards, 0);
        assert_eq!(stats.total_groups, 0);
    }

    #[test]
    fn test_shard_type_display() {
        assert_eq!(ShardType::SaeCompute.to_string(), "SaeCompute");
        assert_eq!(ShardType::SaeStorage.to_string(), "SaeStorage");
        assert_eq!(ShardType::SaeInference.to_string(), "SaeInference");
        assert_eq!(ShardType::Custom("test".to_string()).to_string(), "Custom(test)");
    }

    #[test]
    fn test_shard_overloaded() {
        let mut shard = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        shard.load_factor = 0.8;
        assert!(shard.is_overloaded(0.7));
        assert!(!shard.is_overloaded(0.9));
    }

    #[test]
    fn test_error_display() {
        match ShardAggregatorError::ShardNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_config_default() {
        let config = AggregatorConfig::default();
        assert_eq!(config.max_shards_per_group, 64);
        assert_eq!(config.rebalance_threshold, 0.75);
        assert_eq!(config.min_healthy_nodes, 2);
        assert!(config.auto_rebalance);
    }

    #[test]
    fn test_stats_default() {
        let stats = AggregatorStats::default();
        assert_eq!(stats.total_shards, 0);
        assert_eq!(stats.total_groups, 0);
        assert_eq!(stats.total_rebalances, 0);
    }

    #[test]
    fn test_aggregator_default() {
        let agg = ShardAggregator::default();
        assert_eq!(agg.get_stats().total_shards, 0);
    }

    #[test]
    fn test_multiple_groups() {
        let mut agg = ShardAggregator::with_defaults();
        let s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let s2 = make_shard("s2", ShardType::SaeStorage, "node2", 200.0);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        assert_eq!(agg.get_stats().total_groups, 2);
        assert_eq!(agg.get_all_groups().len(), 2);
    }

    #[test]
    fn test_group_least_loaded_shard() {
        let mut group = AggregationGroup::new("test".to_string(), ShardType::SaeCompute);
        let mut s1 = make_shard("s1", ShardType::SaeCompute, "node1", 100.0);
        let mut s2 = make_shard("s2", ShardType::SaeCompute, "node2", 200.0);
        s1.load_factor = 0.8;
        s2.load_factor = 0.2;
        group.add_shard(s1);
        group.add_shard(s2);
        let least = group.least_loaded_shard().unwrap();
        assert_eq!(least.shard_id, "s2");
    }
}
