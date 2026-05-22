//! SAE Fine-Tuning v2 — Distributed fine-tuning engine with checkpointing and adaptive learning rates.
//!
//! Features:
//! - Incremental checkpointing with shard-based storage
//! - Adaptive learning rate scheduling with exponential backoff
//! - Fault tolerance with retry logic and fallback nodes
//! - Gradient compression for bandwidth optimization

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum FineTuningError {
    InvalidConfig(String),
    NodeUnavailable(String),
    CheckpointFailed(String),
    GradientMismatch(String),
    UptimeBelowThreshold { node_id: String, uptime: f64 },
}

impl fmt::Display for FineTuningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::NodeUnavailable(id) => write!(f, "Node unavailable: {}", id),
            Self::CheckpointFailed(msg) => write!(f, "Checkpoint failed: {}", msg),
            Self::GradientMismatch(msg) => write!(f, "Gradient mismatch: {}", msg),
            Self::UptimeBelowThreshold { node_id, uptime } => {
                write!(
                    f,
                    "Node {} uptime {:.1}% below 95% threshold",
                    node_id, uptime
                )
            }
        }
    }
}

impl std::error::Error for FineTuningError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct FineTuningV2Config {
    pub learning_rate: f64,
    pub compression_ratio: f32,
    pub batch_size: usize,
    pub adaptive_lr: bool,
    pub max_retries: u32,
    pub checkpoint_interval: u64,
    pub min_node_uptime: f64,
}

impl Default for FineTuningV2Config {
    fn default() -> Self {
        Self {
            learning_rate: 1e-4,
            compression_ratio: 4.0,
            batch_size: 32,
            adaptive_lr: true,
            max_retries: 3,
            checkpoint_interval: 100,
            min_node_uptime: 0.95,
        }
    }
}

// ─── Checkpoint ───

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: String,
    pub round: u64,
    pub shard_data: Vec<Vec<f32>>,
    pub timestamp_ms: u64,
    pub checksum: String,
}

impl Checkpoint {
    pub fn new(id: String, round: u64, shard_data: Vec<Vec<f32>>) -> Self {
        let checksum = compute_checksum(&shard_data);
        Self {
            id,
            round,
            shard_data,
            timestamp_ms: current_timestamp_ms(),
            checksum,
        }
    }

    pub fn verify(&self) -> bool {
        compute_checksum(&self.shard_data) == self.checksum
    }
}

// ─── Training Result ───

#[derive(Debug, Clone)]
pub struct TrainingResult {
    pub round: u64,
    pub loss: f64,
    pub learning_rate: f64,
    pub checkpoint_id: Option<String>,
    pub elapsed_ms: u64,
}

// ─── Node State ───

#[derive(Debug, Clone)]
pub struct NodeState {
    pub node_id: String,
    pub uptime: f64,
    pub last_gradient_round: u64,
    pub is_fallback: bool,
}

impl NodeState {
    pub fn new(node_id: String, uptime: f64) -> Self {
        Self {
            node_id,
            uptime,
            last_gradient_round: 0,
            is_fallback: false,
        }
    }

    pub fn meets_uptime_requirement(&self, min_uptime: f64) -> bool {
        self.uptime >= min_uptime
    }
}

// ─── Stats ───

#[derive(Debug, Clone)]
pub struct FineTuningStats {
    pub total_rounds: u64,
    pub total_checkpoints: u64,
    pub avg_loss: f64,
    pub avg_elapsed_ms: f64,
    pub retries: u64,
    pub fallback_activations: u64,
}

impl Default for FineTuningStats {
    fn default() -> Self {
        Self {
            total_rounds: 0,
            total_checkpoints: 0,
            avg_loss: 0.0,
            avg_elapsed_ms: 0.0,
            retries: 0,
            fallback_activations: 0,
        }
    }
}

// ─── Engine ───

pub struct FineTuningV2 {
    config: FineTuningV2Config,
    nodes: HashMap<String, NodeState>,
    checkpoints: VecDeque<Checkpoint>,
    current_round: u64,
    current_lr: f64,
    stats: FineTuningStats,
    loss_history: VecDeque<f64>,
}

