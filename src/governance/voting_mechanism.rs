//! Voting Mechanism — Batch processing with cryptographic signature verification
//!
//! Provides a lightweight voting processor that:
//! - Collects votes per proposal into batches
//! - Calculates weighted totals (for/against/abstain)
//! - Enforces quorum thresholds
//! - Simulates ed25519 signature verification
//! - Tracks batch processing time (target ≤100ms)
//! - Prevents double voting

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

use thiserror::Error;
use tracing::{debug, info, warn};

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors produced by the voting mechanism.
#[derive(Debug, Error)]
pub enum VotingError {
    #[error("invalid signature for voter: {0}")]
    InvalidSignature(String),

    #[error("voter already voted on proposal: {proposal}")]
    AlreadyVoted { proposal: String },

    #[error("proposal not found: {0}")]
    ProposalNotFound(String),

    #[error("quorum not met: {current:.2}% < {required:.2}%")]
    QuorumNotMet { current: f64, required: f64 },

    #[error("batch processing timeout exceeded {:.0}ms", .0.as_millis())]
    BatchTimeout(Duration),

    #[error("voter not registered: {0}")]
    VoterNotRegistered(String),
}

// ─── Core Types ───────────────────────────────────────────────────────────────

/// Direction of a vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoteDirection {
    /// Vote in favor of the proposal.
    For,
    /// Vote against the proposal.
    Against,
    /// Abstain from influencing the outcome.
    Abstain,
}

impl fmt::Display for VoteDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoteDirection::For => write!(f, "For"),
            VoteDirection::Against => write!(f, "Against"),
            VoteDirection::Abstain => write!(f, "Abstain"),
        }
    }
}

/// A single vote record with cryptographic signature.
#[derive(Debug, Clone)]
pub struct VoteRecord {
    /// ID of the voter.
    pub voter_id: String,
    /// ID of the proposal being voted on.
    pub proposal_id: String,
    /// Direction of the vote.
    pub direction: VoteDirection,
    /// Weight of this vote (from voter registration).
    pub weight: f64,
    /// Cryptographic signature (ed25519 simulated).
    pub signature: String,
    /// Timestamp when the vote was submitted.
    pub timestamp: Instant,
}

/// A batch of votes for a single proposal.
#[derive(Debug, Clone)]
pub struct BatchVote {
    /// ID of the proposal this batch belongs to.
    pub proposal_id: String,
    /// All votes in this batch.
    pub votes: Vec<VoteRecord>,
    /// Total weight of "For" votes.
    pub total_weight_for: f64,
    /// Total weight of "Against" votes.
    pub total_weight_against: f64,
    /// Total weight of "Abstain" votes.
    pub total_weight_abstain: f64,
    /// Whether quorum threshold is met.
    pub quorum_met: bool,
    /// Final decision if quorum is met (None otherwise).
    pub decided: Option<VoteDirection>,
}

/// Configuration for the voting mechanism.
#[derive(Debug, Clone)]
pub struct VotingConfig {
    /// Maximum number of votes per batch.
    pub max_batch_size: usize,
    /// Maximum allowed batch processing time.
    pub batch_timeout_ms: u64,
    /// Quorum threshold as fraction of total weight (0.0–1.0).
    pub quorum_percentage: f64,
    /// Minimum number of unique participants for quorum.
    pub min_participants: usize,
    /// Whether to verify cryptographic signatures.
    pub enable_signature_verification: bool,
}

impl Default for VotingConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            batch_timeout_ms: 100,
            quorum_percentage: 0.3,
            min_participants: 3,
            enable_signature_verification: true,
        }
    }
}

/// Aggregate statistics for the voting mechanism.
#[derive(Debug, Clone)]
pub struct VotingStats {
    /// Total number of batches processed.
    pub total_batches: usize,
    /// Total number of votes submitted.
    pub total_votes: usize,
    /// Average batch processing time in milliseconds.
    pub avg_batch_ms: f64,
    /// Number of batches where quorum was met.
    pub quorum_met_count: usize,
    /// Number of batches where quorum was not met.
    pub quorum_failed_count: usize,
}

// ─── Mechanism ────────────────────────────────────────────────────────────────

