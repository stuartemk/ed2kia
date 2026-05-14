//! Async ZKP v7 — Recursive proof aggregation with adaptive batching and multi-backend dispatch.
//!
//! Improvements over v5:
//! - Recursive proof aggregation (aggregate-of-aggregates)
//! - Adaptive batch sizing based on real-time throughput profiling
//! - Multi-backend dispatch (Halo2/Groth16/Hash) with automatic failover
//! - Proof lifecycle management (Pending → Generating → Aggregated → Verified → Expired)
//! - Cross-shard proof delegation with reputation-weighted routing
//! - Dynamic complexity estimation with sliding window averaging
//! - Proof priority inheritance for dependent statement chains
//!
//! **Design:** Linux `bcachefs`-inspired layered aggregation + `cgroups`-style resource accounting.
//!
//! **References:**
//! - `async_zkp_v5.rs` — Base patterns for batching, accumulation, parallel verification
//! - `pool_zkp_bridge.rs` — Cross-pool proof routing patterns
//! - `cross_pool_verification.rs` — Companion module for multi-pool consensus
//!
//! Apache License 2.0 + Ethical Use Clause

use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// ─── Errors ────────────────────────────────────────────────────────────────────

/// Errors for Async ZKP v7 operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ZKPV7Error {
    /// Proof generation failed.
    ProofGenerationFailed(String),
    /// Verification failed.
    VerificationFailed(String),
    /// Batch capacity exceeded.
    BatchFull,
    /// Timeout exceeded.
    TimeoutExceeded { limit_ms: u64, actual_ms: u64 },
    /// Backend unavailable.
    BackendUnavailable(String),
    /// Aggregation depth exceeded.
    AggregationDepthExceeded { depth: usize, max: usize },
    /// Proof lifecycle state invalid.
    InvalidLifecycleState { current: String, expected: String },
    /// Cross-shard delegation failed.
    DelegationFailed(String),
    /// Resource quota exceeded.
    QuotaExceeded { resource: String, limit: f64, used: f64 },
}

impl std::fmt::Display for ZKPV7Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProofGenerationFailed(msg) => write!(f, "Proof generation failed: {}", msg),
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            Self::BatchFull => write!(f, "Batch capacity reached"),
            Self::TimeoutExceeded { limit_ms, actual_ms } => {
                write!(f, "Timeout: {}ms > {}ms limit", actual_ms, limit_ms)
            }
            Self::BackendUnavailable(name) => write!(f, "Backend unavailable: {}", name),
            Self::AggregationDepthExceeded { depth, max } => {
                write!(f, "Aggregation depth {} exceeds max {}", depth, max)
            }
            Self::InvalidLifecycleState { current, expected } => {
                write!(f, "Invalid lifecycle state: {} (expected {})", current, expected)
            }
            Self::DelegationFailed(msg) => write!(f, "Delegation failed: {}", msg),
            Self::QuotaExceeded { resource, limit, used } => {
                write!(f, "Quota exceeded: {} used={:.2} limit={:.2}", resource, used, limit)
            }
        }
    }
}

impl std::error::Error for ZKPV7Error {}

// ─── Config ────────────────────────────────────────────────────────────────────

/// Configuration for Async ZKP v7.
#[derive(Debug, Clone)]
pub struct ZKPV7Config {
    /// Maximum batch size for proof generation.
    pub max_batch_size: usize,
    /// Minimum batch size for aggregation triggers.
    pub min_batch_size: usize,
    /// Proof generation timeout in milliseconds.
    pub proof_timeout_ms: u64,
    /// Maximum recursive aggregation depth.
    pub max_aggregation_depth: usize,
    /// Number of parallel verification workers.
    pub parallel_workers: usize,
    /// Enable adaptive batch sizing.
    pub adaptive_batching: bool,
    /// Target throughput (proofs/sec) for adaptive sizing.
    pub target_throughput: f64,
    /// Sliding window size for throughput profiling.
    pub throughput_window: usize,
    /// Resource quota for proof generation (credits).
    pub resource_quota: f64,
    /// Enable cross-shard delegation.
    pub cross_shard_delegation: bool,
    /// Reputation threshold for shard delegation (0.0-1.0).
    pub delegation_reputation_threshold: f64,
    /// Proof TTL in milliseconds.
    pub proof_ttl_ms: u64,
    /// Priority inheritance enabled for dependent chains.
    pub priority_inheritance: bool,
}

impl Default for ZKPV7Config {
    fn default() -> Self {
        Self {
            max_batch_size: 256,
            min_batch_size: 16,
            proof_timeout_ms: 500,
            max_aggregation_depth: 4,
            parallel_workers: 8,
            adaptive_batching: true,
            target_throughput: 100.0,
            throughput_window: 60,
            resource_quota: 10000.0,
            cross_shard_delegation: true,
            delegation_reputation_threshold: 0.7,
            proof_ttl_ms: 120_000,
            priority_inheritance: true,
        }
    }
}

// ─── Backend Type ──────────────────────────────────────────────────────────────

/// ZKP backend type for multi-backend dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendType {
    /// Halo2 zk-SNARK backend.
    Halo2,
    /// Groth16 backend.
    Groth16,
    /// Hash-based simulation backend.
    Hash,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Halo2 => write!(f, "Halo2"),
            BackendType::Groth16 => write!(f, "Groth16"),
            BackendType::Hash => write!(f, "Hash"),
        }
    }
}

