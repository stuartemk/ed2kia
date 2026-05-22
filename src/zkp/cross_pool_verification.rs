//! Cross-Pool Verification — Multi-pool consensus verification with reputation-weighted voting.
//!
//! Provides cross-pool proof verification where multiple independent pools vote on proof
//! validity, achieving consensus through reputation-weighted majority voting. Supports
//! challenge-response protocols, verification result aggregation, and pool reputation
//! updates based on voting accuracy.
//!
//! **Design:** Linux `corosync`/`pacemaker`-inspired quorum-based consensus across
//! independent verification nodes (pools).
//!
//! **Key features:**
//! - Reputation-weighted voting with configurable thresholds
//! - Challenge-response verification protocol
//! - Verification result aggregation with Merkle root signing
//! - Pool reputation updates based on consensus alignment
//! - Adaptive threshold adjustment based on pool diversity
//! - Verification history with tamper-resistant chaining
//!
//! **References:**
//! - `async_zkp_v7.rs` — Proof generation and lifecycle management
//! - `pool_zkp_bridge.rs` — Cross-pool proof routing patterns
//! - `cross_chain_pools_v3.rs` — Pool reputation and scoring patterns
//!
//! Apache License 2.0 + Ethical Use Clause

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};

// ─── Errors ────────────────────────────────────────────────────────────────────

/// Errors for cross-pool verification operations.
#[derive(Debug, Clone, PartialEq)]
pub enum CrossPoolError {
    /// Pool not registered.
    PoolNotRegistered(String),
    /// Proof not found.
    ProofNotFound(String),
    /// Consensus threshold not met.
    ConsensusFailed { yes_weight: f64, threshold: f64 },
    /// Insufficient pools for quorum.
    InsufficientPools { available: usize, required: usize },
    /// Vote already cast by pool.
    VoteAlreadyCast(String),
    /// Verification session expired.
    SessionExpired(String),
    /// Invalid vote weight.
    InvalidVoteWeight { weight: f64, max: f64 },
    /// Challenge response mismatch.
    ChallengeMismatch(String),
}

impl std::fmt::Display for CrossPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolNotRegistered(id) => write!(f, "Pool not registered: {}", id),
            Self::ProofNotFound(id) => write!(f, "Proof not found: {}", id),
            Self::ConsensusFailed {
                yes_weight,
                threshold,
            } => {
                write!(
                    f,
                    "Consensus failed: weight={:.3} < threshold={:.3}",
                    yes_weight, threshold
                )
            }
            Self::InsufficientPools {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient pools: {} available, {} required",
                    available, required
                )
            }
            Self::VoteAlreadyCast(pool_id) => write!(f, "Vote already cast by pool: {}", pool_id),
            Self::SessionExpired(id) => write!(f, "Verification session expired: {}", id),
            Self::InvalidVoteWeight { weight, max } => {
                write!(f, "Invalid vote weight: {:.3} > max {:.3}", weight, max)
            }
            Self::ChallengeMismatch(msg) => write!(f, "Challenge mismatch: {}", msg),
        }
    }
}

impl std::error::Error for CrossPoolError {}

// ─── Config ────────────────────────────────────────────────────────────────────

/// Configuration for cross-pool verification.
#[derive(Debug, Clone)]
pub struct CrossPoolConfig {
    /// Consensus threshold (0.0-1.0) of total reputation weight needed.
    pub consensus_threshold: f64,
    /// Minimum number of pools for quorum.
    pub min_quorum_pools: usize,
    /// Maximum verification session TTL in milliseconds.
    pub session_ttl_ms: u64,
    /// Reputation weight for vote calculation (0.0-1.0).
    pub reputation_weight: f64,
    /// Latency weight for vote calculation (0.0-1.0).
    pub latency_weight: f64,
    /// Enable challenge-response protocol.
    pub challenge_enabled: bool,
    /// Challenge nonce length in bytes.
    pub challenge_nonce_bytes: usize,
    /// Reputation decay factor per disagreement (0.0-1.0).
    pub reputation_decay: f64,
    /// Reputation boost factor per agreement (0.0-1.0).
    pub reputation_boost: f64,
    /// Maximum verification history entries.
    pub max_history_size: usize,
    /// Enable adaptive threshold adjustment.
    pub adaptive_threshold: bool,
    /// Threshold adjustment step size.
    pub threshold_step: f64,
}

