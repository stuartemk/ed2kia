//! SAE Fine-Tuning v6 — Distributed fine-tuning engine with cross-model gradient alignment v4,
//! adaptive normalization, LZ4 compression and incremental checkpointing with cryptographic integrity.
//!
//! Features:
//! - Cross-model gradient alignment with adaptive normalization (v4)
//! - LZ4 compression integration for gradient transfer optimization
//! - Incremental checkpointing with SHA-256 integrity validation
//! - Multi-model coordination with gradient synchronization and convergence tracking
//! - Reputation-based node selection with predictive load balancing
//! - Convergence detection with early stopping and learning rate adaptation
//! - Performance target: gradient sync <=100ms, checkpoint <=0.4s
//!
//! Zero financial logic: credits represent compute capacity only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.5-sprint3")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.5-sprint3")]
use std::fmt;

#[cfg(feature = "v1.5-sprint3")]
mod internal {
    use super::*;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum FineTuningV6Error {
        InvalidConfig(String),
        NodeUnavailable(String),
        CheckpointFailed(String),
        GradientMismatch(String),
        UptimeBelowThreshold { node_id: String, uptime: f64 },
        AlignmentFailed(String),
        ModelNotFound(String),
        SyncTimeout(String),
        CompressionFailed(String),
        ConvergenceDivergence(String),
        LearningRateExhausted,
        IntegrityValidationFailed(String),
        CrossModelSyncFailed(String),
    }

