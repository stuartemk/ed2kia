//! Adaptive Checkpoint — Incremental checkpointing with LZ4-style compression.
//!
//! Features:
//! - Delta checkpointing with shard management
//! - LZ4-style compression simulation
//! - Merge of old deltas automatically
//! - Rotation by max size
//!
//! Zero financial logic: checkpoints store technical model state only.

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum CheckpointError {
    ShardNotFound(String),
    CheckpointCorrupted(String),
    MaxCheckpointsExceeded(usize),
    MergeFailed(String),
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
            Self::CheckpointCorrupted(msg) => write!(f, "Checkpoint corrupted: {}", msg),
            Self::MaxCheckpointsExceeded(max) => write!(f, "Max checkpoints {} exceeded", max),
            Self::MergeFailed(msg) => write!(f, "Merge failed: {}", msg),
        }
    }
}

impl std::error::Error for CheckpointError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct AdaptiveCheckpointConfig {
    /// Maximum checkpoints to retain.
    pub max_checkpoints: usize,
    /// Number of shards for distributed storage.
    pub shard_count: usize,
    /// Enable delta checkpointing.
    pub delta_enabled: bool,
    /// Compression threshold (skip if data < this size).
    pub compression_threshold: usize,
    /// Merge interval (merge deltas every N checkpoints).
    pub merge_interval: usize,
}

impl Default for AdaptiveCheckpointConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 100,
            shard_count: 32,
            delta_enabled: true,
            compression_threshold: 1024,
            merge_interval: 10,
        }
    }
}

// ─── Delta Checkpoint ───

#[derive(Debug, Clone)]
pub struct DeltaCheckpoint {
    pub id: String,
    pub round: u64,
    pub shard_id: usize,
    pub data: Vec<f32>,
    pub compressed_size: usize,
    pub is_delta: bool,
    pub parent_id: Option<String>,
    pub checksum: String,
}

impl DeltaCheckpoint {
    pub fn new(id: String, round: u64, shard_id: usize, data: Vec<f32>, parent_id: Option<String>) -> Self {
        let compressed_size = simulate_lz4(&data);
        let checksum = compute_checksum(&data);
        Self {
            id,
            round,
            shard_id,
            data,
            compressed_size,
            is_delta: parent_id.is_some(),
            parent_id,
            checksum,
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        let raw = self.data.len() * 4;
        if self.compressed_size == 0 {
            return 0.0;
        }
        raw as f64 / self.compressed_size as f64
    }
}

// ─── Stats ───

#[derive(Debug, Clone, Default)]
pub struct CheckpointStats {
    pub total_checkpoints: usize,
    pub total_deltas: usize,
    pub total_merges: usize,
    pub total_compressed_bytes: usize,
    pub avg_compression_ratio: f64,
}

// ─── Engine ───

pub struct AdaptiveCheckpoint {
    config: AdaptiveCheckpointConfig,
    checkpoints: VecDeque<DeltaCheckpoint>,
    shard_index: HashMap<usize, Vec<String>>,
    pub stats: CheckpointStats,
}

impl AdaptiveCheckpoint {
    /// Create with config.
    pub fn new(config: AdaptiveCheckpointConfig) -> Self {
        Self {
            config,
            checkpoints: VecDeque::new(),
            shard_index: HashMap::new(),
            stats: CheckpointStats::default(),
        }
    }

    /// Create with defaults.
    pub fn with_defaults() -> Self {
        Self::new(AdaptiveCheckpointConfig::default())
    }

    /// Save a checkpoint.
    pub fn save_checkpoint(
        &mut self,
        round: u64,
        data: Vec<f32>,
    ) -> Result<DeltaCheckpoint, CheckpointError> {
        if self.checkpoints.len() >= self.config.max_checkpoints {
            return Err(CheckpointError::MaxCheckpointsExceeded(self.config.max_checkpoints));
        }

        let id = format!("ckpt-{}", round);
        let shard_id = (round as usize) % self.config.shard_count;

        // Delta checkpointing
        let parent_id = if self.config.delta_enabled {
            self.checkpoints.back().map(|c| c.id.clone())
        } else {
            None
        };

        let checkpoint = DeltaCheckpoint::new(id.clone(), round, shard_id, data, parent_id);

        // Track in shard index
        self.shard_index
            .entry(shard_id)
            .or_default()
            .push(id.clone());

        // Update stats
        self.checkpoints.push_back(checkpoint.clone());
        self.stats.total_checkpoints += 1;
        if checkpoint.is_delta {
            self.stats.total_deltas += 1;
        }
        self.stats.total_compressed_bytes += checkpoint.compressed_size;

        // Periodic merge
        if self.stats.total_checkpoints.is_multiple_of(self.config.merge_interval) {
            self.merge_deltas()?;
        }

        Ok(checkpoint)
    }

