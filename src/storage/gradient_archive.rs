//! Gradient Archive — Compressed gradient history storage with versioning.
//!
//! Manages historical gradient data with compression, versioning,
//! and efficient retrieval for model alignment. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::collections::HashMap;

/// Error types for gradient archive operations.
#[derive(Debug)]
pub enum GradientArchiveError {
    /// Gradient version not found.
    NotFound(String),
    /// Archive is full.
    ArchiveFull(usize),
    /// Invalid gradient dimensions.
    InvalidDimensions(String),
    /// Version already exists.
    VersionExists(String),
    /// Corrupted archive data.
    CorruptedData(String),
}

impl std::fmt::Display for GradientArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GradientArchiveError::NotFound(id) => write!(f, "Gradient not found: {}", id),
            GradientArchiveError::ArchiveFull(max) => write!(f, "Archive full: max {}", max),
            GradientArchiveError::InvalidDimensions(msg) => write!(f, "Invalid dimensions: {}", msg),
            GradientArchiveError::VersionExists(id) => write!(f, "Version exists: {}", id),
            GradientArchiveError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
        }
    }
}

/// Gradient version entry.
#[derive(Debug, Clone)]
pub struct GradientVersion {
    /// Unique version identifier.
    pub version_id: String,
    /// Model identifier.
    pub model_id: String,
    /// Training round.
    pub round: u64,
    /// Gradient dimension.
    pub dimension: usize,
    /// Original gradient data.
    pub gradients: Vec<f32>,
    /// Compressed representation (simulated).
    pub compressed_size: usize,
    /// Original size in bytes.
    pub original_size: usize,
    /// Compression ratio.
    pub compression_ratio: f64,
    /// Checksum for integrity.
    pub checksum: u64,
    /// Creation timestamp in milliseconds.
    pub created_ms: u64,
    /// Associated tags for filtering.
    pub tags: Vec<String>,
}

impl GradientVersion {
    pub fn new(
        version_id: String,
        model_id: String,
        round: u64,
        gradients: Vec<f32>,
        created_ms: u64,
    ) -> Self {
        let original_size = gradients.len() * std::mem::size_of::<f32>();
        let checksum = compute_gradient_checksum(&gradients);
        let compressed_size = simulate_gradient_compression(&gradients);
        let compression_ratio = compressed_size as f64 / original_size as f64;
        Self {
            version_id,
            model_id,
            round,
            dimension: gradients.len(),
            gradients,
            compressed_size,
            original_size,
            compression_ratio,
            checksum,
            created_ms,
            tags: Vec::new(),
        }
    }

    /// Add a tag to this version.
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Compute L2 norm of gradients.
    pub fn l2_norm(&self) -> f32 {
        self.gradients.iter().map(|g| g * g).sum::<f32>().sqrt()
    }

    /// Compute mean absolute gradient.
    pub fn mean_abs(&self) -> f32 {
        if self.gradients.is_empty() {
            return 0.0;
        }
        self.gradients.iter().map(|g| g.abs()).sum::<f32>() / self.gradients.len() as f32
    }

    /// Verify integrity using checksum.
    pub fn verify_integrity(&self) -> bool {
        compute_gradient_checksum(&self.gradients) == self.checksum
    }
}

/// Archive configuration.
#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    /// Maximum versions per model.
    pub max_versions_per_model: usize,
    /// Maximum total versions.
    pub max_total_versions: usize,
    /// Enable compression.
    pub compression_enabled: bool,
    /// Auto-prune old versions when limit reached.
    pub auto_prune: bool,
    /// Minimum versions to keep per model.
    pub min_versions_keep: usize,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            max_versions_per_model: 500,
            max_total_versions: 10_000,
            compression_enabled: true,
            auto_prune: true,
            min_versions_keep: 10,
        }
    }
}

/// Archive statistics.
#[derive(Debug, Clone)]
pub struct ArchiveStats {
    /// Total versions stored.
    pub total_versions: u64,
    /// Total models tracked.
    pub total_models: u64,
    /// Total bytes stored (compressed).
    pub total_bytes_stored: u64,
    /// Total bytes saved by compression.
    pub total_bytes_saved: u64,
    /// Total pruned versions.
    pub total_pruned: u64,
    /// Total lookups.
    pub total_lookups: u64,
    /// Total lookup hits.
    pub total_lookup_hits: u64,
}

impl Default for ArchiveStats {
    fn default() -> Self {
        Self {
            total_versions: 0,
            total_models: 0,
            total_bytes_stored: 0,
            total_bytes_saved: 0,
            total_pruned: 0,
            total_lookups: 0,
            total_lookup_hits: 0,
        }
    }
}