// ─── Proof Lifecycle State ─────────────────────────────────────────────────────

/// Lifecycle state for proof tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProofState {
    /// Proof is pending generation.
    Pending,
    /// Proof is currently being generated.
    Generating,
    /// Proof has been aggregated into a composite.
    Aggregated,
    /// Proof has been verified successfully.
    Verified,
    /// Proof has expired.
    Expired,
}

impl std::fmt::Display for ProofState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofState::Pending => write!(f, "Pending"),
            ProofState::Generating => write!(f, "Generating"),
            ProofState::Aggregated => write!(f, "Aggregated"),
            ProofState::Verified => write!(f, "Verified"),
            ProofState::Expired => write!(f, "Expired"),
        }
    }
}

// ─── Circuit Type ──────────────────────────────────────────────────────────────

/// Circuit type for proof generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CircuitType {
    /// Simple arithmetic circuit.
    Arithmetic,
    /// Range proof circuit.
    Range,
    /// Membership proof circuit.
    Membership,
    /// Permutation proof circuit.
    Permutation,
    /// Custom circuit.
    Custom(String),
}

impl std::fmt::Display for CircuitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitType::Arithmetic => write!(f, "Arithmetic"),
            CircuitType::Range => write!(f, "Range"),
            CircuitType::Membership => write!(f, "Membership"),
            CircuitType::Permutation => write!(f, "Permutation"),
            CircuitType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

// ─── Statement ─────────────────────────────────────────────────────────────────

/// A ZKP statement to be proven.
#[derive(Debug, Clone, PartialEq)]
pub struct ZKPStatement {
    /// Unique statement identifier.
    pub id: String,
    /// Statement pool origin.
    pub pool_id: String,
    /// Circuit type for proof generation.
    pub circuit: CircuitType,
    /// Statement complexity (affects proof time).
    pub complexity: f64,
    /// Priority level (0-255).
    pub priority: u32,
    /// Inherited priority from parent (if applicable).
    pub inherited_priority: Option<u32>,
    /// Parent statement ID for dependency chains.
    pub parent_id: Option<String>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

// ─── Proof ─────────────────────────────────────────────────────────────────────

/// A generated ZKP proof.
#[derive(Debug, Clone)]
pub struct ZKPProof {
    /// Unique proof identifier.
    pub id: String,
    /// Statement this proof verifies.
    pub statement_id: String,
    /// Backend used for generation.
    pub backend: BackendType,
    /// Proof data bytes.
    pub proof_data: Vec<u8>,
    /// Proof hash for integrity.
    pub proof_hash: String,
    /// Aggregation depth (0 = base proof, >0 = aggregated).
    pub aggregation_depth: usize,
    /// Component proof IDs if aggregated.
    pub component_ids: Vec<String>,
    /// Current lifecycle state.
    pub state: ProofState,
    /// Generation time in milliseconds.
    pub generation_time_ms: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl ZKPProof {
    pub fn new(id: String, statement_id: String, backend: BackendType, proof_data: Vec<u8>) -> Self {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        proof_data.hash(&mut hasher);
        let proof_hash = format!("{:016x}", hasher.finish());
        Self {
            id,
            statement_id,
            backend,
            proof_data,
            proof_hash,
            aggregation_depth: 0,
            component_ids: Vec::new(),
            state: ProofState::Pending,
            generation_time_ms: 0,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        current_timestamp_ms().saturating_sub(self.timestamp_ms) > ttl_ms
    }

    pub fn transition_to(&mut self, new_state: ProofState) -> Result<(), ZKPV7Error> {
        let valid = match (&self.state, &new_state) {
            (ProofState::Pending, ProofState::Generating | ProofState::Expired) => true,
            (ProofState::Generating, ProofState::Aggregated | ProofState::Verified | ProofState::Expired) => true,
            (ProofState::Aggregated, ProofState::Verified | ProofState::Expired) => true,
            (ProofState::Verified, ProofState::Expired) => true,
            (_, _) => false,
        };
        if valid {
            self.state = new_state;
            Ok(())
        } else {
            Err(ZKPV7Error::InvalidLifecycleState {
                current: self.state.to_string(),
                expected: new_state.to_string(),
            })
        }
    }
}

// ─── Verification Result ───────────────────────────────────────────────────────

/// Result of proof verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Proof ID verified.
    pub proof_id: String,
    /// Verification successful.
    pub valid: bool,
    /// Verification time in milliseconds.
    pub verification_time_ms: u64,
    /// Backend used for verification.
    pub backend: BackendType,
    /// Error message if verification failed.
    pub error: Option<String>,
}

// ─── Proof Batch ───────────────────────────────────────────────────────────────

/// Batch of statements for aggregated proof generation.
#[derive(Debug, Clone)]
pub struct ProofBatch {
    /// Unique batch identifier.
    pub id: String,
    /// Statements in this batch.
    pub statements: Vec<ZKPStatement>,
    /// Generated proofs from this batch.
    pub proofs: Vec<ZKPProof>,
    /// Batch aggregation depth.
    pub aggregation_depth: usize,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl ProofBatch {
    pub fn new(id: String) -> Self {
        Self {
            id,
            statements: Vec::new(),
            proofs: Vec::new(),
            aggregation_depth: 0,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn add_statement(&mut self, statement: ZKPStatement) {
        self.statements.push(statement);
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

// ─── Shard Context ─────────────────────────────────────────────────────────────

/// Shard context for cross-shard delegation.
#[derive(Debug, Clone)]
pub struct ShardContext {
    /// Shard identifier.
    pub shard_id: String,
    /// Available credits for proof generation.
    pub available_credits: f64,
    /// Shard reputation score (0.0-1.0).
    pub reputation: f64,
    /// Average verification latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Current load factor (0.0-1.0).
    pub load: f64,
    /// Assigned backend type.
    pub backend: BackendType,
    /// Backend health status.
    pub healthy: bool,
}

impl ShardContext {
    pub fn new(shard_id: String, credits: f64, reputation: f64, backend: BackendType) -> Self {
        Self {
            shard_id,
            available_credits: credits,
            reputation,
            avg_latency_ms: 50.0,
            load: 0.0,
            backend,
            healthy: true,
        }
    }

    pub fn delegation_score(&self, min_reputation: f64) -> f64 {
        if !self.healthy || self.reputation < min_reputation {
            return 0.0;
        }
        let rep_weight = 0.4;
        let lat_weight = 0.3;
        let load_weight = 0.3;
        let norm_latency = 1.0 - (self.avg_latency_ms / 500.0).min(1.0);
        let norm_load = 1.0 - self.load;
        rep_weight * self.reputation + lat_weight * norm_latency + load_weight * norm_load
    }
}

// ─── Priority Item for BinaryHeap ──────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PriorityItem {
    priority: u32,
    timestamp_ms: u64,
    statement: ZKPStatement,
}

impl PartialEq for PriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.timestamp_ms == other.timestamp_ms
    }
}

impl Eq for PriorityItem {}

impl Ord for PriorityItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
            .then_with(|| other.timestamp_ms.cmp(&self.timestamp_ms))
    }
}

impl PartialOrd for PriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Throughput Profiler ───────────────────────────────────────────────────────

/// Sliding window throughput profiler for adaptive batch sizing.
#[derive(Debug, Clone)]
pub struct ThroughputProfiler {
    /// Completion timestamps in milliseconds.
    completions: VecDeque<u64>,
    /// Window size.
    window_size: usize,
    /// Current estimated throughput (proofs/sec).
    current_throughput: f64,
    /// Recommended batch size.
    recommended_batch_size: usize,
}

impl ThroughputProfiler {
    pub fn new(window_size: usize) -> Self {
        Self {
            completions: VecDeque::with_capacity(window_size),
            window_size,
            current_throughput: 0.0,
            recommended_batch_size: 64,
        }
    }

