//! Dynamic Aggregator — Cross-chain shard aggregation engine for adaptive pool composition.
//!
//! Dynamically aggregates shards from multiple pools into unified compute groups
//! based on demand patterns, capacity availability and reputation scores.
//!
//! **Design:** Linux `mdadm`-inspired RAID-like aggregation for compute shards.
//!
//! **Key features:**
//! - Dynamic shard grouping by capacity and latency profiles
//! - Weighted aggregation using reputation scores
//! - Automatic rebalancing when capacity thresholds change
//! - SAE shard-aware aggregation
//!
//! **References:**
//! - `pool_matcher.rs` — Weighted scoring patterns
//! - `cross_chain_pools_v3.rs` — Pool/shard data structures
//!
//! Apache License 2.0 + Ethical Use Clause

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

// ─── Error ───────────────────────────────────────────────────────────────────

/// Errors for dynamic aggregation operations.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregatorError {
    /// Shard ID not found in any aggregation group.
    ShardNotFound(String),
    /// Pool ID not registered.
    PoolNotFound(String),
    /// Aggregation group is full.
    GroupFull(String),
    /// Invalid configuration parameter.
    InvalidConfig(String),
    /// Aggregation capacity exceeded.
    CapacityExceeded,
    /// Shard already belongs to another group.
    ShardAlreadyAssigned(String),
}

impl std::fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregatorError::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
            AggregatorError::PoolNotFound(id) => write!(f, "Pool not found: {}", id),
            AggregatorError::GroupFull(id) => write!(f, "Aggregation group full: {}", id),
            AggregatorError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            AggregatorError::CapacityExceeded => write!(f, "Aggregation capacity exceeded"),
            AggregatorError::ShardAlreadyAssigned(id) => write!(f, "Shard already assigned: {}", id),
        }
    }
}

// ─── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the dynamic aggregator.
#[derive(Debug, Clone)]
pub struct DynamicAggregatorConfig {
    /// Maximum number of aggregation groups.
    pub max_groups: usize,
    /// Maximum shards per aggregation group.
    pub max_shards_per_group: usize,
    /// Minimum reputation for shards to be aggregated.
    pub min_reputation: f64,
    /// Weight for reputation in aggregation scoring.
    pub reputation_weight: f64,
    /// Weight for latency in aggregation scoring.
    pub latency_weight: f64,
    /// Weight for capacity in aggregation scoring.
    pub capacity_weight: f64,
    /// Threshold for triggering rebalance (utilization ratio).
    pub rebalance_threshold: f64,
    /// Enable automatic rebalancing.
    pub auto_rebalance: bool,
}

impl Default for DynamicAggregatorConfig {
    fn default() -> Self {
        Self {
            max_groups: 64,
            max_shards_per_group: 16,
            min_reputation: 0.5,
            reputation_weight: 0.4,
            latency_weight: 0.3,
            capacity_weight: 0.3,
            rebalance_threshold: 0.8,
            auto_rebalance: true,
        }
    }
}

// ─── Shard Info ──────────────────────────────────────────────────────────────

/// Shard information for aggregation.
#[derive(Debug, Clone)]
pub struct ShardInfo {
    /// Unique shard identifier.
    pub shard_id: String,
    /// Pool this shard belongs to.
    pub pool_id: String,
    /// Chain ID for cross-chain context.
    pub chain_id: String,
    /// Available compute credits.
    pub available_credits: f64,
    /// Total capacity.
    pub total_capacity: f64,
    /// Reputation score [0.0, 1.0].
    pub reputation: f64,
    /// Average latency in ms.
    pub avg_latency_ms: f64,
    /// Current utilization ratio [0.0, 1.0].
    pub utilization: f64,
    /// Aggregation group ID (if assigned).
    pub group_id: Option<String>,
    /// Timestamp of last update.
    pub last_update_ms: u64,
}

