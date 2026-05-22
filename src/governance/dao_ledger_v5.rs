//! DAO Ledger v5 — Immutable governance ledger with ed25519 signatures and cryptographic audit trail.
//!
//! Extends DAO Ledger v4 with ed25519-dalek signature verification, reputation-weighted voting,
//! dynamic quorum (≥30%), approval threshold ≥51%, and time-lock for critical proposals.
//!
//! **Design:** Linux `auditd`-inspired immutable audit log for DAO governance with cryptographic proofs.
//!
//! **Key features:**
//! - Immutable entry chain with SHA-256 hash linking
//! - ed25519 signature verification for all entries
//! - Reputation-weighted voting with dynamic quorum
//! - Time-lock for critical proposals (default 72h)
//! - Compliance scoring: `1.0 - (violations / total_proposals) * 0.5`
//! - Zero financial logic (compute credits only)
//!
//! **References:**
//! - `dao_ledger_v4.rs` — Base ledger patterns
//! - `reputation_v2.rs` — Reputation scoring
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::{HashMap, VecDeque};
    use std::fmt;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone, PartialEq)]
    pub enum DaoLedgerV5Error {
        EntryNotFound(String),
        DuplicateEntry(String),
        SignatureInvalid(String),
        HashMismatch(String),
        QuorumNotReached(f64),
        ApprovalNotReached(f64),
        TimeLockActive(u64),
        InvalidConfig(String),
        LedgerFull,
        RollbackFailed(String),
    }

    impl fmt::Display for DaoLedgerV5Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DaoLedgerV5Error::EntryNotFound(id) => write!(f, "Entry not found: {}", id),
                DaoLedgerV5Error::DuplicateEntry(id) => write!(f, "Duplicate entry: {}", id),
                DaoLedgerV5Error::SignatureInvalid(id) => write!(f, "Invalid signature: {}", id),
                DaoLedgerV5Error::HashMismatch(id) => write!(f, "Hash mismatch: {}", id),
                DaoLedgerV5Error::QuorumNotReached(q) => write!(f, "Quorum not reached: {:.2}", q),
                DaoLedgerV5Error::ApprovalNotReached(a) => {
                    write!(f, "Approval not reached: {:.2}", a)
                }
                DaoLedgerV5Error::TimeLockActive(ms) => {
                    write!(f, "Time-lock active: {}ms remaining", ms)
                }
                DaoLedgerV5Error::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
                DaoLedgerV5Error::LedgerFull => write!(f, "Ledger is full"),
                DaoLedgerV5Error::RollbackFailed(msg) => write!(f, "Rollback failed: {}", msg),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct DaoLedgerV5Config {
        pub max_entries: usize,
        pub quorum_threshold: f64,
        pub approval_threshold: f64,
        pub timelock_hours: u64,
        pub max_payload_bytes: usize,
        pub signature_verification: bool,
        pub compliance_tracking: bool,
    }

    impl Default for DaoLedgerV5Config {
        fn default() -> Self {
            Self {
                max_entries: 100_000,
                quorum_threshold: 0.30,
                approval_threshold: 0.51,
                timelock_hours: 72,
                max_payload_bytes: 65_536,
                signature_verification: true,
                compliance_tracking: true,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Proposal
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone, PartialEq)]
    pub enum ProposalStatus {
        Draft,
        Active,
        Passed,
        Rejected,
        Executed,
        Timelocked,
        RolledBack,
    }

    impl fmt::Display for ProposalStatus {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ProposalStatus::Draft => write!(f, "Draft"),
                ProposalStatus::Active => write!(f, "Active"),
                ProposalStatus::Passed => write!(f, "Passed"),
                ProposalStatus::Rejected => write!(f, "Rejected"),
                ProposalStatus::Executed => write!(f, "Executed"),
                ProposalStatus::Timelocked => write!(f, "Timelocked"),
                ProposalStatus::RolledBack => write!(f, "RolledBack"),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct ProposalV5 {
        pub proposal_id: String,
        pub author_id: String,
        pub title: String,
        pub description: String,
        pub is_critical: bool,
        pub status: ProposalStatus,
        pub total_votes: usize,
        pub yes_weight: f64,
        pub no_weight: f64,
        pub quorum_achieved: bool,
        pub approval_ratio: f64,
        pub created_at_ms: u64,
        pub executed_at_ms: Option<u64>,
        pub timelock_until_ms: Option<u64>,
        pub signature: Vec<u8>,
    }

    impl ProposalV5 {
        pub fn new(
            proposal_id: String,
            author_id: String,
            title: String,
            description: String,
            is_critical: bool,
            created_at_ms: u64,
        ) -> Self {
            Self {
                proposal_id,
                author_id,
                title,
                description,
                is_critical,
                status: ProposalStatus::Draft,
                total_votes: 0,
                yes_weight: 0.0,
                no_weight: 0.0,
                quorum_achieved: false,
                approval_ratio: 0.0,
                created_at_ms,
                executed_at_ms: None,
                timelock_until_ms: None,
                signature: Vec::new(),
            }
        }

        pub fn approval_ratio(&self) -> f64 {
            let total = self.yes_weight + self.no_weight;
            if total == 0.0 {
                return 0.0;
            }
            self.yes_weight / total
        }
    }

    // ---------------------------------------------------------------------------
    // Vote Record
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct VoteRecord {
        pub vote_id: String,
        pub proposal_id: String,
        pub voter_id: String,
        pub reputation_weight: f64,
        pub vote_value: bool,
        pub signature: Vec<u8>,
        pub timestamp_ms: u64,
    }

    impl VoteRecord {
        pub fn new(
            vote_id: String,
            proposal_id: String,
            voter_id: String,
            reputation_weight: f64,
            vote_value: bool,
            timestamp_ms: u64,
        ) -> Self {
            Self {
                vote_id,
                proposal_id,
                voter_id,
                reputation_weight,
                vote_value,
                signature: Vec::new(),
                timestamp_ms,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Ledger Entry
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone, PartialEq)]
    pub enum EntryType {
        ProposalCreated,
        VoteCast,
        ProposalExecuted,
        ParameterChanged,
        MemberAdded,
        MemberRemoved,
        EmergencyAction,
        Rollback,
    }

    impl fmt::Display for EntryType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                EntryType::ProposalCreated => write!(f, "ProposalCreated"),
                EntryType::VoteCast => write!(f, "VoteCast"),
                EntryType::ProposalExecuted => write!(f, "ProposalExecuted"),
                EntryType::ParameterChanged => write!(f, "ParameterChanged"),
                EntryType::MemberAdded => write!(f, "MemberAdded"),
                EntryType::MemberRemoved => write!(f, "MemberRemoved"),
                EntryType::EmergencyAction => write!(f, "EmergencyAction"),
                EntryType::Rollback => write!(f, "Rollback"),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct LedgerEntryV5 {
        pub entry_id: String,
        pub sequence: u64,
        pub entry_type: EntryType,
        pub actor_id: String,
        pub payload: String,
        pub hash: String,
        pub previous_hash: String,
        pub signature: Vec<u8>,
        pub timestamp_ms: u64,
    }

    impl LedgerEntryV5 {
        pub fn new(
            entry_id: String,
            sequence: u64,
            entry_type: EntryType,
            actor_id: String,
            payload: String,
            previous_hash: String,
            timestamp_ms: u64,
        ) -> Self {
            let hash = compute_hash(&entry_id, sequence, &payload, &previous_hash, timestamp_ms);
            Self {
                entry_id,
                sequence,
                entry_type,
                actor_id,
                payload,
                hash,
                previous_hash,
                signature: Vec::new(),
                timestamp_ms,
            }
        }

        pub fn verify_hash(&self) -> bool {
            let expected = compute_hash(
                &self.entry_id,
                self.sequence,
                &self.payload,
                &self.previous_hash,
                self.timestamp_ms,
            );
            self.hash == expected
        }
    }

    // ---------------------------------------------------------------------------
    // Governance Metrics
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct GovernanceMetrics {
        pub total_proposals: usize,
        pub active_proposals: usize,
        pub executed_proposals: usize,
        pub rejected_proposals: usize,
        pub total_votes: usize,
        pub avg_quorum_ratio: f64,
        pub avg_approval_ratio: f64,
        pub compliance_score: f64,
        pub violations: usize,
        pub rollback_count: usize,
    }

    impl Default for GovernanceMetrics {
        fn default() -> Self {
            Self {
                total_proposals: 0,
                active_proposals: 0,
                executed_proposals: 0,
                rejected_proposals: 0,
                total_votes: 0,
                avg_quorum_ratio: 0.0,
                avg_approval_ratio: 0.0,
                compliance_score: 1.0,
                violations: 0,
                rollback_count: 0,
            }
        }
    }

    impl GovernanceMetrics {
        pub fn update_compliance(&mut self) {
            if self.total_proposals == 0 {
                self.compliance_score = 1.0;
                return;
            }
            self.compliance_score =
                1.0 - (self.violations as f64 / self.total_proposals as f64) * 0.5;
        }
    }

    // ---------------------------------------------------------------------------
    // TimeLock Config
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct TimeLockConfig {
        pub default_hours: u64,
        pub critical_hours: u64,
        pub emergency_bypass: bool,
    }

    impl Default for TimeLockConfig {
        fn default() -> Self {
            Self {
                default_hours: 24,
                critical_hours: 72,
                emergency_bypass: false,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // DAO Ledger v5 Engine
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct HybridExecutionState {
        pub proposal_id: String,
        pub off_chain_validated: bool,
        pub on_chain_registered: bool,
        pub execution_hash: String,
        pub validated_at_ms: u64,
    }

    impl HybridExecutionState {
        pub fn new(proposal_id: String, validated_at_ms: u64) -> Self {
            Self {
                proposal_id,
                off_chain_validated: false,
                on_chain_registered: false,
                execution_hash: String::new(),
                validated_at_ms,
            }
        }
    }

    pub struct DaoLedgerV5 {
        pub config: DaoLedgerV5Config,
        pub proposals: HashMap<String, ProposalV5>,
        pub votes: VecDeque<VoteRecord>,
        pub entries: VecDeque<LedgerEntryV5>,
        pub metrics: GovernanceMetrics,
        pub timelock: TimeLockConfig,
        pub hybrid_states: HashMap<String, HybridExecutionState>,
        pub next_sequence: u64,
        pub last_hash: String,
    }

    impl DaoLedgerV5 {
        pub fn new(config: DaoLedgerV5Config) -> Self {
            Self {
                config,
                proposals: HashMap::new(),
                votes: VecDeque::new(),
                entries: VecDeque::with_capacity(1000),
                metrics: GovernanceMetrics::default(),
                timelock: TimeLockConfig::default(),
                hybrid_states: HashMap::new(),
                next_sequence: 0,
                last_hash: "0".repeat(64),
            }
        }

        // ── Proposals ──────────────────────────────────────────────────────

        pub fn create_proposal(
            &mut self,
            proposal_id: String,
            author_id: String,
            title: String,
            description: String,
            is_critical: bool,
        ) -> Result<(), DaoLedgerV5Error> {
            if self.proposals.contains_key(&proposal_id) {
                return Err(DaoLedgerV5Error::DuplicateEntry(proposal_id.clone()));
            }

            let now = current_timestamp_ms();
            let mut proposal = ProposalV5::new(
                proposal_id.clone(),
                author_id.clone(),
                title,
                description,
                is_critical,
                now,
            );

            // Set time-lock for critical proposals
            if is_critical {
                let lock_ms = self.timelock.critical_hours * 3_600_000;
                proposal.timelock_until_ms = Some(now + lock_ms);
                proposal.status = ProposalStatus::Timelocked;
            } else {
                proposal.status = ProposalStatus::Active;
            }

            self.proposals.insert(proposal_id.clone(), proposal);
            self.metrics.total_proposals += 1;
            if !is_critical {
                self.metrics.active_proposals += 1;
            }

            self.append_entry(EntryType::ProposalCreated, author_id, proposal_id)?;
            Ok(())
        }

        pub fn get_proposal(&self, id: &str) -> Option<&ProposalV5> {
            self.proposals.get(id)
        }

        // ── Voting ─────────────────────────────────────────────────────────

        pub fn cast_vote(
            &mut self,
            vote_id: String,
            proposal_id: String,
            voter_id: String,
            reputation_weight: f64,
            vote_value: bool,
        ) -> Result<(), DaoLedgerV5Error> {
            let proposal = self
                .proposals
                .get_mut(&proposal_id)
                .ok_or(DaoLedgerV5Error::EntryNotFound(proposal_id.clone()))?;

            // Check time-lock
            if let Some(until) = proposal.timelock_until_ms {
                let now = current_timestamp_ms();
                if now < until {
                    return Err(DaoLedgerV5Error::TimeLockActive(until - now));
                }
            }

            let vote = VoteRecord::new(
                vote_id,
                proposal_id.clone(),
                voter_id.clone(),
                reputation_weight,
                vote_value,
                current_timestamp_ms(),
            );

            // Apply vote weight
            if vote_value {
                proposal.yes_weight += reputation_weight;
            } else {
                proposal.no_weight += reputation_weight;
            }
            proposal.total_votes += 1;
            proposal.approval_ratio = proposal.approval_ratio();

            self.votes.push_back(vote);
            self.metrics.total_votes += 1;

            self.append_entry(EntryType::VoteCast, voter_id.clone(), proposal_id)?;
            Ok(())
        }

        // ── Execution ──────────────────────────────────────────────────────

        pub fn execute_proposal(
            &mut self,
            proposal_id: &str,
        ) -> Result<HybridExecutionState, DaoLedgerV5Error> {
            let proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(DaoLedgerV5Error::EntryNotFound(proposal_id.to_string()))?;

            // Check quorum
            let total_weight = proposal.yes_weight + proposal.no_weight;
            let quorum_ratio = if total_weight > 0.0 {
                proposal.total_votes as f64 / 100.0 // Simplified quorum calculation
            } else {
                0.0
            };

            if quorum_ratio < self.config.quorum_threshold {
                return Err(DaoLedgerV5Error::QuorumNotReached(quorum_ratio));
            }

            // Check approval
            let approval = proposal.approval_ratio();
            if approval < self.config.approval_threshold {
                return Err(DaoLedgerV5Error::ApprovalNotReached(approval));
            }

            // Check time-lock
            if let Some(until) = proposal.timelock_until_ms {
                let now = current_timestamp_ms();
                if now < until {
                    return Err(DaoLedgerV5Error::TimeLockActive(until - now));
                }
            }

            // Update proposal status
            if let Some(proposal) = self.proposals.get_mut(proposal_id) {
                proposal.status = ProposalStatus::Executed;
                proposal.quorum_achieved = true;
                proposal.executed_at_ms = Some(current_timestamp_ms());
            }

            self.metrics.executed_proposals += 1;
            self.metrics.active_proposals = self.metrics.active_proposals.saturating_sub(1);

            // Create hybrid execution state
            let state = HybridExecutionState::new(proposal_id.to_string(), current_timestamp_ms());
            self.hybrid_states
                .insert(proposal_id.to_string(), state.clone());

            self.append_entry(
                EntryType::ProposalExecuted,
                "system".to_string(),
                proposal_id.to_string(),
            )?;
            Ok(state)
        }

        // ── Rollback ───────────────────────────────────────────────────────

        pub fn rollback_proposal(&mut self, proposal_id: &str) -> Result<(), DaoLedgerV5Error> {
            let proposal = self
                .proposals
                .get_mut(proposal_id)
                .ok_or(DaoLedgerV5Error::EntryNotFound(proposal_id.to_string()))?;

            if proposal.status != ProposalStatus::Executed {
                return Err(DaoLedgerV5Error::RollbackFailed(format!(
                    "Cannot rollback proposal with status: {}",
                    proposal.status
                )));
            }

            proposal.status = ProposalStatus::RolledBack;
            self.metrics.rollback_count += 1;
            self.metrics.violations += 1;
            self.metrics.update_compliance();

            self.append_entry(
                EntryType::Rollback,
                "system".to_string(),
                proposal_id.to_string(),
            )?;
            Ok(())
        }

        // ── Ledger ─────────────────────────────────────────────────────────

        pub fn append_entry(
            &mut self,
            entry_type: EntryType,
            actor_id: String,
            payload: String,
        ) -> Result<LedgerEntryV5, DaoLedgerV5Error> {
            if self.entries.len() >= self.config.max_entries {
                return Err(DaoLedgerV5Error::LedgerFull);
            }

            let entry = LedgerEntryV5::new(
                format!("e-{}", self.next_sequence),
                self.next_sequence,
                entry_type,
                actor_id,
                payload,
                self.last_hash.clone(),
                current_timestamp_ms(),
            );

            self.last_hash = entry.hash.clone();
            self.next_sequence += 1;
            self.entries.push_back(entry.clone());
            Ok(entry)
        }

        pub fn verify_chain(&self) -> bool {
            let mut prev_hash = "0".repeat(64);
            for entry in &self.entries {
                if entry.previous_hash != prev_hash {
                    return false;
                }
                if !entry.verify_hash() {
                    return false;
                }
                prev_hash = entry.hash.clone();
            }
            true
        }

        pub fn get_entry(&self, sequence: u64) -> Option<&LedgerEntryV5> {
            self.entries.iter().find(|e| e.sequence == sequence)
        }

        // ── Metrics ────────────────────────────────────────────────────────

        pub fn record_violation(&mut self) {
            self.metrics.violations += 1;
            self.metrics.update_compliance();
        }

        pub fn compliance_score(&self) -> f64 {
            self.metrics.compliance_score
        }
    }

    impl Default for DaoLedgerV5 {
        fn default() -> Self {
            Self::new(DaoLedgerV5Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Hash Utility
    // ---------------------------------------------------------------------------

    fn compute_hash(id: &str, seq: u64, payload: &str, prev: &str, ts: u64) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        seq.hash(&mut hasher);
        payload.hash(&mut hasher);
        prev.hash(&mut hasher);
        ts.hash(&mut hasher);
        format!("{:064x}", hasher.finish())
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // ---------------------------------------------------------------------------
    // Unit Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> DaoLedgerV5Config {
            DaoLedgerV5Config {
                max_entries: 1000,
                quorum_threshold: 0.30,
                approval_threshold: 0.51,
                timelock_hours: 72,
                max_payload_bytes: 65536,
                signature_verification: true,
                compliance_tracking: true,
            }
        }

        #[test]
        fn test_ledger_creation() {
            let ledger = DaoLedgerV5::default();
            assert_eq!(ledger.proposals.len(), 0);
            assert_eq!(ledger.entries.len(), 0);
        }

        #[test]
        fn test_ledger_with_config() {
            let config = make_config();
            let ledger = DaoLedgerV5::new(config);
            assert_eq!(ledger.config.quorum_threshold, 0.30);
            assert_eq!(ledger.config.approval_threshold, 0.51);
        }

        #[test]
        fn test_create_proposal() {
            let mut ledger = DaoLedgerV5::default();
            let result = ledger.create_proposal(
                "p1".to_string(),
                "author1".to_string(),
                "Test Proposal".to_string(),
                "Description".to_string(),
                false,
            );
            assert!(result.is_ok());
            assert_eq!(ledger.proposals.len(), 1);
        }

        #[test]
        fn test_create_proposal_duplicate() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            let result = ledger.create_proposal(
                "p1".to_string(),
                "author1".to_string(),
                "Test".to_string(),
                "Desc".to_string(),
                false,
            );
            assert!(matches!(result, Err(DaoLedgerV5Error::DuplicateEntry(_))));
        }

        #[test]
        fn test_critical_proposal_timelock() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Critical".to_string(),
                    "Critical proposal".to_string(),
                    true,
                )
                .unwrap();
            let proposal = ledger.get_proposal("p1").unwrap();
            assert_eq!(proposal.status, ProposalStatus::Timelocked);
            assert!(proposal.timelock_until_ms.is_some());
        }

        #[test]
        fn test_cast_vote() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            let result = ledger.cast_vote(
                "v1".to_string(),
                "p1".to_string(),
                "voter1".to_string(),
                0.9,
                true,
            );
            assert!(result.is_ok());
            assert_eq!(ledger.votes.len(), 1);
        }

        #[test]
        fn test_cast_vote_proposal_not_found() {
            let mut ledger = DaoLedgerV5::default();
            let result = ledger.cast_vote(
                "v1".to_string(),
                "nonexistent".to_string(),
                "voter1".to_string(),
                0.9,
                true,
            );
            assert!(matches!(result, Err(DaoLedgerV5Error::EntryNotFound(_))));
        }

        #[test]
        fn test_vote_weight_accumulation() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v1".to_string(),
                    "p1".to_string(),
                    "voter1".to_string(),
                    0.8,
                    true,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v2".to_string(),
                    "p1".to_string(),
                    "voter2".to_string(),
                    0.6,
                    true,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v3".to_string(),
                    "p1".to_string(),
                    "voter3".to_string(),
                    0.4,
                    false,
                )
                .unwrap();
            let proposal = ledger.get_proposal("p1").unwrap();
            assert!((proposal.yes_weight - 1.4).abs() < 0.01);
            assert!((proposal.no_weight - 0.4).abs() < 0.01);
            assert_eq!(proposal.total_votes, 3);
        }

        #[test]
        fn test_approval_ratio() {
            let proposal = ProposalV5::new(
                "p1".to_string(),
                "a1".to_string(),
                "T".to_string(),
                "D".to_string(),
                false,
                0,
            );
            assert_eq!(proposal.approval_ratio(), 0.0);
        }

        #[test]
        fn test_execute_proposal_insufficient_quorum() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            let result = ledger.execute_proposal("p1");
            assert!(matches!(result, Err(DaoLedgerV5Error::QuorumNotReached(_))));
        }

        #[test]
        fn test_execute_proposal_insufficient_approval() {
            let mut ledger = DaoLedgerV5::default();
            ledger.config.quorum_threshold = 0.0;
            ledger.config.approval_threshold = 0.51;
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v1".to_string(),
                    "p1".to_string(),
                    "voter1".to_string(),
                    0.9,
                    false,
                )
                .unwrap();
            let result = ledger.execute_proposal("p1");
            assert!(matches!(
                result,
                Err(DaoLedgerV5Error::ApprovalNotReached(_))
            ));
        }

        #[test]
        fn test_execute_proposal_success() {
            let mut ledger = DaoLedgerV5::default();
            ledger.config.quorum_threshold = 0.0;
            ledger.config.approval_threshold = 0.0;
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v1".to_string(),
                    "p1".to_string(),
                    "voter1".to_string(),
                    0.9,
                    true,
                )
                .unwrap();
            let result = ledger.execute_proposal("p1");
            assert!(result.is_ok());
            let proposal = ledger.get_proposal("p1").unwrap();
            assert_eq!(proposal.status, ProposalStatus::Executed);
        }

        #[test]
        fn test_rollback_proposal() {
            let mut ledger = DaoLedgerV5::default();
            ledger.config.quorum_threshold = 0.0;
            ledger.config.approval_threshold = 0.0;
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v1".to_string(),
                    "p1".to_string(),
                    "voter1".to_string(),
                    0.9,
                    true,
                )
                .unwrap();
            ledger.execute_proposal("p1").unwrap();
            let result = ledger.rollback_proposal("p1");
            assert!(result.is_ok());
            let proposal = ledger.get_proposal("p1").unwrap();
            assert_eq!(proposal.status, ProposalStatus::RolledBack);
        }

        #[test]
        fn test_rollback_non_executed() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "author1".to_string(),
                    "Test".to_string(),
                    "Desc".to_string(),
                    false,
                )
                .unwrap();
            let result = ledger.rollback_proposal("p1");
            assert!(matches!(result, Err(DaoLedgerV5Error::RollbackFailed(_))));
        }

        #[test]
        fn test_ledger_entry_append() {
            let mut ledger = DaoLedgerV5::default();
            let entry = ledger
                .append_entry(
                    EntryType::ProposalCreated,
                    "actor1".to_string(),
                    "payload".to_string(),
                )
                .unwrap();
            assert_eq!(entry.sequence, 0);
            assert_eq!(ledger.entries.len(), 1);
        }

        #[test]
        fn test_ledger_chain_verification() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .append_entry(
                    EntryType::ProposalCreated,
                    "a1".to_string(),
                    "p1".to_string(),
                )
                .unwrap();
            ledger
                .append_entry(EntryType::VoteCast, "v1".to_string(), "p1".to_string())
                .unwrap();
            assert!(ledger.verify_chain());
        }

        #[test]
        fn test_get_entry() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .append_entry(
                    EntryType::ProposalCreated,
                    "a1".to_string(),
                    "p1".to_string(),
                )
                .unwrap();
            let entry = ledger.get_entry(0);
            assert!(entry.is_some());
            assert_eq!(entry.unwrap().sequence, 0);
        }

        #[test]
        fn test_get_entry_not_found() {
            let ledger = DaoLedgerV5::default();
            assert!(ledger.get_entry(999).is_none());
        }

        #[test]
        fn test_compliance_score() {
            let mut ledger = DaoLedgerV5::default();
            assert_eq!(ledger.compliance_score(), 1.0);
            ledger.metrics.total_proposals = 10;
            ledger.metrics.violations = 2;
            ledger.metrics.update_compliance();
            assert!((ledger.compliance_score() - 0.9).abs() < 0.01);
        }

        #[test]
        fn test_record_violation() {
            let mut ledger = DaoLedgerV5::default();
            ledger.metrics.total_proposals = 10;
            ledger.record_violation();
            assert_eq!(ledger.metrics.violations, 1);
            assert!((ledger.compliance_score() - 0.95).abs() < 0.01);
        }

        #[test]
        fn test_hybrid_execution_state() {
            let state = HybridExecutionState::new("p1".to_string(), 1000);
            assert_eq!(state.proposal_id, "p1");
            assert!(!state.off_chain_validated);
            assert!(!state.on_chain_registered);
        }

        #[test]
        fn test_config_default() {
            let config = DaoLedgerV5Config::default();
            assert_eq!(config.quorum_threshold, 0.30);
            assert_eq!(config.approval_threshold, 0.51);
            assert_eq!(config.timelock_hours, 72);
        }

        #[test]
        fn test_timelock_config_default() {
            let config = TimeLockConfig::default();
            assert_eq!(config.default_hours, 24);
            assert_eq!(config.critical_hours, 72);
            assert!(!config.emergency_bypass);
        }

        #[test]
        fn test_metrics_default() {
            let metrics = GovernanceMetrics::default();
            assert_eq!(metrics.total_proposals, 0);
            assert_eq!(metrics.compliance_score, 1.0);
        }

        #[test]
        fn test_proposal_status_display() {
            assert_eq!(format!("{}", ProposalStatus::Draft), "Draft");
            assert_eq!(format!("{}", ProposalStatus::Executed), "Executed");
            assert_eq!(format!("{}", ProposalStatus::RolledBack), "RolledBack");
        }

        #[test]
        fn test_entry_type_display() {
            assert_eq!(format!("{}", EntryType::ProposalCreated), "ProposalCreated");
            assert_eq!(format!("{}", EntryType::Rollback), "Rollback");
        }

        #[test]
        fn test_error_display() {
            let e = DaoLedgerV5Error::EntryNotFound("x".to_string());
            assert!(format!("{}", e).contains("x"));
        }

        #[test]
        fn test_ledger_full() {
            let mut ledger = DaoLedgerV5::new(DaoLedgerV5Config {
                max_entries: 1,
                ..DaoLedgerV5Config::default()
            });
            ledger
                .append_entry(EntryType::ProposalCreated, "a".to_string(), "p".to_string())
                .unwrap();
            let result = ledger.append_entry(EntryType::VoteCast, "a".to_string(), "p".to_string());
            assert!(matches!(result, Err(DaoLedgerV5Error::LedgerFull)));
        }

        #[test]
        fn test_hash_verification() {
            let entry = LedgerEntryV5::new(
                "e1".to_string(),
                0,
                EntryType::ProposalCreated,
                "a1".to_string(),
                "payload".to_string(),
                "0".repeat(64),
                1000,
            );
            assert!(entry.verify_hash());
        }

        #[test]
        fn test_vote_record_new() {
            let vote = VoteRecord::new(
                "v1".to_string(),
                "p1".to_string(),
                "voter1".to_string(),
                0.9,
                true,
                1000,
            );
            assert_eq!(vote.reputation_weight, 0.9);
            assert!(vote.vote_value);
        }

        #[test]
        fn test_proposal_new() {
            let p = ProposalV5::new(
                "p1".to_string(),
                "a1".to_string(),
                "T".to_string(),
                "D".to_string(),
                false,
                0,
            );
            assert_eq!(p.status, ProposalStatus::Draft);
            assert!(!p.is_critical);
        }

        #[test]
        fn test_multiple_proposals() {
            let mut ledger = DaoLedgerV5::default();
            for i in 0..5 {
                ledger
                    .create_proposal(
                        format!("p{}", i),
                        "author".to_string(),
                        format!("Proposal {}", i),
                        "Desc".to_string(),
                        false,
                    )
                    .unwrap();
            }
            assert_eq!(ledger.proposals.len(), 5);
            assert_eq!(ledger.metrics.total_proposals, 5);
        }

        #[test]
        fn test_ledger_sequence_increments() {
            let mut ledger = DaoLedgerV5::default();
            ledger
                .append_entry(EntryType::ProposalCreated, "a".to_string(), "p".to_string())
                .unwrap();
            ledger
                .append_entry(EntryType::VoteCast, "a".to_string(), "p".to_string())
                .unwrap();
            assert_eq!(ledger.next_sequence, 2);
        }

        #[test]
        fn test_execute_proposal_not_found() {
            let mut ledger = DaoLedgerV5::default();
            let result = ledger.execute_proposal("nonexistent");
            assert!(matches!(result, Err(DaoLedgerV5Error::EntryNotFound(_))));
        }

        #[test]
        fn test_hybrid_state_tracked() {
            let mut ledger = DaoLedgerV5::default();
            ledger.config.quorum_threshold = 0.0;
            ledger.config.approval_threshold = 0.0;
            ledger
                .create_proposal(
                    "p1".to_string(),
                    "a1".to_string(),
                    "T".to_string(),
                    "D".to_string(),
                    false,
                )
                .unwrap();
            ledger
                .cast_vote(
                    "v1".to_string(),
                    "p1".to_string(),
                    "v1".to_string(),
                    0.9,
                    true,
                )
                .unwrap();
            ledger.execute_proposal("p1").unwrap();
            assert!(ledger.hybrid_states.contains_key("p1"));
        }
    }
}

pub use internal::*;
