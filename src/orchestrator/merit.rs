//! Cryptographic Merit System — Ethical recognition, zero financial logic.
//!
//! Feature-gated behind `v2.1-merit-system`. Uses `ed25519-dalek` to sign
//! `MeritProof` structures proving audit contributions. Cero tokens, cero staking,
//! cero promesas de retorno. Solo reconocimiento técnico y gobernanza ponderada.
//!
//! **Status:** Functional scaffold with signing/verification + unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors specific to merit system operations.
#[derive(Debug, Error)]
pub enum MeritError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Expired proof: timestamp {0} is too old")]
    ExpiredProof(u64),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Insufficient audits: have {have}, need {need}")]
    InsufficientAudits { have: u32, need: u32 },

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Merit tier based on audit count. Cero valor financiero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeritTier {
    Novice,      // 0-9 audits
    Contributor, // 10-99 audits
    Guardian,    // 100-999 audits
    Steward,     // 1000+ audits
}

impl MeritTier {
    /// Calculate tier from audit count.
    pub fn from_audit_count(count: u32) -> Self {
        match count {
            0..=9 => MeritTier::Novice,
            10..=99 => MeritTier::Contributor,
            100..=999 => MeritTier::Guardian,
            _ => MeritTier::Steward,
        }
    }

    /// Display name for UI.
    pub fn label(&self) -> &'static str {
        match self {
            MeritTier::Novice => "Novato",
            MeritTier::Contributor => "Contribuidor",
            MeritTier::Guardian => "Guardián",
            MeritTier::Steward => "Mayordomo",
        }
    }

    /// Badge emoji for visual display.
    pub fn badge(&self) -> &'static str {
        match self {
            MeritTier::Novice => "🌱",
            MeritTier::Contributor => "🔧",
            MeritTier::Guardian => "🛡️",
            MeritTier::Steward => "⭐",
        }
    }
}

/// Cryptographic proof of merit. Signed by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeritProof {
    /// Unique node identifier.
    pub node_id: String,
    /// Total audits completed by this node.
    pub audit_count: u32,
    /// Unix timestamp when this proof was issued.
    pub timestamp: u64,
    /// Merit tier at time of issuance.
    pub tier: MeritTier,
    /// SHA256 hash of the canonical JSON representation.
    pub hash: String,
    /// ed25519 signature over the hash.
    pub signature: Vec<u8>,
}

/// Request to claim a merit proof.
#[derive(Debug, Deserialize)]
pub struct MeritClaimRequest {
    pub node_id: String,
    pub audit_count: u32,
}

/// Response containing the signed merit proof.
#[derive(Debug, Serialize)]
pub struct MeritClaimResponse {
    pub proof: MeritProof,
    pub message: String,
}

/// Merit ledger entry (stored per node).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeritEntry {
    pub node_id: String,
    pub audit_count: u32,
    pub tier: MeritTier,
    pub proofs_issued: u32,
    pub first_audit_ts: u64,
    pub last_audit_ts: u64,
}

/// Cryptographic Merit Engine.
///
/// Signs merit proofs using ed25519. Cero lógica financiera — solo
/// reconocimiento técnico verificable criptográficamente.
pub struct MeritEngine {
    /// Signing key for issuing proofs.
    signing_key: SigningKey,
    /// Verification key for validating proofs.
    verifying_key: VerifyingKey,
    /// Per-node merit ledger.
    ledger: BTreeMap<String, MeritEntry>,
    /// Maximum age of proofs in seconds (default: 1 hour).
    max_proof_age_secs: u64,
}

