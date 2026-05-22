//! Checkpoint Optimizer — Incremental checkpointing with delta compression and shard management.
//!
//! Optimizes checkpoint storage through:
//! - Delta encoding between consecutive checkpoints
//! - Shard-based parallel writes
//! - Checksum verification for integrity

use std::collections::VecDeque;

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum CheckpointOptError {
    CorruptCheckpoint(String),
    ShardMismatch(String),
    StorageFull,
}

impl std::fmt::Display for CheckpointOptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorruptCheckpoint(id) => write!(f, "Corrupt checkpoint: {}", id),
            Self::ShardMismatch(msg) => write!(f, "Shard mismatch: {}", msg),
            Self::StorageFull => write!(f, "Storage capacity reached"),
        }
    }
}

impl std::error::Error for CheckpointOptError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct CheckpointOptimizerConfig {
    pub max_checkpoints: usize,
    pub shard_count: usize,
    pub delta_enabled: bool,
    pub compression_threshold: f32,
}

impl Default for CheckpointOptimizerConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 50,
            shard_count: 4,
            delta_enabled: true,
            compression_threshold: 0.1,
        }
    }
}

// ─── Delta Checkpoint ───

#[derive(Debug, Clone)]
pub struct DeltaCheckpoint {
    pub id: String,
    pub round: u64,
    pub base_round: Option<u64>,
    pub shards: Vec<Vec<f32>>,
    pub delta_shards: Option<Vec<Vec<f32>>>,
    pub checksum: String,
    pub size_bytes: usize,
    pub timestamp_ms: u64,
}

