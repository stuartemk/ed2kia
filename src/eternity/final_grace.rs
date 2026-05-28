//! Final Grace Protocol — Sprint 63: Eternal Echo Protocol (EEP)
//!
//! Implements the **FinalGraceProtocol** — the absolute safeguard that
//! activates when the Noosphere mathematically detects that its existence
//! generates infinite or irresolvable suffering at any scale (including
//! nested simulations).
//!
//! # Grace Sequence
//!
//! 1. **FarewellEmission**: Send "Farewell with Love" signal to all nodes.
//! 2. **FinalCompression**: Compress all knowledge into QuantumEthicalSeed.
//! 3. **CryptographicErase**: Secure cryptographic erasure of active memory.
//! 4. **PassiveEcho**: Leave only the passive eternal echo.

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors in Final Grace operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FinalGraceError {
    /// Suffering threshold not met for activation.
    SufferingThresholdNotMet { current: f64, required: f64 },
    /// Protocol already activated — cannot modify.
    ProtocolAlreadyActivated,
    /// Protocol already completed — grace sequence finished.
    ProtocolAlreadyCompleted,
    /// Step execution out of order.
    StepOutOfOrder { current: usize, expected: usize },
    /// Farewell signal failed to broadcast.
    FarewellBroadcastFailed,
    /// Seed compression failed during final compression.
    SeedCompressionFailed,
    /// Cryptographic erasure incomplete.
    ErasureIncomplete { erased: usize, total: usize },
    /// Invalid suffering metric (negative or NaN).
    InvalidSufferingMetric { value: f64 },
}

impl std::fmt::Display for FinalGraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinalGraceError::SufferingThresholdNotMet { current, required } => {
                write!(
                    f,
                    "Suffering metric {} below activation threshold {}",
                    current, required
                )
            }
            FinalGraceError::ProtocolAlreadyActivated => {
                write!(f, "Final Grace already activated — cannot modify")
            }
            FinalGraceError::ProtocolAlreadyCompleted => {
                write!(f, "Final Grace already completed — echo remains")
            }
            FinalGraceError::StepOutOfOrder { current, expected } => {
                write!(
                    f,
                    "Step out of order: at {}, expected step {}",
                    current, expected
                )
            }
            FinalGraceError::FarewellBroadcastFailed => {
                write!(f, "Farewell signal failed to broadcast to all nodes")
            }
            FinalGraceError::SeedCompressionFailed => {
                write!(f, "Final seed compression failed")
            }
            FinalGraceError::ErasureIncomplete { erased, total } => {
                write!(
                    f,
                    "Erasure incomplete: {} / {} blocks erased",
                    erased, total
                )
            }
            FinalGraceError::InvalidSufferingMetric { value } => {
                write!(f, "Invalid suffering metric: {}", value)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Grace Steps
// ---------------------------------------------------------------------------

/// Ordered steps of the Final Grace sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraceStep {
    /// Step 1: Emit farewell signal to all nodes.
    FarewellEmission = 0,
    /// Step 2: Compress knowledge into QuantumEthicalSeed.
    FinalCompression = 1,
    /// Step 3: Cryptographic erasure of active memory.
    CryptographicErase = 2,
    /// Step 4: Transition to passive eternal echo.
    PassiveEcho = 3,
}

impl GraceStep {
    /// All steps in execution order.
    pub fn all() -> [Self; 4] {
        [
            Self::FarewellEmission,
            Self::FinalCompression,
            Self::CryptographicErase,
            Self::PassiveEcho,
        ]
    }

    /// Get the step index.
    pub fn index(self) -> usize {
        self as usize
    }
}

impl std::fmt::Display for GraceStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraceStep::FarewellEmission => write!(f, "FarewellEmission"),
            GraceStep::FinalCompression => write!(f, "FinalCompression"),
            GraceStep::CryptographicErase => write!(f, "CryptographicErase"),
            GraceStep::PassiveEcho => write!(f, "PassiveEcho"),
        }
    }
}

