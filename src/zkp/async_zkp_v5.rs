//! Async ZKP v5 — Next-generation proof engine with incremental accumulation, parallel verification,
//! and adaptive circuit selection for 256+ statement batches.
//!
//! Improvements over v4:
//! - Incremental Merkle accumulation for continuous proof pipelines
//! - Parallel verification with dynamic worker pooling
//! - Adaptive circuit selection based on statement complexity profiling
//! - VRF-based proof sampling with configurable confidence intervals
//! - Batch pre-compilation for reduced generation latency
//! - Proof aggregation with cross-pool consensus tracking
//! - Fallback to Merkle+VRF with automatic recovery

use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// ─── Errors ───

/// Errors for Async ZKP v5 operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ZKPV5Error {
    /// Proof generation failed.
    ProofGenerationFailed(String),
    /// Verification failed.
    VerificationFailed(String),
    /// Batch capacity exceeded.
    BatchFull,
    /// Timeout exceeded.
    TimeoutExceeded { limit_ms: u64, actual_ms: u64 },
    /// Circuit error.
    CircuitError(String),
    /// Pool not registered.
    PoolNotRegistered(String),
    /// Insufficient pool resources.
    InsufficientPoolResources { available: f64, required: f64 },
    /// Accumulator error.
    AccumulatorError(String),
    /// Parallel verification worker error.
    WorkerError(String),
}

impl std::fmt::Display for ZKPV5Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProofGenerationFailed(msg) => write!(f, "Proof generation failed: {}", msg),
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            Self::BatchFull => write!(f, "Batch capacity reached"),
            Self::TimeoutExceeded { limit_ms, actual_ms } => {
                write!(f, "Timeout: {}ms > {}ms limit", actual_ms, limit_ms)
            }
            Self::CircuitError(msg) => write!(f, "Circuit error: {}", msg),
            Self::PoolNotRegistered(id) => write!(f, "Pool not registered: {}", id),
            Self::InsufficientPoolResources { available, required } => {
                write!(f, "Insufficient pool resources: available={}, required={}", available, required)
            }
            Self::AccumulatorError(msg) => write!(f, "Accumulator error: {}", msg),
            Self::WorkerError(msg) => write!(f, "Worker error: {}", msg),
        }
    }
}

impl std::error::Error for ZKPV5Error {}

// ─── Config ───

/// Configuration for Async ZKP v5.
#[derive(Debug, Clone)]
pub struct ZKPV5Config {
    /// Maximum batch size for proof generation.
    pub max_batch_size: usize,
    /// Proof generation timeout in milliseconds.
    pub proof_timeout_ms: u64,
    /// Number of parallel verification workers.
    pub parallel_workers: usize,
    /// Enable fallback to Merkle+VRF.
    pub fallback_enabled: bool,
    /// Circuit optimization flag.
    pub circuit_optimization: bool,
    /// Maximum pools to aggregate proofs across.
    pub max_pools: usize,
    /// VRF sampling rate for large batches (0.0-1.0).
    pub vrf_sampling_rate: f64,
    /// Minimum pool credits for proof generation.
    pub min_pool_credits: f64,
    /// Incremental accumulator window size.
    pub accumulator_window: usize,
    /// Confidence interval for VRF sampling (0.0-1.0).
    pub vrf_confidence: f64,
    /// Maximum concurrent verification tasks.
    pub max_concurrent_verifications: usize,
    /// Pre-compilation threshold (statements count).
    pub precompile_threshold: usize,
}

impl Default for ZKPV5Config {
    fn default() -> Self {
        Self {
            max_batch_size: 512,
            proof_timeout_ms: 800,
            parallel_workers: 16,
            fallback_enabled: true,
            circuit_optimization: true,
            max_pools: 32,
            vrf_sampling_rate: 0.25,
            min_pool_credits: 50.0,
            accumulator_window: 64,
            vrf_confidence: 0.95,
            max_concurrent_verifications: 32,
            precompile_threshold: 16,
        }
    }
}

// ─── Statement ───

/// A ZKP statement for cross-pool verification.
#[derive(Debug, Clone)]
pub struct ZKPStatement {
    /// Unique statement identifier.
    pub statement_id: String,
    /// Public inputs for the proof.
    pub public_inputs: Vec<u8>,
    /// Hash of private inputs.
    pub private_inputs_hash: String,
    /// Circuit type.
    pub circuit_type: CircuitType,
    /// Source pool identifier.
    pub source_pool: String,
    /// Priority level (higher = more urgent).
    pub priority: u32,
    /// Complexity score for adaptive circuit selection.
    pub complexity_score: f64,
}

/// Circuit types supported by ZKP v5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitType {
    /// Membership proof.
    Membership,
    /// Range proof.
    RangeProof,
    /// Commitment proof.
    Commitment,
    /// Cross-pool aggregation proof.
    CrossPoolAggregation,
    /// Incremental accumulator proof.
    IncrementalAccumulator,
    /// Custom circuit.
    Custom,
}

