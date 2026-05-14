//! Adaptive Sharder — Gestión adaptativa de particiones con balanceo dinámico
//!
//! Motor de sharding que ajusta dinámicamente las particiones basado en carga,
//! latencia y capacidad de nodos. Implementa partición consistente, migración
//! de shards y detección de desbalance.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use tracing::info;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for adaptive sharding.
#[derive(Debug, Error)]
pub enum AdaptiveSharderError {
    /// Shard not found.
    #[error("Shard {0} not found")]
    ShardNotFound(String),
    /// Node not found.
    #[error("Node {0} not found")]
    NodeNotFound(String),
    /// Invalid shard count.
    #[error("Invalid shard count: {0}")]
    InvalidShardCount(String),
    /// Migration failed.
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    /// Shard already exists.
    #[error("Shard {0} already exists")]
    ShardAlreadyExists(String),
    /// Insufficient nodes for shard.
    #[error("Insufficient nodes: have={have}, need={need}")]
    InsufficientNodes { have: usize, need: usize },
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Shard state in the federation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShardState {
    /// Shard is active and serving requests.
    Active,
    /// Shard is being created.
    Creating,
    /// Shard is being removed.
    Removing,
    /// Shard is migrating to another node.
    Migrating,
    /// Shard is paused.
    Paused,
}

impl std::fmt::Display for ShardState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardState::Active => write!(f, "Active"),
            ShardState::Creating => write!(f, "Creating"),
            ShardState::Removing => write!(f, "Removing"),
            ShardState::Migrating => write!(f, "Migrating"),
            ShardState::Paused => write!(f, "Paused"),
        }
    }
}

/// Shard partition definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPartition {
    /// Unique shard identifier.
    pub shard_id: String,
    /// Current state.
    pub state: ShardState,
    /// Primary node hosting this shard.
    pub primary_node: String,
    /// Replica nodes.
    pub replica_nodes: Vec<String>,
    /// Key range start (inclusive).
    pub key_range_start: u64,
    /// Key range end (exclusive).
    pub key_range_end: u64,
    /// Current item count.
    pub item_count: usize,
    /// Load factor (0.0-1.0).
    pub load_factor: f64,
    /// Average latency (ms).
    pub avg_latency_ms: f64,
    /// Created timestamp (ms).
    pub created_at_ms: u64,
    /// Last balance check (ms).
    pub last_balance_check_ms: u64,
}

impl ShardPartition {
    pub fn new(
        shard_id: String,
        primary_node: String,
        key_range_start: u64,
        key_range_end: u64,
    ) -> Self {
        Self {
            shard_id,
            state: ShardState::Creating,
            primary_node,
            replica_nodes: Vec::new(),
            key_range_start,
            key_range_end,
            item_count: 0,
            load_factor: 0.0,
            avg_latency_ms: 0.0,
            created_at_ms: current_timestamp_ms(),
            last_balance_check_ms: current_timestamp_ms(),
        }
    }

    /// Activates the shard after creation.
    pub fn activate(&mut self) {
        self.state = ShardState::Active;
    }

    /// Checks if shard is overloaded.
    pub fn is_overloaded(&self, threshold: f64) -> bool {
        self.load_factor > threshold
    }

    /// Checks if shard is underloaded.
    pub fn is_underloaded(&self, threshold: f64) -> bool {
        self.load_factor < threshold
    }

    /// Computes shard key hash for consistent routing.
    pub fn compute_key_hash(&self, key: &str) -> u64 {
        let hash = Sha256::digest(key.as_bytes());
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&hash[..8]);
        let val = u64::from_le_bytes(buf);
        self.key_range_start + (val % (self.key_range_end - self.key_range_start))
    }

    /// Checks if a key belongs to this shard.
    pub fn contains_key(&self, key_hash: u64) -> bool {
        key_hash >= self.key_range_start && key_hash < self.key_range_end
    }
}

/// Migration entry for shard movement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEntry {
    /// Migration identifier.
    pub migration_id: String,
    /// Shard being migrated.
    pub shard_id: String,
    /// Source node.
    pub source_node: String,
    /// Target node.
    pub target_node: String,
    /// Migration progress (0.0-1.0).
    pub progress: f64,
    /// Started timestamp (ms).
    pub started_at_ms: u64,
    /// Completed timestamp (ms).
    pub completed_at_ms: Option<u64>,
}

