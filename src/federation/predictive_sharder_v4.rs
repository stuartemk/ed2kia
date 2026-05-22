//! Predictive Sharder v4 — ML-based shard placement with load forecasting.
//!
//! Uses exponential moving averages and trend analysis to predict optimal
//! shard placement before load spikes occur, enabling proactive rebalancing.
//!
//! Feature-gated: `#[cfg(feature = "v1.4-sprint3")]`

mod internal {
    use std::collections::{HashMap, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for predictive sharding v4.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PredictiveSharderError {
        /// Shard not found.
        ShardNotFound(String),
        /// Node not found.
        NodeNotFound(String),
        /// Insufficient nodes for shard.
        InsufficientNodes(usize),
        /// Prediction window not warm.
        PredictionNotWarm,
        /// Shard already exists.
        ShardExists(String),
    }

    impl std::fmt::Display for PredictiveSharderError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PredictiveSharderError::ShardNotFound(id) => write!(f, "Shard {} not found", id),
                PredictiveSharderError::NodeNotFound(id) => write!(f, "Node {} not found", id),
                PredictiveSharderError::InsufficientNodes(min) => {
                    write!(f, "Insufficient nodes: need at least {}", min)
                }
                PredictiveSharderError::PredictionNotWarm => {
                    write!(f, "Prediction window not warm")
                }
                PredictiveSharderError::ShardExists(id) => write!(f, "Shard {} already exists", id),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for predictive sharder v4.
    #[derive(Debug, Clone)]
    pub struct PredictiveSharderConfig {
        /// EMA alpha for load smoothing.
        pub ema_alpha: f64,
        /// Minimum samples before prediction is valid.
        pub min_warm_samples: usize,
        /// Prediction horizon (steps ahead).
        pub prediction_horizon: usize,
        /// Proactive threshold for early rebalance.
        pub proactive_threshold: f64,
        /// Maximum shards.
        pub max_shards: usize,
        /// Minimum nodes per shard.
        pub min_nodes_per_shard: usize,
        /// Trend sensitivity (higher = more reactive).
        pub trend_sensitivity: f64,
    }

    impl Default for PredictiveSharderConfig {
        fn default() -> Self {
            Self {
                ema_alpha: 0.3,
                min_warm_samples: 5,
                prediction_horizon: 3,
                proactive_threshold: 0.70,
                max_shards: 32,
                min_nodes_per_shard: 2,
                trend_sensitivity: 0.5,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Load History
    // ---------------------------------------------------------------------------

    /// Load history with EMA tracking for a single node.
    #[derive(Debug, Clone)]
    pub struct LoadHistory {
        /// Recent load samples.
        pub samples: VecDeque<f64>,
        /// Current EMA value.
        pub ema: f64,
        /// Load trend (positive = increasing).
        pub trend: f64,
        /// Sample count.
        pub count: usize,
    }

    impl LoadHistory {
        pub fn new(max_samples: usize) -> Self {
            Self {
                samples: VecDeque::with_capacity(max_samples),
                ema: 0.0,
                trend: 0.0,
                count: 0,
            }
        }

        pub fn record(&mut self, load: f64, alpha: f64, max_samples: usize) {
            let old_ema = self.ema;
            if self.count == 0 {
                self.ema = load;
            } else {
                self.ema = alpha * load + (1.0 - alpha) * self.ema;
            }
            self.trend = self.ema - old_ema;
            self.samples.push_back(load);
            while self.samples.len() > max_samples {
                self.samples.pop_front();
            }
            self.count += 1;
        }

        pub fn is_warm(&self, min_samples: usize) -> bool {
            self.count >= min_samples
        }

        pub fn predict(&self, horizon: usize, alpha: f64) -> f64 {
            let trend_per_step = self.trend * alpha;
            (self.ema + trend_per_step * horizon as f64).clamp(0.0, 1.0)
        }

        pub fn avg_recent(&self) -> f64 {
            if self.samples.is_empty() {
                return 0.0;
            }
            let sum: f64 = self.samples.iter().sum();
            sum / self.samples.len() as f64
        }
    }

    // ---------------------------------------------------------------------------
    // Shard Placement
    // ---------------------------------------------------------------------------

    /// Shard placement record with prediction data.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ShardPlacement {
        /// Shard identifier.
        pub shard_id: String,
        /// Assigned node IDs.
        pub nodes: Vec<String>,
        /// Current load factor.
        pub load_factor: f64,
        /// Predicted load factor.
        pub predicted_load: f64,
        /// Placement score (higher = better).
        pub placement_score: f64,
        /// Needs proactive rebalance.
        pub proactive_rebalance: bool,
    }

    impl ShardPlacement {
        pub fn new(shard_id: String) -> Self {
            Self {
                shard_id,
                nodes: Vec::new(),
                load_factor: 0.0,
                predicted_load: 0.0,
                placement_score: 1.0,
                proactive_rebalance: false,
            }
        }

        pub fn update_prediction(&mut self, predicted_load: f64, threshold: f64) {
            self.predicted_load = predicted_load;
            self.proactive_rebalance = predicted_load > threshold && self.load_factor <= threshold;
            self.placement_score = (1.0 - self.predicted_load).max(0.0);
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for predictive sharder.
    #[derive(Debug, Clone)]
    pub struct SharderStats {
        pub total_placements: usize,
        pub total_rebalances: usize,
        pub total_proactive_rebalances: usize,
        pub total_predictions: usize,
        pub avg_prediction_accuracy: f64,
        pub active_shards: usize,
        pub warm_nodes: usize,
    }

    impl Default for SharderStats {
        fn default() -> Self {
            Self {
                total_placements: 0,
                total_rebalances: 0,
                total_proactive_rebalances: 0,
                total_predictions: 0,
                avg_prediction_accuracy: 1.0,
                active_shards: 0,
                warm_nodes: 0,
            }
        }
    }

    impl SharderStats {
        pub fn record_accuracy(&mut self, accuracy: f64) {
            let n = self.total_predictions + 1;
            self.avg_prediction_accuracy = self.avg_prediction_accuracy
                * (self.total_predictions as f64 / n as f64)
                + accuracy / n as f64;
            self.total_predictions = n;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Main Sharder
    // ---------------------------------------------------------------------------

    /// Predictive Sharder v4 engine.
    pub struct PredictiveSharderV4 {
        pub config: PredictiveSharderConfig,
        load_histories: HashMap<String, LoadHistory>,
        placements: HashMap<String, ShardPlacement>,
        stats: SharderStats,
    }

    impl PredictiveSharderV4 {
        pub fn new(config: PredictiveSharderConfig) -> Self {
            Self {
                config,
                load_histories: HashMap::new(),
                placements: HashMap::new(),
                stats: SharderStats::default(),
            }
        }

        pub fn with_defaults() -> Self {
            Self::new(PredictiveSharderConfig::default())
        }

        /// Register a node for load tracking.
        pub fn register_node(&mut self, node_id: String) {
            let history = LoadHistory::new(self.config.min_warm_samples * 2);
            self.load_histories.insert(node_id, history);
        }

        /// Record load sample for a node.
        pub fn record_load(
            &mut self,
            node_id: &str,
            load: f64,
        ) -> Result<(), PredictiveSharderError> {
            let history = self
                .load_histories
                .get_mut(node_id)
                .ok_or(PredictiveSharderError::NodeNotFound(node_id.to_string()))?;
            history.record(
                load,
                self.config.ema_alpha,
                self.config.min_warm_samples * 2,
            );
            Ok(())
        }

        /// Create a shard placement.
        pub fn create_shard(
            &mut self,
            shard_id: String,
            nodes: Vec<String>,
        ) -> Result<ShardPlacement, PredictiveSharderError> {
            if self.placements.contains_key(&shard_id) {
                return Err(PredictiveSharderError::ShardExists(shard_id.clone()));
            }
            if nodes.len() < self.config.min_nodes_per_shard {
                return Err(PredictiveSharderError::InsufficientNodes(
                    self.config.min_nodes_per_shard,
                ));
            }
            if self.placements.len() >= self.config.max_shards {
                return Err(PredictiveSharderError::ShardNotFound(format!(
                    "Max shards ({}) reached",
                    self.config.max_shards
                )));
            }
            for node in &nodes {
                if !self.load_histories.contains_key(node.as_str()) {
                    return Err(PredictiveSharderError::NodeNotFound(node.clone()));
                }
            }
            let mut placement = ShardPlacement::new(shard_id.clone());
            placement.nodes = nodes;
            self.update_placement_prediction(&mut placement);
            self.placements.insert(shard_id, placement.clone());
            self.stats.total_placements += 1;
            self.stats.active_shards = self.placements.len();
            Ok(placement)
        }

        /// Predict load for a node.
        pub fn predict_node_load(&self, node_id: &str) -> Result<f64, PredictiveSharderError> {
            let history = self
                .load_histories
                .get(node_id)
                .ok_or(PredictiveSharderError::NodeNotFound(node_id.to_string()))?;
            if !history.is_warm(self.config.min_warm_samples) {
                return Err(PredictiveSharderError::PredictionNotWarm);
            }
            Ok(history.predict(self.config.prediction_horizon, self.config.ema_alpha))
        }

        /// Predict load for a shard (average of node predictions).
        pub fn predict_shard_load(&self, shard_id: &str) -> Result<f64, PredictiveSharderError> {
            let placement = self
                .placements
                .get(shard_id)
                .ok_or(PredictiveSharderError::ShardNotFound(shard_id.to_string()))?;
            if placement.nodes.is_empty() {
                return Ok(0.0);
            }
            let mut total = 0.0;
            let mut count = 0;
            for node_id in &placement.nodes {
                if let Some(history) = self.load_histories.get(node_id) {
                    if history.is_warm(self.config.min_warm_samples) {
                        total +=
                            history.predict(self.config.prediction_horizon, self.config.ema_alpha);
                        count += 1;
                    }
                }
            }
            if count == 0 {
                return Ok(0.0);
            }
            Ok(total / count as f64)
        }

        /// Evaluate all placements for proactive rebalance needs.
        pub fn evaluate_placements(&mut self) -> Vec<(String, bool)> {
            let mut results = Vec::new();
            let threshold = self.config.proactive_threshold;
            let min_warm = self.config.min_warm_samples;
            let horizon = self.config.prediction_horizon;
            let ema_alpha = self.config.ema_alpha;
            for (shard_id, placement) in self.placements.iter_mut() {
                if placement.nodes.is_empty() {
                    placement.predicted_load = 0.0;
                    placement.proactive_rebalance = false;
                    placement.placement_score = 1.0;
                    results.push((shard_id.clone(), false));
                    continue;
                }
                let mut total = 0.0;
                let mut count = 0;
                for node_id in &placement.nodes {
                    if let Some(history) = self.load_histories.get(node_id) {
                        if history.is_warm(min_warm) {
                            total += history.predict(horizon, ema_alpha);
                            count += 1;
                        }
                    }
                }
                placement.predicted_load = if count > 0 { total / count as f64 } else { 0.0 };
                placement.proactive_rebalance =
                    placement.predicted_load > threshold && placement.load_factor <= threshold;
                placement.placement_score = (1.0 - placement.predicted_load).max(0.0);
                let needs_rebalance = placement.proactive_rebalance;
                if needs_rebalance {
                    self.stats.total_proactive_rebalances += 1;
                }
                results.push((shard_id.clone(), needs_rebalance));
            }
            results
        }

        /// Get warm node count.
        pub fn warm_node_count(&self) -> usize {
            self.load_histories
                .values()
                .filter(|h| h.is_warm(self.config.min_warm_samples))
                .count()
        }

        /// Get placement for a shard.
        pub fn get_placement(&self, shard_id: &str) -> Option<&ShardPlacement> {
            self.placements.get(shard_id)
        }

        /// Remove a shard.
        pub fn remove_shard(&mut self, shard_id: &str) -> Result<(), PredictiveSharderError> {
            self.placements
                .remove(shard_id)
                .ok_or(PredictiveSharderError::ShardNotFound(shard_id.to_string()))?;
            self.stats.active_shards = self.placements.len();
            Ok(())
        }

        /// Get stats reference.
        pub fn stats(&self) -> &SharderStats {
            &self.stats
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Get node count.
        pub fn node_count(&self) -> usize {
            self.load_histories.len()
        }

        /// Get shard count.
        pub fn shard_count(&self) -> usize {
            self.placements.len()
        }

        fn update_placement_prediction(&self, placement: &mut ShardPlacement) {
            if placement.nodes.is_empty() {
                placement.predicted_load = 0.0;
                return;
            }
            let mut total = 0.0;
            let mut count = 0;
            for node_id in &placement.nodes {
                if let Some(history) = self.load_histories.get(node_id) {
                    if history.is_warm(self.config.min_warm_samples) {
                        total +=
                            history.predict(self.config.prediction_horizon, self.config.ema_alpha);
                        count += 1;
                    }
                }
            }
            placement.predicted_load = if count > 0 { total / count as f64 } else { 0.0 };
            placement.proactive_rebalance = placement.predicted_load
                > self.config.proactive_threshold
                && placement.load_factor <= self.config.proactive_threshold;
            placement.placement_score = (1.0 - placement.predicted_load).max(0.0);
        }
    }

    impl Default for PredictiveSharderV4 {
        fn default() -> Self {
            Self::with_defaults()
        }
    }
}

#[cfg(feature = "v1.4-sprint3")]
pub use internal::*;

#[cfg(all(test, feature = "v1.4-sprint3"))]
mod tests {
    use super::*;

    #[test]
    fn test_sharder_creation() {
        let sharder = PredictiveSharderV4::with_defaults();
        assert_eq!(sharder.node_count(), 0);
        assert_eq!(sharder.shard_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        assert_eq!(sharder.node_count(), 1);
    }

    #[test]
    fn test_record_load() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.record_load("node1", 0.5).unwrap();
    }

    #[test]
    fn test_record_load_node_not_found() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        let result = sharder.record_load("missing", 0.5);
        assert!(matches!(
            result,
            Err(PredictiveSharderError::NodeNotFound(_))
        ));
    }

    #[test]
    fn test_prediction_not_warm() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.record_load("node1", 0.5).unwrap();
        assert_eq!(
            sharder.predict_node_load("node1"),
            Err(PredictiveSharderError::PredictionNotWarm)
        );
    }

    #[test]
    fn test_prediction_after_warmup() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        for i in 0..10 {
            sharder.record_load("node1", 0.3 + i as f64 * 0.05).unwrap();
        }
        let predicted = sharder.predict_node_load("node1").unwrap();
        assert!(predicted >= 0.0 && predicted <= 1.0);
    }

    #[test]
    fn test_create_shard() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        let placement = sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        assert_eq!(placement.shard_id, "shard1");
        assert_eq!(placement.nodes.len(), 2);
        assert_eq!(sharder.shard_count(), 1);
    }

    #[test]
    fn test_create_shard_duplicate() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        let result = sharder.create_shard(
            "shard1".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );
        assert!(matches!(
            result,
            Err(PredictiveSharderError::ShardExists(_))
        ));
    }

    #[test]
    fn test_create_shard_insufficient_nodes() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        let result = sharder.create_shard("shard1".to_string(), vec!["node1".to_string()]);
        assert!(matches!(
            result,
            Err(PredictiveSharderError::InsufficientNodes(_))
        ));
    }

    #[test]
    fn test_create_shard_node_not_registered() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        let result = sharder.create_shard(
            "shard1".to_string(),
            vec!["node1".to_string(), "unknown".to_string()],
        );
        assert!(matches!(
            result,
            Err(PredictiveSharderError::NodeNotFound(_))
        ));
    }

    #[test]
    fn test_predict_shard_load() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        for _ in 0..10 {
            sharder.record_load("node1", 0.4).unwrap();
            sharder.record_load("node2", 0.6).unwrap();
        }
        sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        let predicted = sharder.predict_shard_load("shard1").unwrap();
        assert!(predicted >= 0.0 && predicted <= 1.0);
    }

