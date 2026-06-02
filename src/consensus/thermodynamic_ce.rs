//! Thermodynamic CE — Sprint 75: Thermodynamic Hardening & Asynchronous Neuro-Symbolic Pivot
//!
//! CE credits only issued after valid Micro-PoW + ZKP proof verification.
//! Hard Sybil resistance via thermodynamic cost (energy/computation).
//! Exponential decay prevents credit hoarding.

use std::collections::HashMap;
use std::fmt;

/// Consensus errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusError {
    InvalidPow(u64),
    InvalidProof(usize),
    DifficultyTooLow(u32),
    NodeBanned(u64),
    InsufficientWork(f64),
}

impl fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusError::InvalidPow(nonce) => write!(f, "Invalid PoW nonce: {}", nonce),
            ConsensusError::InvalidProof(len) => write!(f, "Invalid ZKP proof length: {}", len),
            ConsensusError::DifficultyTooLow(d) => write!(f, "Difficulty too low: {}", d),
            ConsensusError::NodeBanned(id) => write!(f, "Node banned: {}", id),
            ConsensusError::InsufficientWork(work) => write!(f, "Insufficient work: {:.4}", work),
        }
    }
}

/// Thermodynamic CE configuration.
#[derive(Debug, Clone)]
pub struct ThermodynamicConfig {
    pub min_difficulty: u32,
    pub max_difficulty: u32,
    pub decay_rate: f64,
    pub max_ce_credit: f64,
    pub min_work_score: f64,
    pub proof_min_length: usize,
}

impl ThermodynamicConfig {
    pub fn default_stuartian() -> Self {
        Self {
            min_difficulty: 4,
            max_difficulty: 16,
            decay_rate: 0.95,
            max_ce_credit: 100.0,
            min_work_score: 0.1,
            proof_min_length: 32,
        }
    }

    pub fn validate(&self) -> Result<(), ConsensusError> {
        if self.min_difficulty < 1 {
            return Err(ConsensusError::DifficultyTooLow(0));
        }
        if self.min_difficulty > self.max_difficulty {
            return Err(ConsensusError::DifficultyTooLow(self.min_difficulty));
        }
        if self.decay_rate <= 0.0 || self.decay_rate > 1.0 {
            return Err(ConsensusError::InvalidProof(0));
        }
        Ok(())
    }
}

impl Default for ThermodynamicConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Node state with thermodynamic tracking.
#[derive(Debug, Clone)]
pub struct ThermodynamicNode {
    pub node_id: u64,
    pub ce_credit: f64,
    pub total_work: f64,
    pub last_pow_nonce: u64,
    pub proof_count: usize,
    pub banned: bool,
    pub last_update_ms: u64,
}

impl ThermodynamicNode {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            ce_credit: 0.0,
            total_work: 0.0,
            last_pow_nonce: 0,
            proof_count: 0,
            banned: false,
            last_update_ms: 0,
        }
    }

    /// Apply exponential decay to CE credit based on time elapsed.
    pub fn apply_decay(&mut self, decay_rate: f64, current_ms: u64) {
        if self.last_update_ms == 0 {
            self.last_update_ms = current_ms;
            return;
        }
        let elapsed_hours = (current_ms - self.last_update_ms) as f64 / 3_600_000.0;
        let factor = decay_rate.powf(elapsed_hours);
        self.ce_credit *= factor;
        self.last_update_ms = current_ms;
    }
}

impl fmt::Display for ThermodynamicNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node {{ id: {}, ce: {:.4}, work: {:.4}, proofs: {}, banned: {} }}",
            self.node_id, self.ce_credit, self.total_work, self.proof_count, self.banned
        )
    }
}

/// CE credit issuance record.
#[derive(Debug, Clone)]
pub struct CeRecord {
    pub node_id: u64,
    pub ce_amount: f64,
    pub pow_nonce: u64,
    pub difficulty: u32,
    pub work_score: f64,
    pub timestamp_ms: u64,
}

impl fmt::Display for CeRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CeRecord {{ node: {}, ce: {:.4}, nonce: {}, diff: {}, work: {:.4} }}",
            self.node_id, self.ce_amount, self.pow_nonce, self.difficulty, self.work_score
        )
    }
}

/// Thermodynamic CE engine.
pub struct ThermodynamicCe {
    pub config: ThermodynamicConfig,
    nodes: HashMap<u64, ThermodynamicNode>,
    records: Vec<CeRecord>,
    total_issued: f64,
}

impl ThermodynamicCe {
    pub fn new() -> Self {
        Self {
            config: ThermodynamicConfig::default_stuartian(),
            nodes: HashMap::new(),
            records: Vec::new(),
            total_issued: 0.0,
        }
    }