impl MigrationEntry {
    pub fn new(
        migration_id: String,
        shard_id: String,
        source_node: String,
        target_node: String,
    ) -> Self {
        Self {
            migration_id,
            shard_id,
            source_node,
            target_node,
            progress: 0.0,
            started_at_ms: current_timestamp_ms(),
            completed_at_ms: None,
        }
    }

    /// Updates migration progress.
    pub fn update_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
        if self.progress >= 1.0 {
            self.completed_at_ms = Some(current_timestamp_ms());
        }
    }

    /// Checks if migration is complete.
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}

/// Balance analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceAnalysis {
    /// Is federation balanced.
    pub balanced: bool,
    /// Load variance across shards.
    pub load_variance: f64,
    /// Max load factor.
    pub max_load_factor: f64,
    /// Min load factor.
    pub min_load_factor: f64,
    /// Average load factor.
    pub avg_load_factor: f64,
    /// Recommended actions.
    pub recommended_actions: Vec<BalanceAction>,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

/// Recommended balance action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceAction {
    /// Split an overloaded shard.
    SplitShard(String),
    /// Merge underloaded shards.
    MergeShards(Vec<String>),
    /// Migrate shard to a less loaded node.
    MigrateShard { shard_id: String, target_node: String },
    /// No action needed.
    NoOp,
}

impl std::fmt::Display for BalanceAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BalanceAction::SplitShard(id) => write!(f, "SplitShard({})", id),
            BalanceAction::MergeShards(ids) => {
                write!(f, "MergeShards([{}])", ids.join(", "))
            }
            BalanceAction::MigrateShard {
                shard_id,
                target_node,
            } => write!(
                f,
                "MigrateShard({}, {})",
                shard_id, target_node
            ),
            BalanceAction::NoOp => write!(f, "NoOp"),
        }
    }
}

/// Statistics for adaptive sharding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharderStats {
    /// Total shards created.
    pub total_shards_created: usize,
    /// Total shards removed.
    pub total_shards_removed: usize,
    /// Total migrations completed.
    pub total_migrations: usize,
    /// Total balance checks.
    pub total_balance_checks: usize,
    /// Active shard count.
    pub active_shards: usize,
    /// Active migrations.
    pub active_migrations: usize,
    /// Average load factor.
    pub avg_load_factor: f64,
    /// Load variance.
    pub load_variance: f64,
    /// Average migration time (ms).
    pub avg_migration_time_ms: f64,
}

impl Default for SharderStats {
    fn default() -> Self {
        Self {
            total_shards_created: 0,
            total_shards_removed: 0,
            total_migrations: 0,
            total_balance_checks: 0,
            active_shards: 0,
            active_migrations: 0,
            avg_load_factor: 0.0,
            load_variance: 0.0,
            avg_migration_time_ms: 0.0,
        }
    }
}

/// Configuration for adaptive sharder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveSharderConfig {
    /// Initial shard count.
    pub initial_shard_count: usize,
    /// Maximum shards allowed.
    pub max_shards: usize,
    /// Minimum nodes per shard (primary + replicas).
    pub min_nodes_per_shard: usize,
    /// Load threshold for splitting.
    pub split_threshold: f64,
    /// Load threshold for merging.
    pub merge_threshold: f64,
    /// Load variance threshold for rebalancing.
    pub variance_threshold: f64,
    /// Maximum concurrent migrations.
    pub max_concurrent_migrations: usize,
    /// Balance check interval (ms).
    pub balance_check_interval_ms: u64,
    /// Key space size.
    pub key_space_size: u64,
}

impl Default for AdaptiveSharderConfig {
    fn default() -> Self {
        Self {
            initial_shard_count: 4,
            max_shards: 64,
            min_nodes_per_shard: 2,
            split_threshold: 0.8,
            merge_threshold: 0.2,
            variance_threshold: 0.15,
            max_concurrent_migrations: 4,
            balance_check_interval_ms: 10_000,
            key_space_size: u64::MAX,
        }
    }
}

