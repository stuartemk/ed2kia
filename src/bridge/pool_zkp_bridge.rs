//! Pool ZKP Bridge — Bridges ZKP proofs with resource pools for cross-pool verification.
//!
//! This module connects the Async ZKP v4 engine with cross-chain resource pools,
//! enabling proof generation and verification that respects pool resource constraints.
//!
//! **Linux Analogy:** Like `auditd` + `journald` where ZKP proofs are audit logs
//! verified across multiple journal instances (pools) with resource-aware routing.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

// ─── Errors ───

/// Errors for Pool ZKP Bridge operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PoolZKPError {
    /// Pool not found.
    PoolNotFound(String),
    /// Proof not found.
    ProofNotFound(String),
    /// Verification failed.
    VerificationFailed(String),
    /// Insufficient pool resources.
    InsufficientResources { available: f64, required: f64 },
    /// Bridge capacity exceeded.
    BridgeFull,
    /// Consensus threshold not met.
    ConsensusFailed { yes: u64, no: u64 },
    /// Proof expired.
    ProofExpired(String),
}

impl std::fmt::Display for PoolZKPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolNotFound(id) => write!(f, "Pool not found: {}", id),
            Self::ProofNotFound(id) => write!(f, "Proof not found: {}", id),
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            Self::InsufficientResources {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient resources: available={}, required={}",
                    available, required
                )
            }
            Self::BridgeFull => write!(f, "Bridge capacity exceeded"),
            Self::ConsensusFailed { yes, no } => {
                write!(f, "Consensus failed: {} yes, {} no", yes, no)
            }
            Self::ProofExpired(id) => write!(f, "Proof expired: {}", id),
        }
    }
}

impl std::error::Error for PoolZKPError {}

// ─── Config ───

/// Configuration for the Pool ZKP Bridge.
#[derive(Debug, Clone)]
pub struct PoolZKPConfig {
    /// Maximum proofs in flight.
    pub max_proofs_in_flight: usize,
    /// Consensus threshold (0.0-1.0).
    pub consensus_threshold: f64,
    /// Proof TTL in milliseconds.
    pub proof_ttl_ms: u64,
    /// Maximum verification hops.
    pub max_verification_hops: u32,
    /// Resource cost per proof verification.
    pub resource_cost_per_proof: f64,
    /// Enable cross-pool aggregation.
    pub cross_pool_aggregation: bool,
}

impl Default for PoolZKPConfig {
    fn default() -> Self {
        Self {
            max_proofs_in_flight: 128,
            consensus_threshold: 0.67,
            proof_ttl_ms: 60_000,
            max_verification_hops: 3,
            resource_cost_per_proof: 5.0,
            cross_pool_aggregation: true,
        }
    }
}

// ─── Bridge Proof ───

/// A proof being bridged across pools.
#[derive(Debug, Clone)]
pub struct BridgeProof {
    /// Unique proof identifier.
    pub proof_id: String,
    /// Source pool ID.
    pub source_pool: String,
    /// Target pool IDs for verification.
    pub target_pools: Vec<String>,
    /// Proof hash.
    pub proof_hash: String,
    /// Merkle root for chain verification.
    pub merkle_root: String,
    /// Current verification hop count.
    pub verification_hops: u32,
    /// Verification status.
    pub verified: bool,
    /// Verification votes.
    pub votes: HashMap<String, bool>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Resource cost.
    pub resource_cost: f64,
}

impl BridgeProof {
    /// Create a new bridge proof.
    pub fn new(
        proof_id: String,
        source_pool: String,
        target_pools: Vec<String>,
        proof_hash: String,
    ) -> Self {
        Self {
            proof_id,
            source_pool,
            target_pools,
            merkle_root: compute_merkle_root(&proof_hash),
            proof_hash,
            verification_hops: 0,
            verified: false,
            votes: HashMap::new(),
            timestamp_ms: current_timestamp_ms(),
            resource_cost: 0.0,
        }
    }

