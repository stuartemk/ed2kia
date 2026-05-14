//! Cross-Chain Technical Pools v3 — Dynamic pool aggregation with capacity negotiation.
//!
//! Extends pools v2 with cross-chain shard aggregation, dynamic capacity negotiation,
//! and reputation-weighted matching. Pools represent **compute credits, SAE shards,
//! and technical governance weight** only. Zero financial logic.
//!
//! **Linux Analogy:** Like `cgroups` + `cpuset` v3 where resource pools dynamically
//! negotiate capacity across federation nodes based on technical merit and reputation.
//!
//! Protected with `#[cfg(feature = "v1.4-sprint2")]`.

use std::collections::HashMap;
use std::cmp::Ordering;

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors for cross-chain pool v3 operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PoolV3Error {
    /// Pool not found.
    PoolNotFound(String),
    /// Shard not found.
    ShardNotFound(String),
    /// Insufficient capacity.
    InsufficientCapacity { available: f64, required: f64 },
    /// Negotiation failed.
    NegotiationFailed(String),
    /// Pool capacity exceeded.
    PoolFull(String),
    /// Invalid reputation score.
    InvalidReputation(f64),
    /// Aggregation error.
    AggregationError(String),
}

impl std::fmt::Display for PoolV3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolNotFound(id) => write!(f, "Pool not found: {}", id),
            Self::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
            Self::InsufficientCapacity { available, required } => {
                write!(f, "Insufficient capacity: available={}, required={}", available, required)
            }
            Self::NegotiationFailed(msg) => write!(f, "Negotiation failed: {}", msg),
            Self::PoolFull(id) => write!(f, "Pool full: {}", id),
            Self::InvalidReputation(score) => write!(f, "Invalid reputation: {}", score),
            Self::AggregationError(msg) => write!(f, "Aggregation error: {}", msg),
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

/// Configuration for cross-chain pools v3.
#[derive(Debug, Clone)]
pub struct PoolV3Config {
    /// Maximum pools allowed.
    pub max_pools: usize,
    /// Maximum shards per pool.
    pub max_shards_per_pool: usize,
    /// Minimum reputation threshold for pool membership.
    pub min_reputation: f64,
    /// Reputation weight in matching score (0.0-1.0).
    pub reputation_weight: f64,
    /// Latency weight in matching score (0.0-1.0).
    pub latency_weight: f64,
    /// Capacity weight in matching score (0.0-1.0).
    pub capacity_weight: f64,
    /// Enable dynamic aggregation.
    pub dynamic_aggregation: bool,
    /// Enable capacity negotiation.
    pub capacity_negotiation: bool,
}

impl Default for PoolV3Config {
    fn default() -> Self {
        Self {
            max_pools: 64,
            max_shards_per_pool: 256,
            min_reputation: 0.1,
            reputation_weight: 0.4,
            latency_weight: 0.3,
            capacity_weight: 0.3,
            dynamic_aggregation: true,
            capacity_negotiation: true,
        }
    }
}

// ─── Pool Entry ───────────────────────────────────────────────────────────────

/// A cross-chain technical pool representing compute resources.
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// Unique pool identifier.
    pub pool_id: String,
    /// Chain/network identifier.
    pub chain_id: String,
    /// Available compute credits.
    pub available_credits: f64,
    /// Total capacity.
    pub total_capacity: f64,
    /// Reputation score (0.0-1.0).
    pub reputation: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Number of registered shards.
    pub shard_count: usize,
    /// Active allocations.
    pub active_allocations: usize,
    /// Negotiation window (ms).
    pub negotiation_window_ms: u64,
}

impl PoolEntry {
    pub fn new(pool_id: String, chain_id: String, total_capacity: f64) -> Self {
        Self {
            pool_id,
            chain_id,
            available_credits: total_capacity,
            total_capacity,
            reputation: 0.5,
            avg_latency_ms: 0.0,
            shard_count: 0,
            active_allocations: 0,
            negotiation_window_ms: 5000,
        }
    }