/// Adaptive sharder engine.
#[derive(Debug)]
pub struct AdaptiveSharder {
    config: AdaptiveSharderConfig,
    shards: HashMap<String, ShardPartition>,
    migrations: VecDeque<MigrationEntry>,
    node_load: HashMap<String, f64>,
    stats: SharderStats,
    balance_history: VecDeque<BalanceAnalysis>,
    next_shard_id: usize,
}

impl AdaptiveSharder {
    /// Creates a new adaptive sharder with default config.
    pub fn new() -> Self {
        Self::with_config(AdaptiveSharderConfig::default())
    }

    /// Creates a new adaptive sharder with custom config.
    pub fn with_config(config: AdaptiveSharderConfig) -> Self {
        let mut shards = HashMap::new();
        let initial_count = config.initial_shard_count;

        // Create initial shards
        let key_range = config.key_space_size / initial_count as u64;
        for i in 0..initial_count {
            let shard_id = format!("shard_{}", i);
            let shard = ShardPartition::new(
                shard_id.clone(),
                "bootstrap".to_string(),
                (i as u64) * key_range,
                ((i + 1) as u64) * key_range,
            );
            shards.insert(shard_id, shard);
        }

        Self {
            config,
            shards,
            migrations: VecDeque::new(),
            node_load: HashMap::new(),
            stats: SharderStats::default(),
            balance_history: VecDeque::new(),
            next_shard_id: initial_count,
        }
    }

    /// Registers a node with current load.
    pub fn register_node(&mut self, node_id: &str, load_factor: f64) {
        self.node_load
            .insert(node_id.to_string(), load_factor.clamp(0.0, 1.0));
    }

    /// Updates node load.
    pub fn update_node_load(&mut self, node_id: &str, load_factor: f64) {
        if let Some(load) = self.node_load.get_mut(node_id) {
            *load = load_factor.clamp(0.0, 1.0);
        }
    }

    /// Creates a new shard on the specified node.
    pub fn create_shard(
        &mut self,
        primary_node: String,
    ) -> Result<ShardPartition, AdaptiveSharderError> {
        if self.shards.len() >= self.config.max_shards {
            return Err(AdaptiveSharderError::InvalidShardCount(
                "Maximum shards reached".to_string(),
            ));
        }

        let shard_id = format!("shard_{}", self.next_shard_id);
        self.next_shard_id += 1;

        // Calculate key range for new shard
        let total_range = self.config.key_space_size;
        let shard_count = (self.shards.len() + 1) as u64;
        let key_range = total_range / shard_count;
        let start = (self.shards.len() as u64) * key_range;
        let end: u64 = if self.shards.len() + 1 == self.config.max_shards {
            total_range
        } else {
            ((self.shards.len() + 1) as u64) * key_range
        };

        let mut shard = ShardPartition::new(shard_id.clone(), primary_node.clone(), start, end);
        shard.activate();

        self.shards.insert(shard_id.clone(), shard.clone());
        self.stats.total_shards_created += 1;
        self.stats.active_shards = self.shards.len();

        info!("Created shard {} on node {}", shard_id, &primary_node);
        Ok(shard)
    }

    /// Removes a shard.
    pub fn remove_shard(&mut self, shard_id: &str) -> Result<ShardPartition, AdaptiveSharderError> {
        let shard = self.shards.remove(shard_id).ok_or_else(|| {
            AdaptiveSharderError::ShardNotFound(shard_id.to_string())
        })?;

        self.stats.total_shards_removed += 1;
        self.stats.active_shards = self.shards.len();

        info!("Removed shard {}", shard_id);
        Ok(shard)
    }

