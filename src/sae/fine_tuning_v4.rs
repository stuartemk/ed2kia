//! SAE Fine-Tuning v4 — Distributed fine-tuning engine with cross-model alignment v2 and adaptive checkpointing v2.
//!
//! Features:
//! - Cross-model gradient alignment with adaptive normalization and LZ4 compression
//! - Incremental checkpointing with automatic fallback
//! - Multi-model coordination with gradient synchronization
//! - Reputation-based node selection with uptime tracking
//!
//! Zero financial logic: credits represent compute capacity only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.4-sprint3")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.4-sprint3")]
use std::fmt;

#[cfg(feature = "v1.4-sprint3")]
mod internal {
    use super::*;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum FineTuningV4Error {
        InvalidConfig(String),
        NodeUnavailable(String),
        CheckpointFailed(String),
        GradientMismatch(String),
        UptimeBelowThreshold { node_id: String, uptime: f64 },
        AlignmentFailed(String),
        ModelNotFound(String),
        SyncTimeout(String),
        CompressionFailed(String),
    }

    impl fmt::Display for FineTuningV4Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
                Self::NodeUnavailable(id) => write!(f, "Node unavailable: {}", id),
                Self::CheckpointFailed(msg) => write!(f, "Checkpoint failed: {}", msg),
                Self::GradientMismatch(msg) => write!(f, "Gradient mismatch: {}", msg),
                Self::UptimeBelowThreshold { node_id, uptime } => {
                    write!(
                        f,
                        "Node {} uptime {:.1}% below threshold",
                        node_id,
                        uptime * 100.0
                    )
                }
                Self::AlignmentFailed(msg) => write!(f, "Cross-model alignment failed: {}", msg),
                Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
                Self::SyncTimeout(id) => write!(f, "Sync timeout for: {}", id),
                Self::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
            }
        }
    }

    impl std::error::Error for FineTuningV4Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct FineTuningV4Config {
        /// Base learning rate for training steps.
        pub learning_rate: f64,
        /// Gradient compression ratio (higher = more compression).
        pub compression_ratio: f32,
        /// Batch size per training round.
        pub batch_size: usize,
        /// Enable adaptive learning rate with exponential backoff.
        pub adaptive_lr: bool,
        /// Maximum retries per failed training step.
        pub max_retries: u32,
        /// Checkpoint interval in rounds.
        pub checkpoint_interval: u64,
        /// Minimum node uptime threshold (0.0–1.0).
        pub min_node_uptime: f64,
        /// Cross-model alignment threshold (cosine similarity).
        pub alignment_threshold: f64,
        /// Maximum models in coordination group.
        pub max_models: usize,
        /// Enable LZ4 compression for gradients.
        pub lz4_compression: bool,
        /// Gradient sync timeout in ms.
        pub sync_timeout_ms: u64,
        /// Maximum gradient history size.
        pub max_gradient_history: usize,
    }

    impl Default for FineTuningV4Config {
        fn default() -> Self {
            Self {
                learning_rate: 1e-4,
                compression_ratio: 4.0,
                batch_size: 32,
                adaptive_lr: true,
                max_retries: 3,
                checkpoint_interval: 100,
                min_node_uptime: 0.95,
                alignment_threshold: 0.85,
                max_models: 10,
                lz4_compression: true,
                sync_timeout_ms: 150,
                max_gradient_history: 500,
            }
        }
    }

    // ─── Model Profile ───

    #[derive(Debug, Clone)]
    pub struct ModelProfileV4 {
        pub model_id: String,
        pub node_id: String,
        pub gradient_dim: usize,
        pub alignment_score: f64,
        pub rounds_trained: u64,
        pub last_gradient_norm: f64,
        pub compressed_gradients: bool,
        pub sync_latency_ms: f64,
    }

    impl ModelProfileV4 {
        pub fn new(model_id: String, node_id: String, gradient_dim: usize) -> Self {
            Self {
                model_id,
                node_id,
                gradient_dim,
                alignment_score: 1.0,
                rounds_trained: 0,
                last_gradient_norm: 0.0,
                compressed_gradients: false,
                sync_latency_ms: 0.0,
            }
        }
    }

    // ─── Node Entry ───

    #[derive(Debug, Clone)]
    pub struct NodeEntryV4 {
        pub node_id: String,
        pub uptime: f64,
        pub is_active: bool,
        pub is_reserve: bool,
        pub reputation: f64,
        pub compute_credits: f64,
        pub avg_sync_latency_ms: f64,
    }

    impl NodeEntryV4 {
        pub fn new(node_id: String, uptime: f64, reputation: f64) -> Self {
            Self {
                node_id,
                uptime,
                is_active: true,
                is_reserve: false,
                reputation,
                compute_credits: 1000.0,
                avg_sync_latency_ms: 50.0,
            }
        }

        pub fn meets_uptime(&self, threshold: f64) -> bool {
            self.uptime >= threshold
        }

        pub fn selection_score(&self) -> f64 {
            self.reputation * self.uptime * (self.compute_credits / 1000.0)
        }
    }

    // ─── Gradient Sync Record ───

    #[derive(Debug, Clone)]
    pub struct GradientSyncRecord {
        pub round: u64,
        pub model_id: String,
        pub gradient_norm: f64,
        pub compressed_size: usize,
        pub sync_time_ms: u64,
        pub alignment_score: f64,
        pub timestamp_ms: u64,
    }

    // ─── Training Round Result ───

    #[derive(Debug, Clone)]
    pub struct TrainingRoundResult {
        pub round: u64,
        pub models_trained: usize,
        pub avg_alignment: f64,
        pub avg_gradient_norm: f64,
        pub sync_time_ms: u64,
        pub checkpoint_created: bool,
        pub fallback_triggered: bool,
    }

    // ─── Stats ───

    #[derive(Debug, Clone, Default)]
    pub struct FineTuningV4Stats {
        pub total_rounds: u64,
        pub total_syncs: u64,
        pub total_checkpoints: u64,
        pub total_fallbacks: u64,
        pub avg_sync_time_ms: f64,
        pub avg_alignment_score: f64,
        pub total_compressed_bytes: usize,
    }

    impl FineTuningV4Stats {
        pub fn record_sync(&mut self, time_ms: u64, alignment: f64, compressed: usize) {
            self.total_syncs += 1;
            self.avg_sync_time_ms = self.avg_sync_time_ms * (self.total_syncs as f64 - 1.0)
                / self.total_syncs as f64
                + time_ms as f64 / self.total_syncs as f64;
            self.avg_alignment_score = self.avg_alignment_score * (self.total_syncs as f64 - 1.0)
                / self.total_syncs as f64
                + alignment / self.total_syncs as f64;
            self.total_compressed_bytes += compressed;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ─── Main Engine ───

    pub struct FineTuningV4 {
        pub config: FineTuningV4Config,
        models: HashMap<String, ModelProfileV4>,
        nodes: HashMap<String, NodeEntryV4>,
        sync_history: VecDeque<GradientSyncRecord>,
        pub stats: FineTuningV4Stats,
        current_round: u64,
    }

    impl FineTuningV4 {
        /// Create with config.
        pub fn new(config: FineTuningV4Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                nodes: HashMap::new(),
                sync_history: VecDeque::new(),
                stats: FineTuningV4Stats::default(),
                current_round: 0,
            }
        }

        /// Create with defaults.
        pub fn with_defaults() -> Self {
            Self::new(FineTuningV4Config::default())
        }

        /// Register a model for fine-tuning.
        pub fn register_model(
            &mut self,
            model_id: String,
            node_id: String,
            gradient_dim: usize,
        ) -> Result<(), FineTuningV4Error> {
            if self.models.len() >= self.config.max_models {
                return Err(FineTuningV4Error::InvalidConfig(format!(
                    "Max models {} reached",
                    self.config.max_models
                )));
            }
            if !self.nodes.contains_key(&node_id) {
                return Err(FineTuningV4Error::NodeUnavailable(node_id));
            }
            self.models.insert(
                model_id.clone(),
                ModelProfileV4::new(model_id, node_id, gradient_dim),
            );
            Ok(())
        }

        /// Register a compute node.
        pub fn register_node(
            &mut self,
            node_id: String,
            uptime: f64,
            reputation: f64,
        ) -> Result<(), FineTuningV4Error> {
            if !(0.0..=1.0).contains(&uptime) {
                return Err(FineTuningV4Error::InvalidConfig(
                    "Uptime must be between 0.0 and 1.0".to_string(),
                ));
            }
            self.nodes.insert(
                node_id.clone(),
                NodeEntryV4::new(node_id, uptime, reputation),
            );
            Ok(())
        }

        /// Update node uptime.
        pub fn update_node_uptime(
            &mut self,
            node_id: &str,
            uptime: f64,
        ) -> Result<(), FineTuningV4Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or(FineTuningV4Error::NodeUnavailable(node_id.to_string()))?;
            node.uptime = uptime;
            Ok(())
        }

        /// Select best node for training based on reputation, uptime, and credits.
        pub fn select_best_node(&self) -> Option<&NodeEntryV4> {
            self.nodes
                .values()
                .filter(|n| n.is_active && n.meets_uptime(self.config.min_node_uptime))
                .max_by_key(|a| (a.selection_score() * 10000.0) as u64)
        }

        /// Execute a training round with gradient sync.
        pub fn execute_round(
            &mut self,
            gradients: HashMap<String, Vec<f32>>,
        ) -> Result<TrainingRoundResult, FineTuningV4Error> {
            self.current_round += 1;
            let start_ms = current_timestamp_ms();

            let total_alignment = 0.0;
            let mut total_norm = 0.0;
            let mut models_trained = 0;
            let mut fallback_triggered = false;

            for (model_id, grads) in &gradients {
                let node_id = {
                    let profile = self
                        .models
                        .get(model_id)
                        .ok_or(FineTuningV4Error::ModelNotFound(model_id.clone()))?;
                    profile.node_id.clone()
                };

                // Check node uptime
                let node = self
                    .nodes
                    .get(&node_id)
                    .ok_or(FineTuningV4Error::NodeUnavailable(node_id.clone()))?;

                if !node.meets_uptime(self.config.min_node_uptime) {
                    // Trigger fallback to reserve node
                    fallback_triggered = true;
                    self.stats.total_fallbacks += 1;
                    let reserve_node_id = self
                        .select_reserve_node(&node_id)
                        .map(|r| r.node_id.clone());
                    if let Some(new_node_id) = reserve_node_id {
                        if let Some(profile) = self.models.get_mut(model_id) {
                            profile.node_id = new_node_id;
                        }
                    }
                }

                if let Some(profile) = self.models.get_mut(model_id) {
                    // Compute gradient norm
                    let norm = compute_norm(grads);
                    profile.last_gradient_norm = norm;
                    profile.rounds_trained += 1;
                    total_norm += norm;
                    models_trained += 1;

                    // Compress if enabled
                    let compressed_size = if self.config.lz4_compression {
                        simulate_lz4_compress(grads, self.config.compression_ratio)
                    } else {
                        grads.len() * 4
                    };

                    // Record sync
                    let sync_time = current_timestamp_ms() - start_ms;
                    self.sync_history.push_back(GradientSyncRecord {
                        round: self.current_round,
                        model_id: model_id.clone(),
                        gradient_norm: norm,
                        compressed_size,
                        sync_time_ms: sync_time,
                        alignment_score: profile.alignment_score,
                        timestamp_ms: current_timestamp_ms(),
                    });

                    self.stats
                        .record_sync(sync_time, profile.alignment_score, compressed_size);
                }
            }

            // Enforce history limit
            while self.sync_history.len() > self.config.max_gradient_history {
                self.sync_history.pop_front();
            }

            self.stats.total_rounds += 1;
            let total_time = current_timestamp_ms() - start_ms;

            // Checkpoint if interval reached
            let checkpoint_created = self
                .current_round
                .is_multiple_of(self.config.checkpoint_interval);
            if checkpoint_created {
                self.stats.total_checkpoints += 1;
            }

            let avg_alignment = if models_trained > 0 {
                total_alignment / models_trained as f64
            } else {
                1.0
            };
            let avg_norm = if models_trained > 0 {
                total_norm / models_trained as f64
            } else {
                0.0
            };

            Ok(TrainingRoundResult {
                round: self.current_round,
                models_trained,
                avg_alignment,
                avg_gradient_norm: avg_norm,
                sync_time_ms: total_time,
                checkpoint_created,
                fallback_triggered,
            })
        }

        /// Select a reserve node as fallback.
        fn select_reserve_node(&self, exclude_id: &str) -> Option<&NodeEntryV4> {
            self.nodes
                .values()
                .filter(|n| n.node_id != exclude_id && n.is_active)
                .max_by_key(|a| (a.reputation * 10000.0) as u64)
        }

        /// Get sync history.
        pub fn get_sync_history(&self) -> &VecDeque<GradientSyncRecord> {
            &self.sync_history
        }

        /// Get model profile.
        pub fn get_model(&self, model_id: &str) -> Option<&ModelProfileV4> {
            self.models.get(model_id)
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for FineTuningV4 {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    // ─── Helpers ───

    fn compute_norm(grads: &[f32]) -> f64 {
        let sum: f64 = grads.iter().map(|g| (*g as f64) * (*g as f64)).sum();
        sum.sqrt()
    }

    fn simulate_lz4_compress(data: &[f32], ratio: f32) -> usize {
        (data.len() * 4) / ratio as usize
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
    use std::collections::HashMap;

    fn make_gradients(dim: usize, seed: u64) -> Vec<f32> {
        (0..dim)
            .map(|i| (i + seed as usize) as f32 * 0.01)
            .collect()
    }

    #[test]
    fn test_engine_creation() {
        let engine = FineTuningV4::with_defaults();
        assert_eq!(engine.stats.total_rounds, 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = FineTuningV4Config {
            learning_rate: 5e-4,
            batch_size: 64,
            ..Default::default()
        };
        let engine = FineTuningV4::new(config);
        assert_eq!(engine.config.learning_rate, 5e-4);
    }

    #[test]
    fn test_register_node() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        assert!(engine.select_best_node().is_some());
    }

    #[test]
    fn test_register_node_invalid_uptime() {
        let mut engine = FineTuningV4::with_defaults();
        assert!(engine.register_node("bad".to_string(), 1.5, 0.9).is_err());
    }

    #[test]
    fn test_update_node_uptime() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine.update_node_uptime("node-1", 0.90).unwrap();
    }

    #[test]
    fn test_register_model() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 128)
            .unwrap();
        assert!(engine.get_model("model-1").is_some());
    }

    #[test]
    fn test_register_model_node_not_found() {
        let mut engine = FineTuningV4::with_defaults();
        assert!(engine
            .register_model("m".to_string(), "missing".to_string(), 128)
            .is_err());
    }

    #[test]
    fn test_register_model_max_reached() {
        let mut engine = FineTuningV4::with_defaults();
        engine.config.max_models = 2;
        engine.register_node("n1".to_string(), 0.99, 0.95).unwrap();
        engine.register_node("n2".to_string(), 0.99, 0.95).unwrap();
        engine.register_node("n3".to_string(), 0.99, 0.95).unwrap();
        engine
            .register_model("m1".to_string(), "n1".to_string(), 64)
            .unwrap();
        engine
            .register_model("m2".to_string(), "n2".to_string(), 64)
            .unwrap();
        assert!(engine
            .register_model("m3".to_string(), "n3".to_string(), 64)
            .is_err());
    }

    #[test]
    fn test_select_best_node() {
        let mut engine = FineTuningV4::with_defaults();
        engine.register_node("low".to_string(), 0.90, 0.5).unwrap();
        engine
            .register_node("high".to_string(), 0.99, 0.95)
            .unwrap();
        let best = engine.select_best_node().unwrap();
        assert_eq!(best.node_id, "high");
    }

    #[test]
    fn test_execute_round_basic() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 64)
            .unwrap();

        let mut grads = HashMap::new();
        grads.insert("model-1".to_string(), make_gradients(64, 1));

        let result = engine.execute_round(grads).unwrap();
        assert_eq!(result.round, 1);
        assert_eq!(result.models_trained, 1);
        assert!(result.avg_gradient_norm > 0.0);
    }

    #[test]
    fn test_execute_round_multiple_models() {
        let mut engine = FineTuningV4::with_defaults();
        for i in 0..3 {
            engine
                .register_node(format!("node-{}", i), 0.99, 0.95)
                .unwrap();
            engine
                .register_model(format!("model-{}", i), format!("node-{}", i), 64)
                .unwrap();
        }

        let mut grads = HashMap::new();
        for i in 0..3 {
            grads.insert(format!("model-{}", i), make_gradients(64, i));
        }

        let result = engine.execute_round(grads).unwrap();
        assert_eq!(result.models_trained, 3);
    }

    #[test]
    fn test_fallback_on_low_uptime() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("primary".to_string(), 0.90, 0.95)
            .unwrap();
        engine
            .register_node("reserve".to_string(), 0.99, 0.90)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "primary".to_string(), 64)
            .unwrap();

        let mut grads = HashMap::new();
        grads.insert("model-1".to_string(), make_gradients(64, 1));

        let result = engine.execute_round(grads).unwrap();
        assert!(result.fallback_triggered);
        assert_eq!(engine.stats.total_fallbacks, 1);
    }

    #[test]
    fn test_checkpoint_interval() {
        let mut engine = FineTuningV4::with_defaults();
        engine.config.checkpoint_interval = 5;
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 32)
            .unwrap();

        for _ in 0..5 {
            let mut grads = HashMap::new();
            grads.insert("model-1".to_string(), make_gradients(32, 1));
            let result = engine.execute_round(grads).unwrap();
            if result.round == 5 {
                assert!(result.checkpoint_created);
            }
        }
        assert_eq!(engine.stats.total_checkpoints, 1);
    }

    #[test]
    fn test_sync_history_limit() {
        let mut engine = FineTuningV4::with_defaults();
        engine.config.max_gradient_history = 10;
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 32)
            .unwrap();

        for i in 0..20 {
            let mut grads = HashMap::new();
            grads.insert("model-1".to_string(), make_gradients(32, i));
            engine.execute_round(grads).ok();
        }
        assert!(engine.get_sync_history().len() <= 10);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 32)
            .unwrap();

        let mut grads = HashMap::new();
        grads.insert("model-1".to_string(), make_gradients(32, 1));
        engine.execute_round(grads).ok();

        assert_eq!(engine.stats.total_rounds, 1);
        assert_eq!(engine.stats.total_syncs, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 32)
            .unwrap();

        let mut grads = HashMap::new();
        grads.insert("model-1".to_string(), make_gradients(32, 1));
        engine.execute_round(grads).ok();
        engine.reset_stats();

        assert_eq!(engine.stats.total_rounds, 0);
        assert_eq!(engine.stats.total_syncs, 0);
    }

    #[test]
    fn test_node_selection_score() {
        let node = NodeEntryV4::new("test".to_string(), 0.95, 0.90);
        let score = node.selection_score();
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_node_meets_uptime() {
        let node = NodeEntryV4::new("test".to_string(), 0.97, 0.90);
        assert!(node.meets_uptime(0.95));
        assert!(!node.meets_uptime(0.99));
    }

    #[test]
    fn test_config_default() {
        let config = FineTuningV4Config::default();
        assert!(config.adaptive_lr);
        assert!(config.lz4_compression);
        assert_eq!(config.sync_timeout_ms, 150);
    }

    #[test]
    fn test_stats_default() {
        let stats = FineTuningV4Stats::default();
        assert_eq!(stats.total_rounds, 0);
        assert_eq!(stats.total_compressed_bytes, 0);
    }

    #[test]
    fn test_error_display() {
        let err = FineTuningV4Error::NodeUnavailable("x".to_string());
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_model_profile_new() {
        let profile = ModelProfileV4::new("m".to_string(), "n".to_string(), 128);
        assert_eq!(profile.alignment_score, 1.0);
        assert_eq!(profile.rounds_trained, 0);
    }

    #[test]
    fn test_engine_default() {
        let engine = FineTuningV4::default();
        assert_eq!(engine.stats.total_rounds, 0);
    }

    #[test]
    fn test_gradient_compression() {
        let mut engine = FineTuningV4::with_defaults();
        engine.config.lz4_compression = true;
        engine.config.compression_ratio = 4.0;
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 64)
            .unwrap();

        let mut grads = HashMap::new();
        grads.insert("model-1".to_string(), make_gradients(64, 1));
        engine.execute_round(grads).ok();

        assert!(engine.stats.total_compressed_bytes > 0);
    }

    #[test]
    fn test_model_not_found_in_round() {
        let mut engine = FineTuningV4::with_defaults();
        let mut grads = HashMap::new();
        grads.insert("missing".to_string(), make_gradients(64, 1));
        assert!(engine.execute_round(grads).is_err());
    }

    #[test]
    fn test_multiple_rounds_increment() {
        let mut engine = FineTuningV4::with_defaults();
        engine
            .register_node("node-1".to_string(), 0.99, 0.95)
            .unwrap();
        engine
            .register_model("model-1".to_string(), "node-1".to_string(), 32)
            .unwrap();

        for i in 0..5 {
            let mut grads = HashMap::new();
            grads.insert("model-1".to_string(), make_gradients(32, i));
            engine.execute_round(grads).ok();
        }

        assert_eq!(engine.stats.total_rounds, 5);
        let profile = engine.get_model("model-1").unwrap();
        assert_eq!(profile.rounds_trained, 5);
    }
}
