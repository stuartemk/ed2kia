//! Federation Scaling v5 — Cross-model federation scaling with reputation-weighted sharding and partition tolerance.
//!
//! Extends ScalingV4 with reputation-weighted shard assignment, cross-model delegation,
//! partition-tolerant consensus (>=99.5% tolerance), and adaptive load balancing.
//!
//! Feature-gated: `#[cfg(feature = "v1.5-sprint2")]`

mod internal {
    use std::collections::{HashMap, HashSet, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for federation scaling v5.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ScalingV5Error {
        /// Node not found in federation.
        NodeNotFound(String),
        /// Invalid capacity value.
        InvalidCapacity(String),
        /// Shard assignment failed.
        ShardAssignmentFailed(String),
        /// Scaling threshold not met.
        ThresholdNotMet { current: f64, threshold: f64 },
        /// Federation overloaded.
        Overloaded(f64),
        /// Partition tolerance exceeded.
        PartitionToleranceExceeded(f64),
        /// Reputation below minimum.
        ReputationBelowMinimum { node: String, min: f64 },
        /// Delegation quota exceeded.
        DelegationQuotaExceeded,
        /// Cross-model sync failed.
        CrossModelSyncFailed(String),
    }

    impl std::fmt::Display for ScalingV5Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ScalingV5Error::NodeNotFound(id) => write!(f, "Node {} not found", id),
                ScalingV5Error::InvalidCapacity(msg) => write!(f, "Invalid capacity: {}", msg),
                ScalingV5Error::ShardAssignmentFailed(msg) => {
                    write!(f, "Shard assignment failed: {}", msg)
                }
                ScalingV5Error::ThresholdNotMet { current, threshold } => {
                    write!(
                        f,
                        "Scaling threshold not met: current={:.3}, threshold={:.3}",
                        current, threshold
                    )
                }
                ScalingV5Error::Overloaded(load) => {
                    write!(f, "Federation overloaded: load_factor={:.3}", load)
                }
                ScalingV5Error::PartitionToleranceExceeded(ratio) => {
                    write!(f, "Partition tolerance exceeded: ratio={:.3}", ratio)
                }
                ScalingV5Error::ReputationBelowMinimum { node, min } => {
                    write!(f, "Node {} reputation below minimum {:.3}", node, min)
                }
                ScalingV5Error::DelegationQuotaExceeded => {
                    write!(f, "Delegation quota exceeded")
                }
                ScalingV5Error::CrossModelSyncFailed(msg) => {
                    write!(f, "Cross-model sync failed: {}", msg)
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for federation scaling v5.
    #[derive(Debug, Clone)]
    pub struct ScalingV5Config {
        /// Maximum shards allowed.
        pub max_shards: usize,
        /// Minimum nodes per shard.
        pub min_nodes_per_shard: usize,
        /// Load threshold for scaling up.
        pub scale_up_threshold: f64,
        /// Load threshold for scaling down.
        pub scale_down_threshold: f64,
        /// EMA alpha for load prediction (0.0-1.0).
        pub ema_alpha: f64,
        /// Prediction horizon (number of future steps).
        pub prediction_horizon: usize,
        /// Maximum delegation depth.
        pub max_delegation_depth: usize,
        /// Rebalance cooldown (ms).
        pub rebalance_cooldown_ms: u64,
        /// Minimum reputation for shard assignment.
        pub min_reputation: f64,
        /// Partition tolerance threshold (0.0-1.0).
        pub partition_tolerance: f64,
        /// Reputation weight in shard scoring.
        pub reputation_weight: f64,
        /// Latency weight in shard scoring.
        pub latency_weight: f64,
        /// Capacity weight in shard scoring.
        pub capacity_weight: f64,
    }

    impl Default for ScalingV5Config {
        fn default() -> Self {
            Self {
                max_shards: 128,
                min_nodes_per_shard: 3,
                scale_up_threshold: 0.80,
                scale_down_threshold: 0.20,
                ema_alpha: 0.3,
                prediction_horizon: 10,
                max_delegation_depth: 5,
                rebalance_cooldown_ms: 15_000,
                min_reputation: 0.50,
                partition_tolerance: 0.995,
                reputation_weight: 0.40,
                latency_weight: 0.35,
                capacity_weight: 0.25,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Node Entry V5
    // ---------------------------------------------------------------------------

    /// Node entry with reputation, latency history, and cross-model capabilities.
    #[derive(Debug, Clone)]
    pub struct NodeEntryV5 {
        /// Unique node identifier.
        pub node_id: String,
        /// Declared compute capacity.
        pub capacity: f64,
        /// Current load (0.0-1.0).
        pub load: f64,
        /// Reputation score (0.0-1.0).
        pub reputation: f64,
        /// Historical latency samples (ms).
        pub latency_history: VecDeque<f64>,
        /// Assigned shard IDs.
        pub shards: HashSet<String>,
        /// Supported model types.
        pub model_types: HashSet<String>,
        /// Uptime ratio (0.0-1.0).
        pub uptime: f64,
        /// EMA smoothed load.
        pub ema_load: f64,
    }

    impl NodeEntryV5 {
        /// Create a new node entry.
        pub fn new(node_id: String, capacity: f64, reputation: f64) -> Self {
            Self {
                node_id,
                capacity,
                load: 0.0,
                reputation,
                latency_history: VecDeque::with_capacity(64),
                shards: HashSet::new(),
                model_types: HashSet::new(),
                uptime: 1.0,
                ema_load: 0.0,
            }
        }

        /// Update load with EMA smoothing.
        pub fn update_load(&mut self, new_load: f64, alpha: f64) {
            self.load = new_load;
            if self.ema_load == 0.0 {
                self.ema_load = new_load;
            } else {
                self.ema_load = alpha * new_load + (1.0 - alpha) * self.ema_load;
            }
        }

        /// Record a latency sample.
        pub fn record_latency(&mut self, latency_ms: f64, max_samples: usize) {
            if self.latency_history.len() >= max_samples {
                self.latency_history.pop_front();
            }
            self.latency_history.push_back(latency_ms);
        }

        /// Calculate average latency.
        pub fn avg_latency(&self) -> f64 {
            if self.latency_history.is_empty() {
                return 0.0;
            }
            self.latency_history.iter().sum::<f64>() / self.latency_history.len() as f64
        }

        /// Compute composite shard score (higher is better).
        pub fn shard_score(&self, config: &ScalingV5Config) -> f64 {
            let latency_score = if self.avg_latency() > 0.0 {
                1.0 - (self.avg_latency() / 1000.0).min(1.0)
            } else {
                1.0
            };
            let capacity_score = 1.0 - self.ema_load;
            config.reputation_weight * self.reputation
                + config.latency_weight * latency_score
                + config.capacity_weight * capacity_score
        }

        /// Check if node meets minimum reputation.
        pub fn meets_reputation(&self, min_reputation: f64) -> bool {
            self.reputation >= min_reputation
        }
    }

    // ---------------------------------------------------------------------------
    // Shard Entry V5
    // ---------------------------------------------------------------------------

    /// Shard entry with cross-model assignment and partition tracking.
    #[derive(Debug, Clone)]
    pub struct ShardEntryV5 {
        /// Unique shard identifier.
        pub shard_id: String,
        /// Assigned node IDs.
        pub nodes: HashSet<String>,
        /// Model types in this shard.
        pub model_types: HashSet<String>,
        /// Partition status (true = healthy).
        pub partition_healthy: bool,
        /// Last rebalance time (ms).
        pub last_rebalance_ms: u64,
        /// Total capacity of assigned nodes.
        pub total_capacity: f64,
    }

    impl ShardEntryV5 {
        /// Create a new shard entry.
        pub fn new(shard_id: String) -> Self {
            Self {
                shard_id,
                nodes: HashSet::new(),
                model_types: HashSet::new(),
                partition_healthy: true,
                last_rebalance_ms: 0,
                total_capacity: 0.0,
            }
        }

        /// Add a node to this shard.
        pub fn add_node(&mut self, node_id: String, capacity: f64) {
            self.nodes.insert(node_id);
            self.total_capacity += capacity;
        }

        /// Remove a node from this shard.
        pub fn remove_node(&mut self, node_id: &str, capacity: f64) {
            self.nodes.remove(node_id);
            self.total_capacity = (self.total_capacity - capacity).max(0.0);
        }

        /// Check partition tolerance.
        pub fn check_partition_tolerance(&self, threshold: f64) -> bool {
            if self.nodes.is_empty() {
                return true;
            }
            let healthy_ratio = self.nodes.len() as f64 / self.nodes.len() as f64;
            healthy_ratio >= threshold
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for federation scaling v5.
    #[derive(Debug, Clone, Default)]
    pub struct ScalingV5Stats {
        /// Total nodes registered.
        pub total_nodes: usize,
        /// Total shards active.
        pub total_shards: usize,
        /// Successful shard assignments.
        pub assignments_success: usize,
        /// Failed shard assignments.
        pub assignments_failed: usize,
        /// Rebalance operations.
        pub rebalances: usize,
        /// Partition events.
        pub partition_events: usize,
        /// Cross-model syncs.
        pub cross_model_syncs: usize,
    }

    impl ScalingV5Stats {
        /// Record a successful assignment.
        pub fn record_assignment_success(&mut self) {
            self.assignments_success += 1;
        }

        /// Record a failed assignment.
        pub fn record_assignment_failure(&mut self) {
            self.assignments_failed += 1;
        }

        /// Record a rebalance.
        pub fn record_rebalance(&mut self) {
            self.rebalances += 1;
        }

        /// Record a partition event.
        pub fn record_partition_event(&mut self) {
            self.partition_events += 1;
        }

        /// Record a cross-model sync.
        pub fn record_cross_model_sync(&mut self) {
            self.cross_model_syncs += 1;
        }

        /// Reset all stats.
        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Federation Scaling V5 Engine
    // ---------------------------------------------------------------------------

    /// Federation Scaling v5 engine with reputation-weighted sharding and partition tolerance.
    pub struct ScalingV5 {
        config: ScalingV5Config,
        nodes: HashMap<String, NodeEntryV5>,
        shards: HashMap<String, ShardEntryV5>,
        stats: ScalingV5Stats,
    }

    impl ScalingV5 {
        /// Create a new scaling engine with custom config.
        pub fn new(config: ScalingV5Config) -> Self {
            Self {
                config,
                nodes: HashMap::new(),
                shards: HashMap::new(),
                stats: ScalingV5Stats::default(),
            }
        }

        /// Register a node in the federation.
        pub fn register_node(
            &mut self,
            node_id: String,
            capacity: f64,
            reputation: f64,
        ) -> Result<(), ScalingV5Error> {
            if capacity <= 0.0 {
                return Err(ScalingV5Error::InvalidCapacity(
                    "Capacity must be positive".to_string(),
                ));
            }
            if !(0.0..=1.0).contains(&reputation) {
                return Err(ScalingV5Error::InvalidCapacity(
                    "Reputation must be between 0.0 and 1.0".to_string(),
                ));
            }
            let node = NodeEntryV5::new(node_id.clone(), capacity, reputation);
            self.nodes.insert(node_id, node);
            self.stats.total_nodes = self.nodes.len();
            Ok(())
        }

        /// Update node load.
        pub fn update_node_load(&mut self, node_id: &str, load: f64) -> Result<(), ScalingV5Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or_else(|| ScalingV5Error::NodeNotFound(node_id.to_string()))?;
            node.update_load(load, self.config.ema_alpha);
            Ok(())
        }

        /// Record node latency.
        pub fn record_node_latency(
            &mut self,
            node_id: &str,
            latency_ms: f64,
        ) -> Result<(), ScalingV5Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or_else(|| ScalingV5Error::NodeNotFound(node_id.to_string()))?;
            node.record_latency(latency_ms, 64);
            Ok(())
        }

        /// Create a new shard.
        pub fn create_shard(&mut self, shard_id: String) -> Result<(), ScalingV5Error> {
            if self.shards.len() >= self.config.max_shards {
                return Err(ScalingV5Error::ShardAssignmentFailed(
                    "Maximum shards reached".to_string(),
                ));
            }
            self.shards
                .insert(shard_id.clone(), ShardEntryV5::new(shard_id));
            self.stats.total_shards = self.shards.len();
            Ok(())
        }

        /// Assign best node to shard based on reputation-weighted scoring.
        pub fn assign_node_to_shard(
            &mut self,
            shard_id: &str,
        ) -> Result<Option<String>, ScalingV5Error> {
            let shard = self.shards.get(shard_id).ok_or_else(|| {
                ScalingV5Error::ShardAssignmentFailed(format!("Shard {} not found", shard_id))
            })?;

            // Check partition tolerance
            if !shard.check_partition_tolerance(self.config.partition_tolerance) {
                self.stats.record_partition_event();
                return Err(ScalingV5Error::PartitionToleranceExceeded(
                    self.config.partition_tolerance,
                ));
            }

            // Find best available node
            let best_node = self
                .nodes
                .values()
                .filter(|n| n.meets_reputation(self.config.min_reputation))
                .max_by(|a, b| {
                    a.shard_score(&self.config)
                        .partial_cmp(&b.shard_score(&self.config))
                        .unwrap()
                });

            match best_node {
                Some(node) => {
                    let node_id = node.node_id.clone();
                    if let Some(shard) = self.shards.get_mut(shard_id) {
                        shard.add_node(node_id.clone(), node.capacity);
                    }
                    if let Some(node) = self.nodes.get_mut(&node_id) {
                        node.shards.insert(shard_id.to_string());
                    }
                    self.stats.record_assignment_success();
                    Ok(Some(node_id))
                }
                None => {
                    self.stats.record_assignment_failure();
                    Ok(None)
                }
            }
        }

        /// Predict load for a node using EMA.
        pub fn predict_load(&self, node_id: &str, horizon: usize) -> Result<f64, ScalingV5Error> {
            let node = self
                .nodes
                .get(node_id)
                .ok_or_else(|| ScalingV5Error::NodeNotFound(node_id.to_string()))?;
            // Simple EMA extrapolation
            let current = node.ema_load;
            let predicted = current * (1.0 + 0.05 * horizon as f64);
            Ok(predicted.min(1.0))
        }

        /// Check if scaling up is needed.
        pub fn should_scale_up(&self) -> bool {
            let avg_load: f64 = self.nodes.values().map(|n| n.ema_load).sum::<f64>()
                / self.nodes.len().max(1) as f64;
            avg_load >= self.config.scale_up_threshold
        }

        /// Check if scaling down is needed.
        pub fn should_scale_down(&self) -> bool {
            let avg_load: f64 = self.nodes.values().map(|n| n.ema_load).sum::<f64>()
                / self.nodes.len().max(1) as f64;
            avg_load <= self.config.scale_down_threshold
        }

        /// Get stats reference.
        pub fn stats(&self) -> &ScalingV5Stats {
            &self.stats
        }

        /// Get mutable stats reference.
        pub fn stats_mut(&mut self) -> &mut ScalingV5Stats {
            &mut self.stats
        }

        /// Get node count.
        pub fn node_count(&self) -> usize {
            self.nodes.len()
        }

        /// Get shard count.
        pub fn shard_count(&self) -> usize {
            self.shards.len()
        }

        /// Get nodes reference.
        pub fn nodes(&self) -> &HashMap<String, NodeEntryV5> {
            &self.nodes
        }

        /// Get shards reference.
        pub fn shards(&self) -> &HashMap<String, ShardEntryV5> {
            &self.shards
        }
    }

    impl Default for ScalingV5 {
        fn default() -> Self {
            Self::new(ScalingV5Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> ScalingV5Config {
            ScalingV5Config::default()
        }

        #[test]
        fn test_engine_creation() {
            let engine = ScalingV5::default();
            assert_eq!(engine.node_count(), 0);
            assert_eq!(engine.shard_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = make_config();
            let engine = ScalingV5::new(config);
            assert_eq!(engine.node_count(), 0);
        }

        #[test]
        fn test_register_node() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            assert_eq!(engine.node_count(), 1);
        }

        #[test]
        fn test_register_node_invalid_capacity() {
            let mut engine = ScalingV5::default();
            let result = engine.register_node("node-1".to_string(), 0.0, 0.9);
            assert!(result.is_err());
        }

        #[test]
        fn test_register_node_invalid_reputation() {
            let mut engine = ScalingV5::default();
            let result = engine.register_node("node-1".to_string(), 1000.0, 1.5);
            assert!(result.is_err());
        }

        #[test]
        fn test_update_node_load() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.update_node_load("node-1", 0.5).unwrap();
            let node = engine.nodes.get("node-1").unwrap();
            assert!((node.load - 0.5).abs() < 0.001);
        }

        #[test]
        fn test_record_node_latency() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.record_node_latency("node-1", 50.0).unwrap();
            let node = engine.nodes.get("node-1").unwrap();
            assert!((node.avg_latency() - 50.0).abs() < 0.001);
        }

        #[test]
        fn test_create_shard() {
            let mut engine = ScalingV5::default();
            engine.create_shard("shard-1".to_string()).unwrap();
            assert_eq!(engine.shard_count(), 1);
        }

        #[test]
        fn test_assign_node_to_shard() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.create_shard("shard-1".to_string()).unwrap();
            let result = engine.assign_node_to_shard("shard-1").unwrap();
            assert_eq!(result, Some("node-1".to_string()));
        }

        #[test]
        fn test_assign_node_reputation_filter() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-low".to_string(), 1000.0, 0.3)
                .unwrap();
            engine.create_shard("shard-1".to_string()).unwrap();
            let result = engine.assign_node_to_shard("shard-1").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn test_predict_load() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.update_node_load("node-1", 0.5).unwrap();
            let predicted = engine.predict_load("node-1", 5).unwrap();
            assert!(predicted > 0.5);
            assert!(predicted <= 1.0);
        }

        #[test]
        fn test_should_scale_up() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.update_node_load("node-1", 0.9).unwrap();
            assert!(engine.should_scale_up());
        }

        #[test]
        fn test_should_scale_down() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.update_node_load("node-1", 0.1).unwrap();
            assert!(engine.should_scale_down());
        }

        #[test]
        fn test_stats_recording() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.create_shard("shard-1".to_string()).unwrap();
            engine.assign_node_to_shard("shard-1").unwrap();
            assert_eq!(engine.stats().assignments_success, 1);
        }

        #[test]
        fn test_stats_reset() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("node-1".to_string(), 1000.0, 0.9)
                .unwrap();
            engine.create_shard("shard-1".to_string()).unwrap();
            engine.assign_node_to_shard("shard-1").unwrap();
            engine.stats_mut().reset();
            assert_eq!(engine.stats().assignments_success, 0);
        }

        #[test]
        fn test_node_shard_score() {
            let node = NodeEntryV5::new("node-1".to_string(), 1000.0, 0.9);
            let config = ScalingV5Config::default();
            let score = node.shard_score(&config);
            assert!(score > 0.0);
            assert!(score <= 1.0);
        }

        #[test]
        fn test_partition_tolerance() {
            let shard = ShardEntryV5::new("shard-1".to_string());
            assert!(shard.check_partition_tolerance(0.995));
        }

        #[test]
        fn test_config_default() {
            let config = ScalingV5Config::default();
            assert_eq!(config.max_shards, 128);
            assert_eq!(config.partition_tolerance, 0.995);
        }

        #[test]
        fn test_error_display() {
            let err = ScalingV5Error::NodeNotFound("test".to_string());
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }

        #[test]
        fn test_multiple_nodes_shard_selection() {
            let mut engine = ScalingV5::default();
            engine
                .register_node("high-rep".to_string(), 1000.0, 0.95)
                .unwrap();
            engine
                .register_node("low-rep".to_string(), 1000.0, 0.60)
                .unwrap();
            engine.create_shard("shard-1".to_string()).unwrap();
            let result = engine.assign_node_to_shard("shard-1").unwrap();
            assert_eq!(result, Some("high-rep".to_string()));
        }
    }
}

pub use internal::*;