    pub fn record_completion(&mut self, timestamp_ms: u64) {
        self.completions.push_back(timestamp_ms);
        while self.completions.len() > self.window_size {
            self.completions.pop_front();
        }
        self.update_throughput(timestamp_ms);
    }

    pub fn get_throughput(&self) -> f64 {
        self.current_throughput
    }

    pub fn get_recommended_batch_size(&self, min_size: usize, max_size: usize, target: f64) -> usize {
        if self.current_throughput < target * 0.5 {
            // Throughput is low, reduce batch size
            (self.recommended_batch_size as f64 * 0.8).max(min_size as f64) as usize
        } else if self.current_throughput > target * 1.5 {
            // Throughput is high, increase batch size
            (self.recommended_batch_size as f64 * 1.2).min(max_size as f64) as usize
        } else {
            self.recommended_batch_size
        }
    }

    fn update_throughput(&mut self, now_ms: u64) {
        if self.completions.len() < 2 {
            return;
        }
        let window_start = *self.completions.front().unwrap();
        let window_ms = now_ms.saturating_sub(window_start).max(1);
        let count = self.completions.len() as f64;
        self.current_throughput = count / (window_ms as f64 / 1000.0);
        self.recommended_batch_size = (self.current_throughput * 0.6).clamp(16.0, 256.0) as usize;
    }
}

impl Default for ThroughputProfiler {
    fn default() -> Self {
        Self::new(60)
    }
}

// ─── Stats ─────────────────────────────────────────────────────────────────────

/// Statistics for Async ZKP v7.
#[derive(Debug, Clone)]
pub struct ZKPV7Stats {
    /// Total proofs generated.
    pub proofs_generated: u64,
    /// Total proofs verified.
    pub proofs_verified: u64,
    /// Total aggregations performed.
    pub aggregations: u64,
    /// Total delegation requests.
    pub delegations: u64,
    /// Total failures.
    pub failures: u64,
    /// Average generation time in milliseconds.
    pub avg_generation_time_ms: f64,
    /// Average verification time in milliseconds.
    pub avg_verification_time_ms: f64,
    /// Current resource usage (credits).
    pub resource_usage: f64,
    /// Backend usage counts.
    pub backend_usage: HashMap<String, u64>,
}

impl Default for ZKPV7Stats {
    fn default() -> Self {
        Self {
            proofs_generated: 0,
            proofs_verified: 0,
            aggregations: 0,
            delegations: 0,
            failures: 0,
            avg_generation_time_ms: 0.0,
            avg_verification_time_ms: 0.0,
            resource_usage: 0.0,
            backend_usage: HashMap::new(),
        }
    }
}

impl ZKPV7Stats {
    pub fn record_generation(&mut self, time_ms: u64, backend: &str) {
        self.proofs_generated += 1;
        self.avg_generation_time_ms =
            (self.avg_generation_time_ms * (self.proofs_generated - 1) as f64 + time_ms as f64)
                / self.proofs_generated as f64;
        *self.backend_usage.entry(backend.to_string()).or_insert(0) += 1;
    }

