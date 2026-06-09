//! Symbiotic Governance & Global Bootstrap — Final coordination layer for Planetary Mesh.
//!
//! Provides reputation-weighted voting, proposal lifecycle management,
//! symbiotic value distribution, and global bootstrap discovery protocol
//! for new participants joining the planetary mesh.

use sha2::{Digest, Sha256};

type Hash = [u8; 32];

// ---------------------------------------------------------------------------
// Governance Configuration
// ---------------------------------------------------------------------------

/// Configuration for the symbiotic governance system.
#[derive(Debug, Clone)]
pub struct GovernanceConfig {
    /// Minimum trust score to participate in voting
    pub min_trust_to_vote: f64,
    /// Quorum fraction required for proposal passage (0.0–1.0)
    pub quorum_fraction: f64,
    /// Approval threshold fraction (0.0–1.0)
    pub approval_threshold: f64,
    /// Voting window duration in seconds
    pub voting_window_seconds: u64,
    /// Maximum active proposals
    pub max_active_proposals: usize,
    /// Bootstrap peer discovery timeout in milliseconds
    pub bootstrap_timeout_ms: u64,
    /// Maximum bootstrap peers to discover
    pub max_bootstrap_peers: usize,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            min_trust_to_vote: 0.3,
            quorum_fraction: 0.5,
            approval_threshold: 0.6,
            voting_window_seconds: 86400, // 24 hours
            max_active_proposals: 16,
            bootstrap_timeout_ms: 5000,
            max_bootstrap_peers: 32,
        }
    }
}

impl GovernanceConfig {
    /// Create config with custom quorum.
    pub fn with_quorum(mut self, fraction: f64) -> Self {
        self.quorum_fraction = fraction.clamp(0.0, 1.0);
        self
    }

    /// Create config with custom approval threshold.
    pub fn with_approval_threshold(mut self, threshold: f64) -> Self {
        self.approval_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Create config with custom voting window.
    pub fn with_voting_window(mut self, seconds: u64) -> Self {
        self.voting_window_seconds = seconds;
        self
    }

    /// Create config for fast testing (short windows, low thresholds).
    pub fn fast_test() -> Self {
        Self {
            min_trust_to_vote: 0.1,
            quorum_fraction: 0.3,
            approval_threshold: 0.4,
            voting_window_seconds: 60,
            max_active_proposals: 8,
            bootstrap_timeout_ms: 1000,
            max_bootstrap_peers: 8,
        }
    }
}

// ---------------------------------------------------------------------------
// Proposal
// ---------------------------------------------------------------------------

/// Vote option for a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

/// A governance proposal submitted to the symbiotic council.
#[derive(Debug, Clone)]
pub struct Proposal {
    /// Unique proposal identifier (hash)
    pub id: Hash,
    /// Title of the proposal
    pub title: String,
    /// Description / body
    pub description: String,
    /// Proposer node ID
    pub proposer_id: String,
    /// Creation timestamp (Unix epoch seconds)
    pub created_at: u64,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Current status
    pub status: ProposalStatus,
    /// Votes cast (node_id, trust_weight, vote)
    pub votes: Vec<(String, f64, Vote)>,
}

/// Proposal lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Expired,
}

impl Proposal {
    /// Create a new proposal.
    pub fn new(
        title: String,
        description: String,
        proposer_id: String,
        created_at: u64,
        voting_window: u64,
    ) -> Self {
        let id = Self::compute_id(&title, &proposer_id, created_at);
        let expires_at = created_at + voting_window;
        Self {
            id,
            title,
            description,
            proposer_id,
            created_at,
            expires_at,
            status: ProposalStatus::Active,
            votes: Vec::new(),
        }
    }

    /// Compute deterministic proposal ID.
    fn compute_id(title: &str, proposer_id: &str, timestamp: u64) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(proposer_id.as_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.finalize().into()
    }

    /// Cast a vote on this proposal.
    pub fn cast_vote(&mut self, voter_id: String, trust_weight: f64, vote: Vote) {
        if self.status != ProposalStatus::Active {
            return;
        }
        // Replace existing vote from same voter
        if let Some(existing) = self.votes.iter_mut().find(|(id, _, _)| id == &voter_id) {
            existing.1 = trust_weight;
            existing.2 = vote;
        } else {
            self.votes.push((voter_id, trust_weight, vote));
        }
    }

    /// Finalize the proposal based on current votes and config.
    pub fn finalize(&mut self, config: &GovernanceConfig) {
        if self.status != ProposalStatus::Active {
            return;
        }

        let total_weight: f64 = self.votes.iter().map(|(_, w, _)| w).sum();
        let participating: f64 = self
            .votes
            .iter()
            .filter(|(_, _, v)| *v != Vote::Abstain)
            .map(|(_, w, _)| w)
            .sum();

        // Check quorum
        let quorum_met = participating >= total_weight * config.quorum_fraction;

        if !quorum_met {
            self.status = ProposalStatus::Rejected;
            return;
        }

        // Count approvals
        let approve_weight: f64 = self
            .votes
            .iter()
            .filter(|(_, _, v)| *v == Vote::Approve)
            .map(|(_, w, _)| w)
            .sum();

        let approval_ratio = if participating > 0.0 {
            approve_weight / participating
        } else {
            0.0
        };

        if approval_ratio >= config.approval_threshold {
            self.status = ProposalStatus::Passed;
        } else {
            self.status = ProposalStatus::Rejected;
        }
    }