    #[test]
    fn test_predict_shard_load_not_found() {
        let sharder = PredictiveSharderV4::with_defaults();
        let result = sharder.predict_shard_load("missing");
        assert!(matches!(
            result,
            Err(PredictiveSharderError::ShardNotFound(_))
        ));
    }

    #[test]
    fn test_evaluate_placements() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        for _ in 0..10 {
            sharder.record_load("node1", 0.3).unwrap();
            sharder.record_load("node2", 0.3).unwrap();
        }
        sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        let results = sharder.evaluate_placements();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_placement() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        assert!(sharder.get_placement("shard1").is_some());
        assert!(sharder.get_placement("missing").is_none());
    }

    #[test]
    fn test_remove_shard() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        sharder.remove_shard("shard1").unwrap();
        assert_eq!(sharder.shard_count(), 0);
    }

    #[test]
    fn test_remove_shard_not_found() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        let result = sharder.remove_shard("missing");
        assert!(matches!(
            result,
            Err(PredictiveSharderError::ShardNotFound(_))
        ));
    }

    #[test]
    fn test_warm_node_count() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        assert_eq!(sharder.warm_node_count(), 0);
        for _ in 0..10 {
            sharder.record_load("node1", 0.5).unwrap();
        }
        assert_eq!(sharder.warm_node_count(), 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.register_node("node1".to_string());
        sharder.register_node("node2".to_string());
        sharder
            .create_shard(
                "shard1".to_string(),
                vec!["node1".to_string(), "node2".to_string()],
            )
            .unwrap();
        sharder.reset_stats();
        assert_eq!(sharder.stats().total_placements, 0);
    }

    #[test]
    fn test_load_history_creation() {
        let history = LoadHistory::new(10);
        assert_eq!(history.count, 0);
        assert_eq!(history.ema, 0.0);
    }

    #[test]
    fn test_load_history_record() {
        let mut history = LoadHistory::new(10);
        history.record(0.5, 0.3, 10);
        assert_eq!(history.count, 1);
        assert!((history.ema - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_load_history_ema_smoothing() {
        let mut history = LoadHistory::new(10);
        history.record(1.0, 0.3, 10);
        history.record(0.0, 0.3, 10);
        // EMA should be between 0 and 1
        assert!(history.ema > 0.0 && history.ema < 1.0);
    }

    #[test]
    fn test_load_history_is_warm() {
        let mut history = LoadHistory::new(10);
        assert!(!history.is_warm(5));
        for _ in 0..5 {
            history.record(0.5, 0.3, 10);
        }
        assert!(history.is_warm(5));
    }

    #[test]
    fn test_load_history_predict() {
        let mut history = LoadHistory::new(10);
        for i in 0..10 {
            history.record(0.3 + i as f64 * 0.05, 0.3, 10);
        }
        let predicted = history.predict(3, 0.3);
        assert!(predicted >= 0.0 && predicted <= 1.0);
    }

    #[test]
    fn test_load_history_avg_recent() {
        let mut history = LoadHistory::new(10);
        assert_eq!(history.avg_recent(), 0.0);
        history.record(0.5, 0.3, 10);
        history.record(0.5, 0.3, 10);
        assert!((history.avg_recent() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_shard_placement_new() {
        let placement = ShardPlacement::new("s1".to_string());
        assert_eq!(placement.shard_id, "s1");
        assert_eq!(placement.placement_score, 1.0);
        assert!(!placement.proactive_rebalance);
    }

    #[test]
    fn test_shard_placement_update_prediction() {
        let mut placement = ShardPlacement::new("s1".to_string());
        placement.load_factor = 0.5;
        placement.update_prediction(0.8, 0.7);
        assert!(placement.proactive_rebalance);
        assert!((placement.predicted_load - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_stats_default() {
        let stats = SharderStats::default();
        assert_eq!(stats.total_placements, 0);
        assert_eq!(stats.avg_prediction_accuracy, 1.0);
    }

    #[test]
    fn test_stats_record_accuracy() {
        let mut stats = SharderStats::default();
        stats.record_accuracy(0.9);
        assert_eq!(stats.total_predictions, 1);
        assert!((stats.avg_prediction_accuracy - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_config_default() {
        let config = PredictiveSharderConfig::default();
        assert_eq!(config.ema_alpha, 0.3);
        assert_eq!(config.min_warm_samples, 5);
        assert_eq!(config.max_shards, 32);
    }

    #[test]
    fn test_error_display() {
        match PredictiveSharderError::ShardNotFound("x".to_string()) {
            e => assert!(format!("{}", e).contains("x")),
        }
    }

    #[test]
    fn test_sharder_default() {
        let sharder = PredictiveSharderV4::default();
        assert_eq!(sharder.node_count(), 0);
    }

    #[test]
    fn test_max_shards_limit() {
        let mut sharder = PredictiveSharderV4::with_defaults();
        sharder.config.max_shards = 1;
        sharder.register_node("n1".to_string());
        sharder.register_node("n2".to_string());
        sharder.register_node("n3".to_string());
        sharder.register_node("n4".to_string());
        sharder
            .create_shard("s1".to_string(), vec!["n1".to_string(), "n2".to_string()])
            .unwrap();
        assert!(sharder
            .create_shard("s2".to_string(), vec!["n3".to_string(), "n4".to_string()])
            .is_err());
    }
}
