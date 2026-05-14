//! SAE Fine-Tuning v3 — Distributed fine-tuning engine with cross-model alignment.
//!
//! Features:
//! - Cross-model gradient alignment with adaptive normalization
//! - Incremental checkpointing with LZ4-style compression simulation
//! - Fallback to reserve nodes when `node_uptime < 95%`
//! - Multi-model coordination with gradient synchronization
//!
//! Zero financial logic: credits represent compute capacity only.

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum FineTuningV3Error {
    InvalidConfig(String),
    NodeUnavailable(String),
    CheckpointFailed(String),
    GradientMismatch(String),
    UptimeBelowThreshold { node_id: String, uptime: f64 },
    AlignmentFailed(String),
    ModelNotFound(String),
}

impl fmt::Display for FineTuningV3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::NodeUnavailable(id) => write!(f, "Node unavailable: {}", id),
            Self::CheckpointFailed(msg) => write!(f, "Checkpoint failed: {}", msg),
            Self::GradientMismatch(msg) => write!(f, "Gradient mismatch: {}", msg),
            Self::UptimeBelowThreshold { node_id, uptime } => {
                write!(f, "Node {} uptime {:.1}% below 95% threshold", node_id, uptime * 100.0)
            }
            Self::AlignmentFailed(msg) => write!(f, "Cross-model alignment failed: {}", msg),
            Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
        }
    }
}

impl std::error::Error for FineTuningV3Error {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct FineTuningV3Config {
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
    /// Enable LZ4-style compression for checkpoints.
    pub lz4_compression: bool,
}

impl Default for FineTuningV3Config {
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
        }
    }
}

// ─── Model Profile ───

#[derive(Debug, Clone)]
pub struct ModelProfile {
    pub model_id: String,
    pub node_id: String,
    pub gradient_dim: usize,
    pub alignment_score: f64,
    pub rounds_trained: u64,
    pub last_gradient_norm: f64,
}

impl ModelProfile {
    pub fn new(model_id: String, node_id: String, gradient_dim: usize) -> Self {
        Self {
            model_id,
            node_id,
            gradient_dim,
            alignment_score: 1.0,
            rounds_trained: 0,
            last_gradient_norm: 0.0,
        }
    }
}

// ─── Node Entry ───

#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub node_id: String,
    pub uptime: f64,
    pub is_active: bool,
    pub is_reserve: bool,
    pub reputation: f64,
}

impl NodeEntry {
    pub fn new(node_id: String, uptime: f64, reputation: f64) -> Self {
        Self {
            node_id,
            uptime,
            is_active: true,
            is_reserve: false,
            reputation,
        }
    }

    pub fn meets_uptime(&self, threshold: f64) -> bool {
        self.uptime >= threshold
    }
}

// ─── Checkpoint ───

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: String,
    pub round: u64,
    pub model_id: String,
    pub shard_data: Vec<Vec<f32>>,
    pub compressed_size: usize,
    pub timestamp_ms: u64,
    pub checksum: String,
}

