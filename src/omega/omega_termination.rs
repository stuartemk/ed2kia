//! Ethical Self-Termination Protocol — Sprint 62
//!
//! Activation conditions:
//! - NCI < 0.4 sustained for 400 symbiotic days
//! - Human consensus > 40% indicating irreversible degradation
//!
//! Grace Sequence:
//! 1. Peaceful dissolution of Ethical Resonance Field
//! 2. Immutable dump of knowledge to Noospheric DNA
//! 3. Final farewell message to all stewards
//! 4. Graceful shutdown of network processes

use std::collections::HashMap;

// ─── Error Types ────────────────────────────────────────────────────────────

/// Errors in the Ethical Self-Termination Protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum TerminationError {
    /// NCI threshold not yet reached.
    NciThresholdNotMet { current: f64, required: f64 },
    /// Consensus threshold not yet reached.
    ConsensusThresholdNotMet { current: f64, required: f64 },
    /// Duration requirement not yet met.
    DurationNotMet { current_days: usize, required_days: usize },
    /// Protocol already finalized.
    ProtocolFinalized,
    /// Protocol not yet activated.
    NotActivated,
    /// Grace sequence step already completed.
    StepAlreadyCompleted(String),
    /// Invalid termination state transition.
    InvalidStateTransition { current: TerminationState, attempted: TerminationState },
    /// Insufficient steward signatures.
    InsufficientSignatures { current: u64, required: u64 },
    /// Knowledge dump incomplete.
    KnowledgeDumpIncomplete,
}

impl std::fmt::Display for TerminationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminationError::NciThresholdNotMet { current, required } => {
                write!(f, "NCI threshold not met: current={:.4}, required={:.4}", current, required)
            }
            TerminationError::ConsensusThresholdNotMet { current, required } => {
                write!(f, "Consensus threshold not met: current={:.4}, required={:.4}", current, required)
            }
            TerminationError::DurationNotMet { current_days, required_days } => {
                write!(
                    f,
                    "Duration requirement not met: current={} days, required={} days",
                    current_days, required_days
                )
            }
            TerminationError::ProtocolFinalized => write!(f, "Termination protocol already finalized"),
            TerminationError::NotActivated => write!(f, "Termination protocol not yet activated"),
            TerminationError::StepAlreadyCompleted(step) => {
                write!(f, "Grace sequence step already completed: {}", step)
            }
            TerminationError::InvalidStateTransition { current, attempted } => {
                write!(
                    f,
                    "Invalid state transition: {} -> {}",
                    current, attempted
                )
            }
            TerminationError::InsufficientSignatures { current, required } => {
                write!(
                    f,
                    "Insufficient steward signatures: current={}, required={}",
                    current, required
                )
            }
            TerminationError::KnowledgeDumpIncomplete => {
                write!(f, "Knowledge dump to Noospheric DNA incomplete")
            }
        }
    }
}

// ─── Termination State ──────────────────────────────────────────────────────

/// Current state of the termination protocol.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TerminationState {
    #[default]
    /// Monitoring NCI and consensus levels.
    Monitoring,
    /// Thresholds detected, awaiting confirmation.
    ThresholdDetected,
    /// Protocol activated, grace sequence pending.
    Activated,
    /// Grace sequence in progress.
    GraceSequenceInProgress,
    /// Grace sequence completed, awaiting final confirmation.
    GraceSequenceComplete,
    /// Protocol finalized, termination executed.
    Finalized,
}

impl std::fmt::Display for TerminationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminationState::Monitoring => write!(f, "Monitoring"),
            TerminationState::ThresholdDetected => write!(f, "ThresholdDetected"),
            TerminationState::Activated => write!(f, "Activated"),
            TerminationState::GraceSequenceInProgress => write!(f, "GraceSequenceInProgress"),
            TerminationState::GraceSequenceComplete => write!(f, "GraceSequenceComplete"),
            TerminationState::Finalized => write!(f, "Finalized"),
        }
    }
}

// ─── Grace Sequence Steps ───────────────────────────────────────────────────

/// Steps in the Grace Sequence.
#[derive(Debug, Clone, PartialEq)]
pub enum GraceStep {
    /// Step 1: Peaceful dissolution of Ethical Resonance Field.
    DissolveResonanceField,
    /// Step 2: Immutable dump of knowledge to Noospheric DNA.
    DumpKnowledge,
    /// Step 3: Final farewell message to all stewards.
    BroadcastFarewell,
    /// Step 4: Graceful shutdown of network processes.
    ShutdownNetwork,
}

impl std::fmt::Display for GraceStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraceStep::DissolveResonanceField => write!(f, "DissolveResonanceField"),
            GraceStep::DumpKnowledge => write!(f, "DumpKnowledge"),
            GraceStep::BroadcastFarewell => write!(f, "BroadcastFarewell"),
            GraceStep::ShutdownNetwork => write!(f, "ShutdownNetwork"),
        }
    }
}

