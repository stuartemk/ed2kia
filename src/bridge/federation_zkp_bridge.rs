//! Federation ZKP Bridge — Connects Async ZKP v5 with federation shards for cross-pool verification.
//!
//! This module bridges the ZKP v5 engine with the federation layer, enabling:
//! - Cross-shard proof verification with consensus tracking
//! - Federation-aware proof routing based on shard capacity
//! - Proof aggregation across federation boundaries
//! - Automatic proof distribution to verifier pools
//! - Merkle root synchronization between shards
//!
//! **Linux Analogy:** Like `systemd-journal-remote` where ZKP proofs are journal entries
//! replicated across remote instances with integrity verification via Merkle roots.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// ─── Errors ───

/// Errors for Federation ZKP Bridge operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FederationZKPError {
    /// Shard not found.
    ShardNotFound(String),
    /// Proof not found.
    ProofNotFound(String),
    /// Verification failed.
    VerificationFailed(String),
    /// Insufficient shard resources.
    InsufficientResources { available: f64, required: f64 },
    /// Bridge capacity exceeded.
    BridgeFull,
    /// Consensus threshold not met.
    ConsensusFailed { yes: u64, no: u64 },
    /// Proof expired.
    ProofExpired(String),
    /// Merkle root mismatch.
    MerkleMismatch { expected: String, actual: String },
    /// Cross-shard routing failed.
    RoutingFailed(String),
}

impl std::fmt::Display for FederationZKPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
            Self::ProofNotFound(id) => write!(f, "Proof not found: {}", id),
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            Self::InsufficientResources { available, required } => {
                write!(f, "Insufficient resources: available={}, required={}", available, required)
            }
            Self::BridgeFull => write!(f, "Bridge capacity exceeded"),
            Self::ConsensusFailed { yes, no } => {
                write!(f, "Consensus failed: {} yes, {} no", yes, no)
            }
            Self::ProofExpired(id) => write!(f, "Proof expired: {}", id),
            Self::MerkleMismatch { expected, actual } => {
                write!(f, "Merkle mismatch: expected={}, actual={}", expected, actual)
            }
            Self::RoutingFailed(msg) => write!(f, "Routing failed: {}", msg),
        }
    }
}

impl std::error::Error for FederationZKPError {}

// ─── Config ───

/// Configuration for the Federation ZKP Bridge.
#[derive(Debug, Clone)]
pub struct FederationZKPConfig {
    /// Maximum proofs in flight across federation.
    pub max_proofs_in_flight: usize,
    /// Consensus threshold for cross-shard verification (0.0-1.0).
    pub consensus_threshold: f64,
    /// Proof TTL in milliseconds.
    pub proof_ttl_ms: u64,
    /// Maximum verification hops between shards.
    pub max_verification_hops: u32,
    /// Resource cost per proof verification.
    pub resource_cost_per_proof: f64,
    /// Enable cross-shard aggregation.
    pub cross_shard_aggregation: bool,
    /// Maximum shards in federation.
    pub max_shards: usize,
    /// Merkle root sync interval in milliseconds.
    pub merkle_sync_interval_ms: u64,
    /// Proof routing strategy (0 = round-robin, 1 = capacity-based, 2 = reputation-based).
    pub routing_strategy: u8,
}

impl Default for FederationZKPConfig {
    fn default() -> Self {
        Self {
            max_proofs_in_flight: 256,
            consensus_threshold: 0.67,
            proof_ttl_ms: 120_000,
            max_verification_hops: 4,
            resource_cost_per_proof: 5.0,
            cross_shard_aggregation: true,
            max_shards: 64,
            merkle_sync_interval_ms: 10_000,
            routing_strategy: 1, // Capacity-based by default.
        }
    }
}

// ─── Federation Proof ───

/// A proof being bridged across federation shards.
#[derive(Debug, Clone)]
pub struct FederationProof {
    /// Unique proof identifier.
    pub proof_id: String,
    /// Source shard ID.
    pub source_shard: String,
    /// Target shard IDs for verification.
    pub target_shards: Vec<String>,
    /// Proof hash.
    pub proof_hash: String,
    /// Merkle root for chain verification.
    pub merkle_root: String,
    /// Current verification hop count.
    pub verification_hops: u32,
    /// Verification status.
    pub verified: bool,
    /// Verification votes per shard.
    pub votes: HashMap<String, bool>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Resource cost.
    pub resource_cost: f64,
    /// Accumulator index from ZKP v5.
    pub accumulator_index: Option<u64>,
}