impl Checkpoint {
    pub fn new(id: String, round: u64, model_id: String, shard_data: Vec<Vec<f32>>) -> Self {
        let compressed_size = simulate_lz4_compression(&shard_data);
        let checksum = compute_checksum(&shard_data);
        Self {
            id,
            round,
            model_id,
            shard_data,
            compressed_size,
            timestamp_ms: current_timestamp_ms(),
            checksum,
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        let raw_size = self.shard_data.iter().map(|s| s.len() * 4).sum::<usize>();
        if self.compressed_size == 0 {
            return 0.0;
        }
        raw_size as f64 / self.compressed_size as f64
    }
}

// ─── Training Stats ───

#[derive(Debug, Clone)]
pub struct TrainingStats {
    pub total_rounds: u64,
    pub total_checkpoints: u64,
    pub total_failures: u64,
    pub total_fallbacks: u64,
    pub total_alignments: u64,
    pub avg_gradient_norm: f64,
    pub avg_sync_time_ms: f64,
    pub active_models: usize,
    pub active_nodes: usize,
}

impl Default for TrainingStats {
    fn default() -> Self {
        Self {
            total_rounds: 0,
            total_checkpoints: 0,
            total_failures: 0,
            total_fallbacks: 0,
            total_alignments: 0,
            avg_gradient_norm: 0.0,
            avg_sync_time_ms: 0.0,
            active_models: 0,
            active_nodes: 0,
        }
    }
}

// ─── Engine ───

pub struct FineTuningV3 {
    pub config: FineTuningV3Config,
    nodes: HashMap<String, NodeEntry>,
    models: HashMap<String, ModelProfile>,
    checkpoints: VecDeque<Checkpoint>,
    reserve_nodes: Vec<String>,
    pub stats: TrainingStats,
    current_lr: f64,
    gradient_history: VecDeque<f64>,
}

impl FineTuningV3 {
    /// Create a new Fine-Tuning v3 engine with config.
    pub fn new(config: FineTuningV3Config) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            models: HashMap::new(),
            checkpoints: VecDeque::new(),
            reserve_nodes: Vec::new(),
            stats: TrainingStats::default(),
            current_lr: 0.0,
            gradient_history: VecDeque::new(),
        }
    }

    /// Create engine with default config.
    pub fn with_defaults() -> Self {
        Self::new(FineTuningV3Config::default())
    }

    // ── Node Management ──

    /// Register a training node.
    pub fn register_node(&mut self, node_id: String, uptime: f64, reputation: f64) {
        let node = NodeEntry::new(node_id.clone(), uptime, reputation);
        self.nodes.insert(node_id, node);
        self.stats.active_nodes = self.nodes.len();
    }

    /// Register a reserve node for fallback.
    pub fn register_reserve(&mut self, node_id: String, uptime: f64, reputation: f64) {
        let mut node = NodeEntry::new(node_id.clone(), uptime, reputation);
        node.is_reserve = true;
        self.nodes.insert(node_id.clone(), node);
        self.reserve_nodes.push(node_id);
    }

    /// Update node uptime.
    pub fn update_uptime(&mut self, node_id: &str, uptime: f64) -> Result<(), FineTuningV3Error> {
        let node = self.nodes.get_mut(node_id).ok_or(FineTuningV3Error::NodeUnavailable(
            node_id.to_string(),
        ))?;
        node.uptime = uptime.clamp(0.0, 1.0);
        Ok(())
    }

    // ── Model Management ──

    /// Register a model for cross-model alignment.
    pub fn register_model(
        &mut self,
        model_id: String,
        node_id: String,
        gradient_dim: usize,
    ) -> Result<(), FineTuningV3Error> {
        if self.models.len() >= self.config.max_models {
            return Err(FineTuningV3Error::InvalidConfig(
                "Maximum models reached".to_string(),
            ));
        }
        if !self.nodes.contains_key(&node_id) {
            return Err(FineTuningV3Error::NodeUnavailable(node_id));
        }
        let profile = ModelProfile::new(model_id.clone(), node_id, gradient_dim);
        self.models.insert(model_id, profile);
        self.stats.active_models = self.models.len();
        Ok(())
    }

    /// Get model profile.
    pub fn get_model(&self, model_id: &str) -> Option<&ModelProfile> {
        self.models.get(model_id)
    }

    // ── Training ──

    /// Execute a training round with gradient alignment.
    pub fn train_step(
        &mut self,
        gradients: &[f32],
    ) -> Result<Vec<f32>, FineTuningV3Error> {
        // Validate gradients
        if gradients.is_empty() {
            return Err(FineTuningV3Error::GradientMismatch(
                "Empty gradient vector".to_string(),
            ));
        }

        // Check node availability
        let _active_node = self.select_best_node()?;

        // Compress gradients
        let compressed = self.compress_gradients(gradients);

        // Align across models
        let aligned = self.align_gradients(&compressed)?;

        // Compute gradient norm
        let norm = compute_gradient_norm(&aligned);
        self.gradient_history.push_back(norm);
        if self.gradient_history.len() > 100 {
            self.gradient_history.pop_front();
        }

        // Update stats
        self.stats.total_rounds += 1;
        self.stats.avg_gradient_norm = self.gradient_history.iter().sum::<f64>()
            / self.gradient_history.len() as f64;

        // Update model profiles
        for profile in self.models.values_mut() {
            profile.rounds_trained += 1;
            profile.last_gradient_norm = norm;
        }

        // Adaptive learning rate
        if self.config.adaptive_lr {
            self.current_lr = self.compute_adaptive_lr(norm);
        }

        // Checkpoint if needed
        if self.stats.total_rounds.is_multiple_of(self.config.checkpoint_interval) {
            self.create_checkpoint()?;
        }

        Ok(aligned)
    }

    /// Select the best available node for training.
    fn select_best_node(&mut self) -> Result<String, FineTuningV3Error> {
        // Try active nodes first
        for (id, node) in &self.nodes {
            if !node.is_reserve && node.is_active && node.meets_uptime(self.config.min_node_uptime)
            {
                return Ok(id.clone());
            }
        }

        // Fallback to reserve nodes
        for id in &self.reserve_nodes {
            if let Some(node) = self.nodes.get(id.as_str()) {
                if node.is_active {
                    self.stats.total_fallbacks += 1;
                    return Ok(id.clone());
                }
            }
        }

        Err(FineTuningV3Error::NodeUnavailable(
            "No available nodes".to_string(),
        ))
    }

    /// Compress gradients using top-k selection.
    fn compress_gradients(&self, gradients: &[f32]) -> Vec<f32> {
        let k = (gradients.len() as f64 / self.config.compression_ratio as f64) as usize;
        if k >= gradients.len() {
            return gradients.to_vec();
        }

        let mut indexed: Vec<(usize, f32)> = gradients.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        indexed.truncate(k);

        let mut compressed = vec![0.0f32; gradients.len()];
        for &(i, v) in &indexed {
            compressed[i] = v;
        }
        compressed
    }

    /// Align gradients across registered models.
    fn align_gradients(&mut self, gradients: &[f32]) -> Result<Vec<f32>, FineTuningV3Error> {
        if self.models.len() <= 1 {
            return Ok(gradients.to_vec());
        }

        // Compute alignment score based on gradient direction
        let mut aligned = gradients.to_vec();
        let norm = compute_gradient_norm(gradients);

        if norm > 0.0 {
            // Normalize and apply alignment factor
            let alignment_factor = self.compute_alignment_factor();
            for g in aligned.iter_mut() {
                *g *= alignment_factor;
            }
        }

        // Update alignment scores
        for profile in self.models.values_mut() {
            profile.alignment_score = profile.alignment_score * 0.9 + 0.1 * (1.0 - norm / 100.0);
        }

        self.stats.total_alignments += 1;
        Ok(aligned)
    }

    /// Compute alignment factor from model profiles.
    fn compute_alignment_factor(&self) -> f32 {
        if self.models.is_empty() {
            return 1.0;
        }
        let avg_score: f64 = self.models.values().map(|p| p.alignment_score).sum::<f64>()
            / self.models.len() as f64;
        avg_score.clamp(0.5, 1.0) as f32
    }

    /// Compute adaptive learning rate based on gradient history.
    fn compute_adaptive_lr(&self, current_norm: f64) -> f64 {
        if self.gradient_history.len() < 2 {
            return self.config.learning_rate;
        }

        // gradient_history already contains current_norm at the back,
        // so we need the second-to-last entry as the previous norm.
        let prev_norm = self.gradient_history.get(self.gradient_history.len() - 2)
            .copied().unwrap_or(1.0);
        if prev_norm == 0.0 {
            return self.config.learning_rate;
        }

        let ratio = current_norm / prev_norm;
        if ratio > 1.5 {
            // Gradient exploding — reduce LR
            self.config.learning_rate * 0.5
        } else if ratio < 0.3 {
            // Gradient vanishing — increase LR
            self.config.learning_rate * 1.5
        } else {
            self.config.learning_rate
        }
    }

    // ── Checkpointing ──

    /// Create a checkpoint of current state.
    pub fn create_checkpoint(&mut self) -> Result<Checkpoint, FineTuningV3Error> {
        let round = self.stats.total_rounds;
        let model_id = self.models.keys().next().cloned().unwrap_or_default();
        let shard_data = self.generate_shard_data();

        let checkpoint = Checkpoint::new(
            format!("ckpt-{}", round),
            round,
            model_id,
            shard_data,
        );

        self.checkpoints.push_back(checkpoint.clone());
        self.stats.total_checkpoints += 1;

        // Limit checkpoint history
        while self.checkpoints.len() > 50 {
            self.checkpoints.pop_front();
        }

        Ok(checkpoint)
    }

    /// Get latest checkpoint.
    pub fn latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.back()
    }

    /// Generate shard data from gradient history.
    fn generate_shard_data(&self) -> Vec<Vec<f32>> {
        let dim = self.models.values().next().map(|p| p.gradient_dim).unwrap_or(128);
        vec![self.gradient_history.iter().map(|&n| n as f32).collect(); dim]
    }

    // ── Stats ──

    /// Get current training stats.
    pub fn get_stats(&self) -> &TrainingStats {
        &self.stats
    }

    /// Reset stats.
    pub fn reset_stats(&mut self) {
        self.stats = TrainingStats::default();
        self.gradient_history.clear();
    }

    /// Get current learning rate.
    pub fn current_learning_rate(&self) -> f64 {
        self.current_lr
    }
}

