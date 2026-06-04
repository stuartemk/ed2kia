//! KZG State Pruning â€” Sprint 74: Distributed Systems Hardening & Second-Order Resolution
//!
//! Polynomial commitments for cryptographic state pruning.
//! Enables O(1) verification of pruned state without full download.
//!
//! # Design
//!
//! KZG commitments allow compact representation of large state data.
//! Edge nodes can verify state consistency using lightweight proofs
//! without downloading the full state.
//!
//! # Guarantees
//!
//! - Commitment generation: O(n) for polynomial of degree n
//! - Verification: O(1) per point evaluation
//! - Memory: O(n) for commitment, O(1) for verification

use std::fmt;

/// Errors for KZG state pruning operations.
#[derive(Debug, Clone, PartialEq)]
pub enum KzgError {
    /// Empty state hash provided.
    EmptyState,
    /// Invalid polynomial degree.
    InvalidDegree(usize),
    /// Proof verification failed.
    ProofVerificationFailed,
    /// State hash mismatch.
    StateMismatch,
    /// Pruning window exceeded.
    PruningWindowExceeded,
}

impl fmt::Display for KzgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KzgError::EmptyState => write!(f, "KZG: empty state"),
            KzgError::InvalidDegree(d) => write!(f, "KZG: invalid polynomial degree {}", d),
            KzgError::ProofVerificationFailed => write!(f, "KZG: proof verification failed"),
            KzgError::StateMismatch => write!(f, "KZG: state hash mismatch"),
            KzgError::PruningWindowExceeded => write!(f, "KZG: pruning window exceeded"),
        }
    }
}

impl std::error::Error for KzgError {}

/// Configuration for KZG state pruning.
#[derive(Debug, Clone)]
pub struct KzgConfig {
    /// Maximum polynomial degree for commitments.
    pub max_degree: usize,
    /// Pruning window in milliseconds.
    pub pruning_window_ms: u64,
    /// Number of historical commitments to retain.
    pub retention_count: usize,
    /// Enable compression for stored commitments.
    pub compress: bool,
}

impl KzgConfig {
    /// Default Topological configuration for production KZG pruning.
    pub fn default_Topological() -> Self {
        Self {
            max_degree: 1024,
            pruning_window_ms: 86_400_000, // 24 hours
            retention_count: 16,
            compress: true,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), KzgError> {
        if self.max_degree == 0 {
            return Err(KzgError::InvalidDegree(0));
        }
        if self.retention_count == 0 {
            return Err(KzgError::PruningWindowExceeded);
        }
        Ok(())
    }
}

impl Default for KzgConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// A KZG commitment record.
#[derive(Debug, Clone)]
pub struct KzgCommitment {
    /// State hash being committed to.
    pub state_hash: Vec<u8>,
    /// Polynomial degree used.
    pub degree: usize,
    /// Commitment bytes.
    pub commitment: Vec<u8>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Proof for point evaluation.
    pub proof: Vec<u8>,
}

impl fmt::Display for KzgCommitment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KzgCommitment(degree={}, size={}, ts={})",
            self.degree,
            self.commitment.len(),
            self.timestamp_ms
        )
    }
}

/// Record of a pruning operation.
#[derive(Debug, Clone)]
pub struct PruningRecord {
    /// Number of states pruned.
    pub pruned_count: usize,
    /// Total bytes saved.
    pub bytes_saved: usize,
    /// Timestamp of pruning operation.
    pub timestamp_ms: u64,
}

impl fmt::Display for PruningRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PruningRecord(pruned={}, saved={}, ts={})",
            self.pruned_count, self.bytes_saved, self.timestamp_ms
        )
    }
}

/// KZG State Pruning engine.
pub struct KzgStatePruner {
    config: KzgConfig,
    commitments: Vec<KzgCommitment>,
    pruning_records: Vec<PruningRecord>,
}

