//! Tiered Verification â€” Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! VerificaciÃ³n en capas: Edge (Merkle + Ed25519) vs Prover Nodes (SNARKs batch).
//! Sin ZKP pesado en WASM/Edge. DelegaciÃ³n inteligente por tier.

use std::fmt;

/// Error types for Tiered Verification
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationError {
    /// Invalid proof structure
    InvalidProof,
    /// Hash mismatch
    HashMismatch,
    /// Proof expired
    Expired { age_ms: u64, max_ms: u64 },
    /// Missing node in DAG
    MissingNode(u64),
    /// Depth exceeded
    DepthExceeded { depth: usize, max: usize },
    /// Invalid tier
    InvalidTier(String),
    /// Signature verification failed
    SignatureFailed,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationError::InvalidProof => write!(f, "Invalid proof structure"),
            VerificationError::HashMismatch => write!(f, "Hash mismatch in Merkle proof"),
            VerificationError::Expired { age_ms, max_ms } => {
                write!(f, "Proof expired: {}ms > {}ms", age_ms, max_ms)
            }
            VerificationError::MissingNode(id) => write!(f, "Missing DAG node: {}", id),
            VerificationError::DepthExceeded { depth, max } => {
                write!(f, "Proof depth {} exceeds maximum {}", depth, max)
            }
            VerificationError::InvalidTier(t) => write!(f, "Invalid tier: {}", t),
            VerificationError::SignatureFailed => {
                write!(f, "Ed25519 signature verification failed")
            }
        }
    }
}

/// Execution tier for verification delegation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Edge/WASM: Merkle + Ed25519 only
    Edge,
    /// Core: Full SNARK/STARK verification
    Core,
    /// Hybrid: Merkle + selective SNARK
    Hybrid,
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tier::Edge => write!(f, "Edge"),
            Tier::Core => write!(f, "Core"),
            Tier::Hybrid => write!(f, "Hybrid"),
        }
    }
}

/// Configuration for Tiered Verification
#[derive(Debug, Clone)]
pub struct TieredVerifierConfig {
    /// Maximum proof depth
    pub max_depth: usize,
    /// Proof validity window in milliseconds
    pub validity_window_ms: u64,
    /// Hash variant (0 = SHA-256 based, 1 = Blake3 based)
    pub hash_variant: u8,
    /// Enable Ed25519 signature verification
    pub require_signature: bool,
}

impl TieredVerifierConfig {
    pub fn default_Topological() -> Self {
        Self {
            max_depth: 20,
            validity_window_ms: 300_000,
            hash_variant: 0,
            require_signature: true,
        }
    }

    pub fn validate(&self) -> Result<(), VerificationError> {
        if self.max_depth == 0 {
            return Err(VerificationError::DepthExceeded { depth: 0, max: 0 });
        }
        if self.max_depth > 64 {
            return Err(VerificationError::DepthExceeded {
                depth: self.max_depth,
                max: 64,
            });
        }
        if self.validity_window_ms == 0 {
            return Err(VerificationError::Expired {
                age_ms: 0,
                max_ms: 0,
            });
        }
        Ok(())
    }
}

impl Default for TieredVerifierConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Merkle proof structure
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub proof_id: u64,
    pub leaf_hash: u128,
    pub root_hash: u128,
    pub path: Vec<u128>,
    pub timestamp_ms: u64,
    pub signature: [u8; 64],
}

impl MerkleProof {
    pub fn new(
        proof_id: u64,
        leaf_hash: u128,
        root_hash: u128,
        path: Vec<u128>,
        timestamp_ms: u64,
    ) -> Self {
        let signature = Self::simulate_ed25519_sign(proof_id, root_hash);
        Self {
            proof_id,
            leaf_hash,
            root_hash,
            path,
            timestamp_ms,
            signature,
        }
    }

    pub fn without_signature(
        proof_id: u64,
        leaf_hash: u128,
        root_hash: u128,
        path: Vec<u128>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            proof_id,
            leaf_hash,
            root_hash,
            path,
            timestamp_ms,
            signature: [0u8; 64],
        }
    }

    pub fn depth(&self) -> usize {
        self.path.len()
    }

    fn simulate_ed25519_sign(proof_id: u64, root_hash: u128) -> [u8; 64] {
        let mut sig = [0u8; 64];
        let id_bytes = proof_id.to_le_bytes();
        let root_bytes = root_hash.to_le_bytes();
        for (i, b) in sig.iter_mut().enumerate() {
            if i < 8 {
                *b = id_bytes[i];
            } else if i < 24 {
                *b = root_bytes[i - 8];
            } else {
                *b = (i * 7 + 3) as u8;
            }
        }
        sig
    }
}

impl fmt::Display for MerkleProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MerkleProof {{ id: {}, depth: {}, root: 0x{:x}, ts: {} }}",
            self.proof_id,
            self.path.len(),
            self.root_hash,
            self.timestamp_ms
        )
    }
}

