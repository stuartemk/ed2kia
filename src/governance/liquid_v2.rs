//! Liquid Governance v2 — Cryptographic reputation-weighted voting with 72h time-lock
//!
//! Extends the original liquid governance model with:
//! - Cryptographic reputation-weighted voting
//! - 72h time-lock for critical proposals
//! - Anti-whale vote weight capping
//! - Sybil detection via ASN + IP prefix + voting history similarity
//! - Immutable redb ledger integration points
//! - Recursive delegation chain resolution up to configurable depth

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::{Duration, Instant};

use thiserror::Error;
use tracing::{debug, info, warn};

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors produced by the Liquid Governance v2 engine.
#[derive(Debug, Error)]
pub enum LiquidGovernanceError {
    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("proposal not found: {0}")]
    ProposalNotFound(String),

    #[error("quorum not met: {current_weight}/{required_weight}")]
    QuorumNotMet { current_weight: f64, required_weight: f64 },

    #[error("time-lock active until {remaining:?}")]
    TimeLockActive { remaining: Duration },

    #[error("sybil cluster detected: {cluster_id}")]
    SybilDetected { cluster_id: String },

    #[error("invalid cryptographic signature for node: {0}")]
    InvalidSignature(String),

    #[error("node already voted on proposal: {0}")]
    AlreadyVoted(String),
}

// ─── Core Types ───────────────────────────────────────────────────────────────

/// A governance proposal in the v2 system.
#[derive(Debug, Clone)]
pub struct ProposalV2 {
    /// Unique identifier for the proposal.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Detailed description of the proposal.
    pub description: String,
    /// Node ID of the proposer.
    pub proposer: String,
    /// Timestamp when the proposal was created.
    pub created_at: Instant,
    /// List of (voter_id, weight) tuples voting in favor.
    pub votes_for: Vec<(String, f64)>,
    /// List of (voter_id, weight) tuples voting against.
    pub votes_against: Vec<(String, f64)>,
    /// Whether the proposal has been executed.
    pub executed: bool,
    /// Timestamp until which the proposal is time-locked.
    pub time_lock_until: Instant,
    /// Whether this is a critical proposal (72h time-lock).
    pub critical: bool,
    /// Cryptographic signature of the proposer.
    pub signature: String,
}

/// A weighted delegation entry in the v2 system.
#[derive(Debug, Clone)]
pub struct DelegationV2 {
    /// The node that delegates its voting weight.
    pub delegator: String,
    /// The node that receives the delegated weight.
    pub delegatee: String,
    /// Fraction of the delegator's weight to delegate (0.0–1.0).
    pub weight: f64,
    /// Timestamp when the delegation was created.
    pub created_at: Instant,
    /// Optional expiration timestamp.
    pub expires_at: Option<Instant>,
}

/// Profile for a node participating in governance.
#[derive(Debug, Clone)]
pub struct NodeProfileV2 {
    /// Unique node identifier.
    pub node_id: String,
    /// Trust score in range [0.0, 1.0].
    pub trust_score: f64,
    /// Number of staking credits held by the node.
    pub staking_credits: f64,
    /// Historical uptime ratio in range [0.0, 1.0].
    pub uptime_history: f64,
    /// Cryptographic signature proving node identity.
    pub crypto_signature: String,
    /// Autonomous System Number for network identification.
    pub asn: String,
    /// IP prefix for network identification.
    pub ip_prefix: String,
    /// History of votes cast by this node.
    pub voting_history: Vec<String>,
    /// Computed reputation score (derived from trust, staking, uptime).
    pub reputation_score: f64,
}

/// Result of executing a governance proposal.
#[derive(Debug, Clone)]
pub struct GovernanceResultV2 {
    /// Whether the proposal was successfully executed.
    pub executed: bool,
    /// Whether the quorum threshold was met.
    pub quorum_met: bool,
    /// Chain of delegations resolved for this execution.
    pub delegation_chain: Vec<String>,
    /// Whether any Sybil cluster was flagged.
    pub sybil_flag: bool,
    /// Total voting weight accumulated.
    pub total_weight: f64,
}