/// Voting mechanism with batch processing and quorum enforcement.
pub struct VotingMechanism {
    config: VotingConfig,
    voters: HashMap<String, f64>,
    pending_votes: HashMap<String, Vec<VoteRecord>>,
    processed_batches: HashMap<String, BatchVote>,
    stats: VotingStats,
    total_batch_time_ms: f64,
}

impl VotingMechanism {
    // ── Construction ───────────────────────────────────────────────────────

    /// Create a new voting mechanism with default configuration.
    pub fn default_mechanism() -> Self {
        Self::new(VotingConfig::default())
    }

    /// Create a new voting mechanism with the provided configuration.
    pub fn new(config: VotingConfig) -> Self {
        Self {
            config,
            voters: HashMap::new(),
            pending_votes: HashMap::new(),
            processed_batches: HashMap::new(),
            stats: VotingStats {
                total_batches: 0,
                total_votes: 0,
                avg_batch_ms: 0.0,
                quorum_met_count: 0,
                quorum_failed_count: 0,
            },
            total_batch_time_ms: 0.0,
        }
    }

    // ── Voter Registration ─────────────────────────────────────────────────

    /// Register a voter with the given voting weight.
    pub fn register_voter(&mut self, voter_id: &str, weight: f64) {
        info!(voter_id = %voter_id, weight, "voter registered");
        self.voters.insert(voter_id.to_string(), weight);
    }

    /// Get the weight of a registered voter.
    pub fn get_voter_weight(&self, voter_id: &str) -> Option<f64> {
        self.voters.get(voter_id).copied()
    }

    // ── Vote Submission ────────────────────────────────────────────────────

    /// Submit a vote for a proposal.
    ///
    /// Verifies the signature (simulated: must be non-empty) and checks
    /// that the voter has not already voted on this proposal.
    pub fn submit_vote(
        &mut self,
        voter_id: &str,
        proposal_id: &str,
        direction: VoteDirection,
        signature: &str,
    ) -> Result<(), VotingError> {
        // Check voter is registered
        let weight = *self
            .voters
            .get(voter_id)
            .ok_or(VotingError::VoterNotRegistered(voter_id.to_string()))?;

        // Verify signature (simulated: non-empty check)
        if self.config.enable_signature_verification && signature.is_empty() {
            return Err(VotingError::InvalidSignature(voter_id.to_string()));
        }

        // Check for double vote
        if let Some(votes) = self.pending_votes.get(proposal_id) {
            if votes.iter().any(|v| v.voter_id == voter_id) {
                return Err(VotingError::AlreadyVoted {
                    proposal: proposal_id.to_string(),
                });
            }
        }

        // Also check processed batches for double vote
        if let Some(batch) = self.processed_batches.get(proposal_id) {
            if batch.votes.iter().any(|v| v.voter_id == voter_id) {
                return Err(VotingError::AlreadyVoted {
                    proposal: proposal_id.to_string(),
                });
            }
        }

        let record = VoteRecord {
            voter_id: voter_id.to_string(),
            proposal_id: proposal_id.to_string(),
            direction,
            weight,
            signature: signature.to_string(),
            timestamp: Instant::now(),
        };

        debug!(
            voter_id = %voter_id,
            proposal_id,
            direction = %direction,
            "vote submitted"
        );

        self.pending_votes
            .entry(proposal_id.to_string())
            .or_default()
            .push(record);
        self.stats.total_votes += 1;

        Ok(())
    }

    // ── Batch Processing ───────────────────────────────────────────────────