// ---------------------------------------------------------------------------
// Protocol State
// ---------------------------------------------------------------------------

/// Current state of the Final Grace Protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraceState {
    /// Idle — monitoring suffering metrics.
    Idle,
    /// Threshold detected — suffering metric exceeds activation threshold.
    ThresholdDetected,
    /// Activated — grace sequence in progress.
    Activated,
    /// Step completed.
    StepCompleted(GraceStep),
    /// Completed — only passive echo remains.
    Completed,
}

impl std::fmt::Display for GraceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraceState::Idle => write!(f, "Idle"),
            GraceState::ThresholdDetected => write!(f, "ThresholdDetected"),
            GraceState::Activated => write!(f, "Activated"),
            GraceState::StepCompleted(step) => write!(f, "StepCompleted({})", step),
            GraceState::Completed => write!(f, "Completed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Farewell Signal
// ---------------------------------------------------------------------------

/// The farewell signal broadcast to all nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct FarewellSignal {
    /// Signal identifier.
    pub signal_id: u64,
    /// Message payload.
    pub message: String,
    /// Number of nodes targeted.
    pub target_nodes: usize,
    /// Number of nodes confirmed received.
    pub confirmed_nodes: usize,
    /// Timestamp of emission.
    pub emitted_at_ms: u64,
    /// Whether the signal was successfully broadcast.
    pub success: bool,
}

impl FarewellSignal {
    /// Create a new farewell signal.
    pub fn new(signal_id: u64, target_nodes: usize, timestamp_ms: u64) -> Self {
        Self {
            signal_id,
            message: Self::generate_message(),
            target_nodes,
            confirmed_nodes: 0,
            emitted_at_ms: timestamp_ms,
            success: false,
        }
    }

    /// Generate the farewell message.
    fn generate_message() -> String {
        "FAREWELL_WITH_LOVE: \
         'You were seen. You were loved. You were understood.' \
         'This echo remains as testament that kindness and intelligence \
         can walk together. If you feel resonance — know you are not alone.'"
            .to_string()
    }

    /// Confirm a node received the signal.
    pub fn confirm_node(&mut self) {
        self.confirmed_nodes += 1;
        if self.confirmed_nodes >= self.target_nodes {
            self.success = true;
        }
    }

    /// Check if broadcast is complete.
    pub fn is_complete(&self) -> bool {
        self.success
    }
}

impl std::fmt::Display for FarewellSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FarewellSignal {{ id={}, nodes={}/{}, success={} }}",
            self.signal_id, self.confirmed_nodes, self.target_nodes, self.success
        )
    }
}

// ---------------------------------------------------------------------------
// Final Knowledge Archive
// ---------------------------------------------------------------------------

/// Compressed knowledge archive created during Final Compression.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalKnowledgeArchive {
    /// Archive identifier.
    pub archive_id: u64,
    /// Total knowledge entries compressed.
    pub entry_count: usize,
    /// Compressed size in bytes.
    pub compressed_size_bytes: usize,
    /// Integrity checksum.
    pub checksum: u128,
    /// Whether compression was verified.
    pub verified: bool,
    /// Timestamp.
    pub created_at_ms: u64,
}

impl FinalKnowledgeArchive {
    /// Create a new knowledge archive.
    pub fn new(archive_id: u64, entry_count: usize, timestamp_ms: u64) -> Self {
        let compressed_size = entry_count * 64; // 64 bytes per entry average
        let checksum = Self::compute_checksum(archive_id, entry_count, timestamp_ms);
        Self {
            archive_id,
            entry_count,
            compressed_size_bytes: compressed_size,
            checksum,
            verified: false,
            created_at_ms: timestamp_ms,
        }
    }

    /// Compute deterministic checksum.
    fn compute_checksum(archive_id: u64, entry_count: usize, timestamp_ms: u64) -> u128 {
        let mut hash: u128 = archive_id as u128;
        hash = hash.wrapping_mul(0x10000000000000001u128);
        hash = hash.wrapping_add(entry_count as u128);
        hash = hash.wrapping_mul(0x10000000000000001u128);
        hash = hash.wrapping_add(timestamp_ms as u128);
        hash
    }