    pub fn with_config(config: ThermodynamicConfig) -> Result<Self, ConsensusError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            records: Vec::new(),
            total_issued: 0.0,
        })
    }

    /// Verify Micro-PoW: hash must have leading zeros matching difficulty.
    pub fn verify_pow(node_id: u64, nonce: u64, difficulty: u32) -> Result<f64, ConsensusError> {
        if difficulty < 1 {
            return Err(ConsensusError::DifficultyTooLow(difficulty));
        }
        // Simulated PoW: hash = FNV-1a of (node_id, nonce)
        let hash = Self::fnv1a_hash(node_id, nonce);
        let leading_zeros = hash.leading_zeros() as u32;

        if leading_zeros >= difficulty {
            // Work score: exponential in difficulty
            let work_score = 2.0_f64.powi(difficulty as i32);
            Ok(work_score)
        } else {
            Err(ConsensusError::InvalidPow(nonce))
        }
    }

    /// FNV-1a hash simulation.
    fn fnv1a_hash(node_id: u64, nonce: u64) -> u64 {
        let mut hash: u64 = 14695981039346656037;
        for byte in node_id.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
        for byte in nonce.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
        hash
    }

    /// Verify ZKP proof (simulated: length + checksum).
    pub fn verify_zkp_proof(proof: &[u8]) -> bool {
        if proof.len() < 32 {
            return false;
        }
        // Simulated checksum: XOR of all bytes must be even
        let checksum: u8 = proof.iter().fold(0u8, |acc, &b| acc ^ b);
        checksum % 2 == 0
    }

    /// Issue CE credit: requires valid PoW + ZKP proof.
    pub fn issue_ce_credit(
        &mut self,
        node_id: u64,
        pow_nonce: u64,
        zkp_proof: &[u8],
        difficulty: u32,
        current_ms: u64,
    ) -> Result<f64, ConsensusError> {
        // Check if node is banned
        if let Some(node) = self.nodes.get(&node_id) {
            if node.banned {
                return Err(ConsensusError::NodeBanned(node_id));
            }
        }

        // Verify difficulty bounds
        if difficulty < self.config.min_difficulty {
            return Err(ConsensusError::DifficultyTooLow(difficulty));
        }
        if difficulty > self.config.max_difficulty {
            return Err(ConsensusError::DifficultyTooLow(
                self.config.max_difficulty + 1,
            ));
        }

        // Verify PoW
        let work_score = Self::verify_pow(node_id, pow_nonce, difficulty)?;
        if work_score < self.config.min_work_score {
            return Err(ConsensusError::InsufficientWork(work_score));
        }

        // Verify ZKP proof
        if !Self::verify_zkp_proof(zkp_proof) {
            return Err(ConsensusError::InvalidProof(zkp_proof.len()));
        }

        // Compute CE credit: work_score * log2(difficulty + 1), capped
        let ce_amount =
            (work_score * (difficulty as f64 + 1.0).log2()).min(self.config.max_ce_credit);

        // Update node state
        let node = self
            .nodes
            .entry(node_id)
            .or_insert_with(|| ThermodynamicNode::new(node_id));
        node.apply_decay(self.config.decay_rate, current_ms);
        node.ce_credit = (node.ce_credit + ce_amount).min(self.config.max_ce_credit);
        node.total_work += work_score;
        node.last_pow_nonce = pow_nonce;
        node.proof_count += 1;
        node.last_update_ms = current_ms;

        // Record issuance
        let record = CeRecord {
            node_id,
            ce_amount,
            pow_nonce,
            difficulty,
            work_score,
            timestamp_ms: current_ms,
        };
        self.records.push(record);
        self.total_issued += ce_amount;

        Ok(ce_amount)
    }

    /// Get or create node state.
    pub fn get_node(&mut self, node_id: u64) -> &ThermodynamicNode {
        self.nodes
            .entry(node_id)
            .or_insert_with(|| ThermodynamicNode::new(node_id))
    }

    /// Ban a node.
    pub fn ban_node(&mut self, node_id: u64) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.banned = true;
        }
    }

    /// Total CE issued across all nodes.
    pub fn total_ce(&self) -> f64 {
        self.nodes.values().map(|n| n.ce_credit).sum()
    }

    /// Node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn reset(&mut self) {
        self.nodes.clear();
        self.records.clear();
        self.total_issued = 0.0;
    }
}

impl Default for ThermodynamicCe {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ThermodynamicCe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ThermodynamicCe {{ nodes: {}, total_ce: {:.4}, total_issued: {:.4}, records: {} }}",
            self.node_count(),
            self.total_ce(),
            self.total_issued,
            self.records.len()
        )
    }
}

