//! Cross-Chain Identity — Identidad unificada de nodo a través de múltiples cadenas
//!
//! Proporciona identidad criptográfica unificada para nodos ed2kIA que participan
//! en múltiples redes blockchain, con derivación de claves y verificación de firmas.

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};

// ============================================================================
// Error Types
// ============================================================================

/// Error types for cross-chain identity operations
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityError {
    /// Invalid signature
    InvalidSignature,
    /// Chain key not derived
    ChainKeyNotFound(String),
    /// Key derivation failed
    KeyDerivationFailed(String),
    /// Invalid node ID
    InvalidNodeId,
    /// Signature verification failed
    VerificationFailed(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::InvalidSignature => write!(f, "Invalid signature"),
            IdentityError::ChainKeyNotFound(chain_id) => {
                write!(f, "Chain key not found for '{}'", chain_id)
            }
            IdentityError::KeyDerivationFailed(msg) => {
                write!(f, "Key derivation failed: {}", msg)
            }
            IdentityError::InvalidNodeId => write!(f, "Invalid node ID"),
            IdentityError::VerificationFailed(msg) => {
                write!(f, "Verification failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for IdentityError {}

// ============================================================================
// Chain Key Pair
// ============================================================================

/// Per-chain key pair derived from primary identity
#[derive(Debug, Clone)]
pub struct ChainKeyPair {
    /// Chain this key pair belongs to
    pub chain_id: String,
    /// Public key for this chain
    pub public_key: String,
    /// Whether this key was derived from the primary key
    pub derived_from_primary: bool,
}

impl ChainKeyPair {
    /// Create a new chain key pair
    pub fn new(chain_id: String, public_key: String, derived_from_primary: bool) -> Self {
        Self {
            chain_id,
            public_key,
            derived_from_primary,
        }
    }
}

// ============================================================================
// Cross-Chain Reputation
// ============================================================================

/// Reputation aggregated across all chains
#[derive(Debug, Clone)]
pub struct CrossChainReputation {
    /// Global reputation score (0.0 - 1.0)
    pub global_score: f32,
    /// Per-chain reputation scores
    pub chain_scores: HashMap<String, f32>,
    /// Total contributions across all chains
    pub total_contributions: usize,
    /// Last reputation update timestamp
    pub last_updated: Instant,
}

impl CrossChainReputation {
    /// Create a new reputation record
    pub fn new() -> Self {
        Self {
            global_score: 0.5,
            chain_scores: HashMap::new(),
            total_contributions: 0,
            last_updated: Instant::now(),
        }
    }

    /// Update reputation for a specific chain
    pub fn update_chain_score(&mut self, chain_id: &str, delta: f32) {
        let score = self.chain_scores.entry(chain_id.to_string()).or_insert(0.5);
        *score = (*score + delta).clamp(0.0, 1.0);
        self.recalc_global();
        self.last_updated = Instant::now();
    }

    /// Record a contribution
    pub fn record_contribution(&mut self, chain_id: &str) {
        self.total_contributions += 1;
        self.update_chain_score(chain_id, 0.01);
    }

    /// Recalculate global score from chain scores
    fn recalc_global(&mut self) {
        if self.chain_scores.is_empty() {
            self.global_score = 0.5;
        } else {
            let sum: f32 = self.chain_scores.values().sum();
            self.global_score = sum / self.chain_scores.len() as f32;
        }
    }

    /// Get reputation for a specific chain
    pub fn get_chain_score(&self, chain_id: &str) -> f32 {
        *self.chain_scores.get(chain_id).unwrap_or(&0.5)
    }
}

impl Default for CrossChainReputation {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Identity Proof
// ============================================================================

/// Cryptographic proof of identity on a specific chain
#[derive(Debug, Clone)]
pub struct IdentityProof {
    /// Node ID being proven
    pub node_id: String,
    /// Chain this proof is for
    pub chain_id: String,
    /// Signature over the proof data
    pub signature: String,
    /// Proof creation timestamp
    pub timestamp: u64,
    /// Public key used for verification
    pub public_key: String,
}

impl IdentityProof {
    /// Create a new identity proof
    pub fn new(node_id: String, chain_id: String, signature: String, public_key: String) -> Self {
        Self {
            node_id,
            chain_id,
            signature,
            timestamp: current_timestamp_ms(),
            public_key,
        }
    }

    /// Check if the proof has expired
    pub fn is_expired(&self, max_age_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.timestamp) >= max_age_ms
    }
}

// ============================================================================
// Cross-Chain Identity
// ============================================================================

/// Unified node identity across multiple blockchain chains
pub struct CrossChainIdentity {
    /// Unique node identifier
    pub node_id: String,
    /// Primary Ed25519 verifying key
    pub primary_key: VerifyingKey,
    /// Signing key (kept private)
    signing_key: SigningKey,
    /// Per-chain derived key pairs
    pub chain_keys: HashMap<String, ChainKeyPair>,
    /// Cross-chain reputation
    pub reputation: CrossChainReputation,
}

impl CrossChainIdentity {
    /// Create a new cross-chain identity
    pub fn new(node_id: String, signing_key: SigningKey) -> Self {
        let primary_key = signing_key.verifying_key();
        Self {
            node_id,
            primary_key,
            signing_key,
            chain_keys: HashMap::new(),
            reputation: CrossChainReputation::new(),
        }
    }

    /// Derive a chain-specific key from the primary key
    pub fn derive_chain_key(&self, chain_id: &str) -> Result<String, IdentityError> {
        // Check if already derived
        if let Some(kp) = self.chain_keys.get(chain_id) {
            return Ok(kp.public_key.clone());
        }

        // Derive key using HMAC-like construction with node_id + chain_id
        let message = format!("{}:{}", self.node_id, chain_id);
        let signature = self.signing_key.sign(message.as_bytes());
        let derived_key = hex::encode(signature.to_bytes());

        Ok(derived_key)
    }

    /// Register a derived chain key
    pub fn register_chain_key(&mut self, chain_id: &str, public_key: String) {
        let key_pair = ChainKeyPair::new(
            chain_id.to_string(),
            public_key,
            true,
        );
        self.chain_keys.insert(chain_id.to_string(), key_pair);
    }

    /// Verify a signature for a specific chain
    pub fn verify_signature(&self, chain_id: &str, message: &[u8], signature_bytes: &[u8]) -> bool {
        // Try to find chain-specific key first
        if let Some(_kp) = self.chain_keys.get(chain_id) {
            // For derived keys, we verify against the primary key
            // since derived keys are signatures of the primary
            let sig = match Signature::from_slice(signature_bytes) {
                Ok(s) => s,
                Err(_) => return false,
            };
            return self.primary_key.verify_strict(message, &sig).is_ok();
        }

        // Fall back to primary key verification
        let sig = match Signature::from_slice(signature_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        self.primary_key.verify_strict(message, &sig).is_ok()
    }

    /// Sign a message with the primary key
    pub fn sign_message(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Update reputation for a specific chain
    pub fn update_reputation(&mut self, chain_id: &str, delta: f32) {
        self.reputation.update_chain_score(chain_id, delta);
    }

    /// Get global reputation score
    pub fn get_global_reputation(&self) -> f32 {
        self.reputation.global_score
    }

    /// Generate an identity proof for a specific chain
    pub fn generate_proof(&self, chain_id: &str) -> Result<IdentityProof, IdentityError> {
        // Derive or get chain key
        let _chain_key = self.derive_chain_key(chain_id)?;

        // Create proof message
        let proof_message = format!("{}:{}:{}", self.node_id, chain_id, current_timestamp_ms());
        let signature = self.signing_key.sign(proof_message.as_bytes());

        // Create proof
        Ok(IdentityProof::new(
            self.node_id.clone(),
            chain_id.to_string(),
            hex::encode(signature.to_bytes()),
            hex::encode(self.primary_key.to_bytes()),
        ))
    }

    /// Get the primary public key as hex string
    pub fn primary_public_key_hex(&self) -> String {
        hex::encode(self.primary_key.to_bytes())
    }

    /// Get the number of registered chains
    pub fn chain_count(&self) -> usize {
        self.chain_keys.len()
    }

    /// Check if a chain key is registered
    pub fn has_chain_key(&self, chain_id: &str) -> bool {
        self.chain_keys.contains_key(chain_id)
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_identity(node_id: &str) -> CrossChainIdentity {
        let seed = [0u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        CrossChainIdentity::new(node_id.to_string(), signing_key)
    }

    #[test]
    fn test_identity_creation() {
        let identity = make_identity("node-1");
        assert_eq!(identity.node_id, "node-1");
        assert_eq!(identity.chain_count(), 0);
    }

    #[test]
    fn test_derive_chain_key() {
        let identity = make_identity("node-1");
        let key = identity.derive_chain_key("eth-mainnet");
        assert!(key.is_ok());
        assert!(!key.unwrap().is_empty());
    }

    #[test]
    fn test_derive_same_chain_key_twice() {
        let identity = make_identity("node-1");
        let key1 = identity.derive_chain_key("eth-mainnet").unwrap();
        let key2 = identity.derive_chain_key("eth-mainnet").unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_different_chain_keys() {
        let identity = make_identity("node-1");
        let eth_key = identity.derive_chain_key("eth-mainnet").unwrap();
        let sol_key = identity.derive_chain_key("solana-mainnet").unwrap();
        assert_ne!(eth_key, sol_key);
    }

    #[test]
    fn test_sign_and_verify() {
        let identity = make_identity("node-1");
        let message = b"test message";
        let signature = identity.sign_message(message);
        assert!(identity.verify_signature("eth-mainnet", message, &signature.to_bytes()));
    }

    #[test]
    fn test_verify_invalid_signature() {
        let identity = make_identity("node-1");
        let bad_sig = [0u8; 64];
        assert!(!identity.verify_signature("eth-mainnet", b"message", &bad_sig));
    }

    #[test]
    fn test_update_reputation() {
        let mut identity = make_identity("node-1");
        let initial = identity.get_global_reputation();
        identity.update_reputation("eth-mainnet", 0.1);
        assert!(identity.get_global_reputation() > initial);
    }

    #[test]
    fn test_reputation_clamped_to_one() {
        let mut identity = make_identity("node-1");
        identity.update_reputation("eth-mainnet", 2.0);
        assert!(identity.get_global_reputation() <= 1.0);
    }

    #[test]
    fn test_reputation_clamped_to_zero() {
        let mut identity = make_identity("node-1");
        identity.update_reputation("eth-mainnet", -2.0);
        assert!(identity.get_global_reputation() >= 0.0);
    }

    #[test]
    fn test_generate_proof() {
        let identity = make_identity("node-1");
        let proof = identity.generate_proof("eth-mainnet");
        assert!(proof.is_ok());
        let proof = proof.unwrap();
        assert_eq!(proof.node_id, "node-1");
        assert_eq!(proof.chain_id, "eth-mainnet");
    }

    #[test]
    fn test_proof_not_expired() {
        let identity = make_identity("node-1");
        let proof = identity.generate_proof("eth-mainnet").unwrap();
        assert!(!proof.is_expired(60_000));
    }

    #[test]
    fn test_proof_expired() {
        let identity = make_identity("node-1");
        let proof = identity.generate_proof("eth-mainnet").unwrap();
        assert!(proof.is_expired(0));
    }

    #[test]
    fn test_register_chain_key() {
        let mut identity = make_identity("node-1");
        identity.register_chain_key("eth-mainnet", "0xabc123".to_string());
        assert!(identity.has_chain_key("eth-mainnet"));
        assert_eq!(identity.chain_count(), 1);
    }

    #[test]
    fn test_record_contribution() {
        let mut identity = make_identity("node-1");
        identity.reputation.record_contribution("eth-mainnet");
        assert_eq!(identity.reputation.total_contributions, 1);
    }

    #[test]
    fn test_primary_public_key_hex() {
        let identity = make_identity("node-1");
        let hex_key = identity.primary_public_key_hex();
        assert_eq!(hex_key.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_reputation_new() {
        let rep = CrossChainReputation::new();
        assert_eq!(rep.global_score, 0.5);
        assert_eq!(rep.total_contributions, 0);
    }

    #[test]
    fn test_reputation_get_chain_score_default() {
        let rep = CrossChainReputation::new();
        assert_eq!(rep.get_chain_score("unknown"), 0.5);
    }

    #[test]
    fn test_reputation_multiple_chains() {
        let mut identity = make_identity("node-1");
        identity.update_reputation("eth-mainnet", 0.2);
        identity.update_reputation("solana-mainnet", -0.1);
        assert!(identity.reputation.chain_scores.len() == 2);
    }

    #[test]
    fn test_identity_error_display() {
        let err = IdentityError::InvalidSignature;
        assert!(!format!("{}", err).is_empty());
        let err = IdentityError::ChainKeyNotFound("eth".to_string());
        assert!(format!("{}", err).contains("eth"));
    }

    #[test]
    fn test_chain_key_pair() {
        let kp = ChainKeyPair::new("eth".to_string(), "0x123".to_string(), true);
        assert_eq!(kp.chain_id, "eth");
        assert!(kp.derived_from_primary);
    }

    #[test]
    fn test_identity_proof_creation() {
        let proof = IdentityProof::new(
            "node-1".to_string(),
            "eth".to_string(),
            "sig123".to_string(),
            "pub456".to_string(),
        );
        assert_eq!(proof.node_id, "node-1");
        assert_eq!(proof.chain_id, "eth");
    }

    #[test]
    fn test_multiple_signatures_different_messages() {
        let identity = make_identity("node-1");
        let msg1 = b"message 1";
        let msg2 = b"message 2";
        let sig1 = identity.sign_message(msg1);
        let sig2 = identity.sign_message(msg2);
        assert!(identity.verify_signature("eth", msg1, &sig1.to_bytes()));
        assert!(identity.verify_signature("eth", msg2, &sig2.to_bytes()));
        // Cross-verify should fail
        assert!(!identity.verify_signature("eth", msg1, &sig2.to_bytes()));
    }
}