impl FederationProof {
    /// Create a new federation proof.
    pub fn new(
        proof_id: String,
        source_shard: String,
        target_shards: Vec<String>,
        proof_hash: String,
    ) -> Self {
        Self {
            proof_id,
            source_shard,
            target_shards,
            merkle_root: compute_merkle_root(&proof_hash),
            proof_hash,
            verification_hops: 0,
            verified: false,
            votes: HashMap::new(),
            timestamp_ms: current_timestamp_ms(),
            resource_cost: 0.0,
            accumulator_index: None,
        }
    }

    /// Check if proof is expired.
    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.timestamp_ms) > ttl_ms
    }

    /// Add a verification vote from a shard.
    pub fn add_vote(&mut self, shard_id: String, valid: bool) {
        self.votes.insert(shard_id, valid);
    }

    /// Compute consensus result across shards.
    pub fn consensus(&self, threshold: f64) -> Result<bool, FederationZKPError> {
        let total = self.votes.len() as f64;
        if total == 0.0 {
            return Err(FederationZKPError::ConsensusFailed { yes: 0, no: 0 });
        }
        let yes = self.votes.values().filter(|&&v| v).count() as f64;
        let no = total - yes;
        let ratio = yes / total;
        if ratio >= threshold {
            Ok(true)
        } else {
            Err(FederationZKPError::ConsensusFailed {
                yes: yes as u64,
                no: no as u64,
            })
        }
    }

    /// Verify Merkle root integrity.
    pub fn verify_merkle_root(&self, expected_root: &str) -> Result<(), FederationZKPError> {
        if self.merkle_root != expected_root {
            return Err(FederationZKPError::MerkleMismatch {
                expected: expected_root.to_string(),
                actual: self.merkle_root.clone(),
            });
        }
        Ok(())
    }
}

// ─── Shard State ───

/// State of a shard in the federation bridge.
#[derive(Debug, Clone)]
pub struct ShardBridgeState {
    /// Shard identifier.
    pub shard_id: String,
    /// Available resources for verification.
    pub available_resources: f64,
    /// Shard reputation score.
    pub reputation: f64,
    /// Total proofs verified.
    pub proofs_verified: u64,
    /// Total proofs failed.
    pub proofs_failed: u64,
    /// Average verification time in milliseconds.
    pub avg_verification_time_ms: f64,
    /// Active proofs being verified.
    pub active_proofs: usize,
    /// Current shard load (0.0-1.0).
    pub current_load: f64,
    /// Last Merkle root synced.
    pub last_merkle_root: String,
    /// Last sync timestamp.
    pub last_sync_ms: u64,
}

impl ShardBridgeState {
    /// Create a new shard bridge state.
    pub fn new(shard_id: String, available_resources: f64, reputation: f64) -> Self {
        Self {
            shard_id,
            available_resources,
            reputation,
            proofs_verified: 0,
            proofs_failed: 0,
            avg_verification_time_ms: 0.0,
            active_proofs: 0,
            current_load: 0.0,
            last_merkle_root: String::new(),
            last_sync_ms: 0,
        }
    }

    /// Check if shard can accept a proof.
    pub fn can_accept_proof(&self, cost: f64, max_active: usize) -> bool {
        self.available_resources >= cost && self.active_proofs < max_active
    }

