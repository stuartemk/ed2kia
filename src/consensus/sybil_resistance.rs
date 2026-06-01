//! Sybil Resistance — Sprint 72: Asymptotic Optimization & Hard Sybil Resistance
//!
//! Proof-of-Useful-Work (PoUW) + CE decay + diversity weighting.
//! BFT ε-tolerant consensus with geographic/semantic diversity.

use std::collections::HashMap;
use std::fmt;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SybilError {
    InvalidWorkProof,
    ExpiredProof(u64),
    DuplicateSubmission(u64),
    InsufficientDiversity(f64),
    CEBelowThreshold(f64),
    InvalidConfig,
    BFTThresholdNotMet(f64),
    NodeNotFound(u64),
    HashMismatch,
    MaxNodesExceeded(usize),
}

impl fmt::Display for SybilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SybilError::InvalidWorkProof => write!(f, "Invalid Proof-of-Useful-Work"),
            SybilError::ExpiredProof(ts) => write!(f, "Work proof expired at {}", ts),
            SybilError::DuplicateSubmission(id) => {
                write!(f, "Duplicate submission from node {}", id)
            }
            SybilError::InsufficientDiversity(d) => {
                write!(f, "Diversity {} below minimum threshold", d)
            }
            SybilError::CEBelowThreshold(t) => {
                write!(f, "Contribution evidence {} below threshold {}", t, t)
            }
            SybilError::InvalidConfig => write!(f, "Invalid Sybil resistance configuration"),
            SybilError::BFTThresholdNotMet(t) => {
                write!(f, "BFT threshold {} not met", t)
            }
            SybilError::NodeNotFound(id) => write!(f, "Node {} not found", id),
            SybilError::HashMismatch => write!(f, "SAE activation hash mismatch"),
            SybilError::MaxNodesExceeded(n) => write!(f, "Maximum node count {} exceeded", n),
        }
    }
}

impl std::error::Error for SybilError {}

// ─── Configuration ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SybilConfig {
    /// Minimum contribution evidence (CE) threshold
    pub min_ce_threshold: f64,
    /// CE decay rate per tick (exponential)
    pub ce_decay_rate: f64,
    /// Minimum Shannon entropy for geographic diversity
    pub min_diversity_entropy: f64,
    /// BFT epsilon tolerance (0.0 to 1.0)
    pub bft_epsilon: f64,
    /// Maximum proof age in milliseconds
    pub max_proof_age_ms: u64,
    /// Maximum nodes in consensus
    pub max_nodes: usize,
    /// Diversity weight in final score
    pub diversity_weight: f64,
    /// CE weight in final score
    pub ce_weight: f64,
}

impl SybilConfig {
    pub fn default_stuartian() -> Self {
        Self {
            min_ce_threshold: 0.1,
            ce_decay_rate: 0.999,
            min_diversity_entropy: 0.5,
            bft_epsilon: 0.33,         // Tolerate up to 33% Byzantine
            max_proof_age_ms: 300_000, // 5 minutes
            max_nodes: 1024,
            diversity_weight: 0.4,
            ce_weight: 0.6,
        }
    }

    pub fn validate(&self) -> Result<(), SybilError> {
        if self.min_ce_threshold < 0.0 || self.min_ce_threshold > 1.0 {
            return Err(SybilError::InvalidConfig);
        }
        if self.ce_decay_rate <= 0.0 || self.ce_decay_rate >= 1.0 {
            return Err(SybilError::InvalidConfig);
        }
        if self.min_diversity_entropy < 0.0 {
            return Err(SybilError::InvalidConfig);
        }
        if self.bft_epsilon < 0.0 || self.bft_epsilon >= 0.5 {
            return Err(SybilError::InvalidConfig);
        }
        if self.max_nodes == 0 {
            return Err(SybilError::InvalidConfig);
        }
        if self.diversity_weight + self.ce_weight > 1.0 {
            return Err(SybilError::InvalidConfig);
        }
        Ok(())
    }
}

impl Default for SybilConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Work Proof ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct WorkProof {
    pub proof_id: u64,
    pub node_id: u64,
    pub sae_activation_hash: u128,
    pub timestamp_ms: u64,
    pub contribution_evidence: f64,
    pub region: String,
    pub semantic_fingerprint: u64,
}

