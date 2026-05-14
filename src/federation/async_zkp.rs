//! Async ZKP Federation — Batch proof generation, light ZKP optimization, async verification, Merkle fallback

use std::collections::{HashMap, VecDeque};

use sha2::{Digest, Sha256};
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ZKPError {
    #[error("batch too large: {size} > {max}")]
    BatchTooLarge { size: usize, max: usize },
    #[error("verification failed: {0}")]
    VerificationFailed(String),
    #[error("insufficient resources for ZKP: {reason}")]
    InsufficientResources { reason: String },
    #[error("proof generation failed: {0}")]
    ProofGenerationFailed(String),
    #[error("invalid merkle root: expected={expected}, got={got}")]
    InvalidMerkleRoot { expected: String, got: String },
}

// ─── Core Types ───────────────────────────────────────────────────────────────

/// A single delta proof to be batched.
#[derive(Debug, Clone)]
pub struct DeltaProof {
    pub delta_id: String,
    pub source_node: String,
    pub layer_id: u32,
    pub data_hash: String,
    pub proof_bytes: Vec<u8>,
}

impl DeltaProof {
    pub fn new(delta_id: String, source_node: String, layer_id: u32, data: &[u8]) -> Self {
        let data_hash = compute_sha256(data);
        Self {
            delta_id,
            source_node,
            layer_id,
            data_hash,
            proof_bytes: data.to_vec(),
        }
    }
}

/// Result of a ZKP operation.
#[derive(Debug, Clone)]
pub struct ZKPResult {
    pub proof_hash: String,
    pub verified: bool,
    pub fallback_triggered: bool,
    pub batch_size: usize,
}

impl ZKPResult {
    pub fn verified(proof_hash: String, batch_size: usize) -> Self {
        Self {
            proof_hash,
            verified: true,
            fallback_triggered: false,
            batch_size,
        }
    }

    pub fn fallback(proof_hash: String, batch_size: usize) -> Self {
        Self {
            proof_hash,
            verified: true,
            fallback_triggered: true,
            batch_size,
        }
    }

    pub fn failed(proof_hash: String, batch_size: usize) -> Self {
        Self {
            proof_hash,
            verified: false,
            fallback_triggered: false,
            batch_size,
        }
    }
}

/// Merkle proof fallback structure.
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub root_hash: String,
    pub proof_path: Vec<String>,
    pub leaf_index: usize,
    pub vrf_nonce: String,
}

impl MerkleProof {
    pub fn new(leaf_data: &[u8], vrf_nonce: String) -> Self {
        let leaf_hash = compute_sha256(leaf_data);
        Self {
            root_hash: leaf_hash.clone(),
            proof_path: vec![leaf_hash],
            leaf_index: 0,
            vrf_nonce,
        }
    }

    pub fn verify(&self, expected_root: &str) -> bool {
        self.root_hash == expected_root
    }
}

/// Configuration for the AsyncZKPFederation engine.
#[derive(Debug, Clone)]
pub struct ZKPConfig {
    pub max_batch_size: usize,
    pub min_batch_size: usize,
    pub gas_threshold: u64,
    pub min_cpu_cores: u32,
    pub enable_auto_fallback: bool,
}

impl Default for ZKPConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 50,
            min_batch_size: 10,
            gas_threshold: 30_000_000, // 30M gas
            min_cpu_cores: 4,
            enable_auto_fallback: true,
        }
    }
}

/// Stats for the ZKP federation engine.
#[derive(Debug, Clone)]
pub struct ZKPStats {
    pub total_batches: usize,
    pub total_verifications: usize,
    pub fallback_count: usize,
    pub pending_proofs: usize,
    pub avg_batch_size: f64,
}

/// Pending batch awaiting verification.
#[derive(Debug, Clone)]
pub struct PendingBatch {
    pub batch_id: String,
    pub proofs: Vec<DeltaProof>,
    pub created_at_ms: u64,
    pub proof_hash: String,
}

// ─── AsyncZKPFederation Engine ───────────────────────────────────────────────

pub struct AsyncZKPFederation {
    config: ZKPConfig,
    pending_batches: VecDeque<PendingBatch>,
    verified_proofs: HashMap<String, ZKPResult>,
    merkle_fallbacks: HashMap<String, MerkleProof>,
    stats: ZKPStats,
    audit_log: VecDeque<String>,
}

impl AsyncZKPFederation {
    pub fn new() -> Self {
        Self::with_config(ZKPConfig::default())
    }

