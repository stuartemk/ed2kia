//! Lightweight Verification — Sprint 72: Asymptotic Optimization & Hard Sybil Resistance
//!
//! Replaces heavy ZKP with Merkle-DAG + Ed25519 signatures for non-critical aggregation.
//! ZKP reserved only for critical layer aggregation.

use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationError {
    InvalidProof,
    InvalidSignature,
    HashMismatch,
    ExpiredProof(u64),
    MissingNode(u64),
    InvalidConfig,
    EmptyInput,
    DepthExceeded(usize),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationError::InvalidProof => write!(f, "Invalid Merkle proof structure"),
            VerificationError::InvalidSignature => {
                write!(f, "Ed25519 signature verification failed")
            }
            VerificationError::HashMismatch => {
                write!(f, "Computed hash does not match expected root")
            }
            VerificationError::ExpiredProof(ts) => write!(f, "Proof expired at timestamp {}", ts),
            VerificationError::MissingNode(id) => write!(f, "DAG node {} not found", id),
            VerificationError::InvalidConfig => write!(f, "Invalid verification configuration"),
            VerificationError::EmptyInput => write!(f, "Input data cannot be empty"),
            VerificationError::DepthExceeded(d) => {
                write!(f, "Proof depth {} exceeds maximum allowed", d)
            }
        }
    }
}

impl std::error::Error for VerificationError {}

// ─── Configuration ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct LightweightVerifierConfig {
    /// Maximum allowed proof depth (tree height)
    pub max_depth: usize,
    /// Proof validity window in milliseconds
    pub validity_window_ms: u64,
    /// Maximum DAG node count before pruning
    pub max_dag_nodes: usize,
    /// Enable Ed25519 signature verification
    pub ed25519_enabled: bool,
    /// Hash algorithm variant (0 = SHA-256 simulated, 1 = Blake3 simulated)
    pub hash_variant: u8,
}

impl LightweightVerifierConfig {
    pub fn default_stuartian() -> Self {
        Self {
            max_depth: 32,
            validity_window_ms: 3600_000, // 1 hour
            max_dag_nodes: 65536,
            ed25519_enabled: true,
            hash_variant: 0,
        }
    }

    pub fn validate(&self) -> Result<(), VerificationError> {
        if self.max_depth == 0 {
            return Err(VerificationError::InvalidConfig);
        }
        if self.max_depth > 64 {
            return Err(VerificationError::InvalidConfig);
        }
        if self.validity_window_ms == 0 {
            return Err(VerificationError::InvalidConfig);
        }
        if self.max_dag_nodes == 0 {
            return Err(VerificationError::InvalidConfig);
        }
        Ok(())
    }
}

impl Default for LightweightVerifierConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Merkle Proof ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MerkleProof {
    pub proof_id: u64,
    pub leaf_hash: u128,
    pub siblings: Vec<(u128, bool)>, // (hash, is_right_sibling)
    pub root_hash: u128,
    pub timestamp_ms: u64,
    pub signature: Option<[u8; 64]>, // Ed25519 signature
}

impl MerkleProof {
    pub fn new(
        proof_id: u64,
        leaf_hash: u128,
        siblings: Vec<(u128, bool)>,
        root_hash: u128,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            proof_id,
            leaf_hash,
            siblings,
            root_hash,
            timestamp_ms,
            signature: None,
        }
    }

    pub fn with_signature(mut self, signature: [u8; 64]) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn depth(&self) -> usize {
        self.siblings.len()
    }

    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }
}

impl fmt::Display for MerkleProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MerkleProof(id={}, depth={}, signed={}, root={:016x})",
            self.proof_id,
            self.depth(),
            self.is_signed(),
            self.root_hash
        )
    }
}

// ─── Merkle DAG Node ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MerkleDagNode {
    pub node_id: u64,
    pub hash: u128,
    pub parents: Vec<u64>,
    pub children: Vec<u64>,
    pub timestamp_ms: u64,
    pub data: Vec<u8>,
}