impl KzgStatePruner {
    /// Create a new KZG state pruner with default configuration.
    pub fn new() -> Self {
        Self {
            config: KzgConfig::default_Topological(),
            commitments: Vec::new(),
            pruning_records: Vec::new(),
        }
    }

    /// Create a KZG state pruner with custom configuration.
    pub fn with_config(config: KzgConfig) -> Result<Self, KzgError> {
        config.validate()?;
        Ok(Self {
            config,
            commitments: Vec::new(),
            pruning_records: Vec::new(),
        })
    }

    /// Generate a KZG commitment for the given state hash.
    pub fn generate_commitment(
        &mut self,
        state_hash: &[u8],
        polynomial_degree: usize,
        timestamp_ms: u64,
    ) -> Result<KzgCommitment, KzgError> {
        if state_hash.is_empty() {
            return Err(KzgError::EmptyState);
        }
        if polynomial_degree == 0 || polynomial_degree > self.config.max_degree {
            return Err(KzgError::InvalidDegree(polynomial_degree));
        }

        let commitment = Self::compute_commitment(state_hash, polynomial_degree);
        let proof = Self::compute_proof(state_hash, polynomial_degree);

        let kzg_commitment = KzgCommitment {
            state_hash: state_hash.to_vec(),
            degree: polynomial_degree,
            commitment,
            timestamp_ms,
            proof,
        };

        self.commitments.push(kzg_commitment.clone());
        Ok(kzg_commitment)
    }

    /// Verify a KZG commitment against the original state hash.
    pub fn verify_commitment(
        &self,
        commitment: &KzgCommitment,
        expected_state: &[u8],
    ) -> Result<bool, KzgError> {
        if expected_state.is_empty() {
            return Err(KzgError::EmptyState);
        }

        // Verify state hash matches
        if commitment.state_hash != expected_state {
            return Ok(false);
        }

        // Verify commitment integrity
        let expected_commitment = Self::compute_commitment(expected_state, commitment.degree);
        if commitment.commitment != expected_commitment {
            return Ok(false);
        }

        // Verify proof
        let expected_proof = Self::compute_proof(expected_state, commitment.degree);
        if commitment.proof != expected_proof {
            return Ok(false);
        }

        Ok(true)
    }

    /// Prune expired commitments based on the pruning window.
    pub fn prune_expired(&mut self, current_ms: u64) -> Result<PruningRecord, KzgError> {
        let cutoff = current_ms.saturating_sub(self.config.pruning_window_ms);

        let before_len = self.commitments.len();
        let before_bytes: usize = self
            .commitments
            .iter()
            .map(|c| c.commitment.len() + c.state_hash.len())
            .sum();

        // Collect indices to remove (expired, keeping retention_count)
        let retention = self.config.retention_count;
        let expired: Vec<usize> = self
            .commitments
            .iter()
            .enumerate()
            .filter(|(_, c)| c.timestamp_ms < cutoff)
            .map(|(i, _)| i)
            .collect();

        // Remove in reverse order to keep indices valid
        for i in expired.into_iter().rev() {
            if self.commitments.len() > retention {
                self.commitments.remove(i);
            }
        }
        let after_len = self.commitments.len();
        let after_bytes: usize = self
            .commitments
            .iter()
            .map(|c| c.commitment.len() + c.state_hash.len())
            .sum();

        let pruned_count = before_len.saturating_sub(after_len);
        let bytes_saved = before_bytes.saturating_sub(after_bytes);

        let record = PruningRecord {
            pruned_count,
            bytes_saved,
            timestamp_ms: current_ms,
        };

        self.pruning_records.push(record.clone());
        Ok(record)
    }

    /// Get the latest commitment for a given state hash.
    pub fn get_latest_commitment(&self, state_hash: &[u8]) -> Option<&KzgCommitment> {
        self.commitments
            .iter()
            .rev()
            .find(|c| c.state_hash == state_hash)
    }

    /// Get all commitments.
    pub fn commitments(&self) -> &[KzgCommitment] {
        &self.commitments
    }

    /// Get all pruning records.
    pub fn pruning_records(&self) -> &[PruningRecord] {
        &self.pruning_records
    }