    /// Record successful verification.
    pub fn record_success(&mut self, cost: f64, time_ms: u64) {
        self.available_resources = (self.available_resources - cost).max(0.0);
        self.proofs_verified += 1;
        self.active_proofs = self.active_proofs.saturating_sub(1);
        self.avg_verification_time_ms =
            self.avg_verification_time_ms * 0.9 + time_ms as f64 * 0.1;
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

    /// Compute routing score for proof assignment.
    pub fn routing_score(&self, strategy: u8) -> f64 {
        match strategy {
            0 => {
                // Round-robin: use reputation as tiebreaker.
                self.reputation
            }
            1 => {
                // Capacity-based: resources * (1 - load) * reputation.
                let load_factor = 1.0 - (self.current_load.min(0.95));
                self.available_resources * load_factor * self.reputation
            }
            2 => {
                // Reputation-based: reputation / latency.
                self.reputation / (1.0 + self.avg_verification_time_ms)
            }
            _ => {
                // Default to capacity-based.
                let load_factor = 1.0 - (self.current_load.min(0.95));
                self.available_resources * load_factor * self.reputation
            }
        }
    }
}

// ─── Verification Record ───

/// Record of a cross-shard verification event.
#[derive(Debug, Clone)]
pub struct FederationVerificationRecord {
    /// Proof ID.
    pub proof_id: String,
    /// Verifying shard ID.
    pub shard_id: String,
    /// Verification result.
    pub valid: bool,
    /// Time taken in milliseconds.
    pub time_ms: u64,
    /// Timestamp.
    pub timestamp_ms: u64,
    /// Verification hop count.
    pub hop_count: u32,
}

// ─── Merkle Sync Record ───

/// Record of a Merkle root synchronization event.
#[derive(Debug, Clone)]
pub struct MerkleSyncRecord {
    /// Source shard ID.
    pub source_shard: String,
    /// Target shard ID.
    pub target_shard: String,
    /// Merkle root synced.
    pub merkle_root: String,
    /// Timestamp.
    pub timestamp_ms: u64,
    /// Sync successful.
    pub success: bool,
}

// ─── Stats ───

/// Statistics for the Federation ZKP Bridge.
#[derive(Debug, Clone)]
pub struct FederationBridgeStats {
    /// Total proofs bridged across federation.
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
    /// Total Merkle syncs performed.
    pub total_merkle_syncs: u64,
    /// Cross-shard aggregations.
    pub cross_shard_aggregations: u64,
    /// Routing decisions made.
    pub routing_decisions: u64,
}

impl Default for FederationBridgeStats {
    fn default() -> Self {
        Self {
            total_proofs_bridged: 0,
            total_proofs_verified: 0,
            total_verifications_failed: 0,
            total_consensus_reached: 0,
            avg_bridge_time_ms: 0.0,
            total_resources_consumed: 0.0,
            total_merkle_syncs: 0,
            cross_shard_aggregations: 0,
            routing_decisions: 0,
        }
    }
}

// ─── Bridge ───

/// Federation ZKP Bridge connecting ZKP v5 proofs with federation shards.
pub struct FederationZKPBridge {
    /// Configuration.
    config: FederationZKPConfig,
    /// Shard states.
    shards: HashMap<String, ShardBridgeState>,
    /// Active federation proofs.
    federation_proofs: HashMap<String, FederationProof>,
    /// Verification history.
    verification_history: VecDeque<FederationVerificationRecord>,
    /// Merkle sync history.
    merkle_sync_history: VecDeque<MerkleSyncRecord>,
    /// Statistics.
    stats: FederationBridgeStats,
    /// Round-robin counter for routing.
    rr_counter: usize,
}

impl FederationZKPBridge {
    /// Create a new bridge with config.
    pub fn new(config: FederationZKPConfig) -> Self {
        Self {
            config,
            shards: HashMap::new(),
            federation_proofs: HashMap::new(),
            verification_history: VecDeque::new(),
            merkle_sync_history: VecDeque::new(),
            stats: FederationBridgeStats::default(),
            rr_counter: 0,
        }
    }

    /// Create bridge with default config.
    pub fn with_defaults() -> Self {
        Self::new(FederationZKPConfig::default())
    }

    /// Register a shard in the federation.
    pub fn register_shard(
        &mut self,
        shard_id: String,
        resources: f64,
        reputation: f64,
    ) -> Result<(), FederationZKPError> {
        if self.shards.len() >= self.config.max_shards {
            return Err(FederationZKPError::BridgeFull);
        }
        if self.shards.contains_key(&shard_id) {
            return Ok(()); // Already registered.
        }
        self.shards.insert(
            shard_id.clone(),
            ShardBridgeState::new(shard_id, resources, reputation),
        );
        Ok(())
    }

    /// Update shard resources.
    pub fn update_shard_resources(
        &mut self,
        shard_id: &str,
        resources: f64,
    ) -> Result<(), FederationZKPError> {
        let shard = self.shards.get_mut(shard_id)
            .ok_or(FederationZKPError::ShardNotFound(shard_id.to_string()))?;
        shard.available_resources = resources;
        Ok(())
    }