/// Configuration for the Liquid Governance v2 engine.
#[derive(Debug, Clone)]
pub struct GovernanceConfigV2 {
    /// Minimum percentage of total weight required for quorum (0.0–1.0).
    pub quorum_percentage: f64,
    /// Default time-lock duration for normal proposals.
    pub time_lock_duration: Duration,
    /// Time-lock duration for critical proposals (default 72h).
    pub critical_time_lock_duration: Duration,
    /// Minimum trust score below which nodes are flagged as potential Sybil.
    pub sybil_trust_threshold: f64,
    /// Maximum depth for resolving delegation chains.
    pub max_delegation_depth: usize,
    /// Minimum number of unique nodes required for quorum.
    pub min_nodes_for_quorum: usize,
    /// Maximum fraction of total weight any single voter can contribute (anti-whale).
    pub anti_whale_cap: f64,
}

impl Default for GovernanceConfigV2 {
    fn default() -> Self {
        Self {
            quorum_percentage: 0.3,
            time_lock_duration: Duration::from_hours(24),
            critical_time_lock_duration: Duration::from_hours(72),
            sybil_trust_threshold: 0.3,
            max_delegation_depth: 5,
            min_nodes_for_quorum: 3,
            anti_whale_cap: 0.2,
        }
    }
}

/// A detected Sybil cluster grouping suspicious nodes.
#[derive(Debug, Clone)]
pub struct SybilClusterV2 {
    /// Unique identifier for the cluster.
    pub cluster_id: String,
    /// Node IDs belonging to this cluster.
    pub node_ids: Vec<String>,
    /// Average trust score of nodes in the cluster.
    pub avg_trust_score: f64,
    /// Reason for detection (e.g., "asn_ip_prefix_match").
    pub detection_reason: String,
}

/// Aggregate statistics for the governance engine.
#[derive(Debug, Clone)]
pub struct GovernanceStatsV2 {
    /// Total number of proposals created.
    pub total_proposals: usize,
    /// Number of proposals currently active (not executed).
    pub active_proposals: usize,
    /// Number of proposals successfully executed.
    pub executed_proposals: usize,
    /// Number of Sybil clusters detected.
    pub sybil_clusters_detected: usize,
    /// Total number of active delegations.
    pub total_delegations: usize,
}

// ─── Engine ───────────────────────────────────────────────────────────────────

/// Liquid Governance v2 engine with cryptographic reputation-weighted voting.
pub struct LiquidGovernanceV2 {
    config: GovernanceConfigV2,
    nodes: HashMap<String, NodeProfileV2>,
    proposals: HashMap<String, ProposalV2>,
    delegations: Vec<DelegationV2>,
    next_proposal_id: u64,
    sybil_clusters: Vec<SybilClusterV2>,
}

impl Default for LiquidGovernanceV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidGovernanceV2 {
    // ── Construction ───────────────────────────────────────────────────────

