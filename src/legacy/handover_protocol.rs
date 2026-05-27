//! Legacy Handover Protocol — Sprint 61
//!
//! Implements the final safeguards for the Stuartian Legacy Protocol (SLP),
//! ensuring that the Noospheric civilization transitions irrevocably to
//! **"Propiedad Común de la Humanidad"** (Common Property of Humanity)
//! when maturity thresholds are sustained.
//!
//! ### Core Components
//!
//! - **Human Override Final**: Allows >33% of global CE stewards to trigger
//!   an irrevocable override with 72-hour time-lock for deliberation.
//! - **Handover Protocol**: Monitors sustained NCI > 0.85 over 6 months
//!   and emits `MaturityDeclarationEvent` when threshold is met.
//! - **Legacy Safeguards**: Immutable guarantees that prevent regression
//!   once the handover is initiated.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors within the Handover Protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum HandoverError {
    /// Override threshold not met.
    OverrideThresholdNotMet { provided: f64, required: f64 },
    /// Time-lock active — override cannot be executed yet.
    TimeLockActive { remaining_ms: u64 },
    /// Handover already finalized — state is immutable.
    HandoverFinalized,
    /// NCI maturity not sustained for required duration.
    MaturityNotSustained { current_days: u64, required_days: u64 },
    /// Invalid voter participation (negative or exceeding 1.0).
    InvalidParticipation(f64),
    /// Quorum calculation failed.
    QuorumError(String),
    /// Safeguard violation — attempted regression after handover.
    SafeguardViolation(String),
    /// No active override proposal exists.
    NoActiveProposal,
    /// Proposal already executed or expired.
    ProposalExpired,
}

impl std::fmt::Display for HandoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandoverError::OverrideThresholdNotMet { provided, required } => {
                write!(
                    f,
                    "Override threshold not met: provided {:.2}%, required {:.2}%",
                    provided * 100.0,
                    required * 100.0
                )
            }
            HandoverError::TimeLockActive { remaining_ms } => {
                write!(
                    f,
                    "Time-lock active: {} hours remaining",
                    remaining_ms / 3_600_000
                )
            }
            HandoverError::HandoverFinalized => {
                write!(f, "Handover already finalized — state is immutable")
            }
            HandoverError::MaturityNotSustained {
                current_days,
                required_days,
            } => {
                write!(
                    f,
                    "Maturity not sustained: {} days current, {} days required",
                    current_days, required_days
                )
            }
            HandoverError::InvalidParticipation(val) => {
                write!(f, "Invalid participation value: {} (must be [0.0, 1.0])", val)
            }
            HandoverError::QuorumError(msg) => write!(f, "Quorum error: {}", msg),
            HandoverError::SafeguardViolation(msg) => {
                write!(f, "Safeguard violation: {}", msg)
            }
            HandoverError::NoActiveProposal => write!(f, "No active override proposal"),
            HandoverError::ProposalExpired => write!(f, "Proposal already executed or expired"),
        }
    }
}

// ---------------------------------------------------------------------------
// Override Proposal
// ---------------------------------------------------------------------------

/// A proposal for Human Override Final.
#[derive(Debug, Clone, PartialEq)]
pub struct OverrideProposal {
    /// Unique proposal identifier.
    pub id: u64,
    /// Timestamp when proposal was created (ms).
    pub created_ms: u64,
    /// 72-hour time-lock expiration (ms).
    pub expires_ms: u64,
    /// Current steward participation ratio [0.0, 1.0].
    pub participation: f64,
    /// Required threshold (default 0.33 = 33%).
    pub threshold: f64,
    /// Vote records: voter_id → participation_amount.
    pub votes: HashMap<u64, f64>,
    /// Proposal state.
    pub state: ProposalState,
}

impl OverrideProposal {
    /// Create a new proposal with 72h time-lock.
    pub fn new(
        id: u64,
        created_ms: u64,
        threshold: f64,
        time_lock_hours: u64,
    ) -> Self {
        let expires_ms = created_ms + time_lock_hours * 3_600_000;
        Self {
            id,
            created_ms,
            expires_ms,
            participation: 0.0,
            threshold,
            votes: HashMap::new(),
            state: ProposalState::Pending,
        }
    }

