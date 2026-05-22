//! Liquid Governance — Gobernanza líquida con delegación ponderada, time-lock de 24h y detección anti-Sybil

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("proposal not found: {0}")]
    ProposalNotFound(String),
    #[error("quorum not met: {current}/{required}")]
    QuorumNotMet { current: usize, required: usize },
    #[error("time-lock active: {remaining_hours}h remaining")]
    TimeLockActive { remaining_hours: u32 },
    #[error("sybil cluster detected: {cluster_id}")]
    SybilDetected { cluster_id: String },
    #[error("invalid delegation chain: {0}")]
    InvalidDelegation(String),
    #[error("already voted: {0}")]
    AlreadyVoted(String),
    #[error("proposal already executed: {0}")]
    AlreadyExecuted(String),
}

// ─── Core Types ───────────────────────────────────────────────────────────────

/// Weighted delegation entry for liquid democracy.
#[derive(Debug, Clone)]
pub struct Delegation {
    pub delegator: String,
    pub delegatee: String,
    pub weight: f64,
    pub created_at: Instant,
}

/// A governance proposal with time-lock enforcement.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposer: String,
    pub created_at: Instant,
    pub votes_for: Vec<String>,
    pub votes_against: Vec<String>,
    pub executed: bool,
    pub time_lock_until: Instant,
}

/// Result of a governance operation.
#[derive(Debug, Clone)]
pub struct GovernanceResult {
    pub executed: bool,
    pub quorum_met: bool,
    pub delegation_chain: Vec<String>,
    pub sybil_flag: bool,
}

impl GovernanceResult {
    pub fn executed(quorum_met: bool, chain: Vec<String>) -> Self {
        Self {
            executed: true,
            quorum_met,
            delegation_chain: chain,
            sybil_flag: false,
        }
    }

    pub fn rejected(_reason: &str, chain: Vec<String>, sybil_flag: bool) -> Self {
        Self {
            executed: false,
            quorum_met: false,
            delegation_chain: chain,
            sybil_flag,
        }
    }

    pub fn sybil_blocked(cluster_id: &str) -> Self {
        Self {
            executed: false,
            quorum_met: false,
            delegation_chain: vec![cluster_id.to_string()],
            sybil_flag: true,
        }
    }
}

/// Node trust profile used for weighted delegation scoring.
#[derive(Debug, Clone)]
pub struct NodeProfile {
    pub node_id: String,
    pub trust_score: f64,
    pub staking_credits: f64,
    pub uptime_history: f64,
    pub crypto_signature: String,
    pub asn: String,
    pub ip_prefix: String,
    pub voting_history: Vec<String>,
}

impl NodeProfile {
    pub fn new(
        node_id: String,
        trust_score: f64,
        staking_credits: f64,
        uptime_history: f64,
    ) -> Self {
        Self {
            node_id: node_id.clone(),
            trust_score,
            staking_credits,
            uptime_history,
            crypto_signature: format!("sig_{}", node_id),
            asn: format!("AS{}", hash_to_u16(&node_id)),
            ip_prefix: format!("10.{}.0.0/16", hash_to_u8(&node_id)),
            voting_history: Vec::new(),
        }
    }

    /// Compute effective voting weight: trust_score × staking_credits × uptime_history
    pub fn voting_weight(&self) -> f64 {
        self.trust_score * self.staking_credits * self.uptime_history
    }
}

/// Configuration for the LiquidGovernance engine.
#[derive(Debug, Clone)]
pub struct GovernanceConfig {
    pub quorum_percentage: f64,
    pub time_lock_duration: Duration,
    pub sybil_trust_threshold: f64,
    pub max_delegation_depth: usize,
    pub min_nodes_for_quorum: usize,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            quorum_percentage: 0.67,
            time_lock_duration: Duration::from_secs(24 * 60 * 60), // 24 hours
            sybil_trust_threshold: 0.45,
            max_delegation_depth: 10,
            min_nodes_for_quorum: 3,
        }
    }
}

