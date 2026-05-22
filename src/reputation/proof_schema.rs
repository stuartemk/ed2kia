//! Reputation Proof Schema — Ed25519-based reputation proofs with tier system.
//!
//! Provides structures for creating, serializing, and verifying reputation proofs
//! in the ed2kIA federation. Each proof contains:
//! - Ed25519 signature (hex-encoded)
//! - Timestamp (Unix ms)
//! - Compute hash (SHA-256 of contributed work)
//! - Reputation tier (Novice → Guardian)
//!
//! Supports both JSON and FlatBuffers-compatible serialization.
//!
//! # Feature Flag
//!
//! This module is gated behind `#[cfg(feature = "v1.8-sprint1")]`.

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Reputation Tiers
// ============================================================================

/// Reputation tier — 7 levels from Novice to Guardian.
///
/// Tier progression is based on accumulated reputation score:
/// - Novice: 0 — no contributions yet
/// - Contributor: 100 — first verified contribution
/// - Validator: 500 — consistent verification work
/// - Expert: 2000 — advanced contributions
/// - Maintainer: 5000 — governance participation
/// - Council: 15000 — trusted federation member
/// - Guardian: 50000 — core protocol guardian
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum ReputationTier {
    Novice = 0,
    Contributor = 1,
    Validator = 2,
    Expert = 3,
    Maintainer = 4,
    Council = 5,
    Guardian = 6,
}

impl ReputationTier {
    /// Minimum reputation score required for this tier
    pub fn min_score(&self) -> u64 {
        match self {
            ReputationTier::Novice => 0,
            ReputationTier::Contributor => 100,
            ReputationTier::Validator => 500,
            ReputationTier::Expert => 2000,
            ReputationTier::Maintainer => 5000,
            ReputationTier::Council => 15000,
            ReputationTier::Guardian => 50000,
        }
    }

    /// Weight multiplier for this tier (used in consensus voting)
    pub fn weight(&self) -> f64 {
        match self {
            ReputationTier::Novice => 0.1,
            ReputationTier::Contributor => 0.5,
            ReputationTier::Validator => 1.0,
            ReputationTier::Expert => 2.0,
            ReputationTier::Maintainer => 5.0,
            ReputationTier::Council => 10.0,
            ReputationTier::Guardian => 25.0,
        }
    }

    /// Create tier from reputation score
    pub fn from_score(score: u64) -> Self {
        if score >= 50000 {
            ReputationTier::Guardian
        } else if score >= 15000 {
            ReputationTier::Council
        } else if score >= 5000 {
            ReputationTier::Maintainer
        } else if score >= 2000 {
            ReputationTier::Expert
        } else if score >= 500 {
            ReputationTier::Validator
        } else if score >= 100 {
            ReputationTier::Contributor
        } else {
            ReputationTier::Novice
        }
    }
}

impl fmt::Display for ReputationTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReputationTier::Novice => write!(f, "Novice"),
            ReputationTier::Contributor => write!(f, "Contributor"),
            ReputationTier::Validator => write!(f, "Validator"),
            ReputationTier::Expert => write!(f, "Expert"),
            ReputationTier::Maintainer => write!(f, "Maintainer"),
            ReputationTier::Council => write!(f, "Council"),
            ReputationTier::Guardian => write!(f, "Guardian"),
        }
    }
}

// ============================================================================
// Proof Types
// ============================================================================

/// Error types for reputation proof operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofError {
    /// Invalid Ed25519 signature format
    InvalidSignature,
    /// Proof timestamp expired
    TimestampExpired,
    /// Compute hash mismatch
    HashMismatch,
    /// Invalid tier for operation
    InvalidTier,
    /// Proof already used (replay attack)
    ReplayDetected,
    /// Node not found in ledger
    NodeNotFound,
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofError::InvalidSignature => write!(f, "invalid Ed25519 signature"),
            ProofError::TimestampExpired => write!(f, "proof timestamp expired"),
            ProofError::HashMismatch => write!(f, "compute hash mismatch"),
            ProofError::InvalidTier => write!(f, "invalid tier for operation"),
            ProofError::ReplayDetected => write!(f, "replay attack detected"),
            ProofError::NodeNotFound => write!(f, "node not found in ledger"),
        }
    }
}

impl std::error::Error for ProofError {}