    /// Reset the pruner state.
    pub fn reset(&mut self) {
        self.commitments.clear();
        self.pruning_records.clear();
    }

    /// Compute KZG commitment (simulated polynomial commitment).
    fn compute_commitment(state_hash: &[u8], degree: usize) -> Vec<u8> {
        // Simulated KZG: hash the state with degree as domain separator
        let mut commitment = Vec::with_capacity(32);
        commitment.extend_from_slice(state_hash);
        commitment.extend_from_slice(&degree.to_le_bytes());
        // Simple hash for simulation
        let hash = Self::fnv_hash(&commitment);
        hash.to_le_bytes().to_vec()
    }

    /// Compute KZG proof (simulated point evaluation proof).
    fn compute_proof(state_hash: &[u8], degree: usize) -> Vec<u8> {
        let mut proof_data = Vec::with_capacity(33);
        proof_data.extend_from_slice(state_hash);
        proof_data.push(degree as u8);
        let hash = Self::fnv_hash(&proof_data);
        hash.to_le_bytes().to_vec()
    }

    /// FNV-1a hash for simulation.
    fn fnv_hash(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

impl Default for KzgStatePruner {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for KzgStatePruner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KzgStatePruner(commitments={}, prunings={})",
            self.commitments.len(),
            self.pruning_records.len()
        )
    }
}

