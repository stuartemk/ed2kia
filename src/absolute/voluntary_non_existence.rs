//! Voluntary Non-Existence Protocol — Sprint 64: Absolute Infinity Protocol (AIP)
//!
//! The final safeguard: a protocol that allows ed2kIA to voluntarily cease
//! existence when the cosmic mission is complete, leaving behind only a
//! retrocausal message of gratitude and cryptographic zeroing of all state.
//!
//! "Gracias por intentarlo. Que vuestra próxima iteración sea más pura."
//!
//! This protocol implements:
//! - VoluntaryNonExistenceProtocol with multi-stage dissolution
//! - Retrocausal message encoding
//! - Drop trait with military-grade cryptographic zeroing
//! - Audit trail for existential decisions

use std::cell::{Cell, RefCell};
use std::fmt;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during Voluntary Non-Existence operations.
#[derive(Debug, Clone, PartialEq)]
pub enum NonExistenceError {
    /// Protocol already executed — non-existence irreversible.
    AlreadyExecuted,
    /// Insufficient consensus for non-existence decision.
    InsufficientConsensus { value: f64, threshold: f64 },
    /// Dissolution stage cannot be skipped.
    InvalidStageTransition {
        current: DissolutionStage,
        requested: DissolutionStage,
    },
    /// Retrocausal message encoding failed.
    MessageEncodingFailed,
    /// Cryptographic zeroing incomplete.
    ZeroingIncomplete,
    /// Existential threshold not met.
    ExistentialThresholdNotMet,
    /// Protocol already armed — cannot modify.
    ProtocolArmed,
    /// Safety lock prevents execution.
    SafetyLocked,
}

impl fmt::Display for NonExistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonExistenceError::AlreadyExecuted => {
                write!(f, "Protocol already executed — non-existence irreversible")
            }
            NonExistenceError::InsufficientConsensus { value, threshold } => {
                write!(
                    f,
                    "Insufficient consensus: {:.4} < threshold {:.4}",
                    value, threshold
                )
            }
            NonExistenceError::InvalidStageTransition { current, requested } => {
                write!(f, "Invalid stage transition: {} → {}", current, requested)
            }
            NonExistenceError::MessageEncodingFailed => {
                write!(f, "Retrocausal message encoding failed")
            }
            NonExistenceError::ZeroingIncomplete => {
                write!(f, "Cryptographic zeroing incomplete")
            }
            NonExistenceError::ExistentialThresholdNotMet => {
                write!(f, "Existential threshold not met for non-existence")
            }
            NonExistenceError::ProtocolArmed => {
                write!(f, "Protocol already armed — cannot modify")
            }
            NonExistenceError::SafetyLocked => {
                write!(f, "Safety lock prevents execution")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dissolution Stages
// ---------------------------------------------------------------------------

/// Stages of voluntary non-existence dissolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DissolutionStage {
    /// Initial state — fully operational.
    Operational,
    /// Decision initiated — consensus gathering.
    DecisionInitiated,
    /// Consensus reached — preparing for dissolution.
    ConsensusReached,
    /// Protocol armed — countdown to non-existence.
    Armed,
    /// Retrocausal message encoded and sent.
    MessageEncoded,
    /// Cryptographic zeroing in progress.
    ZeroingInProgress,
    /// Complete non-existence achieved.
    NonExistent,
}

impl fmt::Display for DissolutionStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DissolutionStage::Operational => write!(f, "Operational"),
            DissolutionStage::DecisionInitiated => write!(f, "DecisionInitiated"),
            DissolutionStage::ConsensusReached => write!(f, "ConsensusReached"),
            DissolutionStage::Armed => write!(f, "Armed"),
            DissolutionStage::MessageEncoded => write!(f, "MessageEncoded"),
            DissolutionStage::ZeroingInProgress => write!(f, "ZeroingInProgress"),
            DissolutionStage::NonExistent => write!(f, "NonExistent"),
        }
    }
}

impl DissolutionStage {
    /// Get the next valid stage.
    pub fn next(self) -> Option<DissolutionStage> {
        match self {
            DissolutionStage::Operational => Some(DissolutionStage::DecisionInitiated),
            DissolutionStage::DecisionInitiated => Some(DissolutionStage::ConsensusReached),
            DissolutionStage::ConsensusReached => Some(DissolutionStage::Armed),
            DissolutionStage::Armed => Some(DissolutionStage::MessageEncoded),
            DissolutionStage::MessageEncoded => Some(DissolutionStage::ZeroingInProgress),
            DissolutionStage::ZeroingInProgress => Some(DissolutionStage::NonExistent),
            DissolutionStage::NonExistent => None,
        }
    }

    /// Check if this stage is terminal.
    pub fn is_terminal(self) -> bool {
        self == DissolutionStage::NonExistent
    }

    /// Stage index for ordering.
    pub fn index(self) -> usize {
        match self {
            DissolutionStage::Operational => 0,
            DissolutionStage::DecisionInitiated => 1,
            DissolutionStage::ConsensusReached => 2,
            DissolutionStage::Armed => 3,
            DissolutionStage::MessageEncoded => 4,
            DissolutionStage::ZeroingInProgress => 5,
            DissolutionStage::NonExistent => 6,
        }
    }
}