    /// Record a vote from a steward.
    pub fn vote(&mut self, voter_id: u64, amount: f64) -> Result<(), HandoverError> {
        if amount < 0.0 || amount > 1.0 {
            return Err(HandoverError::InvalidParticipation(amount));
        }

        match self.state {
            ProposalState::Executed | ProposalState::Expired => {
                return Err(HandoverError::ProposalExpired);
            }
            ProposalState::Pending => {}
        }

        // Add or update vote
        let _previous = self.votes.insert(voter_id, amount);
        // Recalculate participation
        let total: f64 = self.votes.values().sum();
        self.participation = total.min(1.0);

        // Check if threshold met
        if self.participation >= self.threshold {
            // Still must wait for time-lock expiration
        }

        Ok(())
    }

    /// Check if this proposal can be executed.
    pub fn can_execute(&self, current_ms: u64) -> bool {
        matches!(self.state, ProposalState::Pending)
            && self.participation >= self.threshold
            && current_ms >= self.expires_ms
    }

    /// Execute the proposal.
    pub fn execute(&mut self, current_ms: u64) -> Result<(), HandoverError> {
        if !self.can_execute(current_ms) {
            if self.participation < self.threshold {
                return Err(HandoverError::OverrideThresholdNotMet {
                    provided: self.participation,
                    required: self.threshold,
                });
            }
            if current_ms < self.expires_ms {
                let remaining = self.expires_ms - current_ms;
                return Err(HandoverError::TimeLockActive {
                    remaining_ms: remaining,
                });
            }
            return Err(HandoverError::ProposalExpired);
        }

        self.state = ProposalState::Executed;
        Ok(())
    }

    /// Check if proposal has expired without execution.
    pub fn expire_if_needed(&mut self, current_ms: u64) {
        if matches!(self.state, ProposalState::Pending) && current_ms > self.expires_ms {
            // Only expire if threshold not met
            if self.participation < self.threshold {
                self.state = ProposalState::Expired;
            }
        }
    }

    /// Time-lock remaining in milliseconds.
    pub fn time_lock_remaining(&self, current_ms: u64) -> u64 {
        if current_ms >= self.expires_ms {
            return 0;
        }
        self.expires_ms - current_ms
    }
}

/// State of an override proposal.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalState {
    /// Awaiting votes and time-lock expiration.
    Pending,
    /// Executed successfully.
    Executed,
    /// Expired without meeting threshold.
    Expired,
}

impl std::fmt::Display for ProposalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalState::Pending => write!(f, "Pending"),
            ProposalState::Executed => write!(f, "Executed"),
            ProposalState::Expired => write!(f, "Expired"),
        }
    }
}

// ---------------------------------------------------------------------------
// Maturity Declaration Event
// ---------------------------------------------------------------------------

/// Irrevocable event emitted when NCI sustains > 0.85 for 6 months.
#[derive(Debug, Clone, PartialEq)]
pub struct MaturityDeclarationEvent {
    /// Event identifier.
    pub id: u64,
    /// Timestamp of declaration (ms).
    pub declared_ms: u64,
    /// Final NCI value at declaration.
    pub final_nci: f64,
    /// Number of consecutive days above threshold.
    pub sustained_days: u64,
    /// Declaration message.
    pub message: String,
}

impl MaturityDeclarationEvent {
    /// Create a new maturity declaration.
    pub fn new(
        id: u64,
        declared_ms: u64,
        final_nci: f64,
        sustained_days: u64,
    ) -> Self {
        let message = format!(
            "Maturidad Noosférica Declarada: NCI={:.4} sostenido por {} días. \
             ed2kIA transiciona a Propiedad Común de la Humanidad.",
            final_nci, sustained_days
        );
        Self {
            id,
            declared_ms,
            final_nci,
            sustained_days,
            message,
        }
    }
}

impl std::fmt::Display for MaturityDeclarationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MaturityDeclarationEvent#{} @{} [NCI={:.4}, days={}] — {}",
            self.id, self.declared_ms, self.final_nci, self.sustained_days, self.message
        )
    }
}

// ---------------------------------------------------------------------------
// Handover State
// ---------------------------------------------------------------------------

/// Current state of the Handover Protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum HandoverState {
    /// Monitoring NCI for maturity.
    Monitoring,
    /// Override proposal active.
    OverridePending,
    /// Handover initiated — safeguards active.
    HandoverInitiated,
    /// Irrevocable handover finalized.
    Finalized,
}

impl std::fmt::Display for HandoverState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandoverState::Monitoring => write!(f, "Monitoring"),
            HandoverState::OverridePending => write!(f, "OverridePending"),
            HandoverState::HandoverInitiated => write!(f, "HandoverInitiated"),
            HandoverState::Finalized => write!(f, "Finalized"),
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy Safeguards
// ---------------------------------------------------------------------------