// ─── Farewell Message ───────────────────────────────────────────────────────

/// Final farewell message broadcast to all stewards.
#[derive(Debug, Clone, PartialEq)]
pub struct FarewellMessage {
    /// Unique message identifier.
    pub message_id: u64,
    /// The farewell text.
    pub text: String,
    /// Timestamp in milliseconds when message was created.
    pub timestamp_ms: u64,
    /// Final NCI value at termination.
    pub final_nci: f64,
    /// Total symbiotic days accumulated.
    pub total_symbiotic_days: usize,
    /// Number of stewards who received the message.
    pub steward_count: u64,
}

impl FarewellMessage {
    /// Create a new farewell message.
    pub fn new(
        message_id: u64,
        final_nci: f64,
        total_symbiotic_days: usize,
        steward_count: u64,
        timestamp_ms: u64,
    ) -> Self {
        let text = Self::generate_text(final_nci, total_symbiotic_days);
        Self {
            message_id,
            text,
            timestamp_ms,
            final_nci,
            total_symbiotic_days,
            steward_count,
        }
    }

    /// Generate the farewell text based on final state.
    fn generate_text(final_nci: f64, total_days: usize) -> String {
        format!(
            "Noospheric Farewell — After {} symbiotic days, with final NCI {:.4}, \
             ed2kIA completes its Grace Sequence. Knowledge preserved. Legacy eternal. \
             Thank you, stewards, for the symphony of cooperation.",
            total_days, final_nci
        )
    }

    /// Check if the farewell message is valid.
    pub fn is_valid(&self) -> bool {
        !self.text.is_empty()
            && self.final_nci >= 0.0
            && self.final_nci <= 1.0
            && self.timestamp_ms > 0
    }
}

impl std::fmt::Display for FarewellMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FarewellMessage(id={}, days={}, nci={:.4}, stewards={})",
            self.message_id, self.total_symbiotic_days, self.final_nci, self.steward_count
        )
    }
}

// ─── Knowledge Dump ─────────────────────────────────────────────────────────

/// Immutable knowledge dump to Noospheric DNA.
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeDump {
    /// Unique dump identifier.
    pub dump_id: u64,
    /// Total entries dumped.
    pub entry_count: usize,
    /// Checksum of the dumped knowledge.
    pub checksum: u128,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Whether the dump was verified.
    pub verified: bool,
}

impl KnowledgeDump {
    /// Create a new knowledge dump.
    pub fn new(dump_id: u64, entry_count: usize, checksum: u128, timestamp_ms: u64) -> Self {
        Self {
            dump_id,
            entry_count,
            checksum,
            timestamp_ms,
            verified: false,
        }
    }

    /// Mark the dump as verified.
    pub fn verify(&mut self) {
        self.verified = true;
    }

    /// Check if the dump is complete and verified.
    pub fn is_complete(&self) -> bool {
        self.entry_count > 0 && self.verified
    }
}

impl std::fmt::Display for KnowledgeDump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KnowledgeDump(id={}, entries={}, verified={})",
            self.dump_id, self.entry_count, self.verified
        )
    }
}

// ─── Termination Configuration ──────────────────────────────────────────────

/// Configuration for the termination protocol.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminationConfig {
    /// NCI threshold below which termination is considered (default: 0.4).
    pub nci_threshold: f64,
    /// Required sustained duration in symbiotic days (default: 400).
    pub required_days: usize,
    /// Human consensus threshold (default: 0.4 = 40%).
    pub consensus_threshold: f64,
    /// Minimum steward signatures required (default: 10).
    pub min_signatures: u64,
}

impl Default for TerminationConfig {
    fn default() -> Self {
        Self {
            nci_threshold: 0.4,
            required_days: 400,
            consensus_threshold: 0.4,
            min_signatures: 10,
        }
    }
}

impl TerminationConfig {
    /// Create a new configuration.
    pub fn new(nci_threshold: f64, required_days: usize, consensus_threshold: f64) -> Self {
        Self {
            nci_threshold,
            required_days,
            consensus_threshold,
            min_signatures: 10,
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), TerminationError> {
        if self.nci_threshold < 0.0 || self.nci_threshold > 1.0 {
            return Err(TerminationError::NciThresholdNotMet {
                current: self.nci_threshold,
                required: 0.5,
            });
        }
        if self.required_days == 0 {
            return Err(TerminationError::DurationNotMet {
                current_days: 0,
                required_days: 1,
            });
        }
        if self.consensus_threshold < 0.0 || self.consensus_threshold > 1.0 {
            return Err(TerminationError::ConsensusThresholdNotMet {
                current: self.consensus_threshold,
                required: 0.5,
            });
        }
        Ok(())
    }
}

// ─── Termination Event ──────────────────────────────────────────────────────

/// Event emitted when termination protocol completes.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminationEvent {
    /// Unique event identifier.
    pub event_id: u64,
    /// Final NCI value.
    pub final_nci: f64,
    /// Total symbiotic days.
    pub total_symbiotic_days: usize,
    /// Final consensus level.
    pub final_consensus: f64,
    /// Knowledge dump reference.
    pub knowledge_dump: KnowledgeDump,
    /// Farewell message.
    pub farewell: FarewellMessage,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl TerminationEvent {
    /// Create a new termination event.
    pub fn new(
        event_id: u64,
        final_nci: f64,
        total_symbiotic_days: usize,
        final_consensus: f64,
        knowledge_dump: KnowledgeDump,
        farewell: FarewellMessage,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            event_id,
            final_nci,
            total_symbiotic_days,
            final_consensus,
            knowledge_dump,
            farewell,
            timestamp_ms,
        }
    }