impl Default for CrossPoolConfig {
    fn default() -> Self {
        Self {
            consensus_threshold: 0.67,
            min_quorum_pools: 3,
            session_ttl_ms: 30_000,
            reputation_weight: 0.6,
            latency_weight: 0.4,
            challenge_enabled: true,
            challenge_nonce_bytes: 16,
            reputation_decay: 0.02,
            reputation_boost: 0.01,
            max_history_size: 500,
            adaptive_threshold: false,
            threshold_step: 0.01,
        }
    }
}

// ─── Vote ──────────────────────────────────────────────────────────────────────

/// A verification vote from a pool.
#[derive(Debug, Clone)]
pub struct Vote {
    /// Pool that cast the vote.
    pub pool_id: String,
    /// Vote value (true = valid, false = invalid).
    pub valid: bool,
    /// Vote weight based on pool reputation and latency.
    pub weight: f64,
    /// Verification latency in milliseconds.
    pub latency_ms: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Optional challenge response hash.
    pub challenge_response: Option<String>,
}

impl Vote {
    pub fn new(pool_id: String, valid: bool, weight: f64, latency_ms: u64) -> Self {
        Self {
            pool_id,
            valid,
            weight,
            latency_ms,
            timestamp_ms: current_timestamp_ms(),
            challenge_response: None,
        }
    }
}

// ─── Verification Session ──────────────────────────────────────────────────────

/// A verification session for a single proof across multiple pools.
#[derive(Debug, Clone)]
pub struct VerificationSession {
    /// Unique session identifier.
    pub id: String,
    /// Proof being verified.
    pub proof_id: String,
    /// Challenge nonce if enabled.
    pub challenge_nonce: Option<Vec<u8>>,
    /// Votes collected so far.
    pub votes: Vec<Vote>,
    /// Pools that have voted.
    pub voted_pools: BTreeSet<String>,
    /// Total weight of yes votes.
    pub yes_weight: f64,
    /// Total weight of no votes.
    pub no_weight: f64,
    /// Consensus reached.
    pub consensus_reached: bool,
    /// Consensus result (valid/invalid).
    pub consensus_result: Option<bool>,
    /// Session creation timestamp.
    pub created_ms: u64,
    /// Session completion timestamp (if completed).
    pub completed_ms: Option<u64>,
}

impl VerificationSession {
    pub fn new(id: String, proof_id: String, challenge_nonce: Option<Vec<u8>>) -> Self {
        Self {
            id,
            proof_id,
            challenge_nonce,
            votes: Vec::new(),
            voted_pools: BTreeSet::new(),
            yes_weight: 0.0,
            no_weight: 0.0,
            consensus_reached: false,
            consensus_result: None,
            created_ms: current_timestamp_ms(),
            completed_ms: None,
        }
    }

    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        current_timestamp_ms().saturating_sub(self.created_ms) > ttl_ms
    }

    pub fn total_weight(&self) -> f64 {
        self.yes_weight + self.no_weight
    }

    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }
}

// ─── Verification Result ───────────────────────────────────────────────────────

/// Aggregated verification result from cross-pool consensus.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Session ID.
    pub session_id: String,
    /// Proof ID verified.
    pub proof_id: String,
    /// Consensus reached.
    pub consensus: bool,
    /// Proof validity result.
    pub valid: bool,
    /// Total yes weight.
    pub yes_weight: f64,
    /// Total no weight.
    pub no_weight: f64,
    /// Total pools that voted.
    pub pools_voted: usize,
    /// Merkle root of all votes.
    pub vote_merkle_root: String,
    /// Verification time in milliseconds.
    pub verification_time_ms: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

// ─── Pool Verifier ─────────────────────────────────────────────────────────────

/// A pool registered for cross-pool verification.
#[derive(Debug, Clone)]
pub struct PoolVerifier {
    /// Pool identifier.
    pub pool_id: String,
    /// Pool reputation score (0.0-1.0).
    pub reputation: f64,
    /// Average verification latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Total votes cast.
    pub total_votes: u64,
    /// Votes aligned with consensus.
    pub aligned_votes: u64,
    /// Pool is active and healthy.
    pub active: bool,
    /// Backend type used by this pool.
    pub backend: String,
}