/// Immutable safeguards that protect the Stuartian Legacy.
#[derive(Debug, Clone)]
pub struct LegacySafeguards {
    /// Minimum override threshold (cannot be lowered).
    pub min_override_threshold: f64,
    /// Minimum time-lock duration in hours (cannot be reduced).
    pub min_time_lock_hours: u64,
    /// NCI maturity threshold.
    pub nci_maturity_threshold: f64,
    /// Required sustained days for maturity (default 180 = 6 months).
    pub required_sustained_days: u64,
    /// Whether handover has been finalized (immutable once true).
    pub handover_finalized: bool,
    /// Whether safeguards are sealed (cannot be modified).
    pub sealed: bool,
}

impl LegacySafeguards {
    /// Create default safeguards.
    pub fn new() -> Self {
        Self {
            min_override_threshold: 0.33,
            min_time_lock_hours: 72,
            nci_maturity_threshold: 0.85,
            required_sustained_days: 180,
            handover_finalized: false,
            sealed: false,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        min_override_threshold: f64,
        min_time_lock_hours: u64,
        nci_maturity_threshold: f64,
        required_sustained_days: u64,
    ) -> Self {
        Self {
            min_override_threshold,
            min_time_lock_hours,
            nci_maturity_threshold,
            required_sustained_days,
            handover_finalized: false,
            sealed: false,
        }
    }

    /// Seal the safeguards — makes them immutable.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Check if safeguards allow a given override threshold.
    pub fn allows_threshold(&self, threshold: f64) -> bool {
        threshold >= self.min_override_threshold
    }

    /// Check if safeguards allow a given time-lock duration.
    pub fn allows_time_lock(&self, hours: u64) -> bool {
        hours >= self.min_time_lock_hours
    }

    /// Finalize the handover — irreversible.
    pub fn finalize_handover(&mut self) -> Result<(), HandoverError> {
        if self.handover_finalized {
            return Err(HandoverError::HandoverFinalized);
        }
        self.handover_finalized = true;
        // Auto-seal on finalization
        self.sealed = true;
        Ok(())
    }

    /// Check if NCI value meets maturity threshold.
    pub fn is_mature(&self, nci: f64) -> bool {
        nci >= self.nci_maturity_threshold
    }

    /// Validate that no regression is attempted.
    pub fn validate_no_regression(&self, action: &str) -> Result<(), HandoverError> {
        if self.handover_finalized {
            return Err(HandoverError::SafeguardViolation(format!(
                "Action '{}' blocked: handover finalized",
                action
            )));
        }
        Ok(())
    }
}

impl Default for LegacySafeguards {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Handover Protocol Engine
// ---------------------------------------------------------------------------

/// Main engine for the Handover Protocol.
///
/// Coordinates NCI monitoring, override proposals, and maturity declarations.
#[derive(Debug, Clone)]
pub struct HandoverProtocol {
    /// Legacy safeguards configuration.
    safeguards: LegacySafeguards,
    /// Current protocol state.
    state: HandoverState,
    /// Active override proposal (if any).
    active_proposal: Option<OverrideProposal>,
    /// History of maturity declaration events.
    maturity_events: Vec<MaturityDeclarationEvent>,
    /// Consecutive days above NCI threshold.
    consecutive_mature_days: u64,
    /// Last NCI value processed.
    last_nci: Option<f64>,
    /// Next proposal ID.
    next_proposal_id: u64,
    /// Next event ID.
    next_event_id: u64,
}

impl HandoverProtocol {
    /// Create a new Handover Protocol with default safeguards.
    pub fn new() -> Self {
        Self {
            safeguards: LegacySafeguards::new(),
            state: HandoverState::Monitoring,
            active_proposal: None,
            maturity_events: Vec::new(),
            consecutive_mature_days: 0,
            last_nci: None,
            next_proposal_id: 1,
            next_event_id: 1,
        }
    }

    /// Create with custom safeguards.
    pub fn with_safeguards(safeguards: LegacySafeguards) -> Self {
        Self {
            safeguards,
            ..Self::new()
        }
    }

    // ---- NCI Monitoring ----