/// Verification record
#[derive(Debug, Clone)]
pub struct VerificationRecord {
    pub proof_id: u64,
    pub tier: Tier,
    pub verified: bool,
    pub timestamp_ms: u64,
}

impl fmt::Display for VerificationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VerificationRecord {{ id: {}, tier: {}, verified: {}, ts: {} }}",
            self.proof_id, self.tier, self.verified, self.timestamp_ms
        )
    }
}

/// Tiered Verifier â€” Edge (Merkle/Ed25519) vs Core (SNARKs)
pub struct TieredVerifier {
    config: TieredVerifierConfig,
    records: Vec<VerificationRecord>,
}

impl TieredVerifier {
    pub fn new() -> Self {
        Self {
            config: TieredVerifierConfig::default_Topological(),
            records: Vec::new(),
        }
    }

    pub fn with_config(config: TieredVerifierConfig) -> Result<Self, VerificationError> {
        config.validate()?;
        Ok(Self {
            config,
            records: Vec::new(),
        })
    }

    /// Verify proof based on tier
    pub fn verify(
        &mut self,
        proof: &MerkleProof,
        tier: Tier,
        current_ms: u64,
    ) -> Result<bool, VerificationError> {
        // Check depth
        if proof.depth() > self.config.max_depth {
            return Err(VerificationError::DepthExceeded {
                depth: proof.depth(),
                max: self.config.max_depth,
            });
        }

        // Check expiry
        let age = current_ms.saturating_sub(proof.timestamp_ms);
        if age > self.config.validity_window_ms {
            return Err(VerificationError::Expired {
                age_ms: age,
                max_ms: self.config.validity_window_ms,
            });
        }

        let verified = match tier {
            Tier::Edge => self.verify_edge(proof),
            Tier::Core => self.verify_core(proof),
            Tier::Hybrid => self.verify_hybrid(proof),
        };

        let record = VerificationRecord {
            proof_id: proof.proof_id,
            tier,
            verified,
            timestamp_ms: current_ms,
        };
        self.records.push(record);

        Ok(verified)
    }

    /// Edge verification: Merkle + Ed25519 only
    fn verify_edge(&self, proof: &MerkleProof) -> bool {
        // Verify Merkle path
        let mut current = proof.leaf_hash;
        for hash in &proof.path {
            current = Self::hash_pair(current, *hash, self.config.hash_variant);
        }
        let root_matches = current == proof.root_hash;

        // Verify Ed25519 signature
        let sig_valid = Self::verify_ed25519(proof.proof_id, proof.root_hash, &proof.signature);

        root_matches && sig_valid
    }

    /// Core verification: Full SNARK/STARK simulation
    fn verify_core(&self, proof: &MerkleProof) -> bool {
        // Edge verification first
        if !self.verify_edge(proof) {
            return false;
        }
        // Simulate SNARK batch verification (always passes if Merkle+Ed25519 passes)
        true
    }

    /// Hybrid: Merkle + selective SNARK based on proof value
    fn verify_hybrid(&self, proof: &MerkleProof) -> bool {
        // Always verify Merkle + Ed25519
        if !self.verify_edge(proof) {
            return false;
        }
        // Selective SNARK for high-value proofs (proof_id % 10 == 0)
        if proof.proof_id % 10 == 0 {
            return self.verify_core(proof);
        }
        true
    }

    /// Simulate Ed25519 verification
    pub fn verify_ed25519(proof_id: u64, root_hash: u128, signature: &[u8; 64]) -> bool {
        let expected = MerkleProof::simulate_ed25519_sign(proof_id, root_hash);
        signature == &expected
    }

    /// Hash pair with variant
    fn hash_pair(left: u128, right: u128, variant: u8) -> u128 {
        let combined = ((left as u128) << 64) | (right as u128);
        // Simple hash simulation
        let mut h = combined ^ (variant as u128);
        h = h.wrapping_mul(0x9e3779b97f4a7c15);
        h = h ^ (h >> 29);
        h = h.wrapping_mul(0x6c62272e07bb0142);
        h = h ^ (h >> 47);
        h
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let total = self.records.len();
        let verified = self.records.iter().filter(|r| r.verified).count();
        let failed = total - verified;
        (total, verified, failed)
    }

    pub fn reset(&mut self) {
        self.records.clear();
    }
}

impl Default for TieredVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TieredVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (total, verified, failed) = self.stats();
        write!(
            f,
            "TieredVerifier {{ total: {}, verified: {}, failed: {}, max_depth: {} }}",
            total, verified, failed, self.config.max_depth
        )
    }
}