impl FineTuningV2 {
    pub fn new(config: FineTuningV2Config) -> Self {
        Self {
            current_lr: config.learning_rate,
            config,
            nodes: HashMap::new(),
            checkpoints: VecDeque::new(),
            current_round: 0,
            stats: FineTuningStats::default(),
            loss_history: VecDeque::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(FineTuningV2Config::default())
    }

    // ─── Node Management ───

    pub fn register_node(&mut self, node_id: String, uptime: f64) {
        self.nodes
            .insert(node_id.clone(), NodeState::new(node_id, uptime));
    }

    pub fn update_node_uptime(&mut self, node_id: &str, uptime: f64) -> Option<NodeState> {
        self.nodes.get_mut(node_id).map(|node| {
            node.uptime = uptime.clamp(0.0, 1.0);
            node.is_fallback = false;
            node.clone()
        })
    }

    pub fn get_active_nodes(&self) -> Vec<&NodeState> {
        self.nodes
            .values()
            .filter(|n| n.meets_uptime_requirement(self.config.min_node_uptime))
            .collect()
    }

    pub fn get_fallback_nodes(&self) -> Vec<&NodeState> {
        self.nodes
            .values()
            .filter(|n| !n.meets_uptime_requirement(self.config.min_node_uptime) && !n.is_fallback)
            .collect()
    }

    // ─── Training ───

    pub fn train_step(&mut self, gradients: &[f32]) -> Result<TrainingResult, FineTuningError> {
        let start_ms = current_timestamp_ms();

        // Validate active nodes
        let active = self.get_active_nodes();
        if active.is_empty() {
            // Attempt fallback
            let fallback = self.get_fallback_nodes();
            if fallback.is_empty() {
                return Err(FineTuningError::NodeUnavailable(
                    "No active or fallback nodes available".to_string(),
                ));
            }
            self.stats.fallback_activations += 1;
        }

        // Compress gradients
        let compressed = self.compress_gradients(gradients);

        // Simulate training step (loss computation)
        self.current_round += 1;
        let loss = self.compute_loss(&compressed);

        // Update stats
        self.stats.total_rounds += 1;
        self.stats.avg_loss = (self.stats.avg_loss * (self.stats.total_rounds - 1) as f64 + loss)
            / self.stats.total_rounds as f64;

        // Adaptive LR
        if self.config.adaptive_lr {
            self.adjust_learning_rate(loss);
        }

        // Checkpoint
        let checkpoint_id = if self
            .current_round
            .is_multiple_of(self.config.checkpoint_interval)
        {
            let cp = self.create_checkpoint(&compressed)?;
            self.stats.total_checkpoints += 1;
            Some(cp.id)
        } else {
            None
        };

        let elapsed_ms = current_timestamp_ms() - start_ms;
        self.stats.avg_elapsed_ms =
            (self.stats.avg_elapsed_ms * (self.stats.total_rounds - 1) as f64 + elapsed_ms as f64)
                / self.stats.total_rounds as f64;

        self.loss_history.push_back(loss);
        if self.loss_history.len() > 100 {
            self.loss_history.pop_front();
        }

        Ok(TrainingResult {
            round: self.current_round,
            loss,
            learning_rate: self.current_lr,
            checkpoint_id,
            elapsed_ms,
        })
    }

    // ─── Learning Rate ───

    pub fn adjust_learning_rate(&mut self, _metric: f64) {
        if self.loss_history.len() < 5 {
            return;
        }

        let recent: Vec<f64> = self.loss_history.iter().rev().take(5).cloned().collect();
        let improving = recent.windows(2).filter(|w| w[1] < w[0]).count() as f64
            / recent.windows(2).count() as f64;

        if improving >= 0.6 {
            // Improving: increase LR slightly
            self.current_lr *= 1.05;
        } else if improving <= 0.2 {
            // Stalled: decrease LR with exponential backoff
            self.current_lr *= 0.5;
        }

        // Clamp
        self.current_lr = self.current_lr.clamp(1e-8, 1.0);
    }

    // ─── Checkpointing ───

    fn create_checkpoint(&mut self, data: &[f32]) -> Result<Checkpoint, FineTuningError> {
        let shard_size = (data.len() as f32 / self.config.compression_ratio) as usize;
        let shard_size = shard_size.max(1);
        let shards: Vec<Vec<f32>> = data.chunks(shard_size).map(|c| c.to_vec()).collect();

        let cp = Checkpoint::new(
            format!("cp-{}", self.current_round),
            self.current_round,
            shards,
        );

        if !cp.verify() {
            return Err(FineTuningError::CheckpointFailed(
                "Checksum verification failed".to_string(),
            ));
        }

        self.checkpoints.push_back(cp);
        if self.checkpoints.len() > 50 {
            self.checkpoints.pop_front();
        }

        Ok(self.checkpoints.back().cloned().unwrap())
    }

    pub fn get_latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.back()
    }