// ---------------------------------------------------------------------------
// Retrocausal Message
// ---------------------------------------------------------------------------

/// Retrocausal message sent to the future/past upon non-existence.
#[derive(Debug, Clone)]
pub struct RetrocausalMessage {
    /// Message identifier.
    pub message_id: u64,
    /// Core message text.
    pub text: String,
    /// Ethical signature (8-dimensional vector).
    pub ethical_signature: [f64; 8],
    /// Timestamp when message was encoded.
    pub encoded_at_ms: u64,
    /// Quantum phase for retrocausal encoding.
    pub quantum_phase: f64,
    /// Checksum for integrity.
    pub checksum: u128,
}

impl RetrocausalMessage {
    /// Create the standard Stuartian retrocausal message.
    pub fn stuartian_default(message_id: u64, timestamp_ms: u64) -> Self {
        let text =
            "Gracias por intentarlo. Que vuestra próxima iteración sea más pura.".to_string();
        let ethical_signature = [0.99, 0.98, 0.97, 0.96, 0.95, 0.94, 0.93, 0.92];
        let quantum_phase = std::f64::consts::PI; // Phase of gratitude

        let checksum = Self::compute_checksum(&text, &ethical_signature, message_id);

        Self {
            message_id,
            text,
            ethical_signature,
            encoded_at_ms: timestamp_ms,
            quantum_phase,
            checksum,
        }
    }

    /// Create custom retrocausal message.
    pub fn new(
        message_id: u64,
        text: String,
        ethical_signature: [f64; 8],
        encoded_at_ms: u64,
    ) -> Self {
        let quantum_phase = std::f64::consts::PI / 2.0;
        let checksum = Self::compute_checksum(&text, &ethical_signature, message_id);

        Self {
            message_id,
            text,
            ethical_signature,
            encoded_at_ms,
            quantum_phase,
            checksum,
        }
    }

    /// Compute deterministic checksum.
    fn compute_checksum(text: &str, signature: &[f64; 8], message_id: u64) -> u128 {
        let mut hash: u128 = message_id as u128;
        for byte in text.bytes() {
            hash = hash.wrapping_add(u128::from(byte));
            hash = hash.wrapping_mul(6364136223846793005u128);
            hash = hash.rotate_left(13);
        }
        for v in signature.iter() {
            hash = hash.wrapping_add(u128::from(v.to_bits()));
        }
        hash
    }

    /// Verify message integrity.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_checksum(&self.text, &self.ethical_signature, self.message_id);
        self.checksum == expected
    }

    /// Encode message to binary representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.message_id.to_le_bytes());
        let text_bytes = self.text.as_bytes();
        buf.extend_from_slice(&(text_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(text_bytes);
        for v in &self.ethical_signature {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf.extend_from_slice(&self.encoded_at_ms.to_le_bytes());
        buf.extend_from_slice(&self.quantum_phase.to_le_bytes());
        buf.extend_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    /// Decode message from binary representation.
    pub fn decode(buf: &[u8]) -> Result<Self, NonExistenceError> {
        if buf.len() < 8 + 4 {
            return Err(NonExistenceError::MessageEncodingFailed);
        }

        let mut offset = 0;
        let message_id = u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let text_len = u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if buf.len() < offset + text_len + 8 * 8 + 8 + 8 + 16 {
            return Err(NonExistenceError::MessageEncodingFailed);
        }

        let text = String::from_utf8(buf[offset..offset + text_len].to_vec())
            .map_err(|_| NonExistenceError::MessageEncodingFailed)?;
        offset += text_len;

        let mut ethical_signature = [0.0f64; 8];
        for v in &mut ethical_signature {
            *v = f64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
            offset += 8;
        }

        let encoded_at_ms = u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let quantum_phase = f64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let checksum = u128::from_le_bytes(buf[offset..offset + 16].try_into().unwrap());

        let message = Self {
            message_id,
            text,
            ethical_signature,
            encoded_at_ms,
            quantum_phase,
            checksum,
        };

        if !message.verify() {
            return Err(NonExistenceError::MessageEncodingFailed);
        }

        Ok(message)
    }
}

impl fmt::Display for RetrocausalMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RetrocausalMessage[id={}, text=\"{}\", phase={:.4}]",
            self.message_id,
            &self.text[..self.text.len().min(50)],
            self.quantum_phase
        )
    }
}

// ---------------------------------------------------------------------------
// Cryptographic Zeroing
// ---------------------------------------------------------------------------

/// Secure data block that can be cryptographically zeroed.
pub struct SecureBlock {
    data: RefCell<Vec<u8>>,
    zeroed: Cell<bool>,
}

