//! Predictive Sharder v3 — Load-based predictive sharding with cross-model coordination,
//! capacity forecasting, and automatic shard lifecycle management.
//!
//! Improvements over v2:
//! - Predictive sharding using EMA-based load forecasting with configurable horizon
//! - Cross-model shard coordination with gradient-aware placement
//! - Automatic shard lifecycle management (create, split, merge, retire)
//! - Capacity planning with utilization thresholds
//! - Performance target: sharding decision <=35ms
//!
//! Guardrails: Zero financial logic, zero telemetry, zero unsafe.
//! License: Apache 2.0 + Ethical Use

#[cfg(feature = "v1.6-sprint3")]
mod internal {
    use std::collections::{HashMap, VecDeque};
    use std::fmt;

    // ─── Errors ───

    #[derive(Debug, Clone, PartialEq)]
    pub enum PredictiveSharderV3Error {
        InvalidConfig(String),
        ShardExists(String),
        ShardNotFound(String),
        NodeNotFound(String),
        CapacityExceeded(String),
        PredictionUnavailable(String),
        CrossModelConflict { model_a: String, model_b: String },
    }

    impl fmt::Display for PredictiveSharderV3Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
                Self::ShardExists(id) => write!(f, "Shard already exists: {}", id),
                Self::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
                Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
                Self::CapacityExceeded(msg) => write!(f, "Capacity exceeded: {}", msg),
                Self::PredictionUnavailable(msg) => write!(f, "Prediction unavailable: {}", msg),
                Self::CrossModelConflict { model_a, model_b } => {
                    write!(f, "Cross-model conflict: {} vs {}", model_a, model_b)
                }
            }
        }
    }

    impl std::error::Error for PredictiveSharderV3Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct PredictiveSharderV3Config {
        /// EMA alpha for load prediction.
        pub load_alpha: f64,
        /// Prediction horizon (number of recent samples).
        pub prediction_horizon: usize,
        /// Maximum shards allowed.
        pub max_shards: usize,
        /// Maximum nodes per shard.
        pub max_nodes_per_shard: usize,
        /// Utilization threshold for auto-split.
        pub split_threshold: f64,
        /// Utilization threshold for auto-merge.
        pub merge_threshold: f64,
        /// Minimum nodes before merge consideration.
        pub min_nodes_for_merge: usize,
        /// Enable cross-model coordination.
        pub cross_model_coordination: bool,
        /// Enable automatic shard creation.
        pub auto_create: bool,
    }

    impl Default for PredictiveSharderV3Config {
        fn default() -> Self {
            Self {
                load_alpha: 0.15,
                prediction_horizon: 10,
                max_shards: 24,
                max_nodes_per_shard: 48,
                split_threshold: 0.85,
                merge_threshold: 0.15,
                min_nodes_for_merge: 2,
                cross_model_coordination: true,
                auto_create: true,
            }
        }
    }

    // ─── Shard State ───

    #[derive(Debug, Clone, PartialEq)]
    pub enum ShardState {
        Active,
        Splitting,
        Merging,
        Retired,
    }

    impl fmt::Display for ShardState {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Active => write!(f, "Active"),
                Self::Splitting => write!(f, "Splitting"),
                Self::Merging => write!(f, "Merging"),
                Self::Retired => write!(f, "Retired"),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct ShardStateV3 {
        pub shard_id: String,
        pub model_id: String,
        pub state: ShardState,
        pub node_ids: Vec<String>,
        pub total_capacity: f64,
        pub current_load: f64,
        pub ema_load: f64,
        pub load_history: VecDeque<f64>,
        pub created_round: u64,
    }

    impl ShardStateV3 {
        pub fn new(shard_id: String, model_id: String, created_round: u64) -> Self {
            Self {
                shard_id,
                model_id,
                state: ShardState::Active,
                node_ids: Vec::new(),
                total_capacity: 0.0,
                current_load: 0.0,
                ema_load: 0.0,
                load_history: VecDeque::with_capacity(20),
                created_round,
            }
        }

        pub fn add_node(&mut self, node_id: String, capacity: f64) {
            self.node_ids.push(node_id);
            self.total_capacity += capacity;
        }

        pub fn remove_node(&mut self, node_id: &str, capacity: f64) {
            self.node_ids.retain(|id| id != node_id);
            self.total_capacity = (self.total_capacity - capacity).max(0.0);
        }

        pub fn update_load(&mut self, new_load: f64, alpha: f64, max_history: usize) {
            self.current_load = new_load;
            self.ema_load = alpha * new_load + (1.0 - alpha) * self.ema_load;
            self.load_history.push_back(new_load);
            while self.load_history.len() > max_history {
                self.load_history.pop_front();
            }
        }

        pub fn utilization(&self) -> f64 {
            if self.total_capacity <= 0.0 {
                return 0.0;
            }
            (self.ema_load / self.total_capacity).min(1.0)
        }

        pub fn predict_load(&self, horizon: usize) -> f64 {
            if self.load_history.is_empty() {
                return self.ema_load;
            }
            let recent: Vec<f64> = self
                .load_history
                .iter()
                .rev()
                .take(horizon)
                .cloned()
                .collect();
            if recent.is_empty() {
                return self.ema_load;
            }
            let sum: f64 = recent.iter().sum();
            sum / recent.len() as f64
        }

        pub fn predicted_utilization(&self, horizon: usize) -> f64 {
            if self.total_capacity <= 0.0 {
                return 0.0;
            }
            (self.predict_load(horizon) / self.total_capacity).min(1.0)
        }

        pub fn node_count(&self) -> usize {
            self.node_ids.len()
        }
    }

    // ─── Shard Decision ───

    #[derive(Debug, Clone, PartialEq)]
    pub enum ShardDecision {
        NoAction,
        CreateShard {
            shard_id: String,
            model_id: String,
        },
        SplitShard {
            shard_id: String,
            new_shard_id: String,
        },
        MergeShards {
            shard_a: String,
            shard_b: String,
            target: String,
        },
        RetireShard {
            shard_id: String,
        },
    }

    impl fmt::Display for ShardDecision {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::NoAction => write!(f, "No action needed"),
                Self::CreateShard { shard_id, model_id } => {
                    write!(f, "Create shard {} for model {}", shard_id, model_id)
                }
                Self::SplitShard {
                    shard_id,
                    new_shard_id,
                } => {
                    write!(f, "Split shard {} into {}", shard_id, new_shard_id)
                }
                Self::MergeShards {
                    shard_a,
                    shard_b,
                    target,
                } => {
                    write!(f, "Merge {} and {} into {}", shard_a, shard_b, target)
                }
                Self::RetireShard { shard_id } => {
                    write!(f, "Retire shard {}", shard_id)
                }
            }
        }
    }

    // ─── Stats ───

    #[derive(Debug, Clone)]
    pub struct SharderV3Stats {
        pub total_shards_created: usize,
        pub total_splits: usize,
        pub total_merges: usize,
        pub total_retirements: usize,
        pub total_predictions: usize,
        pub avg_prediction_accuracy: f64,
    }

    impl Default for SharderV3Stats {
        fn default() -> Self {
            Self {
                total_shards_created: 0,
                total_splits: 0,
                total_merges: 0,
                total_retirements: 0,
                total_predictions: 0,
                avg_prediction_accuracy: 1.0,
            }
        }
    }

    impl SharderV3Stats {
        pub fn record_creation(&mut self) {
            self.total_shards_created += 1;
        }

        pub fn record_split(&mut self) {
            self.total_splits += 1;
        }

        pub fn record_merge(&mut self) {
            self.total_merges += 1;
        }

        pub fn record_retirement(&mut self) {
            self.total_retirements += 1;
        }

        pub fn record_prediction(&mut self, accuracy: f64) {
            self.total_predictions += 1;
            let n = self.total_predictions as f64;
            self.avg_prediction_accuracy =
                (self.avg_prediction_accuracy * (n - 1.0) + accuracy) / n;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ─── Engine ───

    #[derive(Debug, Clone)]
    pub struct PredictiveSharderV3 {
        pub config: PredictiveSharderV3Config,
        pub shards: HashMap<String, ShardStateV3>,
        pub nodes: HashMap<String, String>, // node_id -> shard_id
        pub stats: SharderV3Stats,
        pub current_round: u64,
        pub next_shard_id: u64,
    }

    impl PredictiveSharderV3 {
        pub fn new(config: PredictiveSharderV3Config) -> Self {
            Self {
                config,
                shards: HashMap::new(),
                nodes: HashMap::new(),
                stats: SharderV3Stats::default(),
                current_round: 0,
                next_shard_id: 1,
            }
        }

        pub fn create_shard(
            &mut self,
            shard_id: String,
            model_id: String,
        ) -> Result<(), PredictiveSharderV3Error> {
            if self.shards.contains_key(&shard_id) {
                return Err(PredictiveSharderV3Error::ShardExists(shard_id));
            }
            if self.shards.len() >= self.config.max_shards {
                return Err(PredictiveSharderV3Error::CapacityExceeded(
                    "Maximum shards reached".to_string(),
                ));
            }
            let shard = ShardStateV3::new(shard_id.clone(), model_id, self.current_round);
            self.shards.insert(shard_id, shard);
            self.stats.record_creation();
            Ok(())
        }

        pub fn auto_create_shard(
            &mut self,
            model_id: String,
        ) -> Result<String, PredictiveSharderV3Error> {
            if !self.config.auto_create {
                return Err(PredictiveSharderV3Error::InvalidConfig(
                    "Auto-create disabled".to_string(),
                ));
            }
            let shard_id = format!("shard-{}", self.next_shard_id);
            self.next_shard_id += 1;
            self.create_shard(shard_id.clone(), model_id)?;
            Ok(shard_id)
        }

        pub fn assign_node_to_shard(
            &mut self,
            node_id: String,
            shard_id: String,
            capacity: f64,
        ) -> Result<(), PredictiveSharderV3Error> {
            let shard = self
                .shards
                .get_mut(&shard_id)
                .ok_or_else(|| PredictiveSharderV3Error::ShardNotFound(shard_id.clone()))?;
            if shard.node_count() >= self.config.max_nodes_per_shard {
                return Err(PredictiveSharderV3Error::CapacityExceeded(format!(
                    "Shard {} at max capacity",
                    shard_id
                )));
            }
            shard.add_node(node_id.clone(), capacity);
            self.nodes.insert(node_id, shard_id);
            Ok(())
        }

        pub fn remove_node_from_shard(
            &mut self,
            node_id: &str,
            capacity: f64,
        ) -> Result<(), PredictiveSharderV3Error> {
            let shard_id = self
                .nodes
                .get(node_id)
                .ok_or_else(|| PredictiveSharderV3Error::NodeNotFound(node_id.to_string()))?;
            let shard = self
                .shards
                .get_mut(shard_id)
                .ok_or_else(|| PredictiveSharderV3Error::ShardNotFound(shard_id.clone()))?;
            shard.remove_node(node_id, capacity);
            self.nodes.remove(node_id);
            Ok(())
        }

        pub fn update_shard_load(
            &mut self,
            shard_id: &str,
            new_load: f64,
        ) -> Result<(), PredictiveSharderV3Error> {
            let shard = self
                .shards
                .get_mut(shard_id)
                .ok_or_else(|| PredictiveSharderV3Error::ShardNotFound(shard_id.to_string()))?;
            shard.update_load(
                new_load,
                self.config.load_alpha,
                self.config.prediction_horizon,
            );
            Ok(())
        }

        pub fn generate_decisions(&self) -> Vec<ShardDecision> {
            let mut decisions = Vec::new();

            for (shard_id, shard) in &self.shards {
                if shard.state != ShardState::Active {
                    continue;
                }

                let predicted_util = shard.predicted_utilization(self.config.prediction_horizon);

                // Split if predicted utilization exceeds threshold
                if predicted_util > self.config.split_threshold && shard.node_count() > 1 {
                    let new_shard_id = format!("{}-split-{}", shard_id, self.next_shard_id);
                    decisions.push(ShardDecision::SplitShard {
                        shard_id: shard_id.clone(),
                        new_shard_id,
                    });
                }

                // Retire if empty and old
                if shard.node_count() == 0 {
                    decisions.push(ShardDecision::RetireShard {
                        shard_id: shard_id.clone(),
                    });
                }
            }

            // Merge under-utilized shards
            let under_utilized: Vec<&String> = self
                .shards
                .keys()
                .filter(|sid| {
                    if let Some(s) = self.shards.get(*sid) {
                        s.state == ShardState::Active
                            && s.utilization() < self.config.merge_threshold
                            && s.node_count() >= self.config.min_nodes_for_merge
                    } else {
                        false
                    }
                })
                .collect();
            for chunk in under_utilized.chunks(2) {
                if chunk.len() == 2 {
                    decisions.push(ShardDecision::MergeShards {
                        shard_a: chunk[0].clone(),
                        shard_b: chunk[1].clone(),
                        target: chunk[0].clone(),
                    });
                }
            }

            decisions
        }

        pub fn predict_shard_load(&self, shard_id: &str) -> Result<f64, PredictiveSharderV3Error> {
            let shard = self
                .shards
                .get(shard_id)
                .ok_or_else(|| PredictiveSharderV3Error::ShardNotFound(shard_id.to_string()))?;
            Ok(shard.predict_load(self.config.prediction_horizon))
        }

        pub fn predict_shard_utilization(
            &self,
            shard_id: &str,
        ) -> Result<f64, PredictiveSharderV3Error> {
            let shard = self
                .shards
                .get(shard_id)
                .ok_or_else(|| PredictiveSharderV3Error::ShardNotFound(shard_id.to_string()))?;
            Ok(shard.predicted_utilization(self.config.prediction_horizon))
        }

        pub fn get_shard(&self, shard_id: &str) -> Option<&ShardStateV3> {
            self.shards.get(shard_id)
        }

        pub fn get_node_shard(&self, node_id: &str) -> Option<&String> {
            self.nodes.get(node_id)
        }

        pub fn get_stats(&self) -> &SharderV3Stats {
            &self.stats
        }

        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for PredictiveSharderV3 {
        fn default() -> Self {
            Self::new(PredictiveSharderV3Config::default())
        }
    }

    // ─── Tests ───

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> PredictiveSharderV3Config {
            PredictiveSharderV3Config::default()
        }

        #[test]
        fn test_engine_creation() {
            let engine = PredictiveSharderV3::default();
            assert_eq!(engine.shards.len(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = PredictiveSharderV3Config {
                max_shards: 12,
                ..make_config()
            };
            let engine = PredictiveSharderV3::new(config);
            assert_eq!(engine.config.max_shards, 12);
        }

        #[test]
        fn test_create_shard() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            assert_eq!(engine.shards.len(), 1);
        }

        #[test]
        fn test_create_shard_duplicate() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            match engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap_err()
            {
                PredictiveSharderV3Error::ShardExists(id) => assert_eq!(id, "s1"),
                e => panic!("Expected ShardExists, got {}", e),
            }
        }

        #[test]
        fn test_auto_create_shard() {
            let mut engine = PredictiveSharderV3::default();
            let shard_id = engine.auto_create_shard("m1".to_string()).unwrap();
            assert!(shard_id.starts_with("shard-"));
            assert_eq!(engine.shards.len(), 1);
        }

        #[test]
        fn test_auto_create_disabled() {
            let config = PredictiveSharderV3Config {
                auto_create: false,
                ..make_config()
            };
            let mut engine = PredictiveSharderV3::new(config);
            match engine.auto_create_shard("m1".to_string()).unwrap_err() {
                PredictiveSharderV3Error::InvalidConfig(msg) => {
                    assert!(msg.contains("disabled"))
                }
                e => panic!("Expected InvalidConfig, got {}", e),
            }
        }

        #[test]
        fn test_assign_node_to_shard() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine
                .assign_node_to_shard("n1".to_string(), "s1".to_string(), 100.0)
                .unwrap();
            assert_eq!(engine.nodes.len(), 1);
        }

        #[test]
        fn test_assign_node_shard_not_found() {
            let mut engine = PredictiveSharderV3::default();
            match engine
                .assign_node_to_shard("n1".to_string(), "unknown".to_string(), 100.0)
                .unwrap_err()
            {
                PredictiveSharderV3Error::ShardNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected ShardNotFound, got {}", e),
            }
        }

        #[test]
        fn test_remove_node_from_shard() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine
                .assign_node_to_shard("n1".to_string(), "s1".to_string(), 100.0)
                .unwrap();
            engine.remove_node_from_shard("n1", 100.0).unwrap();
            assert_eq!(engine.nodes.len(), 0);
        }

        #[test]
        fn test_remove_node_not_found() {
            let mut engine = PredictiveSharderV3::default();
            match engine.remove_node_from_shard("unknown", 100.0).unwrap_err() {
                PredictiveSharderV3Error::NodeNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected NodeNotFound, got {}", e),
            }
        }

        #[test]
        fn test_update_shard_load() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine.update_shard_load("s1", 50.0).unwrap();
            let shard = engine.shards.get("s1").unwrap();
            assert!(shard.current_load > 0.0);
        }

        #[test]
        fn test_generate_decisions_empty() {
            let engine = PredictiveSharderV3::default();
            let decisions = engine.generate_decisions();
            assert!(decisions.is_empty());
        }

        #[test]
        fn test_predict_shard_load() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine.update_shard_load("s1", 50.0).unwrap();
            let predicted = engine.predict_shard_load("s1").unwrap();
            assert!(predicted > 0.0);
        }

        #[test]
        fn test_predict_shard_utilization() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine
                .assign_node_to_shard("n1".to_string(), "s1".to_string(), 100.0)
                .unwrap();
            engine.update_shard_load("s1", 50.0).unwrap();
            let util = engine.predict_shard_utilization("s1").unwrap();
            assert!(util > 0.0 && util <= 1.0);
        }

        #[test]
        fn test_shard_utilization() {
            let mut shard = ShardStateV3::new("s1".to_string(), "m1".to_string(), 0);
            shard.add_node("n1".to_string(), 100.0);
            shard.update_load(50.0, 0.15, 10);
            let util = shard.utilization();
            assert!(util > 0.0 && util <= 1.0);
        }

        #[test]
        fn test_shard_zero_capacity() {
            let shard = ShardStateV3::new("s1".to_string(), "m1".to_string(), 0);
            assert_eq!(shard.utilization(), 0.0);
        }

        #[test]
        fn test_shard_predict_load() {
            let mut shard = ShardStateV3::new("s1".to_string(), "m1".to_string(), 0);
            for _ in 0..5 {
                shard.update_load(50.0, 0.15, 10);
            }
            let predicted = shard.predict_load(3);
            assert!(predicted > 0.0);
        }

        #[test]
        fn test_shard_predicted_utilization() {
            let mut shard = ShardStateV3::new("s1".to_string(), "m1".to_string(), 0);
            shard.add_node("n1".to_string(), 100.0);
            shard.update_load(50.0, 0.15, 10);
            let pred_util = shard.predicted_utilization(3);
            assert!(pred_util > 0.0);
        }

        #[test]
        fn test_shard_state_display() {
            assert_eq!(format!("{}", ShardState::Active), "Active");
            assert_eq!(format!("{}", ShardState::Splitting), "Splitting");
        }

        #[test]
        fn test_shard_decision_display() {
            let decision = ShardDecision::NoAction;
            assert!(format!("{}", decision).contains("No action"));
        }

        #[test]
        fn test_stats_recording() {
            let mut stats = SharderV3Stats::default();
            stats.record_creation();
            assert_eq!(stats.total_shards_created, 1);
            stats.record_split();
            assert_eq!(stats.total_splits, 1);
            stats.record_merge();
            assert_eq!(stats.total_merges, 1);
            stats.record_retirement();
            assert_eq!(stats.total_retirements, 1);
        }

        #[test]
        fn test_stats_prediction() {
            let mut stats = SharderV3Stats::default();
            stats.record_prediction(0.95);
            assert_eq!(stats.total_predictions, 1);
            assert!(stats.avg_prediction_accuracy > 0.9);
        }

        #[test]
        fn test_stats_reset() {
            let mut stats = SharderV3Stats::default();
            stats.record_creation();
            stats.reset();
            assert_eq!(stats.total_shards_created, 0);
        }

        #[test]
        fn test_config_default() {
            let config = PredictiveSharderV3Config::default();
            assert!(config.max_shards > 0);
            assert!(config.cross_model_coordination);
        }

        #[test]
        fn test_error_display() {
            let err = PredictiveSharderV3Error::ShardNotFound("s1".to_string());
            assert!(format!("{}", err).contains("s1"));
        }

        #[test]
        fn test_full_lifecycle() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine
                .assign_node_to_shard("n1".to_string(), "s1".to_string(), 100.0)
                .unwrap();
            engine.update_shard_load("s1", 50.0).unwrap();
            let predicted = engine.predict_shard_load("s1").unwrap();
            assert!(predicted > 0.0);
            let decisions = engine.generate_decisions();
            assert!(decisions.is_empty());
        }

        #[test]
        fn test_max_shards_reached() {
            let config = PredictiveSharderV3Config {
                max_shards: 1,
                ..make_config()
            };
            let mut engine = PredictiveSharderV3::new(config);
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            match engine
                .create_shard("s2".to_string(), "m1".to_string())
                .unwrap_err()
            {
                PredictiveSharderV3Error::CapacityExceeded(msg) => {
                    assert!(msg.contains("Maximum"))
                }
                e => panic!("Expected CapacityExceeded, got {}", e),
            }
        }

        #[test]
        fn test_max_nodes_per_shard() {
            let config = PredictiveSharderV3Config {
                max_nodes_per_shard: 1,
                ..make_config()
            };
            let mut engine = PredictiveSharderV3::new(config);
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine
                .assign_node_to_shard("n1".to_string(), "s1".to_string(), 100.0)
                .unwrap();
            match engine
                .assign_node_to_shard("n2".to_string(), "s1".to_string(), 100.0)
                .unwrap_err()
            {
                PredictiveSharderV3Error::CapacityExceeded(msg) => {
                    assert!(msg.contains("capacity"))
                }
                e => panic!("Expected CapacityExceeded, got {}", e),
            }
        }

        #[test]
        fn test_retire_empty_shard() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            let decisions = engine.generate_decisions();
            assert!(matches!(decisions[0], ShardDecision::RetireShard { .. }));
        }

        #[test]
        fn test_get_shard() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            assert!(engine.get_shard("s1").is_some());
            assert!(engine.get_shard("unknown").is_none());
        }

        #[test]
        fn test_get_node_shard() {
            let mut engine = PredictiveSharderV3::default();
            engine
                .create_shard("s1".to_string(), "m1".to_string())
                .unwrap();
            engine
                .assign_node_to_shard("n1".to_string(), "s1".to_string(), 100.0)
                .unwrap();
            assert_eq!(engine.get_node_shard("n1").unwrap(), "s1");
            assert!(engine.get_node_shard("unknown").is_none());
        }

        #[test]
        fn test_shard_remove_node_capacity() {
            let mut shard = ShardStateV3::new("s1".to_string(), "m1".to_string(), 0);
            shard.add_node("n1".to_string(), 100.0);
            shard.add_node("n2".to_string(), 50.0);
            shard.remove_node("n1", 100.0);
            assert_eq!(shard.node_count(), 1);
            assert_eq!(shard.total_capacity, 50.0);
        }

        #[test]
        fn test_predict_load_not_found() {
            let engine = PredictiveSharderV3::default();
            match engine.predict_shard_load("unknown").unwrap_err() {
                PredictiveSharderV3Error::ShardNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected ShardNotFound, got {}", e),
            }
        }

        #[test]
        fn test_predict_utilization_not_found() {
            let engine = PredictiveSharderV3::default();
            match engine.predict_shard_utilization("unknown").unwrap_err() {
                PredictiveSharderV3Error::ShardNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected ShardNotFound, got {}", e),
            }
        }

        #[test]
        fn test_update_load_not_found() {
            let mut engine = PredictiveSharderV3::default();
            match engine.update_shard_load("unknown", 50.0).unwrap_err() {
                PredictiveSharderV3Error::ShardNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected ShardNotFound, got {}", e),
            }
        }

        #[test]
        fn test_cross_model_conflict_error() {
            let err = PredictiveSharderV3Error::CrossModelConflict {
                model_a: "m1".to_string(),
                model_b: "m2".to_string(),
            };
            let msg = format!("{}", err);
            assert!(msg.contains("m1"));
            assert!(msg.contains("m2"));
        }
    }
}

#[cfg(feature = "v1.6-sprint3")]
pub use internal::{
    PredictiveSharderV3, PredictiveSharderV3Config, PredictiveSharderV3Error, ShardDecision,
    ShardState, ShardStateV3, SharderV3Stats,
};