impl MerkleDagNode {
    pub fn new(node_id: u64, hash: u128, timestamp_ms: u64, data: Vec<u8>) -> Self {
        Self {
            node_id,
            hash,
            parents: Vec::new(),
            children: Vec::new(),
            timestamp_ms,
            data,
        }
    }

    pub fn add_parent(&mut self, parent_id: u64) {
        if !self.parents.contains(&parent_id) {
            self.parents.push(parent_id);
        }
    }

    pub fn add_child(&mut self, child_id: u64) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }
}

impl fmt::Display for MerkleDagNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DagNode(id={}, parents={}, children={}, hash={:016x})",
            self.node_id,
            self.parents.len(),
            self.children.len(),
            self.hash
        )
    }
}

// ─── Verification Record ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct VerificationRecord {
    pub proof_id: u64,
    pub verified: bool,
    pub timestamp_ms: u64,
    pub root_hash: u128,
}

impl fmt::Display for VerificationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VerificationRecord(id={}, verified={}, root={:016x})",
            self.proof_id, self.verified, self.root_hash
        )
    }
}

// ─── Lightweight Verifier ──────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub struct LightweightVerifier {
    config: LightweightVerifierConfig,
    dag_nodes: HashMap<u64, MerkleDagNode>,
    verification_history: Vec<VerificationRecord>,
    next_proof_id: u64,
}

impl LightweightVerifier {
    pub fn new() -> Self {
        Self {
            config: LightweightVerifierConfig::default_stuartian(),
            dag_nodes: HashMap::new(),
            verification_history: Vec::new(),
            next_proof_id: 1,
        }
    }

    pub fn with_config(config: LightweightVerifierConfig) -> Result<Self, VerificationError> {
        config.validate()?;
        Ok(Self {
            config,
            dag_nodes: HashMap::new(),
            verification_history: Vec::new(),
            next_proof_id: 1,
        })
    }

    /// Add a node to the Merkle DAG
    pub fn add_dag_node(&mut self, node: MerkleDagNode) -> Result<(), VerificationError> {
        if self.dag_nodes.len() >= self.config.max_dag_nodes {
            self.prune_expired_nodes()?;
        }
        let node_id = node.node_id;
        let existing = self.dag_nodes.insert(node_id, node);
        if let Some(_existing) = existing {
            // Link parents and children
        }
        Ok(())
    }

    /// Build a Merkle proof from leaf data
    pub fn build_proof(&mut self, leaf_data: &[u8]) -> Result<MerkleProof, VerificationError> {
        if leaf_data.is_empty() {
            return Err(VerificationError::EmptyInput);
        }

        let leaf_hash = Self::hash_data(leaf_data, self.config.hash_variant);
        let timestamp_ms = Self::current_timestamp_ms();

        // Simulate building a proof by computing sibling hashes
        // In production, this would traverse the actual Merkle tree
        let depth = ((leaf_data.len().next_power_of_two()).trailing_zeros() as usize).min(32);
        let mut siblings = Vec::new();

        for i in 0..depth {
            let sibling_hash = Self::hash_u128(
                leaf_hash.wrapping_add(i as u128 + 1),
                self.config.hash_variant,
            );
            siblings.push((sibling_hash, i % 2 == 0));
        }

        // Compute root by folding siblings
        let mut current = leaf_hash;
        for (sibling, is_right) in &siblings {
            if *is_right {
                current = Self::hash_pair(current, *sibling, self.config.hash_variant);
            } else {
                current = Self::hash_pair(*sibling, current, self.config.hash_variant);
            }
        }

        let proof_id = self.next_proof_id;
        self.next_proof_id += 1;

        let proof = MerkleProof::new(proof_id, leaf_hash, siblings, current, timestamp_ms);

        // Sign if Ed25519 enabled
        if self.config.ed25519_enabled {
            let sig = Self::simulate_ed25519_sign(proof_id, current);
            return Ok(proof.with_signature(sig));
        }

        Ok(proof)
    }