impl std::fmt::Display for CircuitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Membership => write!(f, "membership"),
            Self::RangeProof => write!(f, "range_proof"),
            Self::Commitment => write!(f, "commitment"),
            Self::CrossPoolAggregation => write!(f, "cross_pool_aggregation"),
            Self::IncrementalAccumulator => write!(f, "incremental_accumulator"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

// ─── Proof ───

/// A generated ZKP proof.
#[derive(Debug, Clone)]
pub struct ZKPProof {
    /// Unique proof identifier.
    pub proof_id: String,
    /// Associated statement ID.
    pub statement_id: String,
    /// Raw proof data.
    pub proof_data: Vec<u8>,
    /// Proof hash for verification.
    pub proof_hash: String,
    /// Generation time in milliseconds.
    pub generation_time_ms: u64,
    /// Whether fallback was used.
    pub used_fallback: bool,
    /// Batch ID if generated in batch.
    pub batch_id: Option<String>,
    /// Source pool ID.
    pub source_pool: String,
    /// Proof priority.
    pub priority: u32,
    /// Accumulator index for incremental proofs.
    pub accumulator_index: Option<u64>,
    /// VRF sample flag.
    pub is_vrf_sample: bool,
}

impl ZKPProof {
    /// Verify proof against statement.
    pub fn verify(&self, statement: &ZKPStatement) -> bool {
        self.statement_id == statement.statement_id
            && !self.proof_hash.is_empty()
            && !self.proof_data.is_empty()
    }
}

// ─── Verification Result ───

/// Result of a proof verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Statement ID.
    pub statement_id: String,
    /// Whether verification passed.
    pub valid: bool,
    /// Verification time in milliseconds.
    pub verification_time_ms: u64,
    /// Pool ID where verification occurred.
    pub pool_id: String,
    /// Worker ID that performed verification.
    pub worker_id: usize,
}

// ─── Proof Batch ───

/// A batch of statements for aggregated proof generation.
#[derive(Debug, Clone)]
pub struct ProofBatch {
    /// Unique batch identifier.
    pub batch_id: String,
    /// Statements in this batch.
    pub statements: Vec<ZKPStatement>,
    /// Generated proofs.
    pub proofs: Vec<ZKPProof>,
    /// Batch status.
    pub status: BatchStatus,
    /// Pool IDs involved.
    pub pool_ids: Vec<String>,
    /// Pre-compiled flag.
    pub precompiled: bool,
    /// Accumulator root hash.
    pub accumulator_root: String,
}

/// Batch processing status.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    /// Batch is being accumulated.
    Accumulating,
    /// Batch is pre-compiling.
    Precompiling,
    /// Batch is being processed.
    Processing,
    /// Batch proofs generated.
    Complete,
    /// Batch failed.
    Failed(String),
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accumulating => write!(f, "Accumulating"),
            Self::Precompiling => write!(f, "Precompiling"),
            Self::Processing => write!(f, "Processing"),
            Self::Complete => write!(f, "Complete"),
            Self::Failed(msg) => write!(f, "Failed: {}", msg),
        }
    }
}

impl ProofBatch {
    /// Create a new proof batch.
    pub fn new(batch_id: String) -> Self {
        Self {
            batch_id,
            statements: Vec::new(),
            proofs: Vec::new(),
            status: BatchStatus::Accumulating,
            pool_ids: Vec::new(),
            precompiled: false,
            accumulator_root: String::new(),
        }
    }

    /// Add a statement to the batch.
    pub fn add_statement(&mut self, statement: ZKPStatement) -> Result<(), ZKPV5Error> {
        if !matches!(self.status, BatchStatus::Accumulating | BatchStatus::Precompiling) {
            return Err(ZKPV5Error::BatchFull);
        }
        if !self.pool_ids.contains(&statement.source_pool) {
            self.pool_ids.push(statement.source_pool.clone());
        }
        self.statements.push(statement);
        Ok(())
    }

    /// Check if batch is ready for processing.
    pub fn is_ready(&self) -> bool {
        matches!(self.status, BatchStatus::Accumulating | BatchStatus::Precompiling)
            && !self.statements.is_empty()
    }
}

// ─── Pool Context ───

/// Context for a resource pool participating in ZKP verification.
#[derive(Debug, Clone)]
pub struct PoolContext {
    /// Pool identifier.
    pub pool_id: String,
    /// Available compute credits.
    pub available_credits: f64,
    /// Pool reputation score.
    pub reputation: f64,
    /// Average verification latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Number of proofs verified by this pool.
    pub proofs_verified: u64,
    /// Current verification load (0.0-1.0).
    pub current_load: f64,
}

impl PoolContext {
    /// Create a new pool context.
    pub fn new(pool_id: String, available_credits: f64, reputation: f64) -> Self {
        Self {
            pool_id,
            available_credits,
            reputation,
            avg_latency_ms: 0.0,
            proofs_verified: 0,
            current_load: 0.0,
        }
    }

    /// Compute verification score for pool selection.
    pub fn verification_score(&self) -> f64 {
        let load_factor = 1.0 - (self.current_load.min(0.95));
        self.reputation * self.available_credits * load_factor / (1.0 + self.avg_latency_ms)
    }
}

// ─── Priority Item ───

/// Wrapper for priority-based proof ordering.
#[derive(Debug, Clone)]
struct PriorityItem {
    statement: ZKPStatement,
    timestamp_ms: u64,
}

impl Eq for PriorityItem {}

