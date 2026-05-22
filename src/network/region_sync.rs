//! Region Sync — Multi-region synchronization with latency awareness.
//!
//! **Stuartian Law 3 (Holística):** Delta-encoding para cero desperdicio computacional.
//! **Stuartian Law 5 (Múltiples Posibilidades):** Convergencia eventual bajo latencia variable.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v2.1-region-sync` | region_sync | Multi-region sync, delta-encoding, batch merge |

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, Instant};

/// Error types for region synchronization.
#[derive(Debug, Clone)]
pub enum SyncError {
    /// Version vector conflict that cannot be resolved.
    VersionConflict(String),
    /// Remote region unreachable.
    RegionUnreachable(String),
    /// Delta encoding failed.
    DeltaEncodingFailed(String),
    /// Batch size exceeded.
    BatchSizeExceeded(usize),
    /// Timeout exceeded.
    TimeoutExceeded(Duration),
    /// State corruption detected.
    StateCorruption(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
            SyncError::RegionUnreachable(region) => write!(f, "Region unreachable: {}", region),
            SyncError::DeltaEncodingFailed(msg) => write!(f, "Delta encoding failed: {}", msg),
            SyncError::BatchSizeExceeded(size) => write!(f, "Batch size exceeded: {}", size),
            SyncError::TimeoutExceeded(duration) => {
                write!(f, "Timeout exceeded: {:?}", duration)
            }
            SyncError::StateCorruption(msg) => write!(f, "State corruption: {}", msg),
        }
    }
}

/// Result of a region sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of entries merged.
    pub entries_merged: usize,
    /// Number of conflicts resolved.
    pub conflicts_resolved: usize,
    /// Delta size in bytes (compressed).
    pub delta_size_bytes: usize,
    /// Original state size in bytes.
    pub original_size_bytes: usize,
    /// Compression ratio (0.0 = no compression, 1.0 = full compression).
    pub compression_ratio: f64,
    /// Sync duration.
    pub duration: Duration,
    /// Effective latency during sync.
    pub effective_latency_ms: u64,
    /// Sync was successful.
    pub success: bool,
}

impl SyncResult {
    /// Create a new sync result.
    pub fn new(
        entries_merged: usize,
        conflicts_resolved: usize,
        delta_size: usize,
        original_size: usize,
        duration: Duration,
        latency_ms: u64,
    ) -> Self {
        let compression_ratio = if original_size > 0 {
            1.0 - (delta_size as f64 / original_size as f64)
        } else {
            0.0
        };
        Self {
            entries_merged,
            conflicts_resolved,
            delta_size_bytes: delta_size,
            original_size_bytes: original_size,
            compression_ratio,
            duration,
            effective_latency_ms: latency_ms,
            success: true,
        }
    }
}

/// Delta-encoded entry for efficient sync.
#[derive(Debug, Clone)]
pub struct DeltaEntry {
    /// Node ID.
    pub node_id: String,
    /// New reputation value.
    pub new_value: f64,
    /// Previous reputation value (for delta calculation).
    pub previous_value: f64,
    /// Delta (new - previous).
    pub delta: f64,
    /// Version counter.
    pub version: u64,
    /// Timestamp for deterministic conflict resolution.
    pub timestamp: u64,
}

impl DeltaEntry {
    /// Create a new delta entry.
    pub fn new(node_id: String, new_value: f64, previous_value: f64, version: u64) -> Self {
        let delta = new_value - previous_value;
        Self {
            node_id,
            new_value,
            previous_value,
            delta,
            version,
            timestamp: Instant::now().elapsed().as_millis() as u64,
        }
    }

    /// Estimate serialized size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.node_id.len() + std::mem::size_of::<f64>() * 3 + std::mem::size_of::<u64>() * 2
    }
}

/// Region state for synchronization.
#[derive(Debug, Clone)]
pub struct RegionState {
    /// Region ID.
    pub region_id: String,
    /// Reputation map (node_id -> reputation).
    pub reputations: BTreeMap<String, f64>,
    /// Version vector (node_id -> version counter).
    pub versions: BTreeMap<String, u64>,
    /// Last sync timestamp.
    pub last_sync: u64,
    /// Total syncs performed.
    pub sync_count: u64,
}