    /// Check if proposal has expired.
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expires_at
    }

    /// Get approval ratio (0.0–1.0).
    pub fn approval_ratio(&self) -> f64 {
        let total_weight: f64 = self
            .votes
            .iter()
            .filter(|(_, _, v)| *v != Vote::Abstain)
            .map(|(_, w, _)| w)
            .sum();
        if total_weight == 0.0 {
            return 0.0;
        }
        let approve_weight: f64 = self
            .votes
            .iter()
            .filter(|(_, _, v)| *v == Vote::Approve)
            .map(|(_, w, _)| w)
            .sum();
        approve_weight / total_weight
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Council
// ---------------------------------------------------------------------------

/// The Symbiotic Council manages proposals and voting.
#[derive(Debug)]
pub struct SymbioticCouncil {
    config: GovernanceConfig,
    proposals: Vec<Proposal>,
    /// Total voting power in the council (sum of all member trust scores)
    total_voting_power: f64,
}

impl SymbioticCouncil {
    /// Create a new council with the given config.
    pub fn new(config: GovernanceConfig) -> Self {
        Self {
            config,
            proposals: Vec::new(),
            total_voting_power: 0.0,
        }
    }

    /// Submit a new proposal.
    pub fn submit_proposal(
        &mut self,
        title: String,
        description: String,
        proposer_id: String,
        current_time: u64,
    ) -> Option<Hash> {
        if self.proposals.len() >= self.config.max_active_proposals {
            return None;
        }

        let proposal = Proposal::new(
            title,
            description,
            proposer_id,
            current_time,
            self.config.voting_window_seconds,
        );
        let id = proposal.id;
        self.proposals.push(proposal);
        Some(id)
    }

    /// Cast a vote on a proposal.
    pub fn cast_vote(
        &mut self,
        proposal_id: &Hash,
        voter_id: String,
        trust_weight: f64,
        vote: Vote,
    ) -> bool {
        if trust_weight < self.config.min_trust_to_vote {
            return false;
        }
        if let Some(proposal) = self.proposals.iter_mut().find(|p| p.id == *proposal_id) {
            proposal.cast_vote(voter_id, trust_weight, vote);
            true
        } else {
            false
        }
    }

    /// Finalize all active proposals.
    pub fn finalize_proposals(&mut self) -> usize {
        let mut finalized = 0;
        for proposal in &mut self.proposals {
            if proposal.status == ProposalStatus::Active {
                proposal.finalize(&self.config);
                finalized += 1;
            }
        }
        finalized
    }

    /// Expire proposals past their deadline.
    pub fn expire_proposals(&mut self, current_time: u64) -> usize {
        let mut expired = 0;
        for proposal in &mut self.proposals {
            if proposal.status == ProposalStatus::Active && proposal.is_expired(current_time) {
                proposal.status = ProposalStatus::Expired;
                expired += 1;
            }
        }
        expired
    }

    /// Get a proposal by ID.
    pub fn get_proposal(&self, id: &Hash) -> Option<&Proposal> {
        self.proposals.iter().find(|p| p.id == *id)
    }

    /// Get all active proposals.
    pub fn active_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Active)
            .collect()
    }

    /// Get count of proposals by status.
    pub fn proposal_counts(&self) -> (usize, usize, usize, usize) {
        let mut active = 0;
        let mut passed = 0;
        let mut rejected = 0;
        let mut expired = 0;
        for p in &self.proposals {
            match p.status {
                ProposalStatus::Active => active += 1,
                ProposalStatus::Passed => passed += 1,
                ProposalStatus::Rejected => rejected += 1,
                ProposalStatus::Expired => expired += 1,
            }
        }
        (active, passed, rejected, expired)
    }

    /// Update total voting power.
    pub fn set_total_voting_power(&mut self, power: f64) {
        self.total_voting_power = power;
    }

    /// Get total voting power.
    pub fn total_voting_power(&self) -> f64 {
        self.total_voting_power
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Value Distribution
// ---------------------------------------------------------------------------

/// Result of value distribution computation.
#[derive(Debug, Clone)]
pub struct ValueDistribution {
    /// Node ID
    pub node_id: String,
    /// Trust-weighted share of value
    pub share: f64,
    /// Contribution score used for distribution
    pub contribution_score: f64,
    /// Raw value allocated
    pub value_allocated: f64,
}

impl ValueDistribution {
    /// Compute share as fraction of total.
    pub fn share(&self, total_contribution: f64) -> f64 {
        if total_contribution == 0.0 {
            return 0.0;
        }
        self.contribution_score / total_contribution
    }
}

/// Compute symbiotic value distribution across nodes.
///
/// Uses trust-weighted contribution scores for fair distribution.
///
/// # Arguments
/// * `nodes` — Vector of (node_id, trust_score, contribution_score)
/// * `total_value` — Total value pool to distribute
///
/// # Returns
/// Vector of `ValueDistribution` entries, one per node
pub fn compute_value_distribution(
    nodes: &[(String, f64, f64)],
    total_value: f64,
) -> Vec<ValueDistribution> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let total_weighted: f64 = nodes.iter().map(|(_, t, c)| c * t).sum();
    if total_weighted == 0.0 {
        // Equal distribution if no weighted contributions
        let equal_share = total_value / nodes.len() as f64;
        return nodes
            .iter()
            .map(|(id, trust, _)| ValueDistribution {
                node_id: id.clone(),
                share: 1.0 / nodes.len() as f64,
                contribution_score: 0.0,
                value_allocated: equal_share * trust,
            })
            .collect();
    }

    nodes
        .iter()
        .map(|(id, trust, contribution)| {
            let weighted_score = contribution * trust;
            let share = weighted_score / total_weighted;
            ValueDistribution {
                node_id: id.clone(),
                share,
                contribution_score: *contribution,
                value_allocated: share * total_value,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Global Bootstrap Protocol
// ---------------------------------------------------------------------------

/// A bootstrap peer entry.
#[derive(Debug, Clone)]
pub struct BootstrapPeer {
    /// Peer node ID
    pub node_id: String,
    /// Network address
    pub address: String,
    /// Trust score
    pub trust_score: f64,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Number of successful connections
    pub connection_count: u64,
}

impl BootstrapPeer {
    /// Create a new bootstrap peer.
    pub fn new(node_id: String, address: String, trust_score: f64) -> Self {
        Self {
            node_id,
            address,
            trust_score,
            last_seen: 0,
            connection_count: 0,
        }
    }

    /// Check if peer is active (trust above threshold).
    pub fn is_active(&self, min_trust: f64) -> bool {
        self.trust_score >= min_trust
    }
}

/// Configuration for global bootstrap discovery.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Initial bootstrap peers to contact
    pub initial_peers: Vec<BootstrapPeer>,
    /// Discovery timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum peers to discover
    pub max_peers: usize,
    /// Minimum trust score for bootstrap peers
    pub min_trust: f64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            initial_peers: Vec::new(),
            timeout_ms: 5000,
            max_peers: 32,
            min_trust: 0.5,
        }
    }
}

impl BootstrapConfig {
    /// Create config with initial peers.
    pub fn with_peers(peers: Vec<BootstrapPeer>) -> Self {
        Self {
            initial_peers: peers,
            ..Self::default()
        }
    }

    /// Create config with custom timeout.
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Create config with custom max peers.
    pub fn with_max_peers(mut self, max: usize) -> Self {
        self.max_peers = max;
        self
    }
}

/// Result of bootstrap discovery.
#[derive(Debug, Clone)]
pub struct BootstrapResult {
    /// Discovered peers
    pub discovered_peers: Vec<BootstrapPeer>,
    /// Number of peers discovered
    pub peers_found: usize,
    /// Discovery time in milliseconds
    pub discovery_time_ms: u64,
    /// Success flag
    pub success: bool,
}

impl BootstrapResult {
    /// Create a successful result.
    pub fn success(peers: Vec<BootstrapPeer>, time_ms: u64) -> Self {
        Self {
            peers_found: peers.len(),
            discovered_peers: peers,
            discovery_time_ms: time_ms,
            success: true,
        }
    }

    /// Create a failed result.
    pub fn failure(time_ms: u64) -> Self {
        Self {
            discovered_peers: Vec::new(),
            peers_found: 0,
            discovery_time_ms: time_ms,
            success: false,
        }
    }

    /// Get active peers (above trust threshold).
    pub fn active_peers(&self, min_trust: f64) -> Vec<&BootstrapPeer> {
        self.discovered_peers
            .iter()
            .filter(|p| p.is_active(min_trust))
            .collect()
    }

    /// Compute average trust of discovered peers.
    pub fn avg_trust(&self) -> f64 {
        if self.discovered_peers.is_empty() {
            return 0.0;
        }
        let total: f64 = self.discovered_peers.iter().map(|p| p.trust_score).sum();
        total / self.discovered_peers.len() as f64
    }
}

/// Execute global bootstrap discovery.
///
/// Discovers active bootstrap peers from the initial seed list,
/// filtering by trust score and connection history.
///
/// # Arguments
/// * `config` — Bootstrap configuration
/// * `current_time` — Current Unix timestamp in seconds
///
/// # Returns
/// `BootstrapResult` with discovered peers
pub fn execute_bootstrap_discovery(
    config: &BootstrapConfig,
    _current_time: u64,
) -> BootstrapResult {
    let start = std::time::Instant::now();

    let active_peers: Vec<BootstrapPeer> = config
        .initial_peers
        .iter()
        .filter(|p| p.is_active(config.min_trust))
        .cloned()
        .take(config.max_peers)
        .collect();

    let elapsed = start.elapsed();
    let time_ms = elapsed.as_millis() as u64;

    if active_peers.is_empty() {
        return BootstrapResult::failure(time_ms);
    }

    BootstrapResult::success(active_peers, time_ms)
}

/// Select optimal bootstrap peers using trust-weighted ranking.
///
/// # Arguments
/// * `peers` — Available bootstrap peers
/// * `count` — Number of peers to select
///
/// # Returns
/// Top `count` peers sorted by trust score (descending)
pub fn select_optimal_bootstrap_peers(peers: &[BootstrapPeer], count: usize) -> Vec<BootstrapPeer> {
    let mut sorted = peers.to_vec();
    sorted.sort_by(|a, b| {
        b.trust_score
            .partial_cmp(&a.trust_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.into_iter().take(count).collect()
}

// ---------------------------------------------------------------------------
// Community Bootstrap + No-Econ Incentives (PASO C — Sprint 125)
// ---------------------------------------------------------------------------

/// Non-economic reputation badge types for community recognition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunityBadge {
    /// First node to join a new mesh region
    SeedGuardian,
    /// Node that performed self-healing rebalancing
    MeshHealer,
    /// Node that shared verified knowledge (model weights, proofs)
    KnowledgeSharer,
    /// Node that maintained uptime > 99% for 30 days
    IronUptime,
    /// Node that onboarded 5+ new peers
    CommunityBuilder,
    /// Node that contributed formal verification proofs
    ProofForge,
    /// Node that achieved energy efficiency ratio > 0.95
    GreenSteward,
    /// Node that participated in 10+ governance votes
    CivicVoice,
}

impl std::fmt::Display for CommunityBadge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunityBadge::SeedGuardian => write!(f, "Seed Guardian"),
            CommunityBadge::MeshHealer => write!(f, "Mesh Healer"),
            CommunityBadge::KnowledgeSharer => write!(f, "Knowledge Sharer"),
            CommunityBadge::IronUptime => write!(f, "Iron Uptime"),
            CommunityBadge::CommunityBuilder => write!(f, "Community Builder"),
            CommunityBadge::ProofForge => write!(f, "Proof Forge"),
            CommunityBadge::GreenSteward => write!(f, "Green Steward"),
            CommunityBadge::CivicVoice => write!(f, "Civic Voice"),
        }
    }
}