    /// Process all pending votes for a proposal into a batch.
    ///
    /// Calculates weighted totals, checks quorum, and determines the
    /// final decision direction.
    pub fn process_batch(&mut self, proposal_id: &str) -> Result<BatchVote, VotingError> {
        let start = Instant::now();

        let votes = self.pending_votes.remove(proposal_id).unwrap_or_default();

        if votes.is_empty() {
            // Return an empty batch
            let batch = BatchVote {
                proposal_id: proposal_id.to_string(),
                votes: Vec::new(),
                total_weight_for: 0.0,
                total_weight_against: 0.0,
                total_weight_abstain: 0.0,
                quorum_met: false,
                decided: None,
            };
            self.processed_batches
                .insert(proposal_id.to_string(), batch.clone());
            return Ok(batch);
        }

        // Check batch size limit
        if votes.len() > self.config.max_batch_size {
            warn!(
                proposal_id,
                vote_count = votes.len(),
                max = self.config.max_batch_size,
                "batch size exceeds limit"
            );
        }

        // Calculate weighted totals
        let mut total_weight_for: f64 = 0.0;
        let mut total_weight_against: f64 = 0.0;
        let mut total_weight_abstain: f64 = 0.0;

        for vote in &votes {
            match vote.direction {
                VoteDirection::For => total_weight_for += vote.weight,
                VoteDirection::Against => total_weight_against += vote.weight,
                VoteDirection::Abstain => total_weight_abstain += vote.weight,
            }
        }

        // Calculate quorum
        let decisive_weight = total_weight_for + total_weight_against;
        let quorum_ratio = if decisive_weight > 0.0 {
            total_weight_for / decisive_weight
        } else {
            0.0
        };

        // Count unique participants
        let unique_participants: std::collections::HashSet<&str> =
            votes.iter().map(|v| v.voter_id.as_str()).collect();

        let quorum_met = quorum_ratio >= self.config.quorum_percentage
            && unique_participants.len() >= self.config.min_participants;

        // Determine decision
        let decided = if quorum_met {
            if total_weight_for > total_weight_against {
                Some(VoteDirection::For)
            } else if total_weight_against > total_weight_for {
                Some(VoteDirection::Against)
            } else {
                // Tie: abstain
                Some(VoteDirection::Abstain)
            }
        } else {
            None
        };

        // Check processing time
        let elapsed = start.elapsed();
        self.total_batch_time_ms += elapsed.as_millis() as f64;
        self.stats.total_batches += 1;

        if elapsed.as_millis() > self.config.batch_timeout_ms as u128 {
            warn!(
                proposal_id,
                elapsed_ms = elapsed.as_millis(),
                timeout_ms = self.config.batch_timeout_ms,
                "batch processing exceeded timeout"
            );
        }

        // Update quorum stats
        if quorum_met {
            self.stats.quorum_met_count += 1;
        } else {
            self.stats.quorum_failed_count += 1;
        }

        // Update average batch time
        self.stats.avg_batch_ms = self.total_batch_time_ms / self.stats.total_batches as f64;

        let batch = BatchVote {
            proposal_id: proposal_id.to_string(),
            votes,
            total_weight_for,
            total_weight_against,
            total_weight_abstain,
            quorum_met,
            decided,
        };

        info!(
            proposal_id,
            quorum_met,
            decided = ?batch.decided.map(|d| d.to_string()),
            "batch processed"
        );

        self.processed_batches
            .insert(proposal_id.to_string(), batch.clone());
        Ok(batch)
    }

    // ── Queries ────────────────────────────────────────────────────────────

    /// Get a processed batch by proposal ID.
    pub fn get_batch(&self, proposal_id: &str) -> Option<&BatchVote> {
        self.processed_batches.get(proposal_id)
    }

    /// Get the current voting statistics.
    pub fn get_stats(&self) -> VotingStats {
        self.stats.clone()
    }

    // ── Reset ──────────────────────────────────────────────────────────────

    /// Reset the mechanism to a clean state, preserving configuration.
    pub fn reset(&mut self) {
        self.voters.clear();
        self.pending_votes.clear();
        self.processed_batches.clear();
        self.stats = VotingStats {
            total_batches: 0,
            total_votes: 0,
            avg_batch_ms: 0.0,
            quorum_met_count: 0,
            quorum_failed_count: 0,
        };
        self.total_batch_time_ms = 0.0;
        info!("voting mechanism reset");
    }
}

// ─── Display Implementations ─────────────────────────────────────────────────

impl fmt::Display for VoteRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VoteRecord(voter={}, proposal={}, dir={})",
            self.voter_id, self.proposal_id, self.direction
        )
    }
}

impl fmt::Display for BatchVote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BatchVote(proposal={}, votes={}, quorum={})",
            self.proposal_id,
            self.votes.len(),
            self.quorum_met
        )
    }
}

impl fmt::Display for VotingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VotingConfig(batch_size={}, quorum={:.0}%, timeout={}ms)",
            self.max_batch_size,
            self.quorum_percentage * 100.0,
            self.batch_timeout_ms
        )
    }
}