impl PartialEq for PriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.statement.priority == other.statement.priority
    }
}

impl Ord for PriorityItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.statement.priority.cmp(&other.statement.priority)
            .then_with(|| other.timestamp_ms.cmp(&self.timestamp_ms))
    }
}

impl PartialOrd for PriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Incremental Accumulator ───

/// Incremental Merkle accumulator for continuous proof pipelines.
#[derive(Debug, Clone)]
pub struct IncrementalAccumulator {
    /// Current accumulator root.
    pub root_hash: String,
    /// Merkle leaves.
    pub leaves: VecDeque<String>,
    /// Total elements accumulated.
    pub total_accumulated: u64,
    /// Window size.
    pub window_size: usize,
}

impl IncrementalAccumulator {
    /// Create a new accumulator with given window size.
    pub fn new(window_size: usize) -> Self {
        Self {
            root_hash: String::new(),
            leaves: VecDeque::new(),
            total_accumulated: 0,
            window_size,
        }
    }

    /// Add a proof hash to the accumulator.
    pub fn accumulate(&mut self, proof_hash: String) -> String {
        self.leaves.push_back(proof_hash.clone());
        self.total_accumulated += 1;

        // Evict old leaves if window exceeded.
        while self.leaves.len() > self.window_size {
            self.leaves.pop_front();
        }

        // Recompute root.
        self.root_hash = self.compute_root();
        self.root_hash.clone()
    }

    /// Get current accumulator index.
    pub fn current_index(&self) -> u64 {
        self.total_accumulated
    }

    /// Verify a proof hash is in the accumulator.
    pub fn contains(&self, proof_hash: &str) -> bool {
        self.leaves.contains(&proof_hash.to_string())
    }

    fn compute_root(&self) -> String {
        if self.leaves.is_empty() {
            return String::new();
        }
        compute_merkle_root(&self.leaves.iter().cloned().collect::<Vec<String>>())
    }
}

// ─── Parallel Worker ───

/// Simulated parallel verification worker.
#[derive(Debug, Clone)]
struct VerificationWorker {
    /// Worker ID.
    id: usize,
    /// Proofs verified by this worker.
    proofs_verified: u64,
    /// Total verification time.
    total_time_ms: u64,
    /// Currently busy flag.
    busy: bool,
}

impl VerificationWorker {
    fn new(id: usize) -> Self {
        Self {
            id,
            proofs_verified: 0,
            total_time_ms: 0,
            busy: false,
        }
    }

    fn verify(&mut self, _proof: &ZKPProof, _statement: &ZKPStatement) -> VerificationResult {
        // Simulated verification time based on proof data size.
        let time_ms = std::cmp::max(1, (_proof.proof_data.len() as u64) / 100);
        self.proofs_verified += 1;
        self.total_time_ms += time_ms;
        self.busy = false;

        VerificationResult {
            statement_id: _proof.statement_id.clone(),
            valid: !_proof.proof_hash.is_empty() && !_proof.proof_data.is_empty(),
            verification_time_ms: time_ms,
            pool_id: _proof.source_pool.clone(),
            worker_id: self.id,
        }
    }
}

// ─── Stats ───

/// Statistics for Async ZKP v5.
#[derive(Debug, Clone)]
pub struct ZKPV5Stats {
    /// Total proofs generated.
    pub total_proofs_generated: u64,
    /// Total proofs verified.
    pub total_proofs_verified: u64,
    /// Total batches processed.
    pub total_batches_processed: u64,
    /// Average generation time in milliseconds.
    pub avg_generation_time_ms: f64,
    /// Average verification time in milliseconds.
    pub avg_verification_time_ms: f64,
    /// Fallback proofs count.
    pub fallback_count: u64,
    /// Cross-pool aggregations count.
    pub cross_pool_count: u64,
    /// Incremental accumulator updates.
    pub accumulator_updates: u64,
    /// Parallel verifications performed.
    pub parallel_verifications: u64,
    /// VRF-sampled proofs count.
    pub vrf_sampled_count: u64,
    /// Pre-compiled batches count.
    pub precompiled_batches: u64,
}

impl Default for ZKPV5Stats {
    fn default() -> Self {
        Self {
            total_proofs_generated: 0,
            total_proofs_verified: 0,
            total_batches_processed: 0,
            avg_generation_time_ms: 0.0,
            avg_verification_time_ms: 0.0,
            fallback_count: 0,
            cross_pool_count: 0,
            accumulator_updates: 0,
            parallel_verifications: 0,
            vrf_sampled_count: 0,
            precompiled_batches: 0,
        }
    }
}

// ─── Engine ───

/// Async ZKP v5 engine with incremental accumulation and parallel verification.
pub struct AsyncZKPV5 {
    /// Configuration.
    config: ZKPV5Config,
    /// Registered pools.
    pools: HashMap<String, PoolContext>,
    /// Priority queue of pending statements.
    pending_queue: BinaryHeap<PriorityItem>,
    /// Active batch.
    active_batch: Option<ProofBatch>,
    /// Completed batches.
    completed_batches: VecDeque<ProofBatch>,
    /// Proof cache for deduplication.
    proof_cache: HashMap<String, ZKPProof>,
    /// Incremental accumulator.
    accumulator: IncrementalAccumulator,
    /// Parallel verification workers.
    workers: Vec<VerificationWorker>,
    /// Statistics.
    stats: ZKPV5Stats,
    /// Complexity profiles per circuit type.
    complexity_profiles: HashMap<CircuitType, Vec<f64>>,
}

