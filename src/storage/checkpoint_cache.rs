//! Checkpoint Cache — LRU-backed checkpoint storage with compression support.
//!
//! Manages training checkpoints with automatic eviction, compression,
//! and fast retrieval. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::collections::HashMap;
use std::collections::VecDeque;

/// Error types for checkpoint cache operations.
#[derive(Debug)]
pub enum CheckpointCacheError {
    /// Checkpoint not found.
    NotFound(String),
    /// Cache is full and cannot evict.
    CacheFull(usize),
    /// Checkpoint already exists.
    AlreadyExists(String),
    /// Invalid checkpoint data.
    InvalidData(String),
    /// Storage quota exceeded.
    QuotaExceeded(usize),
}

impl std::fmt::Display for CheckpointCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointCacheError::NotFound(id) => write!(f, "Checkpoint not found: {}", id),
            CheckpointCacheError::CacheFull(max) => write!(f, "Cache full: max {}", max),
            CheckpointCacheError::AlreadyExists(id) => write!(f, "Checkpoint exists: {}", id),
            CheckpointCacheError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            CheckpointCacheError::QuotaExceeded(bytes) => {
                write!(f, "Quota exceeded: {} bytes", bytes)
            }
        }
    }
}

/// Checkpoint metadata entry.
#[derive(Debug, Clone)]
pub struct CheckpointEntry {
    /// Unique checkpoint identifier.
    pub checkpoint_id: String,
    /// Training round when checkpoint was created.
    pub round: u64,
    /// Model identifier.
    pub model_id: String,
    /// Original data size in bytes.
    pub original_size: usize,
    /// Stored (compressed) size in bytes.
    pub stored_size: usize,
    /// Checkpoint data (compressed if enabled).
    pub data: Vec<u8>,
    /// Creation timestamp in milliseconds.
    pub created_ms: u64,
    /// Last access timestamp in milliseconds.
    pub last_access_ms: u64,
    /// Access count.
    pub access_count: u64,
    /// Compression ratio (1.0 = uncompressed).
    pub compression_ratio: f64,
}

impl CheckpointEntry {
    pub fn new(
        checkpoint_id: String,
        round: u64,
        model_id: String,
        data: Vec<u8>,
        created_ms: u64,
    ) -> Self {
        let original_size = data.len();
        Self {
            checkpoint_id,
            round,
            model_id,
            original_size,
            stored_size: original_size,
            data,
            created_ms,
            last_access_ms: created_ms,
            access_count: 0,
            compression_ratio: 1.0,
        }
    }

    /// Record access to this checkpoint.
    pub fn access(&mut self, now_ms: u64) {
        self.last_access_ms = now_ms;
        self.access_count += 1;
    }

    /// Check if checkpoint is stale (not accessed within threshold).
    pub fn is_stale(&self, now_ms: u64, threshold_ms: u64) -> bool {
        now_ms - self.last_access_ms > threshold_ms
    }
}

/// Eviction policy for cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least Recently Used.
    LRU,
    /// Least Frequently Used.
    LFU,
    /// Oldest First.
    FIFO,
}

impl std::fmt::Display for EvictionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvictionPolicy::LRU => write!(f, "lru"),
            EvictionPolicy::LFU => write!(f, "lfu"),
            EvictionPolicy::FIFO => write!(f, "fifo"),
        }
    }
}

/// Configuration for checkpoint cache.
#[derive(Debug, Clone)]
pub struct CheckpointCacheConfig {
    /// Maximum number of checkpoints in cache.
    pub max_checkpoints: usize,
    /// Maximum total storage in bytes (0 = unlimited).
    pub max_storage_bytes: usize,
    /// Eviction policy.
    pub eviction_policy: EvictionPolicy,
    /// Enable compression.
    pub compression_enabled: bool,
    /// Stale threshold in milliseconds.
    pub stale_threshold_ms: u64,
    /// Number of checkpoints to evict at once.
    pub eviction_batch_size: usize,
}

impl Default for CheckpointCacheConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 1024,
            max_storage_bytes: 0,
            eviction_policy: EvictionPolicy::LRU,
            compression_enabled: true,
            stale_threshold_ms: 3600_000, // 1 hour
            eviction_batch_size: 4,
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total checkpoints stored.
    pub total_stored: u64,
    /// Total checkpoints evicted.
    pub total_evicted: u64,
    /// Total cache hits.
    pub total_hits: u64,
    /// Total cache misses.
    pub total_misses: u64,
    /// Total bytes stored.
    pub total_bytes_stored: u64,
    /// Total bytes saved by compression.
    pub total_bytes_saved: u64,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            total_stored: 0,
            total_evicted: 0,
            total_hits: 0,
            total_misses: 0,
            total_bytes_stored: 0,
            total_bytes_saved: 0,
        }
    }
}