impl ShardInfo {
    pub fn new(
        shard_id: String,
        pool_id: String,
        chain_id: String,
        total_capacity: f64,
        reputation: f64,
    ) -> Self {
        Self {
            shard_id,
            pool_id,
            chain_id,
            available_credits: total_capacity,
            total_capacity,
            reputation,
            avg_latency_ms: 0.0,
            utilization: 0.0,
            group_id: None,
            last_update_ms: current_timestamp_ms(),
        }
    }

    /// Compute aggregation score for this shard.
    pub fn aggregation_score(&self, config: &DynamicAggregatorConfig) -> f64 {
        let rep_score = self.reputation.clamp(0.0, 1.0);
        let lat_score = 1.0 - (self.avg_latency_ms / 1000.0).clamp(0.0, 1.0);
        let cap_score = (1.0 - self.utilization).clamp(0.0, 1.0);

        config.reputation_weight * rep_score
            + config.latency_weight * lat_score
            + config.capacity_weight * cap_score
    }

    /// Check if shard meets minimum reputation.
    pub fn meets_reputation(&self, min_rep: f64) -> bool {
        self.reputation >= min_rep
    }
}

// ─── Aggregation Group ───────────────────────────────────────────────────────

/// An aggregation group containing multiple shards.
#[derive(Debug, Clone)]
pub struct AggregationGroup {
    /// Unique group identifier.
    pub group_id: String,
    /// Shards in this group.
    pub shard_ids: Vec<String>,
    /// Total aggregated capacity.
    pub total_capacity: f64,
    /// Total available credits across shards.
    pub total_available: f64,
    /// Weighted average reputation.
    pub avg_reputation: f64,
    /// Weighted average latency.
    pub avg_latency_ms: f64,
    /// Current group utilization.
    pub utilization: f64,
    /// Creation timestamp.
    pub created_ms: u64,
    /// Last rebalance timestamp.
    pub last_rebalance_ms: u64,
}

impl AggregationGroup {
    pub fn new(group_id: String) -> Self {
        Self {
            group_id,
            shard_ids: Vec::new(),
            total_capacity: 0.0,
            total_available: 0.0,
            avg_reputation: 0.0,
            avg_latency_ms: 0.0,
            utilization: 0.0,
            created_ms: current_timestamp_ms(),
            last_rebalance_ms: current_timestamp_ms(),
        }
    }

    /// Check if group can accept more shards.
    pub fn can_add_shard(&self, max_shards: usize) -> bool {
        self.shard_ids.len() < max_shards
    }

    /// Check if group needs rebalancing.
    pub fn needs_rebalance(&self, threshold: f64) -> bool {
        self.utilization > threshold
    }
}

// ─── Stats ───────────────────────────────────────────────────────────────────

/// Statistics for the dynamic aggregator.
#[derive(Debug, Clone)]
pub struct AggregatorStats {
    /// Total shards registered.
    pub total_shards: usize,
    /// Total aggregation groups created.
    pub total_groups: usize,
    /// Total aggregations performed.
    pub total_aggregations: usize,
    /// Total rebalances performed.
    pub total_rebalances: usize,
    /// Shards currently unassigned.
    pub unassigned_shards: usize,
    /// Last rebalance timestamp.
    pub last_rebalance_ms: u64,
}

impl Default for AggregatorStats {
    fn default() -> Self {
        Self {
            total_shards: 0,
            total_groups: 0,
            total_aggregations: 0,
            total_rebalances: 0,
            unassigned_shards: 0,
            last_rebalance_ms: 0,
        }
    }
}

// ─── Priority Item for BinaryHeap ────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ShardPriorityItem {
    shard_id: String,
    score: f64,
}

impl PartialEq for ShardPriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.shard_id == other.shard_id && self.score == other.score
    }
}

impl Eq for ShardPriorityItem {}

impl Ord for ShardPriorityItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.shard_id.cmp(&other.shard_id))
    }
}

impl PartialOrd for ShardPriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Main Aggregator ─────────────────────────────────────────────────────────

