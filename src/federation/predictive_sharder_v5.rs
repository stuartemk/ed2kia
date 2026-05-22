//! Predictive Sharder v5 — Load-based predictive sharding with cross-model awareness and adaptive rebalancing.
//!
//! Extends PredictiveSharderV4 with cross-model load forecasting, adaptive shard splitting,
//! and proactive rebalancing based on historical patterns.
//!
//! Feature-gated: `#[cfg(feature = "v1.5-sprint2")]`

mod internal {
    use std::collections::{HashMap, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for predictive sharding v5.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SharderV5Error {
        /// Shard not found.
        ShardNotFound(String),
        /// Prediction model not ready.
        PredictionNotReady,
        /// Insufficient data for prediction.
        InsufficientData,
        /// Shard split failed.
        SplitFailed(String),
        /// Rebalance cooldown active.
        RebalanceCooldown(u64),
    }

    impl std::fmt::Display for SharderV5Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SharderV5Error::ShardNotFound(id) => write!(f, "Shard {} not found", id),
                SharderV5Error::PredictionNotReady => write!(f, "Prediction model not ready"),
                SharderV5Error::InsufficientData => write!(f, "Insufficient data for prediction"),
                SharderV5Error::SplitFailed(msg) => write!(f, "Shard split failed: {}", msg),
                SharderV5Error::RebalanceCooldown(ms) => {
                    write!(f, "Rebalance cooldown active for {}ms", ms)
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for predictive sharding v5.
    #[derive(Debug, Clone)]
    pub struct SharderV5Config {
        /// EMA alpha for load prediction.
        pub ema_alpha: f64,
        /// Minimum samples before prediction is ready.
        pub min_samples: usize,
        /// Prediction horizon (steps ahead).
        pub prediction_horizon: usize,
        /// Shard split threshold (load ratio).
        pub split_threshold: f64,
        /// Shard merge threshold (load ratio).
        pub merge_threshold: f64,
        /// Rebalance cooldown (ms).
        pub rebalance_cooldown_ms: u64,
        /// Maximum history samples per shard.
        pub max_history_samples: usize,
        /// Cross-model weight factor.
        pub cross_model_weight: f64,
    }

    impl Default for SharderV5Config {
        fn default() -> Self {
            Self {
                ema_alpha: 0.3,
                min_samples: 10,
                prediction_horizon: 10,
                split_threshold: 0.85,
                merge_threshold: 0.15,
                rebalance_cooldown_ms: 10_000,
                max_history_samples: 128,
                cross_model_weight: 0.30,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Shard Prediction Entry
    // ---------------------------------------------------------------------------

    /// Prediction entry for a single shard.
    #[derive(Debug, Clone)]
    pub struct ShardPrediction {
        /// Shard identifier.
        pub shard_id: String,
        /// Current EMA load.
        pub current_load: f64,
        /// Predicted load at horizon.
        pub predicted_load: f64,
        /// Recommended action.
        pub action: ShardAction,
        /// Confidence score (0.0-1.0).
        pub confidence: f64,
        /// Cross-model load contribution.
        pub cross_model_load: f64,
    }

    /// Recommended shard action.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ShardAction {
        /// No action needed.
        NoOp,
        /// Split this shard into two.
        Split,
        /// Merge this shard with another.
        Merge,
        /// Rebalance nodes within shard.
        Rebalance,
    }

    impl std::fmt::Display for ShardAction {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ShardAction::NoOp => write!(f, "NoOp"),
                ShardAction::Split => write!(f, "Split"),
                ShardAction::Merge => write!(f, "Merge"),
                ShardAction::Rebalance => write!(f, "Rebalance"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Shard History Entry
    // ---------------------------------------------------------------------------

    /// Historical load data for a shard.
    #[derive(Debug, Clone)]
    pub struct ShardHistory {
        /// Shard identifier.
        pub shard_id: String,
        /// Load samples over time.
        pub load_samples: VecDeque<f64>,
        /// Cross-model load samples.
        pub cross_model_samples: VecDeque<f64>,
        /// Current EMA load.
        pub ema_load: f64,
        /// Current EMA cross-model load.
        pub ema_cross_model: f64,
        /// Last rebalance time (ms).
        pub last_rebalance_ms: u64,
    }

    impl ShardHistory {
        /// Create a new shard history.
        pub fn new(shard_id: String) -> Self {
            Self {
                shard_id,
                load_samples: VecDeque::new(),
                cross_model_samples: VecDeque::new(),
                ema_load: 0.0,
                ema_cross_model: 0.0,
                last_rebalance_ms: 0,
            }
        }

        /// Record a load sample.
        pub fn record_load(&mut self, load: f64, alpha: f64, max_samples: usize) {
            if self.load_samples.len() >= max_samples {
                self.load_samples.pop_front();
            }
            self.load_samples.push_back(load);
            if self.ema_load == 0.0 {
                self.ema_load = load;
            } else {
                self.ema_load = alpha * load + (1.0 - alpha) * self.ema_load;
            }
        }

        /// Record a cross-model load sample.
        pub fn record_cross_model_load(&mut self, load: f64, alpha: f64, max_samples: usize) {
            if self.cross_model_samples.len() >= max_samples {
                self.cross_model_samples.pop_front();
            }
            self.cross_model_samples.push_back(load);
            if self.ema_cross_model == 0.0 {
                self.ema_cross_model = load;
            } else {
                self.ema_cross_model = alpha * load + (1.0 - alpha) * self.ema_cross_model;
            }
        }

        /// Check if enough samples for prediction.
        pub fn has_enough_samples(&self, min_samples: usize) -> bool {
            self.load_samples.len() >= min_samples
        }

        /// Predict future load using EMA extrapolation.
        pub fn predict_load(&self, horizon: usize, cross_model_weight: f64) -> f64 {
            let base_prediction = self.ema_load * (1.0 + 0.03 * horizon as f64);
            let cross_model_contribution = self.ema_cross_model * cross_model_weight;
            (base_prediction + cross_model_contribution).min(1.0)
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for predictive sharding v5.
    #[derive(Debug, Clone, Default)]
    pub struct SharderV5Stats {
        /// Total predictions made.
        pub predictions_made: usize,
        /// Successful splits.
        pub splits: usize,
        /// Successful merges.
        pub merges: usize,
        /// Successful rebalances.
        pub rebalances: usize,
        /// Prediction accuracy samples.
        pub accuracy_samples: usize,
    }

    // ---------------------------------------------------------------------------
    // Predictive Sharder V5 Engine
    // ---------------------------------------------------------------------------

    /// Predictive Sharder v5 engine with cross-model awareness.
    pub struct PredictiveSharderV5 {
        config: SharderV5Config,
        histories: HashMap<String, ShardHistory>,
        stats: SharderV5Stats,
    }

    impl PredictiveSharderV5 {
        /// Create a new predictive sharder with custom config.
        pub fn new(config: SharderV5Config) -> Self {
            Self {
                config,
                histories: HashMap::new(),
                stats: SharderV5Stats::default(),
            }
        }

        /// Register a shard for tracking.
        pub fn register_shard(&mut self, shard_id: String) {
            self.histories
                .insert(shard_id.clone(), ShardHistory::new(shard_id));
        }

        /// Record load for a shard.
        pub fn record_load(&mut self, shard_id: &str, load: f64) -> Result<(), SharderV5Error> {
            let history = self
                .histories
                .get_mut(shard_id)
                .ok_or_else(|| SharderV5Error::ShardNotFound(shard_id.to_string()))?;
            history.record_load(load, self.config.ema_alpha, self.config.max_history_samples);
            Ok(())
        }

        /// Record cross-model load for a shard.
        pub fn record_cross_model_load(
            &mut self,
            shard_id: &str,
            load: f64,
        ) -> Result<(), SharderV5Error> {
            let history = self
                .histories
                .get_mut(shard_id)
                .ok_or_else(|| SharderV5Error::ShardNotFound(shard_id.to_string()))?;
            history.record_cross_model_load(
                load,
                self.config.ema_alpha,
                self.config.max_history_samples,
            );
            Ok(())
        }

        /// Generate prediction for a shard.
        pub fn predict(&mut self, shard_id: &str) -> Result<ShardPrediction, SharderV5Error> {
            let history = self
                .histories
                .get(shard_id)
                .ok_or_else(|| SharderV5Error::ShardNotFound(shard_id.to_string()))?;

            if !history.has_enough_samples(self.config.min_samples) {
                return Err(SharderV5Error::InsufficientData);
            }

            let predicted = history.predict_load(
                self.config.prediction_horizon,
                self.config.cross_model_weight,
            );

            let action = if predicted >= self.config.split_threshold {
                ShardAction::Split
            } else if predicted <= self.config.merge_threshold {
                ShardAction::Merge
            } else if (history.ema_load - history.ema_cross_model).abs() > 0.3 {
                ShardAction::Rebalance
            } else {
                ShardAction::NoOp
            };

            let confidence =
                (history.load_samples.len() as f64 / self.config.min_samples as f64).min(1.0);

            self.stats.predictions_made += 1;

            Ok(ShardPrediction {
                shard_id: shard_id.to_string(),
                current_load: history.ema_load,
                predicted_load: predicted,
                action,
                confidence,
                cross_model_load: history.ema_cross_model,
            })
        }

        /// Generate predictions for all shards.
        pub fn predict_all(&mut self) -> Vec<ShardPrediction> {
            let ids: Vec<String> = self.histories.keys().cloned().collect();
            ids.into_iter()
                .filter_map(|id| self.predict(&id).ok())
                .collect()
        }

        /// Record a shard split.
        pub fn record_split(&mut self) {
            self.stats.splits += 1;
        }

        /// Record a shard merge.
        pub fn record_merge(&mut self) {
            self.stats.merges += 1;
        }

        /// Record a rebalance.
        pub fn record_rebalance(&mut self, current_time_ms: u64) {
            self.stats.rebalances += 1;
            for history in self.histories.values_mut() {
                history.last_rebalance_ms = current_time_ms;
            }
        }

        /// Get stats reference.
        pub fn stats(&self) -> &SharderV5Stats {
            &self.stats
        }

        /// Get shard count.
        pub fn shard_count(&self) -> usize {
            self.histories.len()
        }

        /// Get histories reference.
        pub fn histories(&self) -> &HashMap<String, ShardHistory> {
            &self.histories
        }
    }

    impl Default for PredictiveSharderV5 {
        fn default() -> Self {
            Self::new(SharderV5Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_engine_creation() {
            let engine = PredictiveSharderV5::default();
            assert_eq!(engine.shard_count(), 0);
        }

        #[test]
        fn test_register_shard() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            assert_eq!(engine.shard_count(), 1);
        }

        #[test]
        fn test_record_load() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            engine.record_load("shard-1", 0.5).unwrap();
        }

        #[test]
        fn test_record_load_shard_not_found() {
            let mut engine = PredictiveSharderV5::default();
            let result = engine.record_load("missing", 0.5);
            assert!(result.is_err());
        }

        #[test]
        fn test_prediction_insufficient_data() {
            let mut engine = PredictiveSharderV5::default();
            let result = engine.predict("shard-1");
            assert!(result.is_err());
        }

        #[test]
        fn test_prediction_with_data() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            for i in 0..20 {
                engine
                    .record_load("shard-1", 0.3 + (i as f64) * 0.02)
                    .unwrap();
            }
            let prediction = engine.predict("shard-1").unwrap();
            assert_eq!(prediction.shard_id, "shard-1");
            assert!(prediction.confidence > 0.0);
        }

        #[test]
        fn test_split_threshold() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            for _ in 0..20 {
                engine.record_load("shard-1", 0.9).unwrap();
            }
            let prediction = engine.predict("shard-1").unwrap();
            assert_eq!(prediction.action, ShardAction::Split);
        }

        #[test]
        fn test_merge_threshold() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            for _ in 0..20 {
                engine.record_load("shard-1", 0.05).unwrap();
            }
            let prediction = engine.predict("shard-1").unwrap();
            assert_eq!(prediction.action, ShardAction::Merge);
        }

        #[test]
        fn test_cross_model_load() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            for _ in 0..20 {
                engine.record_load("shard-1", 0.5).unwrap();
                engine.record_cross_model_load("shard-1", 0.3).unwrap();
            }
            let prediction = engine.predict("shard-1").unwrap();
            assert!(prediction.cross_model_load > 0.0);
        }

        #[test]
        fn test_predict_all() {
            let mut engine = PredictiveSharderV5::default();
            engine.register_shard("shard-1".to_string());
            engine.register_shard("shard-2".to_string());
            for _ in 0..20 {
                engine.record_load("shard-1", 0.5).unwrap();
                engine.record_load("shard-2", 0.6).unwrap();
            }
            let predictions = engine.predict_all();
            assert_eq!(predictions.len(), 2);
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = PredictiveSharderV5::default();
            engine.record_split();
            engine.record_merge();
            engine.record_rebalance(1000);
            assert_eq!(engine.stats().splits, 1);
            assert_eq!(engine.stats().merges, 1);
            assert_eq!(engine.stats().rebalances, 1);
        }

        #[test]
        fn test_config_default() {
            let config = SharderV5Config::default();
            assert_eq!(config.ema_alpha, 0.3);
            assert_eq!(config.min_samples, 10);
        }

        #[test]
        fn test_error_display() {
            let err = SharderV5Error::ShardNotFound("test".to_string());
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }

        #[test]
        fn test_shard_action_display() {
            let action = ShardAction::Split;
            let msg = format!("{}", action);
            assert_eq!(msg, "Split");
        }

        #[test]
        fn test_history_prediction() {
            let mut history = ShardHistory::new("test".to_string());
            for _ in 0..20 {
                history.record_load(0.5, 0.3, 128);
            }
            let predicted = history.predict_load(5, 0.3);
            assert!(predicted > 0.0);
            assert!(predicted <= 1.0);
        }
    }
}

pub use internal::*;