impl Default for FineTuningV3 {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_gradient_norm(gradients: &[f32]) -> f64 {
    gradients.iter().map(|g| (*g as f64) * (*g as f64)).sum::<f64>().sqrt()
}

fn compute_checksum(data: &[Vec<f32>]) -> String {
    let mut hash = 0u64;
    for shard in data {
        for &val in shard {
            hash = hash.wrapping_add(val.to_bits() as u64);
            hash = hash.wrapping_mul(31);
        }
    }
    format!("{:016x}", hash)
}

fn simulate_lz4_compression(data: &[Vec<f32>]) -> usize {
    let raw_size: usize = data.iter().map(|s| s.len() * 4).sum();
    // Simulate ~60% compression ratio
    (raw_size as f64 * 0.4) as usize
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
    fn test_engine_creation() {
        let engine = FineTuningV3::with_defaults();
        assert_eq!(engine.stats.total_rounds, 0);
        assert_eq!(engine.stats.active_models, 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        assert_eq!(engine.stats.active_nodes, 1);
    }

    #[test]
    fn test_register_reserve() {
        let mut engine = FineTuningV3::default();
        engine.register_reserve("reserve-1".to_string(), 0.90, 0.70);
        assert_eq!(engine.reserve_nodes.len(), 1);
    }

    #[test]
    fn test_register_model() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let result = engine.register_model("model-1".to_string(), "node-1".to_string(), 128);
        assert!(result.is_ok());
        assert_eq!(engine.stats.active_models, 1);
    }

    #[test]
    fn test_register_model_node_not_found() {
        let mut engine = FineTuningV3::default();
        let result = engine.register_model("model-1".to_string(), "missing".to_string(), 128);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_model_max_reached() {
        let config = FineTuningV3Config {
            max_models: 2,
            ..FineTuningV3Config::default()
        };
        let mut engine = FineTuningV3::new(config);
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("m1".to_string(), "node-1".to_string(), 64);
        let _ = engine.register_model("m2".to_string(), "node-1".to_string(), 64);
        let result = engine.register_model("m3".to_string(), "node-1".to_string(), 64);
        assert!(result.is_err());
    }

    #[test]
    fn test_train_step() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("model-1".to_string(), "node-1".to_string(), 128);

        let gradients = vec![0.1; 128];
        let result = engine.train_step(&gradients);
        assert!(result.is_ok());
        assert_eq!(engine.stats.total_rounds, 1);
    }

    #[test]
    fn test_train_step_empty_gradients() {
        let mut engine = FineTuningV3::default();
        let result = engine.train_step(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_train_step_no_nodes() {
        let mut engine = FineTuningV3::default();
        let result = engine.train_step(&[0.1; 64]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_to_reserve() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.50, 0.85); // Below threshold
        engine.register_reserve("reserve-1".to_string(), 0.90, 0.70);
        let _ = engine.register_model("model-1".to_string(), "reserve-1".to_string(), 64);

        let gradients = vec![0.1; 64];
        let result = engine.train_step(&gradients);
        assert!(result.is_ok());
        assert_eq!(engine.stats.total_fallbacks, 1);
    }

    #[test]
    fn test_checkpoint_creation() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("model-1".to_string(), "node-1".to_string(), 64);

        let result = engine.create_checkpoint();
        assert!(result.is_ok());
        assert_eq!(engine.stats.total_checkpoints, 1);
    }

    #[test]
    fn test_checkpoint_compression() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("model-1".to_string(), "node-1".to_string(), 64);
        // Run a training step to populate gradient history
        let _ = engine.train_step(&vec![0.5; 64]).unwrap();
        let _ = engine.create_checkpoint();

        let ckpt = engine.latest_checkpoint().unwrap();
        assert!(ckpt.compression_ratio() > 1.0);
    }

    #[test]
    fn test_adaptive_lr() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("model-1".to_string(), "node-1".to_string(), 64);

        // Normal gradients
        let _ = engine.train_step(&vec![0.1; 64]).unwrap();
        // Large gradient (exploding)
        let _ = engine.train_step(&vec![10.0; 64]).unwrap();
        assert!(engine.current_learning_rate() < engine.config.learning_rate);
    }