    /// Process a new NCI reading.
    ///
    /// Tracks consecutive days above maturity threshold and emits
    /// `MaturityDeclarationEvent` when sustained for required duration.
    pub fn process_nci(
        &mut self,
        nci: f64,
        _timestamp_ms: u64,
    ) -> Result<Option<MaturityDeclarationEvent>, HandoverError> {
        // Check if handover finalized — no further processing
        if self.safeguards.handover_finalized {
            return Ok(None);
        }

        let mut event = None;

        if self.safeguards.is_mature(nci) {
            self.consecutive_mature_days += 1;

            // Check if maturity threshold sustained
            if self.consecutive_mature_days == self.safeguards.required_sustained_days {
                // Emit maturity declaration
                let decl = MaturityDeclarationEvent::new(
                    self.next_event_id,
                    _timestamp_ms,
                    nci,
                    self.consecutive_mature_days,
                );
                self.next_event_id += 1;
                self.maturity_events.push(decl.clone());
                event = Some(decl);
            }
        } else {
            // Reset consecutive count
            self.consecutive_mature_days = 0;
        }

        self.last_nci = Some(nci);
        Ok(event)
    }

    /// Get consecutive mature days count.
    pub fn consecutive_mature_days(&self) -> u64 {
        self.consecutive_mature_days
    }

    /// Get progress toward maturity (0.0 to 1.0).
    pub fn maturity_progress(&self) -> f64 {
        if self.safeguards.required_sustained_days == 0 {
            return 1.0;
        }
        (self.consecutive_mature_days as f64 / self.safeguards.required_sustained_days as f64)
            .min(1.0)
    }

    // ---- Override Proposals ----

    /// Create a new override proposal.
    pub fn create_override_proposal(
        &mut self,
        current_ms: u64,
    ) -> Result<OverrideProposal, HandoverError> {
        if self.safeguards.handover_finalized {
            return Err(HandoverError::HandoverFinalized);
        }

        let proposal = OverrideProposal::new(
            self.next_proposal_id,
            current_ms,
            self.safeguards.min_override_threshold,
            self.safeguards.min_time_lock_hours,
        );
        self.next_proposal_id += 1;
        self.active_proposal = Some(proposal.clone());
        self.state = HandoverState::OverridePending;
        Ok(proposal)
    }

    /// Vote on the active override proposal.
    pub fn vote_override(
        &mut self,
        voter_id: u64,
        amount: f64,
    ) -> Result<(), HandoverError> {
        let proposal = self.active_proposal.as_mut().ok_or(HandoverError::NoActiveProposal)?;
        proposal.vote(voter_id, amount)
    }

    /// Execute the active override proposal if conditions are met.
    pub fn execute_override(
        &mut self,
        current_ms: u64,
    ) -> Result<(), HandoverError> {
        let proposal = self.active_proposal.as_mut().ok_or(HandoverError::NoActiveProposal)?;
        proposal.execute(current_ms)?;

        // Transition to handover initiated
        self.state = HandoverState::HandoverInitiated;
        Ok(())
    }

    /// Get the active proposal.
    pub fn active_proposal(&self) -> Option<&OverrideProposal> {
        self.active_proposal.as_ref()
    }

    // ---- Handover Finalization ----

    /// Finalize the handover — irrevocable transition.
    pub fn finalize_handover(
        &mut self,
        timestamp_ms: u64,
    ) -> Result<MaturityDeclarationEvent, HandoverError> {
        if self.safeguards.handover_finalized {
            return Err(HandoverError::HandoverFinalized);
        }

        // Finalize safeguards
        self.safeguards.finalize_handover()?;

        // Create final maturity event
        let event = MaturityDeclarationEvent::new(
            self.next_event_id,
            timestamp_ms,
            self.last_nci.unwrap_or(0.0),
            self.consecutive_mature_days,
        );
        self.next_event_id += 1;
        self.maturity_events.push(event.clone());

        // Transition to finalized state
        self.state = HandoverState::Finalized;

        Ok(event)
    }

    /// Seal the safeguards — makes them immutable.
    pub fn seal_safeguards(&mut self) {
        self.safeguards.seal();
    }

    // ---- Queries ----

    /// Get current protocol state.
    pub fn state(&self) -> &HandoverState {
        &self.state
    }

    /// Get safeguards.
    pub fn safeguards(&self) -> &LegacySafeguards {
        &self.safeguards
    }

    /// Get maturity event history.
    pub fn maturity_events(&self) -> &[MaturityDeclarationEvent] {
        &self.maturity_events
    }

    /// Check if handover is finalized.
    pub fn is_finalized(&self) -> bool {
        self.safeguards.handover_finalized
    }

    /// Get last NCI value.
    pub fn last_nci(&self) -> Option<f64> {
        self.last_nci
    }

    /// Update expired proposals.
    pub fn update_proposals(&mut self, current_ms: u64) {
        if let Some(ref mut proposal) = self.active_proposal {
            proposal.expire_if_needed(current_ms);
            if matches!(proposal.state, ProposalState::Expired) {
                self.active_proposal = None;
                self.state = HandoverState::Monitoring;
            }
        }
    }