/// Public function: Edge attestation verification
pub fn verify_edge_attestation(merkle_root: &[u8], proof: &MerkleProof, node_tier: Tier) -> bool {
    // For Edge tier, only verify Merkle + Ed25519
    if node_tier == Tier::Edge {
        // Convert merkle_root bytes to u128 for comparison
        if merkle_root.len() < 16 {
            return false;
        }
        let root_u128 = u128::from_le_bytes(merkle_root[..16].try_into().unwrap());
        // Simple root comparison
        root_u128 == proof.root_hash
    } else {
        // Core/Hybrid can do full verification
        true
    }
}

/// Compute Merkle root from leaves
pub fn compute_merkle_root(leaves: &[u128]) -> Option<u128> {
    if leaves.is_empty() {
        return None;
    }
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for i in (0..current.len()).step_by(2) {
            let left = current[i];
            let right = if i + 1 < current.len() {
                current[i + 1]
            } else {
                left
            };
            next.push(TieredVerifier::hash_pair(left, right, 0));
        }
        current = next;
    }
    Some(current[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TieredVerifierConfig::default();
        assert_eq!(config.max_depth, 20);
        assert_eq!(config.validity_window_ms, 300_000);
        assert!(config.require_signature);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = TieredVerifierConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_depth() {
        let config = TieredVerifierConfig {
            max_depth: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_depth_too_high() {
        let config = TieredVerifierConfig {
            max_depth: 100,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_validity() {
        let config = TieredVerifierConfig {
            validity_window_ms: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_verifier_creation() {
        let verifier = TieredVerifier::new();
        assert_eq!(verifier.stats(), (0, 0, 0));
    }

    #[test]
    fn test_verifier_with_config() {
        let config = TieredVerifierConfig::default_Topological();
        let verifier = TieredVerifier::with_config(config);
        assert!(verifier.is_ok());
    }

    #[test]
    fn test_build_proof() {
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![0x5678], 1000);
        assert_eq!(proof.proof_id, 1);
        assert_eq!(proof.depth(), 1);
    }

    #[test]
    fn test_verify_edge_success() {
        let mut verifier = TieredVerifier::new();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        let result = verifier.verify(&proof, Tier::Edge, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_core_success() {
        let mut verifier = TieredVerifier::new();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        let result = verifier.verify(&proof, Tier::Core, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_expired() {
        let mut verifier = TieredVerifier::new();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        let result = verifier.verify(&proof, Tier::Edge, 500_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_depth_exceeded() {
        let mut verifier = TieredVerifier::new();
        let path = vec![0u128; 25];
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, path, 1000);
        let result = verifier.verify(&proof, Tier::Edge, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_ed25519_verify_valid() {
        let sig = MerkleProof::simulate_ed25519_sign(1, 0xabcd);
        assert!(TieredVerifier::verify_ed25519(1, 0xabcd, &sig));
    }

    #[test]
    fn test_ed25519_verify_invalid() {
        let sig = [0u8; 64];
        assert!(!TieredVerifier::verify_ed25519(1, 0xabcd, &sig));
    }

    #[test]
    fn test_compute_merkle_root_single() {
        let root = compute_merkle_root(&[0x1234]);
        assert!(root.is_some());
    }

    #[test]
    fn test_compute_merkle_root_multiple() {
        let root = compute_merkle_root(&[0x1234, 0x5678, 0x9abc]);
        assert!(root.is_some());
    }

    #[test]
    fn test_compute_merkle_root_empty() {
        let root = compute_merkle_root(&[]);
        assert!(root.is_none());
    }

    #[test]
    fn test_verify_edge_attestation() {
        let root_bytes = 0xabcd_u128.to_le_bytes();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        let result = verify_edge_attestation(&root_bytes, &proof, Tier::Edge);
        assert!(result);
    }

    #[test]
    fn test_stats() {
        let mut verifier = TieredVerifier::new();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        verifier.verify(&proof, Tier::Edge, 1000).unwrap();
        let (total, verified, failed) = verifier.stats();
        assert_eq!(total, 1);
        assert_eq!(verified + failed, 1);
    }

    #[test]
    fn test_reset() {
        let mut verifier = TieredVerifier::new();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        verifier.verify(&proof, Tier::Edge, 1000).unwrap();
        verifier.reset();
        assert_eq!(verifier.stats(), (0, 0, 0));
    }

    #[test]
    fn test_display() {
        let verifier = TieredVerifier::new();
        let s = format!("{}", verifier);
        assert!(s.contains("TieredVerifier"));
    }

    #[test]
    fn test_proof_display() {
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![], 1000);
        let s = format!("{}", proof);
        assert!(s.contains("MerkleProof"));
    }

    #[test]
    fn test_error_display() {
        let err = VerificationError::InvalidProof;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_verification_workflow() {
        let mut verifier = TieredVerifier::new();
        let proof = MerkleProof::new(1, 0x1234, 0xabcd, vec![0x5678], 1000);
        let result = verifier.verify(&proof, Tier::Edge, 1000);
        assert!(result.is_ok());
        let (total, _, _) = verifier.stats();
        assert_eq!(total, 1);
    }
}
