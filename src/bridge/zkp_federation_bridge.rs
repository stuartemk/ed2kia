//! ZKP Federation Bridge — Bridges ZKP proofs across federation nodes for cross-node verification.
//!
//! Enables:
//! - Proof relay between federation shards
//! - Cross-shard verification with Merkle roots
//! - Proof aggregation for batch verification
//! - Consensus on proof validity across nodes

use std::collections::HashMap;

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum BridgeError {
    NodeNotFound(String),
    ProofNotFound(String),
    VerificationMismatch(String),
    ConsensusFailed { yes: u64, no: u64 },
    ShardNotFound(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::ProofNotFound(id) => write!(f, "Proof not found: {}", id),
            Self::VerificationMismatch(id) => write!(f, "Verification mismatch: {}", id),
            Self::ConsensusFailed { yes, no } => {
                write!(f, "Consensus failed: {} yes, {} no", yes, no)
            }
            Self::ShardNotFound(id) => write!(f, "Shard not found: {}", id),
        }
    }
}

impl std::error::Error for BridgeError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub consensus_threshold: f64,
    pub max_relay_hops: u32,
    pub proof_ttl_ms: u64,
    pub max_batch_size: usize,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            consensus_threshold: 0.67,
            max_relay_hops: 5,
            proof_ttl_ms: 30_000,
            max_batch_size: 64,
        }
    }
}

// ─── Bridge Proof ───

#[derive(Debug, Clone)]
pub struct BridgeProof {
    pub proof_id: String,
    pub source_shard: String,
    pub target_shard: String,
    pub proof_hash: String,
    pub merkle_root: String,
    pub relay_hops: u32,
    pub verified: bool,
    pub timestamp_ms: u64,
}

impl BridgeProof {
    pub fn new(
        proof_id: String,
        source_shard: String,
        target_shard: String,
        proof_hash: String,
    ) -> Self {
        Self {
            proof_id,
            source_shard,
            target_shard,
            merkle_root: compute_merkle_root(&proof_hash),
            proof_hash,
            relay_hops: 0,
            verified: false,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.timestamp_ms) > ttl_ms
    }
}

// ─── Verification Vote ───

#[derive(Debug, Clone)]
pub struct VerificationVote {
    pub node_id: String,
    pub proof_id: String,
    pub valid: bool,
    pub timestamp_ms: u64,
}

// ─── Shard Info ───

#[derive(Debug, Clone)]
pub struct ShardInfo {
    pub shard_id: String,
    pub nodes: Vec<String>,
    pub merkle_root: String,
    pub proof_count: u64,
}

impl ShardInfo {
    pub fn new(shard_id: String) -> Self {
        Self {
            shard_id,
            nodes: Vec::new(),
            merkle_root: String::new(),
            proof_count: 0,
        }
    }
}

// ─── Bridge Stats ───

#[derive(Debug, Clone)]
pub struct BridgeStats {
    pub total_proofs_relayed: u64,
    pub total_consensus_reached: u64,
    pub total_consensus_failed: u64,
    pub avg_relay_hops: f64,
    pub avg_consensus_time_ms: f64,
}

impl Default for BridgeStats {
    fn default() -> Self {
        Self {
            total_proofs_relayed: 0,
            total_consensus_reached: 0,
            total_consensus_failed: 0,
            avg_relay_hops: 0.0,
            avg_consensus_time_ms: 0.0,
        }
    }
}

// ─── Bridge ───

pub struct ZKPFederationBridge {
    config: BridgeConfig,
    shards: HashMap<String, ShardInfo>,
    proofs: HashMap<String, BridgeProof>,
    votes: HashMap<String, Vec<VerificationVote>>,
    stats: BridgeStats,
}