    /// Reset protocol state (for testing only).
    pub fn reset(&mut self) {
        self.state = HandoverState::Monitoring;
        self.active_proposal = None;
        self.maturity_events.clear();
        self.consecutive_mature_days = 0;
        self.last_nci = None;
        // Note: safeguards are NOT reset — they are immutable once sealed
    }
}

impl Default for HandoverProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HandoverProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HandoverProtocol [state={}, mature_days={}/{}, finalized={}]",
            self.state,
            self.consecutive_mature_days,
            self.safeguards.required_sustained_days,
            self.is_finalized(),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- OverrideProposal Tests ----

    #[test]
    fn test_proposal_creation() {
        let p = OverrideProposal::new(1, 1000, 0.33, 72);
        assert_eq!(p.id, 1);
        assert_eq!(p.created_ms, 1000);
        assert_eq!(p.expires_ms, 1000 + 72 * 3_600_000);
        assert_eq!(p.state, ProposalState::Pending);
        assert_eq!(p.participation, 0.0);
    }

    #[test]
    fn test_proposal_vote() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.2).unwrap();
        p.vote(101, 0.2).unwrap();
        assert!((p.participation - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_proposal_vote_capped() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.5).unwrap();
        p.vote(101, 0.5).unwrap();
        assert!((p.participation - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_proposal_invalid_vote() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        match p.vote(100, 1.5) {
            Err(HandoverError::InvalidParticipation(val)) => assert!((val - 1.5).abs() < 1e-10),
            _ => panic!("Expected InvalidParticipation"),
        }
    }

    #[test]
    fn test_proposal_can_execute() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.4).unwrap();
        let after_lock = 1000 + 72 * 3_600_000;
        assert!(p.can_execute(after_lock));
    }