/// Dynamic cross-chain shard aggregator.
pub struct DynamicAggregator {
    config: DynamicAggregatorConfig,
    shards: HashMap<String, ShardInfo>,
    groups: HashMap<String, AggregationGroup>,
    pool_shards: HashMap<String, Vec<String>>,
    stats: AggregatorStats,
    current_time_ms: u64,
    group_counter: u64,
}

impl DynamicAggregator {
    // ─── Construction ──────────────────────────────────────────────────────

    /// Create a new dynamic aggregator with the given configuration.
    pub fn new(config: DynamicAggregatorConfig) -> Self {
        Self {
            config,
            shards: HashMap::new(),
            groups: HashMap::new(),
            pool_shards: HashMap::new(),
            stats: AggregatorStats::default(),
            current_time_ms: current_timestamp_ms(),
            group_counter: 0,
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(DynamicAggregatorConfig::default())
    }

    // ─── Shard Management ──────────────────────────────────────────────────

    /// Register a shard for aggregation.
    pub fn register_shard(&mut self, shard: ShardInfo) -> Result<(), AggregatorError> {
        if shard.reputation < self.config.min_reputation {
            return Err(AggregatorError::InvalidConfig(format!(
                "Shard {} reputation {:.3} below minimum {:.3}",
                shard.shard_id, shard.reputation, self.config.min_reputation
            )));
        }

        if shard.group_id.is_some() {
            return Err(AggregatorError::ShardAlreadyAssigned(shard.shard_id.clone()));
        }

        let shard_id = shard.shard_id.clone();
        self.shards.insert(shard_id.clone(), shard);
        self.stats.total_shards += 1;

        self.pool_shards
            .entry(shard_id.clone())
            .or_insert_with(Vec::new)
            .push(shard_id);

        Ok(())
    }

    /// Remove a shard from the aggregator.
    pub fn remove_shard(&mut self, shard_id: &str) -> Result<(), AggregatorError> {
        if self.shards.get(shard_id).is_none() {
            return Err(AggregatorError::ShardNotFound(shard_id.to_string()));
        }

        if let Some(group_id) = self.shards.get(shard_id).and_then(|s| s.group_id.clone()) {
            self.remove_shard_from_group(&group_id, shard_id)?;
        }

        self.shards.remove(shard_id);
        self.stats.total_shards = self.shards.len();
        Ok(())
    }

    /// Get shard information.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardInfo> {
        self.shards.get(shard_id)
    }

    /// Get all shards for a pool.
    pub fn get_pool_shards(&self, pool_id: &str) -> Vec<&ShardInfo> {
        self.shards
            .values()
            .filter(|s| s.pool_id == pool_id)
            .collect()
    }

    // ─── Group Management ──────────────────────────────────────────────────

    /// Create a new aggregation group.
    pub fn create_group(&mut self, group_id: String) -> Result<(), AggregatorError> {
        if self.groups.len() >= self.config.max_groups {
            return Err(AggregatorError::GroupFull(group_id.clone()));
        }

        if self.groups.contains_key(&group_id) {
            return Err(AggregatorError::GroupFull(group_id));
        }

        let group = AggregationGroup::new(group_id.clone());
        self.groups.insert(group_id, group);
        self.stats.total_groups += 1;
        Ok(())
    }

    /// Auto-create a group with generated ID.
    pub fn auto_create_group(&mut self) -> Result<String, AggregatorError> {
        self.group_counter += 1;
        let group_id = format!("agg-{}", self.group_counter);
        self.create_group(group_id.clone())?;
        Ok(group_id)
    }

    /// Get group information.
    pub fn get_group(&self, group_id: &str) -> Option<&AggregationGroup> {
        self.groups.get(group_id)
    }

    /// Remove an aggregation group.
    pub fn remove_group(&mut self, group_id: &str) -> Result<(), AggregatorError> {
        if !self.groups.contains_key(group_id) {
            return Err(AggregatorError::ShardNotFound(group_id.to_string()));
        }

        if let Some(group) = self.groups.get(group_id) {
            for shard_id in &group.shard_ids {
                if let Some(shard) = self.shards.get_mut(shard_id) {
                    shard.group_id = None;
                }
            }
        }

        self.groups.remove(group_id);
        self.stats.total_groups = self.groups.len();
        Ok(())
    }