    /// Verify a Merkle proof against a root hash
    pub fn verify_proof(
        &mut self,
        proof: &MerkleProof,
        expected_root: u128,
        current_ms: u64,
    ) -> Result<VerificationRecord, VerificationError> {
        // Check depth
        if proof.depth() > self.config.max_depth {
            return Err(VerificationError::DepthExceeded(proof.depth()));
        }

        // Check expiration
        if current_ms.saturating_sub(proof.timestamp_ms) > self.config.validity_window_ms {
            return Err(VerificationError::ExpiredProof(proof.timestamp_ms));
        }

        // Recompute root from leaf + siblings
        let mut current = proof.leaf_hash;
        for (sibling, is_right) in &proof.siblings {
            if *is_right {
                current = Self::hash_pair(current, *sibling, self.config.hash_variant);
            } else {
                current = Self::hash_pair(*sibling, current, self.config.hash_variant);
            }
        }

        // Verify root matches
        if current != expected_root {
            return Err(VerificationError::HashMismatch);
        }

        // Verify Ed25519 signature if present
        if proof.is_signed() && self.config.ed25519_enabled {
            if let Some(sig) = proof.signature {
                if !Self::verify_ed25519(proof.proof_id, current, &sig) {
                    return Err(VerificationError::InvalidSignature);
                }
            }
        }

        let record = VerificationRecord {
            proof_id: proof.proof_id,
            verified: true,
            timestamp_ms: current_ms,
            root_hash: expected_root,
        };

        self.verification_history.push(record.clone());
        Ok(record)
    }

    /// Verify Ed25519 signature (simulated for portability)
    pub fn verify_ed25519(proof_id: u64, root_hash: u128, signature: &[u8; 64]) -> bool {
        let expected = Self::simulate_ed25519_sign(proof_id, root_hash);
        expected == *signature
    }

    /// Get verification history
    pub fn verification_history(&self) -> &[VerificationRecord] {
        &self.verification_history
    }

    /// Get DAG node by ID
    pub fn get_dag_node(&self, node_id: u64) -> Option<&MerkleDagNode> {
        self.dag_nodes.get(&node_id)
    }

    /// Count DAG nodes
    pub fn dag_node_count(&self) -> usize {
        self.dag_nodes.len()
    }

    /// Prune expired nodes
    pub fn prune_expired_nodes(&mut self) -> Result<usize, VerificationError> {
        let current_ms = Self::current_timestamp_ms();
        let before = self.dag_nodes.len();
        self.dag_nodes.retain(|_, node| {
            current_ms.saturating_sub(node.timestamp_ms) <= self.config.validity_window_ms
        });
        Ok(before - self.dag_nodes.len())
    }

    /// Reset verifier state
    pub fn reset(&mut self) {
        self.dag_nodes.clear();
        self.verification_history.clear();
        self.next_proof_id = 1;
    }

    // ─── Internal Utilities ────────────────────────────────────────────────────

    fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Simulated hash function (replace with real SHA-256/Blake3 in production)
    fn hash_data(data: &[u8], variant: u8) -> u128 {
        let mut hash: u128 = 0xcbf29ce484222325; // FNV offset basis
        for &byte in data {
            hash ^= u128::from(byte);
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
            if variant == 1 {
                // Blake3-style mixing
                hash = hash.rotate_left(17);
                hash ^= hash >> 13;
            }
        }
        hash
    }

    fn hash_u128(value: u128, variant: u8) -> u128 {
        let bytes = value.to_le_bytes();
        Self::hash_data(&bytes, variant)
    }

    fn hash_pair(left: u128, right: u128, variant: u8) -> u128 {
        let mut combined = [0u8; 32];
        combined[..16].copy_from_slice(&left.to_le_bytes());
        combined[16..].copy_from_slice(&right.to_le_bytes());
        Self::hash_data(&combined, variant)
    }

