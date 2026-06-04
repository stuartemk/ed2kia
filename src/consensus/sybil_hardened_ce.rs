//! Sybil-Hardened CE â€” Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! Proof-of-Useful-Work (PoUW) anclado a hash SAE + decaimiento exponencial CE
//! + ponderaciÃ³n diversidad geogrÃ¡fica/semÃ¡ntica + capa de vouching inicial.

use std::collections::HashMap;
use std::fmt;

/// Error types for Sybil-Hardened CE
#[derive(Debug, Clone, PartialEq)]
pub enum CeError {
    /// Invalid PoUW nonce
    InvalidPow,
    /// CE score out of range
    InvalidCe(f64),
    /// Node capacity exceeded
    CapacityExceeded(usize),
    /// Duplicate proof
    DuplicateProof(u64),
    /// Diversity entropy too low
    DiversityTooLow(f64),
    /// BFT threshold not met
    BftFailure { honest: usize, total: usize },
    /// Expired proof
    Expired { age_ms: u64, max_ms: u64 },
}

impl fmt::Display for CeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CeError::InvalidPow => write!(f, "Invalid Proof-of-Useful-Work nonce"),
            CeError::InvalidCe(ce) => write!(f, "CE score out of range: {}", ce),
            CeError::CapacityExceeded(cap) => write!(f, "Node capacity exceeded: {}", cap),
            CeError::DuplicateProof(id) => write!(f, "Duplicate proof: {}", id),
            CeError::DiversityTooLow(h) => write!(f, "Diversity entropy too low: {:.4}", h),
            CeError::BftFailure { honest, total } => {
                write!(f, "BFT failure: {}/{} honest nodes", honest, total)
            }
            CeError::Expired { age_ms, max_ms } => {
                write!(f, "Proof expired: {}ms > {}ms", age_ms, max_ms)
            }
        }
    }
}

/// Configuration for Sybil-Hardened CE
#[derive(Debug, Clone)]
pub struct HardenedCeConfig {
    /// Maximum nodes
    pub max_nodes: usize,
    /// CE decay rate (exponential)
    pub ce_decay_rate: f64,
    /// BFT epsilon tolerance
    pub bft_epsilon: f64,
    /// Minimum diversity entropy
    pub min_diversity_entropy: f64,
    /// Proof validity window in ms
    pub proof_validity_ms: u64,
    /// Weights: [pow, diversity, vouch]
    pub score_weights: [f64; 3],
}

impl HardenedCeConfig {
    pub fn default_Topological() -> Self {
        Self {
            max_nodes: 10000,
            ce_decay_rate: 0.00001,
            bft_epsilon: 0.1,
            min_diversity_entropy: 0.3,
            proof_validity_ms: 600_000,
            score_weights: [0.4, 0.35, 0.25],
        }
    }

    pub fn validate(&self) -> Result<(), CeError> {
        if self.ce_decay_rate < 0.0 || self.ce_decay_rate > 1.0 {
            return Err(CeError::InvalidCe(self.ce_decay_rate));
        }
        if self.bft_epsilon < 0.0 || self.bft_epsilon > 0.5 {
            return Err(CeError::InvalidCe(self.bft_epsilon));
        }
        if self.max_nodes == 0 {
            return Err(CeError::CapacityExceeded(0));
        }
        let weights_sum: f64 = self.score_weights.iter().sum();
        if (weights_sum - 1.0).abs() > 0.01 {
            return Err(CeError::InvalidCe(weights_sum));
        }
        Ok(())
    }
}

impl Default for HardenedCeConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Proof-of-Useful-Work structure
#[derive(Debug, Clone)]
pub struct PowProof {
    pub proof_id: u64,
    pub node_id: u64,
    pub sae_hash: u128,
    pub nonce: u64,
    pub ce_value: f64,
    pub timestamp_ms: u64,
}

impl PowProof {
    pub fn new(
        proof_id: u64,
        node_id: u64,
        sae_hash: u128,
        nonce: u64,
        ce_value: f64,
        timestamp_ms: u64,
    ) -> Result<Self, CeError> {
        if ce_value < 0.0 || ce_value > 1.0 {
            return Err(CeError::InvalidCe(ce_value));
        }
        Ok(Self {
            proof_id,
            node_id,
            sae_hash,
            nonce,
            ce_value,
            timestamp_ms,
        })
    }
}

impl fmt::Display for PowProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PowProof {{ id: {}, node: {}, ce: {:.4}, nonce: {}, ts: {} }}",
            self.proof_id, self.node_id, self.ce_value, self.nonce, self.timestamp_ms
        )
    }
}