/// Non-economic incentive record.
#[derive(Debug, Clone)]
pub struct NoEconIncentive {
    /// Node ID receiving the incentive
    pub node_id: String,
    /// Badge awarded
    pub badge: CommunityBadge,
    /// Timestamp of award (Unix seconds)
    pub awarded_at: u64,
    /// Reason / description
    pub reason: String,
    /// Social capital points added
    pub social_capital_points: f64,
}

impl NoEconIncentive {
    /// Create a new non-economic incentive record.
    pub fn new(node_id: String, badge: CommunityBadge, awarded_at: u64, reason: String) -> Self {
        let points = Self::badge_points(badge);
        Self {
            node_id,
            badge,
            awarded_at,
            reason,
            social_capital_points: points,
        }
    }

    /// Compute social capital points for a badge type.
    fn badge_points(badge: CommunityBadge) -> f64 {
        match badge {
            CommunityBadge::SeedGuardian => 10.0,
            CommunityBadge::MeshHealer => 15.0,
            CommunityBadge::KnowledgeSharer => 12.0,
            CommunityBadge::IronUptime => 20.0,
            CommunityBadge::CommunityBuilder => 18.0,
            CommunityBadge::ProofForge => 25.0,
            CommunityBadge::GreenSteward => 22.0,
            CommunityBadge::CivicVoice => 8.0,
        }
    }
}

/// Community bootstrap state for a node or region.
#[derive(Debug, Clone)]
pub struct CommunityBootstrap {
    /// Node ID of the bootstrapping entity
    pub node_id: String,
    /// Discovered bootstrap peers
    pub bootstrap_peers: Vec<BootstrapPeer>,
    /// Awarded incentives (badges)
    pub incentives: Vec<NoEconIncentive>,
    /// Total social capital accumulated
    pub total_social_capital: f64,
    /// Number of peers onboarded by this node
    pub peers_onboarded: usize,
    /// Governance participation count
    pub governance_votes: usize,
    /// Knowledge contributions count
    pub knowledge_contributions: usize,
    /// Self-healing actions count
    pub healing_actions: usize,
}

impl CommunityBootstrap {
    /// Create a new community bootstrap state.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            bootstrap_peers: Vec::new(),
            incentives: Vec::new(),
            total_social_capital: 0.0,
            peers_onboarded: 0,
            governance_votes: 0,
            knowledge_contributions: 0,
            healing_actions: 0,
        }
    }

    /// Add a discovered bootstrap peer.
    pub fn add_peer(&mut self, peer: BootstrapPeer) {
        self.bootstrap_peers.push(peer);
    }

    /// Award a badge (non-economic incentive) to this node.
    pub fn award_badge(&mut self, badge: CommunityBadge, awarded_at: u64, reason: String) {
        let incentive = NoEconIncentive::new(self.node_id.clone(), badge, awarded_at, reason);
        self.total_social_capital += incentive.social_capital_points;
        self.incentives.push(incentive);
    }

    /// Record a peer onboarding event.
    pub fn record_onboarding(&mut self) {
        self.peers_onboarded += 1;
        if self.peers_onboarded >= 5 {
            self.award_badge(
                CommunityBadge::CommunityBuilder,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                format!("Onboarded {} peers", self.peers_onboarded),
            );
        }
    }

    /// Record a governance vote participation.
    pub fn record_governance_vote(&mut self) {
        self.governance_votes += 1;
        if self.governance_votes >= 10 {
            self.award_badge(
                CommunityBadge::CivicVoice,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                format!("Participated in {} governance votes", self.governance_votes),
            );
        }
    }

    /// Record a knowledge contribution.
    pub fn record_knowledge_contribution(&mut self) {
        self.knowledge_contributions += 1;
        if self.knowledge_contributions >= 1 {
            self.award_badge(
                CommunityBadge::KnowledgeSharer,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                format!(
                    "Shared {} knowledge contributions",
                    self.knowledge_contributions
                ),
            );
        }
    }

    /// Record a self-healing action.
    pub fn record_healing_action(&mut self) {
        self.healing_actions += 1;
        if self.healing_actions >= 1 {
            self.award_badge(
                CommunityBadge::MeshHealer,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                format!("Performed {} self-healing actions", self.healing_actions),
            );
        }
    }

    /// Compute the incentive tier based on social capital.
    ///
    /// # Returns
    /// Tier level: 0 (Newcomer), 1 (Contributor), 2 (Steward), 3 (Guardian), 4 (Legend)
    pub fn incentive_tier(&self) -> u32 {
        let capital = self.total_social_capital;
        if capital >= 100.0 {
            4
        } else if capital >= 50.0 {
            3
        } else if capital >= 20.0 {
            2
        } else if capital >= 5.0 {
            1
        } else {
            0
        }
    }

    /// Get the tier name as a string.
    pub fn tier_name(&self) -> &str {
        match self.incentive_tier() {
            0 => "Newcomer",
            1 => "Contributor",
            2 => "Steward",
            3 => "Guardian",
            4 => "Legend",
            _ => unreachable!(),
        }
    }

    /// Check if this node qualifies as a seed guardian (first in region).
    pub fn is_seed_guardian(&self) -> bool {
        self.bootstrap_peers.is_empty() && self.peers_onboarded > 0
    }

    /// Compute a reputation score combining multiple factors.
    ///
    /// Formula: `social_capital * (1.0 + trust_bonus + activity_bonus)`
    /// Where trust_bonus = avg_trust * 0.5, activity_bonus = min(total_actions / 100.0, 0.5)
    pub fn reputation_score(&self, avg_trust: f64) -> f64 {
        let total_actions = self.peers_onboarded
            + self.governance_votes
            + self.knowledge_contributions
            + self.healing_actions;
        let trust_bonus = avg_trust.clamp(0.0, 1.0) * 0.5;
        let activity_bonus = (total_actions as f64 / 100.0).clamp(0.0, 0.5);
        self.total_social_capital * (1.0 + trust_bonus + activity_bonus)
    }

    /// Generate a human-readable status report.
    pub fn status_report(&self) -> String {
        format!(
            "CommunityBootstrap {{ node: {}, tier: {}, social_capital: {:.1}, badges: {}, peers_onboarded: {}, votes: {}, knowledge: {}, healing: {} }}",
            self.node_id,
            self.tier_name(),
            self.total_social_capital,
            self.incentives.len(),
            self.peers_onboarded,
            self.governance_votes,
            self.knowledge_contributions,
            self.healing_actions,
        )
    }
}