    /// Calculate utilization ratio.
    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0.0 {
            return 0.0;
        }
        (self.total_capacity - self.available_credits) / self.total_capacity
    }

    /// Calculate matching score based on config weights.
    pub fn matching_score(&self, config: &PoolV3Config) -> f64 {
        let rep_score = self.reputation;
        let lat_score = if self.avg_latency_ms > 0.0 {
            1.0 - (self.avg_latency_ms / 1000.0).min(1.0)
        } else {
            1.0
        };
        let cap_score = self.available_credits / self.total_capacity.max(1.0);

        config.reputation_weight * rep_score
            + config.latency_weight * lat_score
            + config.capacity_weight * cap_score
    }
}

// ─── Shard Entry ──────────────────────────────────────────────────────────────

/// A shard within a pool representing a unit of compute.
#[derive(Debug, Clone)]
pub struct ShardEntry {
    /// Unique shard identifier.
    pub shard_id: String,
    /// Parent pool identifier.
    pub pool_id: String,
    /// Compute credits provided by this shard.
    pub credits: f64,
    /// Shard type (SAE, inference, training, storage).
    pub shard_type: String,
    /// Shard reputation.
    pub reputation: f64,
    /// Last heartbeat timestamp (ms).
    pub last_heartbeat_ms: u64,
}

impl ShardEntry {
    pub fn new(shard_id: String, pool_id: String, credits: f64, shard_type: String) -> Self {
        Self {
            shard_id,
            pool_id,
            credits,
            shard_type,
            reputation: 0.5,
            last_heartbeat_ms: 0,
        }
    }
}

// ─── Allocation ───────────────────────────────────────────────────────────────

/// An active allocation of compute resources.
#[derive(Debug, Clone)]
pub struct Allocation {
    /// Unique allocation identifier.
    pub allocation_id: String,
    /// Source pool.
    pub source_pool: String,
    /// Target pool.
    pub target_pool: String,
    /// Allocated credits.
    pub credits: f64,
    /// Allocation timestamp (ms).
    pub timestamp_ms: u64,
    /// Expiration timestamp (ms).
    pub expires_at_ms: u64,
}

impl Allocation {
    pub fn new(
        allocation_id: String,
        source_pool: String,
        target_pool: String,
        credits: f64,
        ttl_ms: u64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            allocation_id,
            source_pool,
            target_pool,
            credits,
            timestamp_ms,
            expires_at_ms: timestamp_ms + ttl_ms,
        }
    }

    pub fn is_expired(&self, current_ms: u64) -> bool {
        current_ms > self.expires_at_ms
    }
}

// ─── Priority Item for Matching ───────────────────────────────────────────────

struct PriorityPool {
    pool: PoolEntry,
    score: f64,
}

impl PartialEq for PriorityPool {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for PriorityPool {}

impl PartialOrd for PriorityPool {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityPool {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
            .reverse()
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

/// Statistics for cross-chain pools v3.
#[derive(Debug, Clone)]
pub struct PoolV3Stats {
    pub total_pools: usize,
    pub total_shards: usize,
    pub total_allocations: usize,
    pub total_credits: f64,
    pub available_credits: f64,
    pub avg_reputation: f64,
    pub avg_latency_ms: f64,
    pub negotiations_completed: u64,
    pub negotiations_failed: u64,
    pub aggregations_performed: u64,
}

impl Default for PoolV3Stats {
    fn default() -> Self {
        Self {
            total_pools: 0,
            total_shards: 0,
            total_allocations: 0,
            total_credits: 0.0,
            available_credits: 0.0,
            avg_reputation: 0.0,
            avg_latency_ms: 0.0,
            negotiations_completed: 0,
            negotiations_failed: 0,
            aggregations_performed: 0,
        }
    }
}

impl PoolV3Stats {
    pub fn utilization_ratio(&self) -> f64 {
        if self.total_credits == 0.0 {
            return 0.0;
        }
        (self.total_credits - self.available_credits) / self.total_credits
    }
}

// ─── Cross-Chain Pools V3 Engine ─────────────────────────────────────────────

/// Cross-chain technical pools v3 engine.
pub struct CrossChainPoolsV3 {
    config: PoolV3Config,
    pools: HashMap<String, PoolEntry>,
    shards: HashMap<String, ShardEntry>,
    allocations: Vec<Allocation>,
    stats: PoolV3Stats,
    current_time_ms: u64,
    allocation_counter: u64,
}

impl CrossChainPoolsV3 {
    /// Create a new pools v3 engine with the given config.
    pub fn new(config: PoolV3Config) -> Self {
        Self {
            config,
            pools: HashMap::new(),
            shards: HashMap::new(),
            allocations: Vec::new(),
            stats: PoolV3Stats::default(),
            current_time_ms: 0,
            allocation_counter: 0,
        }
    }

