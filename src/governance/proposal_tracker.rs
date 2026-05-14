//! Proposal Tracker — Tracks DAO technical proposals through their lifecycle.
//!
//! Manages the full lifecycle of technical proposals: creation, voting,
//! execution tracking and completion. Proposals are weighted by technical
//! staking from the TechnicalStaking module.
//! Analogous to Linux's `systemd` unit lifecycle management but for
//! federated governance proposals.
//!
//! Zero financial logic: proposals are technical operations only.

use std::collections::HashMap;

/// Errors for proposal tracking operations.
#[derive(Debug)]
pub enum ProposalError {
    ProposalNotFound(String),
    DuplicateProposal(String),
    InvalidStateTransition(String),
    VotingNotOpen(String),
    InsufficientWeight { available: f64, required: f64 },
    AlreadyVoted(String),
    ProposalExpired(String),
}

impl std::fmt::Display for ProposalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalError::ProposalNotFound(id) => {
                write!(f, "Proposal not found: {}", id)
            }
            ProposalError::DuplicateProposal(id) => {
                write!(f, "Duplicate proposal: {}", id)
            }
            ProposalError::InvalidStateTransition(msg) => {
                write!(f, "Invalid state transition: {}", msg)
            }
            ProposalError::VotingNotOpen(id) => {
                write!(f, "Voting not open: {}", id)
            }
            ProposalError::InsufficientWeight { available, required } => {
                write!(f, "Insufficient voting weight: available={}, required={}", available, required)
            }
            ProposalError::AlreadyVoted(id) => {
                write!(f, "Already voted: {}", id)
            }
            ProposalError::ProposalExpired(id) => {
                write!(f, "Proposal expired: {}", id)
            }
        }
    }
}

/// Configuration for proposal tracking.
#[derive(Debug, Clone)]
pub struct ProposalConfig {
    /// Default voting duration in milliseconds.
    pub default_voting_duration_ms: u64,
    /// Minimum voting weight to create a proposal.
    pub min_proposer_weight: f64,
    /// Quorum threshold (fraction of total weight needed to pass).
    pub quorum_threshold: f64,
    /// Approval threshold (fraction of votes needed to pass).
    pub approval_threshold: f64,
    /// Maximum proposals allowed simultaneously.
    pub max_active_proposals: usize,
}

impl Default for ProposalConfig {
    fn default() -> Self {
        Self {
            default_voting_duration_ms: 86_400_000, // 24 hours
            min_proposer_weight: 0.05,
            quorum_threshold: 0.3,
            approval_threshold: 0.5,
            max_active_proposals: 50,
        }
    }
}

/// Current state of a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalState {
    Draft,
    Voting,
    Passed,
    Rejected,
    Executed,
    Expired,
}

impl std::fmt::Display for ProposalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalState::Draft => write!(f, "Draft"),
            ProposalState::Voting => write!(f, "Voting"),
            ProposalState::Passed => write!(f, "Passed"),
            ProposalState::Rejected => write!(f, "Rejected"),
            ProposalState::Executed => write!(f, "Executed"),
            ProposalState::Expired => write!(f, "Expired"),
        }
    }
}

/// A vote on a proposal.
#[derive(Debug, Clone)]
pub struct ProposalVote {
    /// Voter node identifier.
    pub voter_id: String,
    /// Vote: true = yes, false = no.
    pub vote_yes: bool,
    /// Voting weight of this voter.
    pub voting_weight: f64,
    /// Vote timestamp (ms).
    pub timestamp_ms: u64,
}