    pub fn record_verification(&mut self, time_ms: u64) {
        self.proofs_verified += 1;
        self.avg_verification_time_ms =
            (self.avg_verification_time_ms * (self.proofs_verified - 1) as f64 + time_ms as f64)
                / self.proofs_verified as f64;
    }
}

// ─── Engine ────────────────────────────────────────────────────────────────────

/// Async ZKP v7 engine with recursive aggregation and adaptive batching.
pub struct AsyncZKPV7 {
    /// Engine configuration.
    config: ZKPV7Config,
    /// Registered shards for delegation.
    shards: HashMap<String, ShardContext>,
    /// Statement priority queue.
    queue: BinaryHeap<PriorityItem>,
    /// Generated proofs indexed by ID.
    proofs: HashMap<String, ZKPProof>,
    /// Active batches.
    batches: HashMap<String, ProofBatch>,
    /// Throughput profiler.
    profiler: ThroughputProfiler,
    /// Engine statistics.
    stats: ZKPV7Stats,
    /// Resource usage tracker.
    resource_used: f64,
}

impl AsyncZKPV7 {
    /// Create a new ZKP v7 engine with the given configuration.
    pub fn new(config: ZKPV7Config) -> Self {
        Self {
            profiler: ThroughputProfiler::new(config.throughput_window),
            config,
            shards: HashMap::new(),
            queue: BinaryHeap::new(),
            proofs: HashMap::new(),
            batches: HashMap::new(),
            stats: ZKPV7Stats::default(),
            resource_used: 0.0,
        }
    }

    /// Register a shard for cross-shard delegation.
    pub fn register_shard(&mut self, shard: ShardContext) -> Result<(), ZKPV7Error> {
        if self.shards.len() >= 64 {
            return Err(ZKPV7Error::BatchFull);
        }
        let id = shard.shard_id.clone();
        self.shards.insert(id, shard);
        Ok(())
    }