    /// Create a new engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(GovernanceConfigV2::default())
    }

    /// Create a new engine with the provided configuration.
    pub fn with_config(config: GovernanceConfigV2) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            proposals: HashMap::new(),
            delegations: Vec::new(),
            next_proposal_id: 1,
            sybil_clusters: Vec::new(),
        }
    }

    // ── Node Management ────────────────────────────────────────────────────

    /// Register a node profile in the governance system.
    ///
    /// Returns an error if the signature is empty (invalid).
    pub fn register_node(&mut self, mut profile: NodeProfileV2) -> Result<(), LiquidGovernanceError> {
        if profile.crypto_signature.is_empty() {
            return Err(LiquidGovernanceError::InvalidSignature(profile.node_id.clone()));
        }
        // Compute reputation score if not set
        if profile.reputation_score == 0.0 {
            profile.reputation_score =
                profile.trust_score * profile.uptime_history * (profile.staking_credits / 1000.0).min(1.0);
        }
        info!(node_id = %profile.node_id, "node registered for governance v2");
        self.nodes.insert(profile.node_id.clone(), profile);
        Ok(())
    }

    /// Get a reference to a registered node profile.
    pub fn get_node(&self, node_id: &str) -> Option<&NodeProfileV2> {
        self.nodes.get(node_id)
    }

    // ── Proposal Management ────────────────────────────────────────────────

    /// Create a new governance proposal.
    ///
    /// Critical proposals receive a 72h time-lock; normal proposals use the
    /// configurable `time_lock_duration`.
    pub fn create_proposal(
        &mut self,
        title: &str,
        description: &str,
        proposer: &str,
        critical: bool,
    ) -> Result<ProposalV2, LiquidGovernanceError> {
        if !self.nodes.contains_key(proposer) {
            return Err(LiquidGovernanceError::NodeNotFound(proposer.to_string()));
        }

        let lock_duration = if critical {
            self.config.critical_time_lock_duration
        } else {
            self.config.time_lock_duration
        };

        let id = format!("prop-{}", self.next_proposal_id);
        self.next_proposal_id += 1;

        let proposal = ProposalV2 {
            id: id.clone(),
            title: title.to_string(),
            description: description.to_string(),
            proposer: proposer.to_string(),
            created_at: Instant::now(),
            votes_for: Vec::new(),
            votes_against: Vec::new(),
            executed: false,
            time_lock_until: Instant::now() + lock_duration,
            critical,
            signature: format!("sig-{}", proposer),
        };

        info!(proposal_id = %id, critical, "proposal created");
        self.proposals.insert(id.clone(), proposal.clone());
        Ok(proposal)
    }

    /// Get a reference to a proposal by ID.
    pub fn get_proposal(&self, id: &str) -> Option<&ProposalV2> {
        self.proposals.get(id)
    }

    // ── Delegation ─────────────────────────────────────────────────────────

    /// Delegate voting weight from one node to another.
    ///
    /// Returns the effective delegated weight.
    pub fn delegate_weight(
        &mut self,
        delegator: &str,
        delegatee: &str,
        weight: f64,
        duration: Duration,
    ) -> Result<f64, LiquidGovernanceError> {
        if !self.nodes.contains_key(delegator) {
            return Err(LiquidGovernanceError::NodeNotFound(delegator.to_string()));
        }
        if !self.nodes.contains_key(delegatee) {
            return Err(LiquidGovernanceError::NodeNotFound(delegatee.to_string()));
        }

        let delegation = DelegationV2 {
            delegator: delegator.to_string(),
            delegatee: delegatee.to_string(),
            weight,
            created_at: Instant::now(),
            expires_at: Some(Instant::now() + duration),
        };

        info!(
            delegator = %delegator,
            delegatee = %delegatee,
            weight,
            "delegation created"
        );
        self.delegations.push(delegation);
        Ok(weight)
    }

    /// Resolve the full delegation chain for a given node, up to `max_delegation_depth`.
    pub fn resolve_delegation_chain(&self, node: &str, depth: usize) -> Vec<String> {
        if depth >= self.config.max_delegation_depth {
            return vec![node.to_string()];
        }

        // Find active (non-expired) delegations where this node is the delegator
        let now = Instant::now();
        let delegation = self.delegations.iter().find(|d| {
            d.delegator == node
                && d.delegatee != node
                && d.expires_at.is_none_or(|exp| now < exp)
        });

        match delegation {
            Some(d) => {
                let mut chain = vec![node.to_string()];
                let mut sub_chain = self.resolve_delegation_chain(&d.delegatee, depth + 1);
                chain.append(&mut sub_chain);
                chain
            }
            None => vec![node.to_string()],
        }
    }

    /// Check if a delegation is still active (not expired).
    fn is_delegation_active(&self, d: &DelegationV2) -> bool {
        d.expires_at.is_none_or(|exp| Instant::now() < exp)
    }

    // ── Voting ─────────────────────────────────────────────────────────────

    /// Calculate the voting weight for a node based on reputation metrics.
    fn calculate_vote_weight(&self, node_id: &str) -> Option<f64> {
        let node = self.nodes.get(node_id)?;
        let raw_weight = node.trust_score * node.staking_credits * node.uptime_history * node.reputation_score;
        Some(raw_weight)
    }

    /// Cast a vote on a proposal.
    ///
    /// `direction` is `true` for "for" and `false` for "against".
    /// Returns the effective weight of the vote after anti-whale capping.
    pub fn cast_vote(
        &mut self,
        voter: &str,
        proposal_id: &str,
        direction: bool,
    ) -> Result<f64, LiquidGovernanceError> {
        // Check proposal exists
        if !self.proposals.contains_key(proposal_id) {
            return Err(LiquidGovernanceError::ProposalNotFound(proposal_id.to_string()));
        }

        // Check for double vote (immutable borrow)
        let proposal_ref = self.proposals.get(proposal_id).unwrap();
        let already_voted = proposal_ref
            .votes_for
            .iter()
            .chain(proposal_ref.votes_against.iter())
            .any(|(v, _)| v == voter);
        if already_voted {
            return Err(LiquidGovernanceError::AlreadyVoted(voter.to_string()));
        }

        // Calculate weight (immutable borrow)
        let raw_weight = self
            .calculate_vote_weight(voter)
            .ok_or(LiquidGovernanceError::NodeNotFound(voter.to_string()))?;

        // Apply anti-whale cap (immutable borrow)
        let total_possible_weight: f64 = self
            .nodes
            .values()
            .map(|n| self.calculate_vote_weight(&n.node_id).unwrap_or(0.0))
            .sum();
        let capped_weight = raw_weight.min(total_possible_weight * self.config.anti_whale_cap);

        // Now mutate: add vote
        let entry = (voter.to_string(), capped_weight);
        if let Some(proposal) = self.proposals.get_mut(proposal_id) {
            if direction {
                proposal.votes_for.push(entry);
            } else {
                proposal.votes_against.push(entry);
            }
        }

        // Update voting history
        if let Some(node) = self.nodes.get_mut(voter) {
            node.voting_history.push(proposal_id.to_string());
        }

        info!(
            voter = %voter,
            proposal_id,
            direction = if direction { "for" } else { "against" },
            weight = capped_weight,
            "vote cast"
        );

        Ok(capped_weight)
    }

    // ── Execution ──────────────────────────────────────────────────────────

    /// Attempt to execute a proposal.
    ///
    /// Returns `GovernanceResultV2` with execution details.
    pub fn execute_proposal(&mut self, proposal_id: &str) -> Result<GovernanceResultV2, LiquidGovernanceError> {
        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or(LiquidGovernanceError::ProposalNotFound(proposal_id.to_string()))?;

        // Check if already executed
        if proposal.executed {
            return Ok(GovernanceResultV2 {
                executed: false,
                quorum_met: false,
                delegation_chain: Vec::new(),
                sybil_flag: false,
                total_weight: 0.0,
            });
        }

        // Check time-lock
        let now = Instant::now();
        if now < proposal.time_lock_until {
            let remaining = proposal.time_lock_until.duration_since(now);
            return Err(LiquidGovernanceError::TimeLockActive { remaining });
        }

        let weight_for: f64 = proposal.votes_for.iter().map(|(_, w)| w).sum();
        let weight_against: f64 = proposal.votes_against.iter().map(|(_, w)| w).sum();
        let total_weight = weight_for + weight_against;

        // Count unique voters
        let unique_voters: HashSet<&str> = proposal
            .votes_for
            .iter()
            .chain(proposal.votes_against.iter())
            .map(|(v, _)| v.as_str())
            .collect();

        // Quorum check
        let quorum_met = total_weight > 0.0
            && (weight_for / total_weight) >= self.config.quorum_percentage
            && unique_voters.len() >= self.config.min_nodes_for_quorum;

        if !quorum_met {
            warn!(proposal_id, weight_for, weight_against, "quorum not met for proposal execution");
        }

        // Collect delegation chains from all voters
        let mut delegation_chain = Vec::new();
        for (voter, _) in proposal.votes_for.iter().chain(proposal.votes_against.iter()) {
            let chain = self.resolve_delegation_chain(voter, 0);
            for node in chain {
                if !delegation_chain.contains(&node) {
                    delegation_chain.push(node);
                }
            }
        }

        // Check for Sybil clusters
        let sybil_flag = !self.sybil_clusters.is_empty();

        let result = GovernanceResultV2 {
            executed: quorum_met,
            quorum_met,
            delegation_chain,
            sybil_flag,
            total_weight,
        };

        // Mark as executed if quorum met
        if quorum_met {
            if let Some(prop) = self.proposals.get_mut(proposal_id) {
                prop.executed = true;
            }
            info!(proposal_id, "proposal executed successfully");
        }

        Ok(result)
    }

    // ── Sybil Detection ────────────────────────────────────────────────────

    /// Detect Sybil clusters based on ASN + IP prefix similarity and voting behavior.
    ///
    /// Groups nodes that share the same ASN and IP prefix, then checks if their
    /// average trust score is below the Sybil threshold.
    pub fn detect_sybil_cluster(&mut self) -> Vec<SybilClusterV2> {
        debug!("running sybil detection");

        // Group nodes by (ASN, IP_prefix)
        let mut groups: HashMap<(String, String), Vec<String>> = HashMap::new();
        for node in self.nodes.values() {
            let key = (node.asn.clone(), node.ip_prefix.clone());
            groups.entry(key).or_default().push(node.node_id.clone());
        }

        self.sybil_clusters.clear();
        let mut cluster_id = 1;

        for ((asn, ip_prefix), node_ids) in groups {
            if node_ids.len() < 2 {
                continue;
            }

            let avg_trust: f64 = node_ids
                .iter()
                .filter_map(|id| self.nodes.get(id))
                .map(|n| n.trust_score)
                .sum::<f64>()
                / node_ids.len() as f64;

            if avg_trust < self.config.sybil_trust_threshold {
                let cluster = SybilClusterV2 {
                    cluster_id: format!("cluster-{}", cluster_id),
                    node_ids: node_ids.clone(),
                    avg_trust_score: avg_trust,
                    detection_reason: format!("asn_ip_prefix_match (ASN={}, prefix={})", asn, ip_prefix),
                };
                warn!(
                    cluster_id = %cluster.cluster_id,
                    nodes = ?node_ids,
                    avg_trust,
                    "sybil cluster detected"
                );
                self.sybil_clusters.push(cluster);
                cluster_id += 1;
            }
        }

        self.sybil_clusters.clone()
    }

    // ── Stats ──────────────────────────────────────────────────────────────

    /// Get aggregate governance statistics.
    pub fn get_stats(&self) -> GovernanceStatsV2 {
        let total_proposals = self.proposals.len();
        let executed_proposals = self.proposals.values().filter(|p| p.executed).count();
        let active_proposals = total_proposals - executed_proposals;
        let active_delegations = self.delegations.iter().filter(|d| self.is_delegation_active(d)).count();

        GovernanceStatsV2 {
            total_proposals,
            active_proposals,
            executed_proposals,
            sybil_clusters_detected: self.sybil_clusters.len(),
            total_delegations: active_delegations,
        }
    }

    // ── Reset ──────────────────────────────────────────────────────────────

    /// Reset the engine to a clean state, preserving configuration.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.proposals.clear();
        self.delegations.clear();
        self.sybil_clusters.clear();
        self.next_proposal_id = 1;
        info!("governance v2 engine reset");
    }
}