    /// Check if proof is expired.
    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.timestamp_ms) > ttl_ms
    }

    /// Add a verification vote.
    pub fn add_vote(&mut self, pool_id: String, valid: bool) {
        self.votes.insert(pool_id, valid);
    }

    /// Compute consensus result.
    pub fn consensus(&self, threshold: f64) -> Result<bool, PoolZKPError> {
        let total = self.votes.len() as f64;
        if total == 0.0 {
            return Err(PoolZKPError::ConsensusFailed { yes: 0, no: 0 });
        }
        let yes = self.votes.values().filter(|&&v| v).count() as f64;
        let no = total - yes;
        let ratio = yes / total;
        if ratio >= threshold {
            Ok(true)
        } else {
            Err(PoolZKPError::ConsensusFailed {
                yes: yes as u64,
                no: no as u64,
            })
        }
    }
}

// ─── Pool State ───

/// State of a pool in the bridge.
#[derive(Debug, Clone)]
pub struct PoolBridgeState {
    /// Pool identifier.
    pub pool_id: String,
    /// Available resources for verification.
    pub available_resources: f64,
    /// Total proofs verified.
    pub proofs_verified: u64,
    /// Total proofs failed.
    pub proofs_failed: u64,
    /// Average verification time in milliseconds.
    pub avg_verification_time_ms: f64,
    /// Active proofs being verified.
    pub active_proofs: usize,
}

impl PoolBridgeState {
    /// Create a new pool bridge state.
    pub fn new(pool_id: String, available_resources: f64) -> Self {
        Self {
            pool_id,
            available_resources,
            proofs_verified: 0,
            proofs_failed: 0,
            avg_verification_time_ms: 0.0,
            active_proofs: 0,
        }
    }

    /// Check if pool can accept a proof.
    pub fn can_accept_proof(&self, cost: f64, max_active: usize) -> bool {
        self.available_resources >= cost && self.active_proofs < max_active
    }

    /// Record successful verification.
    pub fn record_success(&mut self, cost: f64, time_ms: u64) {
        self.available_resources = (self.available_resources - cost).max(0.0);
        self.proofs_verified += 1;
        self.active_proofs = self.active_proofs.saturating_sub(1);
        self.avg_verification_time_ms = self.avg_verification_time_ms * 0.9 + time_ms as f64 * 0.1;
    }

    /// Record failed verification.
    pub fn record_failure(&mut self) {
        self.proofs_failed += 1;
        self.active_proofs = self.active_proofs.saturating_sub(1);
    }

    /// Start proof verification.
    pub fn start_proof(&mut self) {
        self.active_proofs += 1;
    }
}

// ─── Verification Record ───

/// Record of a verification event.
#[derive(Debug, Clone)]
pub struct VerificationRecord {
    /// Proof ID.
    pub proof_id: String,
    /// Verifying pool ID.
    pub pool_id: String,
    /// Verification result.
    pub valid: bool,
    /// Time taken in milliseconds.
    pub time_ms: u64,
    /// Timestamp.
    pub timestamp_ms: u64,
}

// ─── Stats ───

/// Statistics for the Pool ZKP Bridge.
#[derive(Debug, Clone)]
pub struct BridgeStats {
    /// Total proofs bridged.
    pub total_proofs_bridged: u64,
    /// Total proofs verified.
    pub total_proofs_verified: u64,
    /// Total verifications failed.
    pub total_verifications_failed: u64,
    /// Total consensus reached.
    pub total_consensus_reached: u64,
    /// Average bridge time in milliseconds.
    pub avg_bridge_time_ms: f64,
    /// Total resources consumed.
    pub total_resources_consumed: f64,
}

impl Default for BridgeStats {
    fn default() -> Self {
        Self {
            total_proofs_bridged: 0,
            total_proofs_verified: 0,
            total_verifications_failed: 0,
            total_consensus_reached: 0,
            avg_bridge_time_ms: 0.0,
            total_resources_consumed: 0.0,
        }
    }
}

// ─── Bridge ───

/// Pool ZKP Bridge connecting ZKP proofs with resource pools.
pub struct PoolZKPBridge {
    /// Configuration.
    config: PoolZKPConfig,
    /// Pool states.
    pools: HashMap<String, PoolBridgeState>,
    /// Active bridge proofs.
    bridge_proofs: HashMap<String, BridgeProof>,
    /// Verification history.
    verification_history: VecDeque<VerificationRecord>,
    /// Statistics.
    stats: BridgeStats,
}