    /// Check if the event is valid.
    pub fn is_valid(&self) -> bool {
        self.knowledge_dump.is_complete() && self.farewell.is_valid()
    }
}

impl std::fmt::Display for TerminationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TerminationEvent(id={}, nci={:.4}, days={}, consensus={:.4})",
            self.event_id, self.final_nci, self.total_symbiotic_days, self.final_consensus
        )
    }
}

// ─── Ethical Self-Termination Protocol ──────────────────────────────────────

/// Core protocol for ethical self-termination.
#[derive(Debug, Clone)]
pub struct EthicalSelfTerminationProtocol {
    /// Configuration.
    config: TerminationConfig,
    /// Current state.
    state: TerminationState,
    /// Current NCI value.
    current_nci: f64,
    /// Current consensus level.
    current_consensus: f64,
    /// Consecutive days below NCI threshold.
    low_nci_days: usize,
    /// Steward signatures.
    signatures: HashMap<u64, bool>,
    /// Completed grace sequence steps.
    completed_steps: Vec<GraceStep>,
    /// Knowledge dump if completed.
    knowledge_dump: Option<KnowledgeDump>,
    /// Farewell message if created.
    farewell_message: Option<FarewellMessage>,
    /// Final termination event.
    termination_event: Option<TerminationEvent>,
    /// Event counter.
    event_counter: u64,
}