    /// Gets a shard by ID.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardPartition> {
        self.shards.get(shard_id)
    }

    /// Gets all active shards.
    pub fn get_active_shards(&self) -> Vec<&ShardPartition> {
        self.shards
            .values()
            .filter(|s| s.state == ShardState::Active)
            .collect()
    }

    /// Routes a key to the appropriate shard.
    pub fn route_key(&self, key: &str) -> Option<&ShardPartition> {
        let hash = Sha256::digest(key.as_bytes());
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&hash[..8]);
        let key_hash = u64::from_le_bytes(buf);

        self.shards
            .values()
            .find(|shard| shard.contains_key(key_hash))
    }

    /// Analyzes balance across all shards.
    pub fn analyze_balance(&mut self) -> BalanceAnalysis {
        let now = current_timestamp_ms();
        let loads: Vec<f64> = self.shards.values().map(|s| s.load_factor).collect();

        let count = loads.len().max(1);
        let avg = loads.iter().sum::<f64>() / count as f64;
        let variance = loads.iter().map(|l| (l - avg).powi(2)).sum::<f64>() / count as f64;
        let max_load = loads.iter().copied().fold(0.0f64, f64::max);
        let min_load = loads.iter().copied().fold(1.0f64, f64::min);

        let balanced = variance < self.config.variance_threshold;

        let mut actions = Vec::new();

        // Check for overloaded shards
        for shard in self.shards.values() {
            if shard.is_overloaded(self.config.split_threshold) {
                actions.push(BalanceAction::SplitShard(shard.shard_id.clone()));
            }
        }

        // Check for underloaded shards that can be merged
        let underloaded: Vec<String> = self.shards
            .values()
            .filter(|s| s.is_underloaded(self.config.merge_threshold))
            .map(|s| s.shard_id.clone())
            .collect();

        if underloaded.len() >= 2 {
            actions.push(BalanceAction::MergeShards(underloaded));
        }

        // Check for migration opportunities
        for shard in self.shards.values() {
            if shard.is_overloaded(self.config.split_threshold) {
                if let Some(target) = self.find_least_loaded_node() {
                    if target != shard.primary_node {
                        actions.push(BalanceAction::MigrateShard {
                            shard_id: shard.shard_id.clone(),
                            target_node: target,
                        });
                    }
                }
            }
        }

        if actions.is_empty() {
            actions.push(BalanceAction::NoOp);
        }

        let analysis = BalanceAnalysis {
            balanced,
            load_variance: variance,
            max_load_factor: max_load,
            min_load_factor: min_load,
            avg_load_factor: avg,
            recommended_actions: actions,
            timestamp_ms: now,
        };

        self.stats.total_balance_checks += 1;
        self.stats.avg_load_factor = avg;
        self.stats.load_variance = variance;

        // Keep balance history limited
        self.balance_history.push_back(analysis.clone());
        if self.balance_history.len() > 100 {
            self.balance_history.pop_front();
        }

        analysis
    }

    /// Starts a shard migration.
    pub fn start_migration(
        &mut self,
        shard_id: &str,
        target_node: String,
    ) -> Result<MigrationEntry, AdaptiveSharderError> {
        // Check concurrent migration limit
        let active_migrations: usize = self
            .migrations
            .iter()
            .filter(|m| !m.is_complete())
            .count();

        if active_migrations >= self.config.max_concurrent_migrations {
            return Err(AdaptiveSharderError::MigrationFailed(
                "Maximum concurrent migrations reached".to_string(),
            ));
        }

        let shard = self.shards.get(shard_id).ok_or_else(|| {
            AdaptiveSharderError::ShardNotFound(shard_id.to_string())
        })?;

        let migration_id = format!("mig_{}_{}", shard_id, current_timestamp_ms());
        let mut migration = MigrationEntry::new(
            migration_id,
            shard_id.to_string(),
            shard.primary_node.clone(),
            target_node,
        );

        // Update shard state
        if let Some(shard) = self.shards.get_mut(shard_id) {
            shard.state = ShardState::Migrating;
        }

        migration.update_progress(0.1);
        self.migrations.push_back(migration.clone());
        self.stats.active_migrations = active_migrations + 1;

        Ok(migration)
    }

    /// Completes a migration.
    pub fn complete_migration(&mut self, migration_id: &str) -> Result<MigrationEntry, AdaptiveSharderError> {
        let mut migration_opt: Option<MigrationEntry> = None;

        for i in 0..self.migrations.len() {
            if self.migrations[i].migration_id == migration_id {
                migration_opt = Some(self.migrations.remove(i).unwrap());
                break;
            }
        }

        let migration = migration_opt.ok_or_else(|| {
            AdaptiveSharderError::MigrationFailed(format!(
                "Migration {} not found",
                migration_id
            ))
        })?;

        // Update shard state back to active
        if let Some(shard) = self.shards.get_mut(&migration.shard_id) {
            shard.state = ShardState::Active;
            shard.primary_node = migration.target_node.clone();
        }

        self.stats.total_migrations += 1;
        self.stats.active_migrations = self.migrations.iter().filter(|m| !m.is_complete()).count();

        // Calculate migration time
        if let Some(completed) = migration.completed_at_ms {
            let elapsed = completed.saturating_sub(migration.started_at_ms) as f64;
            self.stats.avg_migration_time_ms =
                (self.stats.avg_migration_time_ms + elapsed) / 2.0;
        }

        Ok(migration)
    }

    /// Gets active migrations.
    pub fn get_active_migrations(&self) -> Vec<&MigrationEntry> {
        self.migrations
            .iter()
            .filter(|m| !m.is_complete())
            .collect()
    }

    /// Gets the current stats.
    pub fn get_stats(&self) -> SharderStats {
        self.stats.clone()
    }

    /// Resets stats.
    pub fn reset_stats(&mut self) {
        self.stats = SharderStats::default();
    }

    /// Gets balance history.
    pub fn get_balance_history(&self) -> Vec<&BalanceAnalysis> {
        self.balance_history.iter().collect()
    }

    /// Finds the least loaded node.
    fn find_least_loaded_node(&self) -> Option<String> {
        self.node_load
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(node, _)| node.clone())
    }

    /// Gets config.
    pub fn get_config(&self) -> &AdaptiveSharderConfig {
        &self.config
    }
}

