//! Checkpoint Manager — Gestión de checkpoints de entrenamiento distribuido
//!
//! Manela la creación, almacenamiento y restauración de checkpoints con
//! compresión, retención automática y recuperación ante fallos.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointError {
    NotFound(String),
    AlreadyExists(String),
    CorruptedData(String),
    StorageFull(usize),
    InvalidConfig(String),
    SerializationFailed(String),
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckpointError::NotFound(id) => write!(f, "Checkpoint not found: {}", id),
            CheckpointError::AlreadyExists(id) => write!(f, "Checkpoint already exists: {}", id),
            CheckpointError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
            CheckpointError::StorageFull(max_bytes) => {
                write!(f, "Storage full (max: {} bytes)", max_bytes)
            }
            CheckpointError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            CheckpointError::SerializationFailed(msg) => {
                write!(f, "Serialization failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for CheckpointError {}

// ============================================================================
// Checkpoint Type
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointType {
    Full,
    Incremental,
    Differential,
}

impl fmt::Display for CheckpointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckpointType::Full => write!(f, "Full"),
            CheckpointType::Incremental => write!(f, "Incremental"),
            CheckpointType::Differential => write!(f, "Differential"),
        }
    }
}

// ============================================================================
// Checkpoint Metadata
// ============================================================================

#[derive(Debug, Clone)]
pub struct CheckpointMeta {
    pub checkpoint_id: String,
    pub checkpoint_type: CheckpointType,
    pub epoch: usize,
    pub batch: usize,
    pub loss: f32,
    pub gradient_dim: usize,
    pub size_bytes: usize,
    pub created_at: Instant,
    pub parent_id: Option<String>,
    pub checksum: u64,
}

impl CheckpointMeta {
    pub fn new(checkpoint_id: String, checkpoint_type: CheckpointType, epoch: usize) -> Self {
        Self {
            checkpoint_id,
            checkpoint_type,
            epoch,
            batch: 0,
            loss: f32::MAX,
            gradient_dim: 0,
            size_bytes: 0,
            created_at: Instant::now(),
            parent_id: None,
            checksum: 0,
        }
    }

    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }
}

// ============================================================================
// Checkpoint Data
// ============================================================================

#[derive(Debug, Clone)]
pub struct CheckpointData {
    pub meta: CheckpointMeta,
    pub gradients: Vec<f32>,
    pub optimizer_state: HashMap<String, f32>,
    pub training_metrics: HashMap<String, f32>,
}

impl CheckpointData {
    pub fn new(meta: CheckpointMeta) -> Self {
        Self {
            meta,
            gradients: Vec::new(),
            optimizer_state: HashMap::new(),
            training_metrics: HashMap::new(),
        }
    }

    pub fn compute_checksum(&self) -> u64 {
        let mut hash: u64 = 5381;
        for g in &self.gradients {
            hash = hash.wrapping_mul(33).wrapping_add(*g as u64);
        }
        for (k, v) in &self.optimizer_state {
            hash = hash
                .wrapping_mul(37)
                .wrapping_add(k.as_bytes().iter().map(|b| *b as u64).sum::<u64>());
            hash = hash.wrapping_mul(41).wrapping_add(*v as u64);
        }
        hash
    }

    pub fn estimate_size(&self) -> usize {
        self.gradients.len() * std::mem::size_of::<f32>()
            + self.optimizer_state.len() * (32 + std::mem::size_of::<f32>())
            + self.training_metrics.len() * (32 + std::mem::size_of::<f32>())
    }
}

// ============================================================================
// Retention Policy
// ============================================================================

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub max_checkpoints: usize,
    pub max_age: Duration,
    pub max_total_bytes: usize,
    pub keep_latest: usize,
    pub keep_per_epoch: usize,
}

impl RetentionPolicy {
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            max_checkpoints,
            max_age: Duration::from_secs(3600),  // 1 hour
            max_total_bytes: 1024 * 1024 * 1024, // 1 GB
            keep_latest: 5,
            keep_per_epoch: 1,
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::new(50)
    }
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    pub checkpoint_dir: PathBuf,
    pub compression_enabled: bool,
    pub compression_ratio: f32,
    pub auto_checkpoint_interval: Duration,
    pub retention: RetentionPolicy,
}

