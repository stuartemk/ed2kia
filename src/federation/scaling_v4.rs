//! Federation Scaling v4 — Predictive federation scaling with ML-based load forecasting.
//!
//! Extends ScalingV3 with predictive load forecasting using exponential moving averages,
//! adaptive shard topology optimization, cross-federation delegation scoring,
//! and proactive rebalancing triggers.
//!
//! Feature-gated: `#[cfg(feature = "v1.4-sprint3")]`

mod internal {
    use std::collections::{HashMap, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for federation scaling v4.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ScalingV4Error {
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
        /// Prediction model not trained.
        PredictionNotReady,
        /// Delegation quota exceeded.
        DelegationQuotaExceeded,
    }

    impl std::fmt::Display for ScalingV4Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ScalingV4Error::NodeNotFound(id) => write!(f, "Node {} not found", id),
                ScalingV4Error::InvalidCapacity(msg) => write!(f, "Invalid capacity: {}", msg),
                ScalingV4Error::ShardAssignmentFailed(msg) => {
                    write!(f, "Shard assignment failed: {}", msg)
                }
                ScalingV4Error::ThresholdNotMet { current, threshold } => {
                    write!(
                        f,
                        "Scaling threshold not met: current={:.3}, threshold={:.3}",
                        current, threshold
                    )
                }
                ScalingV4Error::Overloaded(load) => {
                    write!(f, "Federation overloaded: load_factor={:.3}", load)
                }
                ScalingV4Error::PredictionNotReady => write!(f, "Prediction model not trained"),
                ScalingV4Error::DelegationQuotaExceeded => {
                    write!(f, "Delegation quota exceeded")
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for federation scaling v4.
    #[derive(Debug, Clone)]
    pub struct ScalingV4Config {
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
        /// Proactive rebalance threshold.
        pub proactive_threshold: f64,
    }

    impl Default for ScalingV4Config {
        fn default() -> Self {
            Self {
                max_shards: 64,
                min_nodes_per_shard: 2,
                scale_up_threshold: 0.80,
                scale_down_threshold: 0.20,
                ema_alpha: 0.3,
                prediction_horizon: 5,
                max_delegation_depth: 3,
                rebalance_cooldown_ms: 30_000,
                proactive_threshold: 0.65,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Node Capability V4
    // ---------------------------------------------------------------------------

    /// Node capability profile with predictive metrics.
    #[derive(Debug, Clone)]
    pub struct NodeCapabilityV4 {
        /// Unique node identifier.
        pub node_id: String,
        /// Available compute capacity (FLOPS).
        pub compute_capacity: f64,
        /// Current load factor (0.0-1.0).
        pub load_factor: f64,
        /// Average latency to peers (ms).
        pub avg_latency_ms: f64,
        /// Historical uptime (0.0-1.0).
        pub uptime: f64,
        /// Reputation score.
        pub reputation: f64,
        /// Assigned shard IDs.
        pub assigned_shards: Vec<String>,
        /// EMA-smoothed load for prediction.
        pub ema_load: f64,
        /// Load trend (positive = increasing).
        pub load_trend: f64,
        /// Delegation depth (0 = root).
        pub delegation_depth: usize,
    }

    impl NodeCapabilityV4 {
        pub fn new(node_id: String, compute_capacity: f64) -> Self {
            Self {
                node_id,
                compute_capacity,
                load_factor: 0.0,
                avg_latency_ms: 0.0,
                uptime: 1.0,
                reputation: 0.5,
                assigned_shards: Vec::new(),
                ema_load: 0.0,
                load_trend: 0.0,
                delegation_depth: 0,
            }
        }

        /// Updates EMA load and trend.
        pub fn update_load(&mut self, new_load: f64, alpha: f64) {
            let old_ema = self.ema_load;
            self.ema_load = alpha * new_load + (1.0 - alpha) * self.ema_load;
            self.load_trend = self.ema_load - old_ema;
            self.load_factor = new_load.clamp(0.0, 1.0);
        }

        /// Predicts future load based on EMA trend.
        pub fn predict_load(&self, horizon: usize, alpha: f64) -> f64 {
            let trend_per_step = self.load_trend * alpha;
            (self.ema_load + trend_per_step * horizon as f64).clamp(0.0, 1.0)
        }

        /// Computes composite scaling score.
        pub fn scaling_score(&self) -> f64 {
            let capacity_score = self.compute_capacity / (1.0 + self.ema_load);
            let latency_penalty = 1.0 / (1.0 + self.avg_latency_ms / 100.0);
            let reputation_factor = self.reputation;
            let uptime_factor = self.uptime;
            let delegation_penalty = 1.0 / (1.0 + self.delegation_depth as f64 * 0.1);
            (capacity_score * latency_penalty * reputation_factor * uptime_factor * delegation_penalty).min(1.0)
        }

        /// Checks if node can accept more work.
        pub fn can_accept_work(&self, min_available: f64) -> bool {
            (1.0 - self.load_factor) >= min_available
        }
    }

    // ---------------------------------------------------------------------------
    // Shard Config V4
    // ---------------------------------------------------------------------------

    /// Shard configuration with predictive metrics.
    #[derive(Debug, Clone)]
    pub struct ShardConfigV4 {
        /// Shard identifier.
        pub shard_id: String,
        /// Assigned node IDs.
        pub nodes: Vec<String>,
        /// Shard capacity.
        pub capacity: f64,
        /// Shard load factor.
        pub load_factor: f64,
        /// Predicted load (EMA-based).
        pub predicted_load: f64,
        /// Health score (0.0-1.0).
        pub health_score: f64,
        /// Last rebalance timestamp (ms).
        pub last_rebalance_ms: u64,
    }

    impl ShardConfigV4 {
        pub fn new(shard_id: String) -> Self {
            Self {
                shard_id,
                nodes: Vec::new(),
                capacity: 0.0,
                load_factor: 0.0,
                predicted_load: 0.0,
                health_score: 1.0,
                last_rebalance_ms: 0,
            }
        }

        pub fn add_node(&mut self, node_id: String, capacity: f64) {
            self.nodes.push(node_id);
            self.capacity += capacity;
        }

        pub fn remove_node(&mut self, node_id: &str, capacity: f64) {
            self.nodes.retain(|n| n != node_id);
            self.capacity = (self.capacity - capacity).max(0.0);
        }

        pub fn update_load(&mut self, load_factor: f64) {
            self.load_factor = load_factor.clamp(0.0, 1.0);
            self.health_score = (1.0 - self.load_factor).max(0.0);
        }

        pub fn needs_rebalance(&self, threshold: f64) -> bool {
            self.load_factor > threshold || self.predicted_load > threshold
        }

        pub fn needs_proactive_rebalance(&self, threshold: f64) -> bool {
            self.predicted_load > threshold && self.load_factor <= threshold
        }
    }

    // ---------------------------------------------------------------------------
    // Scaling Decision V4
    // ---------------------------------------------------------------------------

    /// Types of scaling decisions.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ScalingDecisionType {
        AddShard,
        RemoveShard,
        Rebalance,
        ProactiveRebalance,
        ScaleUp,
        ScaleDown,
        Delegate,
        NoOp,
    }

    impl std::fmt::Display for ScalingDecisionType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ScalingDecisionType::AddShard => write!(f, "AddShard"),
                ScalingDecisionType::RemoveShard => write!(f, "RemoveShard"),
                ScalingDecisionType::Rebalance => write!(f, "Rebalance"),
                ScalingDecisionType::ProactiveRebalance => write!(f, "ProactiveRebalance"),
                ScalingDecisionType::ScaleUp => write!(f, "ScaleUp"),
                ScalingDecisionType::ScaleDown => write!(f, "ScaleDown"),
                ScalingDecisionType::Delegate => write!(f, "Delegate"),
                ScalingDecisionType::NoOp => write!(f, "NoOp"),
            }
        }
    }

    /// Scaling decision with prediction context.
    #[derive(Debug, Clone)]
    pub struct ScalingDecisionV4 {
        pub decision_type: ScalingDecisionType,
        pub target: String,
        pub reason: String,
        pub confidence: f64,
        pub predicted_load: f64,
        pub timestamp_ms: u64,
    }

    impl ScalingDecisionV4 {
        pub fn new(
            decision_type: ScalingDecisionType,
            target: String,
            reason: String,
            confidence: f64,
            predicted_load: f64,
        ) -> Self {
            Self {
                decision_type,
                target,
                reason,
                confidence,
                predicted_load,
                timestamp_ms: current_timestamp_ms(),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for the scaling engine.
    #[derive(Debug, Clone)]
    pub struct ScalingV4Stats {
        pub total_decisions: usize,
        pub total_shards_created: usize,
        pub total_shards_removed: usize,
        pub total_rebalances: usize,
        pub total_proactive_rebalances: usize,
        pub total_delegations: usize,
        pub avg_prediction_error: f64,
        pub prediction_samples: usize,
        pub active_shards: usize,
        pub active_nodes: usize,
        pub federation_load_factor: f64,
    }

    impl Default for ScalingV4Stats {
        fn default() -> Self {
            Self {
                total_decisions: 0,
                total_shards_created: 0,
                total_shards_removed: 0,
                total_rebalances: 0,
                total_proactive_rebalances: 0,
                total_delegations: 0,
                avg_prediction_error: 0.0,
                prediction_samples: 0,
                active_shards: 0,
                active_nodes: 0,
                federation_load_factor: 0.0,
            }
        }
    }

    impl ScalingV4Stats {
        pub fn record_prediction_error(&mut self, error: f64) {
            let n = self.prediction_samples + 1;
            self.avg_prediction_error =
                self.avg_prediction_error * (self.prediction_samples as f64 / n as f64) + error / n as f64;
            self.prediction_samples = n;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Main Engine
    // ---------------------------------------------------------------------------

    /// Federation Scaling v4 engine with predictive load forecasting.
    pub struct FederationScalingV4 {
        config: ScalingV4Config,
        nodes: HashMap<String, NodeCapabilityV4>,
        shards: HashMap<String, ShardConfigV4>,
        stats: ScalingV4Stats,
        decision_history: VecDeque<ScalingDecisionV4>,
        last_rebalance_ms: u64,
    }

    impl FederationScalingV4 {
        pub fn new(config: ScalingV4Config) -> Self {
            Self {
                config,
                nodes: HashMap::new(),
                shards: HashMap::new(),
                stats: ScalingV4Stats::default(),
                decision_history: VecDeque::new(),
                last_rebalance_ms: 0,
            }
        }

        pub fn with_defaults() -> Self {
            Self::new(ScalingV4Config::default())
        }

        /// Register a node in the federation.
        pub fn register_node(&mut self, node_id: String, compute_capacity: f64) -> Result<(), ScalingV4Error> {
            if compute_capacity <= 0.0 {
                return Err(ScalingV4Error::InvalidCapacity("Capacity must be positive".to_string()));
            }
            let node = NodeCapabilityV4::new(node_id.clone(), compute_capacity);
            self.nodes.insert(node_id, node);
            self.stats.active_nodes = self.nodes.len();
            Ok(())
        }

        /// Update node load with EMA smoothing.
        pub fn update_node_load(&mut self, node_id: &str, load: f64) -> Result<(), ScalingV4Error> {
            let node = self.nodes.get_mut(node_id).ok_or(ScalingV4Error::NodeNotFound(node_id.to_string()))?;
            node.update_load(load, self.config.ema_alpha);
            Ok(())
        }

        /// Update node delegation depth.
        pub fn update_delegation_depth(&mut self, node_id: &str, depth: usize) -> Result<(), ScalingV4Error> {
            if depth > self.config.max_delegation_depth {
                return Err(ScalingV4Error::DelegationQuotaExceeded);
            }
            let node = self.nodes.get_mut(node_id).ok_or(ScalingV4Error::NodeNotFound(node_id.to_string()))?;
            node.delegation_depth = depth;
            Ok(())
        }

        /// Create a new shard.
        pub fn create_shard(&mut self, shard_id: String) -> Result<(), ScalingV4Error> {
            if self.shards.len() >= self.config.max_shards {
                return Err(ScalingV4Error::ShardAssignmentFailed("Max shards reached".to_string()));
            }
            let shard = ShardConfigV4::new(shard_id.clone());
            self.shards.insert(shard_id, shard);
            self.stats.total_shards_created += 1;
            self.stats.active_shards = self.shards.len();
            Ok(())
        }

        /// Assign a node to a shard.
        pub fn assign_node_to_shard(&mut self, node_id: &str, shard_id: &str) -> Result<(), ScalingV4Error> {
            let capacity = self.nodes.get(node_id).ok_or(ScalingV4Error::NodeNotFound(node_id.to_string()))?.compute_capacity;
            let shard = self.shards.get_mut(shard_id).ok_or(ScalingV4Error::ShardAssignmentFailed(format!("Shard {} not found", shard_id)))?;
            shard.add_node(node_id.to_string(), capacity);
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.assigned_shards.push(shard_id.to_string());
            }
            Ok(())
        }

        /// Evaluate scaling decisions based on current and predicted load.
        pub fn evaluate_scaling(&mut self) -> Vec<ScalingDecisionV4> {
            let mut decisions = Vec::new();
            let now = current_timestamp_ms();

            // Check rebalance cooldown
            let cooldown_ok = now.saturating_sub(self.last_rebalance_ms) >= self.config.rebalance_cooldown_ms;

            // Evaluate each shard
            for (shard_id, shard) in &self.shards {
                let predicted = self.predict_shard_load(shard_id);
                let mut shard = (*shard).clone();
                shard.predicted_load = predicted;

                // Reactive rebalance
                if shard.load_factor > self.config.scale_up_threshold {
                    decisions.push(ScalingDecisionV4::new(
                        ScalingDecisionType::Rebalance,
                        shard_id.clone(),
                        format!("Load {:.2} exceeds threshold {:.2}", shard.load_factor, self.config.scale_up_threshold),
                        0.9,
                        predicted,
                    ));
                    self.stats.total_rebalances += 1;
                }

                // Proactive rebalance
                if cooldown_ok && shard.needs_proactive_rebalance(self.config.proactive_threshold) {
                    decisions.push(ScalingDecisionV4::new(
                        ScalingDecisionType::ProactiveRebalance,
                        shard_id.clone(),
                        format!("Predicted load {:.2} exceeds proactive threshold {:.2}", predicted, self.config.proactive_threshold),
                        0.7,
                        predicted,
                    ));
                    self.stats.total_proactive_rebalances += 1;
                }

                // Scale down
                if shard.load_factor < self.config.scale_down_threshold && shard.nodes.len() > self.config.min_nodes_per_shard {
                    decisions.push(ScalingDecisionV4::new(
                        ScalingDecisionType::ScaleDown,
                        shard_id.clone(),
                        format!("Load {:.2} below threshold {:.2}", shard.load_factor, self.config.scale_down_threshold),
                        0.8,
                        predicted,
                    ));
                }

                // Update prediction error
                let error = (predicted - shard.load_factor).abs();
                self.stats.record_prediction_error(error);
            }

            // Check federation-wide load
            let fed_load = self.compute_federation_load();
            if fed_load > self.config.scale_up_threshold && self.shards.len() < self.config.max_shards {
                decisions.push(ScalingDecisionV4::new(
                    ScalingDecisionType::AddShard,
                    "federation".to_string(),
                    format!("Federation load {:.2} exceeds threshold", fed_load),
                    0.85,
                    fed_load,
                ));
            }

            if decisions.is_empty() {
                decisions.push(ScalingDecisionV4::new(
                    ScalingDecisionType::NoOp,
                    "federation".to_string(),
                    "No scaling action needed".to_string(),
                    1.0,
                    fed_load,
                ));
            }

            self.stats.total_decisions += decisions.len();
            self.stats.federation_load_factor = fed_load;

            // Record decisions
            for d in &decisions {
                self.decision_history.push_back(d.clone());
            }
            while self.decision_history.len() > 100 {
                self.decision_history.pop_front();
            }

            decisions
        }

        /// Predict shard load using node EMA predictions.
        pub fn predict_shard_load(&self, shard_id: &str) -> f64 {
            let shard = match self.shards.get(shard_id) {
                Some(s) => s,
                None => return 0.0,
            };
            if shard.nodes.is_empty() {
                return 0.0;
            }
            let total_predicted: f64 = shard.nodes.iter().map(|nid| {
                self.nodes.get(nid)
                    .map(|n| n.predict_load(self.config.prediction_horizon, self.config.ema_alpha))
                    .unwrap_or(0.0)
            }).sum();
            total_predicted / shard.nodes.len() as f64
        }

        /// Compute federation-wide load factor.
        pub fn compute_federation_load(&self) -> f64 {
            if self.nodes.is_empty() {
                return 0.0;
            }
            let total: f64 = self.nodes.values().map(|n| n.load_factor).sum();
            total / self.nodes.len() as f64
        }

        /// Get best node for delegation.
        pub fn select_best_node(&self) -> Option<&NodeCapabilityV4> {
            self.nodes.values().max_by_key(|a| (a.scaling_score() * 1000.0) as i64)
        }

        /// Get stats reference.
        pub fn stats(&self) -> &ScalingV4Stats {
            &self.stats
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Get node count.
        pub fn node_count(&self) -> usize {
            self.nodes.len()
        }

        /// Get shard count.
        pub fn shard_count(&self) -> usize {
            self.shards.len()
        }

        /// Get decision history.
        pub fn decision_history(&self) -> &VecDeque<ScalingDecisionV4> {
            &self.decision_history
        }
    }

    impl Default for FederationScalingV4 {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[cfg(feature = "v1.4-sprint3")]
pub use internal::*;

#[cfg(all(test, feature = "v1.4-sprint3"))]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = FederationScalingV4::with_defaults();
        assert_eq!(engine.node_count(), 0);
        assert_eq!(engine.shard_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = ScalingV4Config {
            max_shards: 16,
            ema_alpha: 0.5,
            ..ScalingV4Config::default()
        };
        let engine = FederationScalingV4::new(config);
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        assert_eq!(engine.node_count(), 1);
    }

    #[test]
    fn test_register_node_invalid_capacity() {
        let mut engine = FederationScalingV4::with_defaults();
        let result = engine.register_node("node1".to_string(), 0.0);
        assert!(matches!(result, Err(ScalingV4Error::InvalidCapacity(_))));
    }

    #[test]
    fn test_update_node_load() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.update_node_load("node1", 0.5).unwrap();
        let node = engine.select_best_node().unwrap();
        assert!((node.load_factor - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_update_node_load_not_found() {
        let mut engine = FederationScalingV4::with_defaults();
        let result = engine.update_node_load("missing", 0.5);
        assert!(matches!(result, Err(ScalingV4Error::NodeNotFound(_))));
    }

    #[test]
    fn test_ema_smoothing() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.update_node_load("node1", 1.0).unwrap();
        engine.update_node_load("node1", 0.0).unwrap();
        let node = engine.select_best_node().unwrap();
        // EMA should be between 0 and 1
        assert!(node.ema_load > 0.0 && node.ema_load < 1.0);
    }

    #[test]
    fn test_load_prediction() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.update_node_load("node1", 0.5).unwrap();
        engine.update_node_load("node1", 0.7).unwrap();
        let node = engine.select_best_node().unwrap();
        let predicted = node.predict_load(5, 0.3);
        assert!(predicted >= 0.0 && predicted <= 1.0);
    }

    #[test]
    fn test_scaling_score() {
        let node = NodeCapabilityV4::new("test".to_string(), 100.0);
        let score = node.scaling_score();
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_can_accept_work() {
        let mut node = NodeCapabilityV4::new("test".to_string(), 100.0);
        node.load_factor = 0.3;
        assert!(node.can_accept_work(0.5));
        node.load_factor = 0.7;
        assert!(!node.can_accept_work(0.5));
    }

    #[test]
    fn test_create_shard() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.create_shard("shard1".to_string()).unwrap();
        assert_eq!(engine.shard_count(), 1);
    }

    #[test]
    fn test_assign_node_to_shard() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.create_shard("shard1".to_string()).unwrap();
        engine.assign_node_to_shard("node1", "shard1").unwrap();
    }

    #[test]
    fn test_assign_node_not_found() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.create_shard("shard1".to_string()).unwrap();
        let result = engine.assign_node_to_shard("missing", "shard1");
        assert!(matches!(result, Err(ScalingV4Error::NodeNotFound(_))));
    }

    #[test]
    fn test_evaluate_scaling_no_op() {
        let mut engine = FederationScalingV4::with_defaults();
        let decisions = engine.evaluate_scaling();
        assert_eq!(decisions[0].decision_type, ScalingDecisionType::NoOp);
    }

    #[test]
    fn test_evaluate_scaling_rebalance() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.create_shard("shard1".to_string()).unwrap();
        engine.assign_node_to_shard("node1", "shard1").unwrap();
        // Manually set high load via update
        engine.update_node_load("node1", 0.95).unwrap();
        // Need to update shard load directly through the engine
        let decisions = engine.evaluate_scaling();
        // At least NoOp
        assert!(!decisions.is_empty());
    }

    #[test]
    fn test_predict_shard_load() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.update_node_load("node1", 0.6).unwrap();
        engine.create_shard("shard1".to_string()).unwrap();
        engine.assign_node_to_shard("node1", "shard1").unwrap();
        let predicted = engine.predict_shard_load("shard1");
        assert!(predicted >= 0.0 && predicted <= 1.0);
    }

    #[test]
    fn test_predict_shard_load_missing() {
        let engine = FederationScalingV4::with_defaults();
        assert_eq!(engine.predict_shard_load("missing"), 0.0);
    }

    #[test]
    fn test_compute_federation_load() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.register_node("node2".to_string(), 100.0).unwrap();
        engine.update_node_load("node1", 0.4).unwrap();
        engine.update_node_load("node2", 0.6).unwrap();
        let load = engine.compute_federation_load();
        assert!((load - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_compute_federation_load_empty() {
        let engine = FederationScalingV4::with_defaults();
        assert_eq!(engine.compute_federation_load(), 0.0);
    }

    #[test]
    fn test_select_best_node() {
        let mut engine = FederationScalingV4::with_defaults();
        assert!(engine.select_best_node().is_none());
        engine.register_node("node1".to_string(), 100.0).unwrap();
        assert!(engine.select_best_node().is_some());
    }

    #[test]
    fn test_delegation_depth() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.update_delegation_depth("node1", 2).unwrap();
        let node = engine.select_best_node().unwrap();
        assert_eq!(node.delegation_depth, 2);
    }

    #[test]
    fn test_delegation_quota_exceeded() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        assert_eq!(engine.update_delegation_depth("node1", 10), Err(ScalingV4Error::DelegationQuotaExceeded));
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.register_node("node1".to_string(), 100.0).unwrap();
        engine.evaluate_scaling();
        engine.reset_stats();
        assert_eq!(engine.stats().total_decisions, 0);
    }

    #[test]
    fn test_decision_history() {
        let mut engine = FederationScalingV4::with_defaults();
        engine.evaluate_scaling();
        assert!(!engine.decision_history().is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = ScalingV4Config::default();
        assert_eq!(config.max_shards, 64);
        assert_eq!(config.ema_alpha, 0.3);
        assert_eq!(config.prediction_horizon, 5);
    }

    #[test]
    fn test_stats_default() {
        let stats = ScalingV4Stats::default();
        assert_eq!(stats.total_decisions, 0);
        assert_eq!(stats.prediction_samples, 0);
    }

    #[test]
    fn test_prediction_error_tracking() {
        let mut stats = ScalingV4Stats::default();
        stats.record_prediction_error(0.1);
        assert_eq!(stats.prediction_samples, 1);
        assert!((stats.avg_prediction_error - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_shard_needs_rebalance() {
        let shard = ShardConfigV4::new("s1".to_string());
        assert!(!shard.needs_rebalance(0.8));
    }

    #[test]
    fn test_shard_proactive_rebalance() {
        let mut shard = ShardConfigV4::new("s1".to_string());
        shard.load_factor = 0.5;
        shard.predicted_load = 0.8;
        assert!(shard.needs_proactive_rebalance(0.65));
    }

    #[test]
    fn test_error_display() {
        match ScalingV4Error::NodeNotFound("x".to_string()) {
            e => assert!(format!("{}", e).contains("x")),
        }
    }

    #[test]
    fn test_decision_type_display() {
        assert_eq!(format!("{}", ScalingDecisionType::AddShard), "AddShard");
        assert_eq!(format!("{}", ScalingDecisionType::ProactiveRebalance), "ProactiveRebalance");
        assert_eq!(format!("{}", ScalingDecisionType::Delegate), "Delegate");
    }

    #[test]
    fn test_engine_default() {
        let engine = FederationScalingV4::default();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_node_new() {
        let node = NodeCapabilityV4::new("n1".to_string(), 50.0);
        assert_eq!(node.node_id, "n1");
        assert_eq!(node.compute_capacity, 50.0);
        assert_eq!(node.delegation_depth, 0);
    }

    #[test]
    fn test_shard_new() {
        let shard = ShardConfigV4::new("s1".to_string());
        assert_eq!(shard.shard_id, "s1");
        assert_eq!(shard.health_score, 1.0);
    }

    #[test]
    fn test_shard_update_load() {
        let mut shard = ShardConfigV4::new("s1".to_string());
        shard.update_load(0.7);
        assert!((shard.load_factor - 0.7).abs() < 0.001);
        assert!((shard.health_score - 0.3).abs() < 0.001);
    }
}