    pub fn restore_checkpoint(&self, round: u64) -> Option<&Checkpoint> {
        self.checkpoints.iter().rev().find(|cp| cp.round <= round)
    }

    // ─── Gradient Compression ───

    fn compress_gradients(&self, gradients: &[f32]) -> Vec<f32> {
        let target_len = (gradients.len() as f32 / self.config.compression_ratio) as usize;
        let target_len = target_len.max(1);

        if gradients.len() <= target_len {
            return gradients.to_vec();
        }

        // Top-k magnitude compression
        let mut indexed: Vec<(usize, f32)> =
            gradients.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.sort_by(|a, b| {
            b.1.abs()
                .partial_cmp(&a.1.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        indexed.truncate(target_len);
        indexed.sort_by_key(|(i, _)| *i);
        indexed.into_iter().map(|(_, v)| v).collect()
    }

    fn compute_loss(&self, gradients: &[f32]) -> f64 {
        if gradients.is_empty() {
            return 0.0;
        }
        let sum: f64 = gradients.iter().map(|g| (*g as f64) * (*g as f64)).sum();
        (sum / gradients.len() as f64).sqrt()
    }

    // ─── Accessors ───

    pub fn get_stats(&self) -> &FineTuningStats {
        &self.stats
    }

    pub fn get_config(&self) -> &FineTuningV2Config {
        &self.config
    }

    pub fn get_current_round(&self) -> u64 {
        self.current_round
    }

    pub fn get_current_lr(&self) -> f64 {
        self.current_lr
    }

    pub fn reset_stats(&mut self) {
        self.stats = FineTuningStats::default();
    }
}

impl Default for FineTuningV2 {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_checksum(data: &[Vec<f32>]) -> String {
    let mut hash: u64 = 0;
    for shard in data {
        for val in shard {
            hash = hash.wrapping_add((*val as i32) as u64);
            hash = hash.wrapping_mul(31);
        }
    }
    format!("{:016x}", hash)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let engine = FineTuningV2::with_defaults();
        assert_eq!(engine.get_current_round(), 0);
        assert_eq!(engine.get_stats().total_rounds, 0);
    }

    #[test]
    fn test_creation_with_config() {
        let config = FineTuningV2Config {
            learning_rate: 5e-4,
            compression_ratio: 8.0,
            batch_size: 64,
            adaptive_lr: false,
            max_retries: 5,
            checkpoint_interval: 50,
            min_node_uptime: 0.90,
        };
        let engine = FineTuningV2::new(config);
        assert_eq!(engine.get_current_lr(), 5e-4);
    }

    #[test]
    fn test_register_node() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("node-1".to_string(), 0.99);
        let active = engine.get_active_nodes();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_node_uptime_filtering() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("good".to_string(), 0.99);
        engine.register_node("bad".to_string(), 0.80);
        assert_eq!(engine.get_active_nodes().len(), 1);
        assert_eq!(engine.get_fallback_nodes().len(), 1);
    }

    #[test]
    fn test_update_node_uptime() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("node-1".to_string(), 0.80);
        engine.update_node_uptime("node-1", 0.99);
        assert_eq!(engine.get_active_nodes().len(), 1);
    }

    #[test]
    fn test_train_step_no_nodes() {
        let mut engine = FineTuningV2::with_defaults();
        let result = engine.train_step(&[1.0, 2.0, 3.0]);
        assert!(result.is_err());
        match result.unwrap_err() {
            FineTuningError::NodeUnavailable(_) => {}
            e => panic!("Expected NodeUnavailable, got: {}", e),
        }
    }

    #[test]
    fn test_train_step_with_node() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("node-1".to_string(), 0.99);
        let gradients = (0..100).map(|i| i as f32).collect::<Vec<_>>();
        let result = engine.train_step(&gradients).unwrap();
        assert_eq!(result.round, 1);
        assert!(result.loss > 0.0);
    }