impl PoolZKPBridge {
    /// Create a new bridge with config.
    pub fn new(config: PoolZKPConfig) -> Self {
        Self {
            config,
            pools: HashMap::new(),
            bridge_proofs: HashMap::new(),
            verification_history: VecDeque::new(),
            stats: BridgeStats::default(),
        }
    }

    /// Create bridge with default config.
    pub fn with_defaults() -> Self {
        Self::new(PoolZKPConfig::default())
    }

    /// Register a pool for bridging.
    pub fn register_pool(&mut self, pool_id: String, resources: f64) -> Result<(), PoolZKPError> {
        if self.pools.len() >= self.config.max_proofs_in_flight {
            return Err(PoolZKPError::BridgeFull);
        }
        self.pools
            .insert(pool_id.clone(), PoolBridgeState::new(pool_id, resources));
        Ok(())
    }

    /// Update pool resources.
    pub fn update_pool_resources(
        &mut self,
        pool_id: &str,
        resources: f64,
    ) -> Result<(), PoolZKPError> {
        let pool = self
            .pools
            .get_mut(pool_id)
            .ok_or(PoolZKPError::PoolNotFound(pool_id.to_string()))?;
        pool.available_resources = resources.max(0.0);
        Ok(())
    }

    /// Submit a proof for cross-pool verification.
    pub fn submit_proof(&mut self, proof: BridgeProof) -> Result<(), PoolZKPError> {
        // Check capacity
        if self.bridge_proofs.len() >= self.config.max_proofs_in_flight {
            return Err(PoolZKPError::BridgeFull);
        }

        // Check source pool exists
        if !self.pools.contains_key(&proof.source_pool) {
            return Err(PoolZKPError::PoolNotFound(proof.source_pool.clone()));
        }

        // Check target pools exist
        for target in &proof.target_pools {
            if !self.pools.contains_key(target) {
                return Err(PoolZKPError::PoolNotFound(target.clone()));
            }
        }

        // Check resources
        let cost = proof.resource_cost.max(self.config.resource_cost_per_proof);
        for target in &proof.target_pools {
            let pool = self.pools.get(target).unwrap();
            if !pool.can_accept_proof(cost, self.config.max_proofs_in_flight) {
                return Err(PoolZKPError::InsufficientResources {
                    available: pool.available_resources,
                    required: cost,
                });
            }
        }

        // Start proofs on target pools
        for target in &proof.target_pools {
            if let Some(pool) = self.pools.get_mut(target) {
                pool.start_proof();
            }
        }

        self.bridge_proofs.insert(proof.proof_id.clone(), proof);
        self.stats.total_proofs_bridged += 1;

        Ok(())
    }

    /// Submit a verification vote from a pool.
    pub fn submit_vote(
        &mut self,
        proof_id: &str,
        pool_id: String,
        valid: bool,
    ) -> Result<(), PoolZKPError> {
        let proof = self
            .bridge_proofs
            .get_mut(proof_id)
            .ok_or(PoolZKPError::ProofNotFound(proof_id.to_string()))?;

        proof.add_vote(pool_id.clone(), valid);

        // Record verification
        let record = VerificationRecord {
            proof_id: proof_id.to_string(),
            pool_id,
            valid,
            time_ms: 0,
            timestamp_ms: current_timestamp_ms(),
        };
        self.verification_history.push_back(record);
        if self.verification_history.len() > 1000 {
            self.verification_history.pop_front();
        }

        Ok(())
    }

    /// Check consensus for a proof.
    pub fn check_consensus(&mut self, proof_id: &str) -> Result<bool, PoolZKPError> {
        let proof = self
            .bridge_proofs
            .get(proof_id)
            .ok_or(PoolZKPError::ProofNotFound(proof_id.to_string()))?;

        let result = proof.consensus(self.config.consensus_threshold)?;

        // Update proof status
        if let Some(proof) = self.bridge_proofs.get_mut(proof_id) {
            proof.verified = result;
            proof.verification_hops += 1;
        }

        if result {
            self.stats.total_consensus_reached += 1;
            self.stats.total_proofs_verified += 1;
        } else {
            self.stats.total_verifications_failed += 1;
        }

        Ok(result)
    }