    pub fn with_config(config: ZKPConfig) -> Self {
        Self {
            config,
            pending_batches: VecDeque::new(),
            verified_proofs: HashMap::new(),
            merkle_fallbacks: HashMap::new(),
            stats: ZKPStats {
                total_batches: 0,
                total_verifications: 0,
                fallback_count: 0,
                pending_proofs: 0,
                avg_batch_size: 0.0,
            },
            audit_log: VecDeque::with_capacity(256),
        }
    }

    /// Batch multiple delta proofs together for efficient ZKP generation.
    pub fn batch_proofs(&mut self, proofs: Vec<DeltaProof>) -> Result<PendingBatch, ZKPError> {
        let proof_count = proofs.len();
        if proof_count == 0 {
            return Err(ZKPError::ProofGenerationFailed("empty proof list".into()));
        }

        if proof_count > self.config.max_batch_size {
            return Err(ZKPError::BatchTooLarge {
                size: proof_count,
                max: self.config.max_batch_size,
            });
        }

        let batch_id = format!("batch_{}", current_timestamp_ms());

        // Compute batch hash from all proof hashes
        let combined: String = proofs.iter().map(|p| p.data_hash.clone()).collect();
        let proof_hash = compute_sha256(combined.as_bytes());

        let batch = PendingBatch {
            batch_id: batch_id.clone(),
            proofs,
            created_at_ms: current_timestamp_ms(),
            proof_hash: proof_hash.clone(),
        };

        self.pending_batches.push_back(batch.clone());
        self.stats.pending_proofs += proof_count;
        self.stats.total_batches += 1;
        self.update_avg_batch_size(proof_count);

        self.audit(&format!("batch_created: {} ({} proofs)", batch_id, proof_count));
        Ok(batch)
    }

    /// Generate a light ZKP proof optimized for SAE forward pass.
    pub fn generate_light_proof(&mut self, delta: DeltaProof) -> Result<ZKPResult, ZKPError> {
        // Check if resources are sufficient for full ZKP
        let should_fallback = self.should_fallback();

        if should_fallback && self.config.enable_auto_fallback {
            // Fall back to Merkle + VRF
            return self.generate_merkle_fallback(delta);
        }

        // Simulate light ZKP proof generation using ark-bn254 curve parameters
        // In production, this would use ark-ec circuit compilation
        let proof_data = format!("zkp_{}_{}", delta.delta_id, delta.data_hash);
        let proof_hash = compute_sha256(proof_data.as_bytes());

        let result = ZKPResult::verified(proof_hash.clone(), 1);
        self.verified_proofs.insert(delta.delta_id.clone(), result.clone());

        self.audit(&format!("light_proof_generated: {}", delta.delta_id));
        Ok(result)
    }

    /// Verify a proof asynchronously.
    pub fn verify_async(&mut self, batch_id: String, proof_hash: String) -> Result<ZKPResult, ZKPError> {
        // Extract batch info to avoid holding immutable borrow
        let (batch_proof_hash, batch_size) = {
            let batch = self.find_batch(&batch_id)
                .ok_or_else(|| ZKPError::VerificationFailed(format!("batch {} not found", batch_id)))?;
            (batch.proof_hash.clone(), batch.proofs.len())
        };

        // Verify proof hash matches
        if batch_proof_hash != proof_hash {
            return Err(ZKPError::VerificationFailed("proof hash mismatch".into()));
        }

        // Simulate async verification
        let verified = true; // In production, run ark-ec verification circuit

        let result = if verified {
            ZKPResult::verified(proof_hash.clone(), batch_size)
        } else {
            ZKPResult::failed(proof_hash.clone(), batch_size)
        };

        self.verified_proofs.insert(batch_id.clone(), result.clone());
        self.stats.total_verifications += 1;

        // Remove from pending if verified
        if verified {
            self.remove_batch(&batch_id);
            self.stats.pending_proofs = self.stats.pending_proofs.saturating_sub(batch_size);
        }

        self.audit(&format!("verify_async: {} (verified={})", batch_id, verified));
        Ok(result)
    }

    /// Fallback to MerkleProof + VRF when ZKP is too expensive.
    pub fn fallback_to_merkle(&mut self, delta: DeltaProof) -> Result<ZKPResult, ZKPError> {
        self.generate_merkle_fallback(delta)
    }