/// Execute community bootstrap protocol.
///
/// Combines bootstrap peer discovery with initial badge awards
/// for early participation.
///
/// # Arguments
/// * `node_id` — The bootstrapping node ID
/// * `bootstrap_config` — Bootstrap discovery configuration
/// * `current_time` — Current Unix timestamp in seconds
///
/// # Returns
/// `CommunityBootstrap` state with discovered peers and initial incentives
pub fn community_bootstrap(
    node_id: String,
    bootstrap_config: &BootstrapConfig,
    current_time: u64,
) -> CommunityBootstrap {
    let mut state = CommunityBootstrap::new(node_id.clone());

    // Execute bootstrap discovery
    let result = execute_bootstrap_discovery(bootstrap_config, current_time);

    // Add discovered peers
    for peer in result.discovered_peers {
        state.add_peer(peer);
    }

    // Award Seed Guardian if this node is first in region (no peers discovered but has bootstrap config)
    if result.success && bootstrap_config.initial_peers.is_empty() {
        state.award_badge(
            CommunityBadge::SeedGuardian,
            current_time,
            "First node in new mesh region".to_string(),
        );
    }

    state
}

/// Compute social capital for a set of community bootstrap states.
///
/// Aggregates social capital across all nodes and computes
/// community-wide metrics.
///
/// # Arguments
/// * `bootstraps` — Slice of community bootstrap states
///
/// # Returns
/// Tuple of (total_social_capital, avg_social_capital, max_tier)
pub fn compute_social_capital(bootstraps: &[CommunityBootstrap]) -> (f64, f64, u32) {
    if bootstraps.is_empty() {
        return (0.0, 0.0, 0);
    }

    let total: f64 = bootstraps.iter().map(|b| b.total_social_capital).sum();
    let avg = total / bootstraps.len() as f64;
    let max_tier = bootstraps
        .iter()
        .map(|b| b.incentive_tier())
        .max()
        .unwrap_or(0);

    (total, avg, max_tier)
}