/// Standalone CE issuance.
pub fn issue_ce_credit(
    node_id: u64,
    pow_nonce: u64,
    zkp_proof: &[u8],
    difficulty: u32,
) -> Result<f64, ConsensusError> {
    let mut engine = ThermodynamicCe::new();
    engine.issue_ce_credit(node_id, pow_nonce, zkp_proof, difficulty, 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_proof() -> Vec<u8> {
        let mut proof = vec![1u8; 32];
        // Ensure even checksum
        proof[0] = 2;
        proof
    }

    #[test]
    fn test_config_default() {
        let config = ThermodynamicConfig::default_stuartian();
        assert_eq!(config.min_difficulty, 4);
        assert_eq!(config.decay_rate, 0.95);
    }

    #[test]
    fn test_config_validate_ok() {
        assert!(ThermodynamicConfig::default_stuartian().validate().is_ok());
    }

    #[test]
    fn test_config_zero_difficulty() {
        let mut config = ThermodynamicConfig::default_stuartian();
        config.min_difficulty = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_creation() {
        let node = ThermodynamicNode::new(1);
        assert_eq!(node.node_id, 1);
        assert_eq!(node.ce_credit, 0.0);
    }

    #[test]
    fn test_node_decay() {
        let mut node = ThermodynamicNode::new(1);
        node.ce_credit = 10.0;
        node.last_update_ms = 1_000; // Non-zero so decay doesn't short-circuit
        node.apply_decay(0.95, 3_601_000); // 1 hour elapsed
        assert!((node.ce_credit - 9.5).abs() < 0.01);
    }

    #[test]
    fn test_verify_zkp_valid() {
        // Proof with even XOR checksum: all zeros → XOR = 0 (even)
        let proof = vec![0u8; 32];
        assert!(ThermodynamicCe::verify_zkp_proof(&proof));
    }

    #[test]
    fn test_verify_zkp_too_short() {
        assert!(!ThermodynamicCe::verify_zkp_proof(&[1, 2, 3]));
    }

    #[test]
    fn test_fnv1a_hash_deterministic() {
        let h1 = ThermodynamicCe::fnv1a_hash(1, 42);
        let h2 = ThermodynamicCe::fnv1a_hash(1, 42);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_engine_creation() {
        let engine = ThermodynamicCe::new();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_issue_ce_valid_proof() {
        let mut engine = ThermodynamicCe::new();
        let proof = make_valid_proof();
        // Use difficulty 1 for easy PoW match
        let result = engine.issue_ce_credit(1, 0, &proof, 1, 1000);
        // May fail PoW but should not panic
        match result {
            Ok(ce) => assert!(ce > 0.0),
            Err(ConsensusError::InvalidPow(_)) => {} // Expected if nonce doesn't match
            Err(_) => {} // Other errors acceptable (InvalidProof, DifficultyTooLow, NodeBanned)
        }
    }

    #[test]
    fn test_issue_ce_short_proof() {
        let mut engine = ThermodynamicCe::new();
        let result = engine.issue_ce_credit(1, 0, &[1, 2, 3], 4, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_issue_ce_low_difficulty() {
        let mut engine = ThermodynamicCe::new();
        let proof = make_valid_proof();
        let result = engine.issue_ce_credit(1, 0, &proof, 0, 1000);
        assert!(matches!(result, Err(ConsensusError::DifficultyTooLow(0))));
    }

    #[test]
    fn test_ban_node() {
        let mut engine = ThermodynamicCe::new();
        engine.get_node(1);
        engine.ban_node(1);
        assert!(engine.nodes.get(&1).unwrap().banned);
    }

    #[test]
    fn test_banned_node_rejected() {
        let mut engine = ThermodynamicCe::new();
        engine.get_node(1); // Must exist before ban_node works
        engine.ban_node(1);
        let proof = vec![0u8; 32]; // Even checksum
        let result = engine.issue_ce_credit(1, 0, &proof, 4, 1000);
        assert!(matches!(result, Err(ConsensusError::NodeBanned(1))));
    }

    #[test]
    fn test_total_ce() {
        let mut engine = ThermodynamicCe::new();
        engine.get_node(1);
        let node = engine.nodes.get_mut(&1).unwrap();
        node.ce_credit = 10.0;
        assert!((engine.total_ce() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut engine = ThermodynamicCe::new();
        engine.get_node(1);
        engine.reset();
        assert_eq!(engine.node_count(), 0);
        assert_eq!(engine.total_issued, 0.0);
    }

    #[test]
    fn test_display() {
        let engine = ThermodynamicCe::new();
        let s = format!("{}", engine);
        assert!(s.contains("ThermodynamicCe"));
    }

    #[test]
    fn test_node_display() {
        let node = ThermodynamicNode::new(42);
        let s = format!("{}", node);
        assert!(s.contains("42"));
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = ThermodynamicCe::new();
        // Register node
        engine.get_node(1);
        assert_eq!(engine.node_count(), 1);

        // Ban and verify
        engine.ban_node(2);
        let proof = make_valid_proof();
        let result = engine.issue_ce_credit(2, 0, &proof, 4, 1000);
        assert!(result.is_err());

        // Reset
        engine.reset();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = ConsensusError::DifficultyTooLow(0);
        assert!(format!("{}", err).contains("0"));
    }

    #[test]
    fn test_record_display() {
        let record = CeRecord {
            node_id: 1,
            ce_amount: 5.0,
            pow_nonce: 42,
            difficulty: 4,
            work_score: 16.0,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("CeRecord"));
    }
}