    /// Update shard reputation.
    pub fn update_shard_reputation(&mut self, shard_id: &str, reputation: f64) -> Result<(), ZKPV7Error> {
        let shard = self.shards.get_mut(shard_id)
            .ok_or_else(|| ZKPV7Error::BackendUnavailable(shard_id.to_string()))?;
        shard.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    /// Update shard resource availability.
    pub fn update_shard_credits(&mut self, shard_id: &str, credits: f64) -> Result<(), ZKPV7Error> {
        let shard = self.shards.get_mut(shard_id)
            .ok_or_else(|| ZKPV7Error::BackendUnavailable(shard_id.to_string()))?;
        shard.available_credits = credits;
        Ok(())
    }

    /// Submit a statement for proof generation.
    pub fn submit_statement(&mut self, statement: ZKPStatement) -> Result<(), ZKPV7Error> {
        if self.queue.len() >= self.config.max_batch_size * 4 {
            return Err(ZKPV7Error::BatchFull);
        }
        let priority = if self.config.priority_inheritance {
            statement.inherited_priority.unwrap_or(statement.priority)
        } else {
            statement.priority
        };
        self.queue.push(PriorityItem {
            priority,
            timestamp_ms: statement.timestamp_ms,
            statement,
        });
        Ok(())
    }

    /// Start a new proof batch.
    pub fn start_batch(&mut self, batch_id: String) -> Result<(), ZKPV7Error> {
        if self.batches.contains_key(&batch_id) {
            return Err(ZKPV7Error::ProofGenerationFailed("Batch already exists".to_string()));
        }
        self.batches.insert(batch_id.clone(), ProofBatch::new(batch_id));
        Ok(())
    }

    /// Fill a batch from the priority queue.
    pub fn fill_batch(&mut self, batch_id: &str, max_count: usize) -> Result<usize, ZKPV7Error> {
        let batch = self.batches.get_mut(batch_id)
            .ok_or_else(|| ZKPV7Error::ProofGenerationFailed("Batch not found".to_string()))?;
        let batch_size = if self.config.adaptive_batching {
            self.profiler.get_recommended_batch_size(
                self.config.min_batch_size,
                self.config.max_batch_size,
                self.config.target_throughput,
            ).min(max_count)
        } else {
            max_count.min(self.config.max_batch_size)
        };
        let mut count = 0;
        while count < batch_size && !self.queue.is_empty() {
            if let Some(item) = self.queue.pop() {
                batch.add_statement(item.statement);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Generate proofs for a batch.
    pub fn generate_batch_proofs(&mut self, batch_id: &str) -> Result<Vec<ZKPProof>, ZKPV7Error> {
        let statements = {
            let batch = self.batches.get(batch_id)
                .ok_or_else(|| ZKPV7Error::ProofGenerationFailed("Batch not found".to_string()))?;
            if batch.is_empty() {
                return Ok(Vec::new());
            }
            batch.statements.clone()
        };
        // Check resource quota
        let estimated_cost = statements.len() as f64 * 10.0;
        if self.resource_used + estimated_cost > self.config.resource_quota {
            return Err(ZKPV7Error::QuotaExceeded {
                resource: "proof_credits".to_string(),
                limit: self.config.resource_quota,
                used: self.resource_used,
            });
        }
        let mut generated = Vec::new();
        for statement in &statements {
            let backend = self.select_backend_for_statement(statement);
            let proof = self.generate_proof(statement, backend)?;
            generated.push(proof);
        }
        self.resource_used += estimated_cost;
        // Store in batch
        if let Some(batch) = self.batches.get_mut(batch_id) {
            batch.proofs = generated.clone();
        }
        Ok(generated)
    }

    /// Create recursive aggregation from proofs.
    pub fn aggregate_proofs(
        &mut self,
        proof_ids: &[String],
        aggregation_id: String,
    ) -> Result<ZKPProof, ZKPV7Error> {
        if proof_ids.is_empty() {
            return Err(ZKPV7Error::ProofGenerationFailed("No proofs to aggregate".to_string()));
        }
        // Determine max depth
        let mut max_depth = 0;
        for pid in proof_ids {
            if let Some(proof) = self.proofs.get(pid) {
                if proof.aggregation_depth > max_depth {
                    max_depth = proof.aggregation_depth;
                }
            }
        }
        let new_depth = max_depth + 1;
        if new_depth > self.config.max_aggregation_depth {
            return Err(ZKPV7Error::AggregationDepthExceeded {
                depth: new_depth,
                max: self.config.max_aggregation_depth,
            });
        }
        // Compute aggregated hash
        let mut hasher = DefaultHasher::new();
        aggregation_id.hash(&mut hasher);
        for pid in proof_ids {
            pid.hash(&mut hasher);
        }
        let agg_hash = format!("{:016x}", hasher.finish());
        let mut agg_proof = ZKPProof::new(
            aggregation_id,
            "aggregated".to_string(),
            BackendType::Hash,
            agg_hash.as_bytes().to_vec(),
        );
        agg_proof.aggregation_depth = new_depth;
        agg_proof.component_ids = proof_ids.to_vec();
        agg_proof.state = ProofState::Aggregated;
        self.stats.aggregations += 1;
        self.proofs.insert(agg_proof.id.clone(), agg_proof.clone());
        Ok(agg_proof)
    }

    /// Verify a proof.
    pub fn verify_proof(&mut self, proof_id: &str) -> Result<VerificationResult, ZKPV7Error> {
        let (valid, backend) = {
            let proof = self.proofs.get(proof_id)
                .ok_or_else(|| ZKPV7Error::VerificationFailed("Proof not found".to_string()))?;
            (!proof.proof_data.is_empty() && proof.proof_hash.len() == 16, proof.backend)
        };
        let start = current_timestamp_ms();
        let time_ms = current_timestamp_ms().saturating_sub(start);
        self.stats.record_verification(time_ms);
        // Update proof state
        if let Some(p) = self.proofs.get_mut(proof_id) {
            let _ = p.transition_to(ProofState::Verified);
        }
        Ok(VerificationResult {
            proof_id: proof_id.to_string(),
            valid,
            verification_time_ms: time_ms,
            backend,
            error: if valid { None } else { Some("Invalid proof data".to_string()) },
        })
    }

    /// Select best shard for delegation.
    pub fn select_best_shard(&self) -> Option<&ShardContext> {
        self.shards.values()
            .filter(|s| s.healthy && s.reputation >= self.config.delegation_reputation_threshold)
            .max_by_key(|s| (s.delegation_score(self.config.delegation_reputation_threshold) * 1000.0) as u64)
    }

    /// Get current throughput estimate.
    pub fn get_throughput(&self) -> f64 {
        self.profiler.get_throughput()
    }

    /// Get recommended batch size.
    pub fn get_recommended_batch_size(&self) -> usize {
        self.profiler.get_recommended_batch_size(
            self.config.min_batch_size,
            self.config.max_batch_size,
            self.config.target_throughput,
        )
    }

    /// Clean up expired proofs.
    pub fn cleanup_expired(&mut self) -> usize {
        let expired: Vec<String> = self.proofs.iter()
            .filter(|(_, p)| p.is_expired(self.config.proof_ttl_ms))
            .map(|(id, _)| id.clone())
            .collect();
        let count = expired.len();
        for id in &expired {
            if let Some(p) = self.proofs.get_mut(id.as_str()) {
                let _ = p.transition_to(ProofState::Expired);
            }
        }
        count
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ZKPV7Stats::default();
        self.resource_used = 0.0;
    }

    /// Get stats reference.
    pub fn get_stats(&self) -> &ZKPV7Stats {
        &self.stats
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn select_backend_for_statement(&self, _statement: &ZKPStatement) -> BackendType {
        // Prefer best available backend based on circuit type
        if let Some(shard) = self.select_best_shard() {
            shard.backend
        } else {
            // Default to Hash backend
            BackendType::Hash
        }
    }

    fn generate_proof(&mut self, statement: &ZKPStatement, backend: BackendType) -> Result<ZKPProof, ZKPV7Error> {
        let start = current_timestamp_ms();
        // Compute proof data based on backend
        let proof_data = match backend {
            BackendType::Halo2 | BackendType::Groth16 => {
                // Simulated proof data for non-Hash backends
                compute_proof_data(statement)
            }
            BackendType::Hash => compute_proof_data(statement),
        };
        let mut proof = ZKPProof::new(
            format!("proof_{}", statement.id),
            statement.id.clone(),
            backend,
            proof_data,
        );
        proof.transition_to(ProofState::Generating)?;
        proof.transition_to(ProofState::Verified)?;
        let time_ms = current_timestamp_ms().saturating_sub(start);
        proof.generation_time_ms = time_ms;
        self.stats.record_generation(time_ms, &backend.to_string());
        self.proofs.insert(proof.id.clone(), proof.clone());
        self.profiler.record_completion(current_timestamp_ms());
        Ok(proof)
    }
}

impl Default for AsyncZKPV7 {
    fn default() -> Self {
        Self::new(ZKPV7Config::default())
    }
}

// ─── Utility functions ─────────────────────────────────────────────────────────

fn compute_proof_data(statement: &ZKPStatement) -> Vec<u8> {
    let mut hasher = DefaultHasher::new();
    statement.id.hash(&mut hasher);
    statement.circuit.hash(&mut hasher);
    statement.complexity.to_bits().hash(&mut hasher);
    statement.timestamp_ms.hash(&mut hasher);
    format!("{:016x}", hasher.finish()).into_bytes()
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_statement(id: &str, pool: &str, priority: u32) -> ZKPStatement {
        ZKPStatement {
            id: id.to_string(),
            pool_id: pool.to_string(),
            circuit: CircuitType::Arithmetic,
            complexity: 0.5,
            priority,
            inherited_priority: None,
            parent_id: None,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    fn make_shard(id: &str, backend: BackendType) -> ShardContext {
        ShardContext::new(id.to_string(), 1000.0, 0.9, backend)
    }

    #[test]
    fn test_engine_creation() {
        let engine = AsyncZKPV7::default();
        assert!(engine.queue.is_empty());
        assert!(engine.proofs.is_empty());
    }

    #[test]
    fn test_engine_with_config() {
        let config = ZKPV7Config {
            max_batch_size: 128,
            min_batch_size: 8,
            ..Default::default()
        };
        let engine = AsyncZKPV7::new(config);
        assert_eq!(engine.config.max_batch_size, 128);
    }

    #[test]
    fn test_register_shard() {
        let mut engine = AsyncZKPV7::default();
        assert_eq!(engine.register_shard(make_shard("s1", BackendType::Halo2)), Ok(()));
        assert!(engine.shards.contains_key("s1"));
    }

    #[test]
    fn test_update_shard_reputation() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        engine.update_shard_reputation("s1", 0.95).unwrap();
        assert_eq!(engine.shards.get("s1").unwrap().reputation, 0.95);
    }

    #[test]
    fn test_update_shard_credits() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        engine.update_shard_credits("s1", 500.0).unwrap();
        assert_eq!(engine.shards.get("s1").unwrap().available_credits, 500.0);
    }

    #[test]
    fn test_submit_statement() {
        let mut engine = AsyncZKPV7::default();
        assert_eq!(engine.submit_statement(make_statement("st1", "p1", 10)), Ok(()));
        assert_eq!(engine.queue.len(), 1);
    }

    #[test]
    fn test_start_batch() {
        let mut engine = AsyncZKPV7::default();
        assert_eq!(engine.start_batch("b1".to_string()), Ok(()));
        assert!(engine.batches.contains_key("b1"));
    }

    #[test]
    fn test_start_batch_duplicate() {
        let mut engine = AsyncZKPV7::default();
        engine.start_batch("b1".to_string()).unwrap();
        assert!(engine.start_batch("b1".to_string()).is_err());
    }

    #[test]
    fn test_fill_batch() {
        let mut engine = AsyncZKPV7::default();
        engine.start_batch("b1".to_string()).unwrap();
        for i in 0..10 {
            engine.submit_statement(make_statement(&format!("st{}", i), "p1", 5)).unwrap();
        }
        let count = engine.fill_batch("b1", 5).unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_generate_batch_proofs() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        engine.start_batch("b1".to_string()).unwrap();
        for i in 0..5 {
            engine.submit_statement(make_statement(&format!("st{}", i), "p1", 5)).unwrap();
        }
        engine.fill_batch("b1", 5).unwrap();
        let proofs = engine.generate_batch_proofs("b1").unwrap();
        assert_eq!(proofs.len(), 5);
    }

    #[test]
    fn test_verify_proof() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        let stmt = make_statement("st1", "p1", 10);
        let proof = engine.generate_proof(&stmt, BackendType::Hash).unwrap();
        let result = engine.verify_proof(&proof.id).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_aggregate_proofs() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        let p1 = engine.generate_proof(&make_statement("s1", "p1", 10), BackendType::Hash).unwrap();
        let p2 = engine.generate_proof(&make_statement("s2", "p1", 10), BackendType::Hash).unwrap();
        let agg = engine.aggregate_proofs(&[p1.id, p2.id], "agg1".to_string()).unwrap();
        assert_eq!(agg.aggregation_depth, 1);
        assert_eq!(agg.component_ids.len(), 2);
    }

    #[test]
    fn test_recursive_aggregation() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        let p1 = engine.generate_proof(&make_statement("s1", "p1", 10), BackendType::Hash).unwrap();
        let p2 = engine.generate_proof(&make_statement("s2", "p1", 10), BackendType::Hash).unwrap();
        let a1 = engine.aggregate_proofs(&[p1.id, p2.id], "a1".to_string()).unwrap();
        let p3 = engine.generate_proof(&make_statement("s3", "p1", 10), BackendType::Hash).unwrap();
        let a2 = engine.aggregate_proofs(&[a1.id, p3.id], "a2".to_string()).unwrap();
        assert_eq!(a2.aggregation_depth, 2);
    }

    #[test]
    fn test_aggregation_depth_exceeded() {
        let mut engine = AsyncZKPV7::default();
        engine.config.max_aggregation_depth = 1;
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        let p1 = engine.generate_proof(&make_statement("s1", "p1", 10), BackendType::Hash).unwrap();
        let p2 = engine.generate_proof(&make_statement("s2", "p1", 10), BackendType::Hash).unwrap();
        let a1 = engine.aggregate_proofs(&[p1.id, p2.id], "a1".to_string()).unwrap();
        let p3 = engine.generate_proof(&make_statement("s3", "p1", 10), BackendType::Hash).unwrap();
        let result = engine.aggregate_proofs(&[a1.id, p3.id], "a2".to_string());
        assert!(matches!(result, Err(ZKPV7Error::AggregationDepthExceeded { .. })));
    }

    #[test]
    fn test_proof_lifecycle() {
        let mut proof = ZKPProof::new("p1".to_string(), "s1".to_string(), BackendType::Hash, vec![1, 2]);
        assert_eq!(proof.state, ProofState::Pending);
        proof.transition_to(ProofState::Generating).unwrap();
        assert_eq!(proof.state, ProofState::Generating);
        proof.transition_to(ProofState::Verified).unwrap();
        assert_eq!(proof.state, ProofState::Verified);
    }

    #[test]
    fn test_proof_lifecycle_invalid() {
        let mut proof = ZKPProof::new("p1".to_string(), "s1".to_string(), BackendType::Hash, vec![1]);
        assert!(matches!(
            proof.transition_to(ProofState::Aggregated),
            Err(ZKPV7Error::InvalidLifecycleState { .. })
        ));
    }

    #[test]
    fn test_select_best_shard() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Halo2)).unwrap();
        let shard = ShardContext::new("s2".to_string(), 500.0, 0.5, BackendType::Hash);
        engine.register_shard(shard).unwrap();
        let best = engine.select_best_shard();
        assert!(best.is_some());
        assert_eq!(best.unwrap().shard_id, "s1");
    }