impl ArchiveStats {
    /// Get lookup hit rate.
    pub fn lookup_hit_rate(&self) -> f64 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        self.total_lookup_hits as f64 / self.total_lookups as f64
    }

    /// Get overall compression ratio.
    pub fn compression_ratio(&self) -> f64 {
        let total = self.total_bytes_stored + self.total_bytes_saved;
        if total == 0 {
            return 0.0;
        }
        self.total_bytes_stored as f64 / total as f64
    }
}

/// Gradient archive with versioning and compression.
pub struct GradientArchive {
    config: ArchiveConfig,
    versions: HashMap<String, GradientVersion>,
    model_index: HashMap<String, Vec<String>>,
    stats: ArchiveStats,
    current_time_ms: u64,
}

impl GradientArchive {
    pub fn new(config: ArchiveConfig) -> Self {
        Self {
            config,
            versions: HashMap::new(),
            model_index: HashMap::new(),
            stats: ArchiveStats::default(),
            current_time_ms: 0,
        }
    }

    /// Set current time (for testing).
    pub fn set_time(&mut self, now_ms: u64) {
        self.current_time_ms = now_ms;
    }

    /// Store a new gradient version.
    pub fn store(
        &mut self,
        version_id: String,
        model_id: String,
        round: u64,
        gradients: Vec<f32>,
    ) -> Result<(), GradientArchiveError> {
        if self.versions.contains_key(&version_id) {
            return Err(GradientArchiveError::VersionExists(version_id));
        }

        if gradients.is_empty() {
            return Err(GradientArchiveError::InvalidDimensions(
                "Empty gradients".to_string(),
            ));
        }

        // Check total limit
        if self.versions.len() >= self.config.max_total_versions {
            if self.config.auto_prune {
                self.prune_oldest(1);
            } else {
                return Err(GradientArchiveError::ArchiveFull(
                    self.config.max_total_versions,
                ));
            }
        }

        // Check per-model limit
        let model_count = self
            .model_index
            .get(&model_id)
            .map(|v| v.len())
            .unwrap_or(0);
        if model_count >= self.config.max_versions_per_model {
            if self.config.auto_prune {
                self.prune_oldest(1);
            } else {
                return Err(GradientArchiveError::ArchiveFull(
                    self.config.max_versions_per_model,
                ));
            }
        }

        let version = GradientVersion::new(
            version_id.clone(),
            model_id.clone(),
            round,
            gradients,
            self.current_time_ms,
        );

        // Track storage stats
        self.stats.total_bytes_stored += version.compressed_size as u64;
        self.stats.total_bytes_saved += (version.original_size - version.compressed_size) as u64;
        self.stats.total_versions += 1;
        self.stats.total_models = self.model_index.len() as u64;

        self.versions.insert(version_id.clone(), version);
        self.model_index
            .entry(model_id)
            .or_insert_with(Vec::new)
            .push(version_id);
        Ok(())
    }

    /// Retrieve a gradient version.
    pub fn get(&mut self, version_id: &str) -> Option<&GradientVersion> {
        self.stats.total_lookups += 1;
        if self.versions.contains_key(version_id) {
            self.stats.total_lookup_hits += 1;
        }
        self.versions.get(version_id)
    }

    /// Get the latest version for a model.
    pub fn get_latest(&self, model_id: &str) -> Option<&GradientVersion> {
        if let Some(version_ids) = self.model_index.get(model_id) {
            if let Some(last_id) = version_ids.last() {
                return self.versions.get(last_id);
            }
        }
        None
    }