    /// Update shard reputation.
    pub fn update_shard_reputation(
        &mut self,
        shard_id: &str,
        reputation: f64,
    ) -> Result<(), FederationZKPError> {
        let shard = self.shards.get_mut(shard_id)
            .ok_or(FederationZKPError::ShardNotFound(shard_id.to_string()))?;
        shard.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    /// Update shard load.
    pub fn update_shard_load(
        &mut self,
        shard_id: &str,
        load: f64,
    ) -> Result<(), FederationZKPError> {
        let shard = self.shards.get_mut(shard_id)
            .ok_or(FederationZKPError::ShardNotFound(shard_id.to_string()))?;
        shard.current_load = load.clamp(0.0, 1.0);
        Ok(())
    }

    /// Submit a proof for cross-shard verification.
    pub fn submit_proof(&mut self, mut proof: FederationProof) -> Result<(), FederationZKPError> {
        if self.federation_proofs.len() >= self.config.max_proofs_in_flight {
            return Err(FederationZKPError::BridgeFull);
        }
        if self.federation_proofs.contains_key(&proof.proof_id) {
            return Ok(()); // Dedup.
        }

        // Verify source shard exists.
        if !self.shards.contains_key(&proof.source_shard) {
            return Err(FederationZKPError::ShardNotFound(proof.source_shard.clone()));
        }

        // Check source shard resources.
        let source = self.shards.get(&proof.source_shard).unwrap();
        if !source.can_accept_proof(self.config.resource_cost_per_proof, self.config.max_proofs_in_flight) {
            return Err(FederationZKPError::InsufficientResources {
                available: source.available_resources,
                required: self.config.resource_cost_per_proof,
            });
        }

        // Start proof on source shard.
        self.shards.get_mut(&proof.source_shard).unwrap().start_proof();

        proof.resource_cost = self.config.resource_cost_per_proof;
        self.federation_proofs.insert(proof.proof_id.clone(), proof);
        self.stats.total_proofs_bridged += 1;
        Ok(())
    }

    /// Submit a verification vote from a shard.
    pub fn submit_vote(
        &mut self,
        proof_id: &str,
        shard_id: &str,
        valid: bool,
    ) -> Result<(), FederationZKPError> {
        let proof = self.federation_proofs.get_mut(proof_id)
            .ok_or(FederationZKPError::ProofNotFound(proof_id.to_string()))?;

        proof.add_vote(shard_id.to_string(), valid);
        proof.verification_hops += 1;

        if proof.verification_hops > self.config.max_verification_hops {
            return Err(FederationZKPError::VerificationFailed(
                "Max verification hops exceeded".to_string(),
            ));
        }

        // Record verification.
        let record = FederationVerificationRecord {
            proof_id: proof_id.to_string(),
            shard_id: shard_id.to_string(),
            valid,
            time_ms: 1, // Simulated.
            timestamp_ms: current_timestamp_ms(),
            hop_count: proof.verification_hops,
        };
        self.verification_history.push_back(record);

        // Enforce history limit.
        while self.verification_history.len() > 1000 {
            self.verification_history.pop_front();
        }

        Ok(())
    }

    /// Check consensus for a proof.
    pub fn check_consensus(&mut self, proof_id: &str) -> Result<bool, FederationZKPError> {
        let proof = self.federation_proofs.get(proof_id)
            .ok_or(FederationZKPError::ProofNotFound(proof_id.to_string()))?;

        let result = proof.consensus(self.config.consensus_threshold)?;

        if result {
            self.stats.total_consensus_reached += 1;
        }
        Ok(result)
    }

    /// Complete verification for a proof.
    pub fn complete_verification(
        &mut self,
        proof_id: &str,
        valid: bool,
    ) -> Result<(), FederationZKPError> {
        let proof = self.federation_proofs.get(proof_id)
            .ok_or(FederationZKPError::ProofNotFound(proof_id.to_string()))?;

        // Complete on source shard.
        let shard = self.shards.get_mut(&proof.source_shard).unwrap();
        if valid {
            shard.record_success(self.config.resource_cost_per_proof, 1);
            self.stats.total_proofs_verified += 1;
        } else {
            shard.record_failure();
            self.stats.total_verifications_failed += 1;
        }

        self.stats.total_resources_consumed += self.config.resource_cost_per_proof;

        // Update proof status.
        if let Some(p) = self.federation_proofs.get_mut(proof_id) {
            p.verified = valid;
        }

        Ok(())
    }

    /// Route a proof to the best shard based on routing strategy.
    pub fn route_proof(&mut self, min_resources: f64) -> Option<String> {
        if self.shards.is_empty() {
            return None;
        }

        self.stats.routing_decisions += 1;

        match self.config.routing_strategy {
            0 => {
                // Round-robin.
                let shard_ids: Vec<&String> = self.shards.keys().collect();
                if shard_ids.is_empty() {
                    return None;
                }
                let id = shard_ids[self.rr_counter % shard_ids.len()].clone();
                self.rr_counter += 1;
                Some(id.clone())
            }
            _ => {
                // Capacity or reputation-based: find best scoring shard.
                self.shards.values()
                    .filter(|s| s.available_resources >= min_resources)
                    .max_by_key(|s| s.routing_score(self.config.routing_strategy) as u64)
                    .map(|s| s.shard_id.clone())
            }
        }
    }

    /// Sync Merkle root between shards.
    pub fn sync_merkle_root(
        &mut self,
        source_shard: &str,
        target_shard: &str,
        merkle_root: String,
    ) -> Result<(), FederationZKPError> {
        // Verify source shard exists.
        if !self.shards.contains_key(source_shard) {
            return Err(FederationZKPError::ShardNotFound(source_shard.to_string()));
        }
        // Verify target shard exists.
        if !self.shards.contains_key(target_shard) {
            return Err(FederationZKPError::ShardNotFound(target_shard.to_string()));
        }

        // Update target shard Merkle root.
        let target = self.shards.get_mut(target_shard).unwrap();
        target.last_merkle_root = merkle_root.clone();
        target.last_sync_ms = current_timestamp_ms();

        // Update source shard sync info.
        let source = self.shards.get_mut(source_shard).unwrap();
        source.last_merkle_root = merkle_root.clone();
        source.last_sync_ms = current_timestamp_ms();

        // Record sync.
        let record = MerkleSyncRecord {
            source_shard: source_shard.to_string(),
            target_shard: target_shard.to_string(),
            merkle_root,
            timestamp_ms: current_timestamp_ms(),
            success: true,
        };
        self.merkle_sync_history.push_back(record);

        // Enforce history limit.
        while self.merkle_sync_history.len() > 500 {
            self.merkle_sync_history.pop_front();
        }

        self.stats.total_merkle_syncs += 1;
        Ok(())
    }

    /// Broadcast Merkle root to all shards.
    pub fn broadcast_merkle_root(
        &mut self,
        source_shard: &str,
        merkle_root: String,
    ) -> Result<usize, FederationZKPError> {
        let target_ids: Vec<String> = self.shards.keys()
            .filter(|id| id != &source_shard)
            .cloned()
            .collect();
        let mut count = 0;
        for shard_id in target_ids {
            self.sync_merkle_root(source_shard, &shard_id, merkle_root.clone())?;
            count += 1;
        }
        Ok(count)
    }

    /// Clean up expired proofs.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.federation_proofs.len();
        self.federation_proofs.retain(|_id, proof| {
            !proof.is_expired(self.config.proof_ttl_ms)
        });
        before - self.federation_proofs.len()
    }