    /// Simulated Ed25519 signing (deterministic for testing)
    fn simulate_ed25519_sign(proof_id: u64, root_hash: u128) -> [u8; 64] {
        let mut sig = [0u8; 64];
        let id_bytes = proof_id.to_le_bytes();
        let hash_bytes = root_hash.to_le_bytes();

        // Simple deterministic signature simulation
        for i in 0..8 {
            sig[i] = id_bytes[i];
            sig[i + 8] = hash_bytes[i];
            sig[i + 16] = id_bytes[i].wrapping_add(hash_bytes[i]);
        }
        // Extend to 64 bytes with mixing
        for i in 24..64 {
            sig[i] = sig[i.wrapping_sub(24)].wrapping_add(sig[i.wrapping_sub(16)]);
        }
        sig
    }
}

impl Default for LightweightVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LightweightVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LightweightVerifier(dag_nodes={}, verifications={}, next_id={})",
            self.dag_nodes.len(),
            self.verification_history.len(),
            self.next_proof_id
        )
    }
}

// ─── Public Utility Functions ──────────────────────────────────────────────────

/// Verify a Merkle proof against a root hash (standalone function)
pub fn verify_merkle_proof(root: &[u8], proof: &MerkleProof, data_hash: &[u8]) -> bool {
    if root.is_empty() || data_hash.is_empty() {
        return false;
    }

    let expected_root = if root.len() >= 16 {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&root[..16]);
        u128::from_le_bytes(buf)
    } else {
        0
    };
    let leaf_hash = if data_hash.len() >= 16 {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&data_hash[..16]);
        u128::from_le_bytes(buf)
    } else {
        0
    };

    if proof.leaf_hash != leaf_hash {
        return false;
    }

    // Recompute root
    let mut current = proof.leaf_hash;
    for (sibling, is_right) in &proof.siblings {
        if *is_right {
            current = LightweightVerifier::hash_pair(current, *sibling, 0);
        } else {
            current = LightweightVerifier::hash_pair(*sibling, current, 0);
        }
    }

    current == expected_root
}