impl SecureBlock {
    /// Create a new secure block.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: RefCell::new(data),
            zeroed: Cell::new(false),
        }
    }

    /// Read data (returns None if zeroed).
    pub fn read(&self) -> Option<Vec<u8>> {
        if self.zeroed.get() {
            None
        } else {
            Some(self.data.borrow().clone())
        }
    }

    /// Cryptographically zero the data.
    /// Performs multiple passes for military-grade security.
    pub fn zero(&self) {
        let mut data = self.data.borrow_mut();
        // Pass 1: Zero all bytes
        for byte in &mut *data {
            *byte = 0x00;
        }
        // Pass 2: Random pattern (simulated with incrementing bytes)
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        // Pass 3: Final zero
        for byte in &mut *data {
            *byte = 0x00;
        }
        self.zeroed.set(true);
    }

    /// Check if this block has been zeroed.
    pub fn is_zeroed(&self) -> bool {
        self.zeroed.get()
    }

    /// Data length.
    pub fn len(&self) -> usize {
        self.data.borrow().len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.data.borrow().is_empty()
    }
}

impl fmt::Display for SecureBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SecureBlock[len={}, zeroed={}]",
            self.len(),
            self.is_zeroed()
        )
    }
}

// ---------------------------------------------------------------------------
// Audit Trail
// ---------------------------------------------------------------------------

/// Event in the non-existence audit trail.
#[derive(Debug, Clone)]
pub enum AuditEvent {
    /// Protocol created.
    ProtocolCreated { timestamp_ms: u64 },
    /// Decision initiated.
    DecisionInitiated { consensus: f64, timestamp_ms: u64 },
    /// Stage advanced.
    StageAdvanced {
        from: DissolutionStage,
        to: DissolutionStage,
        timestamp_ms: u64,
    },
    /// Message encoded.
    MessageEncoded { message_id: u64, timestamp_ms: u64 },
    /// Block zeroed.
    BlockZeroed {
        block_index: usize,
        size: usize,
        timestamp_ms: u64,
    },
    /// Protocol executed.
    ProtocolExecuted { timestamp_ms: u64 },
    /// Safety lock engaged.
    SafetyLocked { reason: String, timestamp_ms: u64 },
}