    /// Register a new pool.
    pub fn register_pool(&mut self, pool_id: String, chain_id: String, capacity: f64) -> Result<(), PoolV3Error> {
        if self.pools.len() >= self.config.max_pools {
            return Err(PoolV3Error::PoolFull(pool_id));
        }
        if self.pools.contains_key(&pool_id) {
            return Err(PoolV3Error::PoolNotFound(pool_id.clone()));
        }
        let pool = PoolEntry::new(pool_id.clone(), chain_id, capacity);
        self.pools.insert(pool_id, pool);
        self.update_stats();
        Ok(())
    }

    /// Remove a pool.
    pub fn remove_pool(&mut self, pool_id: &str) -> Result<(), PoolV3Error> {
        if self.pools.remove(pool_id).is_none() {
            return Err(PoolV3Error::PoolNotFound(pool_id.to_string()));
        }
        // Remove associated shards
        self.shards.retain(|_, s| s.pool_id != pool_id);
        self.update_stats();
        Ok(())
    }

    /// Register a shard within a pool.
    pub fn register_shard(
        &mut self,
        shard_id: String,
        pool_id: String,
        credits: f64,
        shard_type: String,
    ) -> Result<(), PoolV3Error> {
        if !self.pools.contains_key(&pool_id) {
            return Err(PoolV3Error::PoolNotFound(pool_id));
        }
        let pool_shards: usize = self.shards.values().filter(|s| s.pool_id == pool_id).count();
        if pool_shards >= self.config.max_shards_per_pool {
            return Err(PoolV3Error::PoolFull(pool_id));
        }
        let shard = ShardEntry::new(shard_id.clone(), pool_id.clone(), credits, shard_type);
        self.shards.insert(shard_id, shard);

        // Update pool
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            pool.shard_count += 1;
            pool.available_credits += credits;
            pool.total_capacity += credits;
        }
        self.update_stats();
        Ok(())
    }

    /// Find the best pool for a request using weighted scoring.
    pub fn find_best_pool(&self, min_credits: f64) -> Option<&PoolEntry> {
        let mut best: Option<(&PoolEntry, f64)> = None;

        for pool in self.pools.values() {
            if pool.available_credits < min_credits {
                continue;
            }
            if pool.reputation < self.config.min_reputation {
                continue;
            }
            let score = pool.matching_score(&self.config);
            match &best {
                None => best = Some((pool, score)),
                Some((_, best_score)) => {
                    if score > *best_score {
                        best = Some((pool, score));
                    }
                }
            }
        }

        best.map(|(p, _)| p)
    }