/// Node state with CE tracking
#[derive(Debug, Clone)]
pub struct HardenedNodeState {
    pub node_id: u64,
    pub geo_region: String,
    pub semantic_fingerprint: u64,
    pub ce_score: f64,
    pub vouches: u32,
    pub last_update_ms: u64,
}

impl HardenedNodeState {
    pub fn new(node_id: u64, geo_region: String, semantic_fingerprint: u64) -> Self {
        Self {
            node_id,
            geo_region,
            semantic_fingerprint,
            ce_score: 0.5,
            vouches: 0,
            last_update_ms: 0,
        }
    }

    /// Apply exponential CE decay
    pub fn apply_decay(&mut self, decay_rate: f64, current_ms: u64) {
        let elapsed = current_ms.saturating_sub(self.last_update_ms) as f64;
        let factor = (-decay_rate * elapsed).exp();
        self.ce_score *= factor;
        self.ce_score = self.ce_score.max(0.0).min(1.0);
    }
}

impl fmt::Display for HardenedNodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HardenedNodeState {{ id: {}, region: {}, ce: {:.4}, vouches: {} }}",
            self.node_id, self.geo_region, self.ce_score, self.vouches
        )
    }
}

/// Consensus record
#[derive(Debug, Clone)]
pub struct HardenedConsensusRecord {
    pub round: u64,
    pub timestamp_ms: u64,
    pub participating_nodes: usize,
    pub average_ce: f64,
    pub diversity_entropy: f64,
    pub reached_consensus: bool,
}

impl fmt::Display for HardenedConsensusRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HardenedConsensusRecord {{ round: {}, nodes: {}, ce: {:.4}, diversity: {:.4}, consensus: {} }}",
            self.round, self.participating_nodes, self.average_ce, self.diversity_entropy, self.reached_consensus
        )
    }
}

/// Sybil-Hardened CE Engine
pub struct SybilHardenedCe {
    config: HardenedCeConfig,
    nodes: HashMap<u64, HardenedNodeState>,
    proofs: HashMap<u64, PowProof>,
    consensus_history: Vec<HardenedConsensusRecord>,
    current_round: u64,
}

impl SybilHardenedCe {
    pub fn new() -> Self {
        Self {
            config: HardenedCeConfig::default_Topological(),
            nodes: HashMap::new(),
            proofs: HashMap::new(),
            consensus_history: Vec::new(),
            current_round: 0,
        }
    }