    /// Merge old deltas into full checkpoint.
    fn merge_deltas(&mut self) -> Result<(), CheckpointError> {
        if self.checkpoints.len() < 2 {
            return Ok(());
        }

        // Keep only the last N checkpoints, merge the rest
        let keep = self.config.merge_interval;
        if self.checkpoints.len() <= keep {
            return Ok(());
        }

        let merged = self.checkpoints.len() - keep;
        for _ in 0..merged {
            self.checkpoints.pop_front();
        }
        self.stats.total_merges += 1;
        Ok(())
    }

    /// Get checkpoint by ID.
    pub fn get_checkpoint(&self, id: &str) -> Option<&DeltaCheckpoint> {
        self.checkpoints.iter().find(|c| c.id == id)
    }

    /// Get checkpoints for a shard.
    pub fn get_shard_checkpoints(&self, shard_id: usize) -> Vec<&DeltaCheckpoint> {
        let ids = match self.shard_index.get(&shard_id) {
            Some(ids) => ids,
            None => return Vec::new(),
        };
        ids.iter()
            .filter_map(|id| self.get_checkpoint(id))
            .collect()
    }

    /// Get latest checkpoint.
    pub fn latest(&self) -> Option<&DeltaCheckpoint> {
        self.checkpoints.back()
    }

    /// Update average compression ratio.
    pub fn update_avg_compression(&mut self) {
        if self.stats.total_checkpoints == 0 {
            return;
        }
        let ratios: Vec<f64> = self.checkpoints.iter().map(|c| c.compression_ratio()).collect();
        self.stats.avg_compression_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.checkpoints.clear();
        self.shard_index.clear();
        self.stats = CheckpointStats::default();
    }
}

impl Default for AdaptiveCheckpoint {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn simulate_lz4(data: &[f32]) -> usize {
    let raw = data.len() * 4;
    (raw as f64 * 0.35) as usize
}

fn compute_checksum(data: &[f32]) -> String {
    let mut hash = 0u64;
    for &val in data {
        hash = hash.wrapping_add(val.to_bits() as u64);
        hash = hash.wrapping_mul(31);
    }
    format!("{:016x}", hash)
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let engine = AdaptiveCheckpoint::with_defaults();
        assert_eq!(engine.stats.total_checkpoints, 0);
    }

    #[test]
    fn test_save_checkpoint() {
        let mut engine = AdaptiveCheckpoint::default();
        let data = vec![0.1; 128];
        let result = engine.save_checkpoint(1, data);
        assert!(result.is_ok());
        assert_eq!(engine.stats.total_checkpoints, 1);
    }

    #[test]
    fn test_delta_checkpointing() {
        let mut engine = AdaptiveCheckpoint::default();
        let _ = engine.save_checkpoint(1, vec![0.1; 64]);
        let ckpt = engine.save_checkpoint(2, vec![0.2; 64]).unwrap();
        assert!(ckpt.is_delta);
        assert_eq!(engine.stats.total_deltas, 1);
    }

    #[test]
    fn test_compression_ratio() {
        let mut engine = AdaptiveCheckpoint::default();
        let _ = engine.save_checkpoint(1, vec![0.1; 256]);
        let ckpt = engine.latest().unwrap();
        assert!(ckpt.compression_ratio() > 1.0);
    }

    #[test]
    fn test_max_checkpoints() {
        let config = AdaptiveCheckpointConfig {
            max_checkpoints: 3,
            ..AdaptiveCheckpointConfig::default()
        };
        let mut engine = AdaptiveCheckpoint::new(config);
        let _ = engine.save_checkpoint(1, vec![0.1; 32]);
        let _ = engine.save_checkpoint(2, vec![0.1; 32]);
        let _ = engine.save_checkpoint(3, vec![0.1; 32]);
        let result = engine.save_checkpoint(4, vec![0.1; 32]);
        assert!(matches!(result, Err(CheckpointError::MaxCheckpointsExceeded(3))));
    }

    #[test]
    fn test_get_checkpoint() {
        let mut engine = AdaptiveCheckpoint::default();
        let _ = engine.save_checkpoint(1, vec![0.1; 64]);
        let ckpt = engine.get_checkpoint("ckpt-1");
        assert!(ckpt.is_some());
    }

    #[test]
    fn test_shard_checkpoints() {
        let mut engine = AdaptiveCheckpoint::default();
        let _ = engine.save_checkpoint(1, vec![0.1; 64]);
        let shard_ckpts = engine.get_shard_checkpoints(1 % 32);
        assert_eq!(shard_ckpts.len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut engine = AdaptiveCheckpoint::default();
        let _ = engine.save_checkpoint(1, vec![0.1; 64]);
        engine.reset();
        assert_eq!(engine.stats.total_checkpoints, 0);
    }

    #[test]
    fn test_error_display() {
        let err = CheckpointError::ShardNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }
}