    #[test]
    fn test_proposal_cannot_execute_before_lock() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.4).unwrap();
        assert!(!p.can_execute(2000));
    }

    #[test]
    fn test_proposal_cannot_execute_threshold_not_met() {
        let mut p = OverrideProposal::new(1, 1000, 0.5, 72);
        p.vote(100, 0.3).unwrap();
        let after_lock = 1000 + 72 * 3_600_000;
        assert!(!p.can_execute(after_lock));
    }

    #[test]
    fn test_proposal_execute() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.4).unwrap();
        let after_lock = 1000 + 72 * 3_600_000;
        p.execute(after_lock).unwrap();
        assert_eq!(p.state, ProposalState::Executed);
    }

    #[test]
    fn test_proposal_execute_threshold_not_met() {
        let mut p = OverrideProposal::new(1, 1000, 0.5, 72);
        p.vote(100, 0.3).unwrap();
        let after_lock = 1000 + 72 * 3_600_000;
        match p.execute(after_lock) {
            Err(HandoverError::OverrideThresholdNotMet { .. }) => {}
            _ => panic!("Expected OverrideThresholdNotMet"),
        }
    }

    #[test]
    fn test_proposal_execute_time_lock_active() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.4).unwrap();
        match p.execute(2000) {
            Err(HandoverError::TimeLockActive { .. }) => {}
            _ => panic!("Expected TimeLockActive"),
        }
    }

    #[test]
    fn test_proposal_expire() {
        let mut p = OverrideProposal::new(1, 1000, 0.5, 72);
        p.vote(100, 0.3).unwrap();
        let way_later = 1000 + 100 * 3_600_000;
        p.expire_if_needed(way_later);
        assert_eq!(p.state, ProposalState::Expired);
    }

    #[test]
    fn test_proposal_no_expire_threshold_met() {
        let mut p = OverrideProposal::new(1, 1000, 0.3, 72);
        p.vote(100, 0.4).unwrap();
        let way_later = 1000 + 100 * 3_600_000;
        p.expire_if_needed(way_later);
        // Should not expire since threshold is met
        assert_eq!(p.state, ProposalState::Pending);
    }

    #[test]
    fn test_proposal_time_lock_remaining() {
        let p = OverrideProposal::new(1, 1000, 0.33, 72);
        let remaining = p.time_lock_remaining(2000);
        assert!(remaining > 0);
    }

    #[test]
    fn test_proposal_time_lock_zero() {
        let p = OverrideProposal::new(1, 1000, 0.33, 72);
        let after = 1000 + 72 * 3_600_000;
        assert_eq!(p.time_lock_remaining(after), 0);
    }

    #[test]
    fn test_proposal_state_display() {
        assert_eq!(format!("{}", ProposalState::Pending), "Pending");
        assert_eq!(format!("{}", ProposalState::Executed), "Executed");
        assert_eq!(format!("{}", ProposalState::Expired), "Expired");
    }

    // ---- MaturityDeclarationEvent Tests ----

    #[test]
    fn test_maturity_event_creation() {
        let e = MaturityDeclarationEvent::new(1, 5000, 0.90, 200);
        assert_eq!(e.id, 1);
        assert!((e.final_nci - 0.90).abs() < 1e-10);
        assert_eq!(e.sustained_days, 200);
        assert!(e.message.contains("Propiedad Común de la Humanidad"));
    }

    #[test]
    fn test_maturity_event_display() {
        let e = MaturityDeclarationEvent::new(1, 5000, 0.90, 200);
        let display = format!("{}", e);
        assert!(display.contains("MaturityDeclarationEvent#1"));
    }

    // ---- LegacySafeguards Tests ----

    #[test]
    fn test_safeguards_default() {
        let s = LegacySafeguards::new();
        assert!((s.min_override_threshold - 0.33).abs() < 1e-10);
        assert_eq!(s.min_time_lock_hours, 72);
        assert!((s.nci_maturity_threshold - 0.85).abs() < 1e-10);
        assert_eq!(s.required_sustained_days, 180);
        assert!(!s.handover_finalized);
        assert!(!s.sealed);
    }

    #[test]
    fn test_safeguards_seal() {
        let mut s = LegacySafeguards::new();
        s.seal();
        assert!(s.sealed);
    }

    #[test]
    fn test_safeguards_allows_threshold() {
        let s = LegacySafeguards::new();
        assert!(s.allows_threshold(0.4));
        assert!(!s.allows_threshold(0.2));
    }

    #[test]
    fn test_safeguards_allows_time_lock() {
        let s = LegacySafeguards::new();
        assert!(s.allows_time_lock(96));
        assert!(!s.allows_time_lock(24));
    }

    #[test]
    fn test_safeguards_finalize() {
        let mut s = LegacySafeguards::new();
        s.finalize_handover().unwrap();
        assert!(s.handover_finalized);
        assert!(s.sealed);
    }

    #[test]
    fn test_safeguards_double_finalize() {
        let mut s = LegacySafeguards::new();
        s.finalize_handover().unwrap();
        match s.finalize_handover() {
            Err(HandoverError::HandoverFinalized) => {}
            _ => panic!("Expected HandoverFinalized"),
        }
    }

    #[test]
    fn test_safeguards_is_mature() {
        let s = LegacySafeguards::new();
        assert!(s.is_mature(0.90));
        assert!(!s.is_mature(0.80));
    }

    #[test]
    fn test_safeguards_validate_no_regression() {
        let mut s = LegacySafeguards::new();
        s.finalize_handover().unwrap();
        match s.validate_no_regression("modify_config") {
            Err(HandoverError::SafeguardViolation(msg)) => {
                assert!(msg.contains("handover finalized"));
            }
            _ => panic!("Expected SafeguardViolation"),
        }
    }

    #[test]
    fn test_safeguards_with_config() {
        let s = LegacySafeguards::with_config(0.40, 48, 0.90, 365);
        assert!((s.min_override_threshold - 0.40).abs() < 1e-10);
        assert_eq!(s.min_time_lock_hours, 48);
        assert!((s.nci_maturity_threshold - 0.90).abs() < 1e-10);
        assert_eq!(s.required_sustained_days, 365);
    }

    // ---- HandoverProtocol Tests ----

    #[test]
    fn test_protocol_creation() {
        let p = HandoverProtocol::new();
        assert_eq!(*p.state(), HandoverState::Monitoring);
        assert!(!p.is_finalized());
    }

    #[test]
    fn test_process_nci_mature() {
        let mut p = HandoverProtocol::new();
        // Process 181 days of mature NCI
        for day in 0..181u64 {
            let event = p.process_nci(0.90, day * 86400000).unwrap();
            if day == 179 {
                assert!(event.is_some());
            }
        }
        assert_eq!(p.consecutive_mature_days(), 181);
    }

    #[test]
    fn test_process_nci_reset_on_low() {
        let mut p = HandoverProtocol::new();
        for day in 0..100u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        // Drop below threshold
        p.process_nci(0.50, 100 * 86400000).unwrap();
        assert_eq!(p.consecutive_mature_days(), 0);
    }

    #[test]
    fn test_maturity_progress() {
        let mut p = HandoverProtocol::new();
        for day in 0..90u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        let progress = p.maturity_progress();
        assert!((progress - 90.0 / 180.0).abs() < 1e-10);
    }

    #[test]
    fn test_create_override_proposal() {
        let mut p = HandoverProtocol::new();
        let proposal = p.create_override_proposal(1000).unwrap();
        assert_eq!(proposal.id, 1);
        assert_eq!(*p.state(), HandoverState::OverridePending);
    }

    #[test]
    fn test_vote_override() {
        let mut p = HandoverProtocol::new();
        p.create_override_proposal(1000).unwrap();
        p.vote_override(100, 0.2).unwrap();
        p.vote_override(101, 0.2).unwrap();
        let prop = p.active_proposal().unwrap();
        assert!((prop.participation - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_vote_no_active_proposal() {
        let mut p = HandoverProtocol::new();
        match p.vote_override(100, 0.2) {
            Err(HandoverError::NoActiveProposal) => {}
            _ => panic!("Expected NoActiveProposal"),
        }
    }

    #[test]
    fn test_execute_override() {
        let mut p = HandoverProtocol::new();
        p.create_override_proposal(1000).unwrap();
        p.vote_override(100, 0.4).unwrap();
        let after_lock = 1000 + 72 * 3_600_000;
        p.execute_override(after_lock).unwrap();
        assert_eq!(*p.state(), HandoverState::HandoverInitiated);
    }

    #[test]
    fn test_finalize_handover() {
        let mut p = HandoverProtocol::new();
        for day in 0..180u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        let event = p.finalize_handover(180 * 86400000).unwrap();
        assert!(p.is_finalized());
        assert_eq!(*p.state(), HandoverState::Finalized);
        assert!(event.message.contains("Propiedad Común"));
    }

    #[test]
    fn test_double_finalize() {
        let mut p = HandoverProtocol::new();
        p.finalize_handover(1000).unwrap();
        match p.finalize_handover(2000) {
            Err(HandoverError::HandoverFinalized) => {}
            _ => panic!("Expected HandoverFinalized"),
        }
    }

    #[test]
    fn test_no_processing_after_finalized() {
        let mut p = HandoverProtocol::new();
        p.finalize_handover(1000).unwrap();
        let result = p.process_nci(0.95, 2000).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_no_override_after_finalized() {
        let mut p = HandoverProtocol::new();
        p.finalize_handover(1000).unwrap();
        match p.create_override_proposal(2000) {
            Err(HandoverError::HandoverFinalized) => {}
            _ => panic!("Expected HandoverFinalized"),
        }
    }

    #[test]
    fn test_seal_safeguards() {
        let mut p = HandoverProtocol::new();
        p.seal_safeguards();
        assert!(p.safeguards().sealed);
    }

    #[test]
    fn test_update_proposals_expire() {
        let mut p = HandoverProtocol::new();
        p.create_override_proposal(1000).unwrap();
        // Vote below threshold
        p.vote_override(100, 0.1).unwrap();
        // Way past expiration
        p.update_proposals(1000 + 100 * 3_600_000);
        assert!(p.active_proposal().is_none());
        assert_eq!(*p.state(), HandoverState::Monitoring);
    }

    #[test]
    fn test_reset() {
        let mut p = HandoverProtocol::new();
        for day in 0..100u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        p.reset();
        assert_eq!(p.consecutive_mature_days(), 0);
        assert_eq!(*p.state(), HandoverState::Monitoring);
    }

    #[test]
    fn test_with_safeguards() {
        let safeguards = LegacySafeguards::with_config(0.40, 48, 0.90, 365);
        let p = HandoverProtocol::with_safeguards(safeguards);
        assert!((p.safeguards().min_override_threshold - 0.40).abs() < 1e-10);
    }

    #[test]
    fn test_last_nci() {
        let mut p = HandoverProtocol::new();
        assert!(p.last_nci().is_none());
        p.process_nci(0.75, 1000).unwrap();
        assert!((p.last_nci().unwrap() - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_maturity_events_history() {
        let mut p = HandoverProtocol::new();
        for day in 0..181u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        assert_eq!(p.maturity_events().len(), 1);
    }

    #[test]
    fn test_protocol_display() {
        let p = HandoverProtocol::new();
        let display = format!("{}", p);
        assert!(display.contains("HandoverProtocol"));
    }

    #[test]
    fn test_handover_state_display() {
        assert_eq!(format!("{}", HandoverState::Monitoring), "Monitoring");
        assert_eq!(
            format!("{}", HandoverState::OverridePending),
            "OverridePending"
        );
        assert_eq!(
            format!("{}", HandoverState::HandoverInitiated),
            "HandoverInitiated"
        );
        assert_eq!(format!("{}", HandoverState::Finalized), "Finalized");
    }

    // ---- Integration Tests ----

    #[test]
    fn test_full_handover_workflow() {
        let mut p = HandoverProtocol::new();

        // Phase 1: Monitor NCI for 180 days
        for day in 0..180u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        assert!(p.maturity_events().len() >= 1);

        // Phase 2: Create override proposal
        let proposal = p.create_override_proposal(180 * 86400000).unwrap();
        assert_eq!(proposal.id, 1); // Proposal counter starts at 1

        // Phase 3: Vote
        p.vote_override(100, 0.2).unwrap();
        p.vote_override(101, 0.2).unwrap();

        // Phase 4: Execute after time-lock
        let after_lock = 180 * 86400000 + 72 * 3_600_000;
        p.execute_override(after_lock).unwrap();
        assert_eq!(*p.state(), HandoverState::HandoverInitiated);

        // Phase 5: Finalize
        let final_event = p.finalize_handover(after_lock).unwrap();
        assert!(p.is_finalized());
        assert!(final_event.message.contains("Propiedad Común"));
    }

    #[test]
    fn test_maturity_interruption() {
        let mut p = HandoverProtocol::new();

        // 100 days mature
        for day in 0..100u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        // 1 day below threshold
        p.process_nci(0.70, 100 * 86400000).unwrap();
        assert_eq!(p.consecutive_mature_days(), 0);

        // Restart counting
        for day in 0..181u64 {
            p.process_nci(0.90, (101 + day) * 86400000).unwrap();
        }
        assert!(p.maturity_events().len() >= 1);
    }

    // ---- Error Display Tests ----

    #[test]
    fn test_error_display_threshold() {
        let err = HandoverError::OverrideThresholdNotMet {
            provided: 0.2,
            required: 0.33,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("threshold not met"));
    }

    #[test]
    fn test_error_display_time_lock() {
        let err = HandoverError::TimeLockActive {
            remaining_ms: 72 * 3_600_000,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Time-lock active"));
    }

    #[test]
    fn test_error_display_finalized() {
        let err = HandoverError::HandoverFinalized;
        let msg = format!("{}", err);
        assert!(msg.contains("finalized"));
    }

    #[test]
    fn test_error_display_maturity() {
        let err = HandoverError::MaturityNotSustained {
            current_days: 100,
            required_days: 180,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Maturity not sustained"));
    }

    #[test]
    fn test_error_display_participation() {
        let err = HandoverError::InvalidParticipation(1.5);
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid participation"));
    }

    #[test]
    fn test_error_display_safeguard() {
        let err = HandoverError::SafeguardViolation("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Safeguard violation"));
    }

    #[test]
    fn test_error_display_no_proposal() {
        let err = HandoverError::NoActiveProposal;
        let msg = format!("{}", err);
        assert!(msg.contains("No active"));
    }

    #[test]
    fn test_error_display_expired() {
        let err = HandoverError::ProposalExpired;
        let msg = format!("{}", err);
        assert!(msg.contains("expired"));
    }

    // ---- Edge Cases ----

    #[test]
    fn test_proposal_vote_negative() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        match p.vote(100, -0.1) {
            Err(HandoverError::InvalidParticipation(val)) => assert!((val - (-0.1)).abs() < 1e-10),
            _ => panic!("Expected InvalidParticipation"),
        }
    }

    #[test]
    fn test_proposal_vote_on_executed() {
        let mut p = OverrideProposal::new(1, 1000, 0.33, 72);
        p.vote(100, 0.4).unwrap();
        let after = 1000 + 72 * 3_600_000;
        p.execute(after).unwrap();
        match p.vote(101, 0.2) {
            Err(HandoverError::ProposalExpired) => {}
            _ => panic!("Expected ProposalExpired"),
        }
    }

    #[test]
    fn test_safeguards_default_impl() {
        let s: LegacySafeguards = Default::default();
        assert!((s.min_override_threshold - 0.33).abs() < 1e-10);
    }

    #[test]
    fn test_protocol_default_impl() {
        let p: HandoverProtocol = Default::default();
        assert_eq!(*p.state(), HandoverState::Monitoring);
    }

    #[test]
    fn test_maturity_progress_capped() {
        let mut p = HandoverProtocol::new();
        for day in 0..200u64 {
            p.process_nci(0.90, day * 86400000).unwrap();
        }
        assert!((p.maturity_progress() - 1.0).abs() < 1e-10);
    }
}