    /// Get proof by ID.
    pub fn get_proof(&self, proof_id: &str) -> Option<&FederationProof> {
        self.federation_proofs.get(proof_id)
    }

    /// Get shard state.
    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardBridgeState> {
        self.shards.get(shard_id)
    }

    /// Get verification history.
    pub fn get_verification_history(&self) -> &[FederationVerificationRecord] {
        self.verification_history.as_slices().0
    }

    /// Get Merkle sync history.
    pub fn get_merkle_sync_history(&self) -> &[MerkleSyncRecord] {
        self.merkle_sync_history.as_slices().0
    }

    /// Get stats.
    pub fn get_stats(&self) -> &FederationBridgeStats {
        &self.stats
    }

    /// Get config.
    pub fn get_config(&self) -> &FederationZKPConfig {
        &self.config
    }

    /// Get active proof count.
    pub fn active_proof_count(&self) -> usize {
        self.federation_proofs.len()
    }

    /// Get registered shard count.
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Reset stats.
    pub fn reset_stats(&mut self) {
        self.stats = FederationBridgeStats::default();
    }

    /// Check if cross-shard aggregation is needed.
    pub fn needs_cross_shard_aggregation(&self, proof: &FederationProof) -> bool {
        self.config.cross_shard_aggregation && proof.target_shards.len() > 1
    }
}

impl Default for FederationZKPBridge {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_merkle_root(proof_hash: &str) -> String {
    compute_hash(format!("merkle:{}", proof_hash).as_bytes())
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
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