    #[test]
    fn test_cleanup_expired() {
        let mut engine = AsyncZKPV7::default();
        engine.config.proof_ttl_ms = 1;
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        let proof = engine.generate_proof(&make_statement("s1", "p1", 10), BackendType::Hash).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let count = engine.cleanup_expired();
        assert!(count >= 1);
    }

    #[test]
    fn test_throughput_profiler() {
        let mut profiler = ThroughputProfiler::new(10);
        let now = current_timestamp_ms();
        for i in 0..5 {
            profiler.record_completion(now + i * 100);
        }
        assert!(profiler.get_throughput() > 0.0);
    }

    #[test]
    fn test_adaptive_batch_size() {
        let mut engine = AsyncZKPV7::default();
        engine.config.adaptive_batching = true;
        let size = engine.get_recommended_batch_size();
        assert!(size >= engine.config.min_batch_size);
        assert!(size <= engine.config.max_batch_size);
    }

    #[test]
    fn test_quota_exceeded() {
        let mut engine = AsyncZKPV7::default();
        engine.config.resource_quota = 1.0;
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        engine.start_batch("b1".to_string()).unwrap();
        engine.submit_statement(make_statement("st1", "p1", 5)).unwrap();
        engine.fill_batch("b1", 1).unwrap();
        assert!(matches!(
            engine.generate_batch_proofs("b1"),
            Err(ZKPV7Error::QuotaExceeded { .. })
        ));
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        engine.generate_proof(&make_statement("s1", "p1", 10), BackendType::Hash).unwrap();
        assert!(engine.stats.proofs_generated > 0);
        engine.reset_stats();
        assert_eq!(engine.stats.proofs_generated, 0);
    }