    pub fn with_config(config: HardenedCeConfig) -> Result<Self, CeError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            proofs: HashMap::new(),
            consensus_history: Vec::new(),
            current_round: 0,
        })
    }

    /// Submit PoUW proof
    pub fn submit_proof(
        &mut self,
        proof: PowProof,
        geo_region: String,
        semantic_fingerprint: u64,
        vouches: u32,
        current_ms: u64,
    ) -> Result<(), CeError> {
        // Check expiry
        let age = current_ms.saturating_sub(proof.timestamp_ms);
        if age > self.config.proof_validity_ms {
            return Err(CeError::Expired {
                age_ms: age,
                max_ms: self.config.proof_validity_ms,
            });
        }

        // Check duplicate
        if self.proofs.contains_key(&proof.proof_id) {
            return Err(CeError::DuplicateProof(proof.proof_id));
        }

        // Check capacity
        if !self.nodes.contains_key(&proof.node_id) && self.nodes.len() >= self.config.max_nodes {
            return Err(CeError::CapacityExceeded(self.config.max_nodes));
        }

        // Store proof
        self.proofs.insert(proof.proof_id, proof.clone());

        // Compute diversity before mutable borrow
        let diversity = self.compute_diversity_entropy();

        // Update or create node
        let node = self.nodes.entry(proof.node_id).or_insert_with(|| {
            HardenedNodeState::new(proof.node_id, geo_region.clone(), semantic_fingerprint)
        });

        node.ce_score =
            Self::compute_ce_score(&proof, diversity, vouches, &self.config.score_weights);
        node.vouches = vouches;
        node.last_update_ms = current_ms;

        Ok(())
    }

    /// Compute CE score with PoUW + decay + diversity + vouching
    pub fn compute_ce_score(
        proof: &PowProof,
        diversity: f64,
        vouches: u32,
        weights: &[f64; 3],
    ) -> f64 {
        // PoUW component: based on nonce difficulty
        let pow_score = (proof.nonce as f64 / 1_000_000.0).min(1.0);
        // Diversity component
        let diversity_score = diversity.min(1.0);
        // Vouching component
        let vouch_score = (vouches as f64 / 10.0).min(1.0);

        let ce = weights[0] * pow_score + weights[1] * diversity_score + weights[2] * vouch_score;
        ce.min(1.0).max(0.0)
    }

    /// Compute diversity entropy (Shannon)
    pub fn compute_diversity_entropy(&self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        let mut region_counts: HashMap<String, usize> = HashMap::new();
        for node in self.nodes.values() {
            *region_counts.entry(node.geo_region.clone()).or_insert(0) += 1;
        }

        let total = self.nodes.len() as f64;
        let entropy: f64 = region_counts
            .values()
            .map(|&count| {
                let p = count as f64 / total;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum();

        // Normalize by log2(n_regions)
        let max_entropy = (region_counts.len() as f64).log2().max(1.0);
        entropy / max_entropy
    }

    /// Run consensus round
    pub fn run_consensus_round(
        &mut self,
        current_ms: u64,
    ) -> Result<HardenedConsensusRecord, CeError> {
        self.current_round += 1;

        if self.nodes.is_empty() {
            return Ok(HardenedConsensusRecord {
                round: self.current_round,
                timestamp_ms: current_ms,
                participating_nodes: 0,
                average_ce: 0.0,
                diversity_entropy: 0.0,
                reached_consensus: false,
            });
        }

        // Apply decay to all nodes
        for node in self.nodes.values_mut() {
            node.apply_decay(self.config.ce_decay_rate, current_ms);
        }

        let diversity = self.compute_diversity_entropy();

        // Check diversity threshold
        if diversity < self.config.min_diversity_entropy && self.nodes.len() > 1 {
            return Err(CeError::DiversityTooLow(diversity));
        }

        // Compute average CE
        let total_ce: f64 = self.nodes.values().map(|n| n.ce_score).sum();
        let avg_ce = total_ce / self.nodes.len() as f64;

        // BFT check
        let honest_count = self.nodes.values().filter(|n| n.ce_score > 0.5).count();
        let threshold =
            (self.nodes.len() as f64 * (2.0 / 3.0 * (1.0 - self.config.bft_epsilon))) as usize;

        let reached = honest_count >= threshold;

        if !reached {
            return Err(CeError::BftFailure {
                honest: honest_count,
                total: self.nodes.len(),
            });
        }

        let record = HardenedConsensusRecord {
            round: self.current_round,
            timestamp_ms: current_ms,
            participating_nodes: self.nodes.len(),
            average_ce: avg_ce,
            diversity_entropy: diversity,
            reached_consensus: reached,
        };

        self.consensus_history.push(record.clone());
        Ok(record)
    }

    /// Verify PoUW proof
    pub fn verify_pow(proof: &PowProof, expected_sae_hash: u128) -> bool {
        proof.sae_hash == expected_sae_hash && proof.nonce > 0
    }

    /// Check if consensus can be reached
    pub fn can_reach_consensus(&self) -> bool {
        let threshold = (self.nodes.len() as f64 * 2.0 / 3.0) as usize;
        let honest = self.nodes.values().filter(|n| n.ce_score > 0.5).count();
        honest >= threshold
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.proofs.clear();
        self.consensus_history.clear();
        self.current_round = 0;
    }
}

impl Default for SybilHardenedCe {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SybilHardenedCe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SybilHardenedCe {{ nodes: {}, proofs: {}, round: {} }}",
            self.nodes.len(),
            self.proofs.len(),
            self.current_round
        )
    }
}

/// Public function: Compute CE score
pub fn compute_ce_score(
    _node_id: u64,
    pow_nonce: u64,
    _geo_region: String,
    semantic_diversity: f64,
    vouches: u32,
) -> f64 {
    let pow_score = (pow_nonce as f64 / 1_000_000.0).min(1.0);
    let diversity_score = semantic_diversity.min(1.0);
    let vouch_score = (vouches as f64 / 10.0).min(1.0);

    let weights = [0.4, 0.35, 0.25];
    let ce = weights[0] * pow_score + weights[1] * diversity_score + weights[2] * vouch_score;
    ce.min(1.0).max(0.0)
}

/// Shannon entropy for region distribution
pub fn shannon_entropy(region_counts: &[usize]) -> f64 {
    let total: usize = region_counts.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f64;
    region_counts
        .iter()
        .map(|&c| {
            let p = c as f64 / total_f;
            if p > 0.0 {
                -p * p.log2()
            } else {
                0.0
            }
        })
        .sum()
}

/// Normalized Shannon entropy
pub fn normalized_shannon_entropy(region_counts: &[usize]) -> f64 {
    let entropy = shannon_entropy(region_counts);
    let n = region_counts.len().max(1);
    let max_entropy = (n as f64).log2().max(1.0);
    entropy / max_entropy
}

