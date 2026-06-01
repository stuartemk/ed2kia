//! ZKP Verification for SCT Metadata + GEI Vector — Sprint 70: Civilization-Scale Architecture
//!
//! Proof generation and verification for ethical alignment metadata
//! using Merkle-DAG aggregation.

use std::collections::HashMap;

/// Error types for ZKP verification.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationError {
    /// Invalid proof format.
    InvalidProof,
    /// Proof verification failed.
    VerificationFailed,
    /// Merkle root mismatch.
    MerkleRootMismatch,
    /// Proof expired.
    ProofExpired,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::InvalidProof => write!(f, "invalid proof format"),
            VerificationError::VerificationFailed => write!(f, "proof verification failed"),
            VerificationError::MerkleRootMismatch => write!(f, "merkle root mismatch"),
            VerificationError::ProofExpired => write!(f, "proof expired"),
        }
    }
}

impl std::error::Error for VerificationError {}

/// A verified proof record.
#[derive(Debug, Clone)]
pub struct ProofRecord {
    /// Unique proof identifier.
    pub proof_id: u64,
    /// SCT-Z value being proven.
    pub z_axis: f64,
    /// GEI vector (8 dimensions).
    pub gei: [f64; 8],
    /// Merkle root hash.
    pub merkle_root: u128,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Proof is verified.
    pub verified: bool,
}

impl ProofRecord {
    pub fn new(
        proof_id: u64,
        z_axis: f64,
        gei: [f64; 8],
        merkle_root: u128,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            proof_id,
            z_axis,
            gei,
            merkle_root,
            timestamp_ms,
            verified: false,
        }
    }

    /// Mark proof as verified.
    pub fn verify(&mut self) {
        self.verified = true;
    }

    /// Check if proof is within validity window.
    pub fn is_valid(&self, current_ms: u64, window_ms: u64) -> bool {
        self.verified && (current_ms - self.timestamp_ms) <= window_ms
    }
}

/// ZKP Verifier — manages proof generation and verification.
#[derive(Debug, Clone)]
pub struct ZkpVerifier {
    records: HashMap<u64, ProofRecord>,
    /// Proof validity window in milliseconds.
    validity_window_ms: u64,
}

impl ZkpVerifier {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            validity_window_ms: 5 * 60 * 1000, // 5 minutes
        }
    }

    pub fn with_validity_window(window_ms: u64) -> Self {
        Self {
            records: HashMap::new(),
            validity_window_ms: window_ms,
        }
    }

    /// Submit a proof record for verification.
    pub fn submit(&mut self, record: ProofRecord) -> Result<(), VerificationError> {
        if record.z_axis < -1.0 || record.z_axis > 1.0 {
            return Err(VerificationError::InvalidProof);
        }
        self.records.insert(record.proof_id, record);
        Ok(())
    }

    /// Verify a proof by ID.
    pub fn verify_proof(&mut self, proof_id: u64) -> Result<&ProofRecord, VerificationError> {
        let record = self
            .records
            .get_mut(&proof_id)
            .ok_or(VerificationError::InvalidProof)?;

        // Simulate verification (in production, run arkworks circuit)
        record.verify();
        Ok(record)
    }

    /// Check proof validity at current time.
    pub fn check_validity(
        &self,
        proof_id: u64,
        current_ms: u64,
    ) -> Result<bool, VerificationError> {
        let record = self
            .records
            .get(&proof_id)
            .ok_or(VerificationError::InvalidProof)?;

        Ok(record.is_valid(current_ms, self.validity_window_ms))
    }

    /// Compute Merkle root from proof records.
    pub fn compute_merkle_root(&self) -> u128 {
        if self.records.is_empty() {
            return 0;
        }

        // Simple hash aggregation (production: proper Merkle tree)
        let mut root: u128 = 0;
        for record in self.records.values() {
            root ^= record.merkle_root;
            root = root.rotate_left(17);
        }
        root
    }

    /// Get verified proof count.
    pub fn verified_count(&self) -> usize {
        self.records.values().filter(|r| r.verified).count()
    }

    /// Get total proof count.
    pub fn total_count(&self) -> usize {
        self.records.len()
    }

    /// Get expired proofs.
    pub fn get_expired(&self, current_ms: u64) -> Vec<u64> {
        self.records
            .iter()
            .filter(|(_, r)| !r.is_valid(current_ms, self.validity_window_ms))
            .map(|(id, _)| *id)
            .collect()
    }
}

impl Default for ZkpVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ZkpVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ZkpVerifier {{ total: {}, verified: {} }}",
            self.total_count(),
            self.verified_count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(id: u64, z: f64) -> ProofRecord {
        ProofRecord::new(id, z, [0.1; 8], 12345, 1000)
    }

    #[test]
    fn test_verifier_creation() {
        let verifier = ZkpVerifier::new();
        assert_eq!(verifier.total_count(), 0);
    }

    #[test]
    fn test_submit_proof() {
        let mut verifier = ZkpVerifier::new();
        verifier.submit(make_proof(1, 0.5)).unwrap();
        assert_eq!(verifier.total_count(), 1);
    }

    #[test]
    fn test_submit_invalid_proof() {
        let mut verifier = ZkpVerifier::new();
        let result = verifier.submit(make_proof(1, 2.0));
        assert_eq!(result, Err(VerificationError::InvalidProof));
    }

    #[test]
    fn test_verify_proof() {
        let mut verifier = ZkpVerifier::new();
        verifier.submit(make_proof(1, 0.5)).unwrap();
        let record = verifier.verify_proof(1).unwrap();
        assert!(record.verified);
    }

    #[test]
    fn test_verify_nonexistent() {
        let mut verifier = ZkpVerifier::new();
        let result = verifier.verify_proof(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_validity_within_window() {
        let mut verifier = ZkpVerifier::new();
        verifier.submit(make_proof(1, 0.5)).unwrap();
        verifier.verify_proof(1).unwrap();
        let valid = verifier.check_validity(1, 2000).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_check_validity_expired() {
        let mut verifier = ZkpVerifier::new();
        verifier.submit(make_proof(1, 0.5)).unwrap();
        verifier.verify_proof(1).unwrap();
        let valid = verifier.check_validity(1, 10_000_000).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_merkle_root() {
        let mut verifier = ZkpVerifier::new();
        verifier.submit(make_proof(1, 0.5)).unwrap();
        verifier.submit(make_proof(2, 0.3)).unwrap();
        let root = verifier.compute_merkle_root();
        assert_ne!(root, 0);
    }

    #[test]
    fn test_merkle_root_empty() {
        let verifier = ZkpVerifier::new();
        assert_eq!(verifier.compute_merkle_root(), 0);
    }

    #[test]
    fn test_get_expired() {
        let mut verifier = ZkpVerifier::new();
        verifier.submit(make_proof(1, 0.5)).unwrap();
        verifier.verify_proof(1).unwrap();
        let expired = verifier.get_expired(10_000_000);
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_verifier_display() {
        let verifier = ZkpVerifier::new();
        let s = format!("{}", verifier);
        assert!(s.contains("ZkpVerifier"));
    }
}