/// Reputation proof — proves a node contributed verified compute work.
///
/// This structure is designed for both JSON serialization (API) and
/// FlatBuffers-compatible layout (binary wire format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationProof {
    /// Proof unique identifier
    pub proof_id: String,
    /// Node ID that created this proof
    pub node_id: String,
    /// Ed25519 signature (hex-encoded, 64 chars)
    pub signature: String,
    /// Ed25519 public key (hex-encoded, 64 chars)
    pub public_key: String,
    /// Timestamp of proof creation (Unix ms)
    pub timestamp_ms: u64,
    /// SHA-256 hash of the compute work contributed (hex, 64 chars)
    pub compute_hash: String,
    /// Reputation tier at time of proof creation
    pub tier: ReputationTier,
    /// Reputation score at time of proof creation
    pub score: u64,
    /// Type of contribution (e.g., "sae_forward", "consensus", "feedback")
    pub contribution_type: String,
    /// Credits earned from this contribution
    pub credits: f64,
    /// Optional metadata (JSON string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

impl ReputationProof {
    /// Create a new reputation proof.
    ///
    /// In production, `signature` would be generated using `ed25519_dalek`.
    /// For the v1.8 baseline, accepts pre-computed signatures.
    pub fn new(
        proof_id: String,
        node_id: String,
        signature: String,
        public_key: String,
        timestamp_ms: u64,
        compute_hash: String,
        tier: ReputationTier,
        score: u64,
        contribution_type: String,
        credits: f64,
    ) -> Self {
        Self {
            proof_id,
            node_id,
            signature,
            public_key,
            timestamp_ms,
            compute_hash,
            tier,
            score,
            contribution_type,
            credits,
            metadata: None,
        }
    }

    /// Add metadata to the proof
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Verify proof format (structural validation).
    ///
    /// Checks:
    /// 1. Signature is valid hex, 64 chars
    /// 2. Public key is valid hex, 64 chars
    /// 3. Compute hash is valid hex, 64 chars
    /// 4. Timestamp is within acceptable range (±10 min from current)
    /// 5. Credits are non-negative
    ///
    /// Returns `Ok(())` if valid, `Err(ProofError)` if invalid.
    pub fn verify_format(&self, current_ms: u64) -> Result<(), ProofError> {
        // Validate signature format
        if self.signature.len() != 64 || !self.signature.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ProofError::InvalidSignature);
        }

        // Validate public key format
        if self.public_key.len() != 64 || !self.public_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ProofError::InvalidSignature);
        }

        // Validate compute hash format
        if self.compute_hash.len() != 64
            || !self.compute_hash.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ProofError::HashMismatch);
        }

        // Validate timestamp (±10 minutes)
        let diff = current_ms.abs_diff(self.timestamp_ms);
        if diff > 600_000 {
            return Err(ProofError::TimestampExpired);
        }

        // Validate credits
        if self.credits < 0.0 {
            return Err(ProofError::InvalidTier);
        }

        Ok(())
    }

    /// Compute a simple deterministic hash of the proof content for deduplication.
    ///
    /// In production, use SHA-256. For baseline, uses a simple string-based hash.
    pub fn content_hash(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.proof_id, self.node_id, self.timestamp_ms, self.compute_hash, self.credits
        )
        .chars()
        .fold(0u64, |acc, c| {
            acc.wrapping_add(c as u64).wrapping_mul(31u64)
        })
        .to_string()
    }
}

// ============================================================================
// Proof Batch — For bulk verification
// ============================================================================

/// Batch of reputation proofs for bulk processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofBatch {
    /// Batch unique identifier
    pub batch_id: String,
    /// Proofs in this batch
    pub proofs: Vec<ReputationProof>,
    /// Batch creation timestamp (Unix ms)
    pub created_at_ms: u64,
    /// Total credits in batch
    pub total_credits: f64,
}

impl ProofBatch {
    /// Create a new proof batch
    pub fn new(batch_id: String, proofs: Vec<ReputationProof>, created_at_ms: u64) -> Self {
        let total_credits = proofs.iter().map(|p| p.credits).sum();
        Self {
            batch_id,
            proofs,
            created_at_ms,
            total_credits,
        }
    }

    /// Validate all proofs in the batch
    pub fn validate_all(&self, current_ms: u64) -> Vec<(String, ProofError)> {
        self.proofs
            .iter()
            .filter_map(|p| {
                p.verify_format(current_ms)
                    .err()
                    .map(|e| (p.proof_id.clone(), e))
            })
            .collect()
    }

    /// Get proofs by tier
    pub fn by_tier(&self, tier: ReputationTier) -> Vec<&ReputationProof> {
        self.proofs.iter().filter(|p| p.tier == tier).collect()
    }

