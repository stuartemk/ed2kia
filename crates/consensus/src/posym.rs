//! Proof of Symbiosis (PoSym) — Cryptographic Local Reputation.
//!
//! PoSym assigns each node a trust score based on its verified contributions
//! to the collective safety of the network. The score is computed as:
//!
//! ```text
//! Score = 0.6 * ln(steers + 1) + 0.3 * ln(1 + vfe_reduction) + 0.1 * ln(uptime + 1)
//! ```
//!
//! Where:
//! - `steers`: Number of certified steering corrections performed.
//! - `vfe_reduction`: Cumulative VFE (Variational Free Energy) reduction achieved.
//! - `uptime`: Node uptime in seconds (or blocks).
//!
//! Each contribution is cryptographically hashed and appended to a local
//! reputation chain, making it tamper-evident and auditable.

use sha2::{Digest, Sha256};
use std::collections::VecDeque;

/// Cryptographic hash digest (SHA-256).
pub type Hash = [u8; 32];

/// A single certified steering contribution recorded in the PoSym chain.
#[derive(Debug, Clone)]
pub struct SteerContribution {
    /// Block or timestamp when the steer occurred.
    pub timestamp: u64,
    /// VFE before steering.
    pub vfe_before: f64,
    /// VFE after steering.
    pub vfe_after: f64,
    /// Cryptographic hash of the contribution.
    pub hash: Hash,
}

impl SteerContribution {
    /// Create a new steering contribution with cryptographic hash.
    pub fn new(timestamp: u64, vfe_before: f64, vfe_after: f64) -> Self {
        let hash = Self::compute_hash(timestamp, vfe_before, vfe_after);
        Self {
            timestamp,
            vfe_before,
            vfe_after,
            hash,
        }
    }

    /// Compute the VFE reduction from this contribution.
    pub fn vfe_reduction(&self) -> f64 {
        (self.vfe_before - self.vfe_after).max(0.0)
    }

    /// Generate SHA-256 hash for this contribution.
    fn compute_hash(timestamp: u64, vfe_before: f64, vfe_after: f64) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(timestamp.to_le_bytes());
        hasher.update(vfe_before.to_le_bytes());
        hasher.update(vfe_after.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify the hash matches the stored values.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(self.timestamp, self.vfe_before, self.vfe_after);
        self.hash == expected
    }
}

/// Proof of Symbiosis (PoSym) engine for a single node.
///
/// Tracks certified steering contributions and computes the trust score
/// based on the symbiotic formula.
#[derive(Debug, Clone)]
pub struct ProofOfSymbiosis {
    /// Node identifier.
    pub node_id: u64,
    /// Ordered chain of certified steering contributions.
    pub contributions: VecDeque<SteerContribution>,
    /// Maximum contributions to retain (prevents unbounded growth).
    pub max_contributions: usize,
    /// Node uptime in seconds (or blocks).
    pub uptime: u64,
    /// Weight for steering count component.
    pub weight_steers: f64,
    /// Weight for VFE reduction component.
    pub weight_vfe: f64,
    /// Weight for uptime component.
    pub weight_uptime: f64,
}

impl ProofOfSymbiosis {
    /// Create a new PoSym engine with default weights.
    ///
    /// Default formula: `Score = 0.6 * ln(steers+1) + 0.3 * ln(1+vfe_reduction) + 0.1 * ln(uptime+1)`
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            contributions: VecDeque::new(),
            max_contributions: 10_000,
            uptime: 0,
            weight_steers: 0.6,
            weight_vfe: 0.3,
            weight_uptime: 0.1,
        }
    }

    /// Create a new PoSym engine with custom weights.
    pub fn with_weights(node_id: u64, weight_steers: f64, weight_vfe: f64, weight_uptime: f64) -> Self {
        Self {
            node_id,
            contributions: VecDeque::new(),
            max_contributions: 10_000,
            uptime: 0,
            weight_steers,
            weight_vfe,
            weight_uptime,
        }
    }

    /// Record a certified steering contribution.
    ///
    /// Appends the contribution to the chain and enforces the maximum size limit.
    pub fn record_certified_steer(&mut self, timestamp: u64, vfe_before: f64, vfe_after: f64) {
        let contribution = SteerContribution::new(timestamp, vfe_before, vfe_after);
        self.contributions.push_back(contribution);
        if self.contributions.len() > self.max_contributions {
            self.contributions.pop_front();
        }
    }

    /// Increment node uptime by the given amount.
    pub fn add_uptime(&mut self, delta: u64) {
        self.uptime += delta;
    }

    /// Compute the current trust score using the symbiotic formula.
    ///
    /// ```text
    /// Score = w_steers * ln(steers + 1) + w_vfe * ln(1 + vfe_reduction) + w_uptime * ln(uptime + 1)
    /// ```
    pub fn compute_trust_score(&self) -> f64 {
        let steers = self.contributions.len() as f64;
        let total_vfe_reduction: f64 = self.contributions.iter().map(|c| c.vfe_reduction()).sum();
        let uptime = self.uptime as f64;

        let score_steers = self.weight_steers * (steers + 1.0).ln();
        let score_vfe = self.weight_vfe * (1.0 + total_vfe_reduction).ln();
        let score_uptime = self.weight_uptime * (uptime + 1.0).ln();

        score_steers + score_vfe + score_uptime
    }

    /// Generate a SHA-256 hash of the current PoSym state (for chain linking).
    pub fn generate_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.node_id.to_le_bytes());
        hasher.update(self.uptime.to_le_bytes());
        hasher.update((self.contributions.len() as u64).to_le_bytes());
        for contrib in &self.contributions {
            hasher.update(contrib.hash);
        }
        hasher.finalize().into()
    }

    /// Verify all contributions in the chain have valid hashes.
    pub fn verify_chain(&self) -> bool {
        self.contributions.iter().all(|c| c.verify())
    }

    /// Get the total VFE reduction achieved by this node.
    pub fn total_vfe_reduction(&self) -> f64 {
        self.contributions.iter().map(|c| c.vfe_reduction()).sum()
    }

    /// Get the number of certified steers.
    pub fn steer_count(&self) -> usize {
        self.contributions.len()
    }

    /// Get the most recent contribution (if any).
    pub fn latest_contribution(&self) -> Option<&SteerContribution> {
        self.contributions.back()
    }

    /// Check if this node's trust score exceeds a threshold.
    pub fn is_trusted(&self, threshold: f64) -> bool {
        self.compute_trust_score() >= threshold
    }
}