/// Check if a community bootstrap state is production-ready.
///
/// A community is considered production-ready when:
/// - At least 3 bootstrap peers discovered
/// - Average social capital >= 10.0
/// - At least one node at Steward tier or above
///
/// # Arguments
/// * `bootstraps` — Slice of community bootstrap states
///
/// # Returns
/// `true` if the community meets production readiness criteria
pub fn is_community_production_ready(bootstraps: &[CommunityBootstrap]) -> bool {
    if bootstraps.is_empty() {
        return false;
    }

    let total_peers: usize = bootstraps.iter().map(|b| b.bootstrap_peers.len()).sum();
    let (_total, avg_capital, max_tier) = compute_social_capital(bootstraps);

    total_peers >= 3 && avg_capital >= 10.0 && max_tier >= 2
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // GovernanceConfig tests

    #[test]
    fn test_governance_config_default() {
        let cfg = GovernanceConfig::default();
        assert_eq!(cfg.min_trust_to_vote, 0.3);
        assert_eq!(cfg.quorum_fraction, 0.5);
        assert_eq!(cfg.approval_threshold, 0.6);
        assert_eq!(cfg.voting_window_seconds, 86400);
        assert_eq!(cfg.max_active_proposals, 16);
    }

    #[test]
    fn test_governance_config_with_quorum() {
        let cfg = GovernanceConfig::default().with_quorum(0.75);
        assert_eq!(cfg.quorum_fraction, 0.75);
    }

    #[test]
    fn test_governance_config_quorum_clamped() {
        let cfg = GovernanceConfig::default().with_quorum(1.5);
        assert_eq!(cfg.quorum_fraction, 1.0);
    }

    #[test]
    fn test_governance_config_with_approval_threshold() {
        let cfg = GovernanceConfig::default().with_approval_threshold(0.8);
        assert_eq!(cfg.approval_threshold, 0.8);
    }

    #[test]
    fn test_governance_config_threshold_clamped() {
        let cfg = GovernanceConfig::default().with_approval_threshold(-0.1);
        assert_eq!(cfg.approval_threshold, 0.0);
    }

    #[test]
    fn test_governance_config_with_voting_window() {
        let cfg = GovernanceConfig::default().with_voting_window(3600);
        assert_eq!(cfg.voting_window_seconds, 3600);
    }

    #[test]
    fn test_governance_config_fast_test() {
        let cfg = GovernanceConfig::fast_test();
        assert_eq!(cfg.min_trust_to_vote, 0.1);
        assert_eq!(cfg.quorum_fraction, 0.3);
        assert_eq!(cfg.approval_threshold, 0.4);
        assert_eq!(cfg.voting_window_seconds, 60);
    }

    // Proposal tests

    #[test]
    fn test_proposal_creation() {
        let p = Proposal::new(
            "Test Proposal".to_string(),
            "Description".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        assert_eq!(p.status, ProposalStatus::Active);
        assert_eq!(p.created_at, 1000);
        assert_eq!(p.expires_at, 4600);
        assert!(p.votes.is_empty());
    }

    #[test]
    fn test_proposal_id_deterministic() {
        let p1 = Proposal::new(
            "Title".to_string(),
            "Desc".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        let p2 = Proposal::new(
            "Title".to_string(),
            "Desc".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn test_proposal_id_unique_per_proposer() {
        let p1 = Proposal::new(
            "Title".to_string(),
            "Desc".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        let p2 = Proposal::new(
            "Title".to_string(),
            "Desc".to_string(),
            "node2".to_string(),
            1000,
            3600,
        );
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn test_proposal_cast_vote() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        p.cast_vote("voter1".to_string(), 0.8, Vote::Approve);
        assert_eq!(p.votes.len(), 1);
        assert_eq!(p.votes[0].0, "voter1");
        assert_eq!(p.votes[0].1, 0.8);
        assert_eq!(p.votes[0].2, Vote::Approve);
    }

    #[test]
    fn test_proposal_vote_replacement() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        p.cast_vote("voter1".to_string(), 0.8, Vote::Approve);
        p.cast_vote("voter1".to_string(), 0.9, Vote::Reject);
        assert_eq!(p.votes.len(), 1);
        assert_eq!(p.votes[0].1, 0.9);
        assert_eq!(p.votes[0].2, Vote::Reject);
    }

    #[test]
    fn test_proposal_finalize_no_votes() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        let cfg = GovernanceConfig::fast_test();
        p.finalize(&cfg);
        assert_eq!(p.status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_proposal_finalize_passes() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        p.cast_vote("v1".to_string(), 1.0, Vote::Approve);
        p.cast_vote("v2".to_string(), 1.0, Vote::Approve);
        let cfg = GovernanceConfig::fast_test();
        p.finalize(&cfg);
        assert_eq!(p.status, ProposalStatus::Passed);
    }

    #[test]
    fn test_proposal_finalize_rejects() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        p.cast_vote("v1".to_string(), 1.0, Vote::Approve);
        p.cast_vote("v2".to_string(), 1.0, Vote::Reject);
        // Use default config: threshold 0.6, so 50% approval fails
        let cfg = GovernanceConfig::default();
        p.finalize(&cfg);
        assert_eq!(p.status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_proposal_finalize_quorum_not_met() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        // Only 1 voter with weight 0.1 out of total 10.0
        p.cast_vote("v1".to_string(), 0.1, Vote::Approve);
        p.cast_vote("v2".to_string(), 5.0, Vote::Abstain);
        p.cast_vote("v3".to_string(), 5.0, Vote::Abstain);
        let cfg = GovernanceConfig::fast_test();
        p.finalize(&cfg);
        // Quorum: participating (0.1) < total (10.1) * 0.3 = 3.03
        assert_eq!(p.status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_proposal_is_expired() {
        let p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        assert!(!p.is_expired(4599));
        assert!(p.is_expired(4600));
        assert!(p.is_expired(5000));
    }

    #[test]
    fn test_proposal_approval_ratio() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        p.cast_vote("v1".to_string(), 1.0, Vote::Approve);
        p.cast_vote("v2".to_string(), 1.0, Vote::Reject);
        assert_eq!(p.approval_ratio(), 0.5);
    }

    #[test]
    fn test_proposal_approval_ratio_empty() {
        let p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        assert_eq!(p.approval_ratio(), 0.0);
    }

    #[test]
    fn test_proposal_finalize_idempotent() {
        let mut p = Proposal::new(
            "P".to_string(),
            "D".to_string(),
            "node1".to_string(),
            1000,
            3600,
        );
        p.cast_vote("v1".to_string(), 1.0, Vote::Approve);
        let cfg = GovernanceConfig::fast_test();
        p.finalize(&cfg);
        let status = p.status;
        p.finalize(&cfg);
        assert_eq!(p.status, status);
    }

    // SymbioticCouncil tests

    #[test]
    fn test_council_new() {
        let council = SymbioticCouncil::new(GovernanceConfig::default());
        assert_eq!(council.proposal_counts(), (0, 0, 0, 0));
    }

    #[test]
    fn test_council_submit_proposal() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let id = council.submit_proposal(
            "P1".to_string(),
            "Desc".to_string(),
            "node1".to_string(),
            1000,
        );
        assert!(id.is_some());
        let counts = council.proposal_counts();
        assert_eq!(counts.0, 1); // active
    }

    #[test]
    fn test_council_submit_max_proposals() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        for i in 0..8 {
            let id = council.submit_proposal(
                format!("P{}", i),
                "Desc".to_string(),
                "node1".to_string(),
                1000,
            );
            assert!(id.is_some());
        }
        // 9th should fail (max is 8)
        let id = council.submit_proposal(
            "P9".to_string(),
            "Desc".to_string(),
            "node1".to_string(),
            1000,
        );
        assert!(id.is_none());
    }

    #[test]
    fn test_council_cast_vote() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let id = council
            .submit_proposal(
                "P1".to_string(),
                "Desc".to_string(),
                "node1".to_string(),
                1000,
            )
            .unwrap();
        let result = council.cast_vote(&id, "voter1".to_string(), 0.5, Vote::Approve);
        assert!(result);
    }

    #[test]
    fn test_council_cast_vote_low_trust() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let id = council
            .submit_proposal(
                "P1".to_string(),
                "Desc".to_string(),
                "node1".to_string(),
                1000,
            )
            .unwrap();
        // Trust 0.05 < min_trust 0.1
        let result = council.cast_vote(&id, "voter1".to_string(), 0.05, Vote::Approve);
        assert!(!result);
    }

    #[test]
    fn test_council_cast_vote_unknown_proposal() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let fake_id: Hash = [0u8; 32];
        let result = council.cast_vote(&fake_id, "voter1".to_string(), 0.5, Vote::Approve);
        assert!(!result);
    }

    #[test]
    fn test_council_finalize_proposals() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let id = council
            .submit_proposal(
                "P1".to_string(),
                "Desc".to_string(),
                "node1".to_string(),
                1000,
            )
            .unwrap();
        council.cast_vote(&id, "v1".to_string(), 1.0, Vote::Approve);
        let finalized = council.finalize_proposals();
        assert_eq!(finalized, 1);
        let counts = council.proposal_counts();
        assert_eq!(counts.1, 1); // passed
    }

    #[test]
    fn test_council_expire_proposals() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        council.submit_proposal(
            "P1".to_string(),
            "Desc".to_string(),
            "node1".to_string(),
            1000,
        );
        // Time 2000 > 1000 + 60 (voting window)
        let expired = council.expire_proposals(2000);
        assert_eq!(expired, 1);
    }

    #[test]
    fn test_council_get_proposal() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let id = council
            .submit_proposal(
                "P1".to_string(),
                "Desc".to_string(),
                "node1".to_string(),
                1000,
            )
            .unwrap();
        let proposal = council.get_proposal(&id);
        assert!(proposal.is_some());
    }

    #[test]
    fn test_council_get_proposal_missing() {
        let council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        let fake_id: Hash = [0u8; 32];
        assert!(council.get_proposal(&fake_id).is_none());
    }

    #[test]
    fn test_council_voting_power() {
        let mut council = SymbioticCouncil::new(GovernanceConfig::fast_test());
        assert_eq!(council.total_voting_power(), 0.0);
        council.set_total_voting_power(100.0);
        assert_eq!(council.total_voting_power(), 100.0);
    }

    // Value Distribution tests

    #[test]
    fn test_value_distribution_empty() {
        let result = compute_value_distribution(&[], 1000.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_value_distribution_single_node() {
        let nodes = vec![("node1".to_string(), 1.0, 100.0)];
        let result = compute_value_distribution(&nodes, 1000.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value_allocated, 1000.0);
        assert_eq!(result[0].share, 1.0);
    }

    #[test]
    fn test_value_distribution_equal_contribution() {
        let nodes = vec![
            ("n1".to_string(), 1.0, 100.0),
            ("n2".to_string(), 1.0, 100.0),
        ];
        let result = compute_value_distribution(&nodes, 1000.0);
        assert_eq!(result[0].value_allocated, 500.0);
        assert_eq!(result[1].value_allocated, 500.0);
    }

    #[test]
    fn test_value_distribution_trust_weighted() {
        let nodes = vec![
            ("n1".to_string(), 1.0, 100.0),
            ("n2".to_string(), 0.5, 100.0),
        ];
        let result = compute_value_distribution(&nodes, 900.0);
        // n1: 100*1.0 = 100, n2: 100*0.5 = 50, total = 150
        // n1 share = 100/150 = 2/3, n2 share = 50/150 = 1/3
        assert!((result[0].value_allocated - 600.0).abs() < 0.01);
        assert!((result[1].value_allocated - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_value_distribution_zero_contribution() {
        let nodes = vec![("n1".to_string(), 0.8, 0.0), ("n2".to_string(), 0.5, 0.0)];
        let result = compute_value_distribution(&nodes, 1000.0);
        // Equal distribution weighted by trust
        assert_eq!(result[0].value_allocated, 400.0); // 500 * 0.8
        assert_eq!(result[1].value_allocated, 250.0); // 500 * 0.5
    }

    #[test]
    fn test_value_distribution_share_method() {
        let entry = ValueDistribution {
            node_id: "n1".to_string(),
            share: 0.5,
            contribution_score: 50.0,
            value_allocated: 500.0,
        };
        assert_eq!(entry.share(100.0), 0.5);
    }

    // Bootstrap Peer tests

    #[test]
    fn test_bootstrap_peer_new() {
        let peer = BootstrapPeer::new("node1".to_string(), "127.0.0.1:8080".to_string(), 0.8);
        assert_eq!(peer.node_id, "node1");
        assert_eq!(peer.trust_score, 0.8);
        assert_eq!(peer.connection_count, 0);
    }

    #[test]
    fn test_bootstrap_peer_is_active() {
        let peer = BootstrapPeer::new("n".to_string(), "addr".to_string(), 0.8);
        assert!(peer.is_active(0.5));
        assert!(!peer.is_active(0.9));
    }

    // Bootstrap Config tests

    #[test]
    fn test_bootstrap_config_default() {
        let cfg = BootstrapConfig::default();
        assert!(cfg.initial_peers.is_empty());
        assert_eq!(cfg.timeout_ms, 5000);
        assert_eq!(cfg.max_peers, 32);
        assert_eq!(cfg.min_trust, 0.5);
    }

    #[test]
    fn test_bootstrap_config_with_peers() {
        let peers = vec![BootstrapPeer::new(
            "n1".to_string(),
            "addr".to_string(),
            0.8,
        )];
        let cfg = BootstrapConfig::with_peers(peers);
        assert_eq!(cfg.initial_peers.len(), 1);
    }

    #[test]
    fn test_bootstrap_config_with_timeout() {
        let cfg = BootstrapConfig::default().with_timeout(10000);
        assert_eq!(cfg.timeout_ms, 10000);
    }

    #[test]
    fn test_bootstrap_config_with_max_peers() {
        let cfg = BootstrapConfig::default().with_max_peers(16);
        assert_eq!(cfg.max_peers, 16);
    }

    // Bootstrap Result tests

    #[test]
    fn test_bootstrap_result_success() {
        let peers = vec![BootstrapPeer::new(
            "n1".to_string(),
            "addr".to_string(),
            0.8,
        )];
        let result = BootstrapResult::success(peers, 100);
        assert!(result.success);
        assert_eq!(result.peers_found, 1);
        assert_eq!(result.discovery_time_ms, 100);
    }

    #[test]
    fn test_bootstrap_result_failure() {
        let result = BootstrapResult::failure(500);
        assert!(!result.success);
        assert_eq!(result.peers_found, 0);
        assert!(result.discovered_peers.is_empty());
    }

    #[test]
    fn test_bootstrap_result_active_peers() {
        let peers = vec![
            BootstrapPeer::new("n1".to_string(), "a1".to_string(), 0.9),
            BootstrapPeer::new("n2".to_string(), "a2".to_string(), 0.3),
        ];
        let result = BootstrapResult::success(peers, 50);
        let active = result.active_peers(0.5);
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_bootstrap_result_avg_trust() {
        let peers = vec![
            BootstrapPeer::new("n1".to_string(), "a1".to_string(), 0.8),
            BootstrapPeer::new("n2".to_string(), "a2".to_string(), 0.6),
        ];
        let result = BootstrapResult::success(peers, 50);
        assert_eq!(result.avg_trust(), 0.7);
    }

    #[test]
    fn test_bootstrap_result_avg_trust_empty() {
        let result = BootstrapResult::failure(100);
        assert_eq!(result.avg_trust(), 0.0);
    }

    // Bootstrap Discovery tests

    #[test]
    fn test_execute_bootstrap_discovery_empty() {
        let cfg = BootstrapConfig::default();
        let result = execute_bootstrap_discovery(&cfg, 1000);
        assert!(!result.success);
    }

    #[test]
    fn test_execute_bootstrap_discovery_success() {
        let peers = vec![
            BootstrapPeer::new("n1".to_string(), "a1".to_string(), 0.9),
            BootstrapPeer::new("n2".to_string(), "a2".to_string(), 0.7),
        ];
        let cfg = BootstrapConfig::with_peers(peers);
        let result = execute_bootstrap_discovery(&cfg, 1000);
        assert!(result.success);
        assert_eq!(result.peers_found, 2);
    }

    #[test]
    fn test_execute_bootstrap_discovery_filters_low_trust() {
        let peers = vec![
            BootstrapPeer::new("n1".to_string(), "a1".to_string(), 0.9),
            BootstrapPeer::new("n2".to_string(), "a2".to_string(), 0.3),
        ];
        let cfg = BootstrapConfig::with_peers(peers);
        let result = execute_bootstrap_discovery(&cfg, 1000);
        assert_eq!(result.peers_found, 1);
    }

    #[test]
    fn test_execute_bootstrap_discovery_respects_max() {
        let peers: Vec<_> = (0..10)
            .map(|i| BootstrapPeer::new(format!("n{}", i), format!("a{}", i), 0.8))
            .collect();
        let cfg = BootstrapConfig::with_peers(peers).with_max_peers(5);
        let result = execute_bootstrap_discovery(&cfg, 1000);
        assert_eq!(result.peers_found, 5);
    }

    // Optimal Peer Selection tests

    #[test]
    fn test_select_optimal_peers_empty() {
        let result = select_optimal_bootstrap_peers(&[], 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_select_optimal_peers_sorted() {
        let peers = vec![
            BootstrapPeer::new("n1".to_string(), "a1".to_string(), 0.5),
            BootstrapPeer::new("n2".to_string(), "a2".to_string(), 0.9),
            BootstrapPeer::new("n3".to_string(), "a3".to_string(), 0.7),
        ];
        let result = select_optimal_bootstrap_peers(&peers, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].trust_score, 0.9);
        assert_eq!(result[1].trust_score, 0.7);
    }

    #[test]
    fn test_select_optimal_peers_count_exceeds() {
        let peers = vec![BootstrapPeer::new("n1".to_string(), "a1".to_string(), 0.8)];
        let result = select_optimal_bootstrap_peers(&peers, 10);
        assert_eq!(result.len(), 1);
    }

    // ---------------------------------------------------------------------------
    // PASO C — Community Bootstrap + No-Econ Incentives tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_community_badge_display() {
        assert_eq!(format!("{}", CommunityBadge::SeedGuardian), "Seed Guardian");
        assert_eq!(format!("{}", CommunityBadge::MeshHealer), "Mesh Healer");
        assert_eq!(
            format!("{}", CommunityBadge::KnowledgeSharer),
            "Knowledge Sharer"
        );
        assert_eq!(format!("{}", CommunityBadge::IronUptime), "Iron Uptime");
        assert_eq!(
            format!("{}", CommunityBadge::CommunityBuilder),
            "Community Builder"
        );
        assert_eq!(format!("{}", CommunityBadge::ProofForge), "Proof Forge");
        assert_eq!(format!("{}", CommunityBadge::GreenSteward), "Green Steward");
        assert_eq!(format!("{}", CommunityBadge::CivicVoice), "Civic Voice");
    }

    #[test]
    fn test_no_econ_incentive_new() {
        let inc = NoEconIncentive::new(
            "node1".to_string(),
            CommunityBadge::SeedGuardian,
            1000,
            "First in region".to_string(),
        );
        assert_eq!(inc.node_id, "node1");
        assert_eq!(inc.badge, CommunityBadge::SeedGuardian);
        assert_eq!(inc.awarded_at, 1000);
        assert_eq!(inc.social_capital_points, 10.0);
    }

    #[test]
    fn test_no_econ_incentive_badge_points() {
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::SeedGuardian),
            10.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::MeshHealer),
            15.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::KnowledgeSharer),
            12.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::IronUptime),
            20.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::CommunityBuilder),
            18.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::ProofForge),
            25.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::GreenSteward),
            22.0
        );
        assert_eq!(
            NoEconIncentive::badge_points(CommunityBadge::CivicVoice),
            8.0
        );
    }

    #[test]
    fn test_community_bootstrap_new() {
        let bs = CommunityBootstrap::new("node1".to_string());
        assert_eq!(bs.node_id, "node1");
        assert!(bs.bootstrap_peers.is_empty());
        assert!(bs.incentives.is_empty());
        assert_eq!(bs.total_social_capital, 0.0);
        assert_eq!(bs.peers_onboarded, 0);
        assert_eq!(bs.governance_votes, 0);
        assert_eq!(bs.knowledge_contributions, 0);
        assert_eq!(bs.healing_actions, 0);
    }

    #[test]
    fn test_community_bootstrap_add_peer() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.add_peer(BootstrapPeer::new(
            "p1".to_string(),
            "addr1".to_string(),
            0.8,
        ));
        assert_eq!(bs.bootstrap_peers.len(), 1);
        assert_eq!(bs.bootstrap_peers[0].node_id, "p1");
    }

    #[test]
    fn test_community_bootstrap_award_badge() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(
            CommunityBadge::ProofForge,
            2000,
            "Formal proof contributed".to_string(),
        );
        assert_eq!(bs.incentives.len(), 1);
        assert_eq!(bs.total_social_capital, 25.0);
        assert_eq!(bs.incentives[0].badge, CommunityBadge::ProofForge);
    }

    #[test]
    fn test_community_bootstrap_multiple_badges() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        bs.award_badge(CommunityBadge::MeshHealer, 1100, "Heal".to_string());
        assert_eq!(bs.incentives.len(), 2);
        assert_eq!(bs.total_social_capital, 25.0); // 10 + 15
    }

    #[test]
    fn test_community_bootstrap_record_onboarding() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        for _ in 0..4 {
            bs.record_onboarding();
        }
        assert_eq!(bs.peers_onboarded, 4);
        // No badge yet (need 5)
        assert!(bs.incentives.is_empty());

        bs.record_onboarding();
        assert_eq!(bs.peers_onboarded, 5);
        assert_eq!(bs.incentives.len(), 1);
        assert_eq!(bs.incentives[0].badge, CommunityBadge::CommunityBuilder);
    }

    #[test]
    fn test_community_bootstrap_record_governance_vote() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        for _ in 0..9 {
            bs.record_governance_vote();
        }
        assert_eq!(bs.governance_votes, 9);
        assert!(bs.incentives.is_empty());

        bs.record_governance_vote();
        assert_eq!(bs.governance_votes, 10);
        assert_eq!(bs.incentives.len(), 1);
        assert_eq!(bs.incentives[0].badge, CommunityBadge::CivicVoice);
    }

    #[test]
    fn test_community_bootstrap_record_knowledge_contribution() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.record_knowledge_contribution();
        assert_eq!(bs.knowledge_contributions, 1);
        assert_eq!(bs.incentives.len(), 1);
        assert_eq!(bs.incentives[0].badge, CommunityBadge::KnowledgeSharer);
    }

    #[test]
    fn test_community_bootstrap_record_healing_action() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.record_healing_action();
        assert_eq!(bs.healing_actions, 1);
        assert_eq!(bs.incentives.len(), 1);
        assert_eq!(bs.incentives[0].badge, CommunityBadge::MeshHealer);
    }

    #[test]
    fn test_incentive_tier_newcomer() {
        let bs = CommunityBootstrap::new("node1".to_string());
        assert_eq!(bs.incentive_tier(), 0);
        assert_eq!(bs.tier_name(), "Newcomer");
    }

    #[test]
    fn test_incentive_tier_contributor() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::CivicVoice, 1000, "Vote".to_string());
        assert_eq!(bs.incentive_tier(), 1);
        assert_eq!(bs.tier_name(), "Contributor");
    }

    #[test]
    fn test_incentive_tier_steward() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        bs.award_badge(CommunityBadge::MeshHealer, 1100, "Heal".to_string());
        assert_eq!(bs.incentive_tier(), 2);
        assert_eq!(bs.tier_name(), "Steward");
    }

    #[test]
    fn test_incentive_tier_guardian() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        // Need >= 50 points: ProofForge(25) + GreenSteward(22) + CivicVoice(8) = 55
        bs.award_badge(CommunityBadge::ProofForge, 1000, "Proof".to_string());
        bs.award_badge(CommunityBadge::GreenSteward, 1100, "Green".to_string());
        bs.award_badge(CommunityBadge::CivicVoice, 1200, "Vote".to_string());
        assert_eq!(bs.incentive_tier(), 3);
        assert_eq!(bs.tier_name(), "Guardian");
    }

    #[test]
    fn test_incentive_tier_legend() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        // Need >= 100 points: 5x ProofForge = 125
        for i in 0..5 {
            bs.award_badge(CommunityBadge::ProofForge, 1000 + i, "Proof".to_string());
        }
        assert_eq!(bs.incentive_tier(), 4);
        assert_eq!(bs.tier_name(), "Legend");
    }

    #[test]
    fn test_is_seed_guardian() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        assert!(!bs.is_seed_guardian()); // No peers onboarded
        bs.record_onboarding();
        assert!(bs.is_seed_guardian()); // No bootstrap peers, has onboarded
    }

    #[test]
    fn test_is_seed_guardian_false_with_peers() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.add_peer(BootstrapPeer::new("p1".to_string(), "a1".to_string(), 0.8));
        bs.record_onboarding();
        assert!(!bs.is_seed_guardian()); // Has bootstrap peers
    }

    #[test]
    fn test_reputation_score_zero() {
        let bs = CommunityBootstrap::new("node1".to_string());
        let score = bs.reputation_score(0.5);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_reputation_score_basic() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        // social_capital = 10, trust = 0.8, actions = 0
        // score = 10 * (1.0 + 0.8*0.5 + 0) = 10 * 1.4 = 14.0
        let score = bs.reputation_score(0.8);
        assert!((score - 14.0).abs() < 0.01);
    }

    #[test]
    fn test_reputation_score_with_activity() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        for _ in 0..20 {
            bs.record_governance_vote();
        }
        // social_capital = 10 + 8 = 18 (CivicVoice awarded at 10 votes)
        // trust = 0.5, actions = 20
        // activity_bonus = min(20/100, 0.5) = 0.2
        // score = 18 * (1.0 + 0.25 + 0.2) = 18 * 1.45 = 26.1
        let score = bs.reputation_score(0.5);
        assert!(score > 20.0);
    }

    #[test]
    fn test_reputation_score_clamped() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        // Trust > 1.0 should be clamped
        let score = bs.reputation_score(2.0);
        // score = 10 * (1.0 + 0.5 + 0) = 15.0
        assert!((score - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_status_report_contains_fields() {
        let mut bs = CommunityBootstrap::new("test-node".to_string());
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        let report = bs.status_report();
        assert!(report.contains("test-node"));
        assert!(report.contains("Contributor"));
        assert!(report.contains("10.0"));
        assert!(report.contains("badges: 1"));
    }

    #[test]
    fn test_community_bootstrap_basic() {
        let peers = vec![
            BootstrapPeer::new("p1".to_string(), "a1".to_string(), 0.8),
            BootstrapPeer::new("p2".to_string(), "a2".to_string(), 0.9),
        ];
        let config = BootstrapConfig::with_peers(peers);
        let result = community_bootstrap("node1".to_string(), &config, 1000);
        assert_eq!(result.node_id, "node1");
        assert_eq!(result.bootstrap_peers.len(), 2);
    }

    #[test]
    fn test_community_bootstrap_awards_seed_guardian() {
        // Empty initial_peers + success = Seed Guardian
        let config = BootstrapConfig::default();
        let result = community_bootstrap("node1".to_string(), &config, 1000);
        // No peers discovered, no Seed Guardian (success is false when no peers)
        assert!(!result
            .incentives
            .iter()
            .any(|i| i.badge == CommunityBadge::SeedGuardian));
    }

    #[test]
    fn test_compute_social_capital_empty() {
        let (total, avg, tier) = compute_social_capital(&[]);
        assert_eq!(total, 0.0);
        assert_eq!(avg, 0.0);
        assert_eq!(tier, 0);
    }

    #[test]
    fn test_compute_social_capital_single() {
        let mut bs = CommunityBootstrap::new("node1".to_string());
        bs.award_badge(CommunityBadge::ProofForge, 1000, "Proof".to_string());
        let (total, avg, tier) = compute_social_capital(&[bs]);
        assert_eq!(total, 25.0);
        assert_eq!(avg, 25.0);
        assert_eq!(tier, 2); // Steward
    }

    #[test]
    fn test_compute_social_capital_multiple() {
        let mut bs1 = CommunityBootstrap::new("n1".to_string());
        bs1.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        let mut bs2 = CommunityBootstrap::new("n2".to_string());
        bs2.award_badge(CommunityBadge::ProofForge, 1000, "Proof".to_string());
        let (total, avg, tier) = compute_social_capital(&[bs1, bs2]);
        assert_eq!(total, 35.0);
        assert!((avg - 17.5).abs() < 0.01);
        assert_eq!(tier, 2); // Steward
    }

    #[test]
    fn test_is_community_production_ready_empty() {
        assert!(!is_community_production_ready(&[]));
    }

    #[test]
    fn test_is_community_production_ready_insufficient_peers() {
        let bs = CommunityBootstrap::new("n1".to_string());
        assert!(!is_community_production_ready(&[bs]));
    }

    #[test]
    fn test_is_community_production_ready_insufficient_tier() {
        let mut bs = CommunityBootstrap::new("n1".to_string());
        bs.add_peer(BootstrapPeer::new("p1".to_string(), "a1".to_string(), 0.8));
        bs.add_peer(BootstrapPeer::new("p2".to_string(), "a2".to_string(), 0.8));
        bs.add_peer(BootstrapPeer::new("p3".to_string(), "a3".to_string(), 0.8));
        // Only 10 social capital, tier = 1 (Contributor)
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        assert!(!is_community_production_ready(&[bs]));
    }

    #[test]
    fn test_is_community_production_ready_all_pass() {
        let mut bs = CommunityBootstrap::new("n1".to_string());
        bs.add_peer(BootstrapPeer::new("p1".to_string(), "a1".to_string(), 0.8));
        bs.add_peer(BootstrapPeer::new("p2".to_string(), "a2".to_string(), 0.8));
        bs.add_peer(BootstrapPeer::new("p3".to_string(), "a3".to_string(), 0.8));
        // Need >= 10 social capital + tier >= 2 (Steward)
        bs.award_badge(CommunityBadge::SeedGuardian, 1000, "Seed".to_string());
        bs.award_badge(CommunityBadge::MeshHealer, 1100, "Heal".to_string());
        // total = 25, avg = 25, tier = 2
        assert!(is_community_production_ready(&[bs]));
    }

    #[test]
    fn test_full_community_bootstrap_lifecycle() {
        let mut bs = CommunityBootstrap::new("steward-1".to_string());

        // Add peers
        for i in 0..5 {
            bs.add_peer(BootstrapPeer::new(
                format!("p{}", i),
                format!("addr{}", i),
                0.7 + i as f64 * 0.05,
            ));
        }

        // Record activities
        bs.record_knowledge_contribution();
        bs.record_healing_action();
        for _ in 0..3 {
            bs.record_onboarding();
        }

        // Check state
        assert_eq!(bs.bootstrap_peers.len(), 5);
        assert_eq!(bs.knowledge_contributions, 1);
        assert_eq!(bs.healing_actions, 1);
        assert_eq!(bs.peers_onboarded, 3);
        // KnowledgeSharer(12) + MeshHealer(15) = 27
        assert!((bs.total_social_capital - 27.0).abs() < 0.01);
        assert_eq!(bs.incentive_tier(), 2); // Steward
        assert_eq!(bs.tier_name(), "Steward");

        let report = bs.status_report();
        assert!(report.contains("steward-1"));
        assert!(report.contains("Steward"));
    }
}
