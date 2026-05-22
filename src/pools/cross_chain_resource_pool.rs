//! Cross-Chain Resource Pool — Aggregates technical compute resources across federation shards.
//!
//! This module provides a pool-based abstraction for aggregating SAE shards and compute credits
//! across federation nodes. Resources are matched by reputation v2 scores and historical latency.
//!
//! **Linux Analogy:** Like `cgroups` + `cpuset` but distributed across a federation of nodes,
//! where technical reputation determines resource allocation priority.

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ─── Errors ───

/// Errors for cross-chain resource pool operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PoolError {
    /// Resource type not found in any shard.
    ResourceNotFound(String),
    /// Pool capacity exceeded.
    PoolFull,
    /// Insufficient compute credits to fulfill request.
    InsufficientCredits { available: f64, required: f64 },
    /// Invalid resource type.
    InvalidResourceType(String),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolError::ResourceNotFound(id) => write!(f, "Resource not found: {}", id),
            PoolError::PoolFull => write!(f, "Pool capacity exceeded"),
            PoolError::InsufficientCredits {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient credits: available={}, required={}",
                    available, required
                )
            }
            PoolError::InvalidResourceType(t) => write!(f, "Invalid resource type: {}", t),
        }
    }
}

// ─── Config ───

/// Configuration for the cross-chain resource pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum total compute credits in the pool.
    pub max_pool_credits: f64,
    /// Minimum reputation score to contribute resources.
    pub min_reputation: f64,
    /// Maximum number of shards per pool.
    pub max_shards: usize,
    /// Credit decay rate per hour (0.0 = no decay).
    pub credit_decay_rate: f64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_pool_credits: 1_000_000.0,
            min_reputation: 0.5,
            max_shards: 64,
            credit_decay_rate: 0.001,
        }
    }
}

// ─── Resource Types ───

/// Types of technical resources that can be pooled.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    /// SAE shard compute capacity.
    SaeShard,
    /// Inference compute credits.
    ComputeCredit,
    /// Storage capacity for model artifacts.
    Storage,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::SaeShard => write!(f, "SAE_Shard"),
            ResourceType::ComputeCredit => write!(f, "Compute_Credit"),
            ResourceType::Storage => write!(f, "Storage"),
        }
    }
}

// ─── Shard Entry ───

/// A shard contributing resources to the pool.
#[derive(Debug, Clone)]
pub struct ShardEntry {
    /// Unique shard identifier.
    pub shard_id: String,
    /// Type of resource provided.
    pub resource_type: ResourceType,
    /// Available compute credits.
    pub credits: f64,
    /// Reputation score of the contributing node.
    pub reputation: f64,
    /// Average historical latency in ms.
    pub avg_latency_ms: f64,
    /// Total credits consumed from this shard.
    pub consumed: f64,
}

impl ShardEntry {
    pub fn new(
        shard_id: String,
        resource_type: ResourceType,
        credits: f64,
        reputation: f64,
    ) -> Self {
        Self {
            shard_id,
            resource_type,
            credits,
            reputation,
            avg_latency_ms: 0.0,
            consumed: 0.0,
        }
    }

    /// Available credits after consumption.
    pub fn available(&self) -> f64 {
        (self.credits - self.consumed).max(0.0)
    }

    /// Priority score: higher reputation + lower latency = higher priority.
    pub fn priority_score(&self) -> f64 {
        if self.available() <= 0.0 {
            return 0.0;
        }
        self.reputation * self.available() / (1.0 + self.avg_latency_ms)
    }
}

// ─── Pool Request ───

/// A request for pooled resources.
#[derive(Debug, Clone)]
pub struct PoolRequest {
    /// Unique request identifier.
    pub request_id: String,
    /// Requesting node ID.
    pub node_id: String,
    /// Type of resource requested.
    pub resource_type: ResourceType,
    /// Required compute credits.
    pub required_credits: f64,
    /// Priority level (0-10, higher = more urgent).
    pub priority: u8,
}

// ─── Allocation Result ───

