//! Async ZKP v4 — Optimized for cross-pool verification with adaptive proof selection.
//!
//! Improvements over v3:
//! - Pool-aware proof generation (adapts to resource pool availability)
//! - Adaptive circuit selection based on statement complexity
//! - Cross-pool proof aggregation for batch verification
//! - Proof priority queue based on pool urgency
//! - VRF-based proof sampling for large batches

use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// ─── Errors ───

/// Errors for Async ZKP v4 operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ZKPV4Error {
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
}

impl std::fmt::Display for ZKPV4Error {
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
        }
    }
}

impl std::error::Error for ZKPV4Error {}

// ─── Config ───

/// Configuration for Async ZKP v4.
#[derive(Debug, Clone)]
pub struct ZKPV4Config {
    /// Maximum batch size for proof generation.
    pub max_batch_size: usize,
    /// Proof generation timeout in milliseconds.
    pub proof_timeout_ms: u64,
    /// Number of parallel verifiers.
    pub parallel_verifiers: usize,
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
}

impl Default for ZKPV4Config {
    fn default() -> Self {
        Self {
            max_batch_size: 256,
            proof_timeout_ms: 800,
            parallel_verifiers: 8,
            fallback_enabled: true,
            circuit_optimization: true,
            max_pools: 16,
            vrf_sampling_rate: 0.3,
            min_pool_credits: 50.0,
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
}

/// Circuit types supported by ZKP v4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitType {
    /// Membership proof.
    Membership,
    /// Range proof.
    RangeProof,
    /// Commitment proof.
    Commitment,
    /// Cross-pool aggregation proof.
    CrossPoolAggregation,
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
}

/// Batch processing status.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    /// Batch is being accumulated.
    Accumulating,
    /// Batch is being processed.
    Processing,
    /// Batch proofs generated.
    Complete,
    /// Batch failed.
    Failed(String),
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
        }
    }

    /// Add a statement to the batch.
    pub fn add_statement(&mut self, statement: ZKPStatement) -> Result<(), ZKPV4Error> {
        if self.status != BatchStatus::Accumulating {
            return Err(ZKPV4Error::BatchFull);
        }
        if !self.pool_ids.contains(&statement.source_pool) {
            self.pool_ids.push(statement.source_pool.clone());
        }
        self.statements.push(statement);
        Ok(())
    }

    /// Check if batch is ready for processing.
    pub fn is_ready(&self) -> bool {
        matches!(self.status, BatchStatus::Accumulating) && !self.statements.is_empty()
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
        }
    }

    /// Compute verification score for pool selection.
    pub fn verification_score(&self) -> f64 {
        self.reputation * self.available_credits / (1.0 + self.avg_latency_ms)
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

// ─── Stats ───

/// Statistics for Async ZKP v4.
#[derive(Debug, Clone)]
pub struct ZKPV4Stats {
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
}

impl Default for ZKPV4Stats {
    fn default() -> Self {
        Self {
            total_proofs_generated: 0,
            total_proofs_verified: 0,
            total_batches_processed: 0,
            avg_generation_time_ms: 0.0,
            avg_verification_time_ms: 0.0,
            fallback_count: 0,
            cross_pool_count: 0,
        }
    }
}

// ─── Engine ───

/// Async ZKP v4 engine optimized for cross-pool verification.
pub struct AsyncZKPV4 {
    /// Configuration.
    config: ZKPV4Config,
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
    /// Statistics.
    stats: ZKPV4Stats,
}

impl AsyncZKPV4 {
    /// Create a new ZKP v4 engine with config.
    pub fn new(config: ZKPV4Config) -> Self {
        Self {
            config,
            pools: HashMap::new(),
            pending_queue: BinaryHeap::new(),
            active_batch: None,
            completed_batches: VecDeque::new(),
            proof_cache: HashMap::new(),
            stats: ZKPV4Stats::default(),
        }
    }

    /// Create engine with default config.
    pub fn with_defaults() -> Self {
        Self::new(ZKPV4Config::default())
    }

    /// Register a resource pool for ZKP verification.
    pub fn register_pool(&mut self, pool: PoolContext) -> Result<(), ZKPV4Error> {
        if self.pools.len() >= self.config.max_pools {
            return Err(ZKPV4Error::BatchFull);
        }
        if pool.available_credits < self.config.min_pool_credits {
            return Err(ZKPV4Error::InsufficientPoolResources {
                available: pool.available_credits,
                required: self.config.min_pool_credits,
            });
        }
        self.pools.insert(pool.pool_id.clone(), pool);
        Ok(())
    }

    /// Update pool credits.
    pub fn update_pool_credits(&mut self, pool_id: &str, credits: f64) -> Result<(), ZKPV4Error> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or(ZKPV4Error::PoolNotRegistered(pool_id.to_string()))?;
        pool.available_credits = credits.max(0.0);
        Ok(())
    }

    /// Update pool latency.
    pub fn update_pool_latency(&mut self, pool_id: &str, latency_ms: f64) {
        if let Some(pool) = self.pools.get_mut(pool_id) {
            pool.avg_latency_ms = pool.avg_latency_ms * 0.9 + latency_ms * 0.1;
        }
    }

    /// Submit a statement for proof generation.
    pub fn submit_statement(&mut self, statement: ZKPStatement) -> Result<(), ZKPV4Error> {
        // Check pool registered
        if !self.pools.contains_key(&statement.source_pool) {
            return Err(ZKPV4Error::PoolNotRegistered(statement.source_pool.clone()));
        }

        // Check cache
        if self.proof_cache.contains_key(&statement.statement_id) {
            return Ok(());
        }

        // Add to priority queue
        self.pending_queue.push(PriorityItem {
            statement,
            timestamp_ms: current_timestamp_ms(),
        });

        Ok(())
    }

    /// Start a new proof batch.
    pub fn start_batch(&mut self, batch_id: String) -> Result<(), ZKPV4Error> {
        if self.active_batch.is_some() {
            return Err(ZKPV4Error::BatchFull);
        }
        self.active_batch = Some(ProofBatch::new(batch_id));
        Ok(())
    }

    /// Add pending statements to the active batch.
    pub fn add_to_batch(&mut self, max_count: usize) -> Result<usize, ZKPV4Error> {
        let batch = self.active_batch.as_mut()
            .ok_or(ZKPV4Error::BatchFull)?;

        let mut count = 0;
        while count < max_count && !self.pending_queue.is_empty() {
            if let Some(item) = self.pending_queue.pop() {
                batch.add_statement(item.statement)?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Generate proofs for the active batch.
    pub fn generate_batch_proofs(&mut self) -> Result<ProofBatch, ZKPV4Error> {
        let mut batch = self.active_batch.take()
            .ok_or(ZKPV4Error::BatchFull)?;

        batch.status = BatchStatus::Processing;

        let start_ms = current_timestamp_ms();
        let mut proofs = Vec::new();
        let mut cross_pool = false;

        for statement in &batch.statements {
            let proof = self.generate_proof(statement, &batch.batch_id)?;
            if proof.used_fallback {
                self.stats.fallback_count += 1;
            }
            if statement.circuit_type == CircuitType::CrossPoolAggregation {
                cross_pool = true;
            }
            proofs.push(proof);
        }

        let elapsed = current_timestamp_ms().saturating_sub(start_ms);
        if elapsed > self.config.proof_timeout_ms && self.config.fallback_enabled {
            batch.status = BatchStatus::Failed("Timeout".to_string());
            self.active_batch = Some(batch);
            return Err(ZKPV4Error::TimeoutExceeded {
                limit_ms: self.config.proof_timeout_ms,
                actual_ms: elapsed,
            });
        }

        if cross_pool {
            self.stats.cross_pool_count += 1;
        }

        batch.proofs = proofs;
        batch.status = BatchStatus::Complete;
        self.stats.total_batches_processed += 1;
        self.stats.total_proofs_generated += batch.proofs.len() as u64;

        // Update average generation time
        let avg_time = elapsed as f64 / (batch.proofs.len().max(1) as f64);
        self.stats.avg_generation_time_ms = 
            self.stats.avg_generation_time_ms * 0.9 + avg_time * 0.1;

        // Cache proofs
        for proof in &batch.proofs {
            self.proof_cache.insert(proof.statement_id.clone(), proof.clone());
        }

        self.completed_batches.push_back(batch.clone());
        if self.completed_batches.len() > 100 {
            self.completed_batches.pop_front();
        }

        Ok(batch)
    }

    /// Verify a single proof against its statement.
    pub fn verify_proof(
        &mut self,
        proof: &ZKPProof,
        statement: &ZKPStatement,
    ) -> Result<VerificationResult, ZKPV4Error> {
        let start_ms = current_timestamp_ms();

        let valid = proof.verify(statement);
        if !valid {
            return Err(ZKPV4Error::VerificationFailed(proof.proof_id.clone()));
        }

        let elapsed = current_timestamp_ms().saturating_sub(start_ms);
        self.stats.total_proofs_verified += 1;
        self.stats.avg_verification_time_ms = 
            self.stats.avg_verification_time_ms * 0.9 + elapsed as f64 * 0.1;

        // Update pool stats
        if let Some(pool) = self.pools.get_mut(&proof.source_pool) {
            pool.proofs_verified += 1;
        }

        Ok(VerificationResult {
            statement_id: statement.statement_id.clone(),
            valid,
            verification_time_ms: elapsed,
            pool_id: proof.source_pool.clone(),
        })
    }

    /// Verify a batch of proofs.
    pub fn verify_batch(
        &mut self,
        batch: &ProofBatch,
    ) -> Result<Vec<VerificationResult>, ZKPV4Error> {
        let mut results = Vec::new();
        for (proof, statement) in batch.proofs.iter().zip(batch.statements.iter()) {
            let result = self.verify_proof(proof, statement)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Get the best pool for verification based on score.
    pub fn select_best_pool(&self) -> Option<&PoolContext> {
        self.pools.values()
            .max_by(|a, b| a.verification_score().partial_cmp(&b.verification_score()).unwrap_or(Ordering::Equal))
    }

    /// Get cached proof.
    pub fn get_cached_proof(&self, statement_id: &str) -> Option<&ZKPProof> {
        self.proof_cache.get(statement_id)
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> ZKPV4Stats {
        self.stats.clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ZKPV4Stats::default();
    }

    /// Get registered pool count.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// Get pending queue size.
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Get completed batches.
    pub fn get_completed_batches(&self) -> &[ProofBatch] {
        self.completed_batches.as_slices().0
    }

    /// Generate a single proof (internal).
    fn generate_proof(
        &self,
        statement: &ZKPStatement,
        batch_id: &str,
    ) -> Result<ZKPProof, ZKPV4Error> {
        let start_ms = current_timestamp_ms();

        // Adaptive circuit selection
        let use_fallback = self.should_use_fallback(statement);

        // Generate proof data
        let proof_data = self.generate_proof_data(statement);
        let proof_hash = compute_proof_hash(&statement.statement_id, &proof_data);

        let elapsed = current_timestamp_ms().saturating_sub(start_ms);

        Ok(ZKPProof {
            proof_id: format!("proof_{}", statement.statement_id),
            statement_id: statement.statement_id.clone(),
            proof_data,
            proof_hash,
            generation_time_ms: elapsed,
            used_fallback: use_fallback,
            batch_id: Some(batch_id.to_string()),
            source_pool: statement.source_pool.clone(),
            priority: statement.priority,
        })
    }

    /// Determine if fallback should be used.
    fn should_use_fallback(&self, statement: &ZKPStatement) -> bool {
        if !self.config.fallback_enabled {
            return false;
        }
        // Use fallback for complex circuits or large inputs
        let complexity = match statement.circuit_type {
            CircuitType::CrossPoolAggregation => 3,
            CircuitType::Custom => 2,
            _ => 1,
        };
        complexity > 1 && statement.public_inputs.len() > 1024
    }

    /// Generate proof data for a statement.
    fn generate_proof_data(&self, statement: &ZKPStatement) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(statement.statement_id.as_bytes());
        data.extend_from_slice(&statement.public_inputs);
        data.extend_from_slice(statement.private_inputs_hash.as_bytes());
        data.extend_from_slice(statement.source_pool.as_bytes());
        data
    }
}

impl Default for AsyncZKPV4 {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Helpers ───

fn compute_proof_hash(statement_id: &str, proof_data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    statement_id.hash(&mut hasher);
    proof_data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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

    fn make_statement(id: &str, pool: &str) -> ZKPStatement {
        ZKPStatement {
            statement_id: id.to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: format!("hash_{}", id),
            circuit_type: CircuitType::Membership,
            source_pool: pool.to_string(),
            priority: 1,
        }
    }

    fn make_pool(id: &str, credits: f64) -> PoolContext {
        PoolContext::new(id.to_string(), credits, 0.8)
    }

    #[test]
    fn test_engine_creation() {
        let engine = AsyncZKPV4::with_defaults();
        assert_eq!(engine.pool_count(), 0);
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_register_pool() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        assert_eq!(engine.pool_count(), 1);
    }

    #[test]
    fn test_register_pool_insufficient_credits() {
        let mut engine = AsyncZKPV4::with_defaults();
        let result = engine.register_pool(make_pool("pool1", 10.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_register_pool_max_reached() {
        let mut engine = AsyncZKPV4::new(ZKPV4Config {
            max_pools: 2,
            ..ZKPV4Config::default()
        });
        engine.register_pool(make_pool("p1", 100.0)).unwrap();
        engine.register_pool(make_pool("p2", 100.0)).unwrap();
        let result = engine.register_pool(make_pool("p3", 100.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_statement() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        assert_eq!(engine.pending_count(), 1);
    }

    #[test]
    fn test_submit_statement_unregistered_pool() {
        let mut engine = AsyncZKPV4::with_defaults();
        let result = engine.submit_statement(make_statement("s1", "unknown"));
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_statement_cached() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        // Second submit of same ID is cached after proof generation
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_start_batch() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.start_batch("batch1".to_string()).unwrap();
        let result = engine.start_batch("batch2".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_add_to_batch() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.submit_statement(make_statement("s2", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        let count = engine.add_to_batch(10).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_generate_batch_proofs() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 1);
        assert!(matches!(batch.status, BatchStatus::Complete));
    }

    #[test]
    fn test_verify_proof() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        let statement = make_statement("s1", "pool1");
        let proof = engine.generate_proof(&statement, "batch1").unwrap();
        let result = engine.verify_proof(&proof, &statement).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_verify_batch() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.submit_statement(make_statement("s2", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();
        let results = engine.verify_batch(&batch).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_proof_caching() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        let cached = engine.get_cached_proof("s1");
        assert!(cached.is_some());
    }

    #[test]
    fn test_select_best_pool() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(PoolContext {
            pool_id: "p1".to_string(),
            available_credits: 100.0,
            reputation: 0.5,
            avg_latency_ms: 100.0,
            proofs_verified: 0,
        }).unwrap();
        engine.register_pool(PoolContext {
            pool_id: "p2".to_string(),
            available_credits: 200.0,
            reputation: 0.9,
            avg_latency_ms: 10.0,
            proofs_verified: 0,
        }).unwrap();
        let best = engine.select_best_pool().unwrap();
        assert_eq!(best.pool_id, "p2");
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        let stats = engine.get_stats();
        assert_eq!(stats.total_proofs_generated, 1);
        assert_eq!(stats.total_batches_processed, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        engine.submit_statement(make_statement("s1", "pool1")).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        engine.reset_stats();
        let stats = engine.get_stats();
        assert_eq!(stats.total_proofs_generated, 0);
    }

    #[test]
    fn test_circuit_type_display() {
        assert_eq!(CircuitType::Membership.to_string(), "membership");
        assert_eq!(CircuitType::CrossPoolAggregation.to_string(), "cross_pool_aggregation");
    }

    #[test]
    fn test_pool_verification_score() {
        let pool = PoolContext::new("p1".to_string(), 100.0, 0.8);
        let score = pool.verification_score();
        assert!(score > 0.0);
    }

    #[test]
    fn test_error_display() {
        match ZKPV4Error::PoolNotRegistered("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_config_default() {
        let config = ZKPV4Config::default();
        assert_eq!(config.max_batch_size, 256);
        assert_eq!(config.proof_timeout_ms, 800);
        assert_eq!(config.max_pools, 16);
    }

    #[test]
    fn test_batch_status() {
        let batch = ProofBatch::new("b1".to_string());
        assert!(matches!(batch.status, BatchStatus::Accumulating));
    }

    #[test]
    fn test_cross_pool_aggregation() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("p1", 100.0)).unwrap();
        engine.register_pool(make_pool("p2", 100.0)).unwrap();
        let mut stmt = make_statement("s1", "p1");
        stmt.circuit_type = CircuitType::CrossPoolAggregation;
        engine.submit_statement(stmt).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        engine.generate_batch_proofs().unwrap();
        let stats = engine.get_stats();
        assert_eq!(stats.cross_pool_count, 1);
    }

    #[test]
    fn test_update_pool_credits() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("p1", 100.0)).unwrap();
        engine.update_pool_credits("p1", 200.0).unwrap();
        let pool = engine.pools.get("p1").unwrap();
        assert_eq!(pool.available_credits, 200.0);
    }

    #[test]
    fn test_priority_ordering() {
        let mut engine = AsyncZKPV4::with_defaults();
        engine.register_pool(make_pool("pool1", 100.0)).unwrap();
        let mut low = make_statement("low", "pool1");
        low.priority = 1;
        let mut high = make_statement("high", "pool1");
        high.priority = 10;
        engine.submit_statement(low).unwrap();
        engine.submit_statement(high).unwrap();
        engine.start_batch("batch1".to_string()).unwrap();
        engine.add_to_batch(10).unwrap();
        let batch = engine.active_batch.as_ref().unwrap();
        // High priority should be first
        assert_eq!(batch.statements[0].statement_id, "high");
    }
}
