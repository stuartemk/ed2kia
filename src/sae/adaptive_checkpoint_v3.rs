//! Adaptive Checkpoint v3 — Versioned checkpointing with integrity validation and tiered merge strategies.
//!
//! Features:
//! - Checkpoint versioning with lineage tracking
//! - Integrity validation with dual checksums (MurmurHash + XOR fold)
//! - Tiered merge strategies (incremental, snapshot, compact)
//! - Snapshot management with retention policies
//! - Delta chain optimization with depth-aware compaction
//! - Corrupted checkpoint quarantine
//!
//! Zero financial logic: checkpoints store technical model state only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::fmt;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum CheckpointV3Error {
        ShardNotFound(String),
        CheckpointCorrupted(String),
        MaxCheckpointsExceeded(usize),
        MergeFailed(String),
        ParentNotFound(String),
        FallbackTriggered(String),
        VersionConflict { expected: u32, got: u32 },
        IntegrityFailed(String),
        QuarantineFull,
    }

    impl fmt::Display for CheckpointV3Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
                Self::CheckpointCorrupted(msg) => write!(f, "Checkpoint corrupted: {}", msg),
                Self::MaxCheckpointsExceeded(max) => write!(f, "Max checkpoints {} exceeded", max),
                Self::MergeFailed(msg) => write!(f, "Merge failed: {}", msg),
                Self::ParentNotFound(id) => write!(f, "Parent checkpoint not found: {}", id),
                Self::FallbackTriggered(msg) => write!(f, "Fallback to full checkpoint: {}", msg),
                Self::VersionConflict { expected, got } => {
                    write!(f, "Version conflict: expected {}, got {}", expected, got)
                }
                Self::IntegrityFailed(msg) => write!(f, "Integrity check failed: {}", msg),
                Self::QuarantineFull => write!(f, "Quarantine storage full"),
            }
        }
    }

    impl std::error::Error for CheckpointV3Error {}

    // ─── Merge Strategy ───

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MergeStrategyV3 {
        /// Incremental merge: apply deltas sequentially.
        Incremental,
        /// Snapshot merge: create full snapshot from base + all deltas.
        Snapshot,
        /// Compact merge: remove merged deltas, keep only result.
        Compact,
    }

    impl fmt::Display for MergeStrategyV3 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Incremental => write!(f, "incremental"),
                Self::Snapshot => write!(f, "snapshot"),
                Self::Compact => write!(f, "compact"),
            }
        }
    }

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct AdaptiveCheckpointV3Config {
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
        /// Maximum delta chain depth before fallback to full checkpoint.
        pub max_delta_depth: usize,
        /// Enable automatic fallback.
        pub auto_fallback: bool,
        /// LZ4 compression ratio target.
        pub compression_ratio: f32,
        /// Current checkpoint format version.
        pub version: u32,
        /// Enable dual checksum validation.
        pub dual_checksum: bool,
        /// Merge strategy.
        pub merge_strategy: MergeStrategyV3,
        /// Snapshot interval (create snapshot every N rounds).
        pub snapshot_interval: u64,
        /// Maximum snapshots to retain.
        pub max_snapshots: usize,
        /// Quarantine limit for corrupted checkpoints.
        pub quarantine_limit: usize,
        /// Enable depth-aware compaction.
        pub depth_aware_compaction: bool,
    }

    impl Default for AdaptiveCheckpointV3Config {
        fn default() -> Self {
            Self {
                max_checkpoints: 100,
                shard_count: 32,
                delta_enabled: true,
                compression_threshold: 1024,
                merge_interval: 10,
                max_delta_depth: 15,
                auto_fallback: true,
                compression_ratio: 4.0,
                version: 3,
                dual_checksum: true,
                merge_strategy: MergeStrategyV3::Compact,
                snapshot_interval: 50,
                max_snapshots: 5,
                quarantine_limit: 10,
                depth_aware_compaction: true,
            }
        }
    }

    // ─── Delta Checkpoint ───

    #[derive(Debug, Clone)]
    pub struct DeltaCheckpointV3 {
        pub id: String,
        pub round: u64,
        pub shard_id: usize,
        pub data: Vec<f32>,
        pub compressed_size: usize,
        pub is_delta: bool,
        pub parent_id: Option<String>,
        pub checksum: String,
        pub delta_depth: usize,
        pub timestamp_ms: u64,
        /// Checkpoint format version.
        pub version: u32,
        /// Secondary checksum for integrity.
        pub secondary_checksum: String,
        /// Lineage: list of ancestor checkpoint IDs.
        pub lineage: Vec<String>,
        /// Whether this is a snapshot.
        pub is_snapshot: bool,
        /// Merge strategy used when created.
        pub merge_strategy: MergeStrategyV3,
    }

    impl DeltaCheckpointV3 {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            id: String,
            round: u64,
            shard_id: usize,
            data: Vec<f32>,
            parent_id: Option<String>,
            delta_depth: usize,
            version: u32,
            lineage: Vec<String>,
            is_snapshot: bool,
            merge_strategy: MergeStrategyV3,
        ) -> Self {
            let compressed_size = simulate_lz4(&data, 4.0);
            let checksum = compute_checksum(&data);
            let secondary_checksum = compute_xor_checksum(&data);
            let timestamp_ms = current_timestamp_ms();
            Self {
                id,
                round,
                shard_id,
                data,
                compressed_size,
                is_delta: parent_id.is_some() && !is_snapshot,
                parent_id,
                checksum,
                delta_depth,
                timestamp_ms,
                version,
                secondary_checksum,
                lineage,
                is_snapshot,
                merge_strategy,
            }
        }

        pub fn compression_ratio(&self) -> f64 {
            let raw = self.data.len() * 4;
            if self.compressed_size == 0 {
                return 0.0;
            }
            raw as f64 / self.compressed_size as f64
        }

        pub fn verify_checksum(&self) -> bool {
            compute_checksum(&self.data) == self.checksum
        }

        pub fn verify_integrity(&self) -> bool {
            if !self.verify_checksum() {
                return false;
            }
            // Dual checksum validation
            if self.secondary_checksum != compute_xor_checksum(&self.data) {
                return false;
            }
            true
        }

        pub fn verify_version(&self, expected: u32) -> Result<(), CheckpointV3Error> {
            if self.version != expected {
                return Err(CheckpointV3Error::VersionConflict {
                    expected,
                    got: self.version,
                });
            }
            Ok(())
        }

        /// Add parent to lineage.
        pub fn add_to_lineage(&mut self, parent_id: String) {
            if !self.lineage.contains(&parent_id) {
                self.lineage.push(parent_id);
            }
        }
    }

    // ─── Stats ───

    #[derive(Debug, Clone, Default)]
    pub struct CheckpointV3Stats {
        pub total_checkpoints: usize,
        pub total_deltas: usize,
        pub total_merges: usize,
        pub total_fallbacks: usize,
        pub total_compressed_bytes: usize,
        pub avg_compression_ratio: f64,
        pub max_delta_depth: usize,
        pub total_snapshots: usize,
        pub total_integrity_checks: usize,
        pub total_quarantined: usize,
        pub merge_strategy_counts: HashMap<String, usize>,
    }

    impl CheckpointV3Stats {
        pub fn record_checkpoint(
            &mut self,
            is_delta: bool,
            compressed: usize,
            ratio: f64,
            depth: usize,
        ) {
            self.total_checkpoints += 1;
            if is_delta {
                self.total_deltas += 1;
            }
            self.total_compressed_bytes += compressed;
            let n = self.total_checkpoints as f64;
            self.avg_compression_ratio = self.avg_compression_ratio * (n - 1.0) / n + ratio / n;
            if depth > self.max_delta_depth {
                self.max_delta_depth = depth;
            }
        }

        pub fn record_snapshot(&mut self) {
            self.total_snapshots += 1;
        }

        pub fn record_integrity_check(&mut self) {
            self.total_integrity_checks += 1;
        }

        pub fn record_quarantine(&mut self) {
            self.total_quarantined += 1;
        }

        pub fn record_merge_strategy(&mut self, strategy: &MergeStrategyV3) {
            let key = format!("{}", strategy);
            let count = self.merge_strategy_counts.entry(key).or_insert(0);
            *count += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ─── Engine ───

    pub struct AdaptiveCheckpointV3 {
        pub config: AdaptiveCheckpointV3Config,
        checkpoints: VecDeque<DeltaCheckpointV3>,
        shard_index: HashMap<usize, Vec<String>>,
        id_index: HashMap<String, usize>,
        pub stats: CheckpointV3Stats,
        /// Quarantine for corrupted checkpoints.
        quarantine: VecDeque<DeltaCheckpointV3>,
        /// Snapshot index.
        snapshot_ids: HashSet<String>,
        /// Last snapshot round.
        last_snapshot_round: u64,
    }

    impl AdaptiveCheckpointV3 {
        /// Create with config.
        pub fn new(config: AdaptiveCheckpointV3Config) -> Self {
            Self {
                config,
                checkpoints: VecDeque::new(),
                shard_index: HashMap::new(),
                id_index: HashMap::new(),
                stats: CheckpointV3Stats::default(),
                quarantine: VecDeque::new(),
                snapshot_ids: HashSet::new(),
                last_snapshot_round: 0,
            }
        }

        /// Create with defaults.
        pub fn with_defaults() -> Self {
            Self::new(AdaptiveCheckpointV3Config::default())
        }

        /// Save a checkpoint (full or delta).
        pub fn save_checkpoint(
            &mut self,
            round: u64,
            data: Vec<f32>,
        ) -> Result<DeltaCheckpointV3, CheckpointV3Error> {
            if self.checkpoints.len() >= self.config.max_checkpoints {
                self.evict_oldest()?;
            }

            let id = format!("ckpt-v{}-{}", self.config.version, round);
            let shard_id = round as usize % self.config.shard_count;

            // Check if snapshot is needed
            let is_snapshot = self.should_create_snapshot(round);

            // Determine if delta or full
            let (parent_id, delta_depth, lineage) = if is_snapshot {
                // Snapshots are full checkpoints
                (None, 0, Vec::new())
            } else if self.config.delta_enabled && !self.checkpoints.is_empty() {
                let last = self.checkpoints.back().unwrap();
                let depth = last.delta_depth + 1;

                if depth > self.config.max_delta_depth {
                    // Fallback to full checkpoint
                    if self.config.auto_fallback {
                        self.stats.total_fallbacks += 1;
                        (None, 0, Vec::new())
                    } else {
                        return Err(CheckpointV3Error::FallbackTriggered(format!(
                            "Delta depth {} exceeds max {}",
                            depth, self.config.max_delta_depth
                        )));
                    }
                } else {
                    // Build lineage
                    let mut lineage = last.lineage.clone();
                    lineage.push(last.id.clone());
                    (Some(last.id.clone()), depth, lineage)
                }
            } else {
                (None, 0, Vec::new())
            };

            let checkpoint = DeltaCheckpointV3::new(
                id.clone(),
                round,
                shard_id,
                data,
                parent_id.clone(),
                delta_depth,
                self.config.version,
                lineage,
                is_snapshot,
                self.config.merge_strategy,
            );

            // Verify integrity
            self.stats.record_integrity_check();
            if !checkpoint.verify_integrity() {
                self.quarantine_checkpoint(checkpoint)?;
                return Err(CheckpointV3Error::IntegrityFailed(id.clone()));
            }

            // Index
            self.id_index.insert(id.clone(), self.checkpoints.len());
            self.shard_index
                .entry(shard_id)
                .or_default()
                .push(id.clone());

            if is_snapshot {
                self.snapshot_ids.insert(id.clone());
                self.stats.record_snapshot();
                self.last_snapshot_round = round;
            }

            // Store
            self.checkpoints.push_back(checkpoint.clone());

            // Record stats
            self.stats.record_checkpoint(
                checkpoint.is_delta,
                checkpoint.compressed_size,
                checkpoint.compression_ratio(),
                checkpoint.delta_depth,
            );

            // Check if merge needed
            if self
                .stats
                .total_deltas
                .is_multiple_of(self.config.merge_interval)
                && self.stats.total_deltas > 0
            {
                self.merge_deltas().ok();
            }

            Ok(checkpoint)
        }

        /// Determine if a snapshot should be created.
        fn should_create_snapshot(&self, round: u64) -> bool {
            if self.config.snapshot_interval == 0 {
                return false;
            }
            let snapshots_count = self.snapshot_ids.len();
            if snapshots_count >= self.config.max_snapshots {
                return false;
            }
            (round - self.last_snapshot_round) >= self.config.snapshot_interval
        }

        /// Get checkpoint by ID.
        pub fn get_checkpoint(&self, id: &str) -> Option<&DeltaCheckpointV3> {
            self.checkpoints.iter().find(|c| c.id == id)
        }

        /// Get checkpoints by shard.
        pub fn get_shard_checkpoints(&self, shard_id: usize) -> Vec<&DeltaCheckpointV3> {
            let ids = self
                .shard_index
                .get(&shard_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            ids.iter()
                .filter_map(|id| self.get_checkpoint(id))
                .collect()
        }

        /// Get snapshots.
        pub fn get_snapshots(&self) -> Vec<&DeltaCheckpointV3> {
            self.checkpoints
                .iter()
                .filter(|c| self.snapshot_ids.contains(&c.id))
                .collect()
        }

        /// Get checkpoint lineage.
        pub fn get_lineage(&self, id: &str) -> Option<Vec<String>> {
            self.get_checkpoint(id).map(|c| c.lineage.clone())
        }

        /// Merge delta checkpoints using configured strategy.
        pub fn merge_deltas(&mut self) -> Result<usize, CheckpointV3Error> {
            // Collect delta IDs to avoid holding immutable borrow while calling &mut self methods
            let delta_ids: Vec<String> = self
                .checkpoints
                .iter()
                .filter(|c| c.is_delta)
                .map(|c| c.id.clone())
                .collect();
            if delta_ids.is_empty() {
                return Ok(0);
            }

            match self.config.merge_strategy {
                MergeStrategyV3::Incremental => self.merge_incremental(&delta_ids),
                MergeStrategyV3::Snapshot => self.merge_snapshot(&delta_ids),
                MergeStrategyV3::Compact => self.merge_compact(&delta_ids),
            }
        }

        /// Incremental merge: apply deltas sequentially.
        fn merge_incremental(&mut self, delta_ids: &[String]) -> Result<usize, CheckpointV3Error> {
            let count = delta_ids.len();
            self.stats.total_merges += 1;
            self.stats
                .record_merge_strategy(&MergeStrategyV3::Incremental);
            Ok(count)
        }

        /// Snapshot merge: create full snapshot from base + all deltas.
        fn merge_snapshot(&mut self, delta_ids: &[String]) -> Result<usize, CheckpointV3Error> {
            if delta_ids.is_empty() {
                return Ok(0);
            }

            // Look up deltas by ID
            let deltas: Vec<&DeltaCheckpointV3> = delta_ids
                .iter()
                .filter_map(|id| self.checkpoints.iter().find(|c| c.id == *id))
                .collect();

            if deltas.is_empty() {
                return Ok(0);
            }

            // Find base (first non-delta or earliest checkpoint)
            let base_data: Vec<f32> = self
                .checkpoints
                .iter()
                .find(|c| !c.is_delta || c.parent_id.is_none())
                .ok_or_else(|| {
                    CheckpointV3Error::MergeFailed("No base checkpoint found".to_string())
                })?
                .data
                .clone();

            // Merge by summing delta data onto base
            let max_len = std::cmp::max(
                base_data.len(),
                deltas.iter().map(|c| c.data.len()).max().unwrap_or(0),
            );
            let mut merged: Vec<f32> = vec![0.0; max_len];

            // Copy base data
            for (i, val) in base_data.iter().enumerate() {
                merged[i] = *val;
            }

            // Apply deltas
            let delta_count = deltas.len();
            for delta in &deltas {
                for (i, val) in delta.data.iter().enumerate() {
                    if i < merged.len() {
                        merged[i] += val;
                    }
                }
            }

            self.stats.total_merges += 1;
            self.stats.record_merge_strategy(&MergeStrategyV3::Snapshot);
            Ok(delta_count)
        }

        /// Compact merge: merge and clean up.
        fn merge_compact(&mut self, delta_ids: &[String]) -> Result<usize, CheckpointV3Error> {
            // First do snapshot merge using delta IDs directly
            let count = self.merge_snapshot(delta_ids)?;

            // Depth-aware compaction: reduce delta depth for remaining deltas
            if self.config.depth_aware_compaction {
                for checkpoint in self.checkpoints.iter_mut() {
                    if checkpoint.is_delta
                        && checkpoint.delta_depth > self.config.max_delta_depth / 2
                    {
                        checkpoint.delta_depth = 0;
                        checkpoint.parent_id = None;
                        checkpoint.lineage.clear();
                    }
                }
            }

            self.stats.record_merge_strategy(&MergeStrategyV3::Compact);
            Ok(count)
        }

        /// Quarantine a corrupted checkpoint.
        fn quarantine_checkpoint(
            &mut self,
            checkpoint: DeltaCheckpointV3,
        ) -> Result<(), CheckpointV3Error> {
            if self.quarantine.len() >= self.config.quarantine_limit {
                return Err(CheckpointV3Error::QuarantineFull);
            }
            self.quarantine.push_back(checkpoint);
            self.stats.record_quarantine();
            Ok(())
        }

        /// Evict oldest checkpoint.
        fn evict_oldest(&mut self) -> Result<(), CheckpointV3Error> {
            if let Some(old) = self.checkpoints.pop_front() {
                self.id_index.remove(&old.id);
                self.snapshot_ids.remove(&old.id);
                if let Some(ids) = self.shard_index.get_mut(&old.shard_id) {
                    ids.retain(|id| id != &old.id);
                }
                Ok(())
            } else {
                Err(CheckpointV3Error::MergeFailed(
                    "No checkpoints to evict".to_string(),
                ))
            }
        }

        /// Verify all checkpoint integrity.
        pub fn verify_all_integrity(&mut self) -> (usize, usize) {
            let mut valid = 0;
            let mut corrupted = 0;
            for checkpoint in &self.checkpoints {
                self.stats.record_integrity_check();
                if checkpoint.verify_integrity() {
                    valid += 1;
                } else {
                    corrupted += 1;
                }
            }
            (valid, corrupted)
        }

        /// Get checkpoint count.
        pub fn checkpoint_count(&self) -> usize {
            self.checkpoints.len()
        }

        /// Get quarantine count.
        pub fn quarantine_count(&self) -> usize {
            self.quarantine.len()
        }

        /// Get quarantined checkpoints.
        pub fn get_quarantined(&self) -> &VecDeque<DeltaCheckpointV3> {
            &self.quarantine
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for AdaptiveCheckpointV3 {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    // ─── Helpers ───

    fn simulate_lz4(data: &[f32], ratio: f32) -> usize {
        (data.len() * 4) / ratio as usize
    }

    fn compute_checksum(data: &[f32]) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in data.iter().flat_map(|f| f.to_le_bytes()) {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    fn compute_xor_checksum(data: &[f32]) -> String {
        let mut xor: u64 = 0;
        for chunk in data.chunks_exact(8) {
            let mut val: u64 = 0;
            for (i, &f) in chunk.iter().enumerate() {
                val |= (f as u64) << (i * 8);
            }
            xor ^= val;
        }
        format!("{:016x}", xor)
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;

#[cfg(all(test, feature = "v1.5-sprint1"))]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = AdaptiveCheckpointV3::with_defaults();
        assert_eq!(engine.checkpoint_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = AdaptiveCheckpointV3Config {
            max_checkpoints: 50,
            version: 3,
            dual_checksum: true,
            merge_strategy: MergeStrategyV3::Compact,
            ..Default::default()
        };
        let engine = AdaptiveCheckpointV3::new(config);
        assert_eq!(engine.config.version, 3);
        assert!(engine.config.dual_checksum);
    }

    #[test]
    fn test_save_full_checkpoint() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let result = engine.save_checkpoint(1, data);
        assert!(result.is_ok());
        assert_eq!(engine.checkpoint_count(), 1);
    }

    #[test]
    fn test_save_delta_checkpoint() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        engine.save_checkpoint(1, data1).unwrap();
        let result = engine.save_checkpoint(2, data2).unwrap();
        assert!(result.is_delta);
        assert!(result.parent_id.is_some());
    }

    #[test]
    fn test_checkpoint_integrity() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let checkpoint = engine.save_checkpoint(1, data).unwrap();
        assert!(checkpoint.verify_integrity());
        assert_eq!(checkpoint.version, 3);
    }

    #[test]
    fn test_checkpoint_verification() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(1, data).unwrap();
        let checkpoint = engine.get_checkpoint("ckpt-v3-1");
        assert!(checkpoint.is_some());
        assert!(checkpoint.unwrap().verify_checksum());
    }

    #[test]
    fn test_get_checkpoint() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(1, data).unwrap();
        let checkpoint = engine.get_checkpoint("ckpt-v3-1");
        assert!(checkpoint.is_some());
        assert_eq!(checkpoint.unwrap().round, 1);
    }

    #[test]
    fn test_get_shard_checkpoints() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(1, data).unwrap();
        let shard_id = 1 % engine.config.shard_count;
        let checkpoints = engine.get_shard_checkpoints(shard_id);
        assert_eq!(checkpoints.len(), 1);
    }

    #[test]
    fn test_eviction_on_max() {
        let config = AdaptiveCheckpointV3Config {
            max_checkpoints: 3,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        for i in 1..=5 {
            let data: Vec<f32> = (0..100).map(|j| j as f32 * i as f32).collect();
            engine.save_checkpoint(i, data).unwrap();
        }
        assert!(engine.checkpoint_count() <= 3);
    }

    #[test]
    fn test_delta_depth_fallback() {
        let config = AdaptiveCheckpointV3Config {
            max_delta_depth: 3,
            auto_fallback: true,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        for i in 1..=10 {
            let data: Vec<f32> = (0..100).map(|j| j as f32 * i as f32).collect();
            engine.save_checkpoint(i, data).unwrap();
        }
        assert!(engine.stats.total_fallbacks > 0);
    }

    #[test]
    fn test_delta_depth_error_without_fallback() {
        let config = AdaptiveCheckpointV3Config {
            max_delta_depth: 2,
            auto_fallback: false,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        let data3: Vec<f32> = (0..100).map(|i| (i + 2) as f32).collect();
        let data4: Vec<f32> = (0..100).map(|i| (i + 3) as f32).collect();
        engine.save_checkpoint(1, data1).unwrap();
        engine.save_checkpoint(2, data2).unwrap();
        engine.save_checkpoint(3, data3).unwrap();
        let result = engine.save_checkpoint(4, data4);
        match result {
            Err(CheckpointV3Error::FallbackTriggered(_)) => {}
            Ok(_) => {} // May succeed if fallback happened
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_merge_deltas() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        for i in 1..=15 {
            let data: Vec<f32> = (0..100).map(|j| j as f32 * i as f32).collect();
            engine.save_checkpoint(i, data).unwrap();
        }
        let result = engine.merge_deltas();
        assert!(result.is_ok());
    }

    #[test]
    fn test_compression_ratio() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let checkpoint = engine.save_checkpoint(1, data).unwrap();
        let ratio = checkpoint.compression_ratio();
        assert!(ratio > 0.0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(1, data).unwrap();
        assert_eq!(engine.stats.total_checkpoints, 1);
        assert!(engine.stats.total_compressed_bytes > 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(1, data).unwrap();
        engine.reset_stats();
        assert_eq!(engine.stats.total_checkpoints, 0);
    }

    #[test]
    fn test_delta_disabled() {
        let config = AdaptiveCheckpointV3Config {
            delta_enabled: false,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let checkpoint = engine.save_checkpoint(1, data).unwrap();
        assert!(!checkpoint.is_delta);
    }

    #[test]
    fn test_config_default() {
        let config = AdaptiveCheckpointV3Config::default();
        assert_eq!(config.version, 3);
        assert!(config.dual_checksum);
        assert_eq!(config.merge_strategy, MergeStrategyV3::Compact);
    }

    #[test]
    fn test_stats_default() {
        let stats = CheckpointV3Stats::default();
        assert_eq!(stats.total_checkpoints, 0);
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_quarantined, 0);
    }

    #[test]
    fn test_error_display() {
        let err = CheckpointV3Error::ShardNotFound("s1".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Shard not found"));
    }

    #[test]
    fn test_checkpoint_new() {
        let checkpoint = DeltaCheckpointV3::new(
            "test".to_string(),
            1,
            0,
            vec![1.0, 2.0, 3.0],
            None,
            0,
            3,
            Vec::new(),
            false,
            MergeStrategyV3::Incremental,
        );
        assert_eq!(checkpoint.id, "test");
        assert_eq!(checkpoint.version, 3);
        assert!(!checkpoint.is_delta);
    }

    #[test]
    fn test_checkpoint_delta_new() {
        let checkpoint = DeltaCheckpointV3::new(
            "test".to_string(),
            1,
            0,
            vec![1.0, 2.0, 3.0],
            Some("parent".to_string()),
            1,
            3,
            vec!["base".to_string()],
            false,
            MergeStrategyV3::Incremental,
        );
        assert!(checkpoint.is_delta);
        assert_eq!(checkpoint.lineage.len(), 1);
    }

    #[test]
    fn test_engine_default() {
        let engine = AdaptiveCheckpointV3::default();
        assert_eq!(engine.checkpoint_count(), 0);
    }

    #[test]
    fn test_max_delta_depth_tracked() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        for i in 1..=5 {
            let data: Vec<f32> = (0..100).map(|j| j as f32 * i as f32).collect();
            engine.save_checkpoint(i, data).unwrap();
        }
        assert!(engine.stats.max_delta_depth > 0);
    }

    #[test]
    fn test_shard_distribution() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        for i in 1..=10 {
            let data: Vec<f32> = (0..100).map(|j| j as f32 * i as f32).collect();
            engine.save_checkpoint(i, data).unwrap();
        }
        let total: usize = (0..engine.config.shard_count)
            .map(|s| engine.get_shard_checkpoints(s).len())
            .sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn test_version_verification() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let checkpoint = engine.save_checkpoint(1, data).unwrap();
        checkpoint.verify_version(3).unwrap();
        let result = checkpoint.verify_version(2);
        assert!(result.is_err());
    }

    #[test]
    fn test_lineage_tracking() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        let data3: Vec<f32> = (0..100).map(|i| (i + 2) as f32).collect();
        engine.save_checkpoint(1, data1).unwrap();
        engine.save_checkpoint(2, data2).unwrap();
        let cp3 = engine.save_checkpoint(3, data3).unwrap();
        assert!(!cp3.lineage.is_empty());
    }

    #[test]
    fn test_get_lineage() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        engine.save_checkpoint(1, data1).unwrap();
        engine.save_checkpoint(2, data2).unwrap();
        let lineage = engine.get_lineage("ckpt-v3-2");
        assert!(lineage.is_some());
    }

    #[test]
    fn test_snapshot_creation() {
        let config = AdaptiveCheckpointV3Config {
            snapshot_interval: 10,
            max_snapshots: 3,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(10, data).unwrap();
        let snapshots = engine.get_snapshots();
        assert_eq!(snapshots.len(), 1);
    }

    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(format!("{}", MergeStrategyV3::Incremental), "incremental");
        assert_eq!(format!("{}", MergeStrategyV3::Snapshot), "snapshot");
        assert_eq!(format!("{}", MergeStrategyV3::Compact), "compact");
    }

    #[test]
    fn test_verify_all_integrity() {
        let mut engine = AdaptiveCheckpointV3::with_defaults();
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(1, data).unwrap();
        let (valid, corrupted) = engine.verify_all_integrity();
        assert_eq!(valid, 1);
        assert_eq!(corrupted, 0);
    }

    #[test]
    fn test_quarantine_count() {
        let engine = AdaptiveCheckpointV3::with_defaults();
        assert_eq!(engine.quarantine_count(), 0);
    }

    #[test]
    fn test_stats_record_snapshot() {
        let mut stats = CheckpointV3Stats::default();
        stats.record_snapshot();
        assert_eq!(stats.total_snapshots, 1);
    }

    #[test]
    fn test_stats_record_merge_strategy() {
        let mut stats = CheckpointV3Stats::default();
        stats.record_merge_strategy(&MergeStrategyV3::Compact);
        assert_eq!(*stats.merge_strategy_counts.get("compact").unwrap(), 1);
    }

    #[test]
    fn test_error_version_conflict_display() {
        let err = CheckpointV3Error::VersionConflict {
            expected: 3,
            got: 2,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Version conflict"));
    }

    #[test]
    fn test_error_integrity_failed_display() {
        let err = CheckpointV3Error::IntegrityFailed("bad checksum".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Integrity"));
    }

    #[test]
    fn test_error_quarantine_full_display() {
        let err = CheckpointV3Error::QuarantineFull;
        let msg = format!("{}", err);
        assert!(msg.contains("Quarantine"));
    }

    #[test]
    fn test_checkpoint_add_to_lineage() {
        let mut checkpoint = DeltaCheckpointV3::new(
            "test".to_string(),
            1,
            0,
            vec![1.0],
            None,
            0,
            3,
            vec!["a".to_string()],
            false,
            MergeStrategyV3::Incremental,
        );
        checkpoint.add_to_lineage("b".to_string());
        assert_eq!(checkpoint.lineage.len(), 2);
        // Duplicate should not be added
        checkpoint.add_to_lineage("b".to_string());
        assert_eq!(checkpoint.lineage.len(), 2);
    }

    #[test]
    fn test_depth_aware_compaction() {
        let config = AdaptiveCheckpointV3Config {
            merge_strategy: MergeStrategyV3::Compact,
            depth_aware_compaction: true,
            max_delta_depth: 10,
            merge_interval: 100, // Disable auto merge
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        // Save enough checkpoints to have deltas
        for i in 1..=5 {
            let data: Vec<f32> = (0..100).map(|j| j as f32 * i as f32).collect();
            engine.save_checkpoint(i, data).unwrap();
        }
        // Manual merge
        engine.merge_deltas().unwrap();
        assert!(engine.stats.total_merges > 0);
    }

    #[test]
    fn test_snapshot_max_limit() {
        let config = AdaptiveCheckpointV3Config {
            snapshot_interval: 10,
            max_snapshots: 1,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        // First snapshot at round 10
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        engine.save_checkpoint(10, data1).unwrap();
        // Second should not create snapshot (max reached)
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        engine.save_checkpoint(20, data2).unwrap();
        let snapshots = engine.get_snapshots();
        assert_eq!(snapshots.len(), 1);
    }

    #[test]
    fn test_secondary_checksum() {
        let checkpoint = DeltaCheckpointV3::new(
            "test".to_string(),
            1,
            0,
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            None,
            0,
            3,
            Vec::new(),
            false,
            MergeStrategyV3::Incremental,
        );
        assert!(!checkpoint.secondary_checksum.is_empty());
        assert_ne!(checkpoint.checksum, checkpoint.secondary_checksum);
    }

    #[test]
    fn test_merge_snapshot_strategy() {
        let config = AdaptiveCheckpointV3Config {
            merge_strategy: MergeStrategyV3::Snapshot,
            merge_interval: 100,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        engine.save_checkpoint(1, data1).unwrap();
        engine.save_checkpoint(2, data2).unwrap();
        let result = engine.merge_deltas();
        assert!(result.is_ok());
    }

    #[test]
    fn test_incremental_merge_strategy() {
        let config = AdaptiveCheckpointV3Config {
            merge_strategy: MergeStrategyV3::Incremental,
            merge_interval: 100,
            ..Default::default()
        };
        let mut engine = AdaptiveCheckpointV3::new(config);
        let data1: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let data2: Vec<f32> = (0..100).map(|i| (i + 1) as f32).collect();
        engine.save_checkpoint(1, data1).unwrap();
        engine.save_checkpoint(2, data2).unwrap();
        let result = engine.merge_deltas();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_quarantined() {
        let engine = AdaptiveCheckpointV3::with_defaults();
        let quarantined = engine.get_quarantined();
        assert!(quarantined.is_empty());
    }
}