impl ProposalVote {
    /// Create a new vote.
    pub fn new(voter_id: String, vote_yes: bool, voting_weight: f64) -> Self {
        Self {
            voter_id,
            vote_yes,
            voting_weight,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

/// A technical proposal in the DAO.
#[derive(Debug, Clone)]
pub struct Proposal {
    /// Unique proposal identifier.
    pub proposal_id: String,
    /// Title of the proposal.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Current state.
    pub state: ProposalState,
    /// Proposer node identifier.
    pub proposer_id: String,
    /// Proposer voting weight.
    pub proposer_weight: f64,
    /// Voting start timestamp (ms).
    pub voting_start_ms: u64,
    /// Voting end timestamp (ms).
    pub voting_end_ms: u64,
    /// All votes cast.
    pub votes: Vec<ProposalVote>,
    /// Total yes weight.
    pub yes_weight: f64,
    /// Total no weight.
    pub no_weight: f64,
    /// Creation timestamp (ms).
    pub created_at_ms: u64,
    /// Resolution timestamp (ms), if applicable.
    pub resolved_at_ms: Option<u64>,
}

impl Proposal {
    /// Create a new proposal in Draft state.
    pub fn new(
        proposal_id: String,
        title: String,
        description: String,
        proposer_id: String,
        proposer_weight: f64,
        voting_duration_ms: u64,
    ) -> Self {
        let now = current_timestamp_ms();
        Self {
            proposal_id,
            title,
            description,
            state: ProposalState::Draft,
            proposer_id,
            proposer_weight,
            voting_start_ms: 0,
            voting_end_ms: now + voting_duration_ms,
            votes: Vec::new(),
            yes_weight: 0.0,
            no_weight: 0.0,
            created_at_ms: now,
            resolved_at_ms: None,
        }
    }

    /// Check if voting is currently open.
    pub fn is_voting_open(&self) -> bool {
        self.state == ProposalState::Voting
    }

    /// Check if proposal has expired.
    pub fn has_expired(&self) -> bool {
        if self.state != ProposalState::Voting {
            return false;
        }
        current_timestamp_ms() > self.voting_end_ms
    }

    /// Total voting weight cast.
    pub fn total_weight_cast(&self) -> f64 {
        self.yes_weight + self.no_weight
    }

    /// Vote count.
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }
}

/// Proposal tracking statistics.
#[derive(Debug, Clone, Default)]
pub struct ProposalStats {
    /// Total proposals created.
    pub total_proposals: usize,
    /// Currently active (voting) proposals.
    pub active_proposals: usize,
    /// Total proposals passed.
    pub passed_proposals: usize,
    /// Total proposals rejected.
    pub rejected_proposals: usize,
    /// Total votes cast.
    pub total_votes: usize,
}

/// Proposal Tracker engine.
pub struct ProposalTracker {
    /// Tracker configuration.
    pub config: ProposalConfig,
    /// All proposals by ID.
    proposals: HashMap<String, Proposal>,
    /// Voting weight registry (node_id -> weight).
    voting_weights: HashMap<String, f64>,
    /// Tracking statistics.
    stats: ProposalStats,
}

impl ProposalTracker {
    /// Create a new proposal tracker with config.
    pub fn new(config: ProposalConfig) -> Self {
        Self {
            config,
            proposals: HashMap::new(),
            voting_weights: HashMap::new(),
            stats: ProposalStats::default(),
        }
    }

    /// Create tracker with default config.
    pub fn with_defaults() -> Self {
        Self::new(ProposalConfig::default())
    }

    /// Register a node's voting weight.
    pub fn register_voter(&mut self, node_id: String, weight: f64) {
        self.voting_weights.insert(node_id, weight.clamp(0.0, 1.0));
    }

    /// Update a node's voting weight.
    pub fn update_voter_weight(&mut self, node_id: &str, weight: f64) {
        self.voting_weights.insert(node_id.to_string(), weight.clamp(0.0, 1.0));
    }

    /// Create a new proposal.
    pub fn create_proposal(
        &mut self,
        proposal_id: String,
        title: String,
        description: String,
        proposer_id: &str,
    ) -> Result<Proposal, ProposalError> {
        // Check duplicate
        if self.proposals.contains_key(&proposal_id) {
            return Err(ProposalError::DuplicateProposal(proposal_id.clone()));
        }

        // Check proposer weight
        let weight = self.voting_weights.get(proposer_id)
            .ok_or(ProposalError::InsufficientWeight {
                available: 0.0,
                required: self.config.min_proposer_weight,
            })?;

        if *weight < self.config.min_proposer_weight {
            return Err(ProposalError::InsufficientWeight {
                available: *weight,
                required: self.config.min_proposer_weight,
            });
        }

        // Create proposal
        let proposal = Proposal::new(
            proposal_id.clone(),
            title,
            description,
            proposer_id.to_string(),
            *weight,
            self.config.default_voting_duration_ms,
        );

        self.proposals.insert(proposal_id.clone(), proposal);
        self.stats.total_proposals += 1;

        Ok(self.proposals.get(&proposal_id).unwrap().clone())
    }

    /// Open voting for a proposal (Draft -> Voting).
    pub fn open_voting(&mut self, proposal_id: &str) -> Result<(), ProposalError> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or(ProposalError::ProposalNotFound(proposal_id.to_string()))?;

        if proposal.state != ProposalState::Draft {
            return Err(ProposalError::InvalidStateTransition(
                format!("Expected Draft, got {}", proposal.state),
            ));
        }

        let now = current_timestamp_ms();
        proposal.state = ProposalState::Voting;
        proposal.voting_start_ms = now;
        proposal.voting_end_ms = now + self.config.default_voting_duration_ms;
        self.stats.active_proposals += 1;

        Ok(())
    }

    /// Cast a vote on a proposal.
    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter_id: &str,
        vote_yes: bool,
    ) -> Result<ProposalVote, ProposalError> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or(ProposalError::ProposalNotFound(proposal_id.to_string()))?;