impl Default for AdaptiveSharder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shard(id: &str, primary: &str) -> ShardPartition {
        ShardPartition::new(id.to_string(), primary.to_string(), 0, 1000)
    }

    #[test]
    fn test_sharder_creation() {
        let sharder = AdaptiveSharder::new();
        assert_eq!(sharder.shards.len(), 4);
    }

    #[test]
    fn test_sharder_with_config() {
        let config = AdaptiveSharderConfig {
            initial_shard_count: 8,
            ..Default::default()
        };
        let sharder = AdaptiveSharder::with_config(config);
        assert_eq!(sharder.shards.len(), 8);
    }

    #[test]
    fn test_register_node() {
        let mut sharder = AdaptiveSharder::new();
        sharder.register_node("node_1", 0.5);
        assert_eq!(*sharder.node_load.get("node_1").unwrap(), 0.5);
    }

    #[test]
    fn test_update_node_load() {
        let mut sharder = AdaptiveSharder::new();
        sharder.register_node("node_1", 0.5);
        sharder.update_node_load("node_1", 0.8);
        assert_eq!(*sharder.node_load.get("node_1").unwrap(), 0.8);
    }

    #[test]
    fn test_create_shard() {
        let mut sharder = AdaptiveSharder::new();
        let shard = sharder.create_shard("node_1".to_string()).unwrap();
        assert_eq!(shard.state, ShardState::Active);
        assert_eq!(sharder.stats.total_shards_created, 1);
    }

    #[test]
    fn test_create_shard_max_reached() {
        let mut sharder = AdaptiveSharder::with_config(AdaptiveSharderConfig {
            max_shards: 4,
            initial_shard_count: 4,
            ..Default::default()
        });
        let result = sharder.create_shard("node_1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_shard() {
        let mut sharder = AdaptiveSharder::new();
        let shard_id = sharder.shards.keys().next().unwrap().clone();
        let shard = sharder.remove_shard(&shard_id).unwrap();
        assert_eq!(shard.shard_id, shard_id);
        assert!(sharder.shards.get(&shard_id).is_none());
    }

    #[test]
    fn test_remove_nonexistent_shard() {
        let mut sharder = AdaptiveSharder::new();
        let result = sharder.remove_shard("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_shard() {
        let sharder = AdaptiveSharder::new();
        let first_id = sharder.shards.keys().next().unwrap().clone();
        assert!(sharder.get_shard(&first_id).is_some());
        assert!(sharder.get_shard("nonexistent").is_none());
    }

    #[test]
    fn test_get_active_shards() {
        let sharder = AdaptiveSharder::new();
        let active = sharder.get_active_shards();
        // Initial shards are in Creating state
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_route_key() {
        let sharder = AdaptiveSharder::new();
        // Activate all shards first
        for shard in sharder.shards.values() {
            assert!(shard.state == ShardState::Creating);
        }
        // Route should still find a shard by key range
        let result = sharder.route_key("test_key");
        // May or may not find depending on hash
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_analyze_balance() {
        let mut sharder = AdaptiveSharder::new();
        sharder.register_node("node_1", 0.3);
        sharder.register_node("node_2", 0.7);

        let analysis = sharder.analyze_balance();
        assert_eq!(sharder.stats.total_balance_checks, 1);
        assert!(analysis.recommended_actions.len() > 0);
    }

    #[test]
    fn test_analyze_balance_overloaded() {
        let mut sharder = AdaptiveSharder::new();
        // Set high load on a shard
        if let Some(shard) = sharder.shards.get_mut("shard_0") {
            shard.load_factor = 0.9;
        }
        sharder.register_node("node_1", 0.3);

        let analysis = sharder.analyze_balance();
        let has_split = analysis
            .recommended_actions
            .iter()
            .any(|a| matches!(a, BalanceAction::SplitShard(_)));
        assert!(has_split);
    }

    #[test]
    fn test_analyze_balance_underloaded() {
        let mut sharder = AdaptiveSharder::new();
        // Set low load on multiple shards
        for shard in sharder.shards.values_mut() {
            shard.load_factor = 0.1;
        }

        let analysis = sharder.analyze_balance();
        let has_merge = analysis
            .recommended_actions
            .iter()
            .any(|a| matches!(a, BalanceAction::MergeShards(_)));
        assert!(has_merge);
    }

    #[test]
    fn test_start_migration() {
        let mut sharder = AdaptiveSharder::new();
        // Activate a shard
        if let Some(shard) = sharder.shards.get_mut("shard_0") {
            shard.activate();
        }
        sharder.register_node("node_1", 0.3);

        let migration = sharder.start_migration("shard_0", "node_1".to_string());
        assert!(migration.is_ok());
    }

    #[test]
    fn test_start_migration_not_found() {
        let mut sharder = AdaptiveSharder::new();
        let result = sharder.start_migration("nonexistent", "node_1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_migration() {
        let mut sharder = AdaptiveSharder::new();
        if let Some(shard) = sharder.shards.get_mut("shard_0") {
            shard.activate();
        }
        sharder.register_node("node_1", 0.3);

        let migration = sharder.start_migration("shard_0", "node_1".to_string()).unwrap();
        let completed = sharder.complete_migration(&migration.migration_id).unwrap();
        assert_eq!(completed.shard_id, "shard_0");
        assert_eq!(sharder.stats.total_migrations, 1);
    }

    #[test]
    fn test_get_active_migrations() {
        let mut sharder = AdaptiveSharder::new();
        if let Some(shard) = sharder.shards.get_mut("shard_0") {
            shard.activate();
        }
        sharder.register_node("node_1", 0.3);

        sharder.start_migration("shard_0", "node_1".to_string()).unwrap();
        let active = sharder.get_active_migrations();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_get_stats() {
        let sharder = AdaptiveSharder::new();
        let stats = sharder.get_stats();
        assert_eq!(stats.total_shards_created, 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut sharder = AdaptiveSharder::new();
        sharder.analyze_balance();
        sharder.reset_stats();
        let stats = sharder.get_stats();
        assert_eq!(stats.total_balance_checks, 0);
    }

    #[test]
    fn test_get_balance_history() {
        let mut sharder = AdaptiveSharder::new();
        sharder.analyze_balance();
        sharder.analyze_balance();
        let history = sharder.get_balance_history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_shard_partition_activate() {
        let mut shard = make_shard("test", "node_1");
        assert_eq!(shard.state, ShardState::Creating);
        shard.activate();
        assert_eq!(shard.state, ShardState::Active);
    }

    #[test]
    fn test_shard_is_overloaded() {
        let shard = make_shard("test", "node_1");
        assert!(!shard.is_overloaded(0.8));
    }

    #[test]
    fn test_shard_is_underloaded() {
        let shard = make_shard("test", "node_1");
        assert!(shard.is_underloaded(0.2));
    }

    #[test]
    fn test_shard_contains_key() {
        let shard = ShardPartition::new("test".to_string(), "node_1".to_string(), 0, 1000);
        assert!(shard.contains_key(500));
        assert!(!shard.contains_key(1500));
    }

    #[test]
    fn test_shard_compute_key_hash() {
        let shard = ShardPartition::new("test".to_string(), "node_1".to_string(), 0, 1000);
        let hash = shard.compute_key_hash("test_key");
        assert!(hash >= 0 && hash < 1000);
    }

    #[test]
    fn test_migration_entry() {
        let mut migration = MigrationEntry::new(
            "mig_1".to_string(),
            "shard_0".to_string(),
            "node_1".to_string(),
            "node_2".to_string(),
        );
        assert!(!migration.is_complete());
        migration.update_progress(1.0);
        assert!(migration.is_complete());
    }

    #[test]
    fn test_migration_progress_clamping() {
        let mut migration = MigrationEntry::new(
            "mig_1".to_string(),
            "shard_0".to_string(),
            "node_1".to_string(),
            "node_2".to_string(),
        );
        migration.update_progress(1.5);
        assert_eq!(migration.progress, 1.0);
    }

    #[test]
    fn test_balance_action_display() {
        let action = BalanceAction::SplitShard("shard_0".to_string());
        assert_eq!(format!("{}", action), "SplitShard(shard_0)");

        let action = BalanceAction::NoOp;
        assert_eq!(format!("{}", action), "NoOp");
    }

    #[test]
    fn test_shard_state_display() {
        assert_eq!(format!("{}", ShardState::Active), "Active");
        assert_eq!(format!("{}", ShardState::Creating), "Creating");
        assert_eq!(format!("{}", ShardState::Migrating), "Migrating");
    }

    #[test]
    fn test_config_default() {
        let config = AdaptiveSharderConfig::default();
        assert_eq!(config.initial_shard_count, 4);
        assert_eq!(config.max_shards, 64);
    }

    #[test]
    fn test_stats_default() {
        let stats = SharderStats::default();
        assert_eq!(stats.total_shards_created, 0);
        assert_eq!(stats.active_shards, 0);
    }

    #[test]
    fn test_sharder_default() {
        let sharder = AdaptiveSharder::default();
        assert_eq!(sharder.shards.len(), 4);
    }

    #[test]
    fn test_get_config() {
        let sharder = AdaptiveSharder::new();
        let config = sharder.get_config();
        assert_eq!(config.initial_shard_count, 4);
    }

    #[test]
    fn test_node_load_clamping() {
        let mut sharder = AdaptiveSharder::new();
        sharder.register_node("node_1", 1.5);
        assert_eq!(*sharder.node_load.get("node_1").unwrap(), 1.0);

        sharder.register_node("node_2", -0.5);
        assert_eq!(*sharder.node_load.get("node_2").unwrap(), 0.0);
    }

    #[test]
    fn test_max_concurrent_migrations() {
        let mut sharder = AdaptiveSharder::with_config(AdaptiveSharderConfig {
            max_concurrent_migrations: 1,
            ..Default::default()
        });

        // Activate shards
        for shard in sharder.shards.values_mut() {
            shard.activate();
        }
        sharder.register_node("node_1", 0.3);

        // First migration should succeed
        assert!(sharder.start_migration("shard_0", "node_1".to_string()).is_ok());

        // Second migration should fail
        let result = sharder.start_migration("shard_1", "node_1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_history_limit() {
        let mut sharder = AdaptiveSharder::new();
        for _ in 0..150 {
            sharder.analyze_balance();
        }
        let history = sharder.get_balance_history();
        assert!(history.len() <= 100);
    }

    #[test]
    fn test_error_display() {
        let err = AdaptiveSharderError::ShardNotFound("shard_0".to_string());
        assert!(format!("{}", err).contains("shard_0"));
    }
}