    /// Complete a proof verification.
    pub fn complete_verification(
        &mut self,
        _proof_id: &str,
        pool_id: &str,
        valid: bool,
        time_ms: u64,
    ) -> Result<(), PoolZKPError> {
        let cost = self.config.resource_cost_per_proof;

        if let Some(pool) = self.pools.get_mut(pool_id) {
            if valid {
                pool.record_success(cost, time_ms);
            } else {
                pool.record_failure();
            }
        }

        self.stats.total_resources_consumed += cost;
        self.stats.avg_bridge_time_ms = self.stats.avg_bridge_time_ms * 0.9 + time_ms as f64 * 0.1;

        Ok(())
    }

    /// Remove expired proofs.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.bridge_proofs.len();
        self.bridge_proofs
            .retain(|_, proof| !proof.is_expired(self.config.proof_ttl_ms));
        before - self.bridge_proofs.len()
    }

    /// Get a bridge proof.
    pub fn get_proof(&self, proof_id: &str) -> Option<&BridgeProof> {
        self.bridge_proofs.get(proof_id)
    }

    /// Get pool state.
    pub fn get_pool_state(&self, pool_id: &str) -> Option<&PoolBridgeState> {
        self.pools.get(pool_id)
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> BridgeStats {
        self.stats.clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = BridgeStats::default();
    }

    /// Get verification history.
    pub fn get_verification_history(&self) -> Vec<&VerificationRecord> {
        self.verification_history.iter().collect()
    }

    /// Get active proof count.
    pub fn active_proof_count(&self) -> usize {
        self.bridge_proofs.len()
    }

    /// Get registered pool count.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

impl Default for PoolZKPBridge {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Helpers ───

fn compute_merkle_root(proof_hash: &str) -> String {
    let mut hasher = DefaultHasher::new();
    proof_hash.hash(&mut hasher);
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

    fn make_proof(id: &str, source: &str, targets: &[&str]) -> BridgeProof {
        BridgeProof::new(
            id.to_string(),
            source.to_string(),
            targets.iter().map(|s| s.to_string()).collect(),
            format!("hash_{}", id),
        )
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = PoolZKPBridge::with_defaults();
        assert_eq!(bridge.pool_count(), 0);
        assert_eq!(bridge.active_proof_count(), 0);
    }

    #[test]
    fn test_register_pool() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("pool1".to_string(), 100.0).unwrap();
        assert_eq!(bridge.pool_count(), 1);
    }

    #[test]
    fn test_update_pool_resources() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("pool1".to_string(), 100.0).unwrap();
        bridge.update_pool_resources("pool1", 200.0).unwrap();
        let state = bridge.get_pool_state("pool1").unwrap();
        assert_eq!(state.available_resources, 200.0);
    }

    #[test]
    fn test_submit_proof() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        let proof = make_proof("p1", "source", &["target1"]);
        bridge.submit_proof(proof).unwrap();
        assert_eq!(bridge.active_proof_count(), 1);
    }

    #[test]
    fn test_submit_proof_unregistered_pool() {
        let mut bridge = PoolZKPBridge::with_defaults();
        let proof = make_proof("p1", "unknown", &["target1"]);
        let result = bridge.submit_proof(proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_proof_insufficient_resources() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 1.0).unwrap();
        let proof = make_proof("p1", "source", &["target1"]);
        let result = bridge.submit_proof(proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_vote() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["target1"]))
            .unwrap();
        bridge
            .submit_vote("p1", "target1".to_string(), true)
            .unwrap();
        let proof = bridge.get_proof("p1").unwrap();
        assert_eq!(proof.votes.len(), 1);
    }

    #[test]
    fn test_consensus_reached() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("t1".to_string(), 100.0).unwrap();
        bridge.register_pool("t2".to_string(), 100.0).unwrap();
        bridge.register_pool("t3".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["t1", "t2", "t3"]))
            .unwrap();
        bridge.submit_vote("p1", "t1".to_string(), true).unwrap();
        bridge.submit_vote("p1", "t2".to_string(), true).unwrap();
        bridge.submit_vote("p1", "t3".to_string(), true).unwrap();
        let result = bridge.check_consensus("p1");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_consensus_failed() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("t1".to_string(), 100.0).unwrap();
        bridge.register_pool("t2".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["t1", "t2"]))
            .unwrap();
        bridge.submit_vote("p1", "t1".to_string(), true).unwrap();
        bridge.submit_vote("p1", "t2".to_string(), false).unwrap();
        let result = bridge.check_consensus("p1");
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_verification() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["target1"]))
            .unwrap();
        bridge
            .complete_verification("p1", "target1", true, 50)
            .unwrap();
        let state = bridge.get_pool_state("target1").unwrap();
        assert_eq!(state.proofs_verified, 1);
    }

    #[test]
    fn test_proof_expiration() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        let mut proof = make_proof("p1", "source", &["target1"]);
        proof.timestamp_ms = 0; // Expired
        bridge.submit_proof(proof).unwrap();
        let cleaned = bridge.cleanup_expired();
        assert_eq!(cleaned, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["target1"]))
            .unwrap();
        let stats = bridge.get_stats();
        assert_eq!(stats.total_proofs_bridged, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["target1"]))
            .unwrap();
        bridge.reset_stats();
        let stats = bridge.get_stats();
        assert_eq!(stats.total_proofs_bridged, 0);
    }

    #[test]
    fn test_pool_can_accept_proof() {
        let pool = PoolBridgeState::new("p1".to_string(), 100.0);
        assert!(pool.can_accept_proof(10.0, 100));
        assert!(!pool.can_accept_proof(200.0, 100));
    }

    #[test]
    fn test_pool_record_success() {
        let mut pool = PoolBridgeState::new("p1".to_string(), 100.0);
        pool.start_proof();
        pool.record_success(10.0, 50);
        assert_eq!(pool.proofs_verified, 1);
        assert_eq!(pool.active_proofs, 0);
    }

    #[test]
    fn test_pool_record_failure() {
        let mut pool = PoolBridgeState::new("p1".to_string(), 100.0);
        pool.start_proof();
        pool.record_failure();
        assert_eq!(pool.proofs_failed, 1);
    }

    #[test]
    fn test_verification_history() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["target1"]))
            .unwrap();
        bridge
            .submit_vote("p1", "target1".to_string(), true)
            .unwrap();
        assert_eq!(bridge.get_verification_history().len(), 1);
    }

    #[test]
    fn test_bridge_full() {
        let mut bridge = PoolZKPBridge::new(PoolZKPConfig {
            max_proofs_in_flight: 2,
            ..PoolZKPConfig::default()
        });
        bridge.register_pool("s".to_string(), 100.0).unwrap();
        bridge.register_pool("t".to_string(), 100.0).unwrap();
        bridge.submit_proof(make_proof("p1", "s", &["t"])).unwrap();
        bridge.submit_proof(make_proof("p2", "s", &["t"])).unwrap();
        let result = bridge.submit_proof(make_proof("p3", "s", &["t"]));
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        match PoolZKPError::PoolNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_config_default() {
        let config = PoolZKPConfig::default();
        assert_eq!(config.max_proofs_in_flight, 128);
        assert_eq!(config.consensus_threshold, 0.67);
        assert_eq!(config.proof_ttl_ms, 60_000);
    }

    #[test]
    fn test_stats_default() {
        let stats = BridgeStats::default();
        assert_eq!(stats.total_proofs_bridged, 0);
        assert_eq!(stats.total_consensus_reached, 0);
    }

    #[test]
    fn test_bridge_default() {
        let bridge = PoolZKPBridge::default();
        assert_eq!(bridge.pool_count(), 0);
    }

    #[test]
    fn test_merkle_root() {
        let proof = make_proof("p1", "s", &["t"]);
        assert!(!proof.merkle_root.is_empty());
        assert_ne!(proof.merkle_root, proof.proof_hash);
    }

    #[test]
    fn test_get_proof() {
        let mut bridge = PoolZKPBridge::with_defaults();
        bridge.register_pool("source".to_string(), 100.0).unwrap();
        bridge.register_pool("target1".to_string(), 100.0).unwrap();
        bridge
            .submit_proof(make_proof("p1", "source", &["target1"]))
            .unwrap();
        let proof = bridge.get_proof("p1");
        assert!(proof.is_some());
        assert!(bridge.get_proof("nonexistent").is_none());
    }
}
