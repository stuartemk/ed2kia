//! Circuit Optimization v1 — Constraint pooling, Pedersen precomputation, and benchmark hooks.
//!
//! Provides optimizations for ZKP circuit generation:
//! - `ConstraintPool` — Reusable constraint allocation pool
//! - `PedersenPrecompute` — Precomputed Pedersen commitment bases
//! - `CircuitBenchmark` — Benchmark hooks for constraint count and gen time
//!
//! Feature-gated behind `cfg(feature = "v1.9-sprint1")`.

mod internal {
    use std::fmt;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Circuit optimization errors.
    #[derive(Debug, Clone, PartialEq)]
    pub enum CircuitOptError {
        /// Constraint pool exhausted.
        PoolExhausted,
        /// Invalid constraint count (must be > 0).
        InvalidConstraintCount,
        /// Precomputation not initialized.
        NotPrecomputed,
        /// Benchmark already started.
        BenchmarkAlreadyStarted,
        /// Benchmark not started.
        BenchmarkNotStarted,
    }

    impl fmt::Display for CircuitOptError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                CircuitOptError::PoolExhausted => {
                    write!(f, "circuit_opt: constraint pool exhausted")
                }
                CircuitOptError::InvalidConstraintCount => {
                    write!(f, "circuit_opt: constraint count must be > 0")
                }
                CircuitOptError::NotPrecomputed => {
                    write!(f, "circuit_opt: precomputation not initialized")
                }
                CircuitOptError::BenchmarkAlreadyStarted => {
                    write!(f, "circuit_opt: benchmark already started")
                }
                CircuitOptError::BenchmarkNotStarted => {
                    write!(f, "circuit_opt: benchmark not started")
                }
            }
        }
    }

    impl std::error::Error for CircuitOptError {}

    // ============================================================================
    // Constraint Pool
    // ============================================================================

    /// A single constraint slot in the pool.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ConstraintSlot {
        /// Constraint index.
        pub index: usize,
        /// Constraint weight.
        pub weight: f64,
        /// Is currently allocated.
        pub allocated: bool,
    }

    impl ConstraintSlot {
        pub fn new(index: usize, weight: f64) -> Self {
            Self {
                index,
                weight,
                allocated: false,
            }
        }
    }

    /// Pool of reusable constraint slots for circuit generation.
    #[derive(Debug)]
    pub struct ConstraintPool {
        /// All constraint slots.
        slots: Vec<ConstraintSlot>,
        /// Next available index.
        next_index: usize,
        /// Total allocations performed.
        total_allocations: u64,
        /// Total deallocations performed.
        total_deallocations: u64,
    }

    impl ConstraintPool {
        /// Create a new constraint pool with the given capacity.
        ///
        /// # Arguments
        /// * `capacity` - Maximum number of constraint slots
        ///
        /// # Errors
        /// * `CircuitOptError::InvalidConstraintCount` if capacity is 0
        pub fn new(capacity: usize) -> Result<Self, CircuitOptError> {
            if capacity == 0 {
                return Err(CircuitOptError::InvalidConstraintCount);
            }
            let slots: Vec<ConstraintSlot> =
                (0..capacity).map(|i| ConstraintSlot::new(i, 1.0)).collect();
            Ok(Self {
                slots,
                next_index: 0,
                total_allocations: 0,
                total_deallocations: 0,
            })
        }

        /// Allocate a constraint slot from the pool.
        ///
        /// # Errors
        /// * `CircuitOptError::PoolExhausted` if no slots available
        pub fn allocate(&mut self) -> Result<&mut ConstraintSlot, CircuitOptError> {
            // Find next unallocated slot
            for slot in self.slots.iter_mut() {
                if !slot.allocated {
                    slot.allocated = true;
                    self.total_allocations += 1;
                    return Ok(slot);
                }
            }
            Err(CircuitOptError::PoolExhausted)
        }

        /// Deallocate a constraint slot by index.
        ///
        /// # Arguments
        /// * `index` - Constraint slot index to free
        pub fn deallocate(&mut self, index: usize) -> Result<(), CircuitOptError> {
            if let Some(slot) = self.slots.get_mut(index) {
                if slot.allocated {
                    slot.allocated = false;
                    self.total_deallocations += 1;
                }
                Ok(())
            } else {
                Err(CircuitOptError::InvalidConstraintCount)
            }
        }

        /// Get the number of available (unallocated) slots.
        pub fn available(&self) -> usize {
            self.slots.iter().filter(|s| !s.allocated).count()
        }

        /// Get the total pool capacity.
        pub fn capacity(&self) -> usize {
            self.slots.len()
        }

        /// Get the utilization rate [0.0, 1.0].
        pub fn utilization(&self) -> f64 {
            if self.slots.is_empty() {
                return 0.0;
            }
            let allocated = self.slots.iter().filter(|s| s.allocated).count();
            allocated as f64 / self.slots.len() as f64
        }

        /// Get total allocations performed.
        pub fn total_allocations(&self) -> u64 {
            self.total_allocations
        }

        /// Get total deallocations performed.
        pub fn total_deallocations(&self) -> u64 {
            self.total_deallocations
        }

        /// Reset all slots to unallocated.
        pub fn reset(&mut self) {
            for slot in self.slots.iter_mut() {
                slot.allocated = false;
            }
        }

        /// Update weight for a specific slot.
        pub fn set_weight(&mut self, index: usize, weight: f64) -> Result<(), CircuitOptError> {
            if let Some(slot) = self.slots.get_mut(index) {
                slot.weight = weight;
                Ok(())
            } else {
                Err(CircuitOptError::InvalidConstraintCount)
            }
        }
    }

    // ============================================================================
    // Pedersen Precomputation
    // ============================================================================

    /// Precomputed Pedersen commitment bases for faster circuit generation.
    #[derive(Debug, Clone)]
    pub struct PedersenPrecompute {
        /// Precomputed base values (simulated as f64 for mock).
        bases: Vec<f64>,
        /// Number of precomputed values.
        count: usize,
        /// Is initialized.
        initialized: bool,
    }

    impl PedersenPrecompute {
        /// Create a new Pedersen precomputation cache.
        ///
        /// # Arguments
        /// * `count` - Number of base values to precompute
        pub fn new(count: usize) -> Self {
            Self {
                bases: vec![0.0; count],
                count,
                initialized: false,
            }
        }

        /// Initialize the precomputation with deterministic base values.
        pub fn initialize(&mut self) {
            for (i, base) in self.bases.iter_mut().enumerate() {
                *base = Self::compute_base(i);
            }
            self.initialized = true;
        }

        /// Get a precomputed base value by index.
        ///
        /// # Errors
        /// * `CircuitOptError::NotPrecomputed` if not initialized
        pub fn get_base(&self, index: usize) -> Result<f64, CircuitOptError> {
            if !self.initialized {
                return Err(CircuitOptError::NotPrecomputed);
            }
            self.bases
                .get(index)
                .copied()
                .ok_or(CircuitOptError::InvalidConstraintCount)
        }

        /// Check if precomputation is initialized.
        pub fn is_initialized(&self) -> bool {
            self.initialized
        }

        /// Get the number of precomputed bases.
        pub fn count(&self) -> usize {
            self.count
        }

        /// Compute a deterministic base value for index i.
        fn compute_base(i: usize) -> f64 {
            // Deterministic hash-like function for mock
            let mut val = (i as f64).fract() * 1000.7;
            val = val.sin() * val.cos() + 0.5;
            val.abs().clamp(0.0, 1.0)
        }

        /// Recompute all bases (for testing).
        pub fn recompute(&mut self) {
            self.initialize();
        }
    }

    impl Default for PedersenPrecompute {
        fn default() -> Self {
            Self::new(256)
        }
    }

    // ============================================================================
    // Circuit Benchmark
    // ============================================================================

    /// Benchmark result for a circuit generation run.
    #[derive(Debug, Clone, PartialEq)]
    pub struct BenchmarkResult {
        /// Constraint count.
        pub constraint_count: usize,
        /// Generation time in milliseconds.
        pub gen_time_ms: f64,
        /// Average constraint weight.
        pub avg_weight: f64,
        /// Pool utilization during generation.
        pub pool_utilization: f64,
    }

    /// Benchmark hooks for measuring circuit generation performance.
    #[derive(Debug)]
    pub struct CircuitBenchmark {
        /// Is currently running.
        running: bool,
        /// Constraint count at start.
        start_constraints: usize,
        /// Constraint count at end.
        end_constraints: usize,
        /// Generation time in ms.
        gen_time_ms: f64,
        /// Sum of constraint weights.
        weight_sum: f64,
        /// Count of recorded weights.
        weight_count: usize,
        /// Pool utilization snapshot.
        pool_utilization: f64,
        /// Historical results.
        results: Vec<BenchmarkResult>,
    }

    impl CircuitBenchmark {
        /// Create a new circuit benchmark.
        pub fn new() -> Self {
            Self {
                running: false,
                start_constraints: 0,
                end_constraints: 0,
                gen_time_ms: 0.0,
                weight_sum: 0.0,
                weight_count: 0,
                pool_utilization: 0.0,
                results: Vec::new(),
            }
        }

        /// Start a benchmark run.
        ///
        /// # Arguments
        /// * `initial_constraints` - Constraint count at start
        ///
        /// # Errors
        /// * `CircuitOptError::BenchmarkAlreadyStarted` if already running
        pub fn start(&mut self, initial_constraints: usize) -> Result<(), CircuitOptError> {
            if self.running {
                return Err(CircuitOptError::BenchmarkAlreadyStarted);
            }
            self.running = true;
            self.start_constraints = initial_constraints;
            self.weight_sum = 0.0;
            self.weight_count = 0;
            Ok(())
        }

        /// Record a constraint weight during benchmark.
        ///
        /// # Errors
        /// * `CircuitOptError::BenchmarkNotStarted` if not running
        pub fn record_weight(&mut self, weight: f64) -> Result<(), CircuitOptError> {
            if !self.running {
                return Err(CircuitOptError::BenchmarkNotStarted);
            }
            self.weight_sum += weight;
            self.weight_count += 1;
            Ok(())
        }

        /// Stop the benchmark run and record results.
        ///
        /// # Arguments
        /// * `final_constraints` - Constraint count at end
        /// * `elapsed_ms` - Elapsed time in milliseconds
        /// * `pool_utilization` - Pool utilization during run
        ///
        /// # Errors
        /// * `CircuitOptError::BenchmarkNotStarted` if not running
        pub fn stop(
            &mut self,
            final_constraints: usize,
            elapsed_ms: f64,
            pool_utilization: f64,
        ) -> Result<BenchmarkResult, CircuitOptError> {
            if !self.running {
                return Err(CircuitOptError::BenchmarkNotStarted);
            }
            self.running = false;
            self.end_constraints = final_constraints;
            self.gen_time_ms = elapsed_ms;
            self.pool_utilization = pool_utilization;

            let constraint_count = final_constraints;
            let total_weights = if self.weight_count > 0 {
                self.weight_count
            } else {
                1
            };
            let avg_weight = self.weight_sum / total_weights as f64;

            let result = BenchmarkResult {
                constraint_count,
                gen_time_ms: elapsed_ms,
                avg_weight,
                pool_utilization,
            };

            self.results.push(result.clone());
            Ok(result)
        }

        /// Get the latest benchmark result.
        pub fn latest(&self) -> Option<&BenchmarkResult> {
            self.results.last()
        }

        /// Get all historical results.
        pub fn results(&self) -> &[BenchmarkResult] {
            &self.results
        }

        /// Get average generation time across all runs.
        pub fn avg_gen_time_ms(&self) -> f64 {
            if self.results.is_empty() {
                return 0.0;
            }
            let sum: f64 = self.results.iter().map(|r| r.gen_time_ms).sum();
            sum / self.results.len() as f64
        }

        /// Check if benchmark is currently running.
        pub fn is_running(&self) -> bool {
            self.running
        }

        /// Clear all historical results.
        pub fn clear(&mut self) {
            self.results.clear();
        }
    }

    impl Default for CircuitBenchmark {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_pool_creation() {
            let pool = ConstraintPool::new(10).unwrap();
            assert_eq!(pool.capacity(), 10);
            assert_eq!(pool.available(), 10);
        }

        #[test]
        fn test_pool_zero_capacity() {
            assert_eq!(
                ConstraintPool::new(0).unwrap_err(),
                CircuitOptError::InvalidConstraintCount
            );
        }

        #[test]
        fn test_pool_allocate() {
            let mut pool = ConstraintPool::new(5).unwrap();
            let slot = pool.allocate().unwrap();
            assert!(slot.allocated);
            assert_eq!(pool.available(), 4);
        }

        #[test]
        fn test_pool_exhausted() {
            let mut pool = ConstraintPool::new(2).unwrap();
            pool.allocate().unwrap();
            pool.allocate().unwrap();
            assert_eq!(pool.allocate().unwrap_err(), CircuitOptError::PoolExhausted);
        }

        #[test]
        fn test_pool_deallocate() {
            let mut pool = ConstraintPool::new(3).unwrap();
            pool.allocate().unwrap();
            assert_eq!(pool.available(), 2);
            pool.deallocate(0).unwrap();
            assert_eq!(pool.available(), 3);
        }

        #[test]
        fn test_pool_utilization() {
            let mut pool = ConstraintPool::new(4).unwrap();
            assert_eq!(pool.utilization(), 0.0);
            pool.allocate().unwrap();
            assert!((pool.utilization() - 0.25).abs() < 0.01);
        }

        #[test]
        fn test_pool_reset() {
            let mut pool = ConstraintPool::new(3).unwrap();
            pool.allocate().unwrap();
            pool.allocate().unwrap();
            pool.reset();
            assert_eq!(pool.available(), 3);
        }

        #[test]
        fn test_pool_set_weight() {
            let mut pool = ConstraintPool::new(3).unwrap();
            pool.set_weight(0, 2.5).unwrap();
            assert_eq!(pool.slots[0].weight, 2.5);
        }

        #[test]
        fn test_pool_total_allocations() {
            let mut pool = ConstraintPool::new(5).unwrap();
            pool.allocate().unwrap();
            pool.allocate().unwrap();
            assert_eq!(pool.total_allocations(), 2);
        }

        #[test]
        fn test_pedersen_new() {
            let precompute = PedersenPrecompute::new(128);
            assert_eq!(precompute.count(), 128);
            assert!(!precompute.is_initialized());
        }

        #[test]
        fn test_pedersen_initialize() {
            let mut precompute = PedersenPrecompute::new(64);
            precompute.initialize();
            assert!(precompute.is_initialized());
        }

        #[test]
        fn test_pedersen_get_base_before_init() {
            let precompute = PedersenPrecompute::new(64);
            assert_eq!(
                precompute.get_base(0).unwrap_err(),
                CircuitOptError::NotPrecomputed
            );
        }

        #[test]
        fn test_pedersen_get_base_after_init() {
            let mut precompute = PedersenPrecompute::new(64);
            precompute.initialize();
            let base = precompute.get_base(0).unwrap();
            assert!(base >= 0.0 && base <= 1.0);
        }

        #[test]
        fn test_pedersen_get_base_out_of_range() {
            let mut precompute = PedersenPrecompute::new(10);
            precompute.initialize();
            assert_eq!(
                precompute.get_base(99).unwrap_err(),
                CircuitOptError::InvalidConstraintCount
            );
        }

        #[test]
        fn test_pedersen_recompute() {
            let mut precompute = PedersenPrecompute::new(32);
            precompute.initialize();
            let base1 = precompute.get_base(5).unwrap();
            precompute.recompute();
            let base2 = precompute.get_base(5).unwrap();
            assert_eq!(base1, base2);
        }

        #[test]
        fn test_pedersen_default() {
            let precompute = PedersenPrecompute::default();
            assert_eq!(precompute.count(), 256);
        }

        #[test]
        fn test_benchmark_creation() {
            let bench = CircuitBenchmark::new();
            assert!(!bench.is_running());
            assert!(bench.results().is_empty());
        }

        #[test]
        fn test_benchmark_start() {
            let mut bench = CircuitBenchmark::new();
            assert!(bench.start(10).is_ok());
            assert!(bench.is_running());
        }

        #[test]
        fn test_benchmark_already_started() {
            let mut bench = CircuitBenchmark::new();
            bench.start(10).unwrap();
            assert_eq!(
                bench.start(5).unwrap_err(),
                CircuitOptError::BenchmarkAlreadyStarted
            );
        }

        #[test]
        fn test_benchmark_record_weight() {
            let mut bench = CircuitBenchmark::new();
            bench.start(10).unwrap();
            assert!(bench.record_weight(1.5).is_ok());
        }

        #[test]
        fn test_benchmark_record_weight_not_started() {
            let mut bench = CircuitBenchmark::new();
            assert_eq!(
                bench.record_weight(1.0).unwrap_err(),
                CircuitOptError::BenchmarkNotStarted
            );
        }

        #[test]
        fn test_benchmark_stop() {
            let mut bench = CircuitBenchmark::new();
            bench.start(10).unwrap();
            bench.record_weight(2.0).unwrap();
            bench.record_weight(3.0).unwrap();
            let result = bench.stop(15, 100.0, 0.75).unwrap();
            assert_eq!(result.constraint_count, 15);
            assert_eq!(result.gen_time_ms, 100.0);
            assert!((result.avg_weight - 2.5).abs() < 0.01);
            assert_eq!(result.pool_utilization, 0.75);
        }

        #[test]
        fn test_benchmark_stop_not_started() {
            let mut bench = CircuitBenchmark::new();
            assert_eq!(
                bench.stop(10, 100.0, 0.5).unwrap_err(),
                CircuitOptError::BenchmarkNotStarted
            );
        }

        #[test]
        fn test_benchmark_latest() {
            let mut bench = CircuitBenchmark::new();
            assert!(bench.latest().is_none());
            bench.start(5).unwrap();
            bench.stop(10, 50.0, 0.5).unwrap();
            assert!(bench.latest().is_some());
        }

        #[test]
        fn test_benchmark_avg_gen_time() {
            let mut bench = CircuitBenchmark::new();
            bench.start(5).unwrap();
            bench.stop(10, 100.0, 0.5).unwrap();
            bench.start(5).unwrap();
            bench.stop(10, 200.0, 0.5).unwrap();
            assert!((bench.avg_gen_time_ms() - 150.0).abs() < 0.01);
        }

        #[test]
        fn test_benchmark_clear() {
            let mut bench = CircuitBenchmark::new();
            bench.start(5).unwrap();
            bench.stop(10, 50.0, 0.5).unwrap();
            bench.clear();
            assert!(bench.results().is_empty());
        }

        #[test]
        fn test_benchmark_default() {
            let bench = CircuitBenchmark::default();
            assert!(!bench.is_running());
        }

        #[test]
        fn test_error_display() {
            assert!(!format!("{}", CircuitOptError::PoolExhausted).is_empty());
            assert!(!format!("{}", CircuitOptError::NotPrecomputed).is_empty());
        }

        #[test]
        fn test_constraint_slot_new() {
            let slot = ConstraintSlot::new(0, 1.5);
            assert_eq!(slot.index, 0);
            assert_eq!(slot.weight, 1.5);
            assert!(!slot.allocated);
        }
    }
}

pub use internal::{
    BenchmarkResult, CircuitBenchmark, CircuitOptError, ConstraintPool, ConstraintSlot,
    PedersenPrecompute,
};