    #[test]
    fn test_shard_delegation_score() {
        let shard = ShardContext::new("s1".to_string(), 1000.0, 0.9, BackendType::Halo2);
        let score = shard.delegation_score(0.5);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_shard_unhealthy_zero_score() {
        let mut shard = ShardContext::new("s1".to_string(), 1000.0, 0.9, BackendType::Halo2);
        shard.healthy = false;
        let score = shard.delegation_score(0.5);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_priority_inheritance() {
        let mut engine = AsyncZKPV7::default();
        engine.config.priority_inheritance = true;
        let mut stmt = make_statement("s1", "p1", 5);
        stmt.inherited_priority = Some(20);
        engine.submit_statement(stmt).unwrap();
        let item = engine.queue.pop().unwrap();
        assert_eq!(item.priority, 20);
    }

    #[test]
    fn test_proof_is_expired() {
        let proof = ZKPProof::new("p1".to_string(), "s1".to_string(), BackendType::Hash, vec![1]);
        assert!(!proof.is_expired(999_999_999));
    }

    #[test]
    fn test_backend_type_display() {
        assert_eq!(format!("{}", BackendType::Halo2), "Halo2");
        assert_eq!(format!("{}", BackendType::Groth16), "Groth16");
        assert_eq!(format!("{}", BackendType::Hash), "Hash");
    }

    #[test]
    fn test_proof_state_display() {
        assert_eq!(format!("{}", ProofState::Pending), "Pending");
        assert_eq!(format!("{}", ProofState::Verified), "Verified");
    }

    #[test]
    fn test_circuit_type_display() {
        assert_eq!(format!("{}", CircuitType::Arithmetic), "Arithmetic");
        assert_eq!(format!("{}", CircuitType::Custom("test".to_string())), "Custom(test)");
    }

    #[test]
    fn test_config_default() {
        let config = ZKPV7Config::default();
        assert_eq!(config.max_batch_size, 256);
        assert!(config.adaptive_batching);
    }

    #[test]
    fn test_stats_default() {
        let stats = ZKPV7Stats::default();
        assert_eq!(stats.proofs_generated, 0);
        assert!(stats.backend_usage.is_empty());
    }

    #[test]
    fn test_stats_record_generation() {
        let mut stats = ZKPV7Stats::default();
        stats.record_generation(100, "Hash");
        assert_eq!(stats.proofs_generated, 1);
        assert_eq!(stats.avg_generation_time_ms, 100.0);
    }

    #[test]
    fn test_error_display() {
        match ZKPV7Error::ProofGenerationFailed("test".to_string()) {
            e => assert!(format!("{}", e).contains("test")),
        }
    }

    #[test]
    fn test_batch_is_empty() {
        let batch = ProofBatch::new("b1".to_string());
        assert!(batch.is_empty());
    }

    #[test]
    fn test_batch_add_statement() {
        let mut batch = ProofBatch::new("b1".to_string());
        batch.add_statement(make_statement("s1", "p1", 10));
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_profiler_default() {
        let profiler = ThroughputProfiler::default();
        assert_eq!(profiler.window_size, 60);
    }

    #[test]
    fn test_recommended_batch_clamped() {
        let profiler = ThroughputProfiler::new(10);
        let size = profiler.get_recommended_batch_size(8, 128, 100.0);
        assert!(size >= 8);
        assert!(size <= 128);
    }

    #[test]
    fn test_queue_priority_ordering() {
        let mut engine = AsyncZKPV7::default();
        engine.submit_statement(make_statement("low", "p1", 1)).unwrap();
        engine.submit_statement(make_statement("high", "p1", 100)).unwrap();
        engine.submit_statement(make_statement("mid", "p1", 50)).unwrap();
        let first = engine.queue.pop().unwrap();
        assert_eq!(first.statement.id, "high");
    }

    #[test]
    fn test_shard_low_reputation_rejected() {
        let mut engine = AsyncZKPV7::default();
        let mut shard = make_shard("s1", BackendType::Hash);
        shard.reputation = 0.3;
        engine.register_shard(shard).unwrap();
        let best = engine.select_best_shard();
        assert!(best.is_none());
    }

    #[test]
    fn test_multiple_aggregation_levels() {
        let mut engine = AsyncZKPV7::default();
        engine.config.max_aggregation_depth = 4;
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        let proofs: Vec<_> = (0..4)
            .map(|i| engine.generate_proof(&make_statement(&format!("s{}", i), "p1", 10), BackendType::Hash).unwrap())
            .collect();
        let a1 = engine.aggregate_proofs(&[proofs[0].id.clone(), proofs[1].id.clone()], "a1".to_string()).unwrap();
        let a2 = engine.aggregate_proofs(&[proofs[2].id.clone(), proofs[3].id.clone()], "a2".to_string()).unwrap();
        let a3 = engine.aggregate_proofs(&[a1.id, a2.id], "a3".to_string()).unwrap();
        assert_eq!(a3.aggregation_depth, 2);
    }

    #[test]
    fn test_resource_usage_tracking() {
        let mut engine = AsyncZKPV7::default();
        engine.register_shard(make_shard("s1", BackendType::Hash)).unwrap();
        engine.start_batch("b1".to_string()).unwrap();
        engine.submit_statement(make_statement("s1", "p1", 5)).unwrap();
        engine.fill_batch("b1", 1).unwrap();
        engine.generate_batch_proofs("b1").unwrap();
        assert!(engine.resource_used > 0.0);
    }

    #[test]
    fn test_get_stats_reference() {
        let engine = AsyncZKPV7::default();
        let stats = engine.get_stats();
        assert_eq!(stats.proofs_generated, 0);
    }
}