    /// Get versions by tag.
    pub fn get_by_tag(&self, tag: &str) -> Vec<&GradientVersion> {
        self.versions
            .values()
            .filter(|v| v.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get all versions for a model.
    pub fn get_model_versions(&self, model_id: &str) -> Vec<&GradientVersion> {
        if let Some(version_ids) = self.model_index.get(model_id) {
            version_ids
                .iter()
                .filter_map(|id| self.versions.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Compute gradient difference between two versions.
    pub fn compute_diff(
        &self,
        version_a: &str,
        version_b: &str,
    ) -> Result<Vec<f32>, GradientArchiveError> {
        let a = self
            .versions
            .get(version_a)
            .ok_or(GradientArchiveError::NotFound(version_a.to_string()))?;
        let b = self
            .versions
            .get(version_b)
            .ok_or(GradientArchiveError::NotFound(version_b.to_string()))?;

        if a.dimension != b.dimension {
            return Err(GradientArchiveError::InvalidDimensions(format!(
                "Dimension mismatch: {} vs {}",
                a.dimension,
                b.dimension
            )));
        }

        Ok(a.gradients
            .iter()
            .zip(b.gradients.iter())
            .map(|(x, y)| x - y)
            .collect())
    }

    /// Compute average gradients across multiple versions.
    pub fn compute_average(&self, version_ids: &[&str]) -> Result<Vec<f32>, GradientArchiveError> {
        if version_ids.is_empty() {
            return Err(GradientArchiveError::InvalidDimensions(
                "No versions specified".to_string(),
            ));
        }

        let mut gradients: Option<&GradientVersion> = None;
        for id in version_ids {
            let v = self
                .versions
                .get(*id)
                .ok_or(GradientArchiveError::NotFound(id.to_string()))?;
            if gradients.is_none() {
                gradients = Some(v);
            }
        }

        let base = gradients.unwrap();
        let mut sum: Vec<f32> = vec![0.0; base.dimension];
        let count = version_ids.len() as f32;

        for id in version_ids {
            let v = self.versions.get(*id).unwrap();
            for (i, g) in v.gradients.iter().enumerate() {
                sum[i] += g;
            }
        }

        Ok(sum.iter().map(|s| s / count).collect())
    }

    /// Prune oldest versions across all models.
    pub fn prune_oldest(&mut self, count: usize) -> usize {
        let mut candidates: Vec<(String, u64)> = self
            .versions
            .iter()
            .map(|(id, v)| (id.clone(), v.created_ms))
            .collect();
        candidates.sort_by_key(|(_, ts)| *ts);
        let to_prune = count.min(candidates.len());
        let mut pruned = 0;
        for (id_str, _) in &candidates[..to_prune] {
            if let Some(version) = self.versions.remove(id_str) {
                // Remove from model index
                if let Some(ids) = self.model_index.get_mut(&version.model_id) {
                    ids.retain(|x| x != id_str);
                }
                self.stats.total_pruned += 1;
                self.stats.total_versions = self.stats.total_versions.saturating_sub(1);
                pruned += 1;
            }
        }
        pruned
    }

    /// Get archive statistics.
    pub fn stats(&self) -> &ArchiveStats {
        &self.stats
    }

    /// Get configuration.
    pub fn config(&self) -> &ArchiveConfig {
        &self.config
    }

    /// Get total version count.
    pub fn len(&self) -> usize {
        self.versions.len()
    }

    /// Check if archive is empty.
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Get tracked model count.
    pub fn model_count(&self) -> usize {
        self.model_index.len()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ArchiveStats::default();
    }
}

impl Default for GradientArchive {
    fn default() -> Self {
        Self::new(ArchiveConfig::default())
    }
}

// ─── Helpers ───

fn compute_gradient_checksum(gradients: &[f32]) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &g in gradients {
        hash ^= g.to_bits() as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn simulate_gradient_compression(gradients: &[f32]) -> usize {
    let original = gradients.len() * std::mem::size_of::<f32>();
    // Simulate ~60% compression for gradient data
    (original as f64 * 0.6) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_creation() {
        let archive = GradientArchive::default();
        assert!(archive.is_empty());
        assert_eq!(archive.len(), 0);
    }

    #[test]
    fn test_store_gradient() {
        let mut archive = GradientArchive::default();
        archive.set_time(1000);
        archive
            .store("v1".to_string(), "model_a".to_string(), 1, vec![1.0, 2.0, 3.0])
            .unwrap();
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn test_store_duplicate() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        let result = archive.store("v1".to_string(), "m".to_string(), 2, vec![2.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_store_empty_gradients() {
        let mut archive = GradientArchive::default();
        let result = archive.store("v1".to_string(), "m".to_string(), 1, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_gradient() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0, 2.0])
            .unwrap();
        let version = archive.get("v1");
        assert!(version.is_some());
        assert_eq!(version.unwrap().dimension, 2);
    }

    #[test]
    fn test_get_missing() {
        let mut archive = GradientArchive::default();
        assert!(archive.get("missing").is_none());
    }

    #[test]
    fn test_get_latest() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![2.0])
            .unwrap();
        let latest = archive.get_latest("m");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version_id, "v2");
    }

    #[test]
    fn test_add_tag() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        let v = archive.versions.get_mut("v1").unwrap();
        v.add_tag("important".to_string());
        assert!(v.tags.contains(&"important".to_string()));
    }

    #[test]
    fn test_get_by_tag() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![2.0])
            .unwrap();
        let v1 = archive.versions.get_mut("v1").unwrap();
        v1.add_tag("tagged".to_string());
        let tagged = archive.get_by_tag("tagged");
        assert_eq!(tagged.len(), 1);
    }

    #[test]
    fn test_compute_diff() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0, 2.0, 3.0])
            .unwrap();
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![4.0, 5.0, 6.0])
            .unwrap();
        let diff = archive.compute_diff("v1", "v2").unwrap();
        assert_eq!(diff, vec![-3.0, -3.0, -3.0]);
    }

    #[test]
    fn test_compute_diff_dimension_mismatch() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0, 2.0])
            .unwrap();
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![1.0, 2.0, 3.0])
            .unwrap();
        let result = archive.compute_diff("v1", "v2");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_average() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![2.0, 4.0])
            .unwrap();
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![4.0, 8.0])
            .unwrap();
        let avg = archive.compute_average(&["v1", "v2"]).unwrap();
        assert_eq!(avg, vec![3.0, 6.0]);
    }

    #[test]
    fn test_l2_norm() {
        let v = GradientVersion::new("v1".to_string(), "m".to_string(), 1, vec![3.0, 4.0], 0);
        assert!((v.l2_norm() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_mean_abs() {
        let v = GradientVersion::new(
            "v1".to_string(),
            "m".to_string(),
            1,
            vec![-1.0, 2.0, -3.0],
            0,
        );
        assert!((v.mean_abs() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_verify_integrity() {
        let v = GradientVersion::new("v1".to_string(), "m".to_string(), 1, vec![1.0, 2.0], 0);
        assert!(v.verify_integrity());
    }

    #[test]
    fn test_prune_oldest() {
        let mut archive = GradientArchive::default();
        archive.set_time(1000);
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        archive.set_time(2000);
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![2.0])
            .unwrap();
        archive.set_time(3000);
        archive
            .store("v3".to_string(), "m".to_string(), 3, vec![3.0])
            .unwrap();
        let pruned = archive.prune_oldest(2);
        assert_eq!(pruned, 2);
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn test_auto_prune() {
        let config = ArchiveConfig {
            max_total_versions: 3,
            auto_prune: true,
            ..Default::default()
        };
        let mut archive = GradientArchive::new(config);
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        archive
            .store("v2".to_string(), "m".to_string(), 2, vec![2.0])
            .unwrap();
        archive
            .store("v3".to_string(), "m".to_string(), 3, vec![3.0])
            .unwrap();
        // This should trigger auto-prune
        archive
            .store("v4".to_string(), "m".to_string(), 4, vec![4.0])
            .unwrap();
        assert_eq!(archive.len(), 3);
        assert_eq!(archive.stats().total_pruned, 1);
    }

    #[test]
    fn test_model_count() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "model_a".to_string(), 1, vec![1.0])
            .unwrap();
        archive
            .store("v2".to_string(), "model_b".to_string(), 1, vec![2.0])
            .unwrap();
        assert_eq!(archive.model_count(), 2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0, 2.0])
            .unwrap();
        archive.get("v1");
        archive.get("missing");
        let stats = archive.stats();
        assert_eq!(stats.total_versions, 1);
        assert_eq!(stats.total_lookups, 2);
        assert_eq!(stats.total_lookup_hits, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "m".to_string(), 1, vec![1.0])
            .unwrap();
        archive.reset_stats();
        assert_eq!(archive.stats().total_versions, 0);
    }

    #[test]
    fn test_lookup_hit_rate() {
        let stats = ArchiveStats {
            total_lookups: 100,
            total_lookup_hits: 80,
            ..Default::default()
        };
        assert_eq!(stats.lookup_hit_rate(), 0.8);
    }

    #[test]
    fn test_compression_ratio_stats() {
        let stats = ArchiveStats {
            total_bytes_stored: 600,
            total_bytes_saved: 400,
            ..Default::default()
        };
        assert_eq!(stats.compression_ratio(), 0.6);
    }

    #[test]
    fn test_config_default() {
        let config = ArchiveConfig::default();
        assert!(config.compression_enabled);
        assert!(config.auto_prune);
        assert_eq!(config.max_versions_per_model, 500);
    }

    #[test]
    fn test_error_display() {
        let e = GradientArchiveError::NotFound("x".to_string());
        assert!(format!("{}", e).contains("x"));
    }

    #[test]
    fn test_get_model_versions() {
        let mut archive = GradientArchive::default();
        archive
            .store("v1".to_string(), "model_a".to_string(), 1, vec![1.0])
            .unwrap();
        archive
            .store("v2".to_string(), "model_b".to_string(), 1, vec![2.0])
            .unwrap();
        archive
            .store("v3".to_string(), "model_a".to_string(), 2, vec![3.0])
            .unwrap();
        let versions = archive.get_model_versions("model_a");
        assert_eq!(versions.len(), 2);
    }
}