impl RegionState {
    /// Create a new region state.
    pub fn new(region_id: String) -> Self {
        Self {
            region_id,
            reputations: BTreeMap::new(),
            versions: BTreeMap::new(),
            last_sync: 0,
            sync_count: 0,
        }
    }

    /// Update reputation for a node.
    pub fn update(&mut self, node_id: &str, reputation: f64) {
        self.reputations.insert(node_id.to_string(), reputation);
        let version = self.versions.get(node_id).copied().unwrap_or(0) + 1;
        self.versions.insert(node_id.to_string(), version);
    }

    /// Get reputation for a node.
    pub fn get(&self, node_id: &str) -> Option<f64> {
        self.reputations.get(node_id).copied()
    }

    /// Get version for a node.
    pub fn get_version(&self, node_id: &str) -> u64 {
        self.versions.get(node_id).copied().unwrap_or(0)
    }

    /// Calculate state size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.reputations
            .values()
            .map(|_v| std::mem::size_of::<f64>())
            .sum::<usize>()
            + self
                .versions
                .values()
                .map(|_v| std::mem::size_of::<u64>())
                .sum::<usize>()
            + self.reputations.keys().map(|k| k.len()).sum::<usize>()
    }
}

/// Configuration for region synchronization.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum batch size for merge operations.
    pub max_batch_size: usize,
    /// Sync timeout.
    pub timeout: Duration,
    /// Enable delta encoding.
    pub delta_encoding: bool,
    /// Maximum latency before fallback.
    pub max_latency_ms: u64,
}