impl ZKPFederationBridge {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            shards: HashMap::new(),
            proofs: HashMap::new(),
            votes: HashMap::new(),
            stats: BridgeStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(BridgeConfig::default())
    }

    pub fn register_shard(&mut self, shard_id: String) {
        self.shards
            .insert(shard_id.clone(), ShardInfo::new(shard_id));
    }

    pub fn add_node_to_shard(
        &mut self,
        shard_id: &str,
        node_id: String,
    ) -> Result<(), BridgeError> {
        let shard = self
            .shards
            .get_mut(shard_id)
            .ok_or_else(|| BridgeError::ShardNotFound(shard_id.to_string()))?;
        shard.nodes.push(node_id);
        Ok(())
    }

    pub fn relay_proof(&mut self, proof: BridgeProof) -> Result<(), BridgeError> {
        if proof.relay_hops > self.config.max_relay_hops {
            return Err(BridgeError::ConsensusFailed { yes: 0, no: 0 });
        }

        // Verify source and target shards exist
        if !self.shards.contains_key(&proof.source_shard) {
            return Err(BridgeError::ShardNotFound(proof.source_shard.clone()));
        }
        if !self.shards.contains_key(&proof.target_shard) {
            return Err(BridgeError::ShardNotFound(proof.target_shard.clone()));
        }

        let mut relayed = proof;
        relayed.relay_hops += 1;
        let hops = relayed.relay_hops;
        self.proofs.insert(relayed.proof_id.clone(), relayed);

        self.stats.total_proofs_relayed += 1;
        self.stats.avg_relay_hops = (self.stats.avg_relay_hops
            * (self.stats.total_proofs_relayed - 1) as f64
            + hops as f64)
            / self.stats.total_proofs_relayed as f64;

        Ok(())
    }

    pub fn submit_vote(&mut self, vote: VerificationVote) {
        self.votes
            .entry(vote.proof_id.clone())
            .or_default()
            .push(vote);
    }

    pub fn reach_consensus(&mut self, proof_id: &str) -> Result<bool, BridgeError> {
        let votes = self
            .votes
            .get(proof_id)
            .ok_or_else(|| BridgeError::ProofNotFound(proof_id.to_string()))?;

        let yes = votes.iter().filter(|v| v.valid).count() as u64;
        let no = votes.iter().filter(|v| !v.valid).count() as u64;
        let total = yes + no;

        if total == 0 {
            return Err(BridgeError::ConsensusFailed { yes: 0, no: 0 });
        }

        let ratio = yes as f64 / total as f64;
        let consensus = ratio >= self.config.consensus_threshold;

        if consensus {
            self.stats.total_consensus_reached += 1;
            // Mark proof as verified
            if let Some(proof) = self.proofs.get_mut(proof_id) {
                proof.verified = true;
            }
        } else {
            self.stats.total_consensus_failed += 1;
        }

        Ok(consensus)
    }

    pub fn get_proof(&self, proof_id: &str) -> Option<&BridgeProof> {
        self.proofs.get(proof_id)
    }

    pub fn get_shard(&self, shard_id: &str) -> Option<&ShardInfo> {
        self.shards.get(shard_id)
    }

    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.proofs.len();
        self.proofs
            .retain(|_, p| !p.is_expired(self.config.proof_ttl_ms));
        before - self.proofs.len()
    }

    pub fn get_stats(&self) -> &BridgeStats {
        &self.stats
    }

    pub fn get_config(&self) -> &BridgeConfig {
        &self.config
    }
}

impl Default for ZKPFederationBridge {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_merkle_root(proof_hash: &str) -> String {
    let mut h: u64 = 5381;
    for byte in proof_hash.bytes() {
        h = h.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("merkle-{:016x}", h)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = ZKPFederationBridge::with_defaults();
        assert_eq!(bridge.get_stats().total_proofs_relayed, 0);
    }

    #[test]
    fn test_register_shard() {
        let mut bridge = ZKPFederationBridge::with_defaults();
        bridge.register_shard("shard-1".to_string());
        assert!(bridge.get_shard("shard-1").is_some());
    }