    /// Get unique node IDs in this batch
    pub fn unique_nodes(&self) -> Vec<&str> {
        let mut nodes: Vec<&str> = self.proofs.iter().map(|p| p.node_id.as_str()).collect();
        nodes.sort();
        nodes.dedup();
        nodes
    }
}

// ============================================================================
// Batch Verification & Anti-Sybil
// ============================================================================

/// Result of batch verification
#[derive(Debug, Clone)]
pub struct BatchVerifyResult {
    /// Total proofs processed
    pub total: usize,
    /// Proofs that passed verification
    pub passed: usize,
    /// Proofs that failed verification
    pub failed: usize,
    /// Total credits from valid proofs
    pub valid_credits: f64,
    /// List of (proof_id, error) for failed proofs
    pub errors: Vec<(String, ProofError)>,
}

/// Verify a batch of reputation proofs.
///
/// Runs format validation on each proof and aggregates results.
pub fn verify_batch(proofs: &[ReputationProof], current_ms: u64) -> BatchVerifyResult {
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut valid_credits = 0.0f64;
    let mut errors = Vec::new();

    for proof in proofs {
        match proof.verify_format(current_ms) {
            Ok(()) => {
                passed += 1;
                valid_credits += proof.credits;
            }
            Err(e) => {
                failed += 1;
                errors.push((proof.proof_id.clone(), e));
            }
        }
    }

    BatchVerifyResult {
        total: proofs.len(),
        passed,
        failed,
        valid_credits,
        errors,
    }
}

/// Anti-Sybil rate limiter — tracks proof submissions by public key.
#[derive(Debug, Clone)]
pub struct AntiSybilLimiter {
    /// Maximum proofs per public key within the window
    max_proofs_per_window: usize,
    /// Window size in milliseconds
    window_ms: u64,
}

impl AntiSybilLimiter {
    /// Create a new anti-Sybil limiter.
    pub fn new(max_proofs_per_window: usize, window_ms: u64) -> Self {
        Self {
            max_proofs_per_window,
            window_ms,
        }
    }

    /// Check if a proof is allowed under rate limiting.
    ///
    /// Returns `true` if the proof is allowed, `false` if rate limited.
    pub fn is_allowed(
        &self,
        public_key: &str,
        timestamp_ms: u64,
        recent_submissions: &[(String, u64)],
    ) -> bool {
        let window_start = timestamp_ms.saturating_sub(self.window_ms);

        let count = recent_submissions
            .iter()
            .filter(|(pk, ts)| pk == public_key && *ts >= window_start && *ts <= timestamp_ms)
            .count();

        count < self.max_proofs_per_window
    }
}

// ============================================================================
// FlatBuffers-Compatible Serialization Helpers
// ============================================================================

/// Serialize a ReputationProof to JSON bytes.
///
/// For FlatBuffers, use the `flatbuffers` crate with generated schema.
/// This JSON serializer provides a compatible baseline.
pub fn serialize_proof(proof: &ReputationProof) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(proof)
}