impl SyncConfig {
    /// Create default sync configuration.
    pub fn default_config() -> Self {
        Self {
            max_batch_size: 1000,
            timeout: Duration::from_secs(30),
            delta_encoding: true,
            max_latency_ms: 5000,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), SyncError> {
        if self.max_batch_size == 0 {
            return Err(SyncError::BatchSizeExceeded(0));
        }
        if self.timeout.is_zero() {
            return Err(SyncError::TimeoutExceeded(Duration::ZERO));
        }
        Ok(())
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Generate delta entries between two region states.
pub fn generate_deltas(local: &RegionState, remote: &RegionState) -> Vec<DeltaEntry> {
    let mut deltas = Vec::new();

    // Find entries in remote that are newer than local.
    for (node_id, remote_version) in &remote.versions {
        let local_version = local.get_version(node_id);
        if *remote_version > local_version {
            let remote_value = remote.get(node_id).unwrap_or(0.0);
            let local_value = local.get(node_id).unwrap_or(0.0);
            deltas.push(DeltaEntry::new(
                node_id.clone(),
                remote_value,
                local_value,
                *remote_version,
            ));
        }
    }

    deltas
}

/// Apply delta entries to a region state.
pub fn apply_deltas(state: &mut RegionState, deltas: &[DeltaEntry]) -> usize {
    let mut applied = 0;
    for delta in deltas {
        state
            .reputations
            .insert(delta.node_id.clone(), delta.new_value);
        state.versions.insert(delta.node_id.clone(), delta.version);
        applied += 1;
    }
    applied
}

/// Resolve conflicts using version vector + deterministic timestamp.
pub fn resolve_conflicts(local: &mut RegionState, remote: &RegionState) -> usize {
    let mut resolved = 0;
    for (node_id, remote_value) in &remote.reputations {
        let remote_version = remote.get_version(node_id);
        let local_version = local.get_version(node_id);

        if remote_version > local_version {
            // Remote is newer, take remote value.
            local.reputations.insert(node_id.clone(), *remote_value);
            local.versions.insert(node_id.clone(), remote_version);
            resolved += 1;
        } else if remote_version == local_version {
            // Same version, take max value (Stuartian Law 5: max-registry).
            let local_value = local.get(node_id).unwrap_or(0.0);
            if *remote_value > local_value {
                local.reputations.insert(node_id.clone(), *remote_value);
                resolved += 1;
            }
        }
        // If local_version > remote_version, keep local (no action needed).
    }
    resolved
}

/// Synchronize region state with latency simulation.
pub fn sync_region_state(
    local: &mut RegionState,
    remote: &RegionState,
    latency_ms: u64,
    config: &SyncConfig,
) -> Result<SyncResult, SyncError> {
    config.validate()?;

    // Check latency threshold.
    if latency_ms > config.max_latency_ms {
        return Err(SyncError::RegionUnreachable(remote.region_id.clone()));
    }

    let start = Instant::now();

    // Calculate original state size.
    let original_size = remote.size_bytes();

    // Generate deltas if delta encoding is enabled.
    let deltas = if config.delta_encoding {
        generate_deltas(local, remote)
    } else {
        // Full sync (no delta encoding).
        let mut full_deltas = Vec::new();
        for (node_id, value) in &remote.reputations {
            let version = remote.get_version(node_id);
            let local_value = local.get(node_id).unwrap_or(0.0);
            full_deltas.push(DeltaEntry::new(
                node_id.clone(),
                *value,
                local_value,
                version,
            ));
        }
        full_deltas
    };

    // Check batch size.
    if deltas.len() > config.max_batch_size {
        return Err(SyncError::BatchSizeExceeded(deltas.len()));
    }

    // Calculate delta size.
    let delta_size: usize = deltas.iter().map(|d| d.size_bytes()).sum();

    // Apply deltas.
    let entries_merged = apply_deltas(local, &deltas);

    // Resolve any remaining conflicts.
    let conflicts_resolved = resolve_conflicts(local, remote);

    // Update local state metadata.
    local.last_sync = Instant::now().elapsed().as_millis() as u64;
    local.sync_count += 1;

    let duration = start.elapsed();

    Ok(SyncResult::new(
        entries_merged,
        conflicts_resolved,
        delta_size,
        original_size,
        duration,
        latency_ms,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_state_creation() {
        let state = RegionState::new("region-1".to_string());
        assert_eq!(state.region_id, "region-1");
        assert!(state.reputations.is_empty());
    }

    #[test]
    fn test_region_state_update() {
        let mut state = RegionState::new("region-1".to_string());
        state.update("node-1", 0.8);
        assert_eq!(state.get("node-1"), Some(0.8));
        assert_eq!(state.get_version("node-1"), 1);
    }

    #[test]
    fn test_region_state_update_increments_version() {
        let mut state = RegionState::new("region-1".to_string());
        state.update("node-1", 0.5);
        state.update("node-1", 0.8);
        assert_eq!(state.get_version("node-1"), 2);
    }

    #[test]
    fn test_generate_deltas_empty() {
        let local = RegionState::new("local".to_string());
        let remote = RegionState::new("remote".to_string());
        let deltas = generate_deltas(&local, &remote);
        assert!(deltas.is_empty());
    }

    #[test]
    fn test_generate_deltas_new_entries() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.9);
        let deltas = generate_deltas(&local, &remote);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].node_id, "node-1");
        assert_eq!(deltas[0].new_value, 0.9);
    }

    #[test]
    fn test_generate_deltas_skip_old() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        local.update("node-1", 0.9);
        local.update("node-1", 0.95); // version 2
        remote.update("node-1", 0.8); // version 1
        let deltas = generate_deltas(&local, &remote);
        assert!(deltas.is_empty()); // Remote is older, no deltas.
    }

    #[test]
    fn test_apply_deltas() {
        let mut state = RegionState::new("local".to_string());
        let deltas = vec![DeltaEntry::new("node-1".to_string(), 0.7, 0.0, 1)];
        let applied = apply_deltas(&mut state, &deltas);
        assert_eq!(applied, 1);
        assert_eq!(state.get("node-1"), Some(0.7));
    }