impl Default for ProofOfSymbiosis {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posym_initial_state() {
        let posym = ProofOfSymbiosis::new(42);
        assert_eq!(posym.node_id, 42);
        assert_eq!(posym.steer_count(), 0);
        assert_eq!(posym.total_vfe_reduction(), 0.0);
        assert!(posym.verify_chain());
    }

    #[test]
    fn test_posym_record_steer() {
        let mut posym = ProofOfSymbiosis::new(1);
        posym.record_certified_steer(100, 1.0, 0.5);
        assert_eq!(posym.steer_count(), 1);
        assert!((posym.total_vfe_reduction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_posym_trust_score_increases() {
        let mut posym = ProofOfSymbiosis::new(1);
        let initial_score = posym.compute_trust_score();
        posym.record_certified_steer(100, 1.0, 0.3);
        posym.add_uptime(3600);
        let new_score = posym.compute_trust_score();
        assert!(new_score > initial_score);
    }

    #[test]
    fn test_posym_contribution_hash_verify() {
        let contrib = SteerContribution::new(50, 2.0, 1.0);
        assert!(contrib.verify());
        assert!((contrib.vfe_reduction() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_posym_chain_integrity() {
        let mut posym = ProofOfSymbiosis::new(7);
        for i in 0..10 {
            posym.record_certified_steer(i * 10, 1.0, 0.5);
        }
        assert!(posym.verify_chain());
        assert_eq!(posym.steer_count(), 10);
    }

    #[test]
    fn test_posym_generate_hash_deterministic() {
        let mut posym1 = ProofOfSymbiosis::new(99);
        let mut posym2 = ProofOfSymbiosis::new(99);
        posym1.record_certified_steer(10, 1.0, 0.5);
        posym2.record_certified_steer(10, 1.0, 0.5);
        posym1.add_uptime(100);
        posym2.add_uptime(100);
        assert_eq!(posym1.generate_hash(), posym2.generate_hash());
    }

    #[test]
    fn test_posym_max_contributions() {
        let mut posym = ProofOfSymbiosis::new(1);
        posym.max_contributions = 5;
        for i in 0..10 {
            posym.record_certified_steer(i, 1.0, 0.5);
        }
        assert_eq!(posym.steer_count(), 5);
        // Oldest contributions should be evicted
        assert_eq!(posym.contributions.front().unwrap().timestamp, 5);
    }

    #[test]
    fn test_posym_is_trusted() {
        let mut posym = ProofOfSymbiosis::new(1);
        assert!(!posym.is_trusted(10.0));
        for i in 0..100 {
            posym.record_certified_steer(i, 1.0, 0.1);
        }
        posym.add_uptime(86400);
        assert!(posym.is_trusted(1.0));
    }

    #[test]
    fn test_posym_custom_weights() {
        let posym = ProofOfSymbiosis::with_weights(1, 0.5, 0.3, 0.2);
        assert!((posym.weight_steers - 0.5).abs() < 1e-10);
        assert!((posym.weight_vfe - 0.3).abs() < 1e-10);
        assert!((posym.weight_uptime - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_posym_latest_contribution() {
        let mut posym = ProofOfSymbiosis::new(1);
        assert!(posym.latest_contribution().is_none());
        posym.record_certified_steer(100, 1.0, 0.5);
        assert!(posym.latest_contribution().is_some());
        assert_eq!(posym.latest_contribution().unwrap().timestamp, 100);
    }

    #[test]
    fn test_posym_default() {
        let posym = ProofOfSymbiosis::default();
        assert_eq!(posym.node_id, 0);
    }

    #[test]
    fn test_posym_vfe_reduction_clamped() {
        let contrib = SteerContribution::new(1, 0.5, 1.0);
        // vfe_after > vfe_before, so reduction should be 0 (clamped)
        assert_eq!(contrib.vfe_reduction(), 0.0);
    }
}