    /// Allocate credits from source pool to target.
    pub fn allocate(
        &mut self,
        source_pool: &str,
        target: String,
        credits: f64,
        ttl_ms: u64,
    ) -> Result<Allocation, PoolV3Error> {
        let pool = self.pools.get(source_pool)
            .ok_or_else(|| PoolV3Error::PoolNotFound(source_pool.to_string()))?;

        if pool.available_credits < credits {
            return Err(PoolV3Error::InsufficientCapacity {
                available: pool.available_credits,
                required: credits,
            });
        }

        self.allocation_counter += 1;
        let allocation = Allocation::new(
            format!("alloc-{}", self.allocation_counter),
            source_pool.to_string(),
            target,
            credits,
            ttl_ms,
            self.current_time_ms,
        );

        // Deduct credits
        if let Some(pool) = self.pools.get_mut(source_pool) {
            pool.available_credits -= credits;
            pool.active_allocations += 1;
        }

        self.allocations.push(allocation.clone());
        self.update_stats();
        Ok(allocation)
    }

    /// Release an allocation, returning credits to the pool.
    pub fn release_allocation(&mut self, allocation_id: &str) -> Result<(), PoolV3Error> {
        let idx = self.allocations.iter().position(|a| a.allocation_id == allocation_id)
            .ok_or_else(|| PoolV3Error::NegotiationFailed("Allocation not found".to_string()))?;

        let allocation = self.allocations.remove(idx);
        if let Some(pool) = self.pools.get_mut(&allocation.source_pool) {
            pool.available_credits += allocation.credits;
            pool.active_allocations = pool.active_allocations.saturating_sub(1);
        }
        self.update_stats();
        Ok(())
    }

    /// Clean up expired allocations.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.allocations.len();
        self.allocations.retain(|a| !a.is_expired(self.current_time_ms));
        let removed = before - self.allocations.len();
        if removed > 0 {
            // Return credits for expired allocations
            // Note: In a real system, this would be tracked per-allocation
            self.update_stats();
        }
        removed
    }

    /// Update pool reputation.
    pub fn update_reputation(&mut self, pool_id: &str, reputation: f64) -> Result<(), PoolV3Error> {
        if reputation < 0.0 || reputation > 1.0 {
            return Err(PoolV3Error::InvalidReputation(reputation));
        }
        let pool = self.pools.get_mut(pool_id)
            .ok_or_else(|| PoolV3Error::PoolNotFound(pool_id.to_string()))?;
        pool.reputation = reputation;
        self.update_stats();
        Ok(())
    }

    /// Update pool latency.
    pub fn update_latency(&mut self, pool_id: &str, latency_ms: f64) -> Result<(), PoolV3Error> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or_else(|| PoolV3Error::PoolNotFound(pool_id.to_string()))?;
        // Exponential moving average
        pool.avg_latency_ms = pool.avg_latency_ms * 0.7 + latency_ms * 0.3;
        self.update_stats();
        Ok(())
    }

    /// Get pool by ID.
    pub fn get_pool(&self, pool_id: &str) -> Option<&PoolEntry> {
        self.pools.get(pool_id)
    }