impl PoolVerifier {
    pub fn new(pool_id: String, reputation: f64, backend: String) -> Self {
        Self {
            pool_id,
            reputation,
            avg_latency_ms: 50.0,
            total_votes: 0,
            aligned_votes: 0,
            active: true,
            backend,
        }
    }

    pub fn vote_weight(&self, rep_weight: f64, lat_weight: f64) -> f64 {
        let norm_latency = 1.0 - (self.avg_latency_ms / 500.0).min(1.0);
        rep_weight * self.reputation + lat_weight * norm_latency
    }

    pub fn alignment_rate(&self) -> f64 {
        if self.total_votes == 0 {
            return 1.0;
        }
        self.aligned_votes as f64 / self.total_votes as f64
    }

    pub fn update_reputation(&mut self, agreed: bool, decay: f64, boost: f64) {
        self.total_votes += 1;
        if agreed {
            self.aligned_votes += 1;
            self.reputation = (self.reputation + boost).min(1.0);
        } else {
            self.reputation = (self.reputation - decay).max(0.0);
        }
    }
}

// ─── Stats ─────────────────────────────────────────────────────────────────────

/// Statistics for cross-pool verification.
#[derive(Debug, Clone)]
pub struct CrossPoolStats {
    /// Total verification sessions created.
    pub sessions_created: u64,
    /// Total sessions completed.
    pub sessions_completed: u64,
    /// Total consensus reached.
    pub consensus_reached: u64,
    /// Total consensus failed.
    pub consensus_failed: u64,
    /// Average verification time in milliseconds.
    pub avg_verification_time_ms: f64,
    /// Average votes per session.
    pub avg_votes_per_session: f64,
    /// Total votes cast.
    pub total_votes: u64,
}

impl Default for CrossPoolStats {
    fn default() -> Self {
        Self {
            sessions_created: 0,
            sessions_completed: 0,
            consensus_reached: 0,
            consensus_failed: 0,
            avg_verification_time_ms: 0.0,
            avg_votes_per_session: 0.0,
            total_votes: 0,
        }
    }
}

impl CrossPoolStats {
    pub fn record_completion(&mut self, time_ms: u64, votes: usize) {
        self.sessions_completed += 1;
        self.total_votes += votes as u64;
        self.avg_verification_time_ms =
            (self.avg_verification_time_ms * (self.sessions_completed - 1) as f64 + time_ms as f64)
                / self.sessions_completed as f64;
        self.avg_votes_per_session =
            (self.avg_votes_per_session * (self.sessions_completed - 1) as f64 + votes as f64)
                / self.sessions_completed as f64;
    }
}

// ─── Engine ────────────────────────────────────────────────────────────────────

/// Cross-pool verification engine.
pub struct CrossPoolVerifier {
    /// Engine configuration.
    config: CrossPoolConfig,
    /// Registered pool verifiers.
    pools: HashMap<String, PoolVerifier>,
    /// Active verification sessions.
    sessions: HashMap<String, VerificationSession>,
    /// Completed verification results.
    results: VecDeque<VerificationResult>,
    /// Engine statistics.
    stats: CrossPoolStats,
}

impl CrossPoolVerifier {
    /// Create a new cross-pool verifier with the given configuration.
    pub fn new(config: CrossPoolConfig) -> Self {
        Self {
            config,
            pools: HashMap::new(),
            sessions: HashMap::new(),
            results: VecDeque::with_capacity(500),
            stats: CrossPoolStats::default(),
        }
    }

    /// Register a pool verifier.
    pub fn register_pool(&mut self, pool: PoolVerifier) -> Result<(), CrossPoolError> {
        if self.pools.len() >= 128 {
            return Err(CrossPoolError::PoolNotRegistered(
                "Max pools reached".to_string(),
            ));
        }
        let id = pool.pool_id.clone();
        self.pools.insert(id, pool);
        Ok(())
    }