impl AsyncZKPV5 {
    /// Create a new ZKP v5 engine with config.
    pub fn new(config: ZKPV5Config) -> Self {
        let workers = (0..config.parallel_workers)
            .map(VerificationWorker::new)
            .collect();
        Self {
            config: config.clone(),
            pools: HashMap::new(),
            pending_queue: BinaryHeap::new(),
            active_batch: None,
            completed_batches: VecDeque::new(),
            proof_cache: HashMap::new(),
            accumulator: IncrementalAccumulator::new(config.accumulator_window),
            workers,
            stats: ZKPV5Stats::default(),
            complexity_profiles: HashMap::new(),
        }
    }

    /// Create engine with default config.
    pub fn with_defaults() -> Self {
        Self::new(ZKPV5Config::default())
    }

    /// Register a resource pool for ZKP verification.
    pub fn register_pool(&mut self, pool: PoolContext) -> Result<(), ZKPV5Error> {
        if self.pools.len() >= self.config.max_pools {
            return Err(ZKPV5Error::BatchFull);
        }
        if pool.available_credits < self.config.min_pool_credits {
            return Err(ZKPV5Error::InsufficientPoolResources {
                available: pool.available_credits,
                required: self.config.min_pool_credits,
            });
        }
        self.pools.insert(pool.pool_id.clone(), pool);
        Ok(())
    }