impl WorkProof {
    pub fn new(
        proof_id: u64,
        node_id: u64,
        sae_activation_hash: u128,
        timestamp_ms: u64,
        contribution_evidence: f64,
        region: String,
        semantic_fingerprint: u64,
    ) -> Self {
        Self {
            proof_id,
            node_id,
            sae_activation_hash,
            timestamp_ms,
            contribution_evidence: contribution_evidence.clamp(0.0, 1.0),
            region,
            semantic_fingerprint,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.contribution_evidence >= 0.0 && self.contribution_evidence <= 1.0
    }
}

impl fmt::Display for WorkProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WorkProof(id={}, node={}, CE={:.4}, region={}, hash={:016x})",
            self.proof_id,
            self.node_id,
            self.contribution_evidence,
            self.region,
            self.sae_activation_hash
        )
    }
}

// ─── Node State ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct NodeState {
    pub node_id: u64,
    pub region: String,
    pub semantic_fingerprint: u64,
    pub contribution_evidence: f64,
    pub decayed_ce: f64,
    pub last_proof_ms: u64,
    pub proof_count: u32,
    pub active: bool,
}

impl NodeState {
    pub fn new(node_id: u64, region: String, semantic_fingerprint: u64) -> Self {
        Self {
            node_id,
            region,
            semantic_fingerprint,
            contribution_evidence: 0.0,
            decayed_ce: 0.0,
            last_proof_ms: 0,
            proof_count: 0,
            active: true,
        }
    }

    pub fn apply_decay(&mut self, decay_rate: f64, ticks: u32) {
        self.decayed_ce = self.contribution_evidence * decay_rate.powi(ticks as i32);
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NodeState(id={}, region={}, CE={:.4}, decayed={:.4}, proofs={})",
            self.node_id,
            self.region,
            self.contribution_evidence,
            self.decayed_ce,
            self.proof_count
        )
    }
}

// ─── Consensus Record ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ConsensusRecord {
    pub round: u64,
    pub timestamp_ms: u64,
    pub participating_nodes: usize,
    pub byzantine_nodes: usize,
    pub diversity_score: f64,
    pub average_ce: f64,
    pub final_score: f64,
    pub reached_consensus: bool,
}

impl ConsensusRecord {
    pub fn is_valid(&self) -> bool {
        self.final_score >= 0.0 && self.final_score <= 1.0
    }
}

impl fmt::Display for ConsensusRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ConsensusRecord(round={}, nodes={}, byz={}, diversity={:.4}, score={:.4}, consensus={})",
            self.round,
            self.participating_nodes,
            self.byzantine_nodes,
            self.diversity_score,
            self.final_score,
            self.reached_consensus
        )
    }
}

// ─── Sybil Resistance Engine ───────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub struct SybilResistance {
    config: SybilConfig,
    nodes: HashMap<u64, NodeState>,
    proof_history: Vec<WorkProof>,
    consensus_history: Vec<ConsensusRecord>,
    next_proof_id: u64,
    current_round: u64,
    decay_ticks: u32,
}

impl SybilResistance {
    pub fn new() -> Self {
        Self {
            config: SybilConfig::default_stuartian(),
            nodes: HashMap::new(),
            proof_history: Vec::new(),
            consensus_history: Vec::new(),
            next_proof_id: 1,
            current_round: 0,
            decay_ticks: 0,
        }
    }