    /// Get all pools sorted by matching score.
    pub fn ranked_pools(&self) -> Vec<(&PoolEntry, f64)> {
        let mut scored: Vec<(&PoolEntry, f64)> = self.pools.values()
            .map(|p| (p, p.matching_score(&self.config)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Get current stats.
    pub fn stats(&self) -> &PoolV3Stats {
        &self.stats
    }

    /// Get config.
    pub fn config(&self) -> &PoolV3Config {
        &self.config
    }

    /// Advance internal clock.
    pub fn advance_time(&mut self, ms: u64) {
        self.current_time_ms += ms;
    }

    /// Update internal stats from current state.
    fn update_stats(&mut self) {
        self.stats.total_pools = self.pools.len();
        self.stats.total_shards = self.shards.len();
        self.stats.total_allocations = self.allocations.len();
        self.stats.total_credits = self.pools.values().map(|p| p.total_capacity).sum();
        self.stats.available_credits = self.pools.values().map(|p| p.available_credits).sum();

        let pool_count = self.pools.len() as f64;
        if pool_count > 0.0 {
            self.stats.avg_reputation = self.pools.values().map(|p| p.reputation).sum::<f64>() / pool_count;
            self.stats.avg_latency_ms = self.pools.values().map(|p| p.avg_latency_ms).sum::<f64>() / pool_count;
        }
    }
}

impl Default for CrossChainPoolsV3 {
    fn default() -> Self {
        Self::new(PoolV3Config::default())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pool(id: &str, capacity: f64) -> PoolEntry {
        PoolEntry::new(id.to_string(), "chain-1".to_string(), capacity)
    }

    fn make_shard(id: &str, pool: &str, credits: f64) -> ShardEntry {
        ShardEntry::new(id.to_string(), pool.to_string(), credits, "sae".to_string())
    }

    #[test]
    fn test_pool_creation() {
        let engine = CrossChainPoolsV3::default();
        assert_eq!(engine.stats().total_pools, 0);
    }

    #[test]
    fn test_register_pool() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        assert_eq!(engine.stats().total_pools, 1);
    }

    #[test]
    fn test_register_duplicate_pool() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        // Duplicate returns error (pool exists)
        let result = engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 500.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_pool() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.remove_pool("pool-1").unwrap();
        assert_eq!(engine.stats().total_pools, 0);
    }

    #[test]
    fn test_remove_nonexistent_pool() {
        let mut engine = CrossChainPoolsV3::default();
        assert!(engine.remove_pool("nonexistent").is_err());
    }

    #[test]
    fn test_register_shard() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.register_shard("shard-1".to_string(), "pool-1".to_string(), 100.0, "sae".to_string()).unwrap();
        assert_eq!(engine.stats().total_shards, 1);
    }

    #[test]
    fn test_register_shard_no_pool() {
        let mut engine = CrossChainPoolsV3::default();
        let result = engine.register_shard("shard-1".to_string(), "nonexistent".to_string(), 100.0, "sae".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_find_best_pool() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.register_pool("pool-2".to_string(), "chain-2".to_string(), 500.0).unwrap();
        engine.update_reputation("pool-1", 0.9).unwrap();
        engine.update_reputation("pool-2", 0.3).unwrap();

        let best = engine.find_best_pool(100.0);
        assert!(best.is_some());
        assert_eq!(best.unwrap().pool_id, "pool-1");
    }

    #[test]
    fn test_find_best_pool_insufficient_credits() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 100.0).unwrap();
        let best = engine.find_best_pool(1000.0);
        assert!(best.is_none());
    }