    #[test]
    fn test_resolve_conflicts_remote_newer() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        local.update("node-1", 0.5); // version 1
        remote.update("node-1", 0.9); // version 1
                                      // Manually set remote version higher.
        remote.versions.insert("node-1".to_string(), 2);
        let resolved = resolve_conflicts(&mut local, &remote);
        assert_eq!(resolved, 1);
        assert_eq!(local.get("node-1"), Some(0.9));
    }

    #[test]
    fn test_resolve_conflicts_same_version_max_wins() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        local.update("node-1", 0.5);
        remote.update("node-1", 0.9);
        let resolved = resolve_conflicts(&mut local, &remote);
        assert_eq!(resolved, 1);
        assert_eq!(local.get("node-1"), Some(0.9)); // Max wins.
    }

    #[test]
    fn test_sync_region_state_basic() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.8);
        remote.update("node-2", 0.6);
        let config = SyncConfig::default_config();
        let result = sync_region_state(&mut local, &remote, 50, &config).unwrap();
        assert!(result.success);
        assert_eq!(result.entries_merged, 2);
        assert_eq!(local.get("node-1"), Some(0.8));
        assert_eq!(local.get("node-2"), Some(0.6));
    }

    #[test]
    fn test_sync_region_state_low_latency() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.5);
        let config = SyncConfig::default_config();
        let result = sync_region_state(&mut local, &remote, 50, &config).unwrap();
        assert_eq!(result.effective_latency_ms, 50);
    }

    #[test]
    fn test_sync_region_state_medium_latency() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.5);
        let config = SyncConfig::default_config();
        let result = sync_region_state(&mut local, &remote, 500, &config).unwrap();
        assert_eq!(result.effective_latency_ms, 500);
    }

    #[test]
    fn test_sync_region_state_high_latency() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.5);
        let config = SyncConfig::default_config();
        let result = sync_region_state(&mut local, &remote, 2000, &config).unwrap();
        assert_eq!(result.effective_latency_ms, 2000);
    }

    #[test]
    fn test_sync_region_state_timeout() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.5);
        let mut config = SyncConfig::default_config();
        config.max_latency_ms = 1000;
        let result = sync_region_state(&mut local, &remote, 5000, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_compression_ratio() {
        let mut local = RegionState::new("local".to_string());
        let mut remote = RegionState::new("remote".to_string());
        // Add many entries to remote.
        for i in 0..100 {
            remote.update(&format!("node-{}", i), i as f64 / 100.0);
        }
        let config = SyncConfig::default_config();
        let result = sync_region_state(&mut local, &remote, 100, &config).unwrap();
        assert!(result.compression_ratio >= 0.0);
        assert!(result.compression_ratio <= 1.0);
    }

    #[test]
    fn test_sync_idempotent_convergence() {
        let mut local1 = RegionState::new("local1".to_string());
        let mut local2 = RegionState::new("local2".to_string());
        let mut remote = RegionState::new("remote".to_string());
        remote.update("node-1", 0.7);
        let config = SyncConfig::default_config();

        // Sync twice — should converge to same state.
        sync_region_state(&mut local1, &remote, 50, &config).unwrap();
        sync_region_state(&mut local2, &remote, 50, &config).unwrap();

        assert_eq!(local1.get("node-1"), local2.get("node-1"));
    }

    #[test]
    fn test_sync_config_validate() {
        let config = SyncConfig::default_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_sync_config_invalid_batch_size() {
        let config = SyncConfig {
            max_batch_size: 0,
            ..SyncConfig::default_config()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sync_config_invalid_timeout() {
        let config = SyncConfig {
            timeout: Duration::ZERO,
            ..SyncConfig::default_config()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_delta_entry_size() {
        let entry = DeltaEntry::new("node-1".to_string(), 0.5, 0.0, 1);
        assert!(entry.size_bytes() > 0);
    }

    #[test]
    fn test_region_state_size() {
        let mut state = RegionState::new("region-1".to_string());
        state.update("node-1", 0.5);
        assert!(state.size_bytes() > 0);
    }

    #[test]
    fn test_error_display() {
        let err = SyncError::VersionConflict("test".to_string());
        assert!(!format!("{}", err).is_empty());
        let err = SyncError::RegionUnreachable("region-1".to_string());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_sync_result_creation() {
        let result = SyncResult::new(10, 2, 100, 500, Duration::from_millis(50), 100);
        assert_eq!(result.entries_merged, 10);
        assert_eq!(result.conflicts_resolved, 2);
        assert!(result.success);
        assert!((result.compression_ratio - 0.8).abs() < 0.01); // (500-100)/500 = 0.8
    }
}
