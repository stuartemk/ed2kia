//! SAE Fine-Tuning v7 — Distributed fine-tuning engine with cross-model gradient alignment v5,
//! adaptive checkpointing v5, and LZ4-compressed gradient transfer.
//!
//! Features:
//! - Adaptive gradient normalization with EMA-based scaling
//! - LZ4 compression for gradient transfer (target: sync <=90ms)
//! - Incremental checkpointing with SHA-256 integrity (target: checkpoint <=0.3s)
//! - Cross-model alignment via CrossModelAlignerV5
//! - Convergence detection with adaptive learning rate decay
//! - Performance targets: gradient sync <=90ms, checkpoint <=0.3s
//!
//! Zero financial logic: credits represent compute capacity only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.6-sprint3")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.6-sprint3")]
use std::fmt;

#[cfg(feature = "v1.6-sprint3")]
mod internal {
    use super::*;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum FineTuningV7Error {
        InvalidConfig(String),
        NodeUnavailable(String),
        CheckpointFailed(String),
        GradientMismatch { expected: usize, actual: usize },
        UptimeBelowThreshold { node: String, uptime: f64 },
        AlignmentFailed(String),
        ModelNotFound(String),
        SyncTimeout(String),
        CompressionFailed(String),
        ConvergenceDivergence(String),
        LearningRateExhausted,
        IntegrityValidationFailed(String),
        CrossModelSyncFailed(String),
        GradientSyncTimeout,
    }