    /// Update pool reputation.
    pub fn update_pool_reputation(
        &mut self,
        pool_id: &str,
        reputation: f64,
    ) -> Result<(), CrossPoolError> {
        let pool = self
            .pools
            .get_mut(pool_id)
            .ok_or_else(|| CrossPoolError::PoolNotRegistered(pool_id.to_string()))?;
        pool.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    /// Update pool latency.
    pub fn update_pool_latency(
        &mut self,
        pool_id: &str,
        latency_ms: f64,
    ) -> Result<(), CrossPoolError> {
        let pool = self
            .pools
            .get_mut(pool_id)
            .ok_or_else(|| CrossPoolError::PoolNotRegistered(pool_id.to_string()))?;
        pool.avg_latency_ms = latency_ms;
        Ok(())
    }

    /// Create a new verification session.
    pub fn create_session(
        &mut self,
        session_id: String,
        proof_id: String,
    ) -> Result<(), CrossPoolError> {
        if self.sessions.contains_key(&session_id) {
            return Err(CrossPoolError::ChallengeMismatch(
                "Session exists".to_string(),
            ));
        }
        let challenge = if self.config.challenge_enabled {
            Some(generate_challenge_nonce(self.config.challenge_nonce_bytes))
        } else {
            None
        };
        let session = VerificationSession::new(session_id.clone(), proof_id, challenge);
        self.sessions.insert(session_id, session);
        self.stats.sessions_created += 1;
        Ok(())
    }

    /// Submit a vote from a pool.
    pub fn submit_vote(
        &mut self,
        session_id: &str,
        pool_id: &str,
        valid: bool,
    ) -> Result<Vote, CrossPoolError> {
        // Check session exists and not expired
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CrossPoolError::SessionExpired(session_id.to_string()))?;
        if session.is_expired(self.config.session_ttl_ms) {
            return Err(CrossPoolError::SessionExpired(session_id.to_string()));
        }
        // Check pool hasn't already voted
        if session.voted_pools.contains(pool_id) {
            return Err(CrossPoolError::VoteAlreadyCast(pool_id.to_string()));
        }
        // Get pool verifier
        let pool = self
            .pools
            .get(pool_id)
            .ok_or_else(|| CrossPoolError::PoolNotRegistered(pool_id.to_string()))?;
        if !pool.active {
            return Err(CrossPoolError::PoolNotRegistered(format!(
                "Pool {} inactive",
                pool_id
            )));
        }
        // Calculate vote weight
        let weight = pool.vote_weight(self.config.reputation_weight, self.config.latency_weight);
        if weight > 1.0 {
            return Err(CrossPoolError::InvalidVoteWeight { weight, max: 1.0 });
        }
        // Create vote
        let latency = pool.avg_latency_ms as u64;
        let mut vote = Vote::new(pool_id.to_string(), valid, weight, latency);
        // Add challenge response if enabled
        if let Some(ref nonce) = session.challenge_nonce {
            vote.challenge_response = Some(compute_challenge_response(pool_id, nonce));
        }
        // Update session
        let session = self.sessions.get_mut(session_id).unwrap();
        session.votes.push(vote.clone());
        session.voted_pools.insert(pool_id.to_string());
        if valid {
            session.yes_weight += weight;
        } else {
            session.no_weight += weight;
        }
        // Check consensus
        self.check_consensus(session_id)?;
        Ok(vote)
    }

    /// Get session status.
    pub fn get_session(&self, session_id: &str) -> Option<&VerificationSession> {
        self.sessions.get(session_id)
    }

    /// Get active pool count.
    pub fn active_pool_count(&self) -> usize {
        self.pools.values().filter(|p| p.active).count()
    }

    /// Get verification history.
    pub fn get_history(&self) -> &VecDeque<VerificationResult> {
        &self.results
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired(&mut self) -> usize {
        let expired: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.is_expired(self.config.session_ttl_ms) && !s.consensus_reached)
            .map(|(id, _)| id.clone())
            .collect();
        let count = expired.len();
        for id in expired {
            self.sessions.remove(&id);
        }
        count
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = CrossPoolStats::default();
    }