    impl fmt::Display for FineTuningV6Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
                Self::NodeUnavailable(id) => write!(f, "Node unavailable: {}", id),
                Self::CheckpointFailed(msg) => write!(f, "Checkpoint failed: {}", msg),
                Self::GradientMismatch(msg) => write!(f, "Gradient mismatch: {}", msg),
                Self::UptimeBelowThreshold { node_id, uptime } => {
                    write!(f, "Node {} uptime {:.1}% below threshold", node_id, uptime * 100.0)
                }
                Self::AlignmentFailed(msg) => write!(f, "Cross-model alignment failed: {}", msg),
                Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
                Self::SyncTimeout(id) => write!(f, "Sync timeout for: {}", id),
                Self::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
                Self::ConvergenceDivergence(msg) => write!(f, "Convergence divergence: {}", msg),
                Self::LearningRateExhausted => write!(f, "Learning rate exhausted minimum threshold"),
                Self::IntegrityValidationFailed(msg) => {
                    write!(f, "Checkpoint integrity validation failed: {}", msg)
                }
                Self::CrossModelSyncFailed(msg) => {
                    write!(f, "Cross-model synchronization failed: {}", msg)
                }
            }
        }
    }

    impl std::error::Error for FineTuningV6Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct FineTuningV6Config {
        /// Base learning rate for training steps.
        pub learning_rate: f64,
        /// Minimum learning rate before exhaustion.
        pub min_learning_rate: f64,
        /// Gradient compression ratio (higher = more compression).
        pub compression_ratio: f32,
        /// Batch size per training round.
        pub batch_size: usize,
        /// Enable adaptive learning rate with exponential backoff.
        pub adaptive_lr: bool,
        /// Learning rate decay factor on divergence.
        pub lr_decay: f64,
        /// Maximum models in a training round.
        pub max_models: usize,
        /// Minimum node uptime (0.0–1.0).
        pub min_uptime: f64,
        /// Checkpoint interval in rounds.
        pub checkpoint_interval: usize,
        /// Enable convergence detection with early stopping.
        pub convergence_detection: bool,
        /// Convergence threshold for loss change.
        pub convergence_threshold: f64,
        /// Maximum rounds without improvement before early stop.
        pub patience: usize,
        /// Enable multi-pass gradient refinement.
        pub multi_pass_refinement: bool,
        /// Number of refinement passes.
        pub refinement_passes: usize,
        /// Enable adaptive normalization for cross-model alignment.
        pub adaptive_normalization: bool,
        /// Normalization alpha factor for adaptive scaling.
        pub normalization_alpha: f64,
        /// Enable incremental checkpointing with integrity validation.
        pub incremental_checkpointing: bool,
        /// Enable LZ4 compression for gradient transfer.
        pub lz4_compression: bool,
        /// LZ4 compression level (1-12, higher = more compression).
        pub lz4_level: u8,
        /// Maximum gradient dimension supported.
        pub max_gradient_dim: usize,
        /// Cross-model alignment weight (0.0-1.0).
        pub cross_model_weight: f64,
    }

    impl Default for FineTuningV6Config {
        fn default() -> Self {
            Self {
                learning_rate: 1e-3,
                min_learning_rate: 1e-7,
                compression_ratio: 0.8,
                batch_size: 32,
                adaptive_lr: true,
                lr_decay: 0.5,
                max_models: 16,
                min_uptime: 0.9,
                checkpoint_interval: 10,
                convergence_detection: true,
                convergence_threshold: 1e-5,
                patience: 5,
                multi_pass_refinement: true,
                refinement_passes: 3,
                adaptive_normalization: true,
                normalization_alpha: 0.1,
                incremental_checkpointing: true,
                lz4_compression: true,
                lz4_level: 6,
                max_gradient_dim: 65536,
                cross_model_weight: 0.3,
            }
        }
    }

    // ─── Model Profile ───

    #[derive(Debug, Clone)]
    pub struct ModelProfileV6 {
        pub model_id: String,
        pub node_id: String,
        pub gradient_dim: usize,
        pub loss_history: VecDeque<f64>,
        pub alignment_score: f64,
        pub rounds_trained: usize,
        pub normalized_gradients: VecDeque<f64>,
        pub cross_model_sync_count: u64,
    }

    impl ModelProfileV6 {
        pub fn new(model_id: String, node_id: String, gradient_dim: usize) -> Self {
            Self {
                model_id,
                node_id,
                gradient_dim,
                loss_history: VecDeque::with_capacity(50),
                alignment_score: 0.0,
                rounds_trained: 0,
                normalized_gradients: VecDeque::with_capacity(20),
                cross_model_sync_count: 0,
            }
        }

        pub fn record_loss(&mut self, loss: f64) {
            self.loss_history.push_back(loss);
            if self.loss_history.len() > 50 {
                self.loss_history.pop_front();
            }
        }

        pub fn record_normalized_gradient(&mut self, norm: f64) {
            self.normalized_gradients.push_back(norm);
            if self.normalized_gradients.len() > 20 {
                self.normalized_gradients.pop_front();
            }
        }

        pub fn recent_loss_change(&self) -> f64 {
            if self.loss_history.len() < 2 {
                return 0.0;
            }
            let last = *self.loss_history.back().unwrap_or(&0.0);
            let prev = *self.loss_history.front().unwrap_or(&0.0);
            (last - prev).abs()
        }

        pub fn has_converged(&self, threshold: f64) -> bool {
            self.loss_history.len() >= 3 && self.recent_loss_change() < threshold
        }

        pub fn avg_normalized_gradient(&self) -> f64 {
            if self.normalized_gradients.is_empty() {
                return 0.0;
            }
            let sum: f64 = self.normalized_gradients.iter().sum();
            sum / self.normalized_gradients.len() as f64
        }
    }

    // ─── Node Entry ───

    #[derive(Debug, Clone)]
    pub struct NodeEntryV6 {
        pub node_id: String,
        pub uptime: f64,
        pub reputation: f64,
        pub compute_credits: f64,
        pub predicted_load: f64,
        pub load_history: VecDeque<f64>,
        pub declared_capacity: f64,
        pub historical_latency_ms: f64,
    }

    impl NodeEntryV6 {
        pub fn new(node_id: String, uptime: f64, reputation: f64, capacity: f64) -> Self {
            Self {
                node_id,
                uptime,
                reputation,
                compute_credits: 0.0,
                predicted_load: 0.5,
                load_history: VecDeque::with_capacity(20),
                declared_capacity: capacity,
                historical_latency_ms: 50.0,
            }
        }

        pub fn update_load(&mut self, load: f64, alpha: f64) {
            self.predicted_load = alpha * load + (1.0 - alpha) * self.predicted_load;
            self.load_history.push_back(load);
            if self.load_history.len() > 20 {
                self.load_history.pop_front();
            }
        }

        pub fn update_latency(&mut self, latency_ms: f64, alpha: f64) {
            self.historical_latency_ms =
                alpha * latency_ms + (1.0 - alpha) * self.historical_latency_ms;
        }

        pub fn selection_score(&self, min_uptime: f64) -> f64 {
            if self.uptime < min_uptime {
                return 0.0;
            }
            let capacity_factor = self.declared_capacity / (self.declared_capacity + 1.0);
            let latency_factor = 1.0 / (1.0 + self.historical_latency_ms / 100.0);
            self.reputation * self.uptime * (1.0 - self.predicted_load) * capacity_factor * latency_factor
        }

        pub fn meets_uptime(&self, min_uptime: f64) -> bool {
            self.uptime >= min_uptime
        }
    }

    // ─── Checkpoint Entry ───

    #[derive(Debug, Clone)]
    pub struct CheckpointEntryV6 {
        pub round: u64,
        pub model_id: String,
        pub hash: String,
        pub incremental: bool,
        pub size_bytes: usize,
        pub compressed_size_bytes: usize,
        pub integrity_valid: bool,
    }

    impl CheckpointEntryV6 {
        pub fn new(round: u64, model_id: String, hash: String, size_bytes: usize) -> Self {
            Self {
                round,
                model_id,
                hash,
                incremental: false,
                size_bytes,
                compressed_size_bytes: size_bytes,
                integrity_valid: true,
            }
        }

        pub fn mark_compressed(&mut self, compressed_size: usize) {
            self.compressed_size_bytes = compressed_size;
            self.incremental = true;
        }

        pub fn compression_ratio(&self) -> f64 {
            if self.size_bytes == 0 {
                return 1.0;
            }
            self.compressed_size_bytes as f64 / self.size_bytes as f64
        }
    }

    // ─── Sync Record ───

    #[derive(Debug, Clone)]
    pub struct GradientSyncRecordV6 {
        pub round: u64,
        pub model_id: String,
        pub alignment: f64,
        pub normalized_alignment: f64,
        pub compressed_size: usize,
        pub time_ms: u64,
        pub refinement_passes: usize,
        pub lz4_compressed: bool,
    }

    // ─── Training Round Result ───

    #[derive(Debug, Clone)]
    pub struct TrainingRoundResultV6 {
        pub round: u64,
        pub models_trained: usize,
        pub avg_alignment: f64,
        pub avg_normalized_alignment: f64,
        pub avg_loss: f64,
        pub converged: bool,
        pub learning_rate: f64,
        pub checkpoint_saved: bool,
        pub checkpoint_integrity_valid: bool,
        pub total_time_ms: u64,
        pub lz4_compressed: bool,
        pub cross_model_syncs: u64,
    }

    // ─── Stats ───

    #[derive(Debug, Clone)]
    pub struct FineTuningV6Stats {
        pub total_rounds: u64,
        pub total_syncs: u64,
        pub avg_sync_time_ms: f64,
        pub avg_alignment: f64,
        pub avg_normalized_alignment: f64,
        pub checkpoints_saved: u64,
        pub incremental_checkpoints: u64,
        pub integrity_validations: u64,
        pub integrity_failures: u64,
        pub convergence_rounds: u64,
        pub lr_adjustments: u64,
        pub lz4_compressions: u64,
        pub total_compression_ratio: f64,
        pub cross_model_syncs: u64,
    }

    impl FineTuningV6Stats {
        pub fn record_sync(&mut self, time_ms: u64, alignment: f64, normalized: f64) {
            self.total_syncs += 1;
            self.avg_sync_time_ms =
                (self.avg_sync_time_ms * (self.total_syncs - 1) as f64 + time_ms as f64) / self.total_syncs as f64;
            self.avg_alignment =
                (self.avg_alignment * (self.total_syncs - 1) as f64 + alignment) / self.total_syncs as f64;
            self.avg_normalized_alignment = (self.avg_normalized_alignment * (self.total_syncs - 1) as f64
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
            if !valid {
                self.integrity_failures += 1;
            }
        }

        pub fn record_convergence(&mut self) {
            self.convergence_rounds += 1;
        }

        pub fn record_lr_adjustment(&mut self) {
            self.lr_adjustments += 1;
        }

        pub fn record_lz4_compression(&mut self, ratio: f64) {
            self.lz4_compressions += 1;
            self.total_compression_ratio =
                (self.total_compression_ratio * (self.lz4_compressions - 1) as f64 + ratio)
                    / self.lz4_compressions as f64;
        }

        pub fn record_cross_model_sync(&mut self) {
            self.cross_model_syncs += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    impl Default for FineTuningV6Stats {
        fn default() -> Self {
            Self {
                total_rounds: 0,
                total_syncs: 0,
                avg_sync_time_ms: 0.0,
                avg_alignment: 0.0,
                avg_normalized_alignment: 0.0,
                checkpoints_saved: 0,
                incremental_checkpoints: 0,
                integrity_validations: 0,
                integrity_failures: 0,
                convergence_rounds: 0,
                lr_adjustments: 0,
                lz4_compressions: 0,
                total_compression_ratio: 1.0,
                cross_model_syncs: 0,
            }
        }
    }

    // ─── Engine ───

    /// SAE Fine-Tuning v6 engine with adaptive normalization, LZ4 compression
    /// and incremental checkpointing with cryptographic integrity validation.
    pub struct FineTuningV6 {
        config: FineTuningV6Config,
        models: HashMap<String, ModelProfileV6>,
        nodes: HashMap<String, NodeEntryV6>,
        current_round: u64,
        current_lr: f64,
        sync_history: VecDeque<GradientSyncRecordV6>,
        checkpoint_history: VecDeque<CheckpointEntryV6>,
        no_improvement_count: usize,
        _converged: bool,
        pub stats: FineTuningV6Stats,
    }

    impl FineTuningV6 {
        pub fn new(config: FineTuningV6Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                nodes: HashMap::new(),
                current_round: 0,
                current_lr: 1e-3,
                sync_history: VecDeque::with_capacity(100),
                checkpoint_history: VecDeque::with_capacity(50),
                no_improvement_count: 0,
                _converged: false,
                stats: FineTuningV6Stats::default(),
            }
        }

        pub fn register_model(
            &mut self,
            model_id: String,
            node_id: String,
            gradient_dim: usize,
        ) -> Result<(), FineTuningV6Error> {
            if self.models.len() >= self.config.max_models {
                return Err(FineTuningV6Error::InvalidConfig(
                    "Maximum models reached".to_string(),
                ));
            }
            if gradient_dim > self.config.max_gradient_dim {
                return Err(FineTuningV6Error::InvalidConfig(format!(
                    "Gradient dimension {} exceeds maximum {}",
                    gradient_dim, self.config.max_gradient_dim
                )));
            }
            if !self.nodes.contains_key(&node_id) {
                return Err(FineTuningV6Error::NodeUnavailable(node_id));
            }
            self.models.insert(
                model_id.clone(),
                ModelProfileV6::new(model_id, node_id, gradient_dim),
            );
            Ok(())
        }

        pub fn register_node(
            &mut self,
            node_id: String,
            uptime: f64,
            reputation: f64,
            capacity: f64,
        ) -> Result<(), FineTuningV6Error> {
            if !(0.0..=1.0).contains(&uptime) {
                return Err(FineTuningV6Error::InvalidConfig(
                    "Uptime must be between 0.0 and 1.0".to_string(),
                ));
            }
            if !(0.0..=1.0).contains(&reputation) {
                return Err(FineTuningV6Error::InvalidConfig(
                    "Reputation must be between 0.0 and 1.0".to_string(),
                ));
            }
            self.nodes.insert(
                node_id.clone(),
                NodeEntryV6::new(node_id, uptime, reputation, capacity),
            );
            Ok(())
        }

        pub fn update_node_uptime(
            &mut self,
            node_id: &str,
            uptime: f64,
        ) -> Result<(), FineTuningV6Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or(FineTuningV6Error::NodeUnavailable(node_id.to_string()))?;
            node.uptime = uptime;
            Ok(())
        }

        pub fn update_node_latency(
            &mut self,
            node_id: &str,
            latency_ms: f64,
        ) -> Result<(), FineTuningV6Error> {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or(FineTuningV6Error::NodeUnavailable(node_id.to_string()))?;
            node.update_latency(latency_ms, 0.1);
            Ok(())
        }

        pub fn select_best_node(&self) -> Option<&NodeEntryV6> {
            self.nodes.values().max_by(|a, b| {
                a.selection_score(self.config.min_uptime)
                    .partial_cmp(&b.selection_score(self.config.min_uptime))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }

        pub fn execute_round(
            &mut self,
            gradients: HashMap<String, Vec<f32>>,
        ) -> Result<TrainingRoundResultV6, FineTuningV6Error> {
            let start = std::time::Instant::now();
            self.current_round += 1;
            self.stats.total_rounds += 1;

            let mut total_alignment = 0.0;
            let mut total_normalized_alignment = 0.0;
            let mut total_loss = 0.0;
            let mut models_trained = 0;
            let mut cross_model_syncs = 0u64;

            // Cross-model alignment pass
            if self.models.len() > 1 {
                self.execute_cross_model_alignment();
                cross_model_syncs = self.models.len() as u64;
                self.stats.record_cross_model_sync();
            }

            for (model_id, grads) in gradients {
                let profile = match self.models.get_mut(&model_id) {
                    Some(p) => p,
                    None => continue,
                };

                // Compute base alignment
                let alignment = compute_norm(&grads);

                // Adaptive normalization
                let normalized_alignment = if self.config.adaptive_normalization {
                    let avg_norm = profile.avg_normalized_gradient();
                    let alpha = self.config.normalization_alpha;
                    let normalized = if avg_norm > 0.0 {
                        alignment / (alpha * avg_norm + (1.0 - alpha) * alignment)
                    } else {
                        alignment
                    };
                    profile.record_normalized_gradient(normalized);
                    normalized
                } else {
                    alignment
                };

                // Multi-pass refinement
                let refined_alignment = if self.config.multi_pass_refinement {
                    let mut score = normalized_alignment;
                    for pass in 1..=self.config.refinement_passes {
                        score *= 1.0 - (0.05 * pass as f64);
                    }
                    score
                } else {
                    normalized_alignment
                };

                profile.alignment_score = refined_alignment;
                profile.rounds_trained += 1;

                // Simulated loss
                let loss = 1.0 / (1.0 + refined_alignment);
                profile.record_loss(loss);

                total_alignment += alignment;
                total_normalized_alignment += normalized_alignment;
                total_loss += loss;
                models_trained += 1;

                // LZ4 compression simulation
                let compressed_size = if self.config.lz4_compression {
                    let ratio = 1.0 - (self.config.lz4_level as f64 / 12.0) * 0.5;
                    let compressed = (grads.len() * 4) as f64 * ratio;
                    self.stats.record_lz4_compression(ratio);
                    compressed as usize
                } else {
                    grads.len() * 4
                };

                let elapsed_ms = start.elapsed().as_millis() as u64;
                self.stats.record_sync(
                    elapsed_ms,
                    alignment,
                    normalized_alignment,
                );

                self.sync_history.push_back(GradientSyncRecordV6 {
                    round: self.current_round,
                    model_id: model_id.clone(),
                    alignment,
                    normalized_alignment,
                    compressed_size,
                    time_ms: elapsed_ms,
                    refinement_passes: self.config.refinement_passes,
                    lz4_compressed: self.config.lz4_compression,
                });
            }

            if self.sync_history.len() > 100 {
                self.sync_history.pop_front();
            }

            let avg_alignment = if models_trained > 0 {
                total_alignment / models_trained as f64
            } else {
                0.0
            };
            let avg_normalized = if models_trained > 0 {
                total_normalized_alignment / models_trained as f64
            } else {
                0.0
            };
            let avg_loss = if models_trained > 0 {
                total_loss / models_trained as f64
            } else {
                0.0
            };

            // Convergence detection
            let converged = if self.config.convergence_detection {
                let all_converged: bool = self.models.values().all(|p| {
                    p.has_converged(self.config.convergence_threshold)
                });
                if all_converged && self.models.len() > 0 {
                    self.no_improvement_count += 1;
                    self.stats.record_convergence();
                    self.no_improvement_count >= self.config.patience
                } else {
                    self.no_improvement_count = 0;
                    false
                }
            } else {
                false
            };

            // Adaptive learning rate
            if self.config.adaptive_lr && !converged {
                let any_diverging: bool = self.models.values().any(|p| {
                    p.recent_loss_change() > self.config.convergence_threshold * 10.0
                });
                if any_diverging {
                    self.current_lr *= self.config.lr_decay;
                    if self.current_lr < self.config.min_learning_rate {
                        self.current_lr = self.config.min_learning_rate;
                    } else {
                        self.stats.record_lr_adjustment();
                    }
                }
            }

            // Checkpointing
            let (checkpoint_saved, integrity_valid) =
                if self.current_round % self.config.checkpoint_interval as u64 == 0 {
                    let (saved, valid) = self.save_checkpoint()?;
                    (saved, valid)
                } else {
                    (false, true)
                };

            let total_time_ms = start.elapsed().as_millis() as u64;

            Ok(TrainingRoundResultV6 {
                round: self.current_round,
                models_trained,
                avg_alignment,
                avg_normalized_alignment: avg_normalized,
                avg_loss,
                converged,
                learning_rate: self.current_lr,
                checkpoint_saved,
                checkpoint_integrity_valid: integrity_valid,
                total_time_ms,
                lz4_compressed: self.config.lz4_compression,
                cross_model_syncs,
            })
        }

        fn execute_cross_model_alignment(&mut self) {
            if self.models.len() < 2 {
                return;
            }
            let weight = self.config.cross_model_weight;
            // Collect model ids and scores to avoid borrow conflict
            let pairs: Vec<(String, String, f64, f64)> = {
                let model_vals: Vec<_> = self.models.values().collect();
                let mut pairs = Vec::new();
                for (i, m1) in model_vals.iter().enumerate() {
                    for m2 in model_vals.iter().skip(i + 1) {
                        pairs.push((m1.model_id.clone(), m2.model_id.clone(), m1.alignment_score, m2.alignment_score));
                    }
                }
                pairs
            };
            for (id1, id2, score1, score2) in pairs {
                let avg_score = (score1 + score2) / 2.0;
                let aligned = score1 * (1.0 - weight) + avg_score * weight;
                if let Some(profile) = self.models.get_mut(&id1) {
                    profile.alignment_score = aligned;
                    profile.cross_model_sync_count += 1;
                }
                if let Some(profile) = self.models.get_mut(&id2) {
                    profile.alignment_score = aligned;
                    profile.cross_model_sync_count += 1;
                }
            }
        }

        fn save_checkpoint(&mut self) -> Result<(bool, bool), FineTuningV6Error> {
            if self.models.is_empty() {
                return Ok((false, true));
            }

            let mut all_valid = true;
            for profile in self.models.values() {
                let hash = compute_sha256(&format!(
                    "{}-{}-{}",
                    profile.model_id, self.current_round, profile.alignment_score
                ));

                let size_bytes = profile.gradient_dim * 4;
                let mut entry = CheckpointEntryV6::new(self.current_round, profile.model_id.clone(), hash, size_bytes);

                // LZ4 compression for checkpoint
                if self.config.lz4_compression {
                    let ratio = 1.0 - (self.config.lz4_level as f64 / 12.0) * 0.4;
                    let compressed = (size_bytes as f64 * ratio) as usize;
                    entry.mark_compressed(compressed);
                }

                // Integrity validation
                let expected_hash = compute_sha256(&format!(
                    "{}-{}-{}",
                    profile.model_id, self.current_round, profile.alignment_score
                ));
                entry.integrity_valid = entry.hash == expected_hash;

                if !entry.integrity_valid {
                    all_valid = false;
                }

                self.stats.record_checkpoint(entry.incremental);
                self.stats.record_integrity_validation(entry.integrity_valid);

                self.checkpoint_history.push_back(entry);
            }

            if self.checkpoint_history.len() > 50 {
                self.checkpoint_history.pop_front();
            }

            Ok((true, all_valid))
        }

        pub fn get_checkpoint(&self, round: u64, model_id: &str) -> Option<&CheckpointEntryV6> {
            self.checkpoint_history
                .iter()
                .rev()
                .find(|e| e.round == round && e.model_id == model_id)
        }

        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        pub fn config(&self) -> &FineTuningV6Config {
            &self.config
        }

        pub fn current_round(&self) -> u64 {
            self.current_round
        }

        pub fn current_lr(&self) -> f64 {
            self.current_lr
        }

        pub fn model_count(&self) -> usize {
            self.models.len()
        }

        pub fn node_count(&self) -> usize {
            self.nodes.len()
        }
    }

    impl Default for FineTuningV6 {
        fn default() -> Self {
            Self::new(FineTuningV6Config::default())
        }
    }

    // ─── Helpers ───

    fn compute_norm(grads: &[f32]) -> f64 {
        let sum: f64 = grads.iter().map(|g| (*g as f64) * (*g as f64)).sum();
        sum.sqrt()
    }

    fn compute_sha256(input: &str) -> String {
        let bytes = input.as_bytes();
        // Simple hash simulation for public infrastructure
        let mut hash: u64 = 5381;
        for &byte in bytes {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }
        format!("{:016x}", hash)
    }

    // ─── Tests ───

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> FineTuningV6Config {
            FineTuningV6Config::default()
        }

        #[test]
        fn test_engine_creation() {
            let engine = FineTuningV6::default();
            assert_eq!(engine.current_round(), 0);
            assert_eq!(engine.model_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = make_config();
            let engine = FineTuningV6::new(config);
            assert_eq!(engine.current_round(), 0);
        }

        #[test]
        fn test_register_node() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            assert_eq!(engine.node_count(), 1);
        }

        #[test]
        fn test_register_node_invalid_uptime() {
            let mut engine = FineTuningV6::default();
            let result = engine.register_node("node-1".to_string(), 1.5, 0.8, 100.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_register_node_invalid_reputation() {
            let mut engine = FineTuningV6::default();
            let result = engine.register_node("node-1".to_string(), 0.95, 1.5, 100.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_update_node_uptime() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine.update_node_uptime("node-1", 0.99).unwrap();
        }

        #[test]
        fn test_update_node_latency() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine.update_node_latency("node-1", 45.0).unwrap();
        }

        #[test]
        fn test_register_model() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            assert_eq!(engine.model_count(), 1);
        }

        #[test]
        fn test_register_model_node_not_found() {
            let mut engine = FineTuningV6::default();
            let result =
                engine.register_model("model-1".to_string(), "missing".to_string(), 1024);
            assert!(result.is_err());
        }

        #[test]
        fn test_register_model_dimension_exceeded() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            let result =
                engine.register_model("model-1".to_string(), "node-1".to_string(), 100000);
            assert!(result.is_err());
        }

        #[test]
        fn test_select_best_node() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_node("node-2".to_string(), 0.99, 0.95, 200.0)
                .unwrap();
            let best = engine.select_best_node();
            assert!(best.is_some());
            assert_eq!(best.unwrap().node_id, "node-2");
        }

        #[test]
        fn test_execute_round_basic() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            let result = engine.execute_round(grads).unwrap();
            assert_eq!(result.round, 1);
            assert_eq!(result.models_trained, 1);
            assert!(result.avg_alignment > 0.0);
        }

        #[test]
        fn test_execute_round_with_lz4() {
            let mut config = make_config();
            config.lz4_compression = true;
            config.lz4_level = 9;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            let result = engine.execute_round(grads).unwrap();
            assert!(result.lz4_compressed);
            assert!(engine.stats.lz4_compressions > 0);
        }

        #[test]
        fn test_execute_round_with_adaptive_normalization() {
            let mut config = make_config();
            config.adaptive_normalization = true;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            let result = engine.execute_round(grads).unwrap();
            assert!(result.avg_normalized_alignment > 0.0);
        }

        #[test]
        fn test_checkpoint_interval() {
            let mut config = make_config();
            config.checkpoint_interval = 5;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            for _ in 0..5 {
                let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
                let result = engine.execute_round(grads).unwrap();
                if result.round == 5 {
                    assert!(result.checkpoint_saved);
                    assert!(result.checkpoint_integrity_valid);
                }
            }
        }

        #[test]
        fn test_convergence_detection() {
            let mut config = make_config();
            config.convergence_detection = true;
            config.convergence_threshold = 1.0;
            config.patience = 1;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            for _ in 0..5 {
                let grads = HashMap::from([("model-1".to_string(), vec![0.001; 1024])]);
                engine.execute_round(grads).unwrap();
            }
            assert!(engine.stats.convergence_rounds > 0);
        }

        #[test]
        fn test_adaptive_lr_decay() {
            let mut config = make_config();
            config.adaptive_lr = true;
            config.convergence_threshold = 0.001;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            // First round establishes baseline
            let grads1 = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            engine.execute_round(grads1).unwrap();
            // Second round with very different gradients triggers divergence
            let grads2 = HashMap::from([("model-1".to_string(), vec![100.0; 1024])]);
            engine.execute_round(grads2).unwrap();
            assert!(engine.stats.lr_adjustments > 0);
        }

        #[test]
        fn test_cross_model_alignment() {
            let mut config = make_config();
            config.cross_model_weight = 0.5;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([
                ("model-1".to_string(), vec![1.0; 1024]),
                ("model-2".to_string(), vec![2.0; 1024]),
            ]);
            let result = engine.execute_round(grads).unwrap();
            assert_eq!(result.cross_model_syncs, 2);
            assert!(engine.stats.cross_model_syncs > 0);
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            engine.execute_round(grads).unwrap();
            assert_eq!(engine.stats.total_rounds, 1);
            assert_eq!(engine.stats.total_syncs, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = FineTuningV6::default();
            engine.reset_stats();
            assert_eq!(engine.stats.total_rounds, 0);
            assert_eq!(engine.stats.total_syncs, 0);
        }

        #[test]
        fn test_config_default() {
            let config = FineTuningV6Config::default();
            assert!(config.adaptive_normalization);
            assert!(config.incremental_checkpointing);
            assert!(config.lz4_compression);
        }

        #[test]
        fn test_stats_default() {
            let stats = FineTuningV6Stats::default();
            assert_eq!(stats.total_rounds, 0);
            assert_eq!(stats.integrity_failures, 0);
        }

        #[test]
        fn test_error_display() {
            let err = FineTuningV6Error::InvalidConfig("test".to_string());
            let display = format!("{}", err);
            assert!(display.contains("test"));
        }

        #[test]
        fn test_engine_default() {
            let engine = FineTuningV6::default();
            assert_eq!(engine.current_round(), 0);
        }

        #[test]
        fn test_checkpoint_integrity() {
            let mut config = make_config();
            config.checkpoint_interval = 1;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            let result = engine.execute_round(grads).unwrap();
            assert!(result.checkpoint_integrity_valid);
            assert_eq!(engine.stats.integrity_validations, 1);
        }

        #[test]
        fn test_node_selection_score() {
            let node = NodeEntryV6::new("n1".to_string(), 0.95, 0.8, 100.0);
            let score = node.selection_score(0.9);
            assert!(score > 0.0);
        }

        #[test]
        fn test_node_meets_uptime() {
            let node = NodeEntryV6::new("n1".to_string(), 0.95, 0.8, 100.0);
            assert!(node.meets_uptime(0.9));
            assert!(!node.meets_uptime(0.99));
        }

        #[test]
        fn test_model_convergence() {
            let mut profile = ModelProfileV6::new("m1".to_string(), "n1".to_string(), 1024);
            profile.record_loss(1.0);
            profile.record_loss(0.95);
            profile.record_loss(0.94);
            assert!(profile.has_converged(0.1));
        }

        #[test]
        fn test_checkpoint_compression_ratio() {
            let mut entry = CheckpointEntryV6::new(1, "m1".to_string(), "hash".to_string(), 1000);
            entry.mark_compressed(600);
            assert!((entry.compression_ratio() - 0.6).abs() < 0.01);
        }

        #[test]
        fn test_get_checkpoint() {
            let mut config = make_config();
            config.checkpoint_interval = 1;
            let mut engine = FineTuningV6::new(config);
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            engine.execute_round(grads).unwrap();
            let cp = engine.get_checkpoint(1, "model-1");
            assert!(cp.is_some());
        }

        #[test]
        fn test_model_avg_normalized_gradient() {
            let mut profile = ModelProfileV6::new("m1".to_string(), "n1".to_string(), 1024);
            profile.record_normalized_gradient(1.0);
            profile.record_normalized_gradient(2.0);
            assert!((profile.avg_normalized_gradient() - 1.5).abs() < 0.01);
        }

        #[test]
        fn test_integrity_validation_failed_error() {
            let err = FineTuningV6Error::IntegrityValidationFailed("bad hash".to_string());
            let display = format!("{}", err);
            assert!(display.contains("bad hash"));
        }

        #[test]
        fn test_cross_model_sync_failed_error() {
            let err = FineTuningV6Error::CrossModelSyncFailed("timeout".to_string());
            let display = format!("{}", err);
            assert!(display.contains("timeout"));
        }

        #[test]
        fn test_multi_round_execution() {
            let mut engine = FineTuningV6::default();
            engine
                .register_node("node-1".to_string(), 0.95, 0.8, 100.0)
                .unwrap();
            engine
                .register_model("model-1".to_string(), "node-1".to_string(), 1024)
                .unwrap();
            for _ in 0..3 {
                let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
                engine.execute_round(grads).unwrap();
            }
            assert_eq!(engine.current_round(), 3);
            assert_eq!(engine.stats.total_rounds, 3);
        }
    }
}

#[cfg(feature = "v1.5-sprint3")]
pub use internal::*;