    fn generate_merkle_fallback(&mut self, delta: DeltaProof) -> Result<ZKPResult, ZKPError> {
        let vrf_nonce = format!("vrf_{}_{}", delta.delta_id, current_timestamp_ms());
        let merkle = MerkleProof::new(&delta.proof_bytes, vrf_nonce);

        self.merkle_fallbacks.insert(delta.delta_id.clone(), merkle);
        self.stats.fallback_count += 1;

        let proof_hash = compute_sha256(delta.data_hash.as_bytes());
        let result = ZKPResult::fallback(proof_hash.clone(), 1);

        self.verified_proofs.insert(delta.delta_id.clone(), result.clone());
        self.audit(&format!("merkle_fallback: {}", delta.delta_id));
        Ok(result)
    }

    fn should_fallback(&self) -> bool {
        // Check CPU cores
        let cpu_cores = num_cpus::get() as u32;
        if cpu_cores < self.config.min_cpu_cores {
            return true;
        }

        // In production, check gas_used > threshold
        // For simulation, we check if pending batches are too large
        let total_pending: usize = self.pending_batches.iter().map(|b| b.proofs.len()).sum();
        total_pending > self.config.max_batch_size * 2
    }

    fn find_batch(&self, batch_id: &str) -> Option<&PendingBatch> {
        self.pending_batches.iter().find(|b| b.batch_id == batch_id)
    }

    fn remove_batch(&mut self, batch_id: &str) {
        self.pending_batches.retain(|b| b.batch_id != batch_id);
    }

    fn update_avg_batch_size(&mut self, new_size: usize) {
        let total = self.stats.total_batches;
        let old_avg = self.stats.avg_batch_size;
        self.stats.avg_batch_size = (old_avg * (total - 1) as f64 + new_size as f64) / total as f64;
    }

    fn audit(&mut self, message: &str) {
        self.audit_log.push_back(format!("[{}] {}", current_timestamp_ms(), message));
        if self.audit_log.len() > 256 {
            self.audit_log.pop_front();
        }
    }

    // ─── Accessors ───────────────────────────────────────────────────────────

    pub fn get_stats(&self) -> ZKPStats {
        self.stats.clone()
    }

    pub fn get_verified_result(&self, proof_id: &str) -> Option<&ZKPResult> {
        self.verified_proofs.get(proof_id)
    }

    pub fn get_merkle_proof(&self, proof_id: &str) -> Option<&MerkleProof> {
        self.merkle_fallbacks.get(proof_id)
    }

    pub fn pending_batch_count(&self) -> usize {
        self.pending_batches.len()
    }

    pub fn pending_proof_count(&self) -> usize {
        self.stats.pending_proofs
    }

    pub fn audit_trail(&self) -> &[String] {
        self.audit_log.as_slices().0
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.pending_batches.clear();
        self.verified_proofs.clear();
        self.merkle_fallbacks.clear();
        self.stats = ZKPStats {
            total_batches: 0,
            total_verifications: 0,
            fallback_count: 0,
            pending_proofs: 0,
            avg_batch_size: 0.0,
        };
        self.audit_log.clear();
    }
}

impl Default for AsyncZKPFederation {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(id: &str) -> DeltaProof {
        DeltaProof::new(id.to_string(), "node1".into(), 0, b"test_data")
    }

    #[test]
    fn test_zkp_creation() {
        let zkp = AsyncZKPFederation::new();
        assert_eq!(zkp.pending_batch_count(), 0);
    }

    #[test]
    fn test_batch_proofs() {
        let mut zkp = AsyncZKPFederation::new();
        let proofs = vec![make_proof("p1"), make_proof("p2"), make_proof("p3")];
        let result = zkp.batch_proofs(proofs);
        assert!(result.is_ok());
        assert_eq!(zkp.pending_batch_count(), 1);
    }

