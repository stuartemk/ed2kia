//! Commitment Pool v1 — Optimized commitment pooling for batch ZKP verification.
//!
//! Provides commitment pooling, base precomputation, and criterion benchmark hooks
//! for optimizing multi-curve ZKP proof generation and verification.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  CommitmentPool                             │
//! │  ┌──────────────────┐  ┌──────────────────────────────┐    │
//! │  │  PoolEntry       │  │  BasePrecomputation          │    │
//! │  │  - commitment    │  │  - bases: Vec<f64>           │    │
//! │  │  - curve         │  │  - initialized: bool         │    │
//! │  │  - added_at_ms   │  │  - algorithm: PrecomputeAlgo │    │
//! │  └──────────────────┘  └──────────────────────────────┘    │
//! ├─────────────────────────────────────────────────────────────┤
//! │              BenchmarkHooks                                  │
//! │  ┌──────────────────┐  ┌──────────────────────────────┐    │
//! │  │  PoolBenchmark   │  │  CriterionAdapter            │    │
//! │  │  - gen_time_ms   │  │  - group_name                │    │
//! │  │  - verify_time   │  │  - samples                   │    │
//! │  │  - reduction     │  │  - measurements              │    │
//! │  └──────────────────┘  └──────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! Feature-gated behind `cfg(feature = "v2.0-sprint2")`.

mod internal {
    use std::collections::HashMap;
    use std::fmt;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Commitment pool operation errors.
    #[derive(Debug, Clone, PartialEq)]
    pub enum PoolError {
        /// Pool capacity exceeded.
        CapacityExceeded,
        /// Invalid commitment value.
        InvalidCommitment,
        /// Curve mismatch in pool.
        CurveMismatch,
        /// Precomputation not initialized.
        NotInitialized,
        /// Benchmark already running.
        BenchmarkInProgress,
        /// No benchmark data available.
        NoBenchmarkData,
    }