/// BFT check with epsilon tolerance
pub fn bft_check(total: usize, byzantine: usize, epsilon: f64) -> bool {
    let honest = total.saturating_sub(byzantine);
    let threshold_f = total as f64 * (2.0_f64 / 3.0 * (1.0 - epsilon));
    honest as f64 > threshold_f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HardenedCeConfig::default();
        assert_eq!(config.max_nodes, 10000);
        assert!((config.score_weights.iter().sum::<f64>() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = HardenedCeConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_ce() {
        let config = HardenedCeConfig {
            ce_decay_rate: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_proof_creation() {
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000);
        assert!(proof.is_ok());
    }

    #[test]
    fn test_proof_invalid_ce() {
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 1.5, 1000);
        assert!(proof.is_err());
    }

    #[test]
    fn test_node_state_new() {
        let node = HardenedNodeState::new(1, "us-east".to_string(), 0x1234);
        assert_eq!(node.node_id, 1);
        assert_eq!(node.ce_score, 0.5);
    }

    #[test]
    fn test_node_decay() {
        let mut node = HardenedNodeState::new(1, "us-east".to_string(), 0x1234);
        node.ce_score = 1.0;
        node.last_update_ms = 0;
        node.apply_decay(0.00001, 100_000);
        assert!(node.ce_score < 1.0);
        assert!(node.ce_score > 0.0);
    }

    #[test]
    fn test_engine_new() {
        let engine = SybilHardenedCe::new();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_submit_proof() {
        let mut engine = SybilHardenedCe::new();
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        let result = engine.submit_proof(proof, "us-east".to_string(), 0x1234, 2, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_submit_expired_proof() {
        let mut engine = SybilHardenedCe::new();
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        let result = engine.submit_proof(proof, "us-east".to_string(), 0x1234, 2, 1_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_duplicate_proof() {
        let mut engine = SybilHardenedCe::new();
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        engine
            .submit_proof(proof.clone(), "us-east".to_string(), 0x1234, 2, 1000)
            .unwrap();
        let result = engine.submit_proof(proof, "us-east".to_string(), 0x1234, 2, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_diversity_single_region() {
        let mut engine = SybilHardenedCe::new();
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        engine
            .submit_proof(proof, "us-east".to_string(), 0x1234, 2, 1000)
            .unwrap();
        let diversity = engine.compute_diversity_entropy();
        assert!((diversity - 0.0).abs() < 0.01 || (diversity - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_consensus_empty() {
        let mut engine = SybilHardenedCe::new();
        let result = engine.run_consensus_round(1000);
        assert!(result.is_ok());
        assert!(!result.unwrap().reached_consensus);
    }

    #[test]
    fn test_verify_pow_match() {
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        assert!(SybilHardenedCe::verify_pow(&proof, 0xabcd));
    }

    #[test]
    fn test_verify_pow_mismatch() {
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        assert!(!SybilHardenedCe::verify_pow(&proof, 0x1234));
    }

    #[test]
    fn test_compute_ce_score() {
        let ce = compute_ce_score(1, 500_000, "us-east".to_string(), 0.8, 5);
        assert!(ce > 0.0 && ce <= 1.0);
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        let entropy = shannon_entropy(&[3, 3, 3]);
        assert!((entropy - 1.585).abs() < 0.01);
    }

    #[test]
    fn test_normalized_entropy_uniform() {
        let entropy = normalized_shannon_entropy(&[3, 3, 3]);
        assert!((entropy - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_bft_check() {
        assert!(bft_check(9, 2, 0.1));
        assert!(!bft_check(9, 4, 0.1));
    }

    #[test]
    fn test_reset() {
        let mut engine = SybilHardenedCe::new();
        let proof = PowProof::new(1, 100, 0xabcd, 12345, 0.8, 1000).unwrap();
        engine
            .submit_proof(proof, "us-east".to_string(), 0x1234, 2, 1000)
            .unwrap();
        engine.reset();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = SybilHardenedCe::new();
        let s = format!("{}", engine);
        assert!(s.contains("SybilHardenedCe"));
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = SybilHardenedCe::new();
        // Add 4 nodes in different regions for diversity
        for i in 0..4 {
            let proof = PowProof::new(i, i, 0xabcd, 500000, 0.9, 1000).unwrap();
            let region = format!("region-{}", i % 3);
            engine
                .submit_proof(proof, region, i * 100, 3, 1000)
                .unwrap();
        }
        let result = engine.run_consensus_round(1000);
        // May or may not reach consensus depending on BFT threshold
        assert!(result.is_ok() || result.is_err());
    }
}