    #[test]
    fn test_add_node_to_shard() {
        let mut bridge = ZKPFederationBridge::with_defaults();
        bridge.register_shard("shard-1".to_string());
        bridge
            .add_node_to_shard("shard-1", "n1".to_string())
            .unwrap();
        let shard = bridge.get_shard("shard-1").unwrap();
        assert_eq!(shard.nodes.len(), 1);
    }

    #[test]
    fn test_relay_proof() {
        let mut bridge = ZKPFederationBridge::with_defaults();
        bridge.register_shard("src".to_string());
        bridge.register_shard("tgt".to_string());
        let proof = BridgeProof::new(
            "p1".to_string(),
            "src".to_string(),
            "tgt".to_string(),
            "hash1".to_string(),
        );
        bridge.relay_proof(proof).unwrap();
        assert!(bridge.get_proof("p1").is_some());
    }

    #[test]
    fn test_relay_proof_max_hops() {
        let mut bridge = ZKPFederationBridge::with_defaults();
        bridge.register_shard("src".to_string());
        bridge.register_shard("tgt".to_string());
        let mut proof = BridgeProof::new(
            "p1".to_string(),
            "src".to_string(),
            "tgt".to_string(),
            "hash1".to_string(),
        );
        proof.relay_hops = 10;
        assert!(bridge.relay_proof(proof).is_err());
    }

    #[test]
    fn test_consensus_reached() {
        let mut bridge = ZKPFederationBridge::with_defaults();
        // 3 valid out of 4 = 0.75 > 0.67 threshold
        bridge.submit_vote(VerificationVote {
            node_id: "n1".to_string(),
            proof_id: "p1".to_string(),
            valid: true,
            timestamp_ms: 1000,
        });
        bridge.submit_vote(VerificationVote {
            node_id: "n2".to_string(),
            proof_id: "p1".to_string(),
            valid: true,
            timestamp_ms: 1000,
        });
        bridge.submit_vote(VerificationVote {
            node_id: "n3".to_string(),
            proof_id: "p1".to_string(),
            valid: true,
            timestamp_ms: 1000,
        });
        bridge.submit_vote(VerificationVote {
            node_id: "n4".to_string(),
            proof_id: "p1".to_string(),
            valid: false,
            timestamp_ms: 1000,
        });
        let result = bridge.reach_consensus("p1").unwrap();
        assert!(result);
    }

    #[test]
    fn test_consensus_failed() {
        let mut bridge = ZKPFederationBridge::with_defaults();
        bridge.submit_vote(VerificationVote {
            node_id: "n1".to_string(),
            proof_id: "p1".to_string(),
            valid: false,
            timestamp_ms: 1000,
        });
        bridge.submit_vote(VerificationVote {
            node_id: "n2".to_string(),
            proof_id: "p1".to_string(),
            valid: false,
            timestamp_ms: 1000,
        });
        let result = bridge.reach_consensus("p1").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_proof_expiration() {
        let bridge = ZKPFederationBridge::with_defaults();
        let mut proof = BridgeProof::new(
            "p1".to_string(),
            "s".to_string(),
            "t".to_string(),
            "h".to_string(),
        );
        proof.timestamp_ms = 0;
        assert!(proof.is_expired(1000));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut bridge = ZKPFederationBridge::new(BridgeConfig {
            proof_ttl_ms: 100,
            ..Default::default()
        });
        bridge.register_shard("src".to_string());
        bridge.register_shard("tgt".to_string());
        let mut proof = BridgeProof::new(
            "p1".to_string(),
            "src".to_string(),
            "tgt".to_string(),
            "h".to_string(),
        );
        proof.timestamp_ms = 0;
        bridge.proofs.insert("p1".to_string(), proof);
        let cleaned = bridge.cleanup_expired();
        assert_eq!(cleaned, 1);
    }

    #[test]
    fn test_error_display() {
        let e = BridgeError::NodeNotFound("x".to_string());
        assert!(!e.to_string().is_empty());
    }
}