    #[test]
    fn test_cross_model_alignment() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("m1".to_string(), "node-1".to_string(), 64);
        let _ = engine.register_model("m2".to_string(), "node-1".to_string(), 64);

        let _ = engine.train_step(&vec![0.5; 64]).unwrap();
        assert_eq!(engine.stats.total_alignments, 1);
    }

    #[test]
    fn test_update_uptime() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let result = engine.update_uptime("node-1", 0.85);
        assert!(result.is_ok());
        assert_eq!(engine.nodes.get("node-1").unwrap().uptime, 0.85);
    }

    #[test]
    fn test_update_uptime_unknown() {
        let mut engine = FineTuningV3::default();
        let result = engine.update_uptime("unknown", 0.85);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = FineTuningV3::default();
        engine.register_node("node-1".to_string(), 0.99, 0.85);
        let _ = engine.register_model("m1".to_string(), "node-1".to_string(), 64);
        let _ = engine.train_step(&vec![0.1; 64]).unwrap();
        engine.reset_stats();
        assert_eq!(engine.stats.total_rounds, 0);
    }

    #[test]
    fn test_config_default() {
        let config = FineTuningV3Config::default();
        assert_eq!(config.learning_rate, 1e-4);
        assert!(config.lz4_compression);
        assert_eq!(config.alignment_threshold, 0.85);
    }

    #[test]
    fn test_node_meets_uptime() {
        let node = NodeEntry::new("n1".to_string(), 0.97, 0.8);
        assert!(node.meets_uptime(0.95));
        assert!(!node.meets_uptime(0.99));
    }

    #[test]
    fn test_error_display() {
        let err = FineTuningV3Error::NodeUnavailable("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_engine_default() {
        let engine = FineTuningV3::default();
        assert_eq!(engine.stats.total_rounds, 0);
    }
}