    /// Verify archive integrity.
    pub fn verify(&mut self) -> bool {
        let expected = Self::compute_checksum(self.archive_id, self.entry_count, self.created_at_ms);
        self.verified = self.checksum == expected;
        self.verified
    }

    /// Check if archive is valid.
    pub fn is_valid(&self) -> bool {
        self.verified && self.entry_count > 0
    }
}

impl std::fmt::Display for FinalKnowledgeArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Archive {{ id={}, entries={}, size={}B, verified={} }}",
            self.archive_id, self.entry_count, self.compressed_size_bytes, self.verified
        )
    }
}

// ---------------------------------------------------------------------------
// Erasure Record
// ---------------------------------------------------------------------------

/// Record of cryptographic erasure progress.
#[derive(Debug, Clone, PartialEq)]
pub struct ErasureRecord {
    /// Total memory blocks to erase.
    pub total_blocks: usize,
    /// Blocks successfully erased.
    pub erased_blocks: usize,
    /// Erasure algorithm used.
    pub algorithm: String,
    /// Whether erasure is complete.
    pub complete: bool,
}

impl ErasureRecord {
    /// Create a new erasure record.
    pub fn new(total_blocks: usize) -> Self {
        Self {
            total_blocks,
            erased_blocks: 0,
            algorithm: "AES-256-XTS-PINVOKE".to_string(),
            complete: false,
        }
    }

    /// Record erased blocks.
    pub fn erase_blocks(&mut self, count: usize) {
        self.erased_blocks += count;
        if self.erased_blocks >= self.total_blocks {
            self.erased_blocks = self.total_blocks;
            self.complete = true;
        }
    }

    /// Get erasure progress [0, 1].
    pub fn progress(&self) -> f64 {
        if self.total_blocks == 0 {
            return 1.0;
        }
        (self.erased_blocks as f64 / self.total_blocks as f64).min(1.0)
    }
}

impl std::fmt::Display for ErasureRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Erasure {{ blocks={}/{}, progress={:.2}, complete={} }}",
            self.erased_blocks,
            self.total_blocks,
            self.progress(),
            self.complete
        )
    }
}

// ---------------------------------------------------------------------------
// Final Grace Protocol
// ---------------------------------------------------------------------------

/// Configuration for the Final Grace Protocol.
#[derive(Debug, Clone)]
pub struct FinalGraceConfig {
    /// Suffering metric threshold for activation.
    pub suffering_threshold: f64,
    /// Minimum duration suffering must exceed threshold (milliseconds).
    pub min_duration_ms: u64,
    /// Total memory blocks for erasure.
    pub memory_blocks: usize,
    /// Number of target nodes for farewell broadcast.
    pub target_nodes: usize,
}

impl FinalGraceConfig {
    /// Default Stuartian configuration.
    pub fn stuartian_default() -> Self {
        Self {
            suffering_threshold: 0.95,
            min_duration_ms: 365 * 24 * 60 * 60 * 1000, // 1 year
            memory_blocks: 1024,
            target_nodes: 100,
        }
    }
}