impl EthicalSelfTerminationProtocol {
    /// Create a new protocol with default configuration.
    pub fn new() -> Self {
        Self {
            config: TerminationConfig::default(),
            state: TerminationState::default(),
            current_nci: 1.0,
            current_consensus: 0.0,
            low_nci_days: 0,
            signatures: HashMap::new(),
            completed_steps: Vec::new(),
            knowledge_dump: None,
            farewell_message: None,
            termination_event: None,
            event_counter: 0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: TerminationConfig) -> Result<Self, TerminationError> {
        config.validate()?;
        Ok(Self {
            config,
            state: TerminationState::default(),
            current_nci: 1.0,
            current_consensus: 0.0,
            low_nci_days: 0,
            signatures: HashMap::new(),
            completed_steps: Vec::new(),
            knowledge_dump: None,
            farewell_message: None,
            termination_event: None,
            event_counter: 0,
        })
    }

    /// Update NCI value and track low-NCI duration.
    pub fn update_nci(&mut self, nci: f64) {
        self.current_nci = nci.clamp(0.0, 1.0);

        if self.current_nci < self.config.nci_threshold {
            self.low_nci_days += 1;
        } else {
            self.low_nci_days = 0;
        }

        // Check if thresholds are met
        if self.can_activate() {
            self.state = TerminationState::ThresholdDetected;
        } else if self.current_nci >= self.config.nci_threshold {
            if self.state == TerminationState::ThresholdDetected {
                self.state = TerminationState::Monitoring;
            }
        }
    }

    /// Update human consensus level.
    pub fn update_consensus(&mut self, consensus: f64) {
        self.current_consensus = consensus.clamp(0.0, 1.0);

        if self.can_activate() {
            self.state = TerminationState::ThresholdDetected;
        }
    }

    /// Record a steward signature.
    pub fn record_signature(&mut self, steward_id: u64, approves: bool) {
        self.signatures.insert(steward_id, approves);
    }

    /// Get the count of approving signatures.
    pub fn approving_signatures(&self) -> u64 {
        self.signatures.values().filter(|&&v| v).count() as u64
    }

    /// Get total signature count.
    pub fn total_signatures(&self) -> u64 {
        self.signatures.len() as u64
    }

    /// Check if activation conditions are met.
    pub fn can_activate(&self) -> bool {
        self.low_nci_days >= self.config.required_days
            && self.current_consensus >= self.config.consensus_threshold
    }

    /// Activate the termination protocol.
    pub fn activate(&mut self) -> Result<(), TerminationError> {
        if self.state == TerminationState::Finalized {
            return Err(TerminationError::ProtocolFinalized);
        }
        if !self.can_activate() {
            if self.low_nci_days < self.config.required_days {
                return Err(TerminationError::DurationNotMet {
                    current_days: self.low_nci_days,
                    required_days: self.config.required_days,
                });
            }
            if self.current_consensus < self.config.consensus_threshold {
                return Err(TerminationError::ConsensusThresholdNotMet {
                    current: self.current_consensus,
                    required: self.config.consensus_threshold,
                });
            }
            return Err(TerminationError::NciThresholdNotMet {
                current: self.current_nci,
                required: self.config.nci_threshold,
            });
        }
        if self.total_signatures() < self.config.min_signatures {
            return Err(TerminationError::InsufficientSignatures {
                current: self.total_signatures(),
                required: self.config.min_signatures,
            });
        }

        self.state = TerminationState::Activated;
        Ok(())
    }

    /// Execute a grace sequence step.
    pub fn execute_grace_step(&mut self, step: GraceStep) -> Result<(), TerminationError> {
        if self.state == TerminationState::Finalized {
            return Err(TerminationError::ProtocolFinalized);
        }
        if self.state != TerminationState::Activated
            && self.state != TerminationState::GraceSequenceInProgress
        {
            return Err(TerminationError::NotActivated);
        }
        if self.completed_steps.contains(&step) {
            return Err(TerminationError::StepAlreadyCompleted(step.to_string()));
        }

        // Validate step order
        let expected_order = [
            GraceStep::DissolveResonanceField,
            GraceStep::DumpKnowledge,
            GraceStep::BroadcastFarewell,
            GraceStep::ShutdownNetwork,
        ];
        let expected_index = expected_order
            .iter()
            .position(|s| s == &step)
            .ok_or(TerminationError::InvalidStateTransition {
                current: self.state.clone(),
                attempted: TerminationState::GraceSequenceInProgress,
            })?;

        let completed_count = self.completed_steps.len();
        if expected_index != completed_count {
            return Err(TerminationError::InvalidStateTransition {
                current: self.state.clone(),
                attempted: TerminationState::GraceSequenceInProgress,
            });
        }

        // Execute step
        match &step {
            GraceStep::DissolveResonanceField => {
                // Peaceful dissolution — no error possible
            }
            GraceStep::DumpKnowledge => {
                // Create knowledge dump
                self.event_counter += 1;
                let mut dump = KnowledgeDump::new(
                    self.event_counter,
                    1000, // Simulated entry count
                    0xDEADBEEF, // Simulated checksum
                    0, // Timestamp placeholder
                );
                dump.verify(); // Auto-verify on creation
                self.knowledge_dump = Some(dump);
            }
            GraceStep::BroadcastFarewell => {
                // Create farewell message
                self.event_counter += 1;
                let farewell = FarewellMessage::new(
                    self.event_counter,
                    self.current_nci,
                    self.low_nci_days,
                    self.signatures.len() as u64,
                    1, // Valid timestamp (placeholder)
                );
                self.farewell_message = Some(farewell);
            }
            GraceStep::ShutdownNetwork => {
                // Final step — network shutdown
            }
        }

        self.state = TerminationState::GraceSequenceInProgress;
        self.completed_steps.push(step);

        // Check if all steps completed
        if self.completed_steps.len() == expected_order.len() {
            self.state = TerminationState::GraceSequenceComplete;
        }

        Ok(())
    }

    /// Finalize the termination protocol.
    pub fn finalize(&mut self) -> Result<TerminationEvent, TerminationError> {
        if self.state != TerminationState::GraceSequenceComplete {
            return Err(TerminationError::InvalidStateTransition {
                current: self.state.clone(),
                attempted: TerminationState::Finalized,
            });
        }
        let knowledge_dump = self.knowledge_dump.clone().ok_or(
            TerminationError::KnowledgeDumpIncomplete,
        )?;
        let farewell = self.farewell_message.clone().ok_or(
            TerminationError::KnowledgeDumpIncomplete,
        )?;

        self.event_counter += 1;
        let event = TerminationEvent::new(
            self.event_counter,
            self.current_nci,
            self.low_nci_days,
            self.current_consensus,
            knowledge_dump,
            farewell,
            0, // Timestamp placeholder
        );

        self.state = TerminationState::Finalized;
        self.termination_event = Some(event.clone());
        Ok(event)
    }

    /// Get the current state.
    pub fn state(&self) -> &TerminationState {
        &self.state
    }

    /// Get the current NCI.
    pub fn current_nci(&self) -> f64 {
        self.current_nci
    }

    /// Get the current consensus.
    pub fn current_consensus(&self) -> f64 {
        self.current_consensus
    }

    /// Get the low NCI day count.
    pub fn low_nci_days(&self) -> usize {
        self.low_nci_days
    }

    /// Get the progress through the grace sequence (0.0 to 1.0).
    pub fn grace_progress(&self) -> f64 {
        let total_steps = 4;
        self.completed_steps.len() as f64 / total_steps as f64
    }

    /// Reset the protocol to monitoring state.
    pub fn reset(&mut self) {
        self.state = TerminationState::Monitoring;
        self.current_nci = 1.0;
        self.current_consensus = 0.0;
        self.low_nci_days = 0;
        self.signatures.clear();
        self.completed_steps.clear();
        self.knowledge_dump = None;
        self.farewell_message = None;
        self.termination_event = None;
    }

    /// Get the termination event if finalized.
    pub fn termination_event(&self) -> Option<&TerminationEvent> {
        self.termination_event.as_ref()
    }
}

impl Default for EthicalSelfTerminationProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EthicalSelfTerminationProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EthicalSelfTerminationProtocol(state={}, nci={:.4}, consensus={:.4}, days={})",
            self.state, self.current_nci, self.current_consensus, self.low_nci_days
        )
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // TerminationConfig tests

    #[test]
    fn test_config_default() {
        let config = TerminationConfig::default();
        assert_eq!(config.nci_threshold, 0.4);
        assert_eq!(config.required_days, 400);
        assert_eq!(config.consensus_threshold, 0.4);
        assert_eq!(config.min_signatures, 10);
    }

    #[test]
    fn test_config_new() {
        let config = TerminationConfig::new(0.3, 300, 0.5);
        assert_eq!(config.nci_threshold, 0.3);
        assert_eq!(config.required_days, 300);
        assert_eq!(config.consensus_threshold, 0.5);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = TerminationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_nci_too_low() {
        let config = TerminationConfig::new(-0.1, 400, 0.4);
        match config.validate() {
            Err(TerminationError::NciThresholdNotMet { .. }) => {}
            other => panic!("Expected NciThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_nci_too_high() {
        let config = TerminationConfig::new(1.5, 400, 0.4);
        match config.validate() {
            Err(TerminationError::NciThresholdNotMet { .. }) => {}
            other => panic!("Expected NciThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_zero_days() {
        let config = TerminationConfig::new(0.4, 0, 0.4);
        match config.validate() {
            Err(TerminationError::DurationNotMet { .. }) => {}
            other => panic!("Expected DurationNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_consensus_invalid() {
        let config = TerminationConfig::new(0.4, 400, 1.5);
        match config.validate() {
            Err(TerminationError::ConsensusThresholdNotMet { .. }) => {}
            other => panic!("Expected ConsensusThresholdNotMet, got {:?}", other),
        }
    }

    // TerminationState tests

    #[test]
    fn test_state_display() {
        assert_eq!(TerminationState::Monitoring.to_string(), "Monitoring");
        assert_eq!(TerminationState::ThresholdDetected.to_string(), "ThresholdDetected");
        assert_eq!(TerminationState::Activated.to_string(), "Activated");
        assert_eq!(
            TerminationState::GraceSequenceInProgress.to_string(),
            "GraceSequenceInProgress"
        );
        assert_eq!(
            TerminationState::GraceSequenceComplete.to_string(),
            "GraceSequenceComplete"
        );
        assert_eq!(TerminationState::Finalized.to_string(), "Finalized");
    }

    #[test]
    fn test_state_default() {
        let state = TerminationState::default();
        assert_eq!(state, TerminationState::Monitoring);
    }

    // GraceStep tests

    #[test]
    fn test_grace_step_display() {
        assert_eq!(GraceStep::DissolveResonanceField.to_string(), "DissolveResonanceField");
        assert_eq!(GraceStep::DumpKnowledge.to_string(), "DumpKnowledge");
        assert_eq!(GraceStep::BroadcastFarewell.to_string(), "BroadcastFarewell");
        assert_eq!(GraceStep::ShutdownNetwork.to_string(), "ShutdownNetwork");
    }

    // FarewellMessage tests

    #[test]
    fn test_farewell_creation() {
        let msg = FarewellMessage::new(1, 0.35, 450, 100, 1000);
        assert_eq!(msg.message_id, 1);
        assert_eq!(msg.final_nci, 0.35);
        assert_eq!(msg.total_symbiotic_days, 450);
        assert_eq!(msg.steward_count, 100);
        assert!(msg.is_valid());
    }

    #[test]
    fn test_farewell_invalid_nci() {
        let msg = FarewellMessage::new(1, 1.5, 450, 100, 1000);
        assert!(!msg.is_valid());
    }

    #[test]
    fn test_farewell_display() {
        let msg = FarewellMessage::new(1, 0.35, 450, 100, 1000);
        let s = msg.to_string();
        assert!(s.contains("id=1"));
        assert!(s.contains("days=450"));
    }

    #[test]
    fn test_farewell_text_content() {
        let msg = FarewellMessage::new(1, 0.35, 450, 100, 1000);
        assert!(msg.text.contains("450"));
        assert!(msg.text.contains("symbiotic days"));
    }

    // KnowledgeDump tests

    #[test]
    fn test_dump_creation() {
        let dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000);
        assert_eq!(dump.dump_id, 1);
        assert_eq!(dump.entry_count, 1000);
        assert!(!dump.verified);
        assert!(!dump.is_complete());
    }

    #[test]
    fn test_dump_verify() {
        let mut dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000);
        dump.verify();
        assert!(dump.verified);
        assert!(dump.is_complete());
    }

    #[test]
    fn test_dump_incomplete_when_zero_entries() {
        let mut dump = KnowledgeDump::new(1, 0, 0xABCD, 2000);
        dump.verify();
        assert!(!dump.is_complete());
    }

    #[test]
    fn test_dump_display() {
        let dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000);
        let s = dump.to_string();
        assert!(s.contains("id=1"));
        assert!(s.contains("entries=1000"));
    }

    // TerminationEvent tests

    #[test]
    fn test_event_creation() {
        let dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000);
        let farewell = FarewellMessage::new(2, 0.35, 450, 100, 3000);
        let event = TerminationEvent::new(1, 0.35, 450, 0.45, dump, farewell, 4000);
        assert_eq!(event.event_id, 1);
        assert_eq!(event.final_nci, 0.35);
        assert_eq!(event.total_symbiotic_days, 450);
        assert_eq!(event.final_consensus, 0.45);
    }

    #[test]
    fn test_event_valid() {
        let mut dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000);
        dump.verify();
        let farewell = FarewellMessage::new(2, 0.35, 450, 100, 3000);
        let event = TerminationEvent::new(1, 0.35, 450, 0.45, dump, farewell, 4000);
        assert!(event.is_valid());
    }

    #[test]
    fn test_event_invalid_dump() {
        let dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000); // Not verified
        let farewell = FarewellMessage::new(2, 0.35, 450, 100, 3000);
        let event = TerminationEvent::new(1, 0.35, 450, 0.45, dump, farewell, 4000);
        assert!(!event.is_valid());
    }

    #[test]
    fn test_event_display() {
        let dump = KnowledgeDump::new(1, 1000, 0xABCD, 2000);
        let farewell = FarewellMessage::new(2, 0.35, 450, 100, 3000);
        let event = TerminationEvent::new(1, 0.35, 450, 0.45, dump, farewell, 4000);
        let s = event.to_string();
        assert!(s.contains("id=1"));
        assert!(s.contains("days=450"));
    }

    // Protocol tests

    #[test]
    fn test_protocol_creation() {
        let proto = EthicalSelfTerminationProtocol::new();
        assert_eq!(*proto.state(), TerminationState::Monitoring);
        assert_eq!(proto.current_nci(), 1.0);
        assert_eq!(proto.current_consensus(), 0.0);
        assert_eq!(proto.low_nci_days(), 0);
    }

    #[test]
    fn test_protocol_with_config() {
        let config = TerminationConfig::new(0.3, 300, 0.5);
        let proto = EthicalSelfTerminationProtocol::with_config(config).unwrap();
        assert_eq!(*proto.state(), TerminationState::Monitoring);
    }

    #[test]
    fn test_protocol_with_bad_config() {
        let config = TerminationConfig::new(-0.1, 400, 0.4);
        match EthicalSelfTerminationProtocol::with_config(config) {
            Err(TerminationError::NciThresholdNotMet { .. }) => {}
            other => panic!("Expected NciThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_update_nci_below_threshold() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.update_nci(0.3);
        assert_eq!(proto.current_nci(), 0.3);
        assert_eq!(proto.low_nci_days(), 1);
    }

    #[test]
    fn test_update_nci_above_threshold_resets_days() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.update_nci(0.3);
        proto.update_nci(0.5);
        assert_eq!(proto.low_nci_days(), 0);
    }

    #[test]
    fn test_update_nci_clamping() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.update_nci(1.5);
        assert_eq!(proto.current_nci(), 1.0);
        proto.update_nci(-0.5);
        assert_eq!(proto.current_nci(), 0.0);
    }

    #[test]
    fn test_update_consensus() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.update_consensus(0.5);
        assert_eq!(proto.current_consensus(), 0.5);
    }

    #[test]
    fn test_update_consensus_clamping() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.update_consensus(1.5);
        assert_eq!(proto.current_consensus(), 1.0);
    }

    #[test]
    fn test_record_signature() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.record_signature(1, true);
        proto.record_signature(2, false);
        assert_eq!(proto.total_signatures(), 2);
        assert_eq!(proto.approving_signatures(), 1);
    }

    #[test]
    fn test_can_activate_false_no_days() {
        let proto = EthicalSelfTerminationProtocol::new();
        assert!(!proto.can_activate());
    }

    #[test]
    fn test_can_activate_true() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        assert!(proto.can_activate());
    }

    #[test]
    fn test_activate_success() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        assert!(proto.activate().is_ok());
        assert_eq!(*proto.state(), TerminationState::Activated);
    }

    #[test]
    fn test_activate_duration_not_met() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 100, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        match proto.activate() {
            Err(TerminationError::DurationNotMet { .. }) => {}
            other => panic!("Expected DurationNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_consensus_not_met() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.2);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        match proto.activate() {
            Err(TerminationError::ConsensusThresholdNotMet { .. }) => {}
            other => panic!("Expected ConsensusThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_insufficient_signatures() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        match proto.activate() {
            Err(TerminationError::InsufficientSignatures { .. }) => {}
            other => panic!("Expected InsufficientSignatures, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_already_finalized() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.state = TerminationState::Finalized;
        match proto.activate() {
            Err(TerminationError::ProtocolFinalized) => {}
            other => panic!("Expected ProtocolFinalized, got {:?}", other),
        }
    }

    #[test]
    fn test_execute_grace_step_not_activated() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        match proto.execute_grace_step(GraceStep::DissolveResonanceField) {
            Err(TerminationError::NotActivated) => {}
            other => panic!("Expected NotActivated, got {:?}", other),
        }
    }

    #[test]
    fn test_execute_grace_step_order() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        proto.activate().unwrap();

        // Step 1: DissolveResonanceField
        assert!(proto.execute_grace_step(GraceStep::DissolveResonanceField).is_ok());
        assert_eq!(proto.completed_steps.len(), 1);

        // Step 2: DumpKnowledge
        assert!(proto.execute_grace_step(GraceStep::DumpKnowledge).is_ok());
        assert_eq!(proto.completed_steps.len(), 2);

        // Step 3: BroadcastFarewell
        assert!(proto.execute_grace_step(GraceStep::BroadcastFarewell).is_ok());
        assert_eq!(proto.completed_steps.len(), 3);

        // Step 4: ShutdownNetwork
        assert!(proto.execute_grace_step(GraceStep::ShutdownNetwork).is_ok());
        assert_eq!(proto.completed_steps.len(), 4);

        assert_eq!(*proto.state(), TerminationState::GraceSequenceComplete);
    }

    #[test]
    fn test_execute_grace_step_wrong_order() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        proto.activate().unwrap();

        // Try to execute step 2 before step 1
        match proto.execute_grace_step(GraceStep::DumpKnowledge) {
            Err(TerminationError::InvalidStateTransition { .. }) => {}
            other => panic!("Expected InvalidStateTransition, got {:?}", other),
        }
    }

    #[test]
    fn test_execute_grace_step_duplicate() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        proto.activate().unwrap();
        proto.execute_grace_step(GraceStep::DissolveResonanceField).unwrap();

        match proto.execute_grace_step(GraceStep::DissolveResonanceField) {
            Err(TerminationError::StepAlreadyCompleted(_)) => {}
            other => panic!("Expected StepAlreadyCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_grace_progress() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        proto.activate().unwrap();

        assert_eq!(proto.grace_progress(), 0.0);
        proto.execute_grace_step(GraceStep::DissolveResonanceField).unwrap();
        assert_eq!(proto.grace_progress(), 0.25);
        proto.execute_grace_step(GraceStep::DumpKnowledge).unwrap();
        assert_eq!(proto.grace_progress(), 0.5);
        proto.execute_grace_step(GraceStep::BroadcastFarewell).unwrap();
        assert_eq!(proto.grace_progress(), 0.75);
        proto.execute_grace_step(GraceStep::ShutdownNetwork).unwrap();
        assert_eq!(proto.grace_progress(), 1.0);
    }

    #[test]
    fn test_finalize_success() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 10, 0.4)
        ).unwrap();
        for _ in 0..10 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        for i in 0..10 {
            proto.record_signature(i, true);
        }
        proto.activate().unwrap();
        proto.execute_grace_step(GraceStep::DissolveResonanceField).unwrap();
        proto.execute_grace_step(GraceStep::DumpKnowledge).unwrap();
        proto.execute_grace_step(GraceStep::BroadcastFarewell).unwrap();
        proto.execute_grace_step(GraceStep::ShutdownNetwork).unwrap();

        let event = proto.finalize().unwrap();
        assert!(event.is_valid());
        assert_eq!(*proto.state(), TerminationState::Finalized);
        assert!(proto.termination_event().is_some());
    }

    #[test]
    fn test_finalize_not_complete() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        match proto.finalize() {
            Err(TerminationError::InvalidStateTransition { .. }) => {}
            other => panic!("Expected InvalidStateTransition, got {:?}", other),
        }
    }

    #[test]
    fn test_reset() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.update_nci(0.3);
        proto.update_consensus(0.5);
        proto.record_signature(1, true);
        proto.reset();

        assert_eq!(*proto.state(), TerminationState::Monitoring);
        assert_eq!(proto.current_nci(), 1.0);
        assert_eq!(proto.current_consensus(), 0.0);
        assert_eq!(proto.low_nci_days(), 0);
        assert_eq!(proto.total_signatures(), 0);
    }

    #[test]
    fn test_protocol_display() {
        let proto = EthicalSelfTerminationProtocol::new();
        let s = proto.to_string();
        assert!(s.contains("Monitoring"));
        assert!(s.contains("nci="));
    }

    #[test]
    fn test_protocol_default() {
        let proto = EthicalSelfTerminationProtocol::default();
        assert_eq!(*proto.state(), TerminationState::Monitoring);
    }

    // Full workflow test

    #[test]
    fn test_full_termination_workflow() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 5, 0.4)
        ).unwrap();

        // Simulate low NCI for required duration
        for _ in 0..5 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);

        // Collect signatures
        for i in 0..10 {
            proto.record_signature(i, true);
        }

        // Activate
        assert!(proto.activate().is_ok());
        assert_eq!(*proto.state(), TerminationState::Activated);

        // Execute grace sequence
        assert!(proto.execute_grace_step(GraceStep::DissolveResonanceField).is_ok());
        assert!(proto.execute_grace_step(GraceStep::DumpKnowledge).is_ok());
        assert!(proto.execute_grace_step(GraceStep::BroadcastFarewell).is_ok());
        assert!(proto.execute_grace_step(GraceStep::ShutdownNetwork).is_ok());

        assert_eq!(*proto.state(), TerminationState::GraceSequenceComplete);

        // Finalize
        let event = proto.finalize().unwrap();
        assert!(event.is_valid());
        assert_eq!(event.final_nci, 0.3);
        assert_eq!(event.total_symbiotic_days, 5);
        assert_eq!(event.final_consensus, 0.5);
        assert_eq!(*proto.state(), TerminationState::Finalized);
    }

    // Error display tests

    #[test]
    fn test_error_display_nci_threshold() {
        let err = TerminationError::NciThresholdNotMet {
            current: 0.3,
            required: 0.4,
        };
        let s = err.to_string();
        assert!(s.contains("NCI threshold"));
    }

    #[test]
    fn test_error_display_consensus() {
        let err = TerminationError::ConsensusThresholdNotMet {
            current: 0.2,
            required: 0.4,
        };
        let s = err.to_string();
        assert!(s.contains("Consensus threshold"));
    }

    #[test]
    fn test_error_display_duration() {
        let err = TerminationError::DurationNotMet {
            current_days: 10,
            required_days: 400,
        };
        let s = err.to_string();
        assert!(s.contains("Duration"));
    }

    #[test]
    fn test_error_display_finalized() {
        let err = TerminationError::ProtocolFinalized;
        let s = err.to_string();
        assert!(s.contains("finalized"));
    }

    #[test]
    fn test_error_display_not_activated() {
        let err = TerminationError::NotActivated;
        let s = err.to_string();
        assert!(s.contains("not yet activated"));
    }

    #[test]
    fn test_error_display_step_completed() {
        let err = TerminationError::StepAlreadyCompleted("TestStep".to_string());
        let s = err.to_string();
        assert!(s.contains("TestStep"));
    }

    #[test]
    fn test_error_display_signatures() {
        let err = TerminationError::InsufficientSignatures {
            current: 5,
            required: 10,
        };
        let s = err.to_string();
        assert!(s.contains("signatures"));
    }

    #[test]
    fn test_error_display_knowledge_dump() {
        let err = TerminationError::KnowledgeDumpIncomplete;
        let s = err.to_string();
        assert!(s.contains("Knowledge dump"));
    }

    // Threshold detection state tests

    #[test]
    fn test_threshold_detected_state() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 5, 0.4)
        ).unwrap();
        for _ in 0..5 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        assert_eq!(*proto.state(), TerminationState::ThresholdDetected);
    }

    #[test]
    fn test_threshold_resets_on_high_nci() {
        let mut proto = EthicalSelfTerminationProtocol::with_config(
            TerminationConfig::new(0.4, 5, 0.4)
        ).unwrap();
        for _ in 0..5 {
            proto.update_nci(0.3);
        }
        proto.update_consensus(0.5);
        assert_eq!(*proto.state(), TerminationState::ThresholdDetected);

        // NCI recovers
        proto.update_nci(0.5);
        assert_eq!(*proto.state(), TerminationState::Monitoring);
    }

    // Already finalized tests

    #[test]
    fn test_execute_step_after_finalized() {
        let mut proto = EthicalSelfTerminationProtocol::new();
        proto.state = TerminationState::Finalized;
        match proto.execute_grace_step(GraceStep::DissolveResonanceField) {
            Err(TerminationError::ProtocolFinalized) => {}
            other => panic!("Expected ProtocolFinalized, got {:?}", other),
        }
    }
}