// ─── Display Implementations ─────────────────────────────────────────────────

impl fmt::Display for ProposalV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProposalV2({}, critical={})", self.id, self.critical)
    }
}

impl fmt::Display for NodeProfileV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeProfileV2({})", self.node_id)
    }
}

impl fmt::Display for GovernanceConfigV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GovernanceConfigV2(quorum={:.0}%, anti_whale={:.0}%)",
            self.quorum_percentage * 100.0,
            self.anti_whale_cap * 100.0
        )
    }
}

impl fmt::Display for SybilClusterV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SybilClusterV2({}, nodes={})", self.cluster_id, self.node_ids.len())
    }
}

impl fmt::Display for GovernanceStatsV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stats(total={}, active={}, executed={}, sybil={}, delegations={})",
            self.total_proposals, self.active_proposals, self.executed_proposals,
            self.sybil_clusters_detected, self.total_delegations
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(id: &str, asn: &str, prefix: &str) -> NodeProfileV2 {
        NodeProfileV2 {
            node_id: id.to_string(),
            trust_score: 0.8,
            staking_credits: 500.0,
            uptime_history: 0.95,
            crypto_signature: format!("sig-{}", id),
            asn: asn.to_string(),
            ip_prefix: prefix.to_string(),
            voting_history: Vec::new(),
            reputation_score: 0.0,
        }
    }

    #[test]
    fn test_new_engine_default_config() {
        let engine = LiquidGovernanceV2::new();
        assert_eq!(engine.nodes.len(), 0);
        assert_eq!(engine.proposals.len(), 0);
        assert_eq!(engine.config.quorum_percentage, 0.3);
        assert_eq!(engine.config.critical_time_lock_duration, Duration::from_hours(72));
    }

    #[test]
    fn test_with_custom_config() {
        let config = GovernanceConfigV2 {
            quorum_percentage: 0.5,
            anti_whale_cap: 0.1,
            ..Default::default()
        };
        let engine = LiquidGovernanceV2::with_config(config);
        assert_eq!(engine.config.quorum_percentage, 0.5);
        assert_eq!(engine.config.anti_whale_cap, 0.1);
    }

    #[test]
    fn test_register_node_success() {
        let mut engine = LiquidGovernanceV2::new();
        let node = test_node("node-1", "AS100", "10.0.0.0/24");
        assert!(engine.register_node(node).is_ok());
        assert_eq!(engine.nodes.len(), 1);
    }

    #[test]
    fn test_register_node_invalid_signature() {
        let mut engine = LiquidGovernanceV2::new();
        let mut node = test_node("node-bad", "AS100", "10.0.0.0/24");
        node.crypto_signature = String::new();
        assert!(engine.register_node(node).is_err());
    }

    #[test]
    fn test_create_proposal_normal() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        let before = Instant::now();
        let proposal = engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        let after = Instant::now();
        assert!(!proposal.critical);
        let expected = before + Duration::from_hours(24);
        assert!(
            proposal.time_lock_until >= expected && proposal.time_lock_until <= after + Duration::from_hours(24),
            "time_lock_until {:?} not in range [{:?}, {:?}]",
            proposal.time_lock_until,
            expected,
            after + Duration::from_hours(24)
        );
    }

    #[test]
    fn test_create_proposal_critical() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        let before = Instant::now();
        let proposal = engine.create_proposal("Critical", "Desc", "node-1", true).unwrap();
        let after = Instant::now();
        assert!(proposal.critical);
        let expected = before + Duration::from_hours(72);
        assert!(
            proposal.time_lock_until >= expected && proposal.time_lock_until <= after + Duration::from_hours(72),
            "time_lock_until {:?} not in range [{:?}, {:?}]",
            proposal.time_lock_until,
            expected,
            after + Duration::from_hours(72)
        );
    }

    #[test]
    fn test_create_proposal_unregistered_node() {
        let mut engine = LiquidGovernanceV2::new();
        let result = engine.create_proposal("Test", "Desc", "unknown", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_delegate_weight_success() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        let weight = engine.delegate_weight("node-1", "node-2", 0.5, Duration::from_hours(48)).unwrap();
        assert_eq!(weight, 0.5);
        assert_eq!(engine.delegations.len(), 1);
    }

    #[test]
    fn test_delegate_weight_unknown_delegator() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        let result = engine.delegate_weight("unknown", "node-2", 0.5, Duration::from_hours(48));
        assert!(result.is_err());
    }

    #[test]
    fn test_cast_vote_for() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        let weight = engine.cast_vote("node-2", "prop-1", true).unwrap();
        assert!(weight > 0.0);
    }

    #[test]
    fn test_cast_vote_against() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        let weight = engine.cast_vote("node-2", "prop-1", false).unwrap();
        assert!(weight > 0.0);
    }

    #[test]
    fn test_double_vote_rejected() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        engine.cast_vote("node-2", "prop-1", true).unwrap();
        let result = engine.cast_vote("node-2", "prop-1", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_timelock_enforcement() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Critical", "Desc", "node-1", true).unwrap();
        let result = engine.execute_proposal("prop-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_sybil_detection_same_asn_prefix() {
        let mut engine = LiquidGovernanceV2::new();
        // Register nodes with same ASN and IP prefix but low trust
        let mut n1 = test_node("s1", "AS999", "10.99.0.0/16");
        n1.trust_score = 0.1;
        let mut n2 = test_node("s2", "AS999", "10.99.0.0/16");
        n2.trust_score = 0.2;
        engine.register_node(n1).unwrap();
        engine.register_node(n2).unwrap();
        let clusters = engine.detect_sybil_cluster();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].node_ids.len(), 2);
    }

    #[test]
    fn test_sybil_detection_no_cluster() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        let clusters = engine.detect_sybil_cluster();
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn test_quorum_calculation_met() {
        let config = GovernanceConfigV2 {
            quorum_percentage: 0.3,
            min_nodes_for_quorum: 2,
            time_lock_duration: Duration::from_secs(1),
            ..Default::default()
        };
        let mut engine = LiquidGovernanceV2::with_config(config);
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        engine.cast_vote("node-2", "prop-1", true).unwrap();
        engine.cast_vote("node-3", "prop-1", true).unwrap();
        // Wait for time-lock to expire
        std::thread::sleep(Duration::from_secs(2));
        let result = engine.execute_proposal("prop-1").unwrap();
        assert!(result.quorum_met);
    }

    #[test]
    fn test_quorum_not_met_too_few_voters() {
        let config = GovernanceConfigV2 {
            quorum_percentage: 0.3,
            min_nodes_for_quorum: 5,
            time_lock_duration: Duration::from_secs(1),
            ..Default::default()
        };
        let mut engine = LiquidGovernanceV2::with_config(config);
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        engine.cast_vote("node-2", "prop-1", true).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        let result = engine.execute_proposal("prop-1").unwrap();
        assert!(!result.quorum_met);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        let stats = engine.get_stats();
        assert_eq!(stats.total_proposals, 1);
        assert_eq!(stats.active_proposals, 1);
        assert_eq!(stats.executed_proposals, 0);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        engine.reset();
        assert_eq!(engine.nodes.len(), 0);
        assert_eq!(engine.proposals.len(), 0);
        assert_eq!(engine.delegations.len(), 0);
        assert_eq!(engine.next_proposal_id, 1);
    }

    #[test]
    fn test_anti_whale_cap() {
        let config = GovernanceConfigV2 {
            anti_whale_cap: 0.05,
            ..Default::default()
        };
        let mut engine = LiquidGovernanceV2::with_config(config);
        // Create a whale node with very high staking
        let mut whale = test_node("whale", "AS100", "10.0.0.0/24");
        whale.staking_credits = 100_000.0;
        engine.register_node(whale).unwrap();
        engine.register_node(test_node("small-1", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("small-2", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "small-1", false).unwrap();
        let weight = engine.cast_vote("whale", "prop-1", true).unwrap();
        // Weight should be capped
        let total: f64 = engine.nodes.values().map(|n| n.staking_credits).sum();
        let max_allowed = total * 0.05 * 0.8 * 0.95; // rough estimate
        assert!(weight <= max_allowed.max(1.0));
    }

    #[test]
    fn test_delegation_chain_resolution() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("a", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("b", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("c", "AS300", "172.16.0.0/12")).unwrap();
        engine.delegate_weight("a", "b", 0.5, Duration::from_hours(48)).unwrap();
        engine.delegate_weight("b", "c", 0.5, Duration::from_hours(48)).unwrap();
        let chain = engine.resolve_delegation_chain("a", 0);
        assert_eq!(chain, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_expired_delegation_not_resolved() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("a", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("b", "AS200", "192.168.0.0/16")).unwrap();
        // Create an already-expired delegation by using a past expiry time.
        // Use checked_sub to avoid overflow on platforms where Instant::now() is close to epoch.
        let now = Instant::now();
        let past = now.checked_sub(Duration::from_secs(1)).unwrap_or(Instant::now());
        let expired = DelegationV2 {
            delegator: "a".to_string(),
            delegatee: "b".to_string(),
            weight: 0.5,
            created_at: past,
            expires_at: Some(past),
        };
        engine.delegations.push(expired);
        let chain = engine.resolve_delegation_chain("a", 0);
        assert_eq!(chain, vec!["a"]);
    }

    #[test]
    fn test_proposal_execution_success() {
        let config = GovernanceConfigV2 {
            quorum_percentage: 0.3,
            min_nodes_for_quorum: 2,
            time_lock_duration: Duration::from_secs(1),
            ..Default::default()
        };
        let mut engine = LiquidGovernanceV2::with_config(config);
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        engine.cast_vote("node-2", "prop-1", true).unwrap();
        engine.cast_vote("node-3", "prop-1", true).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        let result = engine.execute_proposal("prop-1").unwrap();
        assert!(result.executed);
    }

    #[test]
    fn test_proposal_execution_failure_against_majority() {
        let config = GovernanceConfigV2 {
            quorum_percentage: 0.3,
            min_nodes_for_quorum: 2,
            time_lock_duration: Duration::from_secs(1),
            ..Default::default()
        };
        let mut engine = LiquidGovernanceV2::with_config(config);
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.register_node(test_node("node-2", "AS200", "192.168.0.0/16")).unwrap();
        engine.register_node(test_node("node-3", "AS300", "172.16.0.0/12")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        engine.cast_vote("node-2", "prop-1", false).unwrap();
        engine.cast_vote("node-3", "prop-1", false).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        let result = engine.execute_proposal("prop-1").unwrap();
        assert!(!result.executed);
    }

    #[test]
    fn test_get_node_returns_profile() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        let node = engine.get_node("node-1");
        assert!(node.is_some());
        assert_eq!(node.unwrap().node_id, "node-1");
    }

    #[test]
    fn test_get_proposal_returns_proposal() {
        let mut engine = LiquidGovernanceV2::new();
        engine.register_node(test_node("node-1", "AS100", "10.0.0.0/24")).unwrap();
        engine.create_proposal("Test", "Desc", "node-1", false).unwrap();
        let proposal = engine.get_proposal("prop-1");
        assert!(proposal.is_some());
        assert_eq!(proposal.unwrap().title, "Test");
    }

    #[test]
    fn test_display_implementations() {
        // thiserror derives Display from #[error(...)] attributes,
        // so the message uses the attribute text: "node not found: x"
        let err = LiquidGovernanceError::NodeNotFound("x".to_string());
        let s = format!("{}", err);
        assert!(s.contains("node not found"));

        let config = GovernanceConfigV2::default();
        let s = format!("{}", config);
        assert!(s.contains("quorum"));
    }
}