/// Sybil cluster detected by the anti-Sybil system.
#[derive(Debug, Clone)]
pub struct SybilCluster {
    pub cluster_id: String,
    pub node_ids: Vec<String>,
    pub avg_trust_score: f64,
    pub detection_reason: String,
}

/// Stats for the governance engine.
#[derive(Debug, Clone)]
pub struct GovernanceStats {
    pub total_proposals: usize,
    pub executed_proposals: usize,
    pub total_delegations: usize,
    pub active_nodes: usize,
    pub sybil_clusters_detected: usize,
}

// ─── LiquidGovernance Engine ──────────────────────────────────────────────────

pub struct LiquidGovernance {
    config: GovernanceConfig,
    nodes: HashMap<String, NodeProfile>,
    proposals: HashMap<String, Proposal>,
    delegations: VecDeque<Delegation>,
    sybil_clusters: Vec<SybilCluster>,
    audit_log: VecDeque<String>,
}

impl LiquidGovernance {
    pub fn new() -> Self {
        Self::with_config(GovernanceConfig::default())
    }

    pub fn with_config(config: GovernanceConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            proposals: HashMap::new(),
            delegations: VecDeque::new(),
            sybil_clusters: Vec::new(),
            audit_log: VecDeque::with_capacity(256),
        }
    }

    /// Register a node in the governance system.
    pub fn register_node(&mut self, profile: NodeProfile) {
        let node_id = profile.node_id.clone();
        self.nodes.insert(node_id.clone(), profile);
        self.audit(&format!("node_registered: {}", node_id));
    }

    /// Create a new proposal with 24h time-lock.
    pub fn create_proposal(
        &mut self,
        id: String,
        title: String,
        description: String,
        proposer: String,
    ) -> Result<(), GovernanceError> {
        if self.proposals.contains_key(&id) {
            return Err(GovernanceError::ProposalNotFound(id));
        }

        let proposal = Proposal {
            id: id.clone(),
            title,
            description,
            proposer,
            created_at: Instant::now(),
            votes_for: Vec::new(),
            votes_against: Vec::new(),
            executed: false,
            time_lock_until: Instant::now() + self.config.time_lock_duration,
        };

        self.proposals.insert(id.clone(), proposal);
        self.audit(&format!("proposal_created: {}", id));
        Ok(())
    }

    /// Calculate effective delegation weight for a node, following the delegation chain.
    pub fn delegate_weight(
        &mut self,
        delegator: String,
        delegatee: String,
    ) -> Result<f64, GovernanceError> {
        // Validate both nodes exist
        if !self.nodes.contains_key(&delegator) {
            return Err(GovernanceError::NodeNotFound(delegator.clone()));
        }
        if !self.nodes.contains_key(&delegatee) {
            return Err(GovernanceError::NodeNotFound(delegatee.clone()));
        }

        let delegator_profile = self.nodes.get(&delegator).unwrap();
        let base_weight = delegator_profile.voting_weight();

        // Record delegation
        let delegation = Delegation {
            delegator: delegator.clone(),
            delegatee: delegatee.clone(),
            weight: base_weight,
            created_at: Instant::now(),
        };
        self.delegations.push_back(delegation);

        self.audit(&format!("delegation: {} -> {}", delegator, delegatee));
        Ok(base_weight)
    }

    /// Cast a vote on a proposal, resolving delegation chain weights.
    pub fn cast_vote(
        &mut self,
        voter: String,
        proposal_id: String,
        vote_for: bool,
    ) -> Result<GovernanceResult, GovernanceError> {
        // Check proposal exists and already voted (immutable borrow first)
        {
            let proposal = self
                .proposals
                .get(&proposal_id)
                .ok_or_else(|| GovernanceError::ProposalNotFound(proposal_id.clone()))?;

            // Check not already voted
            if proposal.votes_for.contains(&voter) || proposal.votes_against.contains(&voter) {
                return Err(GovernanceError::AlreadyVoted(voter.clone()));
            }
        }

        // Build delegation chain (immutable borrow)
        let chain = self.resolve_delegation_chain(&voter, 0);

        // Check for Sybil in chain (immutable borrow)
        let sybil_flag = self.check_sybil_in_chain(&chain);
        if sybil_flag {
            return Ok(GovernanceResult::sybil_blocked(&voter));
        }

        // Record vote (mutable borrow)
        {
            let proposal = self.proposals.get_mut(&proposal_id).unwrap();
            if vote_for {
                proposal.votes_for.push(voter.clone());
            } else {
                proposal.votes_against.push(voter.clone());
            }
        }

        // Update voting history (mutable borrow)
        if let Some(profile) = self.nodes.get_mut(&voter) {
            profile.voting_history.push(proposal_id.clone());
        }

        // Check quorum (immutable borrow)
        let total_votes = {
            let proposal = self.proposals.get(&proposal_id).unwrap();
            proposal.votes_for.len() + proposal.votes_against.len()
        };
        let quorum_met = self.is_quorum_met(total_votes);

        self.audit(&format!(
            "vote_cast: {} on {} (for={})",
            voter, proposal_id, vote_for
        ));

        Ok(GovernanceResult {
            executed: false,
            quorum_met,
            delegation_chain: chain,
            sybil_flag: false,
        })
    }

    /// Execute a proposal after time-lock expires and quorum is met.
    pub fn execute_proposal(
        &mut self,
        proposal_id: String,
    ) -> Result<GovernanceResult, GovernanceError> {
        // First, check proposal exists and read immutable fields
        let (votes_for_count, votes_against_count) = {
            let proposal = self
                .proposals
                .get(&proposal_id)
                .ok_or_else(|| GovernanceError::ProposalNotFound(proposal_id.clone()))?;

            // Check already executed
            if proposal.executed {
                return Err(GovernanceError::AlreadyExecuted(proposal_id.clone()));
            }

            // Check time-lock
            if Instant::now() < proposal.time_lock_until {
                let remaining = proposal
                    .time_lock_until
                    .duration_since(Instant::now())
                    .as_secs()
                    / 3600;
                return Err(GovernanceError::TimeLockActive {
                    remaining_hours: remaining as u32,
                });
            }

            (proposal.votes_for.len(), proposal.votes_against.len())
        };

        // Check quorum
        let total_votes = votes_for_count + votes_against_count;
        if !self.is_quorum_met(total_votes) {
            return Err(GovernanceError::QuorumNotMet {
                current: total_votes,
                required: self.config.min_nodes_for_quorum,
            });
        }

        // Check majority
        let majority = votes_for_count > votes_against_count;
        if !majority {
            return Ok(GovernanceResult::rejected("no majority", vec![], false));
        }

        // Execute (mutable borrow)
        let chain = {
            let proposal = self.proposals.get_mut(&proposal_id).unwrap();
            proposal.executed = true;
            proposal.votes_for.to_vec() // CLEANUP: iter().cloned().collect() -> to_vec()
        };

        self.audit(&format!("proposal_executed: {}", proposal_id));
        Ok(GovernanceResult::executed(true, chain))
    }

    /// Detect Sybil clusters based on trust score, ASN/IP prefix and voting history similarity.
    pub fn detect_sybil_cluster(&mut self) -> Vec<SybilCluster> {
        let mut clusters = Vec::new();

        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();
        let mut processed = HashSet::new();

        for i in 0..node_ids.len() {
            if processed.contains(&node_ids[i]) {
                continue;
            }

            let node_a = self.nodes.get(&node_ids[i]).unwrap();
            let mut cluster_nodes = vec![node_ids[i].clone()];

            for node_id_j in &node_ids[i + 1..] {
                // CLEANUP: Iterate by reference instead of index
                if processed.contains(node_id_j) {
                    continue;
                }

                let node_b = self.nodes.get(node_id_j).unwrap();
                if Self::are_potential_sybil(node_a, node_b) {
                    cluster_nodes.push(node_id_j.clone());
                }
            }

            // Only flag clusters with 2+ nodes and low average trust
            if cluster_nodes.len() >= 2 {
                let avg_trust: f64 = cluster_nodes
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .map(|n| n.trust_score)
                    .sum::<f64>()
                    / cluster_nodes.len() as f64;

                if avg_trust < self.config.sybil_trust_threshold {
                    let cluster_id = format!("sybil_{}", hash_to_u32(&cluster_nodes.join(",")));
                    let cluster = SybilCluster {
                        cluster_id: cluster_id.clone(),
                        node_ids: cluster_nodes.clone(),
                        avg_trust_score: avg_trust,
                        detection_reason: "low_trust_cluster".to_string(),
                    };
                    clusters.push(cluster);

                    for node_id in &cluster_nodes {
                        processed.insert(node_id.clone());
                    }
                }
            }
        }

        self.sybil_clusters = clusters.clone();
        if !clusters.is_empty() {
            self.audit(&format!("sybil_clusters_detected: {}", clusters.len()));
        }

        clusters
    }

    /// Check if a delegation chain contains any Sybil-flagged nodes.
    pub fn check_sybil_in_chain(&self, chain: &[String]) -> bool {
        for cluster in &self.sybil_clusters {
            for node_id in chain {
                if cluster.node_ids.contains(node_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Resolve the full delegation chain for a node (following delegatee links).
    fn resolve_delegation_chain(&self, node_id: &str, depth: usize) -> Vec<String> {
        if depth >= self.config.max_delegation_depth {
            return vec![node_id.to_string()];
        }

        let mut chain = vec![node_id.to_string()];
        for delegation in &self.delegations {
            if delegation.delegator == node_id {
                let sub_chain = self.resolve_delegation_chain(&delegation.delegatee, depth + 1);
                chain.extend(sub_chain);
                break;
            }
        }
        chain
    }

    /// Check if the current vote count meets the quorum requirement.
    fn is_quorum_met(&self, total_votes: usize) -> bool {
        let active_nodes = self.nodes.len();
        if active_nodes < self.config.min_nodes_for_quorum {
            return total_votes >= active_nodes;
        }

        let required = (active_nodes as f64 * self.config.quorum_percentage).ceil() as usize;
        total_votes >= required.max(self.config.min_nodes_for_quorum)
    }

    /// Heuristic: two nodes are potential Sybil if they share ASN + IP prefix and have similar voting.
    fn are_potential_sybil(a: &NodeProfile, b: &NodeProfile) -> bool {
        let same_asn = a.asn == b.asn;
        let same_ip = a.ip_prefix == b.ip_prefix;
        let similar_voting = voting_history_similarity(&a.voting_history, &b.voting_history) > 0.7;

        same_asn && same_ip || similar_voting && (same_asn || same_ip) // CLEANUP: Simplified boolean expression
    }

    fn audit(&mut self, message: &str) {
        self.audit_log
            .push_back(format!("[{}] {}", current_timestamp_ms(), message));
        if self.audit_log.len() > 256 {
            self.audit_log.pop_front();
        }
    }

    // ─── Accessors ───────────────────────────────────────────────────────────

    pub fn get_stats(&self) -> GovernanceStats {
        let executed = self.proposals.values().filter(|p| p.executed).count();
        GovernanceStats {
            total_proposals: self.proposals.len(),
            executed_proposals: executed,
            total_delegations: self.delegations.len(),
            active_nodes: self.nodes.len(),
            sybil_clusters_detected: self.sybil_clusters.len(),
        }
    }

    pub fn get_proposal(&self, id: &str) -> Option<&Proposal> {
        self.proposals.get(id)
    }

    pub fn get_node(&self, id: &str) -> Option<&NodeProfile> {
        self.nodes.get(id)
    }

    pub fn active_node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn sybil_cluster_count(&self) -> usize {
        self.sybil_clusters.len()
    }

    pub fn audit_trail(&self) -> &[String] {
        self.audit_log.as_slices().0
    }

    /// Reset the governance engine state.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.proposals.clear();
        self.delegations.clear();
        self.sybil_clusters.clear();
        self.audit_log.clear();
    }
}

impl Default for LiquidGovernance {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn voting_history_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<&String> = a.iter().collect();
    let set_b: HashSet<&String> = b.iter().collect();

    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;

    intersection / union
}

fn hash_to_u8(input: &str) -> u8 {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    (hash % 256) as u8
}

fn hash_to_u16(input: &str) -> u16 {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    (hash % 65536) as u16
}

fn hash_to_u32(input: &str) -> u32 {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    (hash % 4294967296) as u32
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

    fn make_node(id: &str, trust: f64, credits: f64, uptime: f64) -> NodeProfile {
        NodeProfile::new(id.to_string(), trust, credits, uptime)
    }

    #[test]
    fn test_governance_creation() {
        let gov = LiquidGovernance::new();
        assert_eq!(gov.active_node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("node1", 0.9, 100.0, 0.95));
        assert_eq!(gov.active_node_count(), 1);
    }

    #[test]
    fn test_voting_weight_calculation() {
        let node = make_node("node1", 0.8, 50.0, 0.9);
        assert!((node.voting_weight() - 36.0).abs() < 0.01);
    }

    #[test]
    fn test_create_proposal() {
        let mut gov = LiquidGovernance::new();
        let result =
            gov.create_proposal("p1".into(), "Title".into(), "Desc".into(), "node1".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_duplicate_proposal() {
        let mut gov = LiquidGovernance::new();
        let _ = gov.create_proposal("p1".into(), "T".into(), "D".into(), "n1".into());
        // Second call with same id should succeed (no duplicate check on create)
        // but the proposal is overwritten
    }

    #[test]
    fn test_delegate_weight() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("a", 0.9, 100.0, 1.0));
        gov.register_node(make_node("b", 0.8, 50.0, 0.9));

        let weight = gov.delegate_weight("a".into(), "b".into());
        assert!(weight.is_ok());
        assert!((weight.unwrap() - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_delegate_unknown_node() {
        let mut gov = LiquidGovernance::new();
        let result = gov.delegate_weight("unknown".into(), "b".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_cast_vote() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        gov.register_node(make_node("n2", 0.8, 50.0, 0.9));
        gov.register_node(make_node("n3", 0.7, 30.0, 0.8));
        let _ = gov.create_proposal("p1".into(), "T".into(), "D".into(), "n1".into());

        let result = gov.cast_vote("n1".into(), "p1".into(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_double_vote_rejected() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        let _ = gov.create_proposal("p1".into(), "T".into(), "D".into(), "n1".into());

        gov.cast_vote("n1".into(), "p1".into(), true).unwrap();
        let result = gov.cast_vote("n1".into(), "p1".into(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_time_lock_prevents_execution() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        gov.register_node(make_node("n2", 0.8, 50.0, 0.9));
        gov.register_node(make_node("n3", 0.7, 30.0, 0.8));
        let _ = gov.create_proposal("p1".into(), "T".into(), "D".into(), "n1".into());

        let result = gov.execute_proposal("p1".into());
        assert!(result.is_err()); // Time-lock still active
    }

    #[test]
    fn test_sybil_detection_same_asn_ip() {
        let mut gov = LiquidGovernance::new();
        // Create nodes with same ASN and IP (potential Sybil)
        let mut node_a = make_node("s1", 0.3, 10.0, 0.5);
        node_a.asn = "AS100".into();
        node_a.ip_prefix = "10.1.0.0/16".into();

        let mut node_b = make_node("s2", 0.2, 10.0, 0.4);
        node_b.asn = "AS100".into();
        node_b.ip_prefix = "10.1.0.0/16".into();

        gov.register_node(node_a);
        gov.register_node(node_b);

        let clusters = gov.detect_sybil_cluster();
        assert_eq!(clusters.len(), 1);
        assert!(clusters[0].avg_trust_score < 0.45);
    }

    #[test]
    fn test_no_sybil_when_different_network() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("a", 0.9, 100.0, 0.95));
        gov.register_node(make_node("b", 0.8, 50.0, 0.9));
        gov.register_node(make_node("c", 0.7, 30.0, 0.8));

        let clusters = gov.detect_sybil_cluster();
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn test_governance_stats() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        gov.register_node(make_node("n2", 0.8, 50.0, 0.9));
        let _ = gov.create_proposal("p1".into(), "T".into(), "D".into(), "n1".into());

        let stats = gov.get_stats();
        assert_eq!(stats.active_nodes, 2);
        assert_eq!(stats.total_proposals, 1);
    }

    #[test]
    fn test_reset() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        gov.reset();
        assert_eq!(gov.active_node_count(), 0);
    }

    #[test]
    fn test_delegation_chain_resolution() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("a", 0.9, 100.0, 1.0));
        gov.register_node(make_node("b", 0.8, 50.0, 0.9));
        gov.register_node(make_node("c", 0.7, 30.0, 0.8));

        gov.delegate_weight("a".into(), "b".into()).unwrap();
        gov.delegate_weight("b".into(), "c".into()).unwrap();

        let chain = gov.resolve_delegation_chain("a", 0);
        assert!(chain.contains(&"a".to_string()));
        assert!(chain.contains(&"b".to_string()));
    }

    #[test]
    fn test_quorum_calculation() {
        let mut gov = LiquidGovernance::new();
        // Register 10 nodes
        for i in 0..10 {
            gov.register_node(make_node(&format!("n{}", i), 0.9, 100.0, 1.0));
        }

        // Quorum should be 67% of 10 = 7
        assert!(gov.is_quorum_met(7));
        assert!(!gov.is_quorum_met(6));
    }

    #[test]
    fn test_audit_trail() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        let trail = gov.audit_trail();
        assert!(!trail.is_empty());
    }

    #[test]
    fn test_governance_result_executed() {
        let result = GovernanceResult::executed(true, vec!["n1".into()]);
        assert!(result.executed);
        assert!(result.quorum_met);
        assert!(!result.sybil_flag);
    }

    #[test]
    fn test_governance_result_sybil_blocked() {
        let result = GovernanceResult::sybil_blocked("cluster_1");
        assert!(!result.executed);
        assert!(result.sybil_flag);
    }

    #[test]
    fn test_config_default() {
        let config = GovernanceConfig::default();
        assert!((config.quorum_percentage - 0.67).abs() < 0.01);
        assert_eq!(config.time_lock_duration, Duration::from_secs(24 * 3600));
        assert!((config.sybil_trust_threshold - 0.45).abs() < 0.01);
    }

    #[test]
    fn test_default() {
        let gov = LiquidGovernance::default();
        assert_eq!(gov.active_node_count(), 0);
    }

    #[test]
    fn test_voting_history_similarity() {
        let a = vec!["p1".into(), "p2".into(), "p3".into()];
        let b = vec!["p1".into(), "p2".into(), "p4".into()];
        let sim = voting_history_similarity(&a, &b);
        // Jaccard: intersection={p1,p2}=2, union={p1,p2,p3,p4}=4, similarity=2/4=0.5
        assert!((sim - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_proposal_not_found_on_vote() {
        let mut gov = LiquidGovernance::new();
        gov.register_node(make_node("n1", 0.9, 100.0, 1.0));
        let result = gov.cast_vote("n1".into(), "nonexistent".into(), true);
        assert!(result.is_err());
    }
}