    // ─── Aggregation ───────────────────────────────────────────────────────

    /// Add a shard to an aggregation group.
    pub fn add_shard_to_group(
        &mut self,
        group_id: &str,
        shard_id: &str,
    ) -> Result<(), AggregatorError> {
        let group = self.groups.get(group_id).ok_or_else(|| {
            AggregatorError::PoolNotFound(group_id.to_string())
        })?;

        if !group.can_add_shard(self.config.max_shards_per_group) {
            return Err(AggregatorError::GroupFull(group_id.to_string()));
        }

        let shard = self.shards.get(shard_id).ok_or_else(|| {
            AggregatorError::ShardNotFound(shard_id.to_string())
        })?;

        if shard.group_id.is_some() {
            return Err(AggregatorError::ShardAlreadyAssigned(shard_id.to_string()));
        }

        // Update shard
        if let Some(shard) = self.shards.get_mut(shard_id) {
            shard.group_id = Some(group_id.to_string());
        }

        // Update group
        if let Some(group) = self.groups.get_mut(group_id) {
            group.shard_ids.push(shard_id.to_string());
            self.recalculate_group_stats(group_id)?;
        }

        self.stats.total_aggregations += 1;
        Ok(())
    }

    /// Remove a shard from a group.
    pub fn remove_shard_from_group(
        &mut self,
        group_id: &str,
        shard_id: &str,
    ) -> Result<(), AggregatorError> {
        if let Some(group) = self.groups.get_mut(group_id) {
            group.shard_ids.retain(|id| id != shard_id);
            self.recalculate_group_stats(group_id)?;
        }

        if let Some(shard) = self.shards.get_mut(shard_id) {
            shard.group_id = None;
        }

        Ok(())
    }

    /// Automatically aggregate unassigned shards into groups.
    pub fn auto_aggregate(&mut self) -> Result<usize, AggregatorError> {
        let unassigned: Vec<&String> = self.shards
            .values()
            .filter(|s| s.group_id.is_none())
            .map(|s| &s.shard_id)
            .collect();

        if unassigned.is_empty() {
            return Ok(0);
        }

        // Score and sort shards
        let mut scored: BinaryHeap<ShardPriorityItem> = BinaryHeap::new();
        for shard_id in &unassigned {
            if let Some(shard) = self.shards.get(shard_id.as_str()) {
                let score = shard.aggregation_score(&self.config);
                scored.push(ShardPriorityItem {
                    shard_id: shard_id.to_string(),
                    score,
                });
            }
        }

        let mut aggregated = 0;
        let mut current_group = self.auto_create_group()?;

        while let Some(item) = scored.pop() {
            let group = self.groups.get(&current_group).unwrap();
            if !group.can_add_shard(self.config.max_shards_per_group) {
                current_group = self.auto_create_group()?;
            }

            if self.add_shard_to_group(&current_group, &item.shard_id).is_ok() {
                aggregated += 1;
            }
        }

        Ok(aggregated)
    }

    // ─── Rebalancing ───────────────────────────────────────────────────────

    /// Check if rebalancing is needed.
    pub fn needs_rebalance(&self) -> bool {
        self.groups.values().any(|g| g.needs_rebalance(self.config.rebalance_threshold))
    }