    pub fn with_config(config: SybilConfig) -> Result<Self, SybilError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            proof_history: Vec::new(),
            consensus_history: Vec::new(),
            next_proof_id: 1,
            current_round: 0,
            decay_ticks: 0,
        })
    }

    /// Submit a Proof-of-Useful-Work
    pub fn submit_proof(&mut self, proof: WorkProof, current_ms: u64) -> Result<(), SybilError> {
        // Check proof age
        if current_ms.saturating_sub(proof.timestamp_ms) > self.config.max_proof_age_ms {
            return Err(SybilError::ExpiredProof(proof.timestamp_ms));
        }

        // Validate proof
        if !proof.is_valid() {
            return Err(SybilError::InvalidWorkProof);
        }

        // Check for duplicate
        if self
            .proof_history
            .iter()
            .any(|p| p.proof_id == proof.proof_id)
        {
            return Err(SybilError::DuplicateSubmission(proof.proof_id));
        }

        // Check max nodes
        if self.nodes.len() >= self.config.max_nodes && !self.nodes.contains_key(&proof.node_id) {
            return Err(SybilError::MaxNodesExceeded(self.config.max_nodes));
        }

        // Update or create node state
        let node = self.nodes.entry(proof.node_id).or_insert_with(|| {
            NodeState::new(
                proof.node_id,
                proof.region.clone(),
                proof.semantic_fingerprint,
            )
        });

        node.contribution_evidence = proof.contribution_evidence;
        node.last_proof_ms = proof.timestamp_ms;
        node.proof_count += 1;
        node.region = proof.region.clone();
        node.semantic_fingerprint = proof.semantic_fingerprint;

        // Apply decay
        node.apply_decay(self.config.ce_decay_rate, self.decay_ticks);

        // Store proof
        self.proof_history.push(proof);
        Ok(())
    }

    /// Compute Shannon entropy for geographic diversity
    pub fn compute_diversity_entropy(&self) -> f64 {
        let total = self.nodes.len();
        if total == 0 {
            return 0.0;
        }

        // Count nodes per region
        let mut region_counts: HashMap<String, usize> = HashMap::new();
        for node in self.nodes.values() {
            *region_counts.entry(node.region.clone()).or_insert(0) += 1;
        }

        let num_regions = region_counts.len();
        if num_regions <= 1 {
            return 0.0;
        }

        // Compute Shannon entropy: H = -Σ(p * log2(p))
        let mut entropy = 0.0_f64;
        for &count in region_counts.values() {
            let p = count as f64 / total as f64;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }

        // Normalize by log2(num_regions)
        let max_entropy = (num_regions as f64).log2();
        if max_entropy > 0.0 {
            entropy /= max_entropy;
        }

        entropy
    }

    /// Run a consensus round
    pub fn run_consensus_round(&mut self, current_ms: u64) -> Result<ConsensusRecord, SybilError> {
        self.current_round += 1;
        self.decay_ticks += 1;

        let active_nodes: Vec<&NodeState> = self.nodes.values().filter(|n| n.active).collect();
        let total = active_nodes.len();

        if total == 0 {
            return Ok(ConsensusRecord {
                round: self.current_round,
                timestamp_ms: current_ms,
                participating_nodes: 0,
                byzantine_nodes: 0,
                diversity_score: 0.0,
                average_ce: 0.0,
                final_score: 0.0,
                reached_consensus: false,
            });
        }

        // Compute diversity
        let diversity = self.compute_diversity_entropy();
        if diversity < self.config.min_diversity_entropy {
            // Log warning but continue
        }

        // Compute average CE
        let avg_ce: f64 = active_nodes.iter().map(|n| n.decayed_ce).sum::<f64>() / total as f64;

        // Check CE threshold
        if avg_ce < self.config.min_ce_threshold {
            // Continue but score will be low
        }

        // Estimate Byzantine nodes (nodes with CE below threshold)
        let byzantine = active_nodes
            .iter()
            .filter(|n| n.decayed_ce < self.config.min_ce_threshold)
            .count();

        // BFT check: byzantine fraction must be < epsilon
        let byzantine_fraction = byzantine as f64 / total as f64;
        let bft_ok = byzantine_fraction < self.config.bft_epsilon;

        // Final score: weighted combination
        let final_score = (self.config.diversity_weight * diversity
            + self.config.ce_weight * avg_ce)
            .min(1.0)
            .max(0.0);

        let reached_consensus = bft_ok && final_score > self.config.min_ce_threshold;

        let record = ConsensusRecord {
            round: self.current_round,
            timestamp_ms: current_ms,
            participating_nodes: total,
            byzantine_nodes: byzantine,
            diversity_score: diversity,
            average_ce: avg_ce,
            final_score,
            reached_consensus,
        };

        self.consensus_history.push(record.clone());
        Ok(record)
    }

    /// Verify a work proof against SAE activation hash
    pub fn verify_work_proof(
        &self,
        proof: &WorkProof,
        expected_hash: u128,
    ) -> Result<(), SybilError> {
        if proof.sae_activation_hash != expected_hash {
            return Err(SybilError::HashMismatch);
        }
        Ok(())
    }

    /// Get node state
    pub fn get_node(&self, node_id: u64) -> Option<&NodeState> {
        self.nodes.get(&node_id)
    }

    /// Get consensus history
    pub fn consensus_history(&self) -> &[ConsensusRecord] {
        &self.consensus_history
    }

    /// Get proof count
    pub fn proof_count(&self) -> usize {
        self.proof_history.len()
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if consensus can be reached
    pub fn can_reach_consensus(&self) -> bool {
        let total = self.nodes.values().filter(|n| n.active).count();
        if total == 0 {
            return false;
        }
        let byzantine = self
            .nodes
            .values()
            .filter(|n| n.active && n.decayed_ce < self.config.min_ce_threshold)
            .count();
        let byzantine_fraction = byzantine as f64 / total as f64;
        byzantine_fraction < self.config.bft_epsilon
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.proof_history.clear();
        self.consensus_history.clear();
        self.next_proof_id = 1;
        self.current_round = 0;
        self.decay_ticks = 0;
    }
}