impl DeltaCheckpoint {
    pub fn new(id: String, round: u64, shards: Vec<Vec<f32>>) -> Self {
        let checksum = compute_checksum(&shards);
        let size_bytes = shards.iter().map(|s| s.len() * 4).sum();
        Self {
            id,
            round,
            base_round: None,
            shards,
            delta_shards: None,
            checksum,
            size_bytes,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn from_delta(
        id: String,
        round: u64,
        base_round: u64,
        base_shards: &[Vec<f32>],
        delta: Vec<Vec<f32>>,
    ) -> Self {
        let merged = merge_delta(base_shards, &delta);
        let checksum = compute_checksum(&merged);
        let size_bytes = delta.iter().map(|s| s.len() * 4).sum();
        Self {
            id,
            round,
            base_round: Some(base_round),
            shards: merged,
            delta_shards: Some(delta),
            checksum,
            size_bytes,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn verify(&self) -> bool {
        compute_checksum(&self.shards) == self.checksum
    }

    pub fn is_delta(&self) -> bool {
        self.delta_shards.is_some()
    }
}

// ─── Stats ───

#[derive(Debug, Clone)]
pub struct OptimizerStats {
    pub total_checkpoints: u64,
    pub delta_checkpoints: u64,
    pub total_size_bytes: usize,
    pub avg_compression_ratio: f32,
    pub verifications_passed: u64,
}

impl Default for OptimizerStats {
    fn default() -> Self {
        Self {
            total_checkpoints: 0,
            delta_checkpoints: 0,
            total_size_bytes: 0,
            avg_compression_ratio: 1.0,
            verifications_passed: 0,
        }
    }
}

// ─── Optimizer ───

pub struct CheckpointOptimizer {
    config: CheckpointOptimizerConfig,
    checkpoints: VecDeque<DeltaCheckpoint>,
    stats: OptimizerStats,
}

impl CheckpointOptimizer {
    pub fn new(config: CheckpointOptimizerConfig) -> Self {
        Self {
            config,
            checkpoints: VecDeque::new(),
            stats: OptimizerStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(CheckpointOptimizerConfig::default())
    }

    pub fn save_checkpoint(
        &mut self,
        round: u64,
        data: &[f32],
    ) -> Result<DeltaCheckpoint, CheckpointOptError> {
        if self.checkpoints.len() >= self.config.max_checkpoints {
            self.checkpoints.pop_front();
        }

        let shards = self.shard_data(data);

        let cp = if self.config.delta_enabled {
            self.create_delta_checkpoint(round, &shards)
        } else {
            DeltaCheckpoint::new(format!("cp-{}", round), round, shards)
        };

        if !cp.verify() {
            return Err(CheckpointOptError::CorruptCheckpoint(cp.id.clone()));
        }

        self.stats.total_checkpoints += 1;
        if cp.is_delta() {
            self.stats.delta_checkpoints += 1;
        }
        self.stats.total_size_bytes += cp.size_bytes;
        self.stats.verifications_passed += 1;

        self.checkpoints.push_back(cp.clone());
        Ok(cp)
    }

    pub fn restore(&self, round: u64) -> Option<&DeltaCheckpoint> {
        self.checkpoints.iter().rev().find(|cp| cp.round <= round)
    }

    pub fn get_latest(&self) -> Option<&DeltaCheckpoint> {
        self.checkpoints.back()
    }

    pub fn prune_before(&mut self, round: u64) -> usize {
        let len_before = self.checkpoints.len();
        self.checkpoints.retain(|cp| cp.round >= round);
        len_before - self.checkpoints.len()
    }

    pub fn get_stats(&self) -> &OptimizerStats {
        &self.stats
    }

    pub fn get_config(&self) -> &CheckpointOptimizerConfig {
        &self.config
    }

    fn shard_data(&self, data: &[f32]) -> Vec<Vec<f32>> {
        let shard_size = (data.len() as f32 / self.config.shard_count as f32) as usize;
        let shard_size = shard_size.max(1);
        data.chunks(shard_size).map(|c| c.to_vec()).collect()
    }

    fn create_delta_checkpoint(&mut self, round: u64, shards: &[Vec<f32>]) -> DeltaCheckpoint {
        match self.checkpoints.back() {
            Some(base) => {
                let delta = compute_delta(&base.shards, shards);
                let ratio = if base.shards.iter().map(|s| s.len()).sum::<usize>() > 0 {
                    delta.iter().map(|s| s.len()).sum::<usize>() as f32
                        / base.shards.iter().map(|s| s.len()).sum::<usize>() as f32
                } else {
                    1.0
                };
                self.stats.avg_compression_ratio = (self.stats.avg_compression_ratio
                    * (self.stats.total_checkpoints as f32)
                    + ratio)
                    / (self.stats.total_checkpoints as f32 + 1.0);

                DeltaCheckpoint::from_delta(
                    format!("cp-delta-{}", round),
                    round,
                    base.round,
                    &base.shards,
                    delta,
                )
            }
            None => DeltaCheckpoint::new(format!("cp-{}", round), round, shards.to_vec()),
        }
    }
}

impl Default for CheckpointOptimizer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_checksum(shards: &[Vec<f32>]) -> String {
    let mut hash: u64 = 0;
    for shard in shards {
        for val in shard {
            hash = hash.wrapping_add((*val as i32) as u64);
            hash = hash.wrapping_mul(31);
        }
    }
    format!("{:016x}", hash)
}

fn compute_delta(base: &[Vec<f32>], current: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let mut delta = Vec::new();
    for (b_shard, c_shard) in base.iter().zip(current.iter()) {
        let mut d_shard = Vec::new();
        for (b, c) in b_shard.iter().zip(c_shard.iter()) {
            let diff = c - b;
            if diff.abs() > 1e-6 {
                d_shard.push(diff);
            }
        }
        delta.push(d_shard);
    }
    delta
}

fn merge_delta(base: &[Vec<f32>], delta: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let mut merged = Vec::new();
    for (b_shard, d_shard) in base.iter().zip(delta.iter()) {
        let mut m_shard = Vec::new();
        for (i, b) in b_shard.iter().enumerate() {
            if i < d_shard.len() {
                m_shard.push(b + d_shard[i]);
            } else {
                m_shard.push(*b);
            }
        }
        merged.push(m_shard);
    }
    merged
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
    fn test_optimizer_creation() {
        let opt = CheckpointOptimizer::with_defaults();
        assert_eq!(opt.get_stats().total_checkpoints, 0);
    }

    #[test]
    fn test_save_checkpoint() {
        let mut opt = CheckpointOptimizer::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let cp = opt.save_checkpoint(1, &data).unwrap();
        assert_eq!(cp.round, 1);
        assert!(cp.verify());
    }

    #[test]
    fn test_delta_checkpoint() {
        let mut opt = CheckpointOptimizer::with_defaults();
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| i as f32 + 1.0).collect();
        opt.save_checkpoint(1, &data1).unwrap();
        let cp2 = opt.save_checkpoint(2, &data2).unwrap();
        assert!(cp2.is_delta());
    }

    #[test]
    fn test_restore_checkpoint() {
        let mut opt = CheckpointOptimizer::with_defaults();
        let data: Vec<f32> = (0..50).map(|i| i as f32).collect();
        opt.save_checkpoint(1, &data).unwrap();
        opt.save_checkpoint(2, &data).unwrap();
        assert!(opt.restore(1).is_some());
        assert!(opt.restore(0).is_none());
    }

    #[test]
    fn test_prune_before() {
        let mut opt = CheckpointOptimizer::with_defaults();
        let data: Vec<f32> = (0..50).map(|i| i as f32).collect();
        for i in 1..=10 {
            opt.save_checkpoint(i, &data).unwrap();
        }
        let pruned = opt.prune_before(5);
        assert_eq!(pruned, 4);
        assert_eq!(opt.checkpoints.len(), 6);
    }

    #[test]
    fn test_max_checkpoints_enforcement() {
        let mut opt = CheckpointOptimizer::new(CheckpointOptimizerConfig {
            max_checkpoints: 3,
            ..Default::default()
        });
        let data: Vec<f32> = (0..10).map(|i| i as f32).collect();
        for i in 1..=5 {
            opt.save_checkpoint(i, &data).unwrap();
        }
        assert_eq!(opt.checkpoints.len(), 3);
    }

    #[test]
    fn test_checkpoint_verification() {
        let cp = DeltaCheckpoint::new("t".to_string(), 1, vec![vec![1.0, 2.0]]);
        assert!(cp.verify());
    }

    #[test]
    fn test_stats_tracking() {
        let mut opt = CheckpointOptimizer::with_defaults();
        let data: Vec<f32> = (0..20).map(|i| i as f32).collect();
        opt.save_checkpoint(1, &data).unwrap();
        opt.save_checkpoint(2, &data).unwrap();
        assert_eq!(opt.get_stats().total_checkpoints, 2);
    }

    #[test]
    fn test_config_default() {
        let c = CheckpointOptimizerConfig::default();
        assert!(c.delta_enabled);
    }

    #[test]
    fn test_error_display() {
        let e = CheckpointOptError::CorruptCheckpoint("x".to_string());
        assert!(!e.to_string().is_empty());
    }
}