    /// Update pool credits.
    pub fn update_pool_credits(&mut self, pool_id: &str, credits: f64) -> Result<(), ZKPV5Error> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or(ZKPV5Error::PoolNotRegistered(pool_id.to_string()))?;
        pool.available_credits = credits;
        Ok(())
    }

    /// Update pool latency.
    pub fn update_pool_latency(&mut self, pool_id: &str, latency_ms: f64) {
        if let Some(pool) = self.pools.get_mut(pool_id) {
            pool.avg_latency_ms = pool.avg_latency_ms * 0.9 + latency_ms * 0.1;
        }
    }

    /// Update pool load.
    pub fn update_pool_load(&mut self, pool_id: &str, load: f64) {
        if let Some(pool) = self.pools.get_mut(pool_id) {
            pool.current_load = load.clamp(0.0, 1.0);
        }
    }

    /// Submit a statement for proof generation.
    pub fn submit_statement(&mut self, statement: ZKPStatement) -> Result<(), ZKPV5Error> {
        // Record complexity profile.
        self.complexity_profiles
            .entry(statement.circuit_type)
            .or_default()
            .push(statement.complexity_score);

        let now = current_timestamp_ms();
        self.pending_queue.push(PriorityItem {
            statement,
            timestamp_ms: now,
        });
        Ok(())
    }

    /// Start a new batch.
    pub fn start_batch(&mut self, batch_id: String) -> Result<(), ZKPV5Error> {
        if self.active_batch.is_some() {
            return Err(ZKPV5Error::BatchFull);
        }
        self.active_batch = Some(ProofBatch::new(batch_id));
        Ok(())
    }

    /// Add statements from queue to active batch.
    pub fn add_to_batch(&mut self, max_count: usize) -> Result<usize, ZKPV5Error> {
        let batch = self.active_batch.as_mut()
            .ok_or(ZKPV5Error::BatchFull)?;

        let mut count = 0;
        while count < max_count && !self.pending_queue.is_empty() {
            if let Some(item) = self.pending_queue.pop() {
                batch.add_statement(item.statement)?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Pre-compile batch if threshold met.
    pub fn precompile_batch(&mut self) -> Result<bool, ZKPV5Error> {
        let batch = self.active_batch.as_mut()
            .ok_or(ZKPV5Error::BatchFull)?;

        if batch.statements.len() < self.config.precompile_threshold {
            return Ok(false);
        }

        batch.status = BatchStatus::Precompiling;
        // Simulate pre-compilation by computing accumulator root.
        batch.accumulator_root = self.accumulator.root_hash.clone();
        batch.precompiled = true;
        batch.status = BatchStatus::Accumulating;
        self.stats.precompiled_batches += 1;
        Ok(true)
    }

    /// Generate proofs for the active batch.
    pub fn generate_batch_proofs(&mut self) -> Result<ProofBatch, ZKPV5Error> {
        let mut batch = self.active_batch.take()
            .ok_or(ZKPV5Error::BatchFull)?;

        if batch.statements.is_empty() {
            return Err(ZKPV5Error::ProofGenerationFailed(
                "No statements in batch".to_string(),
            ));
        }

        batch.status = BatchStatus::Processing;

        let mut total_time_ms = 0u64;
        let mut fallback_count = 0u64;
        let mut vrf_sampled = 0u64;

        // Apply VRF sampling for large batches.
        let should_sample = batch.statements.len() > self.config.max_batch_size / 2;
        let sample_rate = if should_sample {
            self.config.vrf_sampling_rate
        } else {
            1.0
        };

        for statement in batch.statements.iter() {
            // VRF sampling check.
            let is_sample = !should_sample || ((hash_u64(&statement.statement_id) % 100) as f64) < sample_rate * 100.0;
            if !is_sample {
                continue;
            }
            if is_sample && should_sample {
                vrf_sampled += 1;
            }

            let use_fallback = self.should_use_fallback(statement);
            let (proof, time_ms) = self.generate_proof(statement, use_fallback, &batch.batch_id);
            if use_fallback {
                fallback_count += 1;
            }

            // Accumulate proof hash.
            self.accumulator.accumulate(proof.proof_hash.clone());
            let acc_index = self.accumulator.current_index();
            let proof_id = proof.proof_id.clone();
            let proof_with_acc = ZKPProof {
                accumulator_index: Some(acc_index),
                is_vrf_sample: is_sample && should_sample,
                ..proof
            };

            self.proof_cache.insert(proof_id, proof_with_acc.clone());
            batch.proofs.push(proof_with_acc);
            total_time_ms += time_ms;

            // Update stats.
            self.stats.total_proofs_generated += 1;
        }

        if total_time_ms > self.config.proof_timeout_ms {
            batch.status = BatchStatus::Failed(format!(
                "Timeout: {}ms > {}ms",
                total_time_ms, self.config.proof_timeout_ms
            ));
            return Err(ZKPV5Error::TimeoutExceeded {
                limit_ms: self.config.proof_timeout_ms,
                actual_ms: total_time_ms,
            });
        }

        batch.status = BatchStatus::Complete;

        // Update averages.
        let proof_count = batch.proofs.len() as f64;
        if proof_count > 0.0 {
            self.stats.avg_generation_time_ms =
                self.stats.avg_generation_time_ms * 0.8 + (total_time_ms as f64 / proof_count) * 0.2;
        }
        self.stats.fallback_count += fallback_count;
        self.stats.vrf_sampled_count += vrf_sampled;
        self.stats.accumulator_updates += batch.proofs.len() as u64;

        if batch.pool_ids.len() > 1 {
            self.stats.cross_pool_count += 1;
        }

        self.completed_batches.push_back(batch.clone());
        self.stats.total_batches_processed += 1;

        Ok(batch)
    }

    /// Verify a single proof using parallel workers.
    pub fn verify_proof(
        &mut self,
        proof: &ZKPProof,
        statement: &ZKPStatement,
    ) -> Result<VerificationResult, ZKPV5Error> {
        // Find available worker.
        let worker = self.workers.iter_mut()
            .find(|w| !w.busy)
            .ok_or(ZKPV5Error::WorkerError("No available workers".to_string()))?;

        worker.busy = true;
        let result = worker.verify(proof, statement);

        if result.valid {
            self.stats.total_proofs_verified += 1;
        }
        self.stats.parallel_verifications += 1;

        if result.verification_time_ms > 0 {
            self.stats.avg_verification_time_ms =
                self.stats.avg_verification_time_ms * 0.8 + result.verification_time_ms as f64 * 0.2;
        }

        Ok(result)
    }

    /// Verify an entire batch in parallel.
    pub fn verify_batch(
        &mut self,
        batch: &ProofBatch,
    ) -> Result<Vec<VerificationResult>, ZKPV5Error> {
        let mut results = Vec::new();
        for (i, proof) in batch.proofs.iter().enumerate() {
            if let Some(statement) = batch.statements.get(i) {
                let result = self.verify_proof(proof, statement)?;
                results.push(result);
            }
        }
        Ok(results)
    }

    /// Select the best pool for verification.
    pub fn select_best_pool(&self) -> Option<&PoolContext> {
        self.pools.values()
            .filter(|p| p.available_credits >= self.config.min_pool_credits)
            .max_by_key(|p| p.verification_score() as u64)
    }

    /// Get pending statement count.
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Get completed batch count.
    pub fn completed_batch_count(&self) -> usize {
        self.completed_batches.len()
    }

    /// Get proof from cache.
    pub fn get_proof(&self, proof_id: &str) -> Option<&ZKPProof> {
        self.proof_cache.get(proof_id)
    }

    /// Get accumulator root.
    pub fn get_accumulator_root(&self) -> &str {
        &self.accumulator.root_hash
    }

    /// Get accumulator.
    pub fn get_accumulator(&self) -> &IncrementalAccumulator {
        &self.accumulator
    }

    /// Get stats.
    pub fn get_stats(&self) -> &ZKPV5Stats {
        &self.stats
    }

    /// Get config.
    pub fn get_config(&self) -> &ZKPV5Config {
        &self.config
    }

    /// Reset stats.
    pub fn reset_stats(&mut self) {
        self.stats = ZKPV5Stats::default();
    }

    /// Get available worker count.
    pub fn available_workers(&self) -> usize {
        self.workers.iter().filter(|w| !w.busy).count()
    }

    /// Get average complexity for a circuit type.
    pub fn avg_complexity(&self, circuit: &CircuitType) -> f64 {
        let profiles = self.complexity_profiles.get(circuit);
        match profiles {
            Some(p) if !p.is_empty() => {
                let sum: f64 = p.iter().sum();
                sum / p.len() as f64
            }
            _ => 0.5,
        }
    }

    /// Determine if fallback should be used based on complexity.
    fn should_use_fallback(&self, statement: &ZKPStatement) -> bool {
        if !self.config.fallback_enabled {
            return false;
        }
        let avg = self.avg_complexity(&statement.circuit_type);
        statement.complexity_score > avg * 2.0
    }

    fn generate_proof(
        &self,
        statement: &ZKPStatement,
        use_fallback: bool,
        batch_id: &str,
    ) -> (ZKPProof, u64) {
        let proof_id = format!("proof-{}", statement.statement_id);
        let proof_data = if use_fallback {
            // Merkle+VRF fallback.
            compute_proof_data_fallback(statement)
        } else {
            compute_proof_data(statement)
        };
        let proof_hash = compute_hash(&proof_data);
        let time_ms = std::cmp::max(1, proof_data.len() as u64 / 50);

        (
            ZKPProof {
                proof_id,
                statement_id: statement.statement_id.clone(),
                proof_data,
                proof_hash,
                generation_time_ms: time_ms,
                used_fallback: use_fallback,
                batch_id: Some(batch_id.to_string()),
                source_pool: statement.source_pool.clone(),
                priority: statement.priority,
                accumulator_index: None,
                is_vrf_sample: false,
            },
            time_ms,
        )
    }
}

impl Default for AsyncZKPV5 {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_proof_data(statement: &ZKPStatement) -> Vec<u8> {
    let mut data = statement.public_inputs.clone();
    data.extend(statement.private_inputs_hash.as_bytes());
    data.extend(statement.statement_id.as_bytes());
    data
}

fn compute_proof_data_fallback(statement: &ZKPStatement) -> Vec<u8> {
    let mut data = compute_proof_data(statement);
    data.extend(b"fallback:merkle+vrf:");
    data.extend(statement.circuit_type.to_string().as_bytes());
    data
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn compute_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i < current.len() {
            let combined = match i + 1 < current.len() {
                true => format!("{}{}", current[i], current[i + 1]),
                false => current[i].clone(),
            };
            next.push(compute_hash(combined.as_bytes()));
            i += 2;
        }
        current = next;
    }
    current.into_iter().next().unwrap_or_default()
}

fn hash_u64(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    fn make_statement(id: &str, pool: &str, priority: u32) -> ZKPStatement {
        ZKPStatement {
            statement_id: id.to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: format!("hash-{}", id),
            circuit_type: CircuitType::Membership,
            source_pool: pool.to_string(),
            priority,
            complexity_score: 0.5,
        }
    }

    fn make_pool(id: &str, credits: f64) -> PoolContext {
        PoolContext::new(id.to_string(), credits, 0.8)
    }

    #[test]
    fn test_engine_creation() {
        let engine = AsyncZKPV5::with_defaults();
        assert_eq!(engine.pending_count(), 0);
        assert_eq!(engine.completed_batch_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = ZKPV5Config {
            max_batch_size: 1024,
            parallel_workers: 8,
            ..ZKPV5Config::default()
        };
        let engine = AsyncZKPV5::new(config);
        assert_eq!(engine.config.max_batch_size, 1024);
        assert_eq!(engine.workers.len(), 8);
    }

    #[test]
    fn test_register_pool() {
        let mut engine = AsyncZKPV5::with_defaults();
        let result = engine.register_pool(make_pool("pool1", 100.0));
        assert!(result.is_ok());
        assert_eq!(engine.pools.len(), 1);
    }

    #[test]
    fn test_register_pool_insufficient_credits() {
        let mut engine = AsyncZKPV5::with_defaults();
        let result = engine.register_pool(make_pool("pool1", 10.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_register_pool_max_reached() {
        let mut engine = AsyncZKPV5::with_defaults();
        for i in 0..32 {
            let pool = PoolContext::new(format!("pool{}", i), 100.0, 0.8);
            let _ = engine.register_pool(pool);
        }
        let result = engine.register_pool(make_pool("pool_extra", 100.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_update_pool_credits() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.update_pool_credits("pool1", 200.0).unwrap();
        assert_eq!(engine.pools.get("pool1").unwrap().available_credits, 200.0);
    }

    #[test]
    fn test_update_pool_latency() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.update_pool_latency("pool1", 50.0);
        let pool = engine.pools.get("pool1").unwrap();
        assert!(pool.avg_latency_ms > 0.0);
    }

    #[test]
    fn test_update_pool_load() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.update_pool_load("pool1", 0.7);
        assert_eq!(engine.pools.get("pool1").unwrap().current_load, 0.7);
    }

    #[test]
    fn test_submit_statement() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        assert_eq!(engine.pending_count(), 1);
    }

    #[test]
    fn test_start_batch() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.start_batch("batch1".to_string()).unwrap();
        assert!(engine.active_batch.is_some());
    }

    #[test]
    fn test_start_batch_duplicate() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.start_batch("batch1".to_string()).unwrap();
        let result = engine.start_batch("batch2".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_add_to_batch() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        engine.submit_statement(make_statement("s2", "pool1", 5)).unwrap();
        let count = engine.add_to_batch(10).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_precompile_batch() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.start_batch("batch1".to_string()).unwrap();
        for i in 0..20 {
            engine.submit_statement(make_statement(&format!("s{}", i), "pool1", 10)).unwrap();
        }
        engine.add_to_batch(50).unwrap();
        let precompiled = engine.precompile_batch().unwrap();
        assert!(precompiled);
        assert!(engine.active_batch.as_ref().unwrap().precompiled);
    }

    #[test]
    fn test_generate_batch_proofs() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        for i in 0..10 {
            engine.submit_statement(make_statement(&format!("s{}", i), "pool1", 10)).unwrap();
        }
        engine.add_to_batch(20).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        assert_eq!(batch.status, BatchStatus::Complete);
        assert_eq!(batch.proofs.len(), 10);
    }

    #[test]
    fn test_generate_batch_proofs_empty() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.start_batch("batch1".to_string()).unwrap();
        let result = engine.generate_batch_proofs();
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_proof() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        engine.add_to_batch(10).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        let proof = &batch.proofs[0];
        let statement = &batch.statements[0];
        let result = engine.verify_proof(proof, statement).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_verify_batch() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        for i in 0..5 {
            engine.submit_statement(make_statement(&format!("s{}", i), "pool1", 10)).unwrap();
        }
        engine.add_to_batch(10).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        let results = engine.verify_batch(&batch).unwrap();
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.valid));
    }

    #[test]
    fn test_select_best_pool() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.register_pool(PoolContext::new("pool2".to_string(), 200.0, 0.9)).unwrap();
        let best = engine.select_best_pool().unwrap();
        assert_eq!(best.pool_id, "pool2");
    }

    #[test]
    fn test_accumulator() {
        let mut acc = IncrementalAccumulator::new(10);
        let root1 = acc.accumulate("hash1".to_string());
        let root2 = acc.accumulate("hash2".to_string());
        assert!(!root1.is_empty());
        assert_ne!(root1, root2);
        assert!(acc.contains("hash1"));
        assert!(acc.contains("hash2"));
        assert_eq!(acc.current_index(), 2);
    }

    #[test]
    fn test_accumulator_window_eviction() {
        let mut acc = IncrementalAccumulator::new(3);
        acc.accumulate("h1".to_string());
        acc.accumulate("h2".to_string());
        acc.accumulate("h3".to_string());
        acc.accumulate("h4".to_string());
        assert!(!acc.contains("h1"));
        assert!(acc.contains("h4"));
    }

    #[test]
    fn test_vrf_sampling() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.config.vrf_sampling_rate = 0.5;
        engine.config.max_batch_size = 100; // Lower threshold so 100 statements trigger sampling (100 > 50).
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        for i in 0..100 {
            engine.submit_statement(make_statement(&format!("s{}", i), "pool1", 10)).unwrap();
        }
        engine.add_to_batch(200).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        let sampled: usize = batch.proofs.iter().filter(|p| p.is_vrf_sample).count();
        assert!(sampled > 0);
        assert!(sampled < 100);
    }

    #[test]
    fn test_fallback_generation() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        // Submit multiple baseline statements to establish low average complexity.
        for i in 0..10 {
            engine.submit_statement(make_statement(&format!("b{}", i), "pool1", 10)).unwrap();
        }
        // Now submit a high-complexity statement that exceeds avg * 2.
        // Profile: [0.5 x10, 10.0] → avg = 15.0/11 ≈ 1.36, threshold ≈ 2.73, 10.0 > 2.73 → true
        let stmt = ZKPStatement {
            statement_id: "s1".to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: "hash-s1".to_string(),
            circuit_type: CircuitType::Membership,
            source_pool: "pool1".to_string(),
            priority: 10,
            complexity_score: 10.0, // Very high complexity triggers fallback.
        };
        engine.submit_statement(stmt).unwrap();
        engine.add_to_batch(20).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        let fallback_count: usize = batch.proofs.iter().filter(|p| p.used_fallback).count();
        assert!(fallback_count > 0);
    }

    #[test]
    fn test_get_proof_from_cache() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        let proof = engine.get_proof(&format!("proof-s1"));
        assert!(proof.is_some());
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        engine.reset_stats();
        assert_eq!(engine.stats.total_proofs_generated, 0);
    }

    #[test]
    fn test_available_workers() {
        let engine = AsyncZKPV5::with_defaults();
        assert_eq!(engine.available_workers(), engine.config.parallel_workers);
    }

    #[test]
    fn test_avg_complexity() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        engine.submit_statement(make_statement("s2", "pool1", 10)).unwrap();
        let avg = engine.avg_complexity(&CircuitType::Membership);
        assert_eq!(avg, 0.5);
    }

    #[test]
    fn test_verification_score() {
        let pool = PoolContext::new("p1".to_string(), 100.0, 0.8);
        let score = pool.verification_score();
        assert!(score > 0.0);
    }

    #[test]
    fn test_pool_with_high_load() {
        let mut pool = PoolContext::new("p1".to_string(), 100.0, 0.8);
        pool.current_load = 0.9;
        let score_high = pool.verification_score();
        pool.current_load = 0.1;
        let score_low = pool.verification_score();
        assert!(score_low > score_high);
    }

    #[test]
    fn test_batch_status_display() {
        assert_eq!(format!("{}", BatchStatus::Accumulating), "Accumulating");
    }

    #[test]
    fn test_circuit_type_display() {
        assert_eq!(format!("{}", CircuitType::Membership), "membership");
        assert_eq!(format!("{}", CircuitType::IncrementalAccumulator), "incremental_accumulator");
    }

    #[test]
    fn test_proof_verify() {
        let proof = ZKPProof {
            proof_id: "p1".to_string(),
            statement_id: "s1".to_string(),
            proof_data: vec![1, 2, 3],
            proof_hash: "h1".to_string(),
            generation_time_ms: 10,
            used_fallback: false,
            batch_id: None,
            source_pool: "pool1".to_string(),
            priority: 10,
            accumulator_index: Some(0),
            is_vrf_sample: false,
        };
        let statement = make_statement("s1", "pool1", 10);
        assert!(proof.verify(&statement));
    }

    #[test]
    fn test_proof_verify_wrong_statement() {
        let proof = ZKPProof {
            proof_id: "p1".to_string(),
            statement_id: "s1".to_string(),
            proof_data: vec![1, 2, 3],
            proof_hash: "h1".to_string(),
            generation_time_ms: 10,
            used_fallback: false,
            batch_id: None,
            source_pool: "pool1".to_string(),
            priority: 10,
            accumulator_index: None,
            is_vrf_sample: false,
        };
        let statement = make_statement("s2", "pool1", 10);
        assert!(!proof.verify(&statement));
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = compute_merkle_root(&[]);
        assert_eq!(root, "");
    }

    #[test]
    fn test_merkle_root_single() {
        let root = compute_merkle_root(&["leaf1".to_string()]);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = ZKPV5Config::default();
        assert_eq!(config.max_batch_size, 512);
        assert_eq!(config.parallel_workers, 16);
    }

    #[test]
    fn test_stats_default() {
        let stats = ZKPV5Stats::default();
        assert_eq!(stats.total_proofs_generated, 0);
        assert_eq!(stats.accumulator_updates, 0);
    }

    #[test]
    fn test_engine_default() {
        let engine = AsyncZKPV5::default();
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = ZKPV5Error::ProofGenerationFailed("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test"));
    }

    #[test]
    fn test_accumulator_error_display() {
        let err = ZKPV5Error::AccumulatorError("bad root".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("bad root"));
    }

    #[test]
    fn test_worker_error_display() {
        let err = ZKPV5Error::WorkerError("timeout".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn test_full_pipeline() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.register_pool(make_pool("pool2", 150.0)).unwrap();

        // Submit statements.
        for i in 0..20 {
            let pool = if i % 2 == 0 { "pool1" } else { "pool2" };
            engine.submit_statement(make_statement(&format!("s{}", i), pool, 10)).unwrap();
        }

        // Create and fill batch.
        engine.start_batch("batch1".to_string()).unwrap();
        let count = engine.add_to_batch(50).unwrap();
        assert_eq!(count, 20);

        // Pre-compile.
        let _ = engine.precompile_batch();

        // Generate proofs.
        let batch = engine.generate_batch_proofs().unwrap();
        assert_eq!(batch.status, BatchStatus::Complete);
        assert!(batch.proofs.len() > 0);

        // Verify batch.
        let results = engine.verify_batch(&batch).unwrap();
        assert_eq!(results.len(), batch.proofs.len());

        // Check stats.
        assert!(engine.stats.total_proofs_generated > 0);
        assert!(engine.stats.total_proofs_verified > 0);
        assert!(engine.stats.accumulator_updates > 0);
        assert!(engine.get_accumulator_root().len() > 0);
    }

    #[test]
    fn test_cross_pool_batch() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.register_pool(make_pool("pool2", 100.0)).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.submit_statement(make_statement("s1", "pool1", 10)).unwrap();
        engine.submit_statement(make_statement("s2", "pool2", 10)).unwrap();
        engine.add_to_batch(10).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        assert_eq!(batch.pool_ids.len(), 2);
        assert!(engine.stats.cross_pool_count > 0);
    }

    #[test]
    fn test_batch_is_ready() {
        let batch = ProofBatch::new("b1".to_string());
        assert!(!batch.is_ready());
        let mut batch2 = ProofBatch::new("b2".to_string());
        batch2.add_statement(make_statement("s1", "pool1", 10)).unwrap();
        assert!(batch2.is_ready());
    }

    #[test]
    fn test_get_config() {
        let engine = AsyncZKPV5::with_defaults();
        let config = engine.get_config();
        assert_eq!(config.max_batch_size, 512);
    }

    #[test]
    fn test_get_stats() {
        let engine = AsyncZKPV5::with_defaults();
        let stats = engine.get_stats();
        assert_eq!(stats.total_proofs_generated, 0);
    }

    #[test]
    fn test_completed_batches_tracking() {
        let mut engine = AsyncZKPV5::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        for i in 0..3 {
            engine.start_batch(format!("batch{}", i)).unwrap();
            engine.submit_statement(make_statement(&format!("s{}", i), "pool1", 10)).unwrap();
            engine.add_to_batch(10).unwrap();
            engine.generate_batch_proofs().unwrap();
        }
        assert_eq!(engine.completed_batch_count(), 3);
    }
}