impl Default for SybilResistance {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SybilResistance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SybilResistance(nodes={}, proofs={}, round={}, diversity={:.4})",
            self.nodes.len(),
            self.proof_history.len(),
            self.current_round,
            self.compute_diversity_entropy()
        )
    }
}

// ─── Public Utility Functions ──────────────────────────────────────────────────

/// Compute Shannon entropy from region distribution
pub fn shannon_entropy(region_counts: &[usize]) -> f64 {
    let total: usize = region_counts.iter().sum();
    if total == 0 {
        return 0.0;
    }

    let mut entropy = 0.0_f64;
    for &count in region_counts {
        let p = count as f64 / total as f64;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Compute normalized Shannon entropy
pub fn normalized_shannon_entropy(region_counts: &[usize]) -> f64 {
    let num_regions = region_counts.iter().filter(|&&c| c > 0).count();
    if num_regions <= 1 {
        return 0.0;
    }
    let entropy = shannon_entropy(region_counts);
    let max_entropy = (num_regions as f64).log2();
    if max_entropy > 0.0 {
        entropy / max_entropy
    } else {
        0.0
    }
}

/// Compute BFT threshold check
pub fn bft_check(total: usize, byzantine: usize, epsilon: f64) -> bool {
    if total == 0 {
        return false;
    }
    let fraction = byzantine as f64 / total as f64;
    fraction < epsilon
}

/// Apply exponential CE decay
pub fn apply_ce_decay(initial_ce: f64, decay_rate: f64, ticks: u32) -> f64 {
    initial_ce * decay_rate.powi(ticks as i32)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(id: u64, node: u64, ce: f64, region: &str, ts: u64) -> WorkProof {
        WorkProof::new(id, node, 0xABCD, ts, ce, region.to_string(), node * 31)
    }

    // ─── Config Tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_config_default() {
        let config = SybilConfig::default_stuartian();
        assert_eq!(config.bft_epsilon, 0.33);
        assert!(config.min_ce_threshold > 0.0);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = SybilConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_ce() {
        let config = SybilConfig {
            min_ce_threshold: -0.1,
            ..SybilConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(SybilError::InvalidConfig));
    }

    #[test]
    fn test_config_invalid_bft() {
        let config = SybilConfig {
            bft_epsilon: 0.6,
            ..SybilConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(SybilError::InvalidConfig));
    }

    #[test]
    fn test_config_zero_nodes() {
        let config = SybilConfig {
            max_nodes: 0,
            ..SybilConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(SybilError::InvalidConfig));
    }

    #[test]
    fn test_config_weights_exceed() {
        let config = SybilConfig {
            diversity_weight: 0.6,
            ce_weight: 0.6,
            ..SybilConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(SybilError::InvalidConfig));
    }

    // ─── Work Proof Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_proof_creation() {
        let proof = make_proof(1, 100, 0.8, "US-East", 1000);
        assert_eq!(proof.proof_id, 1);
        assert!(proof.is_valid());
    }

    #[test]
    fn test_proof_invalid_ce() {
        let proof = WorkProof::new(1, 100, 0xABCD, 1000, 1.5, "US".to_string(), 31);
        assert!(proof.is_valid()); // CE is clamped to [0, 1]
    }

    #[test]
    fn test_proof_display() {
        let proof = make_proof(1, 100, 0.8, "US-East", 1000);
        let s = format!("{}", proof);
        assert!(s.contains("WorkProof"));
        assert!(s.contains("node=100"));
    }

    // ─── Node State Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_node_state_new() {
        let node = NodeState::new(1, "US-East".to_string(), 31);
        assert_eq!(node.node_id, 1);
        assert_eq!(node.contribution_evidence, 0.0);
        assert!(node.active);
    }

    #[test]
    fn test_node_state_decay() {
        let mut node = NodeState::new(1, "US-East".to_string(), 31);
        node.contribution_evidence = 1.0;
        node.apply_decay(0.9, 10);
        assert!(node.decayed_ce < 1.0);
        assert!((node.decayed_ce - 0.9_f64.powi(10)) < 1e-10);
    }

    #[test]
    fn test_node_state_display() {
        let node = NodeState::new(42, "EU-West".to_string(), 100);
        let s = format!("{}", node);
        assert!(s.contains("NodeState"));
        assert!(s.contains("id=42"));
    }

    // ─── Engine Creation Tests ─────────────────────────────────────────────────

    #[test]
    fn test_engine_new() {
        let engine = SybilResistance::new();
        assert_eq!(engine.node_count(), 0);
        assert_eq!(engine.proof_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = SybilConfig::default_stuartian();
        let engine = SybilResistance::with_config(config).unwrap();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_engine_with_bad_config() {
        let config = SybilConfig {
            max_nodes: 0,
            ..SybilConfig::default_stuartian()
        };
        assert_eq!(
            SybilResistance::with_config(config),
            Err(SybilError::InvalidConfig)
        );
    }

    // ─── Proof Submission Tests ────────────────────────────────────────────────

    #[test]
    fn test_submit_proof() {
        let mut engine = SybilResistance::new();
        let proof = make_proof(1, 100, 0.8, "US-East", 1000);
        engine.submit_proof(proof, 1000).unwrap();
        assert_eq!(engine.node_count(), 1);
        assert_eq!(engine.proof_count(), 1);
    }

    #[test]
    fn test_submit_expired_proof() {
        let mut engine = SybilResistance::new();
        let proof = make_proof(1, 100, 0.8, "US-East", 1000);
        assert_eq!(
            engine.submit_proof(proof, 1000 + engine.config.max_proof_age_ms + 1),
            Err(SybilError::ExpiredProof(1000))
        );
    }

    #[test]
    fn test_submit_duplicate_proof() {
        let mut engine = SybilResistance::new();
        let proof = make_proof(1, 100, 0.8, "US-East", 1000);
        engine.submit_proof(proof.clone(), 1000).unwrap();
        assert_eq!(
            engine.submit_proof(proof, 1000),
            Err(SybilError::DuplicateSubmission(1))
        );
    }

    #[test]
    fn test_submit_max_nodes() {
        let mut engine = SybilResistance::new();
        engine.config.max_nodes = 2;

        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.8, "EU", 1000), 1000)
            .unwrap();
        assert_eq!(
            engine.submit_proof(make_proof(3, 3, 0.8, "AP", 1000), 1000),
            Err(SybilError::MaxNodesExceeded(2))
        );
    }

    #[test]
    fn test_submit_updates_existing_node() {
        let mut engine = SybilResistance::new();
        engine
            .submit_proof(make_proof(1, 100, 0.5, "US-East", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 100, 0.8, "US-East", 2000), 2000)
            .unwrap();

        let node = engine.get_node(100).unwrap();
        assert_eq!(node.proof_count, 2);
        assert!((node.contribution_evidence - 0.8) < 1e-10);
    }

    // ─── Diversity Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_diversity_empty() {
        let engine = SybilResistance::new();
        assert_eq!(engine.compute_diversity_entropy(), 0.0);
    }

    #[test]
    fn test_diversity_single_region() {
        let mut engine = SybilResistance::new();
        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.8, "US", 1000), 1000)
            .unwrap();
        assert_eq!(engine.compute_diversity_entropy(), 0.0);
    }

    #[test]
    fn test_diversity_uniform() {
        let mut engine = SybilResistance::new();
        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.8, "EU", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(3, 3, 0.8, "AP", 1000), 1000)
            .unwrap();
        let entropy = engine.compute_diversity_entropy();
        assert!((entropy - 1.0) < 1e-10); // Uniform = max entropy
    }

    #[test]
    fn test_diversity_skewed() {
        let mut engine = SybilResistance::new();
        // 3 in US, 1 in EU
        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(3, 3, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(4, 4, 0.8, "EU", 1000), 1000)
            .unwrap();
        let entropy = engine.compute_diversity_entropy();
        assert!(entropy > 0.0 && entropy < 1.0);
    }

    // ─── Consensus Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_consensus_empty() {
        let mut engine = SybilResistance::new();
        let record = engine.run_consensus_round(1000).unwrap();
        assert!(!record.reached_consensus);
        assert_eq!(record.participating_nodes, 0);
    }

    #[test]
    fn test_consensus_reached() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.1;
        engine.config.min_diversity_entropy = 0.0;

        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.8, "EU", 1000), 1000)
            .unwrap();

        let record = engine.run_consensus_round(2000).unwrap();
        assert!(record.reached_consensus);
        assert_eq!(record.participating_nodes, 2);
    }

    #[test]
    fn test_consensus_not_reached_low_ce() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.9;

        engine
            .submit_proof(make_proof(1, 1, 0.3, "US", 1000), 1000)
            .unwrap();

        let record = engine.run_consensus_round(2000).unwrap();
        assert!(!record.reached_consensus);
    }

    #[test]
    fn test_consensus_bft_failure() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.5;
        engine.config.bft_epsilon = 0.1; // Very strict

        // 1 good node, 3 bad nodes (CE below threshold)
        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.1, "EU", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(3, 3, 0.1, "AP", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(4, 4, 0.1, "SA", 1000), 1000)
            .unwrap();

        let record = engine.run_consensus_round(2000).unwrap();
        assert!(!record.reached_consensus); // 3/4 = 75% Byzantine > 10% epsilon
    }

    #[test]
    fn test_consensus_record_valid() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.1;
        engine.config.min_diversity_entropy = 0.0;

        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        let record = engine.run_consensus_round(2000).unwrap();
        assert!(record.is_valid());
    }

    #[test]
    fn test_consensus_record_display() {
        let record = ConsensusRecord {
            round: 1,
            timestamp_ms: 1000,
            participating_nodes: 10,
            byzantine_nodes: 2,
            diversity_score: 0.8,
            average_ce: 0.7,
            final_score: 0.75,
            reached_consensus: true,
        };
        let s = format!("{}", record);
        assert!(s.contains("ConsensusRecord"));
    }

    // ─── Verification Tests ────────────────────────────────────────────────────

    #[test]
    fn test_verify_work_proof_match() {
        let engine = SybilResistance::new();
        let proof = make_proof(1, 100, 0.8, "US", 1000);
        assert!(engine.verify_work_proof(&proof, 0xABCD).is_ok());
    }

    #[test]
    fn test_verify_work_proof_mismatch() {
        let engine = SybilResistance::new();
        let proof = make_proof(1, 100, 0.8, "US", 1000);
        assert_eq!(
            engine.verify_work_proof(&proof, 0xDEAD),
            Err(SybilError::HashMismatch)
        );
    }

    // ─── BFT Check Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_can_reach_consensus_true() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.1;
        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.8, "EU", 1000), 1000)
            .unwrap();
        assert!(engine.can_reach_consensus());
    }

    #[test]
    fn test_can_reach_consensus_false() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.5;
        engine.config.bft_epsilon = 0.1;
        engine
            .submit_proof(make_proof(1, 1, 0.1, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(2, 2, 0.1, "EU", 1000), 1000)
            .unwrap();
        assert!(!engine.can_reach_consensus());
    }

    // ─── Utility Function Tests ────────────────────────────────────────────────

    #[test]
    fn test_shannon_entropy_uniform() {
        let counts = vec![2, 2, 2];
        let entropy = shannon_entropy(&counts);
        assert!((entropy - 3_f64.log2()) < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_single() {
        let counts = vec![10];
        assert_eq!(shannon_entropy(&counts), 0.0);
    }

    #[test]
    fn test_shannon_entropy_empty() {
        assert_eq!(shannon_entropy(&[]), 0.0);
    }

    #[test]
    fn test_normalized_entropy_uniform() {
        let counts = vec![2, 2, 2];
        assert!((normalized_shannon_entropy(&counts) - 1.0) < 1e-10);
    }

    #[test]
    fn test_normalized_entropy_single() {
        let counts = vec![10];
        assert_eq!(normalized_shannon_entropy(&counts), 0.0);
    }

    #[test]
    fn test_bft_check_pass() {
        assert!(bft_check(10, 2, 0.33)); // 2/10 = 20% < 33%
    }

    #[test]
    fn test_bft_check_fail() {
        assert!(!bft_check(10, 4, 0.33)); // 4/10 = 40% > 33%
    }

    #[test]
    fn test_bft_check_empty() {
        assert!(!bft_check(0, 0, 0.33));
    }

    #[test]
    fn test_ce_decay() {
        let decayed = apply_ce_decay(1.0, 0.9, 10);
        assert!((decayed - 0.9_f64.powi(10)) < 1e-10);
    }

    #[test]
    fn test_ce_decay_zero_ticks() {
        assert!((apply_ce_decay(0.8, 0.9, 0) - 0.8) < 1e-10);
    }

    // ─── Reset Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_reset() {
        let mut engine = SybilResistance::new();
        engine
            .submit_proof(make_proof(1, 1, 0.8, "US", 1000), 1000)
            .unwrap();
        engine.run_consensus_round(2000).unwrap();

        engine.reset();
        assert_eq!(engine.node_count(), 0);
        assert_eq!(engine.proof_count(), 0);
        assert_eq!(engine.consensus_history().len(), 0);
    }

    // ─── Display Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_engine_display() {
        let engine = SybilResistance::new();
        let s = format!("{}", engine);
        assert!(s.contains("SybilResistance"));
    }

    // ─── Error Display Tests ───────────────────────────────────────────────────

    #[test]
    fn test_error_display_invalid_proof() {
        let e = SybilError::InvalidWorkProof;
        assert!(format!("{}", e).contains("Invalid"));
    }

    #[test]
    fn test_error_display_expired() {
        let e = SybilError::ExpiredProof(42);
        let s = format!("{}", e);
        assert!(s.contains("42"));
    }

    #[test]
    fn test_error_display_duplicate() {
        let e = SybilError::DuplicateSubmission(99);
        let s = format!("{}", e);
        assert!(s.contains("99"));
    }

    #[test]
    fn test_error_display_diversity() {
        let e = SybilError::InsufficientDiversity(0.3);
        let s = format!("{}", e);
        assert!(s.contains("0.3"));
    }

    #[test]
    fn test_error_display_bft() {
        let e = SybilError::BFTThresholdNotMet(0.5);
        let s = format!("{}", e);
        assert!(s.contains("0.5"));
    }

    // ─── Workflow Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_full_sybil_workflow() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.1;
        engine.config.min_diversity_entropy = 0.3;

        // Submit proofs from diverse regions
        for (i, region) in ["US", "EU", "AP", "SA", "AF"].iter().enumerate() {
            engine
                .submit_proof(
                    make_proof((i + 1) as u64, (i + 1) as u64, 0.8, region, 1000),
                    1000,
                )
                .unwrap();
        }

        // Run consensus
        let record = engine.run_consensus_round(2000).unwrap();
        assert!(record.reached_consensus);
        assert_eq!(record.participating_nodes, 5);
        assert!(record.diversity_score > 0.8);

        // Run another round
        let record2 = engine.run_consensus_round(3000).unwrap();
        assert_eq!(record2.round, 2);
    }

    #[test]
    fn test_sybil_attack_detection() {
        let mut engine = SybilResistance::new();
        engine.config.min_ce_threshold = 0.5;
        engine.config.bft_epsilon = 0.33;

        // 1 legitimate node
        engine
            .submit_proof(make_proof(1, 1, 0.9, "US", 1000), 1000)
            .unwrap();

        // 3 Sybil nodes with low CE
        engine
            .submit_proof(make_proof(2, 2, 0.1, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(3, 3, 0.1, "US", 1000), 1000)
            .unwrap();
        engine
            .submit_proof(make_proof(4, 4, 0.1, "US", 1000), 1000)
            .unwrap();

        let record = engine.run_consensus_round(2000).unwrap();
        assert!(!record.reached_consensus); // 3/4 Byzantine > 33%
        assert_eq!(record.byzantine_nodes, 3);
    }
}