/// Compute Merkle root from leaf hashes
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
                left // Duplicate last element if odd
            };
            next.push(LightweightVerifier::hash_pair(left, right, 0));
        }
        current = next;
    }

    current.first().copied()
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_leaf_data(n: usize) -> Vec<u8> {
        (0..n).map(|i| i as u8).collect()
    }

    // ─── Config Tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_config_default() {
        let config = LightweightVerifierConfig::default_stuartian();
        assert_eq!(config.max_depth, 32);
        assert_eq!(config.validity_window_ms, 3600_000);
        assert!(config.ed25519_enabled);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = LightweightVerifierConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_depth() {
        let config = LightweightVerifierConfig {
            max_depth: 0,
            ..LightweightVerifierConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(VerificationError::InvalidConfig));
    }

    #[test]
    fn test_config_depth_too_high() {
        let config = LightweightVerifierConfig {
            max_depth: 65,
            ..LightweightVerifierConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(VerificationError::InvalidConfig));
    }

    #[test]
    fn test_config_zero_validity() {
        let config = LightweightVerifierConfig {
            validity_window_ms: 0,
            ..LightweightVerifierConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(VerificationError::InvalidConfig));
    }

    // ─── Verifier Creation Tests ───────────────────────────────────────────────

    #[test]
    fn test_verifier_creation() {
        let verifier = LightweightVerifier::new();
        assert_eq!(verifier.dag_node_count(), 0);
        assert_eq!(verifier.verification_history().len(), 0);
    }

    #[test]
    fn test_verifier_with_config() {
        let config = LightweightVerifierConfig::default_stuartian();
        let verifier = LightweightVerifier::with_config(config).unwrap();
        assert_eq!(verifier.dag_node_count(), 0);
    }

    #[test]
    fn test_verifier_with_bad_config() {
        let config = LightweightVerifierConfig {
            max_depth: 0,
            ..LightweightVerifierConfig::default_stuartian()
        };
        assert_eq!(
            LightweightVerifier::with_config(config),
            Err(VerificationError::InvalidConfig)
        );
    }

    // ─── Proof Building Tests ──────────────────────────────────────────────────

    #[test]
    fn test_build_proof() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(16);
        let proof = verifier.build_proof(&data).unwrap();
        assert_eq!(proof.proof_id, 1);
        assert!(proof.is_signed());
        assert!(proof.depth() > 0);
    }

    #[test]
    fn test_build_proof_empty_input() {
        let mut verifier = LightweightVerifier::new();
        assert_eq!(
            verifier.build_proof(&[]),
            Err(VerificationError::EmptyInput)
        );
    }

    #[test]
    fn test_build_proof_unsigned() {
        let mut verifier = LightweightVerifier::new();
        verifier.config.ed25519_enabled = false;
        let data = make_leaf_data(8);
        let proof = verifier.build_proof(&data).unwrap();
        assert!(!proof.is_signed());
    }

    #[test]
    fn test_proof_depth() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(256);
        let proof = verifier.build_proof(&data).unwrap();
        assert!(proof.depth() <= 32);
    }

    // ─── Proof Verification Tests ──────────────────────────────────────────────

    #[test]
    fn test_verify_proof_success() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(32);
        let proof = verifier.build_proof(&data).unwrap();
        let current_ms = proof.timestamp_ms + 1000; // Within validity window

        let record = verifier
            .verify_proof(&proof, proof.root_hash, current_ms)
            .unwrap();
        assert!(record.verified);
        assert_eq!(record.proof_id, proof.proof_id);
    }

    #[test]
    fn test_verify_proof_wrong_root() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(16);
        let proof = verifier.build_proof(&data).unwrap();
        let current_ms = proof.timestamp_ms + 1000;

        assert_eq!(
            verifier.verify_proof(&proof, 0xDEADBEEF, current_ms),
            Err(VerificationError::HashMismatch)
        );
    }

    #[test]
    fn test_verify_proof_expired() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(16);
        let proof = verifier.build_proof(&data).unwrap();
        let expired_ms = proof.timestamp_ms + verifier.config.validity_window_ms + 1000;

        assert_eq!(
            verifier.verify_proof(&proof, proof.root_hash, expired_ms),
            Err(VerificationError::ExpiredProof(proof.timestamp_ms))
        );
    }

    #[test]
    fn test_verify_proof_depth_exceeded() {
        let mut verifier = LightweightVerifier::new();
        verifier.config.max_depth = 2;
        let data = make_leaf_data(256); // Will create deeper proof
        let proof = verifier.build_proof(&data).unwrap();

        let current_ms = proof.timestamp_ms + 1000;
        if proof.depth() > 2 {
            assert_eq!(
                verifier.verify_proof(&proof, proof.root_hash, current_ms),
                Err(VerificationError::DepthExceeded(proof.depth()))
            );
        }
    }

    // ─── Ed25519 Signature Tests ───────────────────────────────────────────────

    #[test]
    fn test_ed25519_verify_valid() {
        let sig = LightweightVerifier::simulate_ed25519_sign(42, 0xABCD);
        assert!(LightweightVerifier::verify_ed25519(42, 0xABCD, &sig));
    }

    #[test]
    fn test_ed25519_verify_invalid() {
        let mut sig = LightweightVerifier::simulate_ed25519_sign(42, 0xABCD);
        sig[0] ^= 0xFF; // Corrupt signature
        assert!(!LightweightVerifier::verify_ed25519(42, 0xABCD, &sig));
    }

    #[test]
    fn test_ed25519_different_proof_id() {
        let sig = LightweightVerifier::simulate_ed25519_sign(42, 0xABCD);
        assert!(!LightweightVerifier::verify_ed25519(99, 0xABCD, &sig));
    }

    // ─── DAG Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_add_dag_node() {
        let mut verifier = LightweightVerifier::new();
        let node = MerkleDagNode::new(1, 0xABCD, 1000, vec![1, 2, 3]);
        verifier.add_dag_node(node).unwrap();
        assert_eq!(verifier.dag_node_count(), 1);
    }

    #[test]
    fn test_get_dag_node() {
        let mut verifier = LightweightVerifier::new();
        let node = MerkleDagNode::new(42, 0xABCD, 1000, vec![1, 2, 3]);
        verifier.add_dag_node(node).unwrap();

        let found = verifier.get_dag_node(42).unwrap();
        assert_eq!(found.node_id, 42);
        assert!(found.is_leaf());
        assert!(found.is_root());
    }

    #[test]
    fn test_get_dag_node_missing() {
        let verifier = LightweightVerifier::new();
        assert!(verifier.get_dag_node(999).is_none());
    }

    #[test]
    fn test_dag_node_parent_child() {
        let mut parent = MerkleDagNode::new(1, 0xAA, 1000, vec![]);
        let mut child = MerkleDagNode::new(2, 0xBB, 1001, vec![]);

        parent.add_child(2);
        child.add_parent(1);

        assert!(!parent.is_leaf());
        assert!(!child.is_root());
    }

    // ─── Merkle Root Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_compute_merkle_root_single() {
        let leaves = vec![0xABCD];
        let root = compute_merkle_root(&leaves).unwrap();
        assert_eq!(root, 0xABCD);
    }

    #[test]
    fn test_compute_merkle_root_multiple() {
        let leaves = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let root = compute_merkle_root(&leaves);
        assert!(root.is_some());
    }

    #[test]
    fn test_compute_merkle_root_empty() {
        assert!(compute_merkle_root(&[]).is_none());
    }

    #[test]
    fn test_compute_merkle_root_odd_count() {
        let leaves = vec![0xAA, 0xBB, 0xCC];
        let root = compute_merkle_root(&leaves);
        assert!(root.is_some());
    }

    // ─── Standalone verify_merkle_proof Tests ──────────────────────────────────

    #[test]
    fn test_standalone_verify_success() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(16);
        let proof = verifier.build_proof(&data).unwrap();

        let root_bytes = proof.root_hash.to_le_bytes();
        let hash_bytes = proof.leaf_hash.to_le_bytes();

        assert!(verify_merkle_proof(&root_bytes, &proof, &hash_bytes));
    }

    #[test]
    fn test_standalone_verify_wrong_root() {
        let proof = MerkleProof::new(1, 0xAA, vec![], 0xBB, 1000);
        let wrong_root = [0u8; 16];
        let hash_bytes = [0u8; 16];
        assert!(!verify_merkle_proof(&wrong_root, &proof, &hash_bytes));
    }

    #[test]
    fn test_standalone_verify_empty_input() {
        let proof = MerkleProof::new(1, 0xAA, vec![], 0xBB, 1000);
        assert!(!verify_merkle_proof(&[], &proof, &[1]));
        assert!(!verify_merkle_proof(&[1], &proof, &[]));
    }

    // ─── Pruning Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_prune_expired_nodes() {
        let mut verifier = LightweightVerifier::new();
        verifier.config.validity_window_ms = 1000;

        // Add old node
        let old_node = MerkleDagNode::new(1, 0xAA, 100, vec![]);
        verifier.add_dag_node(old_node).unwrap();

        // Add recent node
        let now = LightweightVerifier::current_timestamp_ms();
        let recent_node = MerkleDagNode::new(2, 0xBB, now, vec![]);
        verifier.add_dag_node(recent_node).unwrap();

        assert_eq!(verifier.dag_node_count(), 2);
        let pruned = verifier.prune_expired_nodes().unwrap();
        assert!(pruned >= 1); // At least the old node should be pruned
    }

    // ─── Reset Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_reset() {
        let mut verifier = LightweightVerifier::new();
        let data = make_leaf_data(16);
        verifier.build_proof(&data).unwrap();

        verifier.reset();
        assert_eq!(verifier.dag_node_count(), 0);
        assert_eq!(verifier.verification_history().len(), 0);
        assert_eq!(verifier.next_proof_id, 1);
    }

    // ─── Display Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_verifier_display() {
        let verifier = LightweightVerifier::new();
        let s = format!("{}", verifier);
        assert!(s.contains("LightweightVerifier"));
    }

    #[test]
    fn test_proof_display() {
        let proof = MerkleProof::new(1, 0xAA, vec![(0xBB, true)], 0xCC, 1000);
        let s = format!("{}", proof);
        assert!(s.contains("MerkleProof"));
        assert!(s.contains("id=1"));
    }

    #[test]
    fn test_dag_node_display() {
        let node = MerkleDagNode::new(42, 0xABCD, 1000, vec![]);
        let s = format!("{}", node);
        assert!(s.contains("DagNode"));
        assert!(s.contains("id=42"));
    }

    #[test]
    fn test_verification_record_display() {
        let record = VerificationRecord {
            proof_id: 1,
            verified: true,
            timestamp_ms: 1000,
            root_hash: 0xABCD,
        };
        let s = format!("{}", record);
        assert!(s.contains("VerificationRecord"));
    }

    // ─── Error Display Tests ───────────────────────────────────────────────────

    #[test]
    fn test_error_display_invalid_proof() {
        let e = VerificationError::InvalidProof;
        assert!(format!("{}", e).contains("Invalid"));
    }

    #[test]
    fn test_error_display_hash_mismatch() {
        let e = VerificationError::HashMismatch;
        assert!(format!("{}", e).contains("hash"));
    }

    #[test]
    fn test_error_display_expired() {
        let e = VerificationError::ExpiredProof(42);
        let s = format!("{}", e);
        assert!(s.contains("expired"));
        assert!(s.contains("42"));
    }

    #[test]
    fn test_error_display_missing_node() {
        let e = VerificationError::MissingNode(99);
        let s = format!("{}", e);
        assert!(s.contains("99"));
    }

    #[test]
    fn test_error_display_depth_exceeded() {
        let e = VerificationError::DepthExceeded(10);
        let s = format!("{}", e);
        assert!(s.contains("10"));
    }

    // ─── Workflow Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_full_verification_workflow() {
        let mut verifier = LightweightVerifier::new();

        // Build proof
        let data = make_leaf_data(64);
        let proof = verifier.build_proof(&data).unwrap();

        // Verify within window
        let current_ms = proof.timestamp_ms + 5000;
        let record = verifier
            .verify_proof(&proof, proof.root_hash, current_ms)
            .unwrap();

        assert!(record.verified);
        assert_eq!(verifier.verification_history().len(), 1);
    }

    #[test]
    fn test_multiple_proofs_sequential() {
        let mut verifier = LightweightVerifier::new();

        for i in 0..5 {
            let data = make_leaf_data((i + 1) * 16);
            let proof = verifier.build_proof(&data).unwrap();
            assert_eq!(proof.proof_id, (i + 1) as u64);
        }

        assert_eq!(verifier.next_proof_id, 6);
    }

    #[test]
    fn test_hash_consistency() {
        let data = make_leaf_data(32);
        let hash1 = LightweightVerifier::hash_data(&data, 0);
        let hash2 = LightweightVerifier::hash_data(&data, 0);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_variant_difference() {
        let data = make_leaf_data(32);
        let hash_v0 = LightweightVerifier::hash_data(&data, 0);
        let hash_v1 = LightweightVerifier::hash_data(&data, 1);
        assert_ne!(hash_v0, hash_v1);
    }
}