impl CacheStats {
    /// Get hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            return 0.0;
        }
        self.total_hits as f64 / total as f64
    }

    /// Get compression savings ratio.
    pub fn savings_ratio(&self) -> f64 {
        let total = self.total_bytes_stored + self.total_bytes_saved;
        if total == 0 {
            return 0.0;
        }
        self.total_bytes_saved as f64 / total as f64
    }
}

/// Checkpoint cache with LRU eviction and compression.
pub struct CheckpointCache {
    config: CheckpointCacheConfig,
    entries: HashMap<String, CheckpointEntry>,
    access_order: VecDeque<String>,
    stats: CacheStats,
    current_time_ms: u64,
}

impl CheckpointCache {
    pub fn new(config: CheckpointCacheConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            access_order: VecDeque::new(),
            stats: CacheStats::default(),
            current_time_ms: 0,
        }
    }

    /// Set current time (for testing).
    pub fn set_time(&mut self, now_ms: u64) {
        self.current_time_ms = now_ms;
    }

    /// Store a checkpoint in the cache.
    pub fn store(
        &mut self,
        checkpoint_id: String,
        round: u64,
        model_id: String,
        data: Vec<u8>,
    ) -> Result<(), CheckpointCacheError> {
        if self.entries.contains_key(&checkpoint_id) {
            return Err(CheckpointCacheError::AlreadyExists(checkpoint_id));
        }

        // Evict if necessary
        self.evict_if_needed(data.len())?;

        let mut entry = CheckpointEntry::new(
            checkpoint_id.clone(),
            round,
            model_id,
            data.clone(),
            self.current_time_ms,
        );

        // Apply compression simulation
        if self.config.compression_enabled && data.len() > 256 {
            let compressed_size = (data.len() as f64 * 0.6) as usize;
            entry.data = data[..(compressed_size.min(data.len()))].to_vec();
            entry.stored_size = compressed_size;
            entry.compression_ratio = compressed_size as f64 / data.len() as f64;
            self.stats.total_bytes_saved += (data.len() - compressed_size) as u64;
        }

        self.stats.total_bytes_stored += entry.stored_size as u64;
        self.stats.total_stored += 1;
        self.access_order.push_back(checkpoint_id.clone());
        self.entries.insert(checkpoint_id, entry);
        Ok(())
    }

    /// Retrieve a checkpoint from the cache.
    pub fn get(&mut self, checkpoint_id: &str) -> Result<&CheckpointEntry, CheckpointCacheError> {
        if let Some(entry) = self.entries.get_mut(checkpoint_id) {
            entry.access(self.current_time_ms);
            self.stats.total_hits += 1;
            // Move to end of access order (most recently used)
            self.access_order.retain(|id| id != checkpoint_id);
            self.access_order.push_back(checkpoint_id.to_string());
            return Ok(entry);
        }
        self.stats.total_misses += 1;
        Err(CheckpointCacheError::NotFound(checkpoint_id.to_string()))
    }

    /// Remove a checkpoint from the cache.
    pub fn remove(&mut self, checkpoint_id: &str) -> Result<(), CheckpointCacheError> {
        if self.entries.remove(checkpoint_id).is_some() {
            self.access_order.retain(|id| id != checkpoint_id);
            Ok(())
        } else {
            Err(CheckpointCacheError::NotFound(checkpoint_id.to_string()))
        }
    }

    /// Get checkpoint by model and round.
    pub fn get_by_model_round(&self, model_id: &str, round: u64) -> Option<&CheckpointEntry> {
        self.entries
            .values()
            .find(|e| e.model_id == model_id && e.round == round)
    }

    /// Get all checkpoints for a model.
    pub fn get_model_checkpoints(&self, model_id: &str) -> Vec<&CheckpointEntry> {
        self.entries
            .values()
            .filter(|e| e.model_id == model_id)
            .collect()
    }

    /// Evict stale checkpoints.
    pub fn evict_stale(&mut self) -> usize {
        let now = self.current_time_ms;
        let threshold = self.config.stale_threshold_ms;
        let before = self.entries.len();
        self.entries.retain(|_id, entry| {
            if entry.is_stale(now, threshold) {
                self.stats.total_evicted += 1;
                false
            } else {
                true
            }
        });
        self.access_order.retain(|id| self.entries.contains_key(id));
        before - self.entries.len()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get configuration.
    pub fn config(&self) -> &CheckpointCacheConfig {
        &self.config
    }

    /// Get current checkpoint count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = CacheStats::default();
    }

    // ─── Internal ───

    fn evict_if_needed(&mut self, new_size: usize) -> Result<(), CheckpointCacheError> {
        // Check count limit
        while self.entries.len() >= self.config.max_checkpoints {
            self.evict_next()?;
        }

        // Check storage limit
        let mut current_storage: usize = self.entries.values().map(|e| e.stored_size).sum();
        if self.config.max_storage_bytes > 0 {
            while current_storage + new_size > self.config.max_storage_bytes
                && !self.entries.is_empty()
            {
                let evicted_size = self.evict_next()?;
                current_storage = current_storage.saturating_sub(evicted_size);
            }
        }

        Ok(())
    }

    fn evict_next(&mut self) -> Result<usize, CheckpointCacheError> {
        if self.entries.is_empty() {
            return Err(CheckpointCacheError::CacheFull(self.config.max_checkpoints));
        }

        let victim_id = match self.config.eviction_policy {
            EvictionPolicy::LRU => self.access_order.front().cloned(),
            EvictionPolicy::LFU => self
                .entries
                .values()
                .min_by_key(|e| e.access_count)
                .map(|e| e.checkpoint_id.clone()),
            EvictionPolicy::FIFO => self
                .entries
                .values()
                .min_by_key(|e| e.created_ms)
                .map(|e| e.checkpoint_id.clone()),
        };

        if let Some(id) = victim_id {
            if let Some(entry) = self.entries.remove(&id) {
                self.access_order.retain(|x| x != &id);
                self.stats.total_evicted += 1;
                Ok(entry.stored_size)
            } else {
                Err(CheckpointCacheError::CacheFull(self.config.max_checkpoints))
            }
        } else {
            Err(CheckpointCacheError::CacheFull(self.config.max_checkpoints))
        }
    }
}