        // Check voting is open
        if !proposal.is_voting_open() {
            return Err(ProposalError::VotingNotOpen(proposal_id.to_string()));
        }

        // Check expiration
        if proposal.has_expired() {
            proposal.state = ProposalState::Expired;
            proposal.resolved_at_ms = Some(current_timestamp_ms());
            self.stats.active_proposals = self.stats.active_proposals.saturating_sub(1);
            return Err(ProposalError::ProposalExpired(proposal_id.to_string()));
        }

        // Check already voted
        if proposal.votes.iter().any(|v| v.voter_id == voter_id) {
            return Err(ProposalError::AlreadyVoted(voter_id.to_string()));
        }

        // Get voter weight
        let weight = self.voting_weights.get(voter_id)
            .ok_or(ProposalError::InsufficientWeight {
                available: 0.0,
                required: 0.01,
            })?;

        // Record vote
        let vote = ProposalVote::new(voter_id.to_string(), vote_yes, *weight);
        if vote_yes {
            proposal.yes_weight += *weight;
        } else {
            proposal.no_weight += *weight;
        }
        proposal.votes.push(vote.clone());
        self.stats.total_votes += 1;

        Ok(vote)
    }

    /// Tally votes and resolve proposal.
    pub fn tally_proposal(&mut self, proposal_id: &str) -> Result<ProposalState, ProposalError> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or(ProposalError::ProposalNotFound(proposal_id.to_string()))?;

        if proposal.state != ProposalState::Voting {
            return Err(ProposalError::InvalidStateTransition(
                format!("Expected Voting, got {}", proposal.state),
            ));
        }

        let total_weight = proposal.total_weight_cast();
        let quorum_met = total_weight >= self.config.quorum_threshold;

        let new_state = if !quorum_met {
            ProposalState::Rejected
        } else if proposal.yes_weight / total_weight >= self.config.approval_threshold {
            ProposalState::Passed
        } else {
            ProposalState::Rejected
        };

        proposal.state = new_state.clone();
        proposal.resolved_at_ms = Some(current_timestamp_ms());
        self.stats.active_proposals = self.stats.active_proposals.saturating_sub(1);

        match &new_state {
            ProposalState::Passed => self.stats.passed_proposals += 1,
            ProposalState::Rejected => self.stats.rejected_proposals += 1,
            _ => {}
        }

        Ok(new_state)
    }

    /// Mark a passed proposal as executed.
    pub fn execute_proposal(&mut self, proposal_id: &str) -> Result<(), ProposalError> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or(ProposalError::ProposalNotFound(proposal_id.to_string()))?;

        if proposal.state != ProposalState::Passed {
            return Err(ProposalError::InvalidStateTransition(
                format!("Expected Passed, got {}", proposal.state),
            ));
        }

        proposal.state = ProposalState::Executed;
        Ok(())
    }

    /// Get a proposal by ID.
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&Proposal> {
        self.proposals.get(proposal_id)
    }

    /// Get all proposals in a given state.
    pub fn get_proposals_by_state(&self, state: &ProposalState) -> Vec<&Proposal> {
        self.proposals.values()
            .filter(|p| &p.state == state)
            .collect()
    }

    /// Get all active (voting) proposals.
    pub fn get_active_proposals(&self) -> Vec<&Proposal> {
        self.get_proposals_by_state(&ProposalState::Voting)
    }

    /// Check and expire any expired proposals.
    pub fn check_expirations(&mut self) -> usize {
        let mut expired = 0;
        for proposal in self.proposals.values_mut() {
            if proposal.has_expired() {
                proposal.state = ProposalState::Expired;
                proposal.resolved_at_ms = Some(current_timestamp_ms());
                expired += 1;
            }
        }
        if expired > 0 {
            self.stats.active_proposals = self.stats.active_proposals.saturating_sub(expired);
        }
        expired
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> ProposalStats {
        self.stats.clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ProposalStats::default();
        self.stats.total_proposals = self.proposals.len();
    }
}