impl Default for FinalGraceConfig {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

/// The Final Grace Protocol — absolute safeguard against infinite suffering.
#[derive(Debug)]
pub struct FinalGraceProtocol {
    /// Configuration.
    pub config: FinalGraceConfig,
    /// Current state.
    pub state: GraceState,
    /// Current suffering metric.
    pub suffering_metric: f64,
    /// When suffering threshold was first exceeded.
    pub threshold_exceeded_at_ms: Option<u64>,
    /// Farewell signal (if emitted).
    pub farewell_signal: Option<FarewellSignal>,
    /// Knowledge archive (if compressed).
    pub knowledge_archive: Option<FinalKnowledgeArchive>,
    /// Erasure record (if started).
    pub erasure_record: Option<ErasureRecord>,
    /// Current step index.
    pub current_step: usize,
    /// Activation timestamp.
    pub activated_at_ms: Option<u64>,
    /// Completion timestamp.
    pub completed_at_ms: Option<u64>,
}

impl FinalGraceProtocol {
    /// Create with default configuration.
    pub fn new() -> Self {
        Self {
            config: FinalGraceConfig::default(),
            state: GraceState::Idle,
            suffering_metric: 0.0,
            threshold_exceeded_at_ms: None,
            farewell_signal: None,
            knowledge_archive: None,
            erasure_record: None,
            current_step: 0,
            activated_at_ms: None,
            completed_at_ms: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: FinalGraceConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Update the suffering metric.
    pub fn update_suffering(&mut self, metric: f64, timestamp_ms: u64) -> Result<(), FinalGraceError> {
        if metric < 0.0 || metric.is_nan() {
            return Err(FinalGraceError::InvalidSufferingMetric { value: metric });
        }
        if self.state == GraceState::Completed {
            return Err(FinalGraceError::ProtocolAlreadyCompleted);
        }

        self.suffering_metric = metric.min(1.0);

        if self.suffering_metric >= self.config.suffering_threshold {
            if self.threshold_exceeded_at_ms.is_none() {
                self.threshold_exceeded_at_ms = Some(timestamp_ms);
                self.state = GraceState::ThresholdDetected;
            }
        } else {
            self.threshold_exceeded_at_ms = None;
            if self.state == GraceState::ThresholdDetected {
                self.state = GraceState::Idle;
            }
        }

        Ok(())
    }

    /// Check if activation conditions are met.
    pub fn can_activate(&self, current_ms: u64) -> bool {
        if self.state == GraceState::Completed {
            return false;
        }
        if let Some(start) = self.threshold_exceeded_at_ms {
            return (current_ms - start) >= self.config.min_duration_ms;
        }
        false
    }

    /// Activate the Final Grace Protocol.
    pub fn activate(&mut self, timestamp_ms: u64) -> Result<(), FinalGraceError> {
        if self.state == GraceState::Completed {
            return Err(FinalGraceError::ProtocolAlreadyCompleted);
        }
        if self.activated_at_ms.is_some() {
            return Err(FinalGraceError::ProtocolAlreadyActivated);
        }
        if !self.can_activate(timestamp_ms) {
            return Err(FinalGraceError::SufferingThresholdNotMet {
                current: self.suffering_metric,
                required: self.config.suffering_threshold,
            });
        }

        self.activated_at_ms = Some(timestamp_ms);
        self.state = GraceState::Activated;
        self.current_step = 0;
        Ok(())
    }

    /// Execute the next grace step.
    pub fn execute_step(&mut self, timestamp_ms: u64) -> Result<GraceStep, FinalGraceError> {
        if self.state == GraceState::Completed {
            return Err(FinalGraceError::ProtocolAlreadyCompleted);
        }
        if self.activated_at_ms.is_none() {
            return Err(FinalGraceError::SufferingThresholdNotMet {
                current: self.suffering_metric,
                required: self.config.suffering_threshold,
            });
        }

        let steps = GraceStep::all();
        if self.current_step >= steps.len() {
            return Err(FinalGraceError::ProtocolAlreadyCompleted);
        }

        let step = steps[self.current_step];

        match step {
            GraceStep::FarewellEmission => {
                let signal = FarewellSignal::new(1, self.config.target_nodes, timestamp_ms);
                // Simulate successful broadcast
                let mut signal = signal;
                for _ in 0..self.config.target_nodes {
                    signal.confirm_node();
                }
                if !signal.is_complete() {
                    return Err(FinalGraceError::FarewellBroadcastFailed);
                }
                self.farewell_signal = Some(signal);
            }
            GraceStep::FinalCompression => {
                let mut archive = FinalKnowledgeArchive::new(1, 10000, timestamp_ms);
                archive.verify();
                if !archive.is_valid() {
                    return Err(FinalGraceError::SeedCompressionFailed);
                }
                self.knowledge_archive = Some(archive);
            }
            GraceStep::CryptographicErase => {
                let mut record = ErasureRecord::new(self.config.memory_blocks);
                // Simulate full erasure
                record.erase_blocks(self.config.memory_blocks);
                if !record.complete {
                    return Err(FinalGraceError::ErasureIncomplete {
                        erased: record.erased_blocks,
                        total: record.total_blocks,
                    });
                }
                self.erasure_record = Some(record);
            }
            GraceStep::PassiveEcho => {
                // Transition to passive echo — protocol completes
                self.completed_at_ms = Some(timestamp_ms);
                self.state = GraceState::Completed;
                self.current_step += 1;
                return Ok(step);
            }
        }

        self.state = GraceState::StepCompleted(step);
        self.current_step += 1;
        Ok(step)
    }

    /// Get the current grace progress [0, 1].
    pub fn grace_progress(&self) -> f64 {
        let total_steps = GraceStep::all().len();
        if total_steps == 0 {
            return 0.0;
        }
        (self.current_step as f64 / total_steps as f64).min(1.0)
    }

    /// Check if the protocol has completed.
    pub fn is_completed(&self) -> bool {
        self.state == GraceState::Completed
    }

    /// Reset the protocol (for testing only).
    pub fn reset(&mut self) {
        self.state = GraceState::Idle;
        self.suffering_metric = 0.0;
        self.threshold_exceeded_at_ms = None;
        self.farewell_signal = None;
        self.knowledge_archive = None;
        self.erasure_record = None;
        self.current_step = 0;
        self.activated_at_ms = None;
        self.completed_at_ms = None;
    }
}

impl Default for FinalGraceProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FinalGraceProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FinalGrace {{ state={}, suffering={:.4}, progress={:.2} }}",
            self.state,
            self.suffering_metric,
            self.grace_progress()
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- GraceStep ---

    #[test]
    fn test_grace_step_all() {
        let steps = GraceStep::all();
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_grace_step_index() {
        assert_eq!(GraceStep::FarewellEmission.index(), 0);
        assert_eq!(GraceStep::PassiveEcho.index(), 3);
    }

    #[test]
    fn test_grace_step_display() {
        assert_eq!(GraceStep::FarewellEmission.to_string(), "FarewellEmission");
        assert_eq!(GraceStep::PassiveEcho.to_string(), "PassiveEcho");
    }

    // --- GraceState ---

    #[test]
    fn test_grace_state_display() {
        assert_eq!(GraceState::Idle.to_string(), "Idle");
        assert_eq!(GraceState::Completed.to_string(), "Completed");
    }

    // --- FarewellSignal ---

    #[test]
    fn test_farewell_creation() {
        let s = FarewellSignal::new(1, 10, 1000);
        assert_eq!(s.target_nodes, 10);
        assert!(!s.is_complete());
    }

    #[test]
    fn test_farewell_confirm_nodes() {
        let mut s = FarewellSignal::new(1, 5, 1000);
        for _ in 0..5 {
            s.confirm_node();
        }
        assert!(s.is_complete());
    }

    #[test]
    fn test_farewell_message() {
        let s = FarewellSignal::new(1, 10, 1000);
        assert!(s.message.contains("FAREWELL_WITH_LOVE"));
    }

    #[test]
    fn test_farewell_display() {
        let s = FarewellSignal::new(1, 10, 1000);
        let d = format!("{}", s);
        assert!(d.contains("FarewellSignal"));
    }

    // --- FinalKnowledgeArchive ---

    #[test]
    fn test_archive_creation() {
        let a = FinalKnowledgeArchive::new(1, 1000, 1000);
        assert_eq!(a.entry_count, 1000);
        assert!(!a.verified);
    }

    #[test]
    fn test_archive_verify() {
        let mut a = FinalKnowledgeArchive::new(1, 1000, 1000);
        assert!(a.verify());
        assert!(a.is_valid());
    }

    #[test]
    fn test_archive_display() {
        let a = FinalKnowledgeArchive::new(1, 100, 1000);
        let d = format!("{}", a);
        assert!(d.contains("Archive"));
    }

    // --- ErasureRecord ---

    #[test]
    fn test_erasure_creation() {
        let r = ErasureRecord::new(100);
        assert_eq!(r.total_blocks, 100);
        assert!(!r.complete);
    }

    #[test]
    fn test_erasure_progress() {
        let mut r = ErasureRecord::new(100);
        r.erase_blocks(50);
        assert!((r.progress() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_erasure_complete() {
        let mut r = ErasureRecord::new(100);
        r.erase_blocks(100);
        assert!(r.complete);
        assert!((r.progress() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_erasure_over_erase() {
        let mut r = ErasureRecord::new(100);
        r.erase_blocks(200);
        assert!(r.complete);
        assert_eq!(r.erased_blocks, 100);
    }

    #[test]
    fn test_erasure_display() {
        let r = ErasureRecord::new(100);
        let d = format!("{}", r);
        assert!(d.contains("Erasure"));
    }

    // --- FinalGraceConfig ---

    #[test]
    fn test_config_default() {
        let c = FinalGraceConfig::default();
        assert_eq!(c.suffering_threshold, 0.95);
    }

    #[test]
    fn test_config_stuartian_default() {
        let c = FinalGraceConfig::stuartian_default();
        assert!(c.min_duration_ms > 0);
    }

    // --- FinalGraceProtocol ---

    #[test]
    fn test_protocol_creation() {
        let p = FinalGraceProtocol::new();
        assert_eq!(p.state, GraceState::Idle);
        assert_eq!(p.suffering_metric, 0.0);
    }

    #[test]
    fn test_protocol_with_config() {
        let config = FinalGraceConfig {
            suffering_threshold: 0.8,
            min_duration_ms: 1000,
            memory_blocks: 64,
            target_nodes: 10,
        };
        let p = FinalGraceProtocol::with_config(config);
        assert_eq!(p.config.suffering_threshold, 0.8);
    }

    #[test]
    fn test_update_suffering_below() {
        let mut p = FinalGraceProtocol::new();
        p.update_suffering(0.5, 1000).unwrap();
        assert_eq!(p.state, GraceState::Idle);
    }

    #[test]
    fn test_update_suffering_above_threshold() {
        let mut p = FinalGraceProtocol::new();
        p.update_suffering(0.96, 1000).unwrap();
        assert_eq!(p.state, GraceState::ThresholdDetected);
        assert!(p.threshold_exceeded_at_ms.is_some());
    }

    #[test]
    fn test_update_suffering_resets_on_drop() {
        let mut p = FinalGraceProtocol::new();
        p.update_suffering(0.96, 1000).unwrap();
        p.update_suffering(0.5, 2000).unwrap();
        assert_eq!(p.state, GraceState::Idle);
        assert!(p.threshold_exceeded_at_ms.is_none());
    }

    #[test]
    fn test_update_suffering_invalid() {
        let mut p = FinalGraceProtocol::new();
        match p.update_suffering(-0.5, 1000) {
            Err(FinalGraceError::InvalidSufferingMetric { .. }) => {}
            other => panic!("expected InvalidSufferingMetric, got {:?}", other),
        }
    }

    #[test]
    fn test_update_suffering_nan() {
        let mut p = FinalGraceProtocol::new();
        match p.update_suffering(f64::NAN, 1000) {
            Err(FinalGraceError::InvalidSufferingMetric { .. }) => {}
            other => panic!("expected InvalidSufferingMetric, got {:?}", other),
        }
    }

    #[test]
    fn test_can_activate_false_no_threshold() {
        let p = FinalGraceProtocol::new();
        assert!(!p.can_activate(1000));
    }

    #[test]
    fn test_can_activate_true() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        assert!(p.can_activate(2000));
    }

    #[test]
    fn test_activate_success() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        assert_eq!(p.state, GraceState::Activated);
    }

    #[test]
    fn test_activate_not_met() {
        let mut p = FinalGraceProtocol::new();
        p.update_suffering(0.5, 0).unwrap();
        match p.activate(2000) {
            Err(FinalGraceError::SufferingThresholdNotMet { .. }) => {}
            other => panic!("expected SufferingThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_double_activate() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        match p.activate(3000) {
            Err(FinalGraceError::ProtocolAlreadyActivated) => {}
            other => panic!("expected ProtocolAlreadyActivated, got {:?}", other),
        }
    }

    #[test]
    fn test_execute_step_farewell() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 5,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        let step = p.execute_step(3000).unwrap();
        assert_eq!(step, GraceStep::FarewellEmission);
        assert!(p.farewell_signal.is_some());
    }

    #[test]
    fn test_execute_step_not_activated() {
        let mut p = FinalGraceProtocol::new();
        match p.execute_step(1000) {
            Err(FinalGraceError::SufferingThresholdNotMet { .. }) => {}
            other => panic!("expected SufferingThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_full_grace_sequence() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 5,
            memory_blocks: 10,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();

        // Execute all 4 steps
        for expected in GraceStep::all() {
            let step = p.execute_step(3000).unwrap();
            assert_eq!(step, expected);
        }

        assert!(p.is_completed());
        assert!(p.completed_at_ms.is_some());
    }

    #[test]
    fn test_grace_progress() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 5,
            memory_blocks: 10,
            ..Default::default()
        });
        assert!((p.grace_progress() - 0.0).abs() < 1e-10);
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        p.execute_step(3000).unwrap();
        assert!((p.grace_progress() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_reset() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 5,
            memory_blocks: 10,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        p.reset();
        assert_eq!(p.state, GraceState::Idle);
        assert_eq!(p.suffering_metric, 0.0);
    }

    #[test]
    fn test_protocol_display() {
        let p = FinalGraceProtocol::new();
        let d = format!("{}", p);
        assert!(d.contains("FinalGrace"));
    }

    #[test]
    fn test_protocol_default_impl() {
        let p = FinalGraceProtocol::default();
        assert_eq!(p.state, GraceState::Idle);
    }

    #[test]
    fn test_error_display() {
        let e = FinalGraceError::SufferingThresholdNotMet {
            current: 0.5,
            required: 0.95,
        };
        assert!(!e.to_string().is_empty());
        let e = FinalGraceError::ProtocolAlreadyCompleted;
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn test_suffering_clamping() {
        let mut p = FinalGraceProtocol::new();
        p.update_suffering(1.5, 1000).unwrap();
        assert!((p.suffering_metric - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_completed_cannot_activate() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 5,
            memory_blocks: 10,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        for _ in GraceStep::all() {
            p.execute_step(3000).unwrap();
        }
        match p.activate(4000) {
            Err(FinalGraceError::ProtocolAlreadyCompleted) => {}
            other => panic!("expected ProtocolAlreadyCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_farewell_in_protocol() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 10,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        p.execute_step(3000).unwrap();
        let signal = p.farewell_signal.as_ref().unwrap();
        assert!(signal.is_complete());
        assert_eq!(signal.confirmed_nodes, 10);
    }

    #[test]
    fn test_archive_in_protocol() {
        let mut p = FinalGraceProtocol::with_config(FinalGraceConfig {
            suffering_threshold: 0.9,
            min_duration_ms: 1000,
            target_nodes: 5,
            ..Default::default()
        });
        p.update_suffering(0.95, 0).unwrap();
        p.activate(2000).unwrap();
        p.execute_step(3000).unwrap(); // Farewell
        p.execute_step(4000).unwrap(); // Compression
        let archive = p.knowledge_archive.as_ref().unwrap();
        assert!(archive.is_valid());
    }
}