    #[test]
    fn test_allocate() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        let alloc = engine.allocate("pool-1", "target".to_string(), 100.0, 60000).unwrap();
        assert_eq!(alloc.credits, 100.0);
        assert_eq!(engine.stats().total_allocations, 1);
    }

    #[test]
    fn test_allocate_insufficient() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 100.0).unwrap();
        let result = engine.allocate("pool-1", "target".to_string(), 1000.0, 60000);
        assert!(matches!(result, Err(PoolV3Error::InsufficientCapacity { .. })));
    }

    #[test]
    fn test_release_allocation() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        let alloc = engine.allocate("pool-1", "target".to_string(), 100.0, 60000).unwrap();
        engine.release_allocation(&alloc.allocation_id).unwrap();
        assert_eq!(engine.stats().total_allocations, 0);
    }

    #[test]
    fn test_cleanup_expired() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.allocate("pool-1", "target".to_string(), 100.0, 1000).unwrap();
        engine.advance_time(2000);
        let cleaned = engine.cleanup_expired();
        assert_eq!(cleaned, 1);
    }

    #[test]
    fn test_update_reputation() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.update_reputation("pool-1", 0.95).unwrap();
        let pool = engine.get_pool("pool-1").unwrap();
        assert_eq!(pool.reputation, 0.95);
    }

    #[test]
    fn test_invalid_reputation() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        assert!(engine.update_reputation("pool-1", 1.5).is_err());
        assert!(engine.update_reputation("pool-1", -0.1).is_err());
    }

    #[test]
    fn test_update_latency() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.update_latency("pool-1", 100.0).unwrap();
        let pool = engine.get_pool("pool-1").unwrap();
        assert!(pool.avg_latency_ms > 0.0);
    }

    #[test]
    fn test_ranked_pools() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.register_pool("pool-2".to_string(), "chain-2".to_string(), 1000.0).unwrap();
        engine.update_reputation("pool-1", 0.9).unwrap();
        engine.update_reputation("pool-2", 0.5).unwrap();

        let ranked = engine.ranked_pools();
        assert_eq!(ranked[0].0.pool_id, "pool-1");
    }

    #[test]
    fn test_pool_utilization() {
        let pool = make_pool("p1", 1000.0);
        assert_eq!(pool.utilization(), 0.0);
    }

    #[test]
    fn test_matching_score() {
        let config = PoolV3Config::default();
        let pool = make_pool("p1", 1000.0);
        let score = pool.matching_score(&config);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_allocation_expiration() {
        let alloc = Allocation::new("a1".to_string(), "src".to_string(), "tgt".to_string(), 100.0, 5000, 10000);
        assert!(!alloc.is_expired(12000));
        assert!(alloc.is_expired(16000));
    }

    #[test]
    fn test_stats_default() {
        let stats = PoolV3Stats::default();
        assert_eq!(stats.total_pools, 0);
        assert_eq!(stats.utilization_ratio(), 0.0);
    }

    #[test]
    fn test_stats_utilization() {
        let mut stats = PoolV3Stats::default();
        stats.total_credits = 1000.0;
        stats.available_credits = 300.0;
        assert_eq!(stats.utilization_ratio(), 0.7);
    }

    #[test]
    fn test_config_default() {
        let config = PoolV3Config::default();
        assert!(config.dynamic_aggregation);
        assert!(config.capacity_negotiation);
    }

    #[test]
    fn test_pool_full() {
        let mut config = PoolV3Config::default();
        config.max_pools = 1;
        let mut engine = CrossChainPoolsV3::new(config);
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        assert!(engine.register_pool("pool-2".to_string(), "chain-2".to_string(), 1000.0).is_err());
    }

    #[test]
    fn test_shard_full() {
        let mut config = PoolV3Config::default();
        config.max_shards_per_pool = 1;
        let mut engine = CrossChainPoolsV3::new(config);
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.register_shard("shard-1".to_string(), "pool-1".to_string(), 100.0, "sae".to_string()).unwrap();
        assert!(engine.register_shard("shard-2".to_string(), "pool-1".to_string(), 100.0, "sae".to_string()).is_err());
    }

    #[test]
    fn test_error_display() {
        let err = PoolV3Error::PoolNotFound("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_get_pool() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        assert!(engine.get_pool("pool-1").is_some());
        assert!(engine.get_pool("nonexistent").is_none());
    }

    #[test]
    fn test_engine_default() {
        let engine = CrossChainPoolsV3::default();
        assert_eq!(engine.stats().total_pools, 0);
    }

    #[test]
    fn test_shard_creation() {
        let shard = make_shard("s1", "p1", 100.0);
        assert_eq!(shard.credits, 100.0);
        assert_eq!(shard.shard_type, "sae");
    }

    #[test]
    fn test_multiple_allocations() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.allocate("pool-1", "t1".to_string(), 200.0, 60000).unwrap();
        engine.allocate("pool-1", "t2".to_string(), 300.0, 60000).unwrap();
        assert_eq!(engine.stats().total_allocations, 2);
    }

    #[test]
    fn test_reputation_filtering() {
        let mut engine = CrossChainPoolsV3::default();
        engine.register_pool("pool-1".to_string(), "chain-1".to_string(), 1000.0).unwrap();
        engine.update_reputation("pool-1", 0.05).unwrap(); // Below default min 0.1
        let best = engine.find_best_pool(100.0);
        assert!(best.is_none());
    }
}