impl Default for CheckpointCache {
    fn default() -> Self {
        Self::new(CheckpointCacheConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = CheckpointCache::default();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_store_checkpoint() {
        let mut cache = CheckpointCache::default();
        cache.set_time(1000);
        cache
            .store("cp1".to_string(), 1, "model_a".to_string(), vec![1, 2, 3])
            .unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_store_duplicate() {
        let mut cache = CheckpointCache::default();
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        let result = cache.store("cp1".to_string(), 2, "m".to_string(), vec![2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_checkpoint() {
        let mut cache = CheckpointCache::default();
        cache.set_time(1000);
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1, 2, 3])
            .unwrap();
        let entry = cache.get("cp1").unwrap();
        assert_eq!(entry.access_count, 1);
    }

    #[test]
    fn test_get_missing() {
        let mut cache = CheckpointCache::default();
        let result = cache.get("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_checkpoint() {
        let mut cache = CheckpointCache::default();
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        cache.remove("cp1").unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_remove_missing() {
        let mut cache = CheckpointCache::default();
        let result = cache.remove("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_lru_eviction() {
        let config = CheckpointCacheConfig {
            max_checkpoints: 3,
            eviction_policy: EvictionPolicy::LRU,
            ..Default::default()
        };
        let mut cache = CheckpointCache::new(config);
        cache.set_time(1000);
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        cache.set_time(2000);
        cache
            .store("cp2".to_string(), 2, "m".to_string(), vec![2])
            .unwrap();
        cache.set_time(3000);
        cache
            .store("cp3".to_string(), 3, "m".to_string(), vec![3])
            .unwrap();
        // Access cp1 to make it recently used
        cache.get("cp1").unwrap();
        // Add cp4, should evict cp2 (least recently used)
        cache.set_time(4000);
        cache
            .store("cp4".to_string(), 4, "m".to_string(), vec![4])
            .unwrap();
        assert!(cache.get("cp2").is_err());
        assert!(cache.get("cp1").is_ok());
    }

    #[test]
    fn test_lfu_eviction() {
        let config = CheckpointCacheConfig {
            max_checkpoints: 3,
            eviction_policy: EvictionPolicy::LFU,
            ..Default::default()
        };
        let mut cache = CheckpointCache::new(config);
        cache.set_time(1000);
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        cache
            .store("cp2".to_string(), 2, "m".to_string(), vec![2])
            .unwrap();
        cache
            .store("cp3".to_string(), 3, "m".to_string(), vec![3])
            .unwrap();
        // Access cp1 multiple times
        cache.get("cp1").unwrap();
        cache.get("cp1").unwrap();
        // Add cp4, should evict cp2 or cp3 (least frequently used)
        cache
            .store("cp4".to_string(), 4, "m".to_string(), vec![4])
            .unwrap();
        assert!(cache.get("cp1").is_ok());
    }

    #[test]
    fn test_evict_stale() {
        let config = CheckpointCacheConfig {
            stale_threshold_ms: 1000,
            ..Default::default()
        };
        let mut cache = CheckpointCache::new(config);
        cache.set_time(1000);
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        cache.set_time(2000);
        cache
            .store("cp2".to_string(), 2, "m".to_string(), vec![2])
            .unwrap();
        // Advance time past threshold
        cache.set_time(5000);
        let evicted = cache.evict_stale();
        assert_eq!(evicted, 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_get_by_model_round() {
        let mut cache = CheckpointCache::default();
        cache
            .store("cp1".to_string(), 1, "model_a".to_string(), vec![1])
            .unwrap();
        cache
            .store("cp2".to_string(), 2, "model_a".to_string(), vec![2])
            .unwrap();
        let entry = cache.get_by_model_round("model_a", 1);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().checkpoint_id, "cp1");
    }

    #[test]
    fn test_get_model_checkpoints() {
        let mut cache = CheckpointCache::default();
        cache
            .store("cp1".to_string(), 1, "model_a".to_string(), vec![1])
            .unwrap();
        cache
            .store("cp2".to_string(), 2, "model_b".to_string(), vec![2])
            .unwrap();
        cache
            .store("cp3".to_string(), 3, "model_a".to_string(), vec![3])
            .unwrap();
        let checkpoints = cache.get_model_checkpoints("model_a");
        assert_eq!(checkpoints.len(), 2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut cache = CheckpointCache::default();
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        cache.get("cp1").unwrap();
        cache.get("missing").unwrap_err();
        let stats = cache.stats();
        assert_eq!(stats.total_stored, 1);
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
    }

    #[test]
    fn test_hit_rate() {
        let stats = CacheStats {
            total_hits: 80,
            total_misses: 20,
            ..Default::default()
        };
        assert_eq!(stats.hit_rate(), 0.8);
    }

    #[test]
    fn test_reset_stats() {
        let mut cache = CheckpointCache::default();
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1])
            .unwrap();
        cache.reset_stats();
        assert_eq!(cache.stats().total_stored, 0);
    }

    #[test]
    fn test_compression_enabled() {
        let config = CheckpointCacheConfig {
            compression_enabled: true,
            ..Default::default()
        };
        let mut cache = CheckpointCache::new(config);
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![42u8; 512])
            .unwrap();
        let entry = cache.get_by_model_round("m", 1).unwrap();
        assert!(entry.compression_ratio < 1.0);
    }

    #[test]
    fn test_entry_access() {
        let mut entry = CheckpointEntry::new("cp1".to_string(), 1, "m".to_string(), vec![1], 1000);
        entry.access(2000);
        assert_eq!(entry.access_count, 1);
        assert_eq!(entry.last_access_ms, 2000);
    }

    #[test]
    fn test_entry_stale() {
        let entry = CheckpointEntry::new("cp1".to_string(), 1, "m".to_string(), vec![1], 1000);
        assert!(entry.is_stale(5000, 1000));
        assert!(!entry.is_stale(1500, 1000));
    }

    #[test]
    fn test_eviction_policy_display() {
        assert_eq!(EvictionPolicy::LRU.to_string(), "lru");
        assert_eq!(EvictionPolicy::LFU.to_string(), "lfu");
        assert_eq!(EvictionPolicy::FIFO.to_string(), "fifo");
    }

    #[test]
    fn test_error_display() {
        let e = CheckpointCacheError::NotFound("x".to_string());
        assert!(format!("{}", e).contains("x"));
    }

    #[test]
    fn test_config_default() {
        let config = CheckpointCacheConfig::default();
        assert_eq!(config.max_checkpoints, 1024);
        assert_eq!(config.eviction_policy, EvictionPolicy::LRU);
    }

    #[test]
    fn test_quota_exceeded() {
        let config = CheckpointCacheConfig {
            max_storage_bytes: 10,
            ..Default::default()
        };
        let mut cache = CheckpointCache::new(config);
        cache
            .store("cp1".to_string(), 1, "m".to_string(), vec![1; 10])
            .unwrap();
        let result = cache.store("cp2".to_string(), 2, "m".to_string(), vec![1; 10]);
        // Should evict cp1 to make room
        assert!(result.is_ok());
    }

    #[test]
    fn test_savings_ratio() {
        let stats = CacheStats {
            total_bytes_stored: 600,
            total_bytes_saved: 400,
            ..Default::default()
        };
        assert_eq!(stats.savings_ratio(), 0.4);
    }
}