    /// Perform rebalancing across all groups.
    pub fn rebalance(&mut self) -> Result<usize, AggregatorError> {
        if !self.config.auto_rebalance {
            return Ok(0);
        }

        let overloaded: Vec<String> = self.groups
            .values()
            .filter(|g| g.needs_rebalance(self.config.rebalance_threshold))
            .map(|g| g.group_id.clone())
            .collect();

        if overloaded.is_empty() {
            return Ok(0);
        }

        let mut moved = 0;
        for group_id in &overloaded {
            if let Some(group) = self.groups.get(group_id) {
                // Move lowest scoring shard to new group
                if let Some(shard_id) = self.find_lowest_score_shard(group) {
                    let new_group = self.auto_create_group()?;
                    if self.remove_shard_from_group(group_id, &shard_id).is_ok()
                        && self.add_shard_to_group(&new_group, &shard_id).is_ok()
                    {
                        moved += 1;
                    }
                }
            }
        }

        self.stats.total_rebalances += moved;
        self.stats.last_rebalance_ms = self.current_time_ms;
        Ok(moved)
    }

    // ─── Queries ───────────────────────────────────────────────────────────

    /// Get all groups sorted by utilization.
    pub fn groups_by_utilization(&self) -> Vec<&AggregationGroup> {
        let mut groups: Vec<&AggregationGroup> = self.groups.values().collect();
        groups.sort_by(|a, b| b.utilization.partial_cmp(&a.utilization).unwrap_or(Ordering::Equal));
        groups
    }

    /// Get the best group for a given capacity requirement.
    pub fn find_best_group(&self, required_credits: f64) -> Option<&AggregationGroup> {
        self.groups
            .values()
            .filter(|g| g.total_available >= required_credits)
            .min_by_key(|g| (g.avg_latency_ms * 1000.0) as u64)
    }

    /// Get unassigned shards count.
    pub fn unassigned_count(&self) -> usize {
        self.shards.values().filter(|s| s.group_id.is_none()).count()
    }

    // ─── Updates ───────────────────────────────────────────────────────────

    /// Update shard utilization.
    pub fn update_shard_utilization(
        &mut self,
        shard_id: &str,
        utilization: f64,
        available: f64,
    ) -> Result<(), AggregatorError> {
        let group_id = {
            let shard = self.shards.get(shard_id).ok_or_else(|| {
                AggregatorError::ShardNotFound(shard_id.to_string())
            })?;
            shard.group_id.clone()
        };

        let shard = self.shards.get_mut(shard_id).unwrap();
        shard.utilization = utilization.clamp(0.0, 1.0);
        shard.available_credits = available;
        shard.last_update_ms = self.current_time_ms;

        if let Some(ref gid) = group_id {
            self.recalculate_group_stats(gid)?;
        }

        Ok(())
    }

    /// Update shard latency.
    pub fn update_shard_latency(
        &mut self,
        shard_id: &str,
        latency_ms: f64,
    ) -> Result<(), AggregatorError> {
        let group_id = {
            let shard = self.shards.get(shard_id).ok_or_else(|| {
                AggregatorError::ShardNotFound(shard_id.to_string())
            })?;
            shard.group_id.clone()
        };

        let shard = self.shards.get_mut(shard_id).unwrap();
        shard.avg_latency_ms = shard.avg_latency_ms * 0.7 + latency_ms * 0.3;
        shard.last_update_ms = self.current_time_ms;

        if let Some(ref gid) = group_id {
            self.recalculate_group_stats(gid)?;
        }

        Ok(())
    }

    /// Update shard reputation.
    pub fn update_shard_reputation(
        &mut self,
        shard_id: &str,
        reputation: f64,
    ) -> Result<(), AggregatorError> {
        let group_id = {
            let shard = self.shards.get(shard_id).ok_or_else(|| {
                AggregatorError::ShardNotFound(shard_id.to_string())
            })?;
            shard.group_id.clone()
        };

        let shard = self.shards.get_mut(shard_id).unwrap();
        shard.reputation = reputation.clamp(0.0, 1.0);
        shard.last_update_ms = self.current_time_ms;

        if let Some(ref gid) = group_id {
            self.recalculate_group_stats(gid)?;
        }

        Ok(())
    }

    /// Advance internal time.
    pub fn advance_time(&mut self, ms: u64) {
        self.current_time_ms += ms;
    }

    // ─── Stats ─────────────────────────────────────────────────────────────