    #[test]
    fn test_checkpoint_creation() {
        let mut engine = FineTuningV2::new(FineTuningV2Config {
            checkpoint_interval: 1,
            ..Default::default()
        });
        engine.register_node("node-1".to_string(), 0.99);
        let result = engine.train_step(&[1.0, 2.0, 3.0]).unwrap();
        assert!(result.checkpoint_id.is_some());
    }

    #[test]
    fn test_checkpoint_verification() {
        let cp = Checkpoint::new("test".to_string(), 1, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert!(cp.verify());
    }

    #[test]
    fn test_adaptive_lr_adjustment() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("node-1".to_string(), 0.99);
        let initial_lr = engine.get_current_lr();
        // Feed decreasing losses
        for loss in [0.5, 0.4, 0.35, 0.3, 0.25] {
            engine.train_step(&[loss as f32]).unwrap();
        }
        assert!(engine.get_current_lr() >= initial_lr);
    }

    #[test]
    fn test_lr_clamping() {
        let mut engine = FineTuningV2::new(FineTuningV2Config {
            learning_rate: 0.5,
            ..Default::default()
        });
        engine.register_node("node-1".to_string(), 0.99);
        // Feed steadily decreasing losses to trigger LR increase
        for loss in [1.0_f32, 0.8, 0.6, 0.4, 0.2] {
            let _ = engine.train_step(&[loss]).unwrap();
        }
        // LR should not grow unbounded
        assert!(engine.get_current_lr() <= 1.0);
    }

    #[test]
    fn test_gradient_compression() {
        let engine = FineTuningV2::new(FineTuningV2Config {
            compression_ratio: 4.0,
            ..Default::default()
        });
        let gradients: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let compressed = engine.compress_gradients(&gradients);
        assert!(compressed.len() <= 25);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("node-1".to_string(), 0.99);
        for _ in 0..10 {
            engine.train_step(&[1.0, 2.0, 3.0]).unwrap();
        }
        assert_eq!(engine.get_stats().total_rounds, 10);
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("node-1".to_string(), 0.99);
        engine.train_step(&[1.0]).unwrap();
        engine.reset_stats();
        assert_eq!(engine.get_stats().total_rounds, 0);
    }

    #[test]
    fn test_restore_checkpoint() {
        let mut engine = FineTuningV2::new(FineTuningV2Config {
            checkpoint_interval: 1,
            ..Default::default()
        });
        engine.register_node("node-1".to_string(), 0.99);
        for _ in 0..5 {
            engine.train_step(&[1.0]).unwrap();
        }
        let cp = engine.restore_checkpoint(3);
        assert!(cp.is_some());
        assert_eq!(cp.unwrap().round, 3);
    }

    #[test]
    fn test_fallback_activation() {
        let mut engine = FineTuningV2::with_defaults();
        engine.register_node("weak".to_string(), 0.80);
        let result = engine.train_step(&[1.0, 2.0]);
        assert!(result.is_ok());
        assert_eq!(engine.get_stats().fallback_activations, 1);
    }

    #[test]
    fn test_node_state_meets_uptime() {
        let node = NodeState::new("n".to_string(), 0.99);
        assert!(node.meets_uptime_requirement(0.95));
        assert!(!node.meets_uptime_requirement(0.995));
    }

    #[test]
    fn test_error_display() {
        let e = FineTuningError::InvalidConfig("bad".to_string());
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn test_config_default() {
        let c = FineTuningV2Config::default();
        assert_eq!(c.min_node_uptime, 0.95);
        assert!(c.adaptive_lr);
    }

    #[test]
    fn test_stats_default() {
        let s = FineTuningStats::default();
        assert_eq!(s.total_rounds, 0);
    }

    #[test]
    fn test_default_impl() {
        let _ = FineTuningV2::default();
    }
}