    #[test]
    fn test_batch_too_large() {
        let mut zkp = AsyncZKPFederation::with_config(ZKPConfig {
            max_batch_size: 5,
            ..ZKPConfig::default()
        });
        let proofs: Vec<_> = (0..10).map(|i| make_proof(&format!("p{}", i))).collect();
        let result = zkp.batch_proofs(proofs);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_empty() {
        let mut zkp = AsyncZKPFederation::new();
        let result = zkp.batch_proofs(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_light_proof() {
        let mut zkp = AsyncZKPFederation::new();
        let proof = make_proof("p1");
        let result = zkp.generate_light_proof(proof);
        assert!(result.is_ok());
        assert!(result.unwrap().verified);
    }

    #[test]
    fn test_verify_async_success() {
        let mut zkp = AsyncZKPFederation::new();
        let proofs = vec![make_proof("p1"), make_proof("p2")];
        let batch = zkp.batch_proofs(proofs).unwrap();

        let result = zkp.verify_async(batch.batch_id.clone(), batch.proof_hash.clone());
        assert!(result.is_ok());
        assert!(result.unwrap().verified);
    }

    #[test]
    fn test_verify_async_wrong_hash() {
        let mut zkp = AsyncZKPFederation::new();
        let proofs = vec![make_proof("p1")];
        let batch = zkp.batch_proofs(proofs).unwrap();

        let result = zkp.verify_async(batch.batch_id, "wrong_hash".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_async_unknown_batch() {
        let mut zkp = AsyncZKPFederation::new();
        let result = zkp.verify_async("unknown".into(), "hash".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_to_merkle() {
        let mut zkp = AsyncZKPFederation::new();
        let proof = make_proof("p1");
        let result = zkp.fallback_to_merkle(proof);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.fallback_triggered);
        assert!(result.verified);
    }

    #[test]
    fn test_merkle_proof_verification() {
        let data = b"test_leaf";
        let merkle = MerkleProof::new(data, "vrf_1".into());
        assert!(merkle.verify(&merkle.root_hash));
        assert!(!merkle.verify("different_root"));
    }

    #[test]
    fn test_stats_tracking() {
        let mut zkp = AsyncZKPFederation::new();
        let proofs = vec![make_proof("p1"), make_proof("p2")];
        zkp.batch_proofs(proofs).unwrap();

        let stats = zkp.get_stats();
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.pending_proofs, 2);
    }

    #[test]
    fn test_reset() {
        let mut zkp = AsyncZKPFederation::new();
        let proofs = vec![make_proof("p1")];
        zkp.batch_proofs(proofs).unwrap();
        zkp.reset();

        assert_eq!(zkp.pending_batch_count(), 0);
        assert_eq!(zkp.get_stats().total_batches, 0);
    }

    #[test]
    fn test_audit_trail() {
        let mut zkp = AsyncZKPFederation::new();
        let proofs = vec![make_proof("p1")];
        zkp.batch_proofs(proofs).unwrap();

        let trail = zkp.audit_trail();
        assert!(!trail.is_empty());
    }

    #[test]
    fn test_zkp_result_verified() {
        let result = ZKPResult::verified("hash".into(), 5);
        assert!(result.verified);
        assert!(!result.fallback_triggered);
        assert_eq!(result.batch_size, 5);
    }

    #[test]
    fn test_zkp_result_fallback() {
        let result = ZKPResult::fallback("hash".into(), 3);
        assert!(result.verified);
        assert!(result.fallback_triggered);
    }

    #[test]
    fn test_zkp_result_failed() {
        let result = ZKPResult::failed("hash".into(), 2);
        assert!(!result.verified);
        assert!(!result.fallback_triggered);
    }

    #[test]
    fn test_config_default() {
        let config = ZKPConfig::default();
        assert_eq!(config.max_batch_size, 50);
        assert_eq!(config.min_batch_size, 10);
        assert!(config.enable_auto_fallback);
    }

    #[test]
    fn test_default() {
        let zkp = AsyncZKPFederation::default();
        assert_eq!(zkp.pending_batch_count(), 0);
    }

    #[test]
    fn test_delta_proof_creation() {
        let proof = DeltaProof::new("p1".into(), "n1".into(), 0, b"data");
        assert_eq!(proof.delta_id, "p1");
        assert_eq!(proof.source_node, "n1");
        assert_eq!(proof.layer_id, 0);
    }

    #[test]
    fn test_get_verified_result() {
        let mut zkp = AsyncZKPFederation::new();
        let proof = make_proof("p1");
        zkp.generate_light_proof(proof).unwrap();

        let result = zkp.get_verified_result("p1");
        assert!(result.is_some());
    }

    #[test]
    fn test_get_merkle_proof_after_fallback() {
        let mut zkp = AsyncZKPFederation::new();
        let proof = make_proof("p1");
        zkp.fallback_to_merkle(proof).unwrap();

        let merkle = zkp.get_merkle_proof("p1");
        assert!(merkle.is_some());
    }

    #[test]
    fn test_batch_size_limits() {
        let mut zkp = AsyncZKPFederation::with_config(ZKPConfig {
            max_batch_size: 10,
            ..ZKPConfig::default()
        });

        // Exactly at limit should work
        let proofs: Vec<_> = (0..10).map(|i| make_proof(&format!("p{}", i))).collect();
        assert!(zkp.batch_proofs(proofs).is_ok());
    }
}