/// Deserialize a ReputationProof from JSON bytes.
pub fn deserialize_proof(data: &[u8]) -> Result<ReputationProof, serde_json::Error> {
    serde_json::from_slice(data)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof() -> ReputationProof {
        ReputationProof::new(
            "proof_001".to_string(),
            "node_001".to_string(),
            "a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2a1b2".to_string(),
            "b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3b2c3".to_string(),
            1_700_000_000_000,
            "c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4c3d4".to_string(),
            ReputationTier::Validator,
            750,
            "sae_forward".to_string(),
            10.0,
        )
    }

    // --- Tier Tests ---

    #[test]
    fn test_tier_min_score() {
        assert_eq!(ReputationTier::Novice.min_score(), 0);
        assert_eq!(ReputationTier::Contributor.min_score(), 100);
        assert_eq!(ReputationTier::Guardian.min_score(), 50000);
    }

    #[test]
    fn test_tier_weight() {
        assert_eq!(ReputationTier::Novice.weight(), 0.1);
        assert_eq!(ReputationTier::Validator.weight(), 1.0);
        assert_eq!(ReputationTier::Guardian.weight(), 25.0);
    }

    #[test]
    fn test_tier_from_score() {
        assert_eq!(ReputationTier::from_score(0), ReputationTier::Novice);
        assert_eq!(ReputationTier::from_score(100), ReputationTier::Contributor);
        assert_eq!(ReputationTier::from_score(500), ReputationTier::Validator);
        assert_eq!(ReputationTier::from_score(2000), ReputationTier::Expert);
        assert_eq!(ReputationTier::from_score(5000), ReputationTier::Maintainer);
        assert_eq!(ReputationTier::from_score(15000), ReputationTier::Council);
        assert_eq!(ReputationTier::from_score(50000), ReputationTier::Guardian);
        assert_eq!(ReputationTier::from_score(99999), ReputationTier::Guardian);
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(ReputationTier::Novice.to_string(), "Novice");
        assert_eq!(ReputationTier::Guardian.to_string(), "Guardian");
    }

    #[test]
    fn test_tier_ordering() {
        assert!(ReputationTier::Guardian > ReputationTier::Novice);
        assert!(ReputationTier::Validator > ReputationTier::Contributor);
    }

    // --- Proof Tests ---

    #[test]
    fn test_proof_creation() {
        let proof = make_proof();
        assert_eq!(proof.proof_id, "proof_001");
        assert_eq!(proof.node_id, "node_001");
        assert_eq!(proof.tier, ReputationTier::Validator);
        assert_eq!(proof.credits, 10.0);
        assert!(proof.metadata.is_none());
    }

    #[test]
    fn test_proof_with_metadata() {
        let proof = make_proof().with_metadata("{\"layer\":15}".to_string());
        assert!(proof.metadata.is_some());
    }

    #[test]
    fn test_proof_verify_format_valid() {
        let proof = make_proof();
        let result = proof.verify_format(1_700_000_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_proof_verify_format_invalid_signature() {
        let mut proof = make_proof();
        proof.signature = "invalid".to_string();
        let result = proof.verify_format(1_700_000_000_000);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProofError::InvalidSignature));
    }

    #[test]
    fn test_proof_verify_format_timestamp_expired() {
        let proof = make_proof();
        // 20 minutes ago
        let result = proof.verify_format(1_700_001_200_000);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProofError::TimestampExpired));
    }

    #[test]
    fn test_proof_verify_format_negative_credits() {
        let mut proof = make_proof();
        proof.credits = -1.0;
        let result = proof.verify_format(1_700_000_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_content_hash_deterministic() {
        let proof1 = make_proof();
        let proof2 = make_proof();
        assert_eq!(proof1.content_hash(), proof2.content_hash());
    }

    #[test]
    fn test_proof_content_hash_different() {
        let mut proof1 = make_proof();
        let mut proof2 = make_proof();
        proof1.credits = 10.0;
        proof2.credits = 20.0;
        assert_ne!(proof1.content_hash(), proof2.content_hash());
    }

    // --- Serialization Tests ---

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let proof = make_proof();
        let data = serialize_proof(&proof).unwrap();
        let deserialized = deserialize_proof(&data).unwrap();
        assert_eq!(deserialized.proof_id, proof.proof_id);
        assert_eq!(deserialized.node_id, proof.node_id);
        assert_eq!(deserialized.tier, proof.tier);
        assert_eq!(deserialized.credits, proof.credits);
    }

    #[test]
    fn test_serialize_json_valid() {
        let proof = make_proof();
        let data = serialize_proof(&proof).unwrap();
        let json = String::from_utf8(data).unwrap();
        assert!(json.contains("\"proof_id\""));
        assert!(json.contains("\"node_id\""));
        assert!(json.contains("\"signature\""));
        assert!(json.contains("\"tier\""));
    }

    #[test]
    fn test_deserialize_invalid_json() {
        let result = deserialize_proof(b"not json");
        assert!(result.is_err());
    }

    // --- Batch Tests ---

    #[test]
    fn test_batch_creation() {
        let proofs = vec![make_proof()];
        let batch = ProofBatch::new("batch_001".to_string(), proofs, 1_700_000_000_000);
        assert_eq!(batch.batch_id, "batch_001");
        assert_eq!(batch.total_credits, 10.0);
    }

    #[test]
    fn test_batch_total_credits() {
        let mut proof1 = make_proof();
        let mut proof2 = make_proof();
        proof1.proof_id = "p1".to_string();
        proof2.proof_id = "p2".to_string();
        proof1.credits = 10.0;
        proof2.credits = 20.0;
        let batch = ProofBatch::new(
            "batch_001".to_string(),
            vec![proof1, proof2],
            1_700_000_000_000,
        );
        assert_eq!(batch.total_credits, 30.0);
    }

    #[test]
    fn test_batch_validate_all() {
        let proofs = vec![make_proof()];
        let batch = ProofBatch::new("batch_001".to_string(), proofs, 1_700_000_000_000);
        let errors = batch.validate_all(1_700_000_000_000);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_batch_by_tier() {
        let mut proof1 = make_proof();
        let mut proof2 = make_proof();
        proof1.tier = ReputationTier::Validator;
        proof2.tier = ReputationTier::Expert;
        let batch = ProofBatch::new(
            "batch_001".to_string(),
            vec![proof1, proof2],
            1_700_000_000_000,
        );
        let validators = batch.by_tier(ReputationTier::Validator);
        assert_eq!(validators.len(), 1);
    }

    #[test]
    fn test_batch_unique_nodes() {
        let mut proof1 = make_proof();
        let mut proof2 = make_proof();
        proof1.node_id = "node_001".to_string();
        proof2.node_id = "node_001".to_string(); // Same node
        let batch = ProofBatch::new(
            "batch_001".to_string(),
            vec![proof1, proof2],
            1_700_000_000_000,
        );
        let nodes = batch.unique_nodes();
        assert_eq!(nodes.len(), 1);
    }

    // --- Error Tests ---

    #[test]
    fn test_error_display() {
        assert_eq!(
            ProofError::InvalidSignature.to_string(),
            "invalid Ed25519 signature"
        );
        assert_eq!(
            ProofError::TimestampExpired.to_string(),
            "proof timestamp expired"
        );
        assert_eq!(
            ProofError::ReplayDetected.to_string(),
            "replay attack detected"
        );
    }

    #[test]
    fn test_proof_error_equality() {
        assert_eq!(ProofError::InvalidSignature, ProofError::InvalidSignature);
        assert_ne!(ProofError::InvalidSignature, ProofError::TimestampExpired);
    }

    // --- Batch Verification Tests ---

    #[test]
    fn test_verify_batch_all_valid() {
        let proofs = vec![make_proof()];
        let result = verify_batch(&proofs, 1_700_000_000_000);
        assert_eq!(result.total, 1);
        assert_eq!(result.passed, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.valid_credits, 10.0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_verify_batch_mixed() {
        let mut proof1 = make_proof();
        let mut proof2 = make_proof();
        proof1.proof_id = "p1".to_string();
        proof2.proof_id = "p2".to_string();
        proof2.signature = "invalid".to_string(); // Will fail
        let result = verify_batch(&[proof1, proof2], 1_700_000_000_000);
        assert_eq!(result.total, 2);
        assert_eq!(result.passed, 1);
        assert_eq!(result.failed, 1);
        assert_eq!(result.valid_credits, 10.0);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_verify_batch_empty() {
        let result = verify_batch(&[], 1_700_000_000_000);
        assert_eq!(result.total, 0);
        assert_eq!(result.passed, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.valid_credits, 0.0);
    }

    // --- Anti-Sybil Tests ---

    #[test]
    fn test_antisybil_allowed() {
        let limiter = AntiSybilLimiter::new(5, 60_000); // 5 proofs per 60s
        let pk = "abc123";
        let submissions: Vec<(String, u64)> =
            vec![(pk.to_string(), 1_00_000), (pk.to_string(), 1_10_000)];
        assert!(limiter.is_allowed(pk, 1_20_000, &submissions));
    }

    #[test]
    fn test_antisybil_rate_limited() {
        let limiter = AntiSybilLimiter::new(3, 60_000); // 3 proofs per 60s
        let pk = "abc123";
        let submissions: Vec<(String, u64)> = vec![
            (pk.to_string(), 1_00_000),
            (pk.to_string(), 1_10_000),
            (pk.to_string(), 1_20_000),
        ];
        assert!(!limiter.is_allowed(pk, 1_30_000, &submissions));
    }

    #[test]
    fn test_antisybil_window_expires() {
        let limiter = AntiSybilLimiter::new(2, 60_000); // 2 proofs per 60s
        let pk = "abc123";
        let submissions: Vec<(String, u64)> =
            vec![(pk.to_string(), 1_00_000), (pk.to_string(), 1_10_000)];
        // After window expires, should be allowed again
        assert!(limiter.is_allowed(pk, 1_80_000, &submissions));
    }

    #[test]
    fn test_antisybil_different_key() {
        let limiter = AntiSybilLimiter::new(1, 60_000);
        let submissions: Vec<(String, u64)> = vec![("key1".to_string(), 1_000_000)];
        // Different key should not be limited
        assert!(limiter.is_allowed("key2", 1_10_000, &submissions));
    }
}