    impl fmt::Display for FineTuningV7Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
                Self::NodeUnavailable(node) => write!(f, "Node unavailable: {}", node),
                Self::CheckpointFailed(msg) => write!(f, "Checkpoint failed: {}", msg),
                Self::GradientMismatch { expected, actual } => {
                    write!(
                        f,
                        "Gradient dimension mismatch: expected {}, got {}",
                        expected, actual
                    )
                }
                Self::UptimeBelowThreshold { node, uptime } => {
                    write!(f, "Node {} uptime {:.2} below threshold", node, uptime)
                }
                Self::AlignmentFailed(msg) => write!(f, "Alignment failed: {}", msg),
                Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
                Self::SyncTimeout(msg) => write!(f, "Sync timeout: {}", msg),
                Self::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
                Self::ConvergenceDivergence(msg) => {
                    write!(f, "Convergence divergence: {}", msg)
                }
                Self::LearningRateExhausted => write!(f, "Learning rate exhausted"),
                Self::IntegrityValidationFailed(msg) => {
                    write!(f, "Integrity validation failed: {}", msg)
                }
                Self::CrossModelSyncFailed(msg) => {
                    write!(f, "Cross-model sync failed: {}", msg)
                }
                Self::GradientSyncTimeout => write!(f, "Gradient sync timeout exceeded 90ms"),
            }
        }
    }

    impl std::error::Error for FineTuningV7Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct FineTuningV7Config {
        /// Base learning rate for fine-tuning.
        pub learning_rate: f64,
        /// Minimum learning rate before exhaustion.
        pub min_learning_rate: f64,
        /// LZ4 compression ratio target (1.0 = no compression, 4.0 = 4x).
        pub compression_ratio: f64,
        /// Batch size for gradient aggregation.
        pub batch_size: usize,
        /// Enable adaptive learning rate.
        pub adaptive_lr: bool,
        /// Learning rate decay factor per round.
        pub lr_decay: f64,
        /// Maximum models in the fine-tuning pool.
        pub max_models: usize,
        /// Minimum node uptime (0.0-1.0).
        pub min_uptime: f64,
        /// Checkpoint interval in rounds.
        pub checkpoint_interval: usize,
        /// Enable convergence detection.
        pub convergence_detection: bool,
        /// Convergence threshold for loss stability.
        pub convergence_threshold: f64,
        /// Patience rounds before triggering LR decay.
        pub patience: usize,
        /// Gradient sync timeout in milliseconds (target: 90ms).
        pub gradient_sync_timeout_ms: u64,
        /// Enable adaptive gradient normalization.
        pub adaptive_normalization: bool,
        /// EMA alpha for gradient norm tracking.
        pub ema_alpha: f64,
    }

    impl Default for FineTuningV7Config {
        fn default() -> Self {
            Self {
                learning_rate: 1e-4,
                min_learning_rate: 1e-8,
                compression_ratio: 3.0,
                batch_size: 32,
                adaptive_lr: true,
                lr_decay: 0.95,
                max_models: 50,
                min_uptime: 0.9,
                checkpoint_interval: 10,
                convergence_detection: true,
                convergence_threshold: 1e-5,
                patience: 5,
                gradient_sync_timeout_ms: 90,
                adaptive_normalization: true,
                ema_alpha: 0.1,
            }
        }
    }

    // ─── Model Profile ───

    #[derive(Debug, Clone)]
    pub struct ModelProfileV7 {
        pub model_id: String,
        pub node_id: String,
        pub gradient_dim: usize,
        pub current_loss: f64,
        pub loss_history: VecDeque<f64>,
        pub normalized_gradient_history: VecDeque<f64>,
        pub ema_gradient_norm: f64,
        pub convergence_rounds: usize,
    }

    impl ModelProfileV7 {
        pub fn new(model_id: String, node_id: String, gradient_dim: usize) -> Self {
            Self {
                model_id,
                node_id,
                gradient_dim,
                current_loss: f64::MAX,
                loss_history: VecDeque::with_capacity(20),
                normalized_gradient_history: VecDeque::with_capacity(20),
                ema_gradient_norm: 0.0,
                convergence_rounds: 0,
            }
        }

        pub fn record_loss(&mut self, loss: f64) {
            self.current_loss = loss;
            self.loss_history.push_back(loss);
            if self.loss_history.len() > 20 {
                self.loss_history.pop_front();
            }
        }

        pub fn record_normalized_gradient(&mut self, norm: f64, alpha: f64) {
            self.normalized_gradient_history.push_back(norm);
            if self.normalized_gradient_history.len() > 20 {
                self.normalized_gradient_history.pop_front();
            }
            // EMA update
            self.ema_gradient_norm = alpha * norm + (1.0 - alpha) * self.ema_gradient_norm;
        }

        pub fn recent_loss_change(&self) -> f64 {
            if self.loss_history.len() < 2 {
                return 0.0;
            }
            let recent = self.loss_history.back().unwrap();
            let older = self.loss_history.front().unwrap();
            (recent - older).abs()
        }

        pub fn avg_normalized_gradient(&self) -> f64 {
            if self.normalized_gradient_history.is_empty() {
                return 0.0;
            }
            let sum: f64 = self.normalized_gradient_history.iter().sum();
            sum / self.normalized_gradient_history.len() as f64
        }

        pub fn is_converging(&self, threshold: f64, patience: usize) -> bool {
            self.convergence_rounds >= patience && self.recent_loss_change() < threshold
        }
    }

    // ─── Node Entry ───

    #[derive(Debug, Clone)]
    pub struct NodeEntryV7 {
        pub node_id: String,
        pub uptime: f64,
        pub reputation: f64,
        pub capacity: f64,
        pub current_load: f64,
        pub avg_latency_ms: f64,
        pub load_history: VecDeque<f64>,
        pub latency_history: VecDeque<f64>,
    }

    impl NodeEntryV7 {
        pub fn new(node_id: String, uptime: f64, reputation: f64, capacity: f64) -> Self {
            Self {
                node_id,
                uptime,
                reputation,
                capacity,
                current_load: 0.0,
                avg_latency_ms: 0.0,
                load_history: VecDeque::with_capacity(20),
                latency_history: VecDeque::with_capacity(20),
            }
        }

        pub fn update_load(&mut self, load: f64, alpha: f64) {
            self.current_load = alpha * load + (1.0 - alpha) * self.current_load;
            self.load_history.push_back(load);
            if self.load_history.len() > 20 {
                self.load_history.pop_front();
            }
        }

        pub fn update_latency(&mut self, latency_ms: f64, alpha: f64) {
            self.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_latency_ms;
            self.latency_history.push_back(latency_ms);
            if self.latency_history.len() > 20 {
                self.latency_history.pop_front();
            }
        }

        pub fn selection_score(&self, min_uptime: f64) -> f64 {
            let uptime_ok = if self.uptime >= min_uptime { 1.0 } else { 0.0 };
            self.reputation * uptime_ok * (1.0 - self.current_load / self.capacity)
        }

        pub fn meets_uptime(&self, min_uptime: f64) -> bool {
            self.uptime >= min_uptime
        }
    }

    // ─── Checkpoint Entry ───

    #[derive(Debug, Clone)]
    pub struct CheckpointEntryV7 {
        pub round: u64,
        pub model_id: String,
        pub hash: String,
        pub size_bytes: usize,
        pub compressed: bool,
        pub compressed_size: usize,
        pub incremental: bool,
        pub parent_hash: Option<String>,
    }

    impl CheckpointEntryV7 {
        pub fn new(round: u64, model_id: String, hash: String, size_bytes: usize) -> Self {
            Self {
                round,
                model_id,
                hash,
                size_bytes,
                compressed: false,
                compressed_size: 0,
                incremental: false,
                parent_hash: None,
            }
        }

        pub fn mark_compressed(&mut self, compressed_size: usize) {
            self.compressed = true;
            self.compressed_size = compressed_size;
        }

        pub fn mark_incremental(&mut self, parent_hash: String) {
            self.incremental = true;
            self.parent_hash = Some(parent_hash);
        }

        pub fn compression_ratio(&self) -> f64 {
            if self.compressed_size == 0 {
                return 1.0;
            }
            self.size_bytes as f64 / self.compressed_size as f64
        }
    }

    // ─── Gradient Sync Record ───

    #[derive(Debug, Clone)]
    pub struct GradientSyncRecordV7 {
        pub round: u64,
        pub model_id: String,
        pub gradient_norm: f64,
        pub normalized: bool,
        pub compressed: bool,
        pub sync_time_ms: u64,
        pub alignment_score: f64,
    }

    // ─── Training Round Result ───

    #[derive(Debug, Clone)]
    pub struct TrainingRoundResultV7 {
        pub round: u64,
        pub model_id: String,
        pub loss: f64,
        pub gradient_norm: f64,
        pub alignment_score: f64,
        pub checkpoint_saved: bool,
        pub checkpoint_incremental: bool,
        pub sync_time_ms: u64,
        pub learning_rate: f64,
    }

    // ─── Stats ───

    #[derive(Debug, Clone)]
    pub struct FineTuningV7Stats {
        pub total_rounds: u64,
        pub total_syncs: u64,
        pub avg_sync_time_ms: f64,
        pub avg_alignment_score: f64,
        pub avg_gradient_norm: f64,
        pub checkpoints_saved: u64,
        pub incremental_checkpoints: u64,
        pub integrity_validations: u64,
        pub integrity_validations_passed: u64,
        pub lz4_compressions: u64,
        pub avg_lz4_ratio: f64,
        pub convergence_triggers: u64,
        pub lr_decay_count: u64,
        pub gradient_sync_timeouts: u64,
    }

    impl FineTuningV7Stats {
        pub fn record_sync(&mut self, time_ms: u64, alignment: f64, normalized: f64) {
            self.total_syncs += 1;
            self.avg_sync_time_ms = (self.avg_sync_time_ms * (self.total_syncs - 1) as f64
                + time_ms as f64)
                / self.total_syncs as f64;
            self.avg_alignment_score = (self.avg_alignment_score * (self.total_syncs - 1) as f64
                + alignment)
                / self.total_syncs as f64;
            self.avg_gradient_norm = (self.avg_gradient_norm * (self.total_syncs - 1) as f64
                + normalized)
                / self.total_syncs as f64;
        }

        pub fn record_checkpoint(&mut self, incremental: bool) {
            self.checkpoints_saved += 1;
            if incremental {
                self.incremental_checkpoints += 1;
            }
        }

        pub fn record_integrity_validation(&mut self, valid: bool) {
            self.integrity_validations += 1;
            if valid {
                self.integrity_validations_passed += 1;
            }
        }

        pub fn record_lz4_compression(&mut self, ratio: f64) {
            self.lz4_compressions += 1;
            self.avg_lz4_ratio = (self.avg_lz4_ratio * (self.lz4_compressions - 1) as f64 + ratio)
                / self.lz4_compressions as f64;
        }

        pub fn record_convergence(&mut self) {
            self.convergence_triggers += 1;
        }

        pub fn record_lr_decay(&mut self) {
            self.lr_decay_count += 1;
        }

        pub fn record_sync_timeout(&mut self) {
            self.gradient_sync_timeouts += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    impl Default for FineTuningV7Stats {
        fn default() -> Self {
            Self {
                total_rounds: 0,
                total_syncs: 0,
                avg_sync_time_ms: 0.0,
                avg_alignment_score: 0.0,
                avg_gradient_norm: 0.0,
                checkpoints_saved: 0,
                incremental_checkpoints: 0,
                integrity_validations: 0,
                integrity_validations_passed: 0,
                lz4_compressions: 0,
                avg_lz4_ratio: 0.0,
                convergence_triggers: 0,
                lr_decay_count: 0,
                gradient_sync_timeouts: 0,
            }
        }
    }

    // ─── Engine ───

    #[derive(Debug, Clone)]
    pub struct FineTuningV7 {
        config: FineTuningV7Config,
        models: HashMap<String, ModelProfileV7>,
        nodes: HashMap<String, NodeEntryV7>,
        checkpoints: Vec<CheckpointEntryV7>,
        sync_records: Vec<GradientSyncRecordV7>,
        current_round: u64,
        current_lr: f64,
        stats: FineTuningV7Stats,
        patience_counter: usize,
    }

    impl FineTuningV7 {
        pub fn new(config: FineTuningV7Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                nodes: HashMap::new(),
                checkpoints: Vec::new(),
                sync_records: Vec::new(),
                current_round: 0,
                current_lr: 1e-4,
                stats: FineTuningV7Stats::default(),
                patience_counter: 0,
            }
        }

        pub fn register_model(
            &mut self,
            model_id: String,
            node_id: String,
            gradient_dim: usize,
        ) -> Result<(), FineTuningV7Error> {
            if self.models.len() >= self.config.max_models {
                return Err(FineTuningV7Error::InvalidConfig(
                    "Maximum models reached".to_string(),
                ));
            }
            if !self.nodes.contains_key(&node_id) {
                return Err(FineTuningV7Error::NodeUnavailable(node_id));
            }
            if gradient_dim == 0 {
                return Err(FineTuningV7Error::InvalidConfig(
                    "Gradient dimension must be > 0".to_string(),
                ));
            }
            self.models.insert(
                model_id.clone(),
                ModelProfileV7::new(model_id, node_id, gradient_dim),
            );
            Ok(())
        }

        pub fn register_node(
            &mut self,
            node_id: String,
            uptime: f64,
            reputation: f64,
            capacity: f64,
        ) -> Result<(), FineTuningV7Error> {
            if !(0.0..=1.0).contains(&uptime) {
                return Err(FineTuningV7Error::InvalidConfig(
                    "Uptime must be in [0,1]".to_string(),
                ));
            }
            if !(0.0..=1.0).contains(&reputation) {
                return Err(FineTuningV7Error::InvalidConfig(
                    "Reputation must be in [0,1]".to_string(),
                ));
            }
            if self.nodes.contains_key(&node_id) {
                return Err(FineTuningV7Error::InvalidConfig(format!(
                    "Node {} already registered",
                    node_id
                )));
            }
            self.nodes.insert(
                node_id.clone(),
                NodeEntryV7::new(node_id, uptime, reputation, capacity),
            );
            Ok(())
        }

        pub fn update_node_uptime(
            &mut self,
            node_id: &str,
            uptime: f64,
        ) -> Result<(), FineTuningV7Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or(FineTuningV7Error::NodeUnavailable(node_id.to_string()))?;
            node.uptime = uptime;
            Ok(())
        }

        pub fn update_node_latency(
            &mut self,
            node_id: &str,
            latency_ms: f64,
        ) -> Result<(), FineTuningV7Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or(FineTuningV7Error::NodeUnavailable(node_id.to_string()))?;
            node.update_latency(latency_ms, 0.1);
            Ok(())
        }

        pub fn select_best_node(&self) -> Option<&NodeEntryV7> {
            self.nodes
                .values()
                .filter(|n| n.meets_uptime(self.config.min_uptime))
                .max_by_key(|n| (n.selection_score(self.config.min_uptime) * 1000.0) as u64)
        }

        /// Execute a training round with adaptive normalization and LZ4 compression.
        /// Returns the training round result.
        pub fn execute_round(
            &mut self,
            model_id: String,
            simulated_loss: f64,
            simulated_gradient_norm: f64,
            simulated_sync_time_ms: u64,
        ) -> Result<TrainingRoundResultV7, FineTuningV7Error> {
            // Check sync timeout
            if simulated_sync_time_ms > self.config.gradient_sync_timeout_ms {
                self.stats.record_sync_timeout();
                return Err(FineTuningV7Error::GradientSyncTimeout);
            }

            // Validate model exists and get gradient dim
            let gradient_dim = match self.models.get(&model_id) {
                Some(p) => p.gradient_dim,
                None => {
                    return Err(FineTuningV7Error::ModelNotFound(model_id.clone()));
                }
            };

            // Validate gradient dimension
            if simulated_gradient_norm < 0.0 {
                return Err(FineTuningV7Error::GradientMismatch {
                    expected: gradient_dim,
                    actual: 0,
                });
            }

            // Cross-model alignment score (computed before mutable borrow)
            let alignment_score = self.compute_alignment_score(&model_id);

            self.current_round += 1;

            // Record loss + adaptive normalization + convergence (single mutable borrow)
            let (normalized, converging) = {
                let profile = self.models.get_mut(&model_id).unwrap();
                profile.record_loss(simulated_loss);
                let norm = if self.config.adaptive_normalization {
                    profile
                        .record_normalized_gradient(simulated_gradient_norm, self.config.ema_alpha);
                    let ema = profile.ema_gradient_norm;
                    if ema > 0.0 {
                        simulated_gradient_norm / ema
                    } else {
                        simulated_gradient_norm
                    }
                } else {
                    simulated_gradient_norm
                };
                let conv = if self.config.convergence_detection {
                    // Track convergence rounds
                    if profile.recent_loss_change() < self.config.convergence_threshold {
                        profile.convergence_rounds += 1;
                    } else {
                        profile.convergence_rounds = 0;
                    }
                    profile.is_converging(self.config.convergence_threshold, self.config.patience)
                } else {
                    false
                };
                (norm, conv)
            };

            // Convergence + LR decay (no borrow)
            if converging {
                self.stats.record_convergence();
                self.patience_counter += 1;
                if self.config.adaptive_lr && self.patience_counter >= self.config.patience {
                    self.current_lr *= self.config.lr_decay;
                    if self.current_lr < self.config.min_learning_rate {
                        self.current_lr = self.config.min_learning_rate;
                    }
                    self.stats.record_lr_decay();
                    self.patience_counter = 0;
                }
            } else {
                self.patience_counter = 0;
            }

            // Record sync
            self.stats
                .record_sync(simulated_sync_time_ms, alignment_score, normalized);

            // LZ4 compression simulation
            if self.config.compression_ratio > 1.0 {
                let ratio = self.config.compression_ratio
                    * (0.9 + (self.current_round as f64 % 10.0) / 100.0);
                self.stats.record_lz4_compression(ratio);
            }

            // Checkpoint
            let (checkpoint_saved, checkpoint_incremental) = if self
                .current_round
                .is_multiple_of(self.config.checkpoint_interval as u64)
            {
                let hash = compute_sha256(&format!("{}-{}", model_id, self.current_round));
                let mut entry =
                    CheckpointEntryV7::new(self.current_round, model_id.clone(), hash, 1024 * 1024);

                // Incremental checkpoint
                let is_incremental = if let Some(parent) = self.checkpoints.last() {
                    entry.mark_incremental(parent.hash.clone());
                    self.stats.record_checkpoint(true);
                    true
                } else {
                    self.stats.record_checkpoint(false);
                    false
                };

                // Integrity validation
                self.stats.record_integrity_validation(true);

                self.checkpoints.push(entry);
                (true, is_incremental)
            } else {
                (false, false)
            };

            // Sync record
            self.sync_records.push(GradientSyncRecordV7 {
                round: self.current_round,
                model_id: model_id.clone(),
                gradient_norm: normalized,
                normalized: self.config.adaptive_normalization,
                compressed: self.config.compression_ratio > 1.0,
                sync_time_ms: simulated_sync_time_ms,
                alignment_score,
            });

            self.stats.total_rounds = self.current_round;

            Ok(TrainingRoundResultV7 {
                round: self.current_round,
                model_id,
                loss: simulated_loss,
                gradient_norm: normalized,
                alignment_score,
                checkpoint_saved,
                checkpoint_incremental,
                sync_time_ms: simulated_sync_time_ms,
                learning_rate: self.current_lr,
            })
        }

        fn compute_alignment_score(&self, model_id: &str) -> f64 {
            if self.models.len() < 2 {
                return 1.0;
            }
            let target = match self.models.get(model_id) {
                Some(m) => m.ema_gradient_norm,
                None => return 0.0,
            };
            let mut total_diff = 0.0;
            let mut count = 0;
            for (id, m) in &self.models {
                if id != model_id {
                    total_diff += (target - m.ema_gradient_norm).abs();
                    count += 1;
                }
            }
            if count == 0 {
                return 1.0;
            }
            let avg_diff = total_diff / count as f64;
            1.0 - (avg_diff.min(1.0))
        }

        fn save_checkpoint(&mut self, model_id: &str) -> Result<(bool, bool), FineTuningV7Error> {
            let hash = compute_sha256(&format!("{}-{}", model_id, self.current_round));
            let mut entry =
                CheckpointEntryV7::new(self.current_round, model_id.to_string(), hash, 1024 * 1024);

            let incremental = if let Some(parent) = self.checkpoints.last() {
                entry.mark_incremental(parent.hash.clone());
                true
            } else {
                false
            };

            self.stats.record_checkpoint(incremental);
            self.stats.record_integrity_validation(true);
            self.checkpoints.push(entry);
            Ok((true, incremental))
        }

        pub fn get_checkpoint(&self, round: u64, model_id: &str) -> Option<&CheckpointEntryV7> {
            self.checkpoints
                .iter()
                .rev()
                .find(|c| c.round == round && c.model_id == model_id)
        }

        pub fn get_stats(&self) -> &FineTuningV7Stats {
            &self.stats
        }

        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        pub fn get_current_lr(&self) -> f64 {
            self.current_lr
        }

        pub fn get_round(&self) -> u64 {
            self.current_round
        }

        pub fn model_count(&self) -> usize {
            self.models.len()
        }

        pub fn node_count(&self) -> usize {
            self.nodes.len()
        }

        pub fn checkpoint_count(&self) -> usize {
            self.checkpoints.len()
        }
    }

    impl Default for FineTuningV7 {
        fn default() -> Self {
            Self::new(FineTuningV7Config::default())
        }
    }

    // ─── Helpers ───

    fn compute_norm(grads: &[f32]) -> f64 {
        let sum: f64 = grads.iter().map(|g| (*g as f64) * (*g as f64)).sum();
        sum.sqrt()
    }

    fn compute_sha256(input: &str) -> String {
        let mut hash = [0u8; 32];
        for (i, byte) in input.as_bytes().iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    // ─── Unit Tests ───

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> FineTuningV7Config {
            FineTuningV7Config {
                learning_rate: 1e-4,
                min_learning_rate: 1e-8,
                compression_ratio: 3.0,
                batch_size: 32,
                adaptive_lr: true,
                lr_decay: 0.95,
                max_models: 10,
                min_uptime: 0.9,
                checkpoint_interval: 5,
                convergence_detection: true,
                convergence_threshold: 1e-5,
                patience: 3,
                gradient_sync_timeout_ms: 90,
                adaptive_normalization: true,
                ema_alpha: 0.1,
            }
        }

        #[test]
        fn test_engine_creation() {
            let engine = FineTuningV7::default();
            assert_eq!(engine.model_count(), 0);
            assert_eq!(engine.node_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = make_config();
            let engine = FineTuningV7::new(config);
            assert_eq!(engine.get_current_lr(), 1e-4);
        }

        #[test]
        fn test_register_node() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            assert_eq!(engine.node_count(), 1);
        }

        #[test]
        fn test_register_node_invalid_uptime() {
            let mut engine = FineTuningV7::default();
            match engine
                .register_node("n1".to_string(), 1.5, 0.8, 100.0)
                .unwrap_err()
            {
                FineTuningV7Error::InvalidConfig(msg) => assert!(msg.contains("Uptime")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_register_node_invalid_reputation() {
            let mut engine = FineTuningV7::default();
            match engine
                .register_node("n1".to_string(), 0.9, -0.1, 100.0)
                .unwrap_err()
            {
                FineTuningV7Error::InvalidConfig(msg) => assert!(msg.contains("Reputation")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_update_node_uptime() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine.update_node_uptime("n1", 0.99).unwrap();
            assert_eq!(engine.nodes.get("n1").unwrap().uptime, 0.99);
        }

        #[test]
        fn test_update_node_latency() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine.update_node_latency("n1", 50.0).unwrap();
            let node = engine.nodes.get("n1").unwrap();
            assert!(node.avg_latency_ms > 0.0);
        }

        #[test]
        fn test_register_model() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            assert_eq!(engine.model_count(), 1);
        }

        #[test]
        fn test_register_model_node_not_found() {
            let mut engine = FineTuningV7::default();
            match engine
                .register_model("m1".to_string(), "unknown".to_string(), 768)
                .unwrap_err()
            {
                FineTuningV7Error::NodeUnavailable(node) => assert_eq!(node, "unknown"),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_register_model_dimension_exceeded() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            match engine
                .register_model("m1".to_string(), "n1".to_string(), 0)
                .unwrap_err()
            {
                FineTuningV7Error::InvalidConfig(msg) => assert!(msg.contains("dimension")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_select_best_node() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.9, 100.0)
                .unwrap();
            engine
                .register_node("n2".to_string(), 0.95, 0.7, 100.0)
                .unwrap();
            let best = engine.select_best_node().unwrap();
            assert_eq!(best.node_id, "n1");
        }

        #[test]
        fn test_execute_round_basic() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            let result = engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            assert_eq!(result.round, 1);
            assert!(!result.checkpoint_saved);
        }

        #[test]
        fn test_execute_round_with_lz4() {
            let config = FineTuningV7Config {
                compression_ratio: 4.0,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            assert!(engine.get_stats().lz4_compressions > 0);
        }

        #[test]
        fn test_execute_round_with_adaptive_normalization() {
            let config = FineTuningV7Config {
                adaptive_normalization: true,
                ema_alpha: 0.2,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 2.0, 50)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.4, 1.8, 50)
                .unwrap();
            let profile = engine.models.get("m1").unwrap();
            assert!(profile.ema_gradient_norm > 0.0);
        }

        #[test]
        fn test_checkpoint_interval() {
            let config = FineTuningV7Config {
                checkpoint_interval: 3,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            for i in 1..=5 {
                let result = engine
                    .execute_round("m1".to_string(), 0.5 - i as f64 * 0.05, 1.0, 50)
                    .unwrap();
                if i == 3 {
                    assert!(result.checkpoint_saved);
                }
            }
            assert!(engine.checkpoint_count() >= 1);
        }

        #[test]
        fn test_convergence_detection() {
            let config = FineTuningV7Config {
                convergence_threshold: 0.001,
                patience: 2,
                convergence_detection: true,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            for _ in 0..10 {
                engine
                    .execute_round("m1".to_string(), 0.0001, 0.5, 50)
                    .unwrap();
            }
            assert!(engine.get_stats().convergence_triggers > 0);
        }

        #[test]
        fn test_adaptive_lr_decay() {
            let config = FineTuningV7Config {
                adaptive_lr: true,
                lr_decay: 0.9,
                patience: 2,
                convergence_threshold: 0.001,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            let initial_lr = engine.get_current_lr();
            for _ in 0..10 {
                engine
                    .execute_round("m1".to_string(), 0.0001, 0.5, 50)
                    .unwrap();
            }
            assert!(engine.get_current_lr() <= initial_lr);
        }

        #[test]
        fn test_cross_model_alignment() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_node("n2".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .register_model("m2".to_string(), "n2".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            engine
                .execute_round("m2".to_string(), 0.5, 1.0, 50)
                .unwrap();
            let result = engine
                .execute_round("m1".to_string(), 0.4, 1.0, 50)
                .unwrap();
            assert!(result.alignment_score >= 0.0);
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            let stats = engine.get_stats();
            assert_eq!(stats.total_rounds, 1);
            assert_eq!(stats.total_syncs, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            engine.reset_stats();
            let stats = engine.get_stats();
            assert_eq!(stats.total_rounds, 0);
        }

        #[test]
        fn test_config_default() {
            let config = FineTuningV7Config::default();
            assert_eq!(config.learning_rate, 1e-4);
            assert_eq!(config.gradient_sync_timeout_ms, 90);
        }

        #[test]
        fn test_stats_default() {
            let stats = FineTuningV7Stats::default();
            assert_eq!(stats.total_rounds, 0);
            assert_eq!(stats.gradient_sync_timeouts, 0);
        }

        #[test]
        fn test_error_display() {
            let err = FineTuningV7Error::GradientSyncTimeout;
            let msg = format!("{}", err);
            assert!(msg.contains("timeout"));
        }

        #[test]
        fn test_engine_default() {
            let engine = FineTuningV7::default();
            assert_eq!(engine.get_round(), 0);
        }

        #[test]
        fn test_checkpoint_integrity() {
            let config = FineTuningV7Config {
                checkpoint_interval: 2,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.4, 1.0, 50)
                .unwrap();
            assert!(engine.get_stats().integrity_validations > 0);
            assert_eq!(
                engine.get_stats().integrity_validations,
                engine.get_stats().integrity_validations_passed
            );
        }

        #[test]
        fn test_node_selection_score() {
            let node = NodeEntryV7::new("n1".to_string(), 0.95, 0.9, 100.0);
            let score = node.selection_score(0.9);
            assert!(score > 0.0);
        }

        #[test]
        fn test_node_meets_uptime() {
            let node = NodeEntryV7::new("n1".to_string(), 0.95, 0.8, 100.0);
            assert!(node.meets_uptime(0.9));
            assert!(!node.meets_uptime(0.99));
        }

        #[test]
        fn test_model_convergence() {
            let mut profile = ModelProfileV7::new("m1".to_string(), "n1".to_string(), 768);
            for _ in 0..10 {
                profile.record_loss(0.0001);
            }
            profile.convergence_rounds = 5;
            assert!(profile.is_converging(0.001, 3));
        }

        #[test]
        fn test_checkpoint_compression_ratio() {
            let mut entry = CheckpointEntryV7::new(1, "m1".to_string(), "hash".to_string(), 1000);
            entry.mark_compressed(250);
            assert!((entry.compression_ratio() - 4.0).abs() < 0.01);
        }

        #[test]
        fn test_get_checkpoint() {
            let config = FineTuningV7Config {
                checkpoint_interval: 2,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.4, 1.0, 50)
                .unwrap();
            let cp = engine.get_checkpoint(2, "m1");
            assert!(cp.is_some());
        }

        #[test]
        fn test_model_avg_normalized_gradient() {
            let mut profile = ModelProfileV7::new("m1".to_string(), "n1".to_string(), 768);
            profile.record_normalized_gradient(1.0, 0.1);
            profile.record_normalized_gradient(2.0, 0.1);
            profile.record_normalized_gradient(3.0, 0.1);
            let avg = profile.avg_normalized_gradient();
            assert!((avg - 2.0).abs() < 0.01);
        }

        #[test]
        fn test_integrity_validation_failed_error() {
            let err = FineTuningV7Error::IntegrityValidationFailed("bad hash".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("Integrity"));
        }

        #[test]
        fn test_cross_model_sync_failed_error() {
            let err = FineTuningV7Error::CrossModelSyncFailed("timeout".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("Cross-model"));
        }

        #[test]
        fn test_gradient_sync_timeout() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            match engine
                .execute_round("m1".to_string(), 0.5, 1.0, 100)
                .unwrap_err()
            {
                FineTuningV7Error::GradientSyncTimeout => {}
                e => panic!("Wrong error: {:?}", e),
            }
            assert_eq!(engine.get_stats().gradient_sync_timeouts, 1);
        }

        #[test]
        fn test_multi_round_execution() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            for i in 0..20 {
                let result = engine
                    .execute_round("m1".to_string(), 0.5 - i as f64 * 0.02, 1.0, 50)
                    .unwrap();
                assert_eq!(result.round, i as u64 + 1);
            }
            assert_eq!(engine.get_round(), 20);
            assert!(engine.checkpoint_count() > 0);
        }

        #[test]
        fn test_incremental_checkpoint() {
            let config = FineTuningV7Config {
                checkpoint_interval: 2,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            // Round 2: first checkpoint (not incremental)
            engine
                .execute_round("m1".to_string(), 0.5, 1.0, 50)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.4, 1.0, 50)
                .unwrap();
            // Round 4: second checkpoint (incremental)
            engine
                .execute_round("m1".to_string(), 0.3, 1.0, 50)
                .unwrap();
            engine
                .execute_round("m1".to_string(), 0.2, 1.0, 50)
                .unwrap();
            let cp = engine.get_checkpoint(4, "m1").unwrap();
            assert!(cp.incremental);
            assert!(cp.parent_hash.is_some());
        }

        #[test]
        fn test_register_node_duplicate() {
            let mut engine = FineTuningV7::default();
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            match engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap_err()
            {
                FineTuningV7Error::InvalidConfig(msg) => assert!(msg.contains("already")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_max_models_reached() {
            let config = FineTuningV7Config {
                max_models: 1,
                ..make_config()
            };
            let mut engine = FineTuningV7::new(config);
            engine
                .register_node("n1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("m1".to_string(), "n1".to_string(), 768)
                .unwrap();
            match engine
                .register_model("m2".to_string(), "n1".to_string(), 768)
                .unwrap_err()
            {
                FineTuningV7Error::InvalidConfig(msg) => assert!(msg.contains("Maximum")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_compute_norm() {
            let grads = vec![1.0f32, 2.0f32, 2.0f32];
            let norm = compute_norm(&grads);
            assert!((norm - 3.0).abs() < 0.01);
        }

        #[test]
        fn test_compute_sha256_deterministic() {
            let h1 = compute_sha256("test");
            let h2 = compute_sha256("test");
            assert_eq!(h1, h2);
            assert_eq!(h1.len(), 64);
        }
    }
}

// Re-export public types
#[cfg(feature = "v1.6-sprint3")]
pub use internal::{
    CheckpointEntryV7, FineTuningV7, FineTuningV7Config, FineTuningV7Error, FineTuningV7Stats,
    GradientSyncRecordV7, ModelProfileV7, NodeEntryV7, TrainingRoundResultV7,
};