/// Result of a resource allocation from the pool.
#[derive(Debug, Clone)]
pub struct AllocationResult {
    /// Request ID this allocation fulfills.
    pub request_id: String,
    /// Allocated shard entries with their contribution amounts.
    pub allocations: Vec<(String, f64)>,
    /// Total credits allocated.
    pub total_credits: f64,
}

// ─── Pool Stats ───

/// Statistics for the resource pool.
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_shards: usize,
    pub total_credits: f64,
    pub total_allocated: f64,
    pub total_requests: u64,
    pub successful_allocations: u64,
    pub failed_allocations: u64,
    pub avg_allocation_ms: f64,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_shards: 0,
            total_credits: 0.0,
            total_allocated: 0.0,
            total_requests: 0,
            successful_allocations: 0,
            failed_allocations: 0,
            avg_allocation_ms: 0.0,
        }
    }
}

// ─── Main Pool Engine ───

/// Cross-chain resource pool engine.
///
/// Aggregates technical compute resources from multiple federation shards and allocates
/// them based on reputation-weighted priority matching.
pub struct CrossChainResourcePool {
    pub config: PoolConfig,
    shards: HashMap<String, ShardEntry>,
    request_history: VecDeque<AllocationResult>,
    stats: PoolStats,
}

impl CrossChainResourcePool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            shards: HashMap::new(),
            request_history: VecDeque::new(),
            stats: PoolStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(PoolConfig::default())
    }

    // ─── Shard Management ───

    /// Register a shard contributing resources to the pool.
    pub fn register_shard(&mut self, shard: ShardEntry) -> Result<(), PoolError> {
        if shard.reputation < self.config.min_reputation {
            return Err(PoolError::InvalidResourceType(format!(
                "Reputation {} below minimum {}",
                shard.reputation, self.config.min_reputation
            )));
        }
        if self.shards.len() >= self.config.max_shards {
            return Err(PoolError::PoolFull);
        }
        let total: f64 = self.shards.values().map(|s| s.credits).sum();
        if total + shard.credits > self.config.max_pool_credits {
            return Err(PoolError::PoolFull);
        }
        self.shards.insert(shard.shard_id.clone(), shard);
        self.stats.total_shards = self.shards.len();
        self.stats.total_credits = self.shards.values().map(|s| s.available()).sum();
        Ok(())
    }

    /// Remove a shard from the pool.
    pub fn remove_shard(&mut self, shard_id: &str) -> Result<ShardEntry, PoolError> {
        let shard = self
            .shards
            .remove(shard_id)
            .ok_or(PoolError::ResourceNotFound(shard_id.to_string()))?;
        self.stats.total_shards = self.shards.len();
        self.stats.total_credits = self.shards.values().map(|s| s.available()).sum();
        Ok(shard)
    }

    /// Update shard latency.
    pub fn update_latency(&mut self, shard_id: &str, latency_ms: f64) {
        if let Some(shard) = self.shards.get_mut(shard_id) {
            if shard.avg_latency_ms == 0.0 {
                shard.avg_latency_ms = latency_ms;
            } else {
                shard.avg_latency_ms = shard.avg_latency_ms * 0.9 + latency_ms * 0.1;
            }
        }
    }

    /// Update shard reputation.
    pub fn update_reputation(&mut self, shard_id: &str, reputation: f64) {
        if let Some(shard) = self.shards.get_mut(shard_id) {
            shard.reputation = reputation.clamp(0.0, 1.0);
        }
    }

    // ─── Resource Allocation ───

    /// Allocate resources from the pool for a given request.
    pub fn allocate(&mut self, request: &PoolRequest) -> Result<AllocationResult, PoolError> {
        let start = current_timestamp_ms();

        // Find matching shards
        let mut candidates: Vec<&mut ShardEntry> = self
            .shards
            .values_mut()
            .filter(|s| s.resource_type == request.resource_type && s.available() > 0.0)
            .collect();

        // Sort by priority score (reputation * available / latency)
        candidates.sort_by(|a, b| {
            b.priority_score()
                .partial_cmp(&a.priority_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut remaining = request.required_credits;
        let mut allocations = Vec::new();

        for shard in &mut candidates {
            if remaining <= 0.0 {
                break;
            }
            let contribution = remaining.min(shard.available());
            shard.consumed += contribution;
            remaining -= contribution;
            allocations.push((shard.shard_id.clone(), contribution));
        }

        if remaining > 0.0 {
            self.stats.total_requests += 1;
            self.stats.failed_allocations += 1;
            let available: f64 = candidates.iter().map(|s| s.available()).sum();
            return Err(PoolError::InsufficientCredits {
                available,
                required: request.required_credits,
            });
        }

        let total_allocated: f64 = allocations.iter().map(|(_, v)| v).sum();
        let result = AllocationResult {
            request_id: request.request_id.clone(),
            allocations,
            total_credits: total_allocated,
        };

        // Update stats
        let elapsed = current_timestamp_ms() - start;
        self.stats.total_requests += 1;
        self.stats.successful_allocations += 1;
        self.stats.total_allocated += total_allocated;
        self.stats.avg_allocation_ms = (self.stats.avg_allocation_ms
            * (self.stats.total_requests - 1) as f64
            + elapsed as f64)
            / self.stats.total_requests as f64;

        // Record history
        self.request_history.push_back(result.clone());
        if self.request_history.len() > 1000 {
            self.request_history.pop_front();
        }

        Ok(result)
    }

    // ─── Queries ───

    /// Get available credits for a resource type.
    pub fn available_credits(&self, resource_type: &ResourceType) -> f64 {
        self.shards
            .values()
            .filter(|s| &s.resource_type == resource_type)
            .map(|s| s.available())
            .sum()
    }

    /// Get a shard entry.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardEntry> {
        self.shards.get(shard_id)
    }

    /// Get all shards of a given type.
    pub fn get_shards_by_type(&self, resource_type: &ResourceType) -> Vec<&ShardEntry> {
        self.shards
            .values()
            .filter(|s| &s.resource_type == resource_type)
            .collect()
    }

    /// Get pool statistics.
    pub fn get_stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Apply credit decay to all shards.
    pub fn apply_decay(&mut self) {
        for shard in self.shards.values_mut() {
            shard.credits *= 1.0 - self.config.credit_decay_rate;
            shard.consumed *= 1.0 - self.config.credit_decay_rate;
        }
        self.stats.total_credits = self.shards.values().map(|s| s.available()).sum();
    }

    /// Reset pool statistics.
    pub fn reset_stats(&mut self) {
        self.stats = PoolStats::default();
    }
}

impl Default for CrossChainResourcePool {
    fn default() -> Self {
        Self::with_defaults()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = CrossChainResourcePool::with_defaults();
        assert_eq!(pool.get_stats().total_shards, 0);
    }

    #[test]
    fn test_register_shard() {
        let mut pool = CrossChainResourcePool::with_defaults();
        let shard = ShardEntry::new("shard-1".to_string(), ResourceType::SaeShard, 100.0, 0.8);
        pool.register_shard(shard).unwrap();
        assert_eq!(pool.get_stats().total_shards, 1);
    }

    #[test]
    fn test_register_low_reputation() {
        let mut pool = CrossChainResourcePool::with_defaults();
        let shard = ShardEntry::new("shard-1".to_string(), ResourceType::SaeShard, 100.0, 0.3);
        assert!(pool.register_shard(shard).is_err());
    }

    #[test]
    fn test_allocate_resources() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::ComputeCredit,
            200.0,
            0.9,
        ))
        .unwrap();
        pool.register_shard(ShardEntry::new(
            "shard-2".to_string(),
            ResourceType::ComputeCredit,
            150.0,
            0.7,
        ))
        .unwrap();

        let request = PoolRequest {
            request_id: "req-1".to_string(),
            node_id: "node-1".to_string(),
            resource_type: ResourceType::ComputeCredit,
            required_credits: 100.0,
            priority: 5,
        };

        let result = pool.allocate(&request).unwrap();
        assert_eq!(result.total_credits, 100.0);
        assert!(!result.allocations.is_empty());
    }

    #[test]
    fn test_allocate_insufficient() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::ComputeCredit,
            50.0,
            0.9,
        ))
        .unwrap();

        let request = PoolRequest {
            request_id: "req-1".to_string(),
            node_id: "node-1".to_string(),
            resource_type: ResourceType::ComputeCredit,
            required_credits: 100.0,
            priority: 5,
        };

        assert!(pool.allocate(&request).is_err());
    }

    #[test]
    fn test_priority_scoring() {
        let mut pool = CrossChainResourcePool::with_defaults();
        // High rep, low latency = highest priority
        pool.register_shard(ShardEntry::new(
            "shard-a".to_string(),
            ResourceType::ComputeCredit,
            100.0,
            0.95,
        ))
        .unwrap();
        pool.update_latency("shard-a", 10.0);

        pool.register_shard(ShardEntry::new(
            "shard-b".to_string(),
            ResourceType::ComputeCredit,
            100.0,
            0.6,
        ))
        .unwrap();
        pool.update_latency("shard-b", 50.0);

        let request = PoolRequest {
            request_id: "req-1".to_string(),
            node_id: "node-1".to_string(),
            resource_type: ResourceType::ComputeCredit,
            required_credits: 50.0,
            priority: 5,
        };

        let result = pool.allocate(&request).unwrap();
        // Should allocate from shard-a first (higher priority)
        assert_eq!(result.allocations[0].0, "shard-a");
    }

    #[test]
    fn test_remove_shard() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::SaeShard,
            100.0,
            0.8,
        ))
        .unwrap();
        let removed = pool.remove_shard("shard-1").unwrap();
        assert_eq!(removed.shard_id, "shard-1");
        assert_eq!(pool.get_stats().total_shards, 0);
    }

    #[test]
    fn test_available_credits() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::ComputeCredit,
            200.0,
            0.8,
        ))
        .unwrap();
        assert_eq!(pool.available_credits(&ResourceType::ComputeCredit), 200.0);
    }

    #[test]
    fn test_credit_decay() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::ComputeCredit,
            100.0,
            0.8,
        ))
        .unwrap();
        pool.apply_decay();
        assert!(pool.available_credits(&ResourceType::ComputeCredit) < 100.0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::ComputeCredit,
            200.0,
            0.8,
        ))
        .unwrap();

        let request = PoolRequest {
            request_id: "req-1".to_string(),
            node_id: "node-1".to_string(),
            resource_type: ResourceType::ComputeCredit,
            required_credits: 50.0,
            priority: 5,
        };
        pool.allocate(&request).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_allocations, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut pool = CrossChainResourcePool::with_defaults();
        pool.reset_stats();
        assert_eq!(pool.get_stats().total_requests, 0);
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(format!("{}", ResourceType::SaeShard), "SAE_Shard");
        assert_eq!(format!("{}", ResourceType::ComputeCredit), "Compute_Credit");
        assert_eq!(format!("{}", ResourceType::Storage), "Storage");
    }

    #[test]
    fn test_error_display() {
        let err = PoolError::ResourceNotFound("x".to_string());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_pool_full() {
        let mut pool = CrossChainResourcePool::new(PoolConfig {
            max_shards: 1,
            ..Default::default()
        });
        pool.register_shard(ShardEntry::new(
            "shard-1".to_string(),
            ResourceType::SaeShard,
            100.0,
            0.8,
        ))
        .unwrap();
        assert!(pool
            .register_shard(ShardEntry::new(
                "shard-2".to_string(),
                ResourceType::SaeShard,
                100.0,
                0.8
            ))
            .is_err());
    }

    #[test]
    fn test_shard_available() {
        let mut shard = ShardEntry::new("s1".to_string(), ResourceType::SaeShard, 100.0, 0.8);
        assert_eq!(shard.available(), 100.0);
        shard.consumed = 30.0;
        assert_eq!(shard.available(), 70.0);
    }

    #[test]
    fn test_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_shards, 64);
        assert_eq!(config.min_reputation, 0.5);
    }
}