impl CheckpointConfig {
    pub fn new(checkpoint_dir: PathBuf) -> Self {
        Self {
            checkpoint_dir,
            compression_enabled: true,
            compression_ratio: 0.5,
            auto_checkpoint_interval: Duration::from_secs(300), // 5 min
            retention: RetentionPolicy::default(),
        }
    }

    pub fn validate(&self) -> Result<(), CheckpointError> {
        if self.compression_ratio <= 0.0 || self.compression_ratio > 1.0 {
            return Err(CheckpointError::InvalidConfig(
                "Compression ratio must be between 0 and 1".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self::new(PathBuf::from("./checkpoints"))
    }
}

// ============================================================================
// Checkpoint Manager
// ============================================================================

pub struct CheckpointManager {
    config: CheckpointConfig,
    checkpoints: HashMap<String, CheckpointData>,
    epoch_index: HashMap<usize, Vec<String>>,
    total_bytes: usize,
    last_checkpoint_at: Option<Instant>,
}

impl CheckpointManager {
    pub fn new(config: CheckpointConfig) -> Self {
        Self {
            config,
            checkpoints: HashMap::new(),
            epoch_index: HashMap::new(),
            total_bytes: 0,
            last_checkpoint_at: None,
        }
    }

    // ------------------------------------------------------------------
    // Checkpoint Creation
    // ------------------------------------------------------------------

    pub fn create_checkpoint(
        &mut self,
        checkpoint_id: String,
        checkpoint_type: CheckpointType,
        epoch: usize,
        gradients: Vec<f32>,
        loss: f32,
    ) -> Result<CheckpointMeta, CheckpointError> {
        if self.checkpoints.contains_key(&checkpoint_id) {
            return Err(CheckpointError::AlreadyExists(checkpoint_id.clone()));
        }

        let size = gradients.len() * std::mem::size_of::<f32>();
        if self.total_bytes + size > self.config.retention.max_total_bytes {
            self.apply_retention()?;
        }

        let meta = CheckpointMeta {
            checkpoint_id: checkpoint_id.clone(),
            checkpoint_type,
            epoch,
            batch: 0,
            loss,
            gradient_dim: gradients.len(),
            size_bytes: size,
            created_at: Instant::now(),
            parent_id: None,
            checksum: 0,
        };

        let mut data = CheckpointData::new(meta.clone());
        data.gradients = gradients;
        data.meta.checksum = data.compute_checksum();
        data.meta.size_bytes = data.estimate_size();

        self.total_bytes += data.meta.size_bytes;
        self.epoch_index
            .entry(epoch)
            .or_default()
            .push(checkpoint_id.clone());
        self.checkpoints.insert(checkpoint_id, data);
        self.last_checkpoint_at = Some(Instant::now());

        Ok(meta)
    }

    pub fn create_incremental_checkpoint(
        &mut self,
        checkpoint_id: String,
        epoch: usize,
        delta_gradients: Vec<f32>,
        parent_id: String,
        loss: f32,
    ) -> Result<CheckpointMeta, CheckpointError> {
        if !self.checkpoints.contains_key(&parent_id) {
            return Err(CheckpointError::NotFound(parent_id.clone()));
        }

        let meta = self.create_checkpoint(
            checkpoint_id,
            CheckpointType::Incremental,
            epoch,
            delta_gradients,
            loss,
        )?;

        // Update parent reference
        if let Some(data) = self.checkpoints.get_mut(&meta.checkpoint_id) {
            data.meta.parent_id = Some(parent_id);
        }

        Ok(meta)
    }

    // ------------------------------------------------------------------
    // Checkpoint Retrieval
    // ------------------------------------------------------------------

    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Result<&CheckpointData, CheckpointError> {
        self.checkpoints
            .get(checkpoint_id)
            .ok_or(CheckpointError::NotFound(checkpoint_id.to_string()))
    }

    pub fn get_latest_for_epoch(&self, epoch: usize) -> Option<&CheckpointData> {
        self.epoch_index
            .get(&epoch)
            .and_then(|ids| ids.last())
            .and_then(|id| self.checkpoints.get(id))
    }

    pub fn get_latest_checkpoint(&self) -> Option<&CheckpointData> {
        self.checkpoints.values().max_by_key(|d| d.meta.epoch)
    }

    pub fn get_checkpoints_by_epoch(&self, epoch: usize) -> Vec<&CheckpointData> {
        self.epoch_index
            .get(&epoch)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.checkpoints.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_checkpoint_chain(&self, checkpoint_id: &str) -> Vec<&CheckpointData> {
        let mut chain = Vec::new();
        let mut current_id = Some(checkpoint_id.to_string());

        while let Some(id) = current_id {
            if let Some(data) = self.checkpoints.get(&id) {
                chain.push(data);
                current_id = data.meta.parent_id.clone();
            } else {
                break;
            }
        }

        chain.reverse();
        chain
    }

    // ------------------------------------------------------------------
    // Checkpoint Restoration
    // ------------------------------------------------------------------

    pub fn restore_gradients(&self, checkpoint_id: &str) -> Result<Vec<f32>, CheckpointError> {
        let data = self.get_checkpoint(checkpoint_id)?;

        // For incremental checkpoints, reconstruct full gradients
        if let Some(parent_id) = &data.meta.parent_id {
            let parent_gradients = self.restore_gradients(parent_id)?;
            let mut restored = parent_gradients;
            for (i, delta) in data.gradients.iter().enumerate() {
                if i < restored.len() {
                    restored[i] += delta;
                } else {
                    restored.push(*delta);
                }
            }
            Ok(restored)
        } else {
            Ok(data.gradients.clone())
        }
    }

    pub fn verify_checkpoint(&self, checkpoint_id: &str) -> Result<bool, CheckpointError> {
        let data = self.get_checkpoint(checkpoint_id)?;
        let computed_checksum = data.compute_checksum();
        Ok(computed_checksum == data.meta.checksum)
    }

    // ------------------------------------------------------------------
    // Retention & Cleanup
    // ------------------------------------------------------------------

    pub fn apply_retention(&mut self) -> Result<usize, CheckpointError> {
        let mut removed = 0;

        // Remove expired checkpoints
        let expired: Vec<String> = self
            .checkpoints
            .values()
            .filter(|d| d.meta.is_expired(self.config.retention.max_age))
            .map(|d| d.meta.checkpoint_id.clone())
            .collect();

        for id in &expired {
            self.remove_checkpoint(id)?;
            removed += 1;
        }

        // Enforce max checkpoints
        while self.checkpoints.len() > self.config.retention.max_checkpoints {
            let oldest = self.find_oldest_checkpoint();
            if let Some(id) = oldest {
                self.remove_checkpoint(&id)?;
                removed += 1;
            } else {
                break;
            }
        }

        Ok(removed)
    }

    pub fn remove_checkpoint(&mut self, checkpoint_id: &str) -> Result<(), CheckpointError> {
        if let Some(data) = self.checkpoints.remove(checkpoint_id) {
            self.total_bytes -= data.meta.size_bytes;

            // Update epoch index
            if let Some(ids) = self.epoch_index.get_mut(&data.meta.epoch) {
                ids.retain(|id| id != checkpoint_id);
            }

            Ok(())
        } else {
            Err(CheckpointError::NotFound(checkpoint_id.to_string()))
        }
    }

    pub fn prune_epoch(&mut self, epoch: usize, keep: usize) -> Result<usize, CheckpointError> {
        if let Some(ids) = self.epoch_index.get(&epoch) {
            let to_remove: Vec<String> = ids.iter().rev().skip(keep).cloned().collect();

            let mut removed = 0;
            for id in to_remove {
                self.remove_checkpoint(&id)?;
                removed += 1;
            }
            Ok(removed)
        } else {
            Ok(0)
        }
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn total_storage_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn get_checkpoints_since(&self, since: Instant) -> Vec<&CheckpointData> {
        self.checkpoints
            .values()
            .filter(|d| d.meta.created_at > since)
            .collect()
    }

    pub fn get_epoch_range(&self) -> Option<(usize, usize)> {
        let epochs: Vec<usize> = self.epoch_index.keys().cloned().collect();
        if epochs.is_empty() {
            None
        } else {
            Some((*epochs.iter().min().unwrap(), *epochs.iter().max().unwrap()))
        }
    }

    pub fn should_auto_checkpoint(&self) -> bool {
        self.last_checkpoint_at
            .map(|t| t.elapsed() > self.config.auto_checkpoint_interval)
            .unwrap_or(true)
    }

    fn find_oldest_checkpoint(&self) -> Option<String> {
        self.checkpoints
            .values()
            .min_by_key(|d| d.meta.created_at.elapsed().as_secs())
            .map(|d| d.meta.checkpoint_id.clone())
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new(CheckpointConfig::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gradients(dim: usize, value: f32) -> Vec<f32> {
        vec![value; dim]
    }

    #[test]
    fn test_manager_creation() {
        let manager = CheckpointManager::default();
        assert_eq!(manager.checkpoint_count(), 0);
        assert_eq!(manager.total_storage_bytes(), 0);
    }

    #[test]
    fn test_create_checkpoint() {
        let mut manager = CheckpointManager::default();
        let meta = manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(64, 0.5),
                0.3,
            )
            .unwrap();

        assert_eq!(meta.checkpoint_id, "cp-1");
        assert_eq!(meta.epoch, 1);
        assert_eq!(manager.checkpoint_count(), 1);
    }

    #[test]
    fn test_create_duplicate_checkpoint() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(32, 0.5),
                0.3,
            )
            .unwrap();

        match manager.create_checkpoint(
            "cp-1".to_string(),
            CheckpointType::Full,
            2,
            make_gradients(32, 0.5),
            0.2,
        ) {
            Err(CheckpointError::AlreadyExists(id)) => assert_eq!(id, "cp-1"),
            _ => panic!("Expected AlreadyExists"),
        }
    }

    #[test]
    fn test_get_checkpoint() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(32, 0.5),
                0.3,
            )
            .unwrap();

        let data = manager.get_checkpoint("cp-1").unwrap();
        assert_eq!(data.gradients.len(), 32);
    }

    #[test]
    fn test_get_nonexistent_checkpoint() {
        let manager = CheckpointManager::default();
        match manager.get_checkpoint("nonexistent") {
            Err(CheckpointError::NotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_incremental_checkpoint() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-base".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(4, 1.0),
                0.5,
            )
            .unwrap();

        manager
            .create_incremental_checkpoint(
                "cp-inc".to_string(),
                2,
                make_gradients(4, 0.5),
                "cp-base".to_string(),
                0.3,
            )
            .unwrap();

        // Restore should give [1.5, 1.5, 1.5, 1.5]
        let restored = manager.restore_gradients("cp-inc").unwrap();
        assert_eq!(restored, vec![1.5, 1.5, 1.5, 1.5]);
    }

    #[test]
    fn test_checkpoint_chain() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(2, 1.0),
                0.5,
            )
            .unwrap();
        manager
            .create_incremental_checkpoint(
                "cp-2".to_string(),
                2,
                make_gradients(2, 0.5),
                "cp-1".to_string(),
                0.4,
            )
            .unwrap();
        manager
            .create_incremental_checkpoint(
                "cp-3".to_string(),
                3,
                make_gradients(2, 0.25),
                "cp-2".to_string(),
                0.3,
            )
            .unwrap();

        let chain = manager.get_checkpoint_chain("cp-3");
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].meta.checkpoint_id, "cp-1");
        assert_eq!(chain[2].meta.checkpoint_id, "cp-3");
    }

    #[test]
    fn test_verify_checkpoint() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(16, 0.5),
                0.3,
            )
            .unwrap();

        assert!(manager.verify_checkpoint("cp-1").unwrap());
    }

    #[test]
    fn test_remove_checkpoint() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(16, 0.5),
                0.3,
            )
            .unwrap();

        assert!(manager.remove_checkpoint("cp-1").is_ok());
        assert_eq!(manager.checkpoint_count(), 0);
    }

    #[test]
    fn test_get_latest_for_epoch() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1a".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(8, 0.5),
                0.5,
            )
            .unwrap();
        manager
            .create_checkpoint(
                "cp-1b".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(8, 0.4),
                0.4,
            )
            .unwrap();

        let latest = manager.get_latest_for_epoch(1).unwrap();
        assert_eq!(latest.meta.checkpoint_id, "cp-1b");
    }

    #[test]
    fn test_get_latest_checkpoint() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(8, 0.5),
                0.5,
            )
            .unwrap();
        manager
            .create_checkpoint(
                "cp-3".to_string(),
                CheckpointType::Full,
                3,
                make_gradients(8, 0.3),
                0.3,
            )
            .unwrap();

        let latest = manager.get_latest_checkpoint().unwrap();
        assert_eq!(latest.meta.epoch, 3);
    }

    #[test]
    fn test_prune_epoch() {
        let mut manager = CheckpointManager::default();
        for i in 0..5 {
            manager
                .create_checkpoint(
                    format!("cp-{}", i),
                    CheckpointType::Full,
                    1,
                    make_gradients(8, 0.5),
                    0.5,
                )
                .unwrap();
        }

        let removed = manager.prune_epoch(1, 2).unwrap();
        assert_eq!(removed, 3);
        assert_eq!(manager.get_checkpoints_by_epoch(1).len(), 2);
    }

    #[test]
    fn test_epoch_range() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(8, 0.5),
                0.5,
            )
            .unwrap();
        manager
            .create_checkpoint(
                "cp-5".to_string(),
                CheckpointType::Full,
                5,
                make_gradients(8, 0.3),
                0.3,
            )
            .unwrap();

        let range = manager.get_epoch_range().unwrap();
        assert_eq!(range, (1, 5));
    }

    #[test]
    fn test_should_auto_checkpoint() {
        let mut config = CheckpointConfig::default();
        config.auto_checkpoint_interval = Duration::from_millis(100);
        let mut manager = CheckpointManager::new(config);

        assert!(manager.should_auto_checkpoint());

        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(8, 0.5),
                0.5,
            )
            .unwrap();

        assert!(!manager.should_auto_checkpoint());

        std::thread::sleep(Duration::from_millis(150));
        assert!(manager.should_auto_checkpoint());
    }

    #[test]
    fn test_config_validation() {
        let mut config = CheckpointConfig::default();
        assert!(config.validate().is_ok());

        config.compression_ratio = 0.0;
        match config.validate() {
            Err(CheckpointError::InvalidConfig(msg)) => {
                assert!(msg.contains("Compression ratio"));
            }
            _ => panic!("Expected InvalidConfig"),
        }
    }

    #[test]
    fn test_checkpoint_type_display() {
        assert_eq!(format!("{}", CheckpointType::Full), "Full");
        assert_eq!(format!("{}", CheckpointType::Incremental), "Incremental");
    }

    #[test]
    fn test_error_display() {
        match CheckpointError::NotFound("test".into()) {
            e => assert!(format!("{}", e).contains("test")),
            _ => {}
        }
    }

    #[test]
    fn test_checkpoints_since() {
        let mut manager = CheckpointManager::default();
        manager
            .create_checkpoint(
                "cp-1".to_string(),
                CheckpointType::Full,
                1,
                make_gradients(8, 0.5),
                0.5,
            )
            .unwrap();

        let since = Instant::now();
        std::thread::sleep(Duration::from_millis(10));

        manager
            .create_checkpoint(
                "cp-2".to_string(),
                CheckpointType::Full,
                2,
                make_gradients(8, 0.4),
                0.4,
            )
            .unwrap();

        let recent = manager.get_checkpoints_since(since);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].meta.checkpoint_id, "cp-2");
    }

    #[test]
    fn test_storage_bytes_tracking() {
        let mut manager = CheckpointManager::default();
        let gradients = make_gradients(100, 1.0);
        let expected_size = gradients.len() * std::mem::size_of::<f32>();

        manager
            .create_checkpoint("cp-1".to_string(), CheckpointType::Full, 1, gradients, 0.5)
            .unwrap();

        assert!(manager.total_storage_bytes() >= expected_size);
    }

    #[test]
    fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.max_checkpoints, 50);
    }

    #[test]
    fn test_config_default() {
        let config = CheckpointConfig::default();
        assert!(config.compression_enabled);
    }

    #[test]
    fn test_manager_default() {
        let manager = CheckpointManager::default();
        assert_eq!(manager.checkpoint_count(), 0);
    }

    #[test]
    fn test_empty_epoch_range() {
        let manager = CheckpointManager::default();
        assert!(manager.get_epoch_range().is_none());
    }

    #[test]
    fn test_checksum_computation() {
        let meta = CheckpointMeta::new("test".into(), CheckpointType::Full, 1);
        let data = CheckpointData::new(meta);
        let checksum = data.compute_checksum();
        assert!(checksum > 0);
    }
}