    impl fmt::Display for PoolError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PoolError::CapacityExceeded => write!(f, "commitment pool capacity exceeded"),
                PoolError::InvalidCommitment => write!(f, "invalid commitment value"),
                PoolError::CurveMismatch => write!(f, "curve mismatch in commitment pool"),
                PoolError::NotInitialized => write!(f, "base precomputation not initialized"),
                PoolError::BenchmarkInProgress => write!(f, "benchmark already in progress"),
                PoolError::NoBenchmarkData => write!(f, "no benchmark data available"),
            }
        }
    }

    impl std::error::Error for PoolError {}

    // ============================================================================
    // Precomputation Algorithm
    // ============================================================================

    /// Algorithm used for base precomputation.
    #[derive(Debug, Clone, PartialEq)]
    pub enum PrecomputeAlgo {
        /// Pedersen commitment scheme.
        Pedersen,
        /// Inner-product argument.
        InnerProduct,
        /// Lagrange interpolation.
        Lagrange,
    }

    impl fmt::Display for PrecomputeAlgo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PrecomputeAlgo::Pedersen => write!(f, "pedersen"),
                PrecomputeAlgo::InnerProduct => write!(f, "inner_product"),
                PrecomputeAlgo::Lagrange => write!(f, "lagrange"),
            }
        }
    }

    // ============================================================================
    // Pool Entry
    // ============================================================================

    /// A single commitment entry in the pool.
    #[derive(Debug, Clone)]
    pub struct PoolEntry {
        /// Commitment hash (simulated as string).
        pub commitment: String,
        /// Curve identifier.
        pub curve: String,
        /// Timestamp when added to pool (ms).
        pub added_at_ms: u64,
        /// Commitment value.
        pub value: f64,
    }

    impl PoolEntry {
        /// Create a new pool entry.
        pub fn new(commitment: String, curve: String, value: f64, added_at_ms: u64) -> Self {
            Self {
                commitment,
                curve,
                added_at_ms,
                value,
            }
        }
    }

    // ============================================================================
    // Base Precomputation
    // ============================================================================

    /// Precomputed bases for commitment scheme optimization.
    pub struct BasePrecomputation {
        /// Precomputed base values.
        bases: Vec<f64>,
        /// Whether precomputation is complete.
        initialized: bool,
        /// Algorithm used.
        algorithm: PrecomputeAlgo,
        /// Number of bases.
        count: usize,
    }

    impl BasePrecomputation {
        /// Create new precomputation structure.
        pub fn new(count: usize, algorithm: PrecomputeAlgo) -> Self {
            Self {
                bases: vec![0.0; count],
                initialized: false,
                algorithm,
                count,
            }
        }

        /// Initialize precomputation.
        pub fn initialize(&mut self) {
            for i in 0..self.count {
                self.bases[i] = Self::compute_base(i, &self.algorithm);
            }
            self.initialized = true;
        }

        /// Compute a single base value.
        fn compute_base(i: usize, algorithm: &PrecomputeAlgo) -> f64 {
            match algorithm {
                PrecomputeAlgo::Pedersen => {
                    // Simulated Pedersen base: hash-like value
                    let h = Self::simple_hash(format!("pedersen_base_{}", i));
                    (h % 1000000) as f64 / 1000000.0
                }
                PrecomputeAlgo::InnerProduct => {
                    // Simulated inner product base
                    let h = Self::simple_hash(format!("inner_product_base_{}", i));
                    (h % 1000000) as f64 / 1000000.0
                }
                PrecomputeAlgo::Lagrange => {
                    // Simulated Lagrange base
                    let h = Self::simple_hash(format!("lagrange_base_{}", i));
                    (h % 1000000) as f64 / 1000000.0
                }
            }
        }

        /// Simple hash function for deterministic base generation.
        fn simple_hash(input: String) -> u64 {
            let mut hash: u64 = 5381;
            for byte in input.bytes() {
                hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
            }
            hash
        }

        /// Get a precomputed base by index.
        pub fn get_base(&self, index: usize) -> Result<f64, PoolError> {
            if !self.initialized {
                return Err(PoolError::NotInitialized);
            }
            if index >= self.count {
                return Err(PoolError::InvalidCommitment);
            }
            Ok(self.bases[index])
        }

        /// Check if precomputation is initialized.
        pub fn is_initialized(&self) -> bool {
            self.initialized
        }

        /// Get the number of precomputed bases.
        pub fn base_count(&self) -> usize {
            self.count
        }

        /// Get the algorithm used.
        pub fn algorithm(&self) -> &PrecomputeAlgo {
            &self.algorithm
        }

        /// Recompute all bases.
        pub fn recompute(&mut self) {
            self.initialize();
        }
    }

    // ============================================================================
    // Commitment Pool
    // ============================================================================

    /// Optimized commitment pool for batch ZKP verification.
    pub struct CommitmentPool {
        /// Pool entries.
        entries: Vec<PoolEntry>,
        /// Maximum pool capacity.
        capacity: usize,
        /// Current memory usage estimate (bytes).
        memory_usage: usize,
        /// Precomputed bases.
        precomputation: Option<BasePrecomputation>,
        /// Pool statistics.
        stats: PoolStats,
    }

    impl CommitmentPool {
        /// Create a new commitment pool.
        pub fn new(capacity: usize) -> Self {
            Self {
                entries: Vec::with_capacity(capacity),
                capacity,
                memory_usage: 0,
                precomputation: None,
                stats: PoolStats::new(),
            }
        }

        /// Add a commitment to the pool.
        pub fn add(&mut self, entry: PoolEntry) -> Result<(), PoolError> {
            if self.entries.len() >= self.capacity {
                return Err(PoolError::CapacityExceeded);
            }
            self.entries.push(entry.clone());
            self.memory_usage += Self::estimate_entry_size(&entry);
            self.stats.total_additions += 1;
            Ok(())
        }

        /// Estimate memory size of a pool entry.
        fn estimate_entry_size(entry: &PoolEntry) -> usize {
            entry.commitment.len() + entry.curve.len() + 24 // String data + u64 + f64
        }

        /// Get the number of entries in the pool.
        pub fn len(&self) -> usize {
            self.entries.len()
        }

        /// Check if the pool is empty.
        pub fn is_empty(&self) -> bool {
            self.entries.is_empty()
        }

        /// Get all entries.
        pub fn entries(&self) -> &[PoolEntry] {
            &self.entries
        }

        /// Clear the pool.
        pub fn clear(&mut self) {
            self.entries.clear();
            self.memory_usage = 0;
            self.stats.total_clears += 1;
        }

        /// Initialize precomputation.
        pub fn init_precomputation(&mut self, count: usize, algorithm: PrecomputeAlgo) {
            let mut precomp = BasePrecomputation::new(count, algorithm);
            precomp.initialize();
            self.precomputation = Some(precomp);
            self.stats.precomputation_inits += 1;
        }

        /// Get precomputed base.
        pub fn get_precomputed_base(&self, index: usize) -> Result<f64, PoolError> {
            match &self.precomputation {
                Some(precomp) => precomp.get_base(index),
                None => Err(PoolError::NotInitialized),
            }
        }

        /// Compute aggregate commitment from all entries.
        pub fn aggregate_commitment(&self) -> String {
            if self.entries.is_empty() {
                return "empty".to_string();
            }
            let mut combined = String::new();
            for entry in &self.entries {
                combined.push_str(&entry.commitment);
            }
            // Simple hash of combined commitments
            let hash = BasePrecomputation::simple_hash(combined);
            format!("{:016x}", hash)
        }

        /// Get current memory usage.
        pub fn memory_usage(&self) -> usize {
            self.memory_usage
        }

        /// Get pool statistics.
        pub fn stats(&self) -> &PoolStats {
            &self.stats
        }

        /// Get pool utilization ratio.
        pub fn utilization(&self) -> f64 {
            if self.capacity == 0 {
                return 0.0;
            }
            self.entries.len() as f64 / self.capacity as f64
        }
    }

    // ============================================================================
    // Pool Statistics
    // ============================================================================

    /// Statistics for commitment pool operations.
    #[derive(Debug, Clone)]
    pub struct PoolStats {
        /// Total additions.
        pub total_additions: usize,
        /// Total clears.
        pub total_clears: usize,
        /// Total precomputation initializations.
        pub precomputation_inits: usize,
        /// Total aggregations.
        pub total_aggregations: usize,
    }

    impl PoolStats {
        /// Create new empty stats.
        pub fn new() -> Self {
            Self {
                total_additions: 0,
                total_clears: 0,
                precomputation_inits: 0,
                total_aggregations: 0,
            }
        }

        /// Reset all stats.
        pub fn reset(&mut self) {
            self.total_additions = 0;
            self.total_clears = 0;
            self.precomputation_inits = 0;
            self.total_aggregations = 0;
        }
    }

    impl Default for PoolStats {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Benchmark Hooks
    // ============================================================================

    /// Benchmark result for commitment pool operations.
    #[derive(Debug, Clone)]
    pub struct PoolBenchmark {
        /// Proof generation time (ms).
        pub gen_time_ms: f64,
        /// Verification time (ms).
        pub verify_time_ms: f64,
        /// Number of commitments in batch.
        pub batch_size: usize,
        /// Reduction ratio (original / batch).
        pub reduction_ratio: f64,
        /// Memory usage during benchmark (bytes).
        pub memory_bytes: usize,
    }

    impl PoolBenchmark {
        /// Create a new benchmark result.
        pub fn new(
            gen_time_ms: f64,
            verify_time_ms: f64,
            batch_size: usize,
            reduction_ratio: f64,
            memory_bytes: usize,
        ) -> Self {
            Self {
                gen_time_ms,
                verify_time_ms,
                batch_size,
                reduction_ratio,
                memory_bytes,
            }
        }
    }

    /// Criterion-compatible benchmark adapter.
    pub struct CriterionAdapter {
        /// Benchmark group name.
        pub group_name: String,
        /// Collected measurements.
        pub measurements: Vec<PoolBenchmark>,
    }

    impl CriterionAdapter {
        /// Create a new criterion adapter.
        pub fn new(group_name: String) -> Self {
            Self {
                group_name,
                measurements: Vec::new(),
            }
        }

        /// Add a measurement.
        pub fn add_measurement(&mut self, benchmark: PoolBenchmark) {
            self.measurements.push(benchmark);
        }

        /// Get average generation time.
        pub fn avg_gen_time_ms(&self) -> Result<f64, PoolError> {
            if self.measurements.is_empty() {
                return Err(PoolError::NoBenchmarkData);
            }
            let sum: f64 = self.measurements.iter().map(|m| m.gen_time_ms).sum();
            Ok(sum / self.measurements.len() as f64)
        }

        /// Get average verification time.
        pub fn avg_verify_time_ms(&self) -> Result<f64, PoolError> {
            if self.measurements.is_empty() {
                return Err(PoolError::NoBenchmarkData);
            }
            let sum: f64 = self.measurements.iter().map(|m| m.verify_time_ms).sum();
            Ok(sum / self.measurements.len() as f64)
        }

        /// Get average reduction ratio.
        pub fn avg_reduction_ratio(&self) -> Result<f64, PoolError> {
            if self.measurements.is_empty() {
                return Err(PoolError::NoBenchmarkData);
            }
            let sum: f64 = self.measurements.iter().map(|m| m.reduction_ratio).sum();
            Ok(sum / self.measurements.len() as f64)
        }

        /// Get number of measurements.
        pub fn measurement_count(&self) -> usize {
            self.measurements.len()
        }

        /// Clear all measurements.
        pub fn clear(&mut self) {
            self.measurements.clear();
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    mod tests {
        use super::*;

        #[test]
        fn test_pool_entry_new() {
            let entry = PoolEntry::new("commit_001".to_string(), "BN254".to_string(), 0.5, 1000);
            assert_eq!(entry.commitment, "commit_001");
            assert_eq!(entry.curve, "BN254");
            assert_eq!(entry.value, 0.5);
            assert_eq!(entry.added_at_ms, 1000);
        }

        #[test]
        fn test_precompute_algo_display() {
            assert_eq!(format!("{}", PrecomputeAlgo::Pedersen), "pedersen");
            assert_eq!(format!("{}", PrecomputeAlgo::InnerProduct), "inner_product");
            assert_eq!(format!("{}", PrecomputeAlgo::Lagrange), "lagrange");
        }

        #[test]
        fn test_base_precomputation_new() {
            let precomp = BasePrecomputation::new(10, PrecomputeAlgo::Pedersen);
            assert_eq!(precomp.base_count(), 10);
            assert!(!precomp.is_initialized());
            assert_eq!(*precomp.algorithm(), PrecomputeAlgo::Pedersen);
        }

        #[test]
        fn test_base_precomputation_initialize() {
            let mut precomp = BasePrecomputation::new(5, PrecomputeAlgo::Pedersen);
            precomp.initialize();
            assert!(precomp.is_initialized());
        }

        #[test]
        fn test_base_precomputation_get_base_before_init() {
            let precomp = BasePrecomputation::new(5, PrecomputeAlgo::Pedersen);
            assert_eq!(precomp.get_base(0), Err(PoolError::NotInitialized));
        }

        #[test]
        fn test_base_precomputation_get_base_after_init() {
            let mut precomp = BasePrecomputation::new(5, PrecomputeAlgo::Pedersen);
            precomp.initialize();
            let base = precomp.get_base(0).unwrap();
            assert!(base >= 0.0 && base <= 1.0);
        }

        #[test]
        fn test_base_precomputation_get_base_out_of_range() {
            let mut precomp = BasePrecomputation::new(5, PrecomputeAlgo::Pedersen);
            precomp.initialize();
            assert_eq!(precomp.get_base(5), Err(PoolError::InvalidCommitment));
        }

        #[test]
        fn test_base_precomputation_recompute() {
            let mut precomp = BasePrecomputation::new(5, PrecomputeAlgo::Pedersen);
            precomp.initialize();
            let base1 = precomp.get_base(0).unwrap();
            precomp.recompute();
            let base2 = precomp.get_base(0).unwrap();
            assert_eq!(base1, base2); // Deterministic
        }

        #[test]
        fn test_commitment_pool_new() {
            let pool = CommitmentPool::new(10);
            assert!(pool.is_empty());
            assert_eq!(pool.len(), 0);
            assert_eq!(pool.memory_usage(), 0);
        }

        #[test]
        fn test_commitment_pool_add() {
            let mut pool = CommitmentPool::new(10);
            let entry = PoolEntry::new("c1".to_string(), "BN254".to_string(), 0.5, 1000);
            pool.add(entry).unwrap();
            assert_eq!(pool.len(), 1);
            assert!(!pool.is_empty());
        }

        #[test]
        fn test_commitment_pool_capacity() {
            let mut pool = CommitmentPool::new(2);
            pool.add(PoolEntry::new(
                "c1".to_string(),
                "BN254".to_string(),
                0.5,
                1000,
            ))
            .unwrap();
            pool.add(PoolEntry::new(
                "c2".to_string(),
                "BN254".to_string(),
                0.6,
                1001,
            ))
            .unwrap();
            assert_eq!(
                pool.add(PoolEntry::new(
                    "c3".to_string(),
                    "BN254".to_string(),
                    0.7,
                    1002
                )),
                Err(PoolError::CapacityExceeded)
            );
        }

        #[test]
        fn test_commitment_pool_clear() {
            let mut pool = CommitmentPool::new(10);
            pool.add(PoolEntry::new(
                "c1".to_string(),
                "BN254".to_string(),
                0.5,
                1000,
            ))
            .unwrap();
            pool.clear();
            assert!(pool.is_empty());
            assert_eq!(pool.memory_usage(), 0);
        }

        #[test]
        fn test_commitment_pool_utilization() {
            let mut pool = CommitmentPool::new(10);
            assert_eq!(pool.utilization(), 0.0);
            pool.add(PoolEntry::new(
                "c1".to_string(),
                "BN254".to_string(),
                0.5,
                1000,
            ))
            .unwrap();
            assert!((pool.utilization() - 0.1).abs() < 0.001);
        }

        #[test]
        fn test_commitment_pool_aggregate_empty() {
            let pool = CommitmentPool::new(10);
            assert_eq!(pool.aggregate_commitment(), "empty");
        }

        #[test]
        fn test_commitment_pool_aggregate() {
            let mut pool = CommitmentPool::new(10);
            pool.add(PoolEntry::new(
                "c1".to_string(),
                "BN254".to_string(),
                0.5,
                1000,
            ))
            .unwrap();
            pool.add(PoolEntry::new(
                "c2".to_string(),
                "BN254".to_string(),
                0.6,
                1001,
            ))
            .unwrap();
            let agg = pool.aggregate_commitment();
            assert_ne!(agg, "empty");
            assert_eq!(agg.len(), 16); // Hex hash
        }

        #[test]
        fn test_commitment_pool_precomputation() {
            let mut pool = CommitmentPool::new(10);
            pool.init_precomputation(5, PrecomputeAlgo::Pedersen);
            let base = pool.get_precomputed_base(0).unwrap();
            assert!(base >= 0.0 && base <= 1.0);
        }

        #[test]
        fn test_commitment_pool_precomputation_not_init() {
            let pool = CommitmentPool::new(10);
            assert_eq!(pool.get_precomputed_base(0), Err(PoolError::NotInitialized));
        }

        #[test]
        fn test_pool_stats_new() {
            let stats = PoolStats::new();
            assert_eq!(stats.total_additions, 0);
            assert_eq!(stats.total_clears, 0);
        }

        #[test]
        fn test_pool_stats_reset() {
            let mut stats = PoolStats::new();
            stats.total_additions = 5;
            stats.total_clears = 2;
            stats.reset();
            assert_eq!(stats.total_additions, 0);
            assert_eq!(stats.total_clears, 0);
        }

        #[test]
        fn test_pool_stats_default() {
            let stats = PoolStats::default();
            assert_eq!(stats.total_additions, 0);
        }

        #[test]
        fn test_benchmark_new() {
            let bench = PoolBenchmark::new(10.0, 5.0, 100, 0.8, 1024);
            assert_eq!(bench.gen_time_ms, 10.0);
            assert_eq!(bench.verify_time_ms, 5.0);
            assert_eq!(bench.batch_size, 100);
        }

        #[test]
        fn test_criterion_adapter_new() {
            let adapter = CriterionAdapter::new("test_group".to_string());
            assert_eq!(adapter.group_name, "test_group");
            assert_eq!(adapter.measurement_count(), 0);
        }

        #[test]
        fn test_criterion_adapter_add_measurement() {
            let mut adapter = CriterionAdapter::new("test".to_string());
            adapter.add_measurement(PoolBenchmark::new(10.0, 5.0, 100, 0.8, 1024));
            assert_eq!(adapter.measurement_count(), 1);
        }

        #[test]
        fn test_criterion_adapter_avg_gen_time() {
            let mut adapter = CriterionAdapter::new("test".to_string());
            adapter.add_measurement(PoolBenchmark::new(10.0, 5.0, 100, 0.8, 1024));
            adapter.add_measurement(PoolBenchmark::new(20.0, 8.0, 200, 0.9, 2048));
            let avg = adapter.avg_gen_time_ms().unwrap();
            assert!((avg - 15.0).abs() < 0.001);
        }

        #[test]
        fn test_criterion_adapter_no_data() {
            let adapter = CriterionAdapter::new("test".to_string());
            assert_eq!(adapter.avg_gen_time_ms(), Err(PoolError::NoBenchmarkData));
        }

        #[test]
        fn test_criterion_adapter_clear() {
            let mut adapter = CriterionAdapter::new("test".to_string());
            adapter.add_measurement(PoolBenchmark::new(10.0, 5.0, 100, 0.8, 1024));
            adapter.clear();
            assert_eq!(adapter.measurement_count(), 0);
        }

        #[test]
        fn test_error_display() {
            let err = PoolError::CapacityExceeded;
            let msg = format!("{}", err);
            assert!(msg.contains("capacity"));
        }

        #[test]
        fn test_full_pool_lifecycle() {
            let mut pool = CommitmentPool::new(100);

            // Initial state
            assert!(pool.is_empty());
            assert_eq!(pool.utilization(), 0.0);

            // Add entries
            for i in 0..10 {
                pool.add(PoolEntry::new(
                    format!("commit_{}", i),
                    "BN254".to_string(),
                    i as f64 * 0.1,
                    1000 + i as u64,
                ))
                .unwrap();
            }
            assert_eq!(pool.len(), 10);
            assert!((pool.utilization() - 0.1).abs() < 0.001);

            // Init precomputation
            pool.init_precomputation(16, PrecomputeAlgo::Pedersen);
            let base = pool.get_precomputed_base(0).unwrap();
            assert!(base >= 0.0);

            // Aggregate
            let agg = pool.aggregate_commitment();
            assert_ne!(agg, "empty");

            // Check stats
            assert_eq!(pool.stats().total_additions, 10);
            assert_eq!(pool.stats().precomputation_inits, 1);

            // Clear
            pool.clear();
            assert!(pool.is_empty());
            assert_eq!(pool.stats().total_clears, 1);
        }
    }
}

pub use internal::{
    BasePrecomputation, CommitmentPool, CriterionAdapter, PoolBenchmark, PoolEntry, PoolError,
    PoolStats, PrecomputeAlgo,
};