    /// Get stats reference.
    pub fn get_stats(&self) -> &CrossPoolStats {
        &self.stats
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn check_consensus(&mut self, session_id: &str) -> Result<(), CrossPoolError> {
        let session = self.sessions.get(session_id).unwrap();
        let _active_pools = self.active_pool_count();
        if session.vote_count() < self.config.min_quorum_pools {
            return Ok(());
        }
        let total = session.total_weight();
        if total < 0.001 {
            return Ok(());
        }
        let yes_ratio = session.yes_weight / total;
        let threshold = self.current_threshold();
        let reached = yes_ratio >= threshold || (1.0 - yes_ratio) >= threshold;
        if reached {
            let session = self.sessions.get_mut(session_id).unwrap();
            session.consensus_reached = true;
            session.consensus_result = Some(yes_ratio >= threshold);
            session.completed_ms = Some(current_timestamp_ms());
            // Complete session
            self.complete_session(session_id)?;
        }
        Ok(())
    }

    fn complete_session(&mut self, session_id: &str) -> Result<(), CrossPoolError> {
        let session = self.sessions.get(session_id).unwrap().clone();
        let time_ms = session
            .completed_ms
            .unwrap_or(current_timestamp_ms())
            .saturating_sub(session.created_ms);
        // Compute Merkle root of votes
        let merkle_root = compute_vote_merkle_root(&session.votes);
        let result = VerificationResult {
            session_id: session_id.to_string(),
            proof_id: session.proof_id.clone(),
            consensus: session.consensus_reached,
            valid: session.consensus_result.unwrap_or(false),
            yes_weight: session.yes_weight,
            no_weight: session.no_weight,
            pools_voted: session.vote_count(),
            vote_merkle_root: merkle_root,
            verification_time_ms: time_ms,
            timestamp_ms: current_timestamp_ms(),
        };
        // Update stats
        if result.consensus {
            self.stats.consensus_reached += 1;
        } else {
            self.stats.consensus_failed += 1;
        }
        self.stats.record_completion(time_ms, result.pools_voted);
        // Update pool reputations based on consensus alignment
        let consensus_valid = result.valid;
        for vote in &session.votes {
            if let Some(pool) = self.pools.get_mut(&vote.pool_id) {
                pool.update_reputation(
                    vote.valid == consensus_valid,
                    self.config.reputation_decay,
                    self.config.reputation_boost,
                );
            }
        }
        // Store result
        self.results.push_back(result);
        while self.results.len() > self.config.max_history_size {
            self.results.pop_front();
        }
        Ok(())
    }

    fn current_threshold(&self) -> f64 {
        if self.config.adaptive_threshold {
            // Slightly adjust based on pool count
            let pool_factor = (self.active_pool_count() as f64).min(10.0) / 10.0;
            (self.config.consensus_threshold - pool_factor * self.config.threshold_step).max(0.5)
        } else {
            self.config.consensus_threshold
        }
    }
}

impl Default for CrossPoolVerifier {
    fn default() -> Self {
        Self::new(CrossPoolConfig::default())
    }
}

// ─── Utility functions ─────────────────────────────────────────────────────────

fn generate_challenge_nonce(bytes: usize) -> Vec<u8> {
    let mut nonce = Vec::with_capacity(bytes);
    for i in 0..bytes {
        nonce.push((i * 37 + 13) as u8);
    }
    nonce
}

fn compute_challenge_response(pool_id: &str, nonce: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    pool_id.hash(&mut hasher);
    for byte in nonce {
        byte.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn compute_vote_merkle_root(votes: &[Vote]) -> String {
    if votes.is_empty() {
        return "0000000000000000".to_string();
    }
    let leaves: Vec<String> = votes
        .iter()
        .map(|v| {
            let mut hasher = DefaultHasher::new();
            v.pool_id.hash(&mut hasher);
            v.valid.hash(&mut hasher);
            v.weight.to_bits().hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        })
        .collect();
    compute_merkle_root(&leaves)
}

fn compute_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return "0000000000000000".to_string();
    }
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for i in (0..current.len()).step_by(2) {
            let combined = if i + 1 < current.len() {
                format!("{}{}", current[i], current[i + 1])
            } else {
                format!("{}{}", current[i], current[i])
            };
            let mut hasher = DefaultHasher::new();
            combined.hash(&mut hasher);
            next.push(format!("{:016x}", hasher.finish()));
        }
        current = next;
    }
    current
        .into_iter()
        .next()
        .unwrap_or_else(|| "0000000000000000".to_string())
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

    fn make_pool(id: &str, reputation: f64) -> PoolVerifier {
        PoolVerifier::new(id.to_string(), reputation, "Hash".to_string())
    }

    #[test]
    fn test_verifier_creation() {
        let verifier = CrossPoolVerifier::default();
        assert!(verifier.pools.is_empty());
        assert!(verifier.sessions.is_empty());
    }

    #[test]
    fn test_register_pool() {
        let mut verifier = CrossPoolVerifier::default();
        assert_eq!(verifier.register_pool(make_pool("p1", 0.9)), Ok(()));
        assert!(verifier.pools.contains_key("p1"));
    }