    fn make_proof(id: &str, source: &str, targets: &[&str]) -> FederationProof {
        FederationProof::new(
            id.to_string(),
            source.to_string(),
            targets.iter().map(|t| t.to_string()).collect(),
            format!("hash-{}", id),
        )
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = FederationZKPBridge::with_defaults();
        assert_eq!(bridge.shard_count(), 0);
        assert_eq!(bridge.active_proof_count(), 0);
    }

    #[test]
    fn test_bridge_with_config() {
        let config = FederationZKPConfig {
            max_shards: 128,
            consensus_threshold: 0.75,
            ..FederationZKPConfig::default()
        };
        let bridge = FederationZKPBridge::new(config);
        assert_eq!(bridge.config.max_shards, 128);
    }

    #[test]
    fn test_register_shard() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        assert_eq!(bridge.shard_count(), 1);
    }

    #[test]
    fn test_register_shard_duplicate() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("shard1".to_string(), 200.0, 0.9).unwrap();
        assert_eq!(bridge.shard_count(), 1);
    }

    #[test]
    fn test_register_shard_max_reached() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.config.max_shards = 2;
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 100.0, 0.8).unwrap();
        let result = bridge.register_shard("s3".to_string(), 100.0, 0.8);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_shard_resources() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.update_shard_resources("shard1", 200.0).unwrap();
        assert_eq!(
            bridge.get_shard("shard1").unwrap().available_resources,
            200.0
        );
    }

    #[test]
    fn test_update_shard_reputation() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.update_shard_reputation("shard1", 0.95).unwrap();
        assert_eq!(bridge.get_shard("shard1").unwrap().reputation, 0.95);
    }

    #[test]
    fn test_update_shard_load() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.update_shard_load("shard1", 0.6).unwrap();
        assert_eq!(bridge.get_shard("shard1").unwrap().current_load, 0.6);
    }

    #[test]
    fn test_submit_proof() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("shard2".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &["shard2"]);
        bridge.submit_proof(proof).unwrap();
        assert_eq!(bridge.active_proof_count(), 1);
    }

    #[test]
    fn test_submit_proof_unknown_shard() {
        let mut bridge = FederationZKPBridge::with_defaults();
        let proof = make_proof("p1", "unknown", &["shard2"]);
        let result = bridge.submit_proof(proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_proof_duplicate() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &[]);
        bridge.submit_proof(proof.clone()).unwrap();
        bridge.submit_proof(proof).unwrap(); // Dedup.
        assert_eq!(bridge.active_proof_count(), 1);
    }

    #[test]
    fn test_submit_vote() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("shard2".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &["shard2"]);
        bridge.submit_proof(proof).unwrap();
        bridge.submit_vote("p1", "shard2", true).unwrap();
        let proof = bridge.get_proof("p1").unwrap();
        assert_eq!(proof.votes.len(), 1);
    }

    #[test]
    fn test_consensus_reached() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("shard2".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("shard3".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &["shard2", "shard3"]);
        bridge.submit_proof(proof).unwrap();
        bridge.submit_vote("p1", "shard2", true).unwrap();
        bridge.submit_vote("p1", "shard3", true).unwrap();
        let result = bridge.check_consensus("p1");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_consensus_failed() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("shard2".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &["shard2"]);
        bridge.submit_proof(proof).unwrap();
        bridge.submit_vote("p1", "shard2", false).unwrap();
        let result = bridge.check_consensus("p1");
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_verification() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &[]);
        bridge.submit_proof(proof).unwrap();
        bridge.complete_verification("p1", true).unwrap();
        assert_eq!(bridge.stats.total_proofs_verified, 1);
    }

    #[test]
    fn test_route_proof_capacity() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 200.0, 0.9).unwrap();
        let best = bridge.route_proof(50.0).unwrap();
        assert_eq!(best, "s2");
    }

    #[test]
    fn test_route_proof_round_robin() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.config.routing_strategy = 0;
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 200.0, 0.9).unwrap();
        let r1 = bridge.route_proof(50.0).unwrap();
        let r2 = bridge.route_proof(50.0).unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_sync_merkle_root() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 100.0, 0.8).unwrap();
        bridge.sync_merkle_root("s1", "s2", "root123".to_string()).unwrap();
        assert_eq!(bridge.get_shard("s2").unwrap().last_merkle_root, "root123");
        assert_eq!(bridge.stats.total_merkle_syncs, 1);
    }

    #[test]
    fn test_broadcast_merkle_root() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s3".to_string(), 100.0, 0.8).unwrap();
        let count = bridge.broadcast_merkle_root("s1", "root1".to_string()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_proof_expiration() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        let mut proof = make_proof("p1", "shard1", &[]);
        proof.timestamp_ms = 0; // Very old.
        bridge.submit_proof(proof).unwrap();
        let cleaned = bridge.cleanup_expired();
        assert_eq!(cleaned, 1);
    }

    #[test]
    fn test_get_proof() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &[]);
        bridge.submit_proof(proof).unwrap();
        assert!(bridge.get_proof("p1").is_some());
        assert!(bridge.get_proof("p2").is_none());
    }

    #[test]
    fn test_reset_stats() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("shard1".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "shard1", &[]);
        bridge.submit_proof(proof).unwrap();
        bridge.reset_stats();
        assert_eq!(bridge.stats.total_proofs_bridged, 0);
    }

    #[test]
    fn test_shard_can_accept_proof() {
        let shard = ShardBridgeState::new("s1".to_string(), 100.0, 0.8);
        assert!(shard.can_accept_proof(10.0, 100));
        assert!(!shard.can_accept_proof(200.0, 100));
    }

    #[test]
    fn test_shard_record_success() {
        let mut shard = ShardBridgeState::new("s1".to_string(), 100.0, 0.8);
        shard.start_proof();
        shard.record_success(10.0, 5);
        assert_eq!(shard.proofs_verified, 1);
        assert!(shard.available_resources < 100.0);
    }

    #[test]
    fn test_shard_record_failure() {
        let mut shard = ShardBridgeState::new("s1".to_string(), 100.0, 0.8);
        shard.start_proof();
        shard.record_failure();
        assert_eq!(shard.proofs_failed, 1);
    }

    #[test]
    fn test_routing_score_capacity() {
        let mut shard = ShardBridgeState::new("s1".to_string(), 100.0, 0.8);
        shard.current_load = 0.5;
        let score = shard.routing_score(1);
        assert!(score > 0.0);
    }

    #[test]
    fn test_routing_score_reputation() {
        let mut shard = ShardBridgeState::new("s1".to_string(), 100.0, 0.9);
        shard.avg_verification_time_ms = 10.0;
        let score = shard.routing_score(2);
        assert!(score > 0.0);
    }

    #[test]
    fn test_merkle_root_computation() {
        let root = compute_merkle_root("hash1");
        assert!(!root.is_empty());
    }

    #[test]
    fn test_proof_verify_merkle() {
        let proof = make_proof("p1", "s1", &["s2"]);
        proof.verify_merkle_root(&proof.merkle_root).unwrap();
    }

    #[test]
    fn test_proof_merkle_mismatch() {
        let proof = make_proof("p1", "s1", &["s2"]);
        let result = proof.verify_merkle_root("wrong_root");
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_is_expired() {
        let mut proof = make_proof("p1", "s1", &[]);
        assert!(!proof.is_expired(60_000));
        proof.timestamp_ms = 0;
        assert!(proof.is_expired(60_000));
    }

    #[test]
    fn test_needs_cross_shard_aggregation() {
        let bridge = FederationZKPBridge::with_defaults();
        let proof = make_proof("p1", "s1", &["s2", "s3"]);
        assert!(bridge.needs_cross_shard_aggregation(&proof));
        let proof_single = make_proof("p2", "s1", &["s2"]);
        // Single target shard does NOT need cross-shard aggregation
        assert!(!bridge.needs_cross_shard_aggregation(&proof_single));
    }

    #[test]
    fn test_verification_history() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "s1", &["s2"]);
        bridge.submit_proof(proof).unwrap();
        bridge.submit_vote("p1", "s2", true).unwrap();
        assert_eq!(bridge.get_verification_history().len(), 1);
    }

    #[test]
    fn test_merkle_sync_history() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 100.0, 0.8).unwrap();
        bridge.sync_merkle_root("s1", "s2", "root1".to_string()).unwrap();
        assert_eq!(bridge.get_merkle_sync_history().len(), 1);
    }

    #[test]
    fn test_config_default() {
        let config = FederationZKPConfig::default();
        assert_eq!(config.max_shards, 64);
        assert_eq!(config.consensus_threshold, 0.67);
    }

    #[test]
    fn test_stats_default() {
        let stats = FederationBridgeStats::default();
        assert_eq!(stats.total_proofs_bridged, 0);
        assert_eq!(stats.total_merkle_syncs, 0);
    }

    #[test]
    fn test_bridge_default() {
        let bridge = FederationZKPBridge::default();
        assert_eq!(bridge.shard_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = FederationZKPError::ShardNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_merkle_mismatch_error() {
        let err = FederationZKPError::MerkleMismatch {
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("a"));
        assert!(msg.contains("b"));
    }

    #[test]
    fn test_routing_failed_error() {
        let err = FederationZKPError::RoutingFailed("no path".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("no path"));
    }

    #[test]
    fn test_full_federation_pipeline() {
        let mut bridge = FederationZKPBridge::with_defaults();

        // Register shards.
        bridge.register_shard("shard1".to_string(), 200.0, 0.9).unwrap();
        bridge.register_shard("shard2".to_string(), 150.0, 0.85).unwrap();
        bridge.register_shard("shard3".to_string(), 100.0, 0.8).unwrap();

        // Submit proof.
        let proof = make_proof("p1", "shard1", &["shard2", "shard3"]);
        bridge.submit_proof(proof).unwrap();
        assert_eq!(bridge.active_proof_count(), 1);

        // Submit votes.
        bridge.submit_vote("p1", "shard2", true).unwrap();
        bridge.submit_vote("p1", "shard3", true).unwrap();

        // Check consensus.
        let consensus = bridge.check_consensus("p1").unwrap();
        assert!(consensus);

        // Complete verification.
        bridge.complete_verification("p1", true).unwrap();
        assert_eq!(bridge.stats.total_proofs_verified, 1);
        assert_eq!(bridge.stats.total_consensus_reached, 1);

        // Sync Merkle root.
        bridge.broadcast_merkle_root("shard1", "root_final".to_string()).unwrap();
        assert_eq!(bridge.stats.total_merkle_syncs, 2);

        // Route new proof.
        let routed = bridge.route_proof(50.0).unwrap();
        assert!(!routed.is_empty());
    }

    #[test]
    fn test_max_verification_hops() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.config.max_verification_hops = 1;
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.register_shard("s2".to_string(), 100.0, 0.8).unwrap();
        let proof = make_proof("p1", "s1", &["s2"]);
        bridge.submit_proof(proof).unwrap();
        bridge.submit_vote("p1", "s2", true).unwrap();
        let result = bridge.submit_vote("p1", "s2", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_bridge_full() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.config.max_proofs_in_flight = 1;
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.submit_proof(make_proof("p1", "s1", &[])).unwrap();
        let result = bridge.submit_proof(make_proof("p2", "s1", &[]));
        assert!(result.is_err());
    }

    #[test]
    fn test_insufficient_resources() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 1.0, 0.8).unwrap();
        let proof = make_proof("p1", "s1", &[]);
        let result = bridge.submit_proof(proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_config() {
        let bridge = FederationZKPBridge::with_defaults();
        let config = bridge.get_config();
        assert_eq!(config.max_shards, 64);
    }

    #[test]
    fn test_get_stats() {
        let bridge = FederationZKPBridge::with_defaults();
        let stats = bridge.get_stats();
        assert_eq!(stats.total_proofs_bridged, 0);
    }

    #[test]
    fn test_reputation_clamping() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.update_shard_reputation("s1", 1.5).unwrap();
        assert_eq!(bridge.get_shard("s1").unwrap().reputation, 1.0);
        bridge.update_shard_reputation("s1", -0.5).unwrap();
        assert_eq!(bridge.get_shard("s1").unwrap().reputation, 0.0);
    }

    #[test]
    fn test_load_clamping() {
        let mut bridge = FederationZKPBridge::with_defaults();
        bridge.register_shard("s1".to_string(), 100.0, 0.8).unwrap();
        bridge.update_shard_load("s1", 1.5).unwrap();
        assert_eq!(bridge.get_shard("s1").unwrap().current_load, 1.0);
        bridge.update_shard_load("s1", -0.5).unwrap();
        assert_eq!(bridge.get_shard("s1").unwrap().current_load, 0.0);
    }
}