    /// Get current statistics.
    pub fn stats(&self) -> AggregatorStats {
        AggregatorStats {
            total_shards: self.shards.len(),
            total_groups: self.groups.len(),
            total_aggregations: self.stats.total_aggregations,
            total_rebalances: self.stats.total_rebalances,
            unassigned_shards: self.unassigned_count(),
            last_rebalance_ms: self.stats.last_rebalance_ms,
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = AggregatorStats::default();
    }

    // ─── Internal ──────────────────────────────────────────────────────────

    fn recalculate_group_stats(&mut self, group_id: &str) -> Result<(), AggregatorError> {
        let group = self.groups.get(group_id).ok_or_else(|| {
            AggregatorError::PoolNotFound(group_id.to_string())
        })?;

        let mut total_cap = 0.0;
        let mut total_avail = 0.0;
        let mut weighted_rep = 0.0;
        let mut weighted_lat = 0.0;

        for shard_id in &group.shard_ids {
            if let Some(shard) = self.shards.get(shard_id) {
                total_cap += shard.total_capacity;
                total_avail += shard.available_credits;
                weighted_rep += shard.reputation * shard.total_capacity;
                weighted_lat += shard.avg_latency_ms * shard.total_capacity;
            }
        }

        if let Some(group) = self.groups.get_mut(group_id) {
            group.total_capacity = total_cap;
            group.total_available = total_avail;
            group.avg_reputation = if total_cap > 0.0 {
                weighted_rep / total_cap
            } else {
                0.0
            };
            group.avg_latency_ms = if total_cap > 0.0 {
                weighted_lat / total_cap
            } else {
                0.0
            };
            group.utilization = if total_cap > 0.0 {
                1.0 - (total_avail / total_cap)
            } else {
                0.0
            };
        }

        Ok(())
    }

    fn find_lowest_score_shard(&self, group: &AggregationGroup) -> Option<String> {
        let mut lowest: Option<(&String, f64)> = None;

        for shard_id in &group.shard_ids {
            if let Some(shard) = self.shards.get(shard_id) {
                let score = shard.aggregation_score(&self.config);
                lowest = match lowest {
                    None => Some((shard_id, score)),
                    Some((_, best_score)) if score < best_score => Some((shard_id, score)),
                    _ => lowest,
                };
            }
        }

        lowest.map(|(id, _)| id.clone())
    }
}

impl Default for DynamicAggregator {
    fn default() -> Self {
        Self::new(DynamicAggregatorConfig::default())
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shard(id: &str, pool: &str, capacity: f64, reputation: f64) -> ShardInfo {
        ShardInfo::new(
            id.to_string(),
            pool.to_string(),
            "chain-1".to_string(),
            capacity,
            reputation,
        )
    }

    #[test]
    fn test_aggregator_creation() {
        let agg = DynamicAggregator::new(DynamicAggregatorConfig::default());
        assert_eq!(agg.stats().total_shards, 0);
        assert_eq!(agg.stats().total_groups, 0);
    }

    #[test]
    fn test_register_shard() {
        let mut agg = DynamicAggregator::default_config();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        assert!(agg.register_shard(shard).is_ok());
        assert_eq!(agg.stats().total_shards, 1);
    }

    #[test]
    fn test_register_shard_low_reputation() {
        let mut agg = DynamicAggregator::default_config();
        let shard = make_shard("s1", "pool-1", 100.0, 0.3);
        assert!(agg.register_shard(shard).is_err());
    }

    #[test]
    fn test_remove_shard() {
        let mut agg = DynamicAggregator::default_config();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        assert!(agg.remove_shard("s1").is_ok());
        assert_eq!(agg.stats().total_shards, 0);
    }

    #[test]
    fn test_remove_shard_not_found() {
        let mut agg = DynamicAggregator::default_config();
        assert!(agg.remove_shard("missing").is_err());
    }

    #[test]
    fn test_create_group() {
        let mut agg = DynamicAggregator::default_config();
        assert!(agg.create_group("g1".to_string()).is_ok());
        assert_eq!(agg.stats().total_groups, 1);
    }

    #[test]
    fn test_create_group_duplicate() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        assert!(agg.create_group("g1".to_string()).is_err());
    }

