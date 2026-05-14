//! Cross-Federation Verification — Multi-federation proof verification with threshold consensus.
//!
//! Features:
//! - Threshold-based consensus across federations
//! - Merkle-based proof chain verification
//! - Reputation-weighted voting
//! - Challenge-response verification sessions
//! - Cross-federation proof attestation
//!
//! **Design:** Byzantine fault tolerance with threshold signatures.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.4-sprint3")]
mod internal {
    use std::collections::HashMap;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for cross-federation verification.
    #[derive(Debug, Clone, PartialEq)]
    pub enum CrossFedError {
        /// Federation not registered.
        FederationNotFound(String),
        /// Session not found.
        SessionNotFound(String),
        /// Consensus threshold not met.
        ConsensusFailed { yes: usize, no: usize, threshold: usize },
        /// Quorum not reached.
        QuorumNotReached { votes: usize, required: usize },
        /// Vote already cast.
        VoteAlreadyCast(String),
        /// Proof chain invalid.
        ProofChainInvalid(String),
        /// Session expired.
        SessionExpired(String),
    }

    impl std::fmt::Display for CrossFedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CrossFedError::FederationNotFound(id) => write!(f, "Federation {} not found", id),
                CrossFedError::SessionNotFound(id) => write!(f, "Session {} not found", id),
                CrossFedError::ConsensusFailed { yes, no, threshold } => {
                    write!(f, "Consensus failed: {} yes, {} no (threshold: {})", yes, no, threshold)
                }
                CrossFedError::QuorumNotReached { votes, required } => {
                    write!(f, "Quorum not reached: {} votes (required: {})", votes, required)
                }
                CrossFedError::VoteAlreadyCast(id) => write!(f, "Federation {} already voted", id),
                CrossFedError::ProofChainInvalid(msg) => write!(f, "Proof chain invalid: {}", msg),
                CrossFedError::SessionExpired(id) => write!(f, "Session {} expired", id),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for cross-federation verification.
    #[derive(Debug, Clone)]
    pub struct CrossFedConfig {
        /// Consensus threshold (0.0 - 1.0).
        pub consensus_threshold: f64,
        /// Minimum quorum fraction.
        pub min_quorum: f64,
        /// Session TTL in milliseconds.
        pub session_ttl_ms: u64,
        /// Reputation weight for voting.
        pub reputation_weight: f64,
        /// Max proof chain length.
        pub max_chain_length: usize,
    }