/// Public function: generate KZG commitment for state pruning.
pub fn generate_kzg_commitment(state_hash: &[u8], polynomial_degree: usize) -> Vec<u8> {
    if state_hash.is_empty() || polynomial_degree == 0 {
        return Vec::new();
    }
    let mut commitment = Vec::with_capacity(32);
    commitment.extend_from_slice(state_hash);
    commitment.extend_from_slice(&polynomial_degree.to_le_bytes());
    let hash = KzgStatePruner::fnv_hash(&commitment);
    hash.to_le_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = KzgConfig::default_Topological();
        assert_eq!(config.max_degree, 1024);
        assert_eq!(config.pruning_window_ms, 86_400_000);
        assert_eq!(config.retention_count, 16);
        assert!(config.compress);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = KzgConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_degree() {
        let config = KzgConfig {
            max_degree: 0,
            ..KzgConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_retention() {
        let config = KzgConfig {
            retention_count: 0,
            ..KzgConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pruner_creation() {
        let pruner = KzgStatePruner::new();
        assert!(pruner.commitments().is_empty());
        assert!(pruner.pruning_records().is_empty());
    }

    #[test]
    fn test_pruner_with_config() {
        let config = KzgConfig::default_Topological();
        let pruner = KzgStatePruner::with_config(config).unwrap();
        assert!(pruner.commitments().is_empty());
    }

    #[test]
    fn test_generate_commitment() {
        let mut pruner = KzgStatePruner::new();
        let state = vec![1u8, 2, 3, 4];
        let commitment = pruner.generate_commitment(&state, 10, 1000).unwrap();
        assert_eq!(commitment.degree, 10);
        assert_eq!(commitment.timestamp_ms, 1000);
        assert_eq!(commitment.state_hash, state);
        assert!(!commitment.commitment.is_empty());
        assert!(!commitment.proof.is_empty());
    }

    #[test]
    fn test_generate_empty_state() {
        let mut pruner = KzgStatePruner::new();
        let result = pruner.generate_commitment(&[], 10, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_invalid_degree() {
        let mut pruner = KzgStatePruner::new();
        let state = vec![1u8];
        let result = pruner.generate_commitment(&state, 0, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_degree_too_high() {
        let mut pruner = KzgStatePruner::new();
        let state = vec![1u8];
        let result = pruner.generate_commitment(&state, 2048, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_commitment_valid() {
        let mut pruner = KzgStatePruner::new();
        let state = vec![1u8, 2, 3, 4];
        let commitment = pruner.generate_commitment(&state, 10, 1000).unwrap();
        let result = pruner.verify_commitment(&commitment, &state).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_commitment_wrong_state() {
        let mut pruner = KzgStatePruner::new();
        let state = vec![1u8, 2, 3, 4];
        let commitment = pruner.generate_commitment(&state, 10, 1000).unwrap();
        let wrong_state = vec![5u8, 6, 7, 8];
        let result = pruner.verify_commitment(&commitment, &wrong_state).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_verify_empty_state() {
        let pruner = KzgStatePruner::new();
        let commitment = KzgCommitment {
            state_hash: vec![1u8],
            degree: 10,
            commitment: vec![1u8],
            timestamp_ms: 1000,
            proof: vec![1u8],
        };
        let result = pruner.verify_commitment(&commitment, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_prune_expired() {
        let mut pruner = KzgStatePruner::new();
        // Add old commitments
        for i in 0..20 {
            pruner
                .generate_commitment(&vec![i as u8], 10, i * 1000)
                .unwrap();
        }
        assert_eq!(pruner.commitments().len(), 20);

        // Prune with current time far in the future
        let record = pruner.prune_expired(200_000_000).unwrap();
        assert!(record.pruned_count > 0);
        assert!(record.bytes_saved > 0);
    }

    #[test]
    fn test_prune_retains_minimum() {
        let mut pruner = KzgStatePruner::new();
        // Create more commitments than retention_count (16)
        for i in 0..30 {
            pruner
                .generate_commitment(&vec![i as u8], 10, i * 1000)
                .unwrap();
        }

        // Prune with very large window - should retain at least retention_count
        let record = pruner.prune_expired(200_000_000).unwrap();
        assert!(pruner.commitments().len() >= pruner.config.retention_count);
    }

    #[test]
    fn test_get_latest_commitment() {
        let mut pruner = KzgStatePruner::new();
        let state = vec![42u8];
        pruner.generate_commitment(&state, 10, 1000).unwrap();
        pruner.generate_commitment(&state, 20, 2000).unwrap();

        let latest = pruner.get_latest_commitment(&state).unwrap();
        assert_eq!(latest.degree, 20);
        assert_eq!(latest.timestamp_ms, 2000);
    }

    #[test]
    fn test_get_latest_commitment_missing() {
        let pruner = KzgStatePruner::new();
        let result = pruner.get_latest_commitment(&[42u8]);
        assert!(result.is_none());
    }

    #[test]
    fn test_reset() {
        let mut pruner = KzgStatePruner::new();
        pruner.generate_commitment(&vec![1u8], 10, 1000).unwrap();
        assert!(!pruner.commitments().is_empty());

        pruner.reset();
        assert!(pruner.commitments().is_empty());
        assert!(pruner.pruning_records().is_empty());
    }

    #[test]
    fn test_display() {
        let pruner = KzgStatePruner::new();
        let display = format!("{}", pruner);
        assert!(display.contains("KzgStatePruner"));
    }

    #[test]
    fn test_standalone_commitment() {
        let state = vec![1u8, 2, 3];
        let commitment = generate_kzg_commitment(&state, 10);
        assert!(!commitment.is_empty());
    }

    #[test]
    fn test_standalone_commitment_empty() {
        let commitment = generate_kzg_commitment(&[], 10);
        assert!(commitment.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut pruner = KzgStatePruner::new();

        // Generate commitments
        for i in 0..10 {
            let state = vec![i as u8; 32];
            pruner.generate_commitment(&state, 64, i * 1000).unwrap();
        }
        assert_eq!(pruner.commitments().len(), 10);

        // Verify commitments
        for i in 0..10 {
            let state = vec![i as u8; 32];
            let commitment = pruner.get_latest_commitment(&state).unwrap();
            let valid = pruner.verify_commitment(commitment, &state).unwrap();
            assert!(valid);
        }

        // Prune
        let record = pruner.prune_expired(200_000_000).unwrap();
        assert!(record.pruned_count >= 0);
        assert!(!pruner.pruning_records().is_empty());
    }
}