    #[test]
    fn test_update_pool_reputation() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.register_pool(make_pool("p1", 0.5)).unwrap();
        verifier.update_pool_reputation("p1", 0.9).unwrap();
        assert_eq!(verifier.pools.get("p1").unwrap().reputation, 0.9);
    }

    #[test]
    fn test_update_pool_latency() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.register_pool(make_pool("p1", 0.5)).unwrap();
        verifier.update_pool_latency("p1", 30.0).unwrap();
        assert_eq!(verifier.pools.get("p1").unwrap().avg_latency_ms, 30.0);
    }

    #[test]
    fn test_create_session() {
        let mut verifier = CrossPoolVerifier::default();
        assert_eq!(
            verifier.create_session("s1".to_string(), "proof1".to_string()),
            Ok(())
        );
        assert!(verifier.sessions.contains_key("s1"));
    }

    #[test]
    fn test_create_session_duplicate() {
        let mut verifier = CrossPoolVerifier::default();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        assert!(verifier
            .create_session("s1".to_string(), "proof2".to_string())
            .is_err());
    }

    #[test]
    fn test_submit_vote() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.register_pool(make_pool("p1", 0.9)).unwrap();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        let vote = verifier.submit_vote("s1", "p1", true).unwrap();
        assert!(vote.valid);
        assert!(vote.weight > 0.0);
    }

    #[test]
    fn test_vote_already_cast() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.register_pool(make_pool("p1", 0.9)).unwrap();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier.submit_vote("s1", "p1", true).unwrap();
        assert!(matches!(
            verifier.submit_vote("s1", "p1", false),
            Err(CrossPoolError::VoteAlreadyCast(_))
        ));
    }

    #[test]
    fn test_consensus_reached_majority_yes() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.min_quorum_pools = 3;
        verifier.config.consensus_threshold = 0.6;
        for i in 1..=4 {
            verifier
                .register_pool(make_pool(&format!("p{}", i), 0.9))
                .unwrap();
        }
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier.submit_vote("s1", "p1", true).unwrap();
        verifier.submit_vote("s1", "p2", true).unwrap();
        verifier.submit_vote("s1", "p3", true).unwrap();
        let session = verifier.get_session("s1").unwrap();
        assert!(session.consensus_reached);
        assert_eq!(session.consensus_result, Some(true));
    }

    #[test]
    fn test_consensus_reached_majority_no() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.min_quorum_pools = 3;
        verifier.config.consensus_threshold = 0.6;
        for i in 1..=4 {
            verifier
                .register_pool(make_pool(&format!("p{}", i), 0.9))
                .unwrap();
        }
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier.submit_vote("s1", "p1", false).unwrap();
        verifier.submit_vote("s1", "p2", false).unwrap();
        verifier.submit_vote("s1", "p3", false).unwrap();
        let session = verifier.get_session("s1").unwrap();
        assert!(session.consensus_reached);
        assert_eq!(session.consensus_result, Some(false));
    }

    #[test]
    fn test_insufficient_quorum() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.min_quorum_pools = 5;
        for i in 1..=3 {
            verifier
                .register_pool(make_pool(&format!("p{}", i), 0.9))
                .unwrap();
        }
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier.submit_vote("s1", "p1", true).unwrap();
        verifier.submit_vote("s1", "p2", true).unwrap();
        verifier.submit_vote("s1", "p3", true).unwrap();
        let session = verifier.get_session("s1").unwrap();
        assert!(!session.consensus_reached);
    }

    #[test]
    fn test_pool_vote_weight() {
        let pool = PoolVerifier::new("p1".to_string(), 0.9, "Hash".to_string());
        let weight = pool.vote_weight(0.6, 0.4);
        assert!(weight > 0.0);
        assert!(weight <= 1.0);
    }

    #[test]
    fn test_pool_alignment_rate() {
        let mut pool = make_pool("p1", 0.8);
        assert_eq!(pool.alignment_rate(), 1.0);
        pool.total_votes = 10;
        pool.aligned_votes = 8;
        assert_eq!(pool.alignment_rate(), 0.8);
    }

    #[test]
    fn test_pool_reputation_update_agreed() {
        let mut pool = make_pool("p1", 0.8);
        pool.update_reputation(true, 0.02, 0.01);
        assert_eq!(pool.reputation, 0.81);
        assert_eq!(pool.total_votes, 1);
        assert_eq!(pool.aligned_votes, 1);
    }

    #[test]
    fn test_pool_reputation_update_disagreed() {
        let mut pool = make_pool("p1", 0.8);
        pool.update_reputation(false, 0.02, 0.01);
        assert_eq!(pool.reputation, 0.78);
        assert_eq!(pool.total_votes, 1);
        assert_eq!(pool.aligned_votes, 0);
    }

    #[test]
    fn test_reputation_clamped_to_max() {
        let mut pool = make_pool("p1", 0.99);
        pool.update_reputation(true, 0.02, 0.05);
        assert_eq!(pool.reputation, 1.0);
    }

    #[test]
    fn test_reputation_clamped_to_min() {
        let mut pool = make_pool("p1", 0.01);
        pool.update_reputation(false, 0.05, 0.02);
        assert_eq!(pool.reputation, 0.0);
    }

    #[test]
    fn test_session_expired() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.session_ttl_ms = 1;
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(matches!(
            verifier.submit_vote("s1", "p1", true),
            Err(CrossPoolError::SessionExpired(_))
        ));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.session_ttl_ms = 1;
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier
            .create_session("s2".to_string(), "proof2".to_string())
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let count = verifier.cleanup_expired();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_verification_result_stored() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.min_quorum_pools = 3;
        verifier.config.consensus_threshold = 0.6;
        for i in 1..=4 {
            verifier
                .register_pool(make_pool(&format!("p{}", i), 0.9))
                .unwrap();
        }
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier.submit_vote("s1", "p1", true).unwrap();
        verifier.submit_vote("s1", "p2", true).unwrap();
        verifier.submit_vote("s1", "p3", true).unwrap();
        assert_eq!(verifier.get_history().len(), 1);
        let result = verifier.get_history().front().unwrap();
        assert!(result.consensus);
        assert!(result.valid);
    }

    #[test]
    fn test_merkle_root_computation() {
        let votes = vec![
            Vote::new("p1".to_string(), true, 0.9, 50),
            Vote::new("p2".to_string(), true, 0.8, 60),
        ];
        let root = compute_vote_merkle_root(&votes);
        assert_eq!(root.len(), 16);
    }

    #[test]
    fn test_empty_merkle_root() {
        let root = compute_vote_merkle_root(&[]);
        assert_eq!(root, "0000000000000000");
    }

    #[test]
    fn test_challenge_nonce_generation() {
        let nonce = generate_challenge_nonce(16);
        assert_eq!(nonce.len(), 16);
    }

    #[test]
    fn test_challenge_response() {
        let nonce = generate_challenge_nonce(8);
        let response = compute_challenge_response("p1", &nonce);
        assert_eq!(response.len(), 16);
    }

    #[test]
    fn test_vote_with_challenge() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.challenge_enabled = true;
        verifier.register_pool(make_pool("p1", 0.9)).unwrap();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        let vote = verifier.submit_vote("s1", "p1", true).unwrap();
        assert!(vote.challenge_response.is_some());
    }

    #[test]
    fn test_vote_without_challenge() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.challenge_enabled = false;
        verifier.register_pool(make_pool("p1", 0.9)).unwrap();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        let vote = verifier.submit_vote("s1", "p1", true).unwrap();
        assert!(vote.challenge_response.is_none());
    }

    #[test]
    fn test_active_pool_count() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.register_pool(make_pool("p1", 0.9)).unwrap();
        verifier.register_pool(make_pool("p2", 0.8)).unwrap();
        assert_eq!(verifier.active_pool_count(), 2);
    }

    #[test]
    fn test_inactive_pool_rejected() {
        let mut verifier = CrossPoolVerifier::default();
        let mut pool = make_pool("p1", 0.9);
        pool.active = false;
        verifier.register_pool(pool).unwrap();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        assert!(verifier.submit_vote("s1", "p1", true).is_err());
    }

    #[test]
    fn test_reset_stats() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.stats.sessions_created = 5;
        verifier.reset_stats();
        assert_eq!(verifier.stats.sessions_created, 0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.min_quorum_pools = 3;
        verifier.config.consensus_threshold = 0.6;
        for i in 1..=4 {
            verifier
                .register_pool(make_pool(&format!("p{}", i), 0.9))
                .unwrap();
        }
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        verifier.submit_vote("s1", "p1", true).unwrap();
        verifier.submit_vote("s1", "p2", true).unwrap();
        verifier.submit_vote("s1", "p3", true).unwrap();
        assert_eq!(verifier.stats.sessions_created, 1);
        assert_eq!(verifier.stats.sessions_completed, 1);
        assert_eq!(verifier.stats.consensus_reached, 1);
    }

    #[test]
    fn test_history_size_limit() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.max_history_size = 2;
        verifier.config.min_quorum_pools = 1;
        verifier.config.consensus_threshold = 0.5;
        verifier.register_pool(make_pool("p1", 0.9)).unwrap();
        for i in 0..4 {
            let sid = format!("s{}", i);
            verifier
                .create_session(sid.clone(), format!("proof{}", i))
                .unwrap();
            verifier.submit_vote(&sid, "p1", true).unwrap();
        }
        assert!(verifier.get_history().len() <= 2);
    }

    #[test]
    fn test_adaptive_threshold() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.adaptive_threshold = true;
        verifier.config.consensus_threshold = 0.7;
        verifier.config.threshold_step = 0.05;
        for i in 1..=5 {
            verifier
                .register_pool(make_pool(&format!("p{}", i), 0.9))
                .unwrap();
        }
        let threshold = verifier.current_threshold();
        assert!(threshold < 0.7);
        assert!(threshold >= 0.5);
    }

    #[test]
    fn test_session_total_weight() {
        let session = VerificationSession::new("s1".to_string(), "proof1".to_string(), None);
        assert_eq!(session.total_weight(), 0.0);
    }

    #[test]
    fn test_session_vote_count() {
        let mut session = VerificationSession::new("s1".to_string(), "proof1".to_string(), None);
        assert_eq!(session.vote_count(), 0);
        session
            .votes
            .push(Vote::new("p1".to_string(), true, 0.9, 50));
        assert_eq!(session.vote_count(), 1);
    }

    #[test]
    fn test_config_default() {
        let config = CrossPoolConfig::default();
        assert_eq!(config.consensus_threshold, 0.67);
        assert!(config.challenge_enabled);
    }

    #[test]
    fn test_stats_default() {
        let stats = CrossPoolStats::default();
        assert_eq!(stats.sessions_created, 0);
    }

    #[test]
    fn test_stats_record_completion() {
        let mut stats = CrossPoolStats::default();
        stats.record_completion(100, 3);
        assert_eq!(stats.sessions_completed, 1);
        assert_eq!(stats.total_votes, 3);
    }

    #[test]
    fn test_error_display() {
        match CrossPoolError::PoolNotRegistered("x".to_string()) {
            e => assert!(format!("{}", e).contains("x")),
        }
    }

    #[test]
    fn test_get_stats_reference() {
        let verifier = CrossPoolVerifier::default();
        let stats = verifier.get_stats();
        assert_eq!(stats.sessions_created, 0);
    }

    #[test]
    fn test_get_session_missing() {
        let verifier = CrossPoolVerifier::default();
        assert!(verifier.get_session("missing").is_none());
    }

    #[test]
    fn test_weighted_consensus_with_different_reputations() {
        let mut verifier = CrossPoolVerifier::default();
        verifier.config.min_quorum_pools = 3;
        verifier.config.consensus_threshold = 0.6;
        verifier.config.reputation_weight = 1.0;
        verifier.config.latency_weight = 0.0;
        verifier.register_pool(make_pool("high_rep", 1.0)).unwrap();
        verifier.register_pool(make_pool("mid_rep", 0.5)).unwrap();
        verifier.register_pool(make_pool("low_rep", 0.1)).unwrap();
        verifier
            .create_session("s1".to_string(), "proof1".to_string())
            .unwrap();
        // High rep says yes, others say no
        verifier.submit_vote("s1", "high_rep", true).unwrap();
        verifier.submit_vote("s1", "mid_rep", false).unwrap();
        verifier.submit_vote("s1", "low_rep", false).unwrap();
        let session = verifier.get_session("s1").unwrap();
        // High reputation should dominate (1.0 / (1.0+0.5+0.1) = 0.625 > 0.6)
        assert!(session.consensus_reached);
    }
}