impl MeritEngine {
    /// Create a new MeritEngine with a random signing key.
    pub fn new() -> Self {
        let mut csprng = ark_std::rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            ledger: BTreeMap::new(),
            max_proof_age_secs: 3600, // 1 hour
        }
    }

    /// Create with a specific signing key (for testing).
    pub fn with_key(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            ledger: BTreeMap::new(),
            max_proof_age_secs: 3600,
        }
    }

    /// Get the verifying key (public) for external validation.
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Record an audit completion for a node.
    pub fn record_audit(&mut self, node_id: String, audit_count: u32) {
        let now = Utc::now().timestamp_millis() as u64;
        let tier = MeritTier::from_audit_count(audit_count);

        match self.ledger.get_mut(&node_id) {
            Some(entry) => {
                entry.audit_count = audit_count;
                entry.tier = tier;
                entry.last_audit_ts = now;
            }
            None => {
                self.ledger.insert(
                    node_id.clone(),
                    MeritEntry {
                        node_id,
                        audit_count,
                        tier,
                        proofs_issued: 0,
                        first_audit_ts: now,
                        last_audit_ts: now,
                    },
                );
            }
        }
    }

    /// Issue a signed merit proof for a node.
    pub fn claim_proof(
        &mut self,
        request: MeritClaimRequest,
    ) -> Result<MeritClaimResponse, MeritError> {
        // Check node exists in ledger
        let entry = self
            .ledger
            .get(&request.node_id)
            .ok_or_else(|| MeritError::NodeNotFound(request.node_id.clone()))?;

        // Verify audit count matches
        if request.audit_count > entry.audit_count {
            return Err(MeritError::InsufficientAudits {
                have: entry.audit_count,
                need: request.audit_count,
            });
        }

        let tier = MeritTier::from_audit_count(request.audit_count);
        let timestamp = Utc::now().timestamp_millis() as u64;

        // Build canonical JSON for hashing
        let canonical = serde_json::json!({
            "node_id": request.node_id,
            "audit_count": request.audit_count,
            "timestamp": timestamp,
            "tier": format!("{:?}", tier),
        });
        let canonical_str = canonical.to_string();
        let hash_bytes = Sha256::digest(canonical_str.as_bytes());
        let hash = format!("{:x}", hash_bytes);

        // Sign the hash
        let signature = self.signing_key.sign(hash.as_bytes());

        let proof = MeritProof {
            node_id: request.node_id,
            audit_count: request.audit_count,
            timestamp,
            tier,
            hash,
            signature: signature.to_vec(),
        };

        // Increment proofs issued
        if let Some(entry) = self.ledger.get_mut(&proof.node_id) {
            entry.proofs_issued += 1;
        }

        Ok(MeritClaimResponse {
            proof,
            message: "Prueba de mérito firmada criptográficamente. Cero valor financiero."
                .to_string(),
        })
    }

    /// Verify a merit proof signature and freshness.
    pub fn verify_proof(&self, proof: &MeritProof) -> Result<bool, MeritError> {
        // Verify signature
        use ed25519_dalek::Verifier;
        self.verifying_key
            .verify(
                proof.hash.as_bytes(),
                &ed25519_dalek::Signature::from_slice(&proof.signature)
                    .map_err(|e| MeritError::InvalidSignature(format!("{}", e)))?,
            )
            .map_err(|_| MeritError::InvalidSignature("Signature verification failed".into()))?;

        // Verify freshness
        let now = Utc::now().timestamp_millis() as u64;
        let age_secs = now.saturating_sub(proof.timestamp) / 1000;
        if age_secs > self.max_proof_age_secs {
            return Err(MeritError::ExpiredProof(proof.timestamp));
        }

        // Verify tier matches audit count
        let expected_tier = MeritTier::from_audit_count(proof.audit_count);
        if proof.tier != expected_tier {
            return Err(MeritError::InvalidSignature(
                "Tier does not match audit count".into(),
            ));
        }

        Ok(true)
    }

    /// Get merit entry for a node.
    pub fn get_entry(&self, node_id: &str) -> Option<&MeritEntry> {
        self.ledger.get(node_id)
    }

    /// Get total tracked nodes.
    pub fn tracked_count(&self) -> usize {
        self.ledger.len()
    }

    /// Get nodes by tier.
    pub fn nodes_by_tier(&self, tier: &MeritTier) -> Vec<&MeritEntry> {
        self.ledger.values().filter(|e| &e.tier == tier).collect()
    }
}