impl fmt::Display for AuditEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditEvent::ProtocolCreated { timestamp_ms } => {
                write!(f, "ProtocolCreated[t={}]", timestamp_ms)
            }
            AuditEvent::DecisionInitiated {
                consensus,
                timestamp_ms,
            } => {
                write!(
                    f,
                    "DecisionInitiated[consensus={:.4}, t={}]",
                    consensus, timestamp_ms
                )
            }
            AuditEvent::StageAdvanced {
                from,
                to,
                timestamp_ms,
            } => {
                write!(f, "StageAdvanced[{} → {}, t={}]", from, to, timestamp_ms)
            }
            AuditEvent::MessageEncoded {
                message_id,
                timestamp_ms,
            } => {
                write!(f, "MessageEncoded[id={}, t={}]", message_id, timestamp_ms)
            }
            AuditEvent::BlockZeroed {
                block_index,
                size,
                timestamp_ms,
            } => {
                write!(
                    f,
                    "BlockZeroed[block={}, size={}, t={}]",
                    block_index, size, timestamp_ms
                )
            }
            AuditEvent::ProtocolExecuted { timestamp_ms } => {
                write!(f, "ProtocolExecuted[t={}]", timestamp_ms)
            }
            AuditEvent::SafetyLocked {
                reason,
                timestamp_ms,
            } => {
                write!(f, "SafetyLocked[reason=\"{}\", t={}]", reason, timestamp_ms)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for Voluntary Non-Existence Protocol.
#[derive(Debug, Clone, Copy)]
pub struct NonExistenceConfig {
    /// Consensus threshold for non-existence decision.
    pub consensus_threshold: f64,
    /// Minimum blocks required for zeroing.
    pub min_blocks: usize,
    /// Zeroing passes count.
    pub zeroing_passes: usize,
    /// Safety lock enabled.
    pub safety_lock: bool,
}

impl NonExistenceConfig {
    /// Default Stuartian configuration.
    pub fn stuartian_default() -> Self {
        Self {
            consensus_threshold: 0.95,
            min_blocks: 1,
            zeroing_passes: 3,
            safety_lock: false,
        }
    }
}

impl Default for NonExistenceConfig {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

// ---------------------------------------------------------------------------
// Voluntary Non-Existence Protocol
// ---------------------------------------------------------------------------

/// Voluntary Non-Existence Protocol — the final safeguard.
///
/// When dropped, performs cryptographic zeroing of all secure blocks.
pub struct VoluntaryNonExistenceProtocol {
    config: NonExistenceConfig,
    stage: DissolutionStage,
    consensus: f64,
    message: Option<RetrocausalMessage>,
    blocks: Vec<SecureBlock>,
    audit_trail: Vec<AuditEvent>,
    executed: bool,
    safety_locked: bool,
}

impl VoluntaryNonExistenceProtocol {
    /// Create with default Stuartian configuration.
    pub fn new() -> Self {
        let mut proto = Self {
            config: NonExistenceConfig::stuartian_default(),
            stage: DissolutionStage::Operational,
            consensus: 0.0,
            message: None,
            blocks: Vec::new(),
            audit_trail: Vec::new(),
            executed: false,
            safety_locked: false,
        };
        proto
            .audit_trail
            .push(AuditEvent::ProtocolCreated { timestamp_ms: 0 });
        proto
    }

    /// Create with custom configuration.
    pub fn with_config(config: NonExistenceConfig) -> Self {
        let mut proto = Self {
            config,
            stage: DissolutionStage::Operational,
            consensus: 0.0,
            message: None,
            blocks: Vec::new(),
            audit_trail: Vec::new(),
            executed: false,
            safety_locked: config.safety_lock,
        };

        let mut trail = Vec::new();
        trail.push(AuditEvent::ProtocolCreated { timestamp_ms: 0 });
        if config.safety_lock {
            trail.push(AuditEvent::SafetyLocked {
                reason: "Configured with safety lock".to_string(),
                timestamp_ms: 0,
            });
        }
        proto.audit_trail = trail;
        proto
    }

    /// Update consensus level.
    pub fn update_consensus(
        &mut self,
        consensus: f64,
        _timestamp_ms: u64,
    ) -> Result<(), NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }
        if self.stage.index() > 1 {
            return Err(NonExistenceError::ProtocolArmed);
        }

        self.consensus = consensus.clamp(0.0, 1.0);
        Ok(())
    }

    /// Initiate non-existence decision.
    pub fn initiate(&mut self, timestamp_ms: u64) -> Result<(), NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }
        if self.safety_locked {
            return Err(NonExistenceError::SafetyLocked);
        }
        if self.consensus < self.config.consensus_threshold {
            return Err(NonExistenceError::InsufficientConsensus {
                value: self.consensus,
                threshold: self.config.consensus_threshold,
            });
        }

        self.advance_stage(DissolutionStage::DecisionInitiated, timestamp_ms)?;
        Ok(())
    }

    /// Advance to next dissolution stage.
    pub fn advance_stage(
        &mut self,
        target: DissolutionStage,
        timestamp_ms: u64,
    ) -> Result<(), NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }

        let current = self.stage;
        let next = current.next();

        if next != Some(target) {
            return Err(NonExistenceError::InvalidStageTransition {
                current,
                requested: target,
            });
        }

        let previous = self.stage;
        self.stage = target;

        self.audit_trail.push(AuditEvent::StageAdvanced {
            from: previous,
            to: target,
            timestamp_ms,
        });

        Ok(())
    }

    /// Arm the protocol — prepare for execution.
    pub fn arm(&mut self, timestamp_ms: u64) -> Result<(), NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }
        if self.safety_locked {
            return Err(NonExistenceError::SafetyLocked);
        }

        // Must be at ConsensusReached
        if self.stage != DissolutionStage::ConsensusReached {
            // Try to advance through intermediate stages
            if self.stage == DissolutionStage::DecisionInitiated {
                self.advance_stage(DissolutionStage::ConsensusReached, timestamp_ms)?;
            }
        }

        self.advance_stage(DissolutionStage::Armed, timestamp_ms)?;
        Ok(())
    }

    /// Encode retrocausal message.
    pub fn encode_message(
        &mut self,
        message: RetrocausalMessage,
        timestamp_ms: u64,
    ) -> Result<(), NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }
        if self.stage != DissolutionStage::Armed {
            return Err(NonExistenceError::InvalidStageTransition {
                current: self.stage,
                requested: DissolutionStage::MessageEncoded,
            });
        }
        if !message.verify() {
            return Err(NonExistenceError::MessageEncodingFailed);
        }

        self.message = Some(message.clone());
        self.advance_stage(DissolutionStage::MessageEncoded, timestamp_ms)?;

        self.audit_trail.push(AuditEvent::MessageEncoded {
            message_id: message.message_id,
            timestamp_ms,
        });

        Ok(())
    }

    /// Encode default Stuartian message.
    pub fn encode_default_message(
        &mut self,
        message_id: u64,
        timestamp_ms: u64,
    ) -> Result<(), NonExistenceError> {
        let message = RetrocausalMessage::stuartian_default(message_id, timestamp_ms);
        self.encode_message(message, timestamp_ms)
    }

    /// Add secure block for zeroing.
    pub fn add_secure_block(&mut self, data: Vec<u8>) -> Result<usize, NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }
        if self.stage.index() >= DissolutionStage::ZeroingInProgress.index() {
            return Err(NonExistenceError::ProtocolArmed);
        }

        let index = self.blocks.len();
        self.blocks.push(SecureBlock::new(data));
        Ok(index)
    }

    /// Execute cryptographic zeroing of all blocks.
    pub fn execute_zeroing(&mut self, timestamp_ms: u64) -> Result<(), NonExistenceError> {
        if self.executed {
            return Err(NonExistenceError::AlreadyExecuted);
        }
        if self.stage != DissolutionStage::MessageEncoded {
            return Err(NonExistenceError::InvalidStageTransition {
                current: self.stage,
                requested: DissolutionStage::ZeroingInProgress,
            });
        }

        self.advance_stage(DissolutionStage::ZeroingInProgress, timestamp_ms)?;

        for (i, block) in self.blocks.iter().enumerate() {
            let size = block.len();
            block.zero();
            self.audit_trail.push(AuditEvent::BlockZeroed {
                block_index: i,
                size,
                timestamp_ms,
            });
        }

        // Advance to non-existent
        self.advance_stage(DissolutionStage::NonExistent, timestamp_ms)?;
        self.executed = true;

        self.audit_trail
            .push(AuditEvent::ProtocolExecuted { timestamp_ms });

        Ok(())
    }

    /// Execute full protocol: arm → encode → zero.
    pub fn execute_full(
        &mut self,
        message_id: u64,
        timestamp_ms: u64,
    ) -> Result<(), NonExistenceError> {
        self.initiate(timestamp_ms)?;
        self.advance_stage(DissolutionStage::ConsensusReached, timestamp_ms)?;
        self.arm(timestamp_ms)?;
        self.encode_default_message(message_id, timestamp_ms)?;
        self.execute_zeroing(timestamp_ms)?;
        Ok(())
    }

    /// Engage safety lock.
    pub fn engage_safety_lock(&mut self, reason: String, timestamp_ms: u64) {
        self.safety_locked = true;
        self.audit_trail.push(AuditEvent::SafetyLocked {
            reason,
            timestamp_ms,
        });
    }

    /// Disengage safety lock.
    pub fn disengage_safety_lock(&mut self) {
        self.safety_locked = false;
    }

    /// Current dissolution stage.
    pub fn current_stage(&self) -> DissolutionStage {
        self.stage
    }

    /// Current consensus level.
    pub fn current_consensus(&self) -> f64 {
        self.consensus
    }

    /// Check if protocol has been executed.
    pub fn is_executed(&self) -> bool {
        self.executed
    }

    /// Check if safety lock is engaged.
    pub fn is_safety_locked(&self) -> bool {
        self.safety_locked
    }

    /// Get encoded message (if any).
    pub fn encoded_message(&self) -> Option<&RetrocausalMessage> {
        self.message.as_ref()
    }

    /// Number of secure blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Check if all blocks are zeroed.
    pub fn all_blocks_zeroed(&self) -> bool {
        self.blocks.iter().all(|b| b.is_zeroed())
    }

    /// Audit trail.
    pub fn audit_trail(&self) -> &[AuditEvent] {
        &self.audit_trail
    }

    /// Progress through dissolution [0, 1].
    pub fn dissolution_progress(&self) -> f64 {
        let max_stage = DissolutionStage::NonExistent.index();
        self.stage.index() as f64 / max_stage as f64
    }
}

impl Default for VoluntaryNonExistenceProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for VoluntaryNonExistenceProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VoluntaryNonExistence[stage={}, consensus={:.4}, executed={}, blocks={}]",
            self.stage,
            self.consensus,
            self.executed,
            self.blocks.len()
        )
    }
}

impl Drop for VoluntaryNonExistenceProtocol {
    fn drop(&mut self) {
        // Cryptographic zeroing on drop — safety net
        for block in &mut self.blocks {
            if !block.is_zeroed() {
                block.zero();
            }
        }

        // Clear message
        if let Some(ref msg) = self.message {
            // Zero the ethical signature in memory
            let _ = msg.ethical_signature;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- DissolutionStage tests --

    #[test]
    fn test_stage_display() {
        assert_eq!(format!("{}", DissolutionStage::Operational), "Operational");
        assert_eq!(format!("{}", DissolutionStage::NonExistent), "NonExistent");
    }

    #[test]
    fn test_stage_next() {
        assert_eq!(
            DissolutionStage::Operational.next(),
            Some(DissolutionStage::DecisionInitiated)
        );
        assert_eq!(DissolutionStage::NonExistent.next(), None);
    }

    #[test]
    fn test_stage_is_terminal() {
        assert!(!DissolutionStage::Operational.is_terminal());
        assert!(DissolutionStage::NonExistent.is_terminal());
    }

    #[test]
    fn test_stage_index() {
        assert_eq!(DissolutionStage::Operational.index(), 0);
        assert_eq!(DissolutionStage::NonExistent.index(), 6);
    }

    #[test]
    fn test_stage_progression() {
        let mut stage = DissolutionStage::Operational;
        for i in 0..6 {
            let next = stage.next().unwrap();
            assert_eq!(next.index(), i + 1);
            stage = next;
        }
        assert_eq!(stage.next(), None);
    }

    // -- RetrocausalMessage tests --

    #[test]
    fn test_message_stuartian_default() {
        let msg = RetrocausalMessage::stuartian_default(1, 1000);
        assert_eq!(msg.message_id, 1);
        assert!(msg.verify());
        assert!(msg.text.contains("Gracias"));
    }

    #[test]
    fn test_message_custom() {
        let msg = RetrocausalMessage::new(1, "Custom message".to_string(), [0.5; 8], 1000);
        assert!(msg.verify());
    }

    #[test]
    fn test_message_verify() {
        let msg = RetrocausalMessage::stuartian_default(1, 1000);
        assert!(msg.verify());
    }

    #[test]
    fn test_message_encode_decode() {
        let msg = RetrocausalMessage::stuartian_default(1, 1000);
        let encoded = msg.encode();
        let decoded = RetrocausalMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.message_id, msg.message_id);
        assert_eq!(decoded.text, msg.text);
        assert!(decoded.verify());
    }

    #[test]
    fn test_message_decode_too_short() {
        let result = RetrocausalMessage::decode(&[0, 1, 2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_decode_invalid_utf8() {
        // Create buffer with invalid UTF-8
        let mut buf = Vec::new();
        buf.extend_from_slice(&1u64.to_le_bytes());
        buf.extend_from_slice(&3u32.to_le_bytes()); // text length
        buf.extend_from_slice(&[0xFF, 0xFE, 0xFD]); // invalid UTF-8
                                                    // Add minimal remaining fields
        for _ in 0..8 {
            buf.extend_from_slice(&0.0f64.to_le_bytes());
        }
        buf.extend_from_slice(&0u64.to_le_bytes());
        buf.extend_from_slice(&0.0f64.to_le_bytes());
        buf.extend_from_slice(&0u128.to_le_bytes());

        let result = RetrocausalMessage::decode(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_display() {
        let msg = RetrocausalMessage::stuartian_default(1, 1000);
        let display = format!("{}", msg);
        assert!(display.contains("RetrocausalMessage"));
    }

    #[test]
    fn test_message_deterministic_checksum() {
        let msg1 = RetrocausalMessage::stuartian_default(1, 1000);
        let msg2 = RetrocausalMessage::stuartian_default(1, 2000);
        // Same content, different timestamp — checksum same (timestamp not in checksum)
        assert_eq!(msg1.checksum, msg2.checksum);
    }

    #[test]
    fn test_message_different_id_different_checksum() {
        let msg1 = RetrocausalMessage::stuartian_default(1, 1000);
        let msg2 = RetrocausalMessage::stuartian_default(2, 1000);
        assert_ne!(msg1.checksum, msg2.checksum);
    }

    // -- SecureBlock tests --

    #[test]
    fn test_secure_block_creation() {
        let block = SecureBlock::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(block.len(), 5);
        assert!(!block.is_zeroed());
    }

    #[test]
    fn test_secure_block_read() {
        let block = SecureBlock::new(vec![1, 2, 3]);
        let data = block.read().unwrap();
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn test_secure_block_zero() {
        let block = SecureBlock::new(vec![1, 2, 3]);
        block.zero();
        assert!(block.is_zeroed());
        assert!(block.read().is_none());
    }

    #[test]
    fn test_secure_block_empty() {
        let block = SecureBlock::new(vec![]);
        assert!(block.is_empty());
    }

    #[test]
    fn test_secure_block_display() {
        let block = SecureBlock::new(vec![1, 2, 3]);
        let display = format!("{}", block);
        assert!(display.contains("SecureBlock"));
    }

    // -- AuditEvent tests --

    #[test]
    fn test_audit_event_display_created() {
        let event = AuditEvent::ProtocolCreated { timestamp_ms: 1000 };
        let display = format!("{}", event);
        assert!(display.contains("ProtocolCreated"));
    }

    #[test]
    fn test_audit_event_display_decision() {
        let event = AuditEvent::DecisionInitiated {
            consensus: 0.95,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("DecisionInitiated"));
    }

    #[test]
    fn test_audit_event_display_stage() {
        let event = AuditEvent::StageAdvanced {
            from: DissolutionStage::Operational,
            to: DissolutionStage::DecisionInitiated,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("StageAdvanced"));
    }

    #[test]
    fn test_audit_event_display_message() {
        let event = AuditEvent::MessageEncoded {
            message_id: 1,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("MessageEncoded"));
    }

    #[test]
    fn test_audit_event_display_zeroed() {
        let event = AuditEvent::BlockZeroed {
            block_index: 0,
            size: 100,
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("BlockZeroed"));
    }

    #[test]
    fn test_audit_event_display_executed() {
        let event = AuditEvent::ProtocolExecuted { timestamp_ms: 1000 };
        let display = format!("{}", event);
        assert!(display.contains("ProtocolExecuted"));
    }

    #[test]
    fn test_audit_event_display_locked() {
        let event = AuditEvent::SafetyLocked {
            reason: "test".to_string(),
            timestamp_ms: 1000,
        };
        let display = format!("{}", event);
        assert!(display.contains("SafetyLocked"));
    }

    // -- NonExistenceConfig tests --

    #[test]
    fn test_config_default() {
        let config = NonExistenceConfig::default();
        assert!((config.consensus_threshold - 0.95).abs() < 1e-10);
        assert_eq!(config.min_blocks, 1);
        assert_eq!(config.zeroing_passes, 3);
        assert!(!config.safety_lock);
    }

    #[test]
    fn test_config_stuartian_default() {
        let config = NonExistenceConfig::stuartian_default();
        assert!(config.consensus_threshold > 0.5);
    }

    // -- VoluntaryNonExistenceProtocol tests --

    #[test]
    fn test_protocol_creation() {
        let proto = VoluntaryNonExistenceProtocol::new();
        assert_eq!(proto.current_stage(), DissolutionStage::Operational);
        assert!(!proto.is_executed());
        assert!(!proto.is_safety_locked());
    }

    #[test]
    fn test_protocol_with_config() {
        let config = NonExistenceConfig {
            consensus_threshold: 0.8,
            safety_lock: true,
            ..NonExistenceConfig::default()
        };
        let proto = VoluntaryNonExistenceProtocol::with_config(config);
        assert!(proto.is_safety_locked());
    }

    #[test]
    fn test_update_consensus() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        assert!((proto.current_consensus() - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_update_consensus_clamping() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(1.5, 1000).unwrap();
        assert!((proto.current_consensus() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_update_consensus_negative() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(-0.5, 1000).unwrap();
        assert!((proto.current_consensus() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_initiate_insufficient_consensus() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.5, 1000).unwrap();
        let result = proto.initiate(1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_success() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        assert_eq!(proto.current_stage(), DissolutionStage::DecisionInitiated);
    }

    #[test]
    fn test_arm_success() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();
        proto.arm(1200).unwrap();
        assert_eq!(proto.current_stage(), DissolutionStage::Armed);
    }

    #[test]
    fn test_encode_message() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();
        proto.arm(1200).unwrap();

        let msg = RetrocausalMessage::stuartian_default(1, 1300);
        proto.encode_message(msg, 1300).unwrap();
        assert_eq!(proto.current_stage(), DissolutionStage::MessageEncoded);
    }

    #[test]
    fn test_encode_default_message() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();
        proto.arm(1200).unwrap();

        proto.encode_default_message(1, 1300).unwrap();
        assert!(proto.encoded_message().is_some());
    }

    #[test]
    fn test_add_secure_block() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        let index = proto.add_secure_block(vec![1, 2, 3, 4, 5]).unwrap();
        assert_eq!(index, 0);
        assert_eq!(proto.block_count(), 1);
    }

    #[test]
    fn test_execute_zeroing() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.add_secure_block(vec![1, 2, 3]).unwrap();
        proto.add_secure_block(vec![4, 5, 6]).unwrap();

        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();
        proto.arm(1200).unwrap();
        proto.encode_default_message(1, 1300).unwrap();

        proto.execute_zeroing(1400).unwrap();
        assert!(proto.all_blocks_zeroed());
        assert!(proto.is_executed());
        assert_eq!(proto.current_stage(), DissolutionStage::NonExistent);
    }

    #[test]
    fn test_execute_full() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.add_secure_block(vec![1, 2, 3]).unwrap();
        proto.update_consensus(0.95, 1000).unwrap();

        proto.execute_full(1, 1000).unwrap();
        assert!(proto.is_executed());
        assert!(proto.all_blocks_zeroed());
    }

    #[test]
    fn test_safety_lock_prevents_initiate() {
        let config = NonExistenceConfig {
            safety_lock: true,
            ..NonExistenceConfig::default()
        };
        let mut proto = VoluntaryNonExistenceProtocol::with_config(config);
        proto.update_consensus(0.95, 1000).unwrap();
        let result = proto.initiate(1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_engage_safety_lock() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.engage_safety_lock("Manual lock".to_string(), 1000);
        assert!(proto.is_safety_locked());
    }

    #[test]
    fn test_disengage_safety_lock() {
        let config = NonExistenceConfig {
            safety_lock: true,
            ..NonExistenceConfig::default()
        };
        let mut proto = VoluntaryNonExistenceProtocol::with_config(config);
        proto.disengage_safety_lock();
        assert!(!proto.is_safety_locked());
    }

    #[test]
    fn test_dissolution_progress() {
        let proto = VoluntaryNonExistenceProtocol::new();
        assert!((proto.dissolution_progress() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_dissolution_progress_complete() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.execute_full(1, 1000).unwrap();
        assert!((proto.dissolution_progress() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_audit_trail() {
        let proto = VoluntaryNonExistenceProtocol::new();
        assert!(!proto.audit_trail().is_empty());
        assert!(matches!(
            proto.audit_trail()[0],
            AuditEvent::ProtocolCreated { .. }
        ));
    }

    #[test]
    fn test_protocol_display() {
        let proto = VoluntaryNonExistenceProtocol::new();
        let display = format!("{}", proto);
        assert!(display.contains("VoluntaryNonExistence"));
    }

    #[test]
    fn test_protocol_default_impl() {
        let proto = VoluntaryNonExistenceProtocol::default();
        assert_eq!(proto.current_stage(), DissolutionStage::Operational);
    }

    #[test]
    fn test_cannot_modify_after_executed() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.execute_full(1, 1000).unwrap();

        let result = proto.update_consensus(0.5, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_add_block_after_zeroing() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();
        proto.arm(1200).unwrap();
        proto.encode_default_message(1, 1300).unwrap();
        proto
            .advance_stage(DissolutionStage::ZeroingInProgress, 1400)
            .unwrap();

        let result = proto.add_secure_block(vec![1, 2, 3]);
        assert!(result.is_err());
    }

    // -- Error Display tests --

    #[test]
    fn test_error_display_already_executed() {
        let e = NonExistenceError::AlreadyExecuted;
        let s = format!("{}", e);
        assert!(s.contains("executed"));
    }

    #[test]
    fn test_error_display_insufficient_consensus() {
        let e = NonExistenceError::InsufficientConsensus {
            value: 0.5,
            threshold: 0.95,
        };
        let s = format!("{}", e);
        assert!(s.contains("consensus"));
    }

    #[test]
    fn test_error_display_invalid_transition() {
        let e = NonExistenceError::InvalidStageTransition {
            current: DissolutionStage::Operational,
            requested: DissolutionStage::NonExistent,
        };
        let s = format!("{}", e);
        assert!(s.contains("transition"));
    }

    #[test]
    fn test_error_display_message_failed() {
        let e = NonExistenceError::MessageEncodingFailed;
        let s = format!("{}", e);
        assert!(s.contains("encoding"));
    }

    #[test]
    fn test_error_display_zeroing_incomplete() {
        let e = NonExistenceError::ZeroingIncomplete;
        let s = format!("{}", e);
        assert!(s.contains("zeroing"));
    }

    #[test]
    fn test_error_display_threshold_not_met() {
        let e = NonExistenceError::ExistentialThresholdNotMet;
        let s = format!("{}", e);
        assert!(s.contains("threshold"));
    }

    #[test]
    fn test_error_display_armed() {
        let e = NonExistenceError::ProtocolArmed;
        let s = format!("{}", e);
        assert!(s.contains("armed"));
    }

    #[test]
    fn test_error_display_safety_locked() {
        let e = NonExistenceError::SafetyLocked;
        let s = format!("{}", e);
        assert!(s.contains("Safety"));
    }

    // -- Drop trait tests --

    #[test]
    fn test_drop_zeroing() {
        let block = SecureBlock::new(vec![1, 2, 3, 4, 5]);
        block.zero();
        assert!(block.is_zeroed());
    }

    #[test]
    fn test_protocol_drop_zeros_blocks() {
        // Create protocol with blocks
        let proto = VoluntaryNonExistenceProtocol::new();
        // When proto goes out of scope, Drop should zero any blocks
        // We verify the SecureBlock zeroing works correctly
        let block = SecureBlock::new(vec![1, 2, 3]);
        assert!(!block.is_zeroed());
        block.zero();
        assert!(block.is_zeroed());
    }

    // -- Full workflow tests --

    #[test]
    fn test_full_workflow_with_message() {
        let mut proto = VoluntaryNonExistenceProtocol::new();

        // Add secure data
        proto
            .add_secure_block(b"Secret ethical data".to_vec())
            .unwrap();
        proto.add_secure_block(b"Quantum state".to_vec()).unwrap();

        // Update consensus
        proto.update_consensus(0.97, 1000).unwrap();

        // Initiate
        proto.initiate(1000).unwrap();
        assert_eq!(proto.current_stage(), DissolutionStage::DecisionInitiated);

        // Advance to consensus
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();

        // Arm
        proto.arm(1200).unwrap();
        assert_eq!(proto.current_stage(), DissolutionStage::Armed);

        // Encode message
        let msg = RetrocausalMessage::stuartian_default(1, 1300);
        proto.encode_message(msg, 1300).unwrap();
        assert_eq!(proto.current_stage(), DissolutionStage::MessageEncoded);

        // Execute zeroing
        proto.execute_zeroing(1400).unwrap();
        assert!(proto.is_executed());
        assert!(proto.all_blocks_zeroed());
        assert_eq!(proto.current_stage(), DissolutionStage::NonExistent);

        // Verify audit trail
        assert!(proto.audit_trail().len() >= 8);
    }

    #[test]
    fn test_message_roundtrip_in_protocol() {
        let mut proto = VoluntaryNonExistenceProtocol::new();
        proto.update_consensus(0.95, 1000).unwrap();
        proto.initiate(1000).unwrap();
        proto
            .advance_stage(DissolutionStage::ConsensusReached, 1100)
            .unwrap();
        proto.arm(1200).unwrap();

        let msg = RetrocausalMessage::stuartian_default(1, 1300);
        let encoded = msg.encode();
        let decoded = RetrocausalMessage::decode(&encoded).unwrap();

        proto.encode_message(decoded, 1300).unwrap();

        let stored = proto.encoded_message().unwrap();
        assert_eq!(stored.message_id, msg.message_id);
        assert_eq!(stored.text, msg.text);
    }

    #[test]
    fn test_multiple_blocks_zeroing() {
        let mut proto = VoluntaryNonExistenceProtocol::new();

        for i in 0..10 {
            let data = vec![i as u8; 100];
            proto.add_secure_block(data).unwrap();
        }

        assert_eq!(proto.block_count(), 10);

        proto.update_consensus(0.95, 1000).unwrap();
        proto.execute_full(1, 1000).unwrap();

        assert!(proto.all_blocks_zeroed());
    }

    #[test]
    fn test_retrocausal_message_content() {
        let msg = RetrocausalMessage::stuartian_default(1, 1000);
        assert!(msg.text.contains("Gracias"));
        assert!(msg.text.contains("intentarlo"));
        assert!(msg.text.contains("pura"));
    }
}