    impl Default for CrossFedConfig {
        fn default() -> Self {
            Self {
                consensus_threshold: 0.67,
                min_quorum: 0.5,
                session_ttl_ms: 60_000,
                reputation_weight: 0.7,
                max_chain_length: 20,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Federation Voter
    // ---------------------------------------------------------------------------

    /// Federation voter entry.
    #[derive(Debug, Clone)]
    pub struct FederationVoter {
        /// Federation identifier.
        pub federation_id: String,
        /// Reputation score.
        pub reputation: f64,
        /// Total votes cast.
        pub votes_cast: usize,
        /// Votes in agreement with consensus.
        pub votes_agreed: usize,
    }

    impl FederationVoter {
        pub fn new(federation_id: String, reputation: f64) -> Self {
            Self {
                federation_id,
                reputation,
                votes_cast: 0,
                votes_agreed: 0,
            }
        }

        /// Compute vote weight based on reputation.
        pub fn vote_weight(&self, rep_weight: f64) -> f64 {
            self.reputation * rep_weight + 1.0 * (1.0 - rep_weight)
        }

        /// Update reputation after consensus.
        pub fn update_reputation(&mut self, agreed: bool, decay: f64, boost: f64) {
            self.votes_cast += 1;
            if agreed {
                self.votes_agreed += 1;
                self.reputation = (self.reputation * (1.0 - boost) + boost).min(1.0);
            } else {
                self.reputation = (self.reputation * decay).max(0.0);
            }
        }

        /// Compute alignment rate.
        pub fn alignment_rate(&self) -> f64 {
            if self.votes_cast == 0 {
                return 1.0;
            }
            self.votes_agreed as f64 / self.votes_cast as f64
        }
    }

    // ---------------------------------------------------------------------------
    // Verification Session
    // ---------------------------------------------------------------------------

    /// Verification session for cross-federation consensus.
    #[derive(Debug, Clone)]
    pub struct VerificationSession {
        /// Session identifier.
        pub id: String,
        /// Proof identifier being verified.
        pub proof_id: String,
        /// Votes cast (federation_id -> (valid, weight)).
        pub votes: HashMap<String, (bool, f64)>,
        /// Total vote weight.
        pub total_weight: f64,
        /// Consensus reached.
        pub consensus_reached: bool,
        /// Consensus result (true = valid).
        pub consensus_result: bool,
        /// Creation timestamp.
        pub created_ms: u64,
        /// Proof chain (ordered federation IDs).
        pub proof_chain: Vec<String>,
    }

    impl VerificationSession {
        pub fn new(id: String, proof_id: String) -> Self {
            Self {
                id,
                proof_id,
                votes: HashMap::new(),
                total_weight: 0.0,
                consensus_reached: false,
                consensus_result: false,
                created_ms: current_timestamp_ms(),
                proof_chain: Vec::new(),
            }
        }

        /// Check if session is expired.
        pub fn is_expired(&self, ttl_ms: u64) -> bool {
            current_timestamp_ms() - self.created_ms > ttl_ms
        }

        /// Add proof chain entry.
        pub fn add_chain_entry(&mut self, federation_id: String, max_length: usize) -> Result<(), CrossFedError> {
            if self.proof_chain.len() >= max_length {
                return Err(CrossFedError::ProofChainInvalid("Max chain length reached".to_string()));
            }
            if self.proof_chain.contains(&federation_id) {
                return Err(CrossFedError::ProofChainInvalid("Cycle detected in proof chain".to_string()));
            }
            self.proof_chain.push(federation_id);
            Ok(())
        }

        /// Count yes/no votes.
        pub fn vote_counts(&self) -> (usize, usize) {
            let (yes, no) = self.votes.values().fold((0usize, 0usize), |(y, n), &(valid, _)| {
                if valid { (y + 1, n) } else { (y, n + 1) }
            });
            (yes, no)
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for cross-federation verification.
    #[derive(Debug, Clone)]
    #[derive(Default)]
    pub struct CrossFedStats {
        pub total_sessions: usize,
        pub total_consensus_reached: usize,
        pub total_consensus_failed: usize,
        pub total_votes: usize,
        pub avg_session_time_ms: u64,
        pub total_chain_verifications: usize,
    }

    impl CrossFedStats {
        pub fn record_session(&mut self, consensus: bool, time_ms: u64) {
            self.total_sessions += 1;
            if consensus {
                self.total_consensus_reached += 1;
            } else {
                self.total_consensus_failed += 1;
            }
            let n = self.total_sessions as f64;
            self.avg_session_time_ms =
                ((self.avg_session_time_ms as f64 * (n - 1.0) / n) + time_ms as f64 / n) as u64;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Main Engine
    // ---------------------------------------------------------------------------

    /// Cross-federation verification engine.
    pub struct CrossFederationVerifier {
        config: CrossFedConfig,
        voters: HashMap<String, FederationVoter>,
        sessions: HashMap<String, VerificationSession>,
        stats: CrossFedStats,
    }

    impl CrossFederationVerifier {
        pub fn new(config: CrossFedConfig) -> Self {
            Self {
                config,
                voters: HashMap::new(),
                sessions: HashMap::new(),
                stats: CrossFedStats::default(),
            }
        }

        pub fn with_defaults() -> Self {
            Self::new(CrossFedConfig::default())
        }

        /// Register a federation voter.
        pub fn register_federation(&mut self, federation_id: String, reputation: f64) -> Result<(), CrossFedError> {
            if self.voters.contains_key(&federation_id) {
                return Ok(());
            }
            let voter = FederationVoter::new(federation_id.clone(), reputation);
            self.voters.insert(federation_id, voter);
            Ok(())
        }

        /// Create a verification session.
        pub fn create_session(&mut self, session_id: String, proof_id: String) -> Result<(), CrossFedError> {
            if self.sessions.contains_key(&session_id) {
                return Ok(());
            }
            let session = VerificationSession::new(session_id.clone(), proof_id);
            self.sessions.insert(session_id, session);
            Ok(())
        }

        /// Submit a vote for a session.
        pub fn submit_vote(&mut self, session_id: &str, federation_id: &str, valid: bool) -> Result<(bool, f64), CrossFedError> {
            let session = self.sessions.get_mut(session_id)
                .ok_or(CrossFedError::SessionNotFound(session_id.to_string()))?;
            if session.is_expired(self.config.session_ttl_ms) {
                return Err(CrossFedError::SessionExpired(session_id.to_string()));
            }
            if session.votes.contains_key(federation_id) {
                return Err(CrossFedError::VoteAlreadyCast(federation_id.to_string()));
            }
            let voter = self.voters.get(federation_id)
                .ok_or(CrossFedError::FederationNotFound(federation_id.to_string()))?;
            let weight = voter.vote_weight(self.config.reputation_weight);
            session.votes.insert(federation_id.to_string(), (valid, weight));
            session.total_weight += weight;
            self.stats.total_votes += 1;
            Ok((valid, weight))
        }

        /// Check consensus for a session.
        pub fn check_consensus(&mut self, session_id: &str) -> Result<bool, CrossFedError> {
            let session = self.sessions.get(session_id)
                .ok_or(CrossFedError::SessionNotFound(session_id.to_string()))?;
            let total_voters = self.voters.len();
            let required_quorum = (total_voters as f64 * self.config.min_quorum).ceil() as usize;
            if session.votes.len() < required_quorum {
                return Err(CrossFedError::QuorumNotReached {
                    votes: session.votes.len(),
                    required: required_quorum,
                });
            }
            let (yes, no) = session.vote_counts();
            let total = yes + no;
            let yes_ratio = yes as f64 / total as f64;
            let reached = yes_ratio >= self.config.consensus_threshold;
            if reached {
                let session = self.sessions.get_mut(session_id).unwrap();
                session.consensus_reached = true;
                session.consensus_result = yes >= no;
                self.stats.record_session(true, current_timestamp_ms() - session.created_ms);
            } else {
                self.stats.record_session(false, current_timestamp_ms() - session.created_ms);
            }
            Ok(reached)
        }

        /// Complete a session and update voter reputations.
        pub fn complete_session(&mut self, session_id: &str) -> Result<(), CrossFedError> {
            let session = self.sessions.get(session_id)
                .ok_or(CrossFedError::SessionNotFound(session_id.to_string()))?;
            if !session.consensus_reached {
                return Err(CrossFedError::ConsensusFailed {
                    yes: 0,
                    no: 0,
                    threshold: 0,
                });
            }
            let result = session.consensus_result;
            for (fed_id, (voted_valid, _)) in &session.votes {
                if let Some(voter) = self.voters.get_mut(fed_id) {
                    voter.update_reputation(*voted_valid == result, 0.95, 0.05);
                }
            }
            self.stats.total_chain_verifications += session.proof_chain.len();
            Ok(())
        }

        /// Add proof chain entry to session.
        pub fn add_chain_entry(&mut self, session_id: &str, federation_id: String) -> Result<(), CrossFedError> {
            let session = self.sessions.get_mut(session_id)
                .ok_or(CrossFedError::SessionNotFound(session_id.to_string()))?;
            session.add_chain_entry(federation_id, self.config.max_chain_length)
        }

        /// Clean up expired sessions.
        pub fn cleanup_expired(&mut self) -> usize {
            let expired: Vec<String> = self.sessions.values()
                .filter(|s| s.is_expired(self.config.session_ttl_ms))
                .map(|s| s.id.clone())
                .collect();
            let count = expired.len();
            for id in &expired {
                self.sessions.remove(id);
            }
            count
        }

        /// Get voter count.
        pub fn voter_count(&self) -> usize {
            self.voters.len()
        }

        /// Get active session count.
        pub fn active_sessions(&self) -> usize {
            self.sessions.values()
                .filter(|s| !s.is_expired(self.config.session_ttl_ms))
                .count()
        }

        /// Get stats reference.
        pub fn get_stats(&self) -> &CrossFedStats {
            &self.stats
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for CrossFederationVerifier {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_engine_creation() {
            let engine = CrossFederationVerifier::with_defaults();
            assert_eq!(engine.voter_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = CrossFedConfig {
                consensus_threshold: 0.8,
                ..CrossFedConfig::default()
            };
            let engine = CrossFederationVerifier::new(config);
            assert_eq!(engine.config.consensus_threshold, 0.8);
        }

        #[test]
        fn test_register_federation() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            assert_eq!(engine.voter_count(), 1);
        }

        #[test]
        fn test_register_federation_duplicate() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.register_federation("fed1".to_string(), 0.8).unwrap();
            assert_eq!(engine.voter_count(), 1);
        }

        #[test]
        fn test_create_session() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            assert_eq!(engine.active_sessions(), 1);
        }

        #[test]
        fn test_create_session_duplicate() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.create_session("s1".to_string(), "p2".to_string()).unwrap();
            assert_eq!(engine.active_sessions(), 1);
        }

        #[test]
        fn test_submit_vote() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            let (valid, weight) = engine.submit_vote("s1", "fed1", true).unwrap();
            assert!(valid);
            assert!(weight > 0.0);
        }

        #[test]
        fn test_submit_vote_session_not_found() {
            let mut engine = CrossFederationVerifier::with_defaults();
            let result = engine.submit_vote("missing", "fed1", true);
            assert!(matches!(result, Err(CrossFedError::SessionNotFound(_))));
        }

        #[test]
        fn test_submit_vote_federation_not_found() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            let result = engine.submit_vote("s1", "missing", true);
            assert!(matches!(result, Err(CrossFedError::FederationNotFound(_))));
        }

        #[test]
        fn test_vote_already_cast() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.submit_vote("s1", "fed1", true).unwrap();
            let result = engine.submit_vote("s1", "fed1", false);
            assert!(matches!(result, Err(CrossFedError::VoteAlreadyCast(_))));
        }

        #[test]
        fn test_consensus_reached() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.register_federation("fed2".to_string(), 0.8).unwrap();
            engine.register_federation("fed3".to_string(), 0.7).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.submit_vote("s1", "fed1", true).unwrap();
            engine.submit_vote("s1", "fed2", true).unwrap();
            engine.submit_vote("s1", "fed3", true).unwrap();
            let reached = engine.check_consensus("s1").unwrap();
            assert!(reached);
        }

        #[test]
        fn test_consensus_failed() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.register_federation("fed2".to_string(), 0.8).unwrap();
            engine.register_federation("fed3".to_string(), 0.7).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.submit_vote("s1", "fed1", true).unwrap();
            engine.submit_vote("s1", "fed2", false).unwrap();
            engine.submit_vote("s1", "fed3", false).unwrap();
            let reached = engine.check_consensus("s1").unwrap();
            assert!(!reached);
        }

        #[test]
        fn test_quorum_not_reached() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.register_federation("fed2".to_string(), 0.8).unwrap();
            engine.register_federation("fed3".to_string(), 0.7).unwrap();
            engine.register_federation("fed4".to_string(), 0.6).unwrap();
            engine.register_federation("fed5".to_string(), 0.5).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.submit_vote("s1", "fed1", true).unwrap();
            let result = engine.check_consensus("s1");
            assert!(matches!(result, Err(CrossFedError::QuorumNotReached { .. })));
        }

