//! Snapshot Manager — State snapshot creation, restoration, and verification with Merkle integrity.
//!
//! Provides efficient state snapshots with cryptographic verification,
//! incremental diffing, and compression for state synchronization.

mod internal {
    use std::collections::HashMap;

    // ---------------------------------------------------------------------
    // Error
    // ---------------------------------------------------------------------

    /// Errors for snapshot operations.
    #[derive(Debug)]
    pub enum SnapshotError {
        /// Snapshot not found.
        SnapshotNotFound(String),
        /// State key not found in snapshot.
        KeyNotFound(String),
        /// Integrity verification failed.
        IntegrityFailed(String),
        /// Corrupted snapshot data.
        CorruptedData(String),
        /// Snapshot already exists.
        SnapshotExists(String),
    }

    impl std::fmt::Display for SnapshotError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SnapshotError::SnapshotNotFound(id) => write!(f, "Snapshot not found: {}", id),
                SnapshotError::KeyNotFound(key) => write!(f, "Key not found: {}", key),
                SnapshotError::IntegrityFailed(msg) => write!(f, "Integrity failed: {}", msg),
                SnapshotError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
                SnapshotError::SnapshotExists(id) => write!(f, "Snapshot already exists: {}", id),
            }
        }
    }

    // ---------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------

    /// Configuration for snapshot manager.
    #[derive(Clone)]
    pub struct SnapshotConfig {
        /// Maximum number of snapshots to retain.
        pub max_snapshots: usize,
        /// Enable LZ4 compression for snapshots.
        pub enable_compression: bool,
        /// Enable Merkle integrity verification.
        pub enable_merkle_verification: bool,
        /// Snapshot TTL in milliseconds (0 = no expiry).
        pub snapshot_ttl_ms: u64,
    }

    impl Default for SnapshotConfig {
        fn default() -> Self {
            Self {
                max_snapshots: 50,
                enable_compression: true,
                enable_merkle_verification: true,
                snapshot_ttl_ms: 0,
            }
        }
    }

    // ---------------------------------------------------------------------
    // Snapshot Entry
    // ---------------------------------------------------------------------

    /// A single state snapshot with integrity metadata.
    #[derive(Clone, Debug)]
    pub struct SnapshotEntry {
        /// Unique snapshot identifier.
        pub id: String,
        /// Key-value state at snapshot time.
        pub state: HashMap<String, Vec<u8>>,
        /// Merkle root for integrity verification.
        pub merkle_root: String,
        /// Creation timestamp in milliseconds.
        pub created_at_ms: u64,
        /// Monotonic sequence number for insertion order.
        pub sequence: u64,
        /// Size in bytes before compression.
        pub size_bytes: usize,
        /// Size in bytes after compression (if compressed).
        pub compressed_size_bytes: usize,
        /// Parent snapshot ID for incremental diffs.
        pub parent_id: Option<String>,
    }

    impl SnapshotEntry {
        pub fn new(id: String, created_at_ms: u64, sequence: u64) -> Self {
            Self {
                id,
                state: HashMap::new(),
                merkle_root: String::new(),
                created_at_ms,
                sequence,
                size_bytes: 0,
                compressed_size_bytes: 0,
                parent_id: None,
            }
        }

        /// Calculate compression ratio.
        pub fn compression_ratio(&self) -> f64 {
            if self.size_bytes == 0 {
                return 1.0;
            }
            self.compressed_size_bytes as f64 / self.size_bytes as f64
        }

        /// Calculate space saved in bytes.
        pub fn space_saved(&self) -> usize {
            self.size_bytes.saturating_sub(self.compressed_size_bytes)
        }
    }

    // ---------------------------------------------------------------------
    // Diff Entry
    // ---------------------------------------------------------------------

    /// Represents a state difference between two snapshots.
    #[derive(Clone, Debug)]
    pub struct DiffEntry {
        /// Key that changed.
        pub key: String,
        /// Old value (None if new key).
        pub old_value: Option<Vec<u8>>,
        /// New value (None if deleted).
        pub new_value: Option<Vec<u8>>,
    }

    // ---------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------

    /// Statistics for snapshot operations.
    #[derive(Clone)]
    pub struct SnapshotStats {
        /// Total snapshots created.
        pub total_snapshots: u64,
        /// Total restores performed.
        pub total_restores: u64,
        /// Total verifications performed.
        pub total_verifications: u64,
        /// Total integrity failures.
        pub integrity_failures: u64,
        /// Average creation time in milliseconds.
        pub avg_creation_time_ms: f64,
        /// Total space saved through compression.
        pub total_space_saved: usize,
    }

    impl Default for SnapshotStats {
        fn default() -> Self {
            Self {
                total_snapshots: 0,
                total_restores: 0,
                total_verifications: 0,
                integrity_failures: 0,
                avg_creation_time_ms: 0.0,
                total_space_saved: 0,
            }
        }
    }

    impl SnapshotStats {
        /// Record a snapshot creation.
        pub fn record_creation(&mut self, time_ms: u64, space_saved: usize) {
            self.total_snapshots += 1;
            self.total_space_saved += space_saved;
            self.avg_creation_time_ms = self.avg_creation_time_ms
                * (self.total_snapshots - 1) as f64
                / self.total_snapshots as f64
                + time_ms as f64 / self.total_snapshots as f64;
        }

        /// Record a restore operation.
        pub fn record_restore(&mut self) {
            self.total_restores += 1;
        }

        /// Record a verification attempt.
        pub fn record_verification(&mut self, valid: bool) {
            self.total_verifications += 1;
            if !valid {
                self.integrity_failures += 1;
            }
        }

        /// Reset all statistics.
        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------
    // Snapshot Manager
    // ---------------------------------------------------------------------

    /// Manages state snapshots with integrity verification and compression.
    pub struct SnapshotManager {
        config: SnapshotConfig,
        snapshots: HashMap<String, SnapshotEntry>,
        stats: SnapshotStats,
        next_sequence: u64,
    }

    impl SnapshotManager {
        /// Create a new snapshot manager with the given configuration.
        pub fn new(config: SnapshotConfig) -> Self {
            Self {
                config,
                snapshots: HashMap::new(),
                stats: SnapshotStats::default(),
                next_sequence: 0,
            }
        }

        /// Create a new snapshot from current state.
        pub fn create_snapshot(
            &mut self,
            id: String,
            state: HashMap<String, Vec<u8>>,
            parent_id: Option<String>,
        ) -> Result<SnapshotEntry, SnapshotError> {
            // Check if snapshot already exists
            if self.snapshots.contains_key(&id) {
                return Err(SnapshotError::SnapshotExists(id));
            }

            let start_ms = current_timestamp_ms();

            // Calculate total size
            let size_bytes: usize = state.values().map(|v| v.len()).sum();

            // Create snapshot entry with sequence for deterministic ordering
            let sequence = self.next_sequence;
            self.next_sequence += 1;
            let mut entry = SnapshotEntry::new(id.clone(), start_ms, sequence);
            entry.parent_id = parent_id;
            entry.size_bytes = size_bytes;

            // Apply compression if enabled
            if self.config.enable_compression {
                entry.state = Self::compress_state(&state);
                entry.compressed_size_bytes = entry.state.values().map(|v| v.len()).sum();
            } else {
                entry.state = state;
                entry.compressed_size_bytes = size_bytes;
            }

            // Compute Merkle root for integrity
            if self.config.enable_merkle_verification {
                entry.merkle_root = Self::compute_merkle_root(&entry.state);
            }

            // Enforce max snapshots
            while self.snapshots.len() >= self.config.max_snapshots {
                self.evict_oldest();
            }

            let creation_time = current_timestamp_ms().saturating_sub(start_ms);
            let space_saved = entry.space_saved();
            self.stats.record_creation(creation_time, space_saved);

            self.snapshots.insert(id.clone(), entry.clone());
            Ok(entry)
        }

        /// Get a snapshot by ID.
        pub fn get_snapshot(&self, id: &str) -> Result<&SnapshotEntry, SnapshotError> {
            self.snapshots
                .get(id)
                .ok_or_else(|| SnapshotError::SnapshotNotFound(id.to_string()))
        }

        /// Get the latest snapshot.
        pub fn get_latest_snapshot(&self) -> Option<&SnapshotEntry> {
            self.snapshots
                .values()
                .max_by_key(|s| (s.created_at_ms, s.sequence))
        }

        /// Verify snapshot integrity using Merkle root.
        pub fn verify_integrity(&mut self, id: &str) -> Result<bool, SnapshotError> {
            let entry = self
                .snapshots
                .get(id)
                .ok_or_else(|| SnapshotError::SnapshotNotFound(id.to_string()))?;

            let current_root = Self::compute_merkle_root(&entry.state);
            let valid = current_root == entry.merkle_root;

            self.stats.record_verification(valid);

            if !valid {
                return Err(SnapshotError::IntegrityFailed(id.to_string()));
            }

            Ok(true)
        }

        /// Restore state from a snapshot.
        pub fn restore(&mut self, id: &str) -> Result<HashMap<String, Vec<u8>>, SnapshotError> {
            // Verify integrity before restore (needs mutable access)
            if self.config.enable_merkle_verification {
                self.verify_integrity(id)?;
            }

            // Get snapshot data after verification
            let entry = self.get_snapshot(id)?;
            let restored = if self.config.enable_compression {
                Self::decompress_state(&entry.state)
            } else {
                entry.state.clone()
            };

            self.stats.record_restore();
            Ok(restored)
        }

        /// Get the latest version of a key across snapshots.
        pub fn get_key(&self, key: &str) -> Option<&Vec<u8>> {
            self.snapshots
                .values()
                .max_by_key(|s| s.created_at_ms)
                .and_then(|s| s.state.get(key))
        }

        /// Compute diff between two snapshots.
        pub fn compute_diff(
            &self,
            old_id: &str,
            new_id: &str,
        ) -> Result<Vec<DiffEntry>, SnapshotError> {
            let old = self.get_snapshot(old_id)?;
            let new = self.get_snapshot(new_id)?;

            let mut diffs = Vec::new();

            // Find added and modified keys
            for (key, new_value) in &new.state {
                match old.state.get(key) {
                    Some(old_value) => {
                        if old_value != new_value {
                            diffs.push(DiffEntry {
                                key: key.clone(),
                                old_value: Some(old_value.clone()),
                                new_value: Some(new_value.clone()),
                            });
                        }
                    }
                    None => {
                        diffs.push(DiffEntry {
                            key: key.clone(),
                            old_value: None,
                            new_value: Some(new_value.clone()),
                        });
                    }
                }
            }

            // Find deleted keys
            for key in old.state.keys() {
                if !new.state.contains_key(key) {
                    diffs.push(DiffEntry {
                        key: key.clone(),
                        old_value: old.state.get(key).cloned(),
                        new_value: None,
                    });
                }
            }

            Ok(diffs)
        }

        /// Remove expired snapshots based on TTL.
        pub fn cleanup_expired(&mut self, current_ms: u64) -> usize {
            if self.config.snapshot_ttl_ms == 0 {
                return 0;
            }

            let cutoff = current_ms.saturating_sub(self.config.snapshot_ttl_ms);

            let keys_to_remove: Vec<String> = self
                .snapshots
                .iter()
                .filter(|(_, s)| s.created_at_ms <= cutoff)
                .map(|(k, _)| k.clone())
                .collect();

            let count = keys_to_remove.len();
            for key in keys_to_remove {
                self.snapshots.remove(&key);
            }

            count
        }

        /// Get statistics.
        pub fn get_stats(&self) -> SnapshotStats {
            self.stats.clone()
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Get total snapshot count.
        pub fn snapshot_count(&self) -> usize {
            self.snapshots.len()
        }

        // ----------------------------------------------------------------
        // Private helpers
        // ----------------------------------------------------------------

        /// Evict the oldest snapshot.
        fn evict_oldest(&mut self) {
            if let Some(oldest_id) = self
                .snapshots
                .iter()
                .min_by_key(|(_, s)| (s.created_at_ms, s.sequence))
                .map(|(k, _)| k.clone())
            {
                self.snapshots.remove(&oldest_id);
            }
        }

        /// Compute Merkle root from state map.
        fn compute_merkle_root(state: &HashMap<String, Vec<u8>>) -> String {
            if state.is_empty() {
                return compute_sha256("empty");
            }

            let mut leaves: Vec<String> = state
                .iter()
                .map(|(k, v)| format!("{}:{}", k, hex::encode(v)))
                .collect();
            leaves.sort();

            Self::build_merkle_root(&leaves)
        }

        /// Build Merkle root from sorted leaves.
        fn build_merkle_root(leaves: &[String]) -> String {
            if leaves.is_empty() {
                return compute_sha256("empty");
            }

            let mut current = leaves.to_vec();
            while current.len() > 1 {
                let mut next = Vec::new();
                for chunk in current.chunks(2) {
                    let left = &chunk[0];
                    let right = if chunk.len() > 1 { &chunk[1] } else { left };
                    next.push(compute_sha256(&format!("{}{}", left, right)));
                }
                current = next;
            }

            current
                .into_iter()
                .next()
                .unwrap_or_else(|| compute_sha256("empty"))
        }

        /// Compress state values using simple run-length encoding.
        fn compress_state(state: &HashMap<String, Vec<u8>>) -> HashMap<String, Vec<u8>> {
            state
                .iter()
                .map(|(k, v)| {
                    let compressed = rle_compress(v);
                    (k.clone(), compressed)
                })
                .collect()
        }

        /// Decompress state values.
        fn decompress_state(state: &HashMap<String, Vec<u8>>) -> HashMap<String, Vec<u8>> {
            state
                .iter()
                .map(|(k, v)| {
                    let decompressed = rle_decompress(v);
                    (k.clone(), decompressed)
                })
                .collect()
        }
    }

    impl Default for SnapshotManager {
        fn default() -> Self {
            Self::new(SnapshotConfig::default())
        }
    }

    // ---------------------------------------------------------------------
    // Compression helpers (simple RLE for portability)
    // ---------------------------------------------------------------------

    fn rle_compress(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let current = data[i];
            let mut count = 1;

            while i + count < data.len() && data[i + count] == current && count < 255 {
                count += 1;
            }

            result.push(current);
            result.push(count as u8);
            i += count;
        }

        result
    }

    fn rle_decompress(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();

        for chunk in data.chunks(2) {
            if chunk.len() < 2 {
                break;
            }
            let value = chunk[0];
            let count = chunk[1] as usize;
            result.extend(std::iter::repeat_n(value, count));
        }

        result
    }

    // ---------------------------------------------------------------------
    // Hash helper
    // ---------------------------------------------------------------------

    fn compute_sha256(input: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // ---------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> SnapshotConfig {
            SnapshotConfig {
                max_snapshots: 10,
                enable_compression: true,
                enable_merkle_verification: true,
                snapshot_ttl_ms: 0,
            }
        }

        #[test]
        fn test_manager_creation() {
            let manager = SnapshotManager::default();
            assert_eq!(manager.snapshot_count(), 0);
        }

        #[test]
        fn test_manager_with_config() {
            let config = make_config();
            let manager = SnapshotManager::new(config);
            assert_eq!(manager.snapshot_count(), 0);
        }

        #[test]
        fn test_create_snapshot() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            state.insert("key2".to_string(), vec![4, 5, 6]);

            let snapshot = manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            assert_eq!(snapshot.id, "snap1");
            assert_eq!(manager.snapshot_count(), 1);
        }

        #[test]
        fn test_create_snapshot_duplicate() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);

            manager
                .create_snapshot("snap1".to_string(), state.clone(), None)
                .unwrap();

            match manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap_err()
            {
                SnapshotError::SnapshotExists(id) => assert_eq!(id, "snap1"),
                e => panic!("Expected SnapshotExists, got {:?}", e),
            }
        }

        #[test]
        fn test_get_snapshot() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            let snapshot = manager.get_snapshot("snap1").unwrap();
            assert_eq!(snapshot.id, "snap1");
        }

        #[test]
        fn test_get_snapshot_not_found() {
            let manager = SnapshotManager::new(make_config());
            match manager.get_snapshot("missing").unwrap_err() {
                SnapshotError::SnapshotNotFound(id) => assert_eq!(id, "missing"),
                e => panic!("Expected SnapshotNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_verify_integrity() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            let valid = manager.verify_integrity("snap1").unwrap();
            assert!(valid);
        }

        #[test]
        fn test_restore() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            state.insert("key2".to_string(), vec![4, 5, 6]);
            manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            let restored = manager.restore("snap1").unwrap();
            assert_eq!(restored.get("key1"), Some(&vec![1, 2, 3]));
            assert_eq!(restored.get("key2"), Some(&vec![4, 5, 6]));
        }

        #[test]
        fn test_compute_diff_added() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state1 = HashMap::new();
            state1.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state1, None)
                .unwrap();

            let mut state2 = HashMap::new();
            state2.insert("key1".to_string(), vec![1, 2, 3]);
            state2.insert("key2".to_string(), vec![4, 5, 6]);
            manager
                .create_snapshot("snap2".to_string(), state2, Some("snap1".to_string()))
                .unwrap();

            let diffs = manager.compute_diff("snap1", "snap2").unwrap();
            assert_eq!(diffs.len(), 1);
            assert_eq!(diffs[0].key, "key2");
            assert!(diffs[0].old_value.is_none());
            assert!(diffs[0].new_value.is_some());
        }

        #[test]
        fn test_compute_diff_modified() {
            let config = SnapshotConfig {
                max_snapshots: 10,
                enable_compression: false,
                enable_merkle_verification: true,
                snapshot_ttl_ms: 0,
            };
            let mut manager = SnapshotManager::new(config);
            let mut state1 = HashMap::new();
            state1.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state1, None)
                .unwrap();

            let mut state2 = HashMap::new();
            state2.insert("key1".to_string(), vec![7, 8, 9]);
            manager
                .create_snapshot("snap2".to_string(), state2, Some("snap1".to_string()))
                .unwrap();

            let diffs = manager.compute_diff("snap1", "snap2").unwrap();
            assert_eq!(diffs.len(), 1);
            assert_eq!(diffs[0].key, "key1");
            assert_eq!(diffs[0].old_value, Some(vec![1, 2, 3]));
            assert_eq!(diffs[0].new_value, Some(vec![7, 8, 9]));
        }

        #[test]
        fn test_compute_diff_deleted() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state1 = HashMap::new();
            state1.insert("key1".to_string(), vec![1, 2, 3]);
            state1.insert("key2".to_string(), vec![4, 5, 6]);
            manager
                .create_snapshot("snap1".to_string(), state1, None)
                .unwrap();

            let mut state2 = HashMap::new();
            state2.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap2".to_string(), state2, Some("snap1".to_string()))
                .unwrap();

            let diffs = manager.compute_diff("snap1", "snap2").unwrap();
            assert_eq!(diffs.len(), 1);
            assert_eq!(diffs[0].key, "key2");
            assert!(diffs[0].old_value.is_some());
            assert!(diffs[0].new_value.is_none());
        }

        #[test]
        fn test_max_snapshots_eviction() {
            let config = SnapshotConfig {
                max_snapshots: 3,
                ..make_config()
            };
            let mut manager = SnapshotManager::new(config);

            for i in 0..5 {
                let mut state = HashMap::new();
                state.insert(format!("key{}", i), vec![i as u8]);
                manager
                    .create_snapshot(format!("snap{}", i), state, None)
                    .unwrap();
            }

            assert_eq!(manager.snapshot_count(), 3);
            // Oldest snapshots should be evicted
            assert!(manager.get_snapshot("snap0").is_err());
            assert!(manager.get_snapshot("snap1").is_err());
            assert!(manager.get_snapshot("snap4").is_ok());
        }

        #[test]
        fn test_cleanup_expired() {
            let config = SnapshotConfig {
                snapshot_ttl_ms: 1000,
                ..make_config()
            };
            let mut manager = SnapshotManager::new(config);

            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            let current_ms = current_timestamp_ms() + 2000;
            let cleaned = manager.cleanup_expired(current_ms);
            assert_eq!(cleaned, 1);
            assert_eq!(manager.snapshot_count(), 0);
        }

        #[test]
        fn test_get_latest_snapshot() {
            let mut manager = SnapshotManager::new(make_config());

            for i in 0..3 {
                let mut state = HashMap::new();
                state.insert(format!("key{}", i), vec![i as u8]);
                manager
                    .create_snapshot(format!("snap{}", i), state, None)
                    .unwrap();
            }

            let latest = manager.get_latest_snapshot().unwrap();
            assert_eq!(latest.id, "snap2");
        }

        #[test]
        fn test_stats_recording() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            let stats = manager.get_stats();
            assert_eq!(stats.total_snapshots, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            state.insert("key1".to_string(), vec![1, 2, 3]);
            manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            manager.reset_stats();
            let stats = manager.get_stats();
            assert_eq!(stats.total_snapshots, 0);
        }

        #[test]
        fn test_compression_ratio() {
            let mut manager = SnapshotManager::new(make_config());
            let mut state = HashMap::new();
            // Highly compressible data
            state.insert("key1".to_string(), vec![0xAA; 100]);
            let snapshot = manager
                .create_snapshot("snap1".to_string(), state, None)
                .unwrap();

            assert!(snapshot.compression_ratio() < 1.0);
        }

        #[test]
        fn test_rle_roundtrip() {
            let data = vec![1, 1, 1, 2, 2, 3, 3, 3, 3];
            let compressed = rle_compress(&data);
            let decompressed = rle_decompress(&compressed);
            assert_eq!(data, decompressed);
        }

        #[test]
        fn test_error_display() {
            let err = SnapshotError::SnapshotNotFound("test".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("test"));
        }

        #[test]
        fn test_config_default() {
            let config = SnapshotConfig::default();
            assert!(config.enable_compression);
            assert!(config.enable_merkle_verification);
        }
    }
}

pub use internal::*;