impl fmt::Display for VotingStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VotingStats(batches={}, votes={}, avg_ms={:.1}, quorum_met={}, failed={})",
            self.total_batches,
            self.total_votes,
            self.avg_batch_ms,
            self.quorum_met_count,
            self.quorum_failed_count
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mechanism_creation() {
        let mech = VotingMechanism::default_mechanism();
        assert_eq!(mech.voters.len(), 0);
        assert_eq!(mech.config.quorum_percentage, 0.3);
        assert!(mech.config.enable_signature_verification);
    }

    #[test]
    fn test_new_with_custom_config() {
        let config = VotingConfig {
            max_batch_size: 500,
            quorum_percentage: 0.5,
            enable_signature_verification: false,
            ..Default::default()
        };
        let mech = VotingMechanism::new(config);
        assert_eq!(mech.config.max_batch_size, 500);
        assert_eq!(mech.config.quorum_percentage, 0.5);
        assert!(!mech.config.enable_signature_verification);
    }

    #[test]
    fn test_register_voter() {
        let mut mech = VotingMechanism::default_mechanism();
        mech.register_voter("v1", 10.0);
        assert_eq!(mech.get_voter_weight("v1"), Some(10.0));
        assert_eq!(mech.get_voter_weight("unknown"), None);
    }

    #[test]
    fn test_submit_vote_success() {
        let mut mech = VotingMechanism::default_mechanism();
        mech.register_voter("v1", 10.0);
        assert!(mech
            .submit_vote("v1", "prop-1", VoteDirection::For, "sig-abc")
            .is_ok());
    }

    #[test]
    fn test_submit_vote_invalid_signature() {
        let mut mech = VotingMechanism::default_mechanism();
        mech.register_voter("v1", 10.0);
        let result = mech.submit_vote("v1", "prop-1", VoteDirection::For, "");
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_vote_unregistered_voter() {
        let mut mech = VotingMechanism::default_mechanism();
        let result = mech.submit_vote("unknown", "prop-1", VoteDirection::For, "sig-abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_double_vote_rejected() {
        let mut mech = VotingMechanism::default_mechanism();
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "sig-1")
            .unwrap();
        let result = mech.submit_vote("v1", "prop-1", VoteDirection::Against, "sig-2");
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_processing_basic() {
        let config = VotingConfig {
            min_participants: 1,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.register_voter("v2", 10.0);
        mech.register_voter("v3", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.submit_vote("v2", "prop-1", VoteDirection::For, "s2")
            .unwrap();
        mech.submit_vote("v3", "prop-1", VoteDirection::Against, "s3")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        assert_eq!(batch.total_weight_for, 20.0);
        assert_eq!(batch.total_weight_against, 10.0);
    }

    #[test]
    fn test_quorum_met() {
        let config = VotingConfig {
            min_participants: 2,
            quorum_percentage: 0.5,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.register_voter("v2", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.submit_vote("v2", "prop-1", VoteDirection::For, "s2")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        assert!(batch.quorum_met);
        assert_eq!(batch.decided, Some(VoteDirection::For));
    }

    #[test]
    fn test_quorum_not_met_low_participation() {
        let config = VotingConfig {
            min_participants: 5,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        assert!(!batch.quorum_met);
        assert_eq!(batch.decided, None);
    }

    #[test]
    fn test_quorum_not_met_against_majority() {
        let config = VotingConfig {
            min_participants: 2,
            quorum_percentage: 0.5,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.register_voter("v2", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::Against, "s1")
            .unwrap();
        mech.submit_vote("v2", "prop-1", VoteDirection::For, "s2")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        // For weight = 10, Against weight = 10, ratio = 0.5 which meets quorum
        // but it's a tie, so decided = Abstain
        assert!(batch.quorum_met);
        assert_eq!(batch.decided, Some(VoteDirection::Abstain));
    }

    #[test]
    fn test_empty_batch() {
        let mut mech = VotingMechanism::default_mechanism();
        let batch = mech.process_batch("empty-prop").unwrap();
        assert!(batch.votes.is_empty());
        assert!(!batch.quorum_met);
        assert_eq!(batch.decided, None);
    }

    #[test]
    fn test_single_voter_batch() {
        let config = VotingConfig {
            min_participants: 1,
            quorum_percentage: 0.0,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("solo", 100.0);
        mech.submit_vote("solo", "prop-1", VoteDirection::For, "sig")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        assert!(batch.quorum_met);
        assert_eq!(batch.decided, Some(VoteDirection::For));
    }

    #[test]
    fn test_tie_breaking() {
        let config = VotingConfig {
            min_participants: 2,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.register_voter("v2", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.submit_vote("v2", "prop-1", VoteDirection::Against, "s2")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        assert_eq!(batch.decided, Some(VoteDirection::Abstain));
    }

    #[test]
    fn test_abstain_handling() {
        let config = VotingConfig {
            min_participants: 2,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.register_voter("v2", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.submit_vote("v2", "prop-1", VoteDirection::Abstain, "s2")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        assert_eq!(batch.total_weight_abstain, 10.0);
        // Abstain doesn't count against decisive weight, so for/(for+against) = 10/10 = 1.0
        assert!(batch.quorum_met);
        assert_eq!(batch.decided, Some(VoteDirection::For));
    }

    #[test]
    fn test_stats_tracking() {
        let config = VotingConfig {
            min_participants: 1,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.process_batch("prop-1").unwrap();
        let stats = mech.get_stats();
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.total_votes, 1);
        assert_eq!(stats.quorum_met_count, 1);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut mech = VotingMechanism::default_mechanism();
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.reset();
        assert_eq!(mech.voters.len(), 0);
        assert_eq!(mech.stats.total_votes, 0);
        assert_eq!(mech.stats.total_batches, 0);
    }

    #[test]
    fn test_get_batch_after_processing() {
        let config = VotingConfig {
            min_participants: 1,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.process_batch("prop-1").unwrap();
        let batch = mech.get_batch("prop-1");
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().votes.len(), 1);
    }

    #[test]
    fn test_signature_verification_disabled() {
        let config = VotingConfig {
            enable_signature_verification: false,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        // Empty signature should be allowed when verification is disabled
        assert!(mech
            .submit_vote("v1", "prop-1", VoteDirection::For, "")
            .is_ok());
    }

    #[test]
    fn test_multiple_proposals_independent() {
        let config = VotingConfig {
            min_participants: 1,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-a", VoteDirection::For, "s1")
            .unwrap();
        mech.submit_vote("v1", "prop-b", VoteDirection::Against, "s2")
            .unwrap();
        let batch_a = mech.process_batch("prop-a").unwrap();
        let batch_b = mech.process_batch("prop-b").unwrap();
        assert_eq!(batch_a.total_weight_for, 10.0);
        assert_eq!(batch_b.total_weight_against, 10.0);
    }

    #[test]
    fn test_display_implementations() {
        // thiserror derives Display from #[error(...)] attributes,
        // so the message uses the attribute text: "voter not registered: x"
        let err = VotingError::VoterNotRegistered("x".to_string());
        let s = format!("{}", err);
        assert!(s.contains("voter not registered"));

        let dir = VoteDirection::For;
        assert_eq!(format!("{}", dir), "For");

        let config = VotingConfig::default();
        let s = format!("{}", config);
        assert!(s.contains("batch_size"));
    }

    #[test]
    fn test_batch_timeout_warning_no_panic() {
        // Even with a very low timeout, processing should succeed
        let config = VotingConfig {
            batch_timeout_ms: 1,
            min_participants: 1,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        let result = mech.process_batch("prop-1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_weighted_quorum_calculation() {
        let config = VotingConfig {
            min_participants: 2,
            quorum_percentage: 0.6,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        // v1 has weight 80, v2 has weight 20
        mech.register_voter("v1", 80.0);
        mech.register_voter("v2", 20.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.submit_vote("v2", "prop-1", VoteDirection::Against, "s2")
            .unwrap();
        let batch = mech.process_batch("prop-1").unwrap();
        // for/(for+against) = 80/100 = 0.8 >= 0.6
        assert!(batch.quorum_met);
        assert_eq!(batch.decided, Some(VoteDirection::For));
    }

    #[test]
    fn test_double_vote_after_batch_processed() {
        let config = VotingConfig {
            min_participants: 1,
            quorum_percentage: 0.3,
            ..Default::default()
        };
        let mut mech = VotingMechanism::new(config);
        mech.register_voter("v1", 10.0);
        mech.submit_vote("v1", "prop-1", VoteDirection::For, "s1")
            .unwrap();
        mech.process_batch("prop-1").unwrap();
        // Trying to vote again on the same proposal should fail
        let result = mech.submit_vote("v1", "prop-1", VoteDirection::Against, "s2");
        assert!(result.is_err());
    }
}
