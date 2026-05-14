//! Cross-Chain Pools v4 — Adaptive pool aggregation with exponential decay balancing.
//!
//! Extends pools v3 with adaptive load balancing using exponential decay windows,
//! demand prediction, and confidence-weighted capacity allocation.
//! Pools represent **compute credits, SAE shards, and technical governance weight** only.
//! Zero financial logic.
//!
//! **Linux Analogy:** Like `cgroups` + `cpuset` v4 where resource pools self-tune
//! capacity allocation using historical load patterns and predictive scaling.
//!
//! Protected with `#[cfg(feature = "v1.5-sprint1")]`.

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::{HashMap, VecDeque};

    // ─── Errors ───────────────────────────────────────────────────────────────────

    /// Errors for cross-chain pool v4 operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum PoolV4Error {
        /// Pool not found.
        PoolNotFound(String),
        /// Chain not found.
        ChainNotFound(String),
        /// Insufficient capacity.
        InsufficientCapacity { available: f64, required: f64 },
        /// Confidence too low for routing.
        LowConfidence { confidence: f64, threshold: f64 },
        /// Pool capacity exceeded.
        PoolFull(String),
        /// Invalid reputation score.
        InvalidReputation(f64),
        /// Aggregation error.
        AggregationError(String),
    }

    impl std::fmt::Display for PoolV4Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::PoolNotFound(id) => write!(f, "Pool not found: {}", id),
                Self::ChainNotFound(id) => write!(f, "Chain not found: {}", id),
                Self::InsufficientCapacity { available, required } => {
                    write!(f, "Insufficient capacity: available={}, required={}", available, required)
                }
                Self::LowConfidence { confidence, threshold } => {
                    write!(f, "Confidence too low: {:.4} < {:.4}", confidence, threshold)
                }
                Self::PoolFull(id) => write!(f, "Pool full: {}", id),
                Self::InvalidReputation(score) => write!(f, "Invalid reputation: {}", score),
                Self::AggregationError(msg) => write!(f, "Aggregation error: {}", msg),
            }
        }
    }

    // ─── Config ───────────────────────────────────────────────────────────────────

    /// Configuration for cross-chain pools v4.
    #[derive(Debug, Clone)]
    pub struct PoolV4Config {
        /// Maximum pools allowed.
        pub max_pools: usize,
        /// Maximum shards per pool.
        pub max_shards_per_pool: usize,
        /// Minimum reputation threshold.
        pub min_reputation: f64,
        /// Decay window in seconds for load metrics.
        pub decay_window_secs: u64,
        /// Exponential decay factor (0.0-1.0).
        pub decay_factor: f64,
        /// Minimum confidence for adaptive routing.
        pub min_confidence: f64,
        /// Enable demand prediction.
        pub demand_prediction: bool,
        /// Prediction horizon (number of samples).
        pub prediction_horizon: usize,
    }

    impl Default for PoolV4Config {
        fn default() -> Self {
            Self {
                max_pools: 128,
                max_shards_per_pool: 512,
                min_reputation: 0.1,
                decay_window_secs: 10,
                decay_factor: 0.85,
                min_confidence: 0.0,
                demand_prediction: true,
                prediction_horizon: 20,
            }
        }
    }

    // ─── Chain Resource Slot ──────────────────────────────────────────────────────

    /// A resource slot on a specific chain within a pool.
    #[derive(Debug, Clone)]
    pub struct ChainResourceSlot {
        /// Chain identifier.
        pub chain_id: String,
        /// Available compute credits.
        pub available_credits: f64,
        /// Total capacity.
        pub total_capacity: f64,
        /// Current load factor (0.0-1.0).
        pub load_factor: f64,
        /// EMA-smoothed load.
        pub ema_load: f64,
        /// Load history for prediction.
        pub load_history: VecDeque<f64>,
        /// Reputation score.
        pub reputation: f64,
        /// Confidence in current metrics.
        pub confidence: f64,
    }

    impl ChainResourceSlot {
        pub fn new(chain_id: String, total_capacity: f64, reputation: f64) -> Self {
            Self {
                chain_id,
                available_credits: total_capacity,
                total_capacity,
                load_factor: 0.0,
                ema_load: 0.0,
                load_history: VecDeque::with_capacity(50),
                reputation,
                confidence: 0.0,
            }
        }

        /// Update load with EMA smoothing.
        pub fn update_load(&mut self, new_load: f64, alpha: f64, max_samples: usize) {
            self.load_factor = new_load;
            if self.ema_load == 0.0 {
                self.ema_load = new_load;
            } else {
                self.ema_load = alpha * new_load + (1.0 - alpha) * self.ema_load;
            }
            self.load_history.push_back(new_load);
            while self.load_history.len() > max_samples {
                self.load_history.pop_front();
            }
            // Update confidence based on history depth
            let depth = self.load_history.len() as f64;
            self.confidence = (depth / max_samples as f64).min(1.0);
        }

        /// Predict future load using EMA trend.
        pub fn predict_load(&self, horizon: usize, alpha: f64) -> f64 {
            if self.load_history.len() < 2 {
                return self.ema_load;
            }
            let samples: Vec<f64> = self.load_history.iter().copied().collect();
            let n = samples.len();
            let mut predicted = self.ema_load;
            for _ in 0..horizon.min(n) {
                predicted = alpha * predicted + (1.0 - alpha) * self.ema_load;
            }
            predicted
        }

        /// Compute utilization score (lower is better for routing).
        pub fn utilization_score(&self) -> f64 {
            if self.total_capacity == 0.0 {
                return 1.0;
            }
            (1.0 - self.available_credits / self.total_capacity) * self.ema_load
        }

        /// Weighted score combining utilization, reputation, and confidence.
        pub fn routing_score(&self, reputation_weight: f64, confidence_weight: f64) -> f64 {
            let util = self.utilization_score();
            util * (1.0 - reputation_weight - confidence_weight)
                + (1.0 - self.reputation) * reputation_weight
                + (1.0 - self.confidence) * confidence_weight
        }
    }

    // ─── Pool Entry v4 ────────────────────────────────────────────────────────────

    /// A cross-chain technical pool v4 with adaptive balancing.
    #[derive(Debug, Clone)]
    pub struct PoolEntryV4 {
        /// Unique pool identifier.
        pub pool_id: String,
        /// Chain resource slots.
        pub slots: HashMap<String, ChainResourceSlot>,
        /// Total aggregated capacity.
        pub total_capacity: f64,
        /// Total available credits.
        pub available_credits: f64,
        /// Shard count.
        pub shard_count: usize,
        /// Active flag.
        pub active: bool,
    }

    impl PoolEntryV4 {
        pub fn new(pool_id: String) -> Self {
            Self {
                pool_id,
                slots: HashMap::new(),
                total_capacity: 0.0,
                available_credits: 0.0,
                shard_count: 0,
                active: true,
            }
        }

        pub fn add_slot(&mut self, chain_id: String, capacity: f64, reputation: f64) {
            let slot = ChainResourceSlot::new(chain_id.clone(), capacity, reputation);
            self.total_capacity += capacity;
            self.available_credits += capacity;
            self.slots.insert(chain_id, slot);
        }

        pub fn update_slot_load(&mut self, chain_id: &str, load: f64, alpha: f64, max_samples: usize) -> Result<(), PoolV4Error> {
            let slot = self.slots.get_mut(chain_id)
                .ok_or(PoolV4Error::ChainNotFound(chain_id.to_string()))?;
            let old_available = slot.available_credits;
            slot.update_load(load, alpha, max_samples);
            slot.available_credits = slot.total_capacity * (1.0 - slot.ema_load);
            let delta = slot.available_credits - old_available;
            self.available_credits = (self.available_credits + delta).max(0.0);
            Ok(())
        }

        pub fn best_slot(&self, reputation_weight: f64, confidence_weight: f64) -> Option<&ChainResourceSlot> {
            self.slots.values()
                .min_by(|a, b| {
                    a.routing_score(reputation_weight, confidence_weight)
                        .partial_cmp(&b.routing_score(reputation_weight, confidence_weight))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        }

        pub fn can_allocate(&self, required: f64) -> bool {
            self.available_credits >= required && self.active
        }

        pub fn allocate(&mut self, amount: f64) -> Result<(), PoolV4Error> {
            if !self.can_allocate(amount) {
                return Err(PoolV4Error::InsufficientCapacity {
                    available: self.available_credits,
                    required: amount,
                });
            }
            self.available_credits -= amount;
            Ok(())
        }

        pub fn release(&mut self, amount: f64) {
            self.available_credits += amount;
        }
    }

    // ─── Pool Stats v4 ────────────────────────────────────────────────────────────

    /// Statistics for pool v4 operations.
    #[derive(Debug, Clone)]
    pub struct PoolV4Stats {
        pub total_allocations: usize,
        pub total_releases: usize,
        pub total_failures: usize,
        pub total_predictions: usize,
        pub avg_confidence: f64,
        pub fallback_count: usize,
    }

    impl Default for PoolV4Stats {
        fn default() -> Self {
            Self {
                total_allocations: 0,
                total_releases: 0,
                total_failures: 0,
                total_predictions: 0,
                avg_confidence: 1.0,
                fallback_count: 0,
            }
        }
    }

    impl PoolV4Stats {
        pub fn record_allocation(&mut self, confidence: f64) {
            self.total_allocations += 1;
            self.avg_confidence =
                (self.avg_confidence * (self.total_allocations - 1) as f64 + confidence)
                    / self.total_allocations as f64;
        }

        pub fn record_failure(&mut self) {
            self.total_failures += 1;
        }

        pub fn record_prediction(&mut self) {
            self.total_predictions += 1;
        }

        pub fn record_fallback(&mut self) {
            self.fallback_count += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ─── Cross-Chain Pools Manager v4 ─────────────────────────────────────────────

    /// Manager for cross-chain technical pools v4.
    pub struct CrossChainPoolsV4 {
        config: PoolV4Config,
        pools: HashMap<String, PoolEntryV4>,
        pub stats: PoolV4Stats,
    }

    impl CrossChainPoolsV4 {
        pub fn new(config: PoolV4Config) -> Self {
            Self {
                config,
                pools: HashMap::new(),
                stats: PoolV4Stats::default(),
            }
        }

        /// Create a new pool.
        pub fn create_pool(&mut self, pool_id: String) -> Result<(), PoolV4Error> {
            if self.pools.len() >= self.config.max_pools {
                return Err(PoolV4Error::PoolFull(pool_id.clone()));
            }
            if self.pools.contains_key(&pool_id) {
                return Err(PoolV4Error::AggregationError(format!("Pool {} already exists", pool_id)));
            }
            self.pools.insert(pool_id.clone(), PoolEntryV4::new(pool_id));
            Ok(())
        }

        /// Add a chain resource slot to a pool.
        pub fn add_chain_slot(
            &mut self,
            pool_id: &str,
            chain_id: String,
            capacity: f64,
            reputation: f64,
        ) -> Result<(), PoolV4Error> {
            if !(0.0..=1.0).contains(&reputation) {
                return Err(PoolV4Error::InvalidReputation(reputation));
            }
            let pool = self.pools.get_mut(pool_id)
                .ok_or(PoolV4Error::PoolNotFound(pool_id.to_string()))?;
            pool.add_slot(chain_id, capacity, reputation);
            Ok(())
        }

        /// Update load for a chain slot.
        pub fn update_load(
            &mut self,
            pool_id: &str,
            chain_id: &str,
            load: f64,
        ) -> Result<(), PoolV4Error> {
            let alpha = 1.0 - self.config.decay_factor;
            let horizon = self.config.prediction_horizon;
            let pool = self.pools.get_mut(pool_id)
                .ok_or(PoolV4Error::PoolNotFound(pool_id.to_string()))?;
            pool.update_slot_load(chain_id, load, alpha, horizon)
        }

        /// Allocate credits from the best available slot.
        pub fn allocate(
            &mut self,
            pool_id: &str,
            amount: f64,
        ) -> Result<String, PoolV4Error> {
            // Check confidence first with immutable access
            let pool_ref = self.pools.get(pool_id)
                .ok_or(PoolV4Error::PoolNotFound(pool_id.to_string()))?;

            if let Some(best) = pool_ref.best_slot(0.4, 0.3) {
                if best.confidence < self.config.min_confidence {
                    self.stats.record_fallback();
                    return Err(PoolV4Error::LowConfidence {
                        confidence: best.confidence,
                        threshold: self.config.min_confidence,
                    });
                }
            }

            let best_chain = pool_ref.best_slot(0.4, 0.3)
                .map(|s| s.chain_id.clone())
                .ok_or(PoolV4Error::PoolNotFound(pool_id.to_string()))?;

            // Now get mutable reference for allocation
            let pool = self.pools.get_mut(pool_id).unwrap();
            pool.allocate(amount)?;
            let confidence = pool.best_slot(0.4, 0.3)
                .map(|s| s.confidence)
                .unwrap_or(0.0);
            self.stats.record_allocation(confidence);
            Ok(best_chain)
        }

        /// Release credits back to a pool.
        pub fn release(&mut self, pool_id: &str, amount: f64) -> Result<(), PoolV4Error> {
            let pool = self.pools.get_mut(pool_id)
                .ok_or(PoolV4Error::PoolNotFound(pool_id.to_string()))?;
            pool.release(amount);
            self.stats.total_releases += 1;
            Ok(())
        }

        /// Predict demand for a pool.
        pub fn predict_demand(&mut self, pool_id: &str) -> Result<f64, PoolV4Error> {
            let pool = self.pools.get(pool_id)
                .ok_or(PoolV4Error::PoolNotFound(pool_id.to_string()))?;
            let mut total_predicted = 0.0;
            for slot in pool.slots.values() {
                let predicted = slot.predict_load(self.config.prediction_horizon, 1.0 - self.config.decay_factor);
                total_predicted += predicted * slot.total_capacity;
            }
            self.stats.record_prediction();
            Ok(total_predicted)
        }

        /// Get pool by ID.
        pub fn get_pool(&self, pool_id: &str) -> Option<&PoolEntryV4> {
            self.pools.get(pool_id)
        }

        /// Get active pool count.
        pub fn active_pool_count(&self) -> usize {
            self.pools.values().filter(|p| p.active).count()
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for CrossChainPoolsV4 {
        fn default() -> Self {
            Self::new(PoolV4Config::default())
        }
    }

    // ─── Tests ────────────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> PoolV4Config {
            PoolV4Config {
                max_pools: 16,
                max_shards_per_pool: 64,
                min_reputation: 0.1,
                decay_window_secs: 10,
                decay_factor: 0.85,
                min_confidence: 0.5,
                demand_prediction: true,
                prediction_horizon: 10,
            }
        }

        #[test]
        fn test_engine_creation() {
            let engine = CrossChainPoolsV4::default();
            assert_eq!(engine.active_pool_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = make_config();
            let engine = CrossChainPoolsV4::new(config);
            assert_eq!(engine.config.max_pools, 16);
        }

        #[test]
        fn test_create_pool() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            assert_eq!(engine.active_pool_count(), 1);
        }

        #[test]
        fn test_create_pool_duplicate() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            let result = engine.create_pool("p1".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn test_create_pool_max_reached() {
            let config = PoolV4Config { max_pools: 2, ..Default::default() };
            let mut engine = CrossChainPoolsV4::new(config);
            engine.create_pool("p1".to_string()).unwrap();
            engine.create_pool("p2".to_string()).unwrap();
            assert!(engine.create_pool("p3".to_string()).is_err());
        }

        #[test]
        fn test_add_chain_slot() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            let pool = engine.get_pool("p1").unwrap();
            assert_eq!(pool.slots.len(), 1);
            assert_eq!(pool.total_capacity, 100.0);
        }

        #[test]
        fn test_add_chain_slot_invalid_reputation() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            let result = engine.add_chain_slot("p1", "eth".to_string(), 100.0, 1.5);
            assert!(result.is_err());
        }

        #[test]
        fn test_update_load() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            engine.update_load("p1", "eth", 0.3).unwrap();
            let pool = engine.get_pool("p1").unwrap();
            let slot = pool.slots.get("eth").unwrap();
            assert!(slot.ema_load > 0.0);
        }

        #[test]
        fn test_allocate() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            // Warm up confidence
            for i in 0..10 {
                engine.update_load("p1", "eth", 0.1 + i as f64 * 0.05).unwrap();
            }
            let chain = engine.allocate("p1", 20.0).unwrap();
            assert_eq!(chain, "eth");
        }

        #[test]
        fn test_allocate_insufficient() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 10.0, 0.9).unwrap();
            for _ in 0..10 {
                engine.update_load("p1", "eth", 0.1).unwrap();
            }
            let result = engine.allocate("p1", 100.0);
            assert!(matches!(result, Err(PoolV4Error::InsufficientCapacity { .. })));
        }

        #[test]
        fn test_release() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            for _ in 0..10 {
                engine.update_load("p1", "eth", 0.1).unwrap();
            }
            engine.allocate("p1", 20.0).unwrap();
            engine.release("p1", 10.0).unwrap();
            assert!(engine.stats.total_releases == 1);
        }

        #[test]
        fn test_predict_demand() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            for i in 0..10 {
                engine.update_load("p1", "eth", 0.2 + i as f64 * 0.05).unwrap();
            }
            let demand = engine.predict_demand("p1").unwrap();
            assert!(demand > 0.0);
        }

        #[test]
        fn test_low_confidence_fallback() {
            let config = PoolV4Config {
                min_confidence: 0.9,
                ..PoolV4Config::default()
            };
            let mut engine = CrossChainPoolsV4::new(config);
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            // Don't warm up — confidence stays low
            let result = engine.allocate("p1", 10.0);
            assert!(matches!(result, Err(PoolV4Error::LowConfidence { .. })));
            assert!(engine.stats.fallback_count > 0);
        }

        #[test]
        fn test_ema_smoothing() {
            let mut slot = ChainResourceSlot::new("test".to_string(), 100.0, 0.9);
            slot.update_load(0.5, 0.2, 10);
            slot.update_load(0.3, 0.2, 10);
            assert!(slot.ema_load < 0.5);
            assert!(slot.ema_load > 0.3);
        }

        #[test]
        fn test_load_prediction() {
            let mut slot = ChainResourceSlot::new("test".to_string(), 100.0, 0.9);
            for i in 0..10 {
                slot.update_load(0.1 + i as f64 * 0.05, 0.3, 20);
            }
            let predicted = slot.predict_load(5, 0.3);
            assert!(predicted > 0.0);
        }

        #[test]
        fn test_routing_score() {
            let slot = ChainResourceSlot::new("test".to_string(), 100.0, 0.9);
            let score = slot.routing_score(0.4, 0.3);
            assert!(score >= 0.0);
            assert!(score <= 1.0);
        }

        #[test]
        fn test_utilization_score() {
            let slot = ChainResourceSlot::new("test".to_string(), 100.0, 0.9);
            let score = slot.utilization_score();
            assert_eq!(score, 0.0); // fresh slot, full capacity
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            for _ in 0..10 {
                engine.update_load("p1", "eth", 0.1).unwrap();
            }
            engine.allocate("p1", 10.0).unwrap();
            assert_eq!(engine.stats.total_allocations, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            for _ in 0..10 {
                engine.update_load("p1", "eth", 0.1).unwrap();
            }
            engine.allocate("p1", 10.0).unwrap();
            engine.reset_stats();
            assert_eq!(engine.stats.total_allocations, 0);
        }

        #[test]
        fn test_config_default() {
            let config = PoolV4Config::default();
            assert_eq!(config.decay_window_secs, 10);
            assert_eq!(config.decay_factor, 0.85);
        }

        #[test]
        fn test_stats_default() {
            let stats = PoolV4Stats::default();
            assert_eq!(stats.total_allocations, 0);
            assert_eq!(stats.avg_confidence, 1.0);
        }

        #[test]
        fn test_error_display() {
            let e = PoolV4Error::PoolNotFound("x".to_string());
            assert!(format!("{}", e).contains("x"));
        }

        #[test]
        fn test_pool_new() {
            let pool = PoolEntryV4::new("p1".to_string());
            assert!(pool.active);
            assert_eq!(pool.total_capacity, 0.0);
        }

        #[test]
        fn test_slot_new() {
            let slot = ChainResourceSlot::new("eth".to_string(), 100.0, 0.9);
            assert_eq!(slot.total_capacity, 100.0);
            assert_eq!(slot.confidence, 0.0);
        }

        #[test]
        fn test_engine_default() {
            let engine = CrossChainPoolsV4::default();
            assert_eq!(engine.pools.len(), 0);
        }

        #[test]
        fn test_multi_chain_pool() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            engine.add_chain_slot("p1", "sol".to_string(), 80.0, 0.8).unwrap();
            let pool = engine.get_pool("p1").unwrap();
            assert_eq!(pool.slots.len(), 2);
            assert_eq!(pool.total_capacity, 180.0);
        }

        #[test]
        fn test_best_slot_selection() {
            let mut engine = CrossChainPoolsV4::default();
            engine.create_pool("p1".to_string()).unwrap();
            engine.add_chain_slot("p1", "eth".to_string(), 100.0, 0.9).unwrap();
            engine.add_chain_slot("p1", "sol".to_string(), 100.0, 0.5).unwrap();
            for _ in 0..10 {
                engine.update_load("p1", "eth", 0.8).unwrap();
                engine.update_load("p1", "sol", 0.2).unwrap();
            }
            let best = engine.get_pool("p1").unwrap().best_slot(0.4, 0.3);
            assert!(best.is_some());
            assert_eq!(best.unwrap().chain_id, "sol");
        }

        #[test]
        fn test_confidence_builds_over_time() {
            let mut slot = ChainResourceSlot::new("test".to_string(), 100.0, 0.9);
            assert_eq!(slot.confidence, 0.0); // initial — no data yet
            slot.update_load(0.5, 0.2, 100);
            assert!(slot.confidence > 0.0); // starts building
            assert!(slot.confidence < 1.0); // not yet warm
            for _ in 0..100 {
                slot.update_load(0.5, 0.2, 100);
            }
            assert!((slot.confidence - 1.0).abs() < 0.01);
        }
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;