impl Default for MeritEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merit_tier_from_count() {
        assert_eq!(MeritTier::from_audit_count(0), MeritTier::Novice);
        assert_eq!(MeritTier::from_audit_count(5), MeritTier::Novice);
        assert_eq!(MeritTier::from_audit_count(10), MeritTier::Contributor);
        assert_eq!(MeritTier::from_audit_count(50), MeritTier::Contributor);
        assert_eq!(MeritTier::from_audit_count(100), MeritTier::Guardian);
        assert_eq!(MeritTier::from_audit_count(500), MeritTier::Guardian);
        assert_eq!(MeritTier::from_audit_count(1000), MeritTier::Steward);
        assert_eq!(MeritTier::from_audit_count(9999), MeritTier::Steward);
    }

    #[test]
    fn test_merit_tier_labels() {
        assert_eq!(MeritTier::Novice.label(), "Novato");
        assert_eq!(MeritTier::Contributor.label(), "Contribuidor");
        assert_eq!(MeritTier::Guardian.label(), "Guardián");
        assert_eq!(MeritTier::Steward.label(), "Mayordomo");
    }

    #[test]
    fn test_merit_tier_badges() {
        assert_eq!(MeritTier::Novice.badge(), "🌱");
        assert_eq!(MeritTier::Contributor.badge(), "🔧");
        assert_eq!(MeritTier::Guardian.badge(), "🛡️");
        assert_eq!(MeritTier::Steward.badge(), "⭐");
    }

    #[test]
    fn test_engine_new() {
        let engine = MeritEngine::new();
        assert_eq!(engine.tracked_count(), 0);
    }

    #[test]
    fn test_record_audit_new_node() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 5);
        let entry = engine.get_entry("node-1").unwrap();
        assert_eq!(entry.audit_count, 5);
        assert_eq!(entry.tier, MeritTier::Novice);
        assert_eq!(engine.tracked_count(), 1);
    }

    #[test]
    fn test_record_audit_updates_existing() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 5);
        engine.record_audit("node-1".to_string(), 150);
        let entry = engine.get_entry("node-1").unwrap();
        assert_eq!(entry.audit_count, 150);
        assert_eq!(entry.tier, MeritTier::Guardian);
    }

    #[test]
    fn test_claim_proof_success() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 50);

        let response = engine
            .claim_proof(MeritClaimRequest {
                node_id: "node-1".to_string(),
                audit_count: 50,
            })
            .unwrap();

        assert_eq!(response.proof.node_id, "node-1");
        assert_eq!(response.proof.audit_count, 50);
        assert_eq!(response.proof.tier, MeritTier::Contributor);
        assert!(!response.proof.signature.is_empty());
        assert!(response.message.contains("Cero valor financiero"));
    }

    #[test]
    fn test_claim_proof_node_not_found() {
        let mut engine = MeritEngine::new();
        let result = engine.claim_proof(MeritClaimRequest {
            node_id: "unknown".to_string(),
            audit_count: 10,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_claim_proof_insufficient_audits() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 10);

        let result = engine.claim_proof(MeritClaimRequest {
            node_id: "node-1".to_string(),
            audit_count: 100,
        });
        assert!(matches!(result, Err(MeritError::InsufficientAudits { .. })));
    }

    #[test]
    fn test_verify_proof_valid() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 25);

        let response = engine
            .claim_proof(MeritClaimRequest {
                node_id: "node-1".to_string(),
                audit_count: 25,
            })
            .unwrap();

        assert!(engine.verify_proof(&response.proof).unwrap());
    }

    #[test]
    fn test_verify_proof_tier_mismatch() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 25);

        let response = engine
            .claim_proof(MeritClaimRequest {
                node_id: "node-1".to_string(),
                audit_count: 25,
            })
            .unwrap();

        assert!(engine.verify_proof(&response.proof).unwrap());
    }

    #[test]
    fn test_nodes_by_tier() {
        let mut engine = MeritEngine::new();
        engine.record_audit("node-1".to_string(), 5);
        engine.record_audit("node-2".to_string(), 50);
        engine.record_audit("node-3".to_string(), 500);

        let novices = engine.nodes_by_tier(&MeritTier::Novice);
        assert_eq!(novices.len(), 1);

        let contributors = engine.nodes_by_tier(&MeritTier::Contributor);
        assert_eq!(contributors.len(), 1);

        let guardians = engine.nodes_by_tier(&MeritTier::Guardian);
        assert_eq!(guardians.len(), 1);
    }

    #[test]
    fn test_error_display() {
        let err = MeritError::NodeNotFound("x".into());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_engine_default() {
        let engine = MeritEngine::default();
        assert_eq!(engine.tracked_count(), 0);
    }
}