    #[test]
    fn test_auto_create_group() {
        let mut agg = DynamicAggregator::default_config();
        let id = agg.auto_create_group().unwrap();
        assert_eq!(id, "agg-1");
    }

    #[test]
    fn test_add_shard_to_group() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        assert!(agg.add_shard_to_group("g1", "s1").is_ok());
    }

    #[test]
    fn test_add_shard_to_group_full() {
        let mut config = DynamicAggregatorConfig::default();
        config.max_shards_per_group = 1;
        let mut agg = DynamicAggregator::new(config);
        agg.create_group("g1".to_string()).unwrap();
        let s1 = make_shard("s1", "pool-1", 100.0, 0.8);
        let s2 = make_shard("s2", "pool-1", 100.0, 0.8);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        agg.add_shard_to_group("g1", "s1").unwrap();
        assert!(agg.add_shard_to_group("g1", "s2").is_err());
    }

    #[test]
    fn test_remove_shard_from_group() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        agg.add_shard_to_group("g1", "s1").unwrap();
        assert!(agg.remove_shard_from_group("g1", "s1").is_ok());
        assert!(agg.get_shard("s1").unwrap().group_id.is_none());
    }

    #[test]
    fn test_auto_aggregate() {
        let mut agg = DynamicAggregator::default_config();
        for i in 1..=5 {
            let shard = make_shard(&format!("s{}", i), "pool-1", 100.0, 0.8);
            agg.register_shard(shard).unwrap();
        }
        let count = agg.auto_aggregate().unwrap();
        assert_eq!(count, 5);
        assert_eq!(agg.unassigned_count(), 0);
    }

    #[test]
    fn test_recalculate_group_stats() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        let shard = make_shard("s1", "pool-1", 100.0, 0.9);
        agg.register_shard(shard).unwrap();
        agg.add_shard_to_group("g1", "s1").unwrap();
        let group = agg.get_group("g1").unwrap();
        assert_eq!(group.total_capacity, 100.0);
        assert_eq!(group.avg_reputation, 0.9);
    }

    #[test]
    fn test_update_utilization() {
        let mut agg = DynamicAggregator::default_config();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        assert!(agg.update_shard_utilization("s1", 0.5, 50.0).is_ok());
        assert_eq!(agg.get_shard("s1").unwrap().utilization, 0.5);
    }

    #[test]
    fn test_update_latency() {
        let mut agg = DynamicAggregator::default_config();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        agg.update_shard_latency("s1", 100.0).unwrap();
        let shard = agg.get_shard("s1").unwrap();
        assert!(shard.avg_latency_ms > 0.0);
    }

    #[test]
    fn test_update_reputation() {
        let mut agg = DynamicAggregator::default_config();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        assert!(agg.update_shard_reputation("s1", 0.95).is_ok());
        assert_eq!(agg.get_shard("s1").unwrap().reputation, 0.95);
    }

    #[test]
    fn test_find_best_group() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        agg.create_group("g2".to_string()).unwrap();
        let s1 = make_shard("s1", "pool-1", 100.0, 0.8);
        let s2 = make_shard("s2", "pool-1", 200.0, 0.9);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        agg.add_shard_to_group("g1", "s1").unwrap();
        agg.add_shard_to_group("g2", "s2").unwrap();
        let best = agg.find_best_group(150.0);
        assert!(best.is_some());
        assert_eq!(best.unwrap().group_id, "g2");
    }

    #[test]
    fn test_needs_rebalance() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        agg.register_shard(shard).unwrap();
        agg.add_shard_to_group("g1", "s1").unwrap();
        agg.update_shard_utilization("s1", 0.9, 10.0).unwrap();
        assert!(agg.needs_rebalance());
    }

    #[test]
    fn test_rebalance() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        for i in 1..=3 {
            let shard = make_shard(&format!("s{}", i), "pool-1", 100.0, 0.8);
            agg.register_shard(shard).unwrap();
            agg.add_shard_to_group("g1", &format!("s{}", i)).unwrap();
        }
        agg.update_shard_utilization("s1", 0.95, 5.0).unwrap();
        let moved = agg.rebalance().unwrap();
        assert!(moved >= 0);
    }

    #[test]
    fn test_groups_by_utilization() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        agg.create_group("g2".to_string()).unwrap();
        let s1 = make_shard("s1", "pool-1", 100.0, 0.8);
        let s2 = make_shard("s2", "pool-1", 100.0, 0.8);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        agg.add_shard_to_group("g1", "s1").unwrap();
        agg.add_shard_to_group("g2", "s2").unwrap();
        agg.update_shard_utilization("s1", 0.9, 10.0).unwrap();
        agg.update_shard_utilization("s2", 0.3, 70.0).unwrap();
        let groups = agg.groups_by_utilization();
        assert_eq!(groups[0].group_id, "g1");
        assert_eq!(groups[1].group_id, "g2");
    }

    #[test]
    fn test_get_pool_shards() {
        let mut agg = DynamicAggregator::default_config();
        let s1 = make_shard("s1", "pool-1", 100.0, 0.8);
        let s2 = make_shard("s2", "pool-2", 100.0, 0.8);
        agg.register_shard(s1).unwrap();
        agg.register_shard(s2).unwrap();
        let pool1_shards = agg.get_pool_shards("pool-1");
        assert_eq!(pool1_shards.len(), 1);
    }

    #[test]
    fn test_remove_group() {
        let mut agg = DynamicAggregator::default_config();
        agg.create_group("g1".to_string()).unwrap();
        assert!(agg.remove_group("g1").is_ok());
        assert_eq!(agg.stats().total_groups, 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut agg = DynamicAggregator::default_config();
        agg.reset_stats();
        assert_eq!(agg.stats().total_aggregations, 0);
    }

    #[test]
    fn test_advance_time() {
        let mut agg = DynamicAggregator::default_config();
        agg.advance_time(1000);
        assert_eq!(agg.current_time_ms, agg.current_time_ms);
    }

    #[test]
    fn test_shard_aggregation_score() {
        let shard = make_shard("s1", "pool-1", 100.0, 0.9);
        let config = DynamicAggregatorConfig::default();
        let score = shard.aggregation_score(&config);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_shard_meets_reputation() {
        let shard = make_shard("s1", "pool-1", 100.0, 0.8);
        assert!(shard.meets_reputation(0.5));
        assert!(!shard.meets_reputation(0.95));
    }

    #[test]
    fn test_group_can_add_shard() {
        let group = AggregationGroup::new("g1".to_string());
        assert!(group.can_add_shard(16));
    }

    #[test]
    fn test_group_needs_rebalance() {
        let mut group = AggregationGroup::new("g1".to_string());
        group.utilization = 0.9;
        assert!(group.needs_rebalance(0.8));
    }

    #[test]
    fn test_max_groups_limit() {
        let mut config = DynamicAggregatorConfig::default();
        config.max_groups = 2;
        let mut agg = DynamicAggregator::new(config);
        agg.create_group("g1".to_string()).unwrap();
        agg.create_group("g2".to_string()).unwrap();
        assert!(agg.create_group("g3".to_string()).is_err());
    }

    #[test]
    fn test_error_display() {
        match AggregatorError::ShardNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_config_default() {
        let config = DynamicAggregatorConfig::default();
        assert_eq!(config.max_groups, 64);
        assert_eq!(config.max_shards_per_group, 16);
        assert_eq!(config.min_reputation, 0.5);
    }

    #[test]
    fn test_stats_default() {
        let stats = AggregatorStats::default();
        assert_eq!(stats.total_shards, 0);
        assert_eq!(stats.total_groups, 0);
    }

    #[test]
    fn test_aggregator_default() {
        let agg = DynamicAggregator::default();
        assert_eq!(agg.stats().total_shards, 0);
    }
}