impl Default for ProposalTracker {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Helper to get current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = ProposalTracker::with_defaults();
        assert_eq!(tracker.get_stats().total_proposals, 0);
    }

    #[test]
    fn test_register_voter() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.5);
        assert_eq!(*tracker.voting_weights.get("node1").unwrap(), 0.5);
    }

    #[test]
    fn test_create_proposal() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.5);
        let proposal = tracker.create_proposal(
            "p1".to_string(),
            "Test Proposal".to_string(),
            "Description".to_string(),
            "node1",
        ).unwrap();
        assert_eq!(proposal.proposal_id, "p1");
        assert_eq!(proposal.state, ProposalState::Draft);
    }

    #[test]
    fn test_create_proposal_insufficient_weight() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.01);
        match tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ) {
            Err(ProposalError::InsufficientWeight { .. }) => {},
            _ => panic!("Expected InsufficientWeight"),
        }
    }

    #[test]
    fn test_duplicate_proposal() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.5);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        match tracker.create_proposal(
            "p1".to_string(),
            "Test2".to_string(),
            "Desc2".to_string(),
            "node1",
        ) {
            Err(ProposalError::DuplicateProposal(id)) => assert_eq!(id, "p1"),
            _ => panic!("Expected DuplicateProposal"),
        }
    }

    #[test]
    fn test_open_voting() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.5);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        assert!(tracker.open_voting("p1").is_ok());
        assert!(tracker.get_proposal("p1").unwrap().is_voting_open());
    }

    #[test]
    fn test_cast_vote() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.5);
        tracker.register_voter("node2".to_string(), 0.3);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        tracker.open_voting("p1").unwrap();
        let vote = tracker.cast_vote("p1", "node2", true).unwrap();
        assert!(vote.vote_yes);
        assert_eq!(vote.voting_weight, 0.3);
    }

    #[test]
    fn test_already_voted() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.5);
        tracker.register_voter("node2".to_string(), 0.3);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        tracker.open_voting("p1").unwrap();
        tracker.cast_vote("p1", "node2", true).unwrap();
        match tracker.cast_vote("p1", "node2", false) {
            Err(ProposalError::AlreadyVoted(id)) => assert_eq!(id, "node2"),
            _ => panic!("Expected AlreadyVoted"),
        }
    }

    #[test]
    fn test_tally_passed() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.6);
        tracker.register_voter("node2".to_string(), 0.4);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        tracker.open_voting("p1").unwrap();
        tracker.cast_vote("p1", "node1", true).unwrap();
        tracker.cast_vote("p1", "node2", true).unwrap();
        let state = tracker.tally_proposal("p1").unwrap();
        assert_eq!(state, ProposalState::Passed);
    }

    #[test]
    fn test_tally_rejected() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.3);
        tracker.register_voter("node2".to_string(), 0.6);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node2",
        ).unwrap();
        tracker.open_voting("p1").unwrap();
        tracker.cast_vote("p1", "node1", true).unwrap();
        tracker.cast_vote("p1", "node2", false).unwrap();
        let state = tracker.tally_proposal("p1").unwrap();
        // 0.3 yes vs 0.6 no → 0.3/0.9 = 33% < 50% approval threshold
        assert_eq!(state, ProposalState::Rejected);
    }

    #[test]
    fn test_execute_proposal() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.6);
        tracker.register_voter("node2".to_string(), 0.4);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        tracker.open_voting("p1").unwrap();
        tracker.cast_vote("p1", "node1", true).unwrap();
        tracker.cast_vote("p1", "node2", true).unwrap();
        tracker.tally_proposal("p1").unwrap();
        assert!(tracker.execute_proposal("p1").is_ok());
        assert_eq!(tracker.get_proposal("p1").unwrap().state, ProposalState::Executed);
    }

    #[test]
    fn test_stats_tracking() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.6);
        tracker.register_voter("node2".to_string(), 0.4);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        tracker.open_voting("p1").unwrap();
        tracker.cast_vote("p1", "node1", true).unwrap();
        tracker.cast_vote("p1", "node2", true).unwrap();
        tracker.tally_proposal("p1").unwrap();
        let stats = tracker.get_stats();
        assert_eq!(stats.total_proposals, 1);
        assert_eq!(stats.passed_proposals, 1);
        assert_eq!(stats.total_votes, 2);
    }

    #[test]
    fn test_get_proposals_by_state() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("node1".to_string(), 0.6);
        tracker.create_proposal(
            "p1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "node1",
        ).unwrap();
        let drafts = tracker.get_proposals_by_state(&ProposalState::Draft);
        assert_eq!(drafts.len(), 1);
    }

    #[test]
    fn test_proposal_state_display() {
        assert_eq!(ProposalState::Draft.to_string(), "Draft");
        assert_eq!(ProposalState::Voting.to_string(), "Voting");
        assert_eq!(ProposalState::Passed.to_string(), "Passed");
        assert_eq!(ProposalState::Rejected.to_string(), "Rejected");
        assert_eq!(ProposalState::Executed.to_string(), "Executed");
        assert_eq!(ProposalState::Expired.to_string(), "Expired");
    }

    #[test]
    fn test_config_default() {
        let config = ProposalConfig::default();
        assert_eq!(config.default_voting_duration_ms, 86_400_000);
        assert_eq!(config.min_proposer_weight, 0.05);
        assert_eq!(config.quorum_threshold, 0.3);
        assert_eq!(config.approval_threshold, 0.5);
    }

    #[test]
    fn test_stats_default() {
        let stats = ProposalStats::default();
        assert_eq!(stats.total_proposals, 0);
        assert_eq!(stats.active_proposals, 0);
        assert_eq!(stats.total_votes, 0);
    }

    #[test]
    fn test_tracker_default() {
        let tracker = ProposalTracker::default();
        assert_eq!(tracker.get_stats().total_proposals, 0);
    }

    #[test]
    fn test_error_display() {
        match ProposalError::ProposalNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_proposal_vote_count() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("n1".to_string(), 0.6);
        tracker.register_voter("n2".to_string(), 0.4);
        tracker.create_proposal("p1".to_string(), "T".to_string(), "D".to_string(), "n1").unwrap();
        tracker.open_voting("p1").unwrap();
        tracker.cast_vote("p1", "n1", true).unwrap();
        tracker.cast_vote("p1", "n2", false).unwrap();
        assert_eq!(tracker.get_proposal("p1").unwrap().vote_count(), 2);
    }

    #[test]
    fn test_weight_clamping() {
        let mut tracker = ProposalTracker::with_defaults();
        tracker.register_voter("n1".to_string(), 1.5);
        assert_eq!(*tracker.voting_weights.get("n1").unwrap(), 1.0);
    }
}