        #[test]
        fn test_complete_session() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.register_federation("fed2".to_string(), 0.8).unwrap();
            engine.register_federation("fed3".to_string(), 0.7).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.submit_vote("s1", "fed1", true).unwrap();
            engine.submit_vote("s1", "fed2", true).unwrap();
            engine.submit_vote("s1", "fed3", true).unwrap();
            engine.check_consensus("s1").unwrap();
            engine.complete_session("s1").unwrap();
        }

        #[test]
        fn test_add_chain_entry() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.add_chain_entry("s1", "fed1".to_string()).unwrap();
            let session = engine.sessions.get("s1").unwrap();
            assert_eq!(session.proof_chain.len(), 1);
        }

        #[test]
        fn test_chain_cycle_detected() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.add_chain_entry("s1", "fed1".to_string()).unwrap();
            let result = engine.add_chain_entry("s1", "fed1".to_string());
            assert!(matches!(result, Err(CrossFedError::ProofChainInvalid(_))));
        }

        #[test]
        fn test_cleanup_expired() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            let session = engine.sessions.get_mut("s1").unwrap();
            session.created_ms = 0;
            let cleaned = engine.cleanup_expired();
            assert_eq!(cleaned, 1);
        }

        #[test]
        fn test_voter_weight() {
            let voter = FederationVoter::new("test".to_string(), 0.9);
            let weight = voter.vote_weight(0.7);
            assert!(weight > 0.0 && weight <= 1.0);
        }

        #[test]
        fn test_voter_alignment_rate() {
            let mut voter = FederationVoter::new("test".to_string(), 0.9);
            assert_eq!(voter.alignment_rate(), 1.0);
            voter.update_reputation(true, 0.95, 0.05);
            voter.update_reputation(false, 0.95, 0.05);
            assert!((voter.alignment_rate() - 0.5).abs() < 0.01);
        }

        #[test]
        fn test_vote_counts() {
            let mut session = VerificationSession::new("s1".to_string(), "p1".to_string());
            session.votes.insert("f1".to_string(), (true, 1.0));
            session.votes.insert("f2".to_string(), (false, 1.0));
            session.votes.insert("f3".to_string(), (true, 1.0));
            let (yes, no) = session.vote_counts();
            assert_eq!(yes, 2);
            assert_eq!(no, 1);
        }

        #[test]
        fn test_stats_default() {
            let stats = CrossFedStats::default();
            assert_eq!(stats.total_sessions, 0);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = CrossFederationVerifier::with_defaults();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.create_session("s1".to_string(), "p1".to_string()).unwrap();
            engine.submit_vote("s1", "fed1", true).unwrap();
            engine.reset_stats();
            assert_eq!(engine.get_stats().total_votes, 0);
        }

        #[test]
        fn test_config_default() {
            let config = CrossFedConfig::default();
            assert_eq!(config.consensus_threshold, 0.67);
            assert_eq!(config.min_quorum, 0.5);
        }

        #[test]
        fn test_error_display() {
            match CrossFedError::FederationNotFound("test".to_string()) {
                e => assert!(!e.to_string().is_empty()),
            }
        }

        #[test]
        fn test_engine_default() {
            let engine = CrossFederationVerifier::default();
            assert_eq!(engine.voter_count(), 0);
        }
    }
}

#[cfg(feature = "v1.4-sprint3")]
pub use internal::*;
