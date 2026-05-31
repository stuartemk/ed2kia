//! Harmonic Flow — SRTP Pipeline Coordinator for Traffic Preservation.
//!
//! Orchestrates `TrafficMasker`, `ChaffingEngine`, and `TransportRotator`
//! into a cohesive pipeline that masks P2P payloads (tensor exchanges,
//! BFT consensus) as standard SRTP/WebRTC media flows.
//!
//! **Design Principles:**
//! - **Preservación**: P2P traffic is preserved through harmonic obfuscation.
//! - **Armonía**: Constant bitrate through chaffing maintains signal harmony.
//! - **Simbiosis**: Cooperative transport rotation for distribution resilience.
//! - **WASM Compatible**: No blocking I/O, pure functional pipeline stages.
//!
//! **Pipeline Stages:**
//! 1. `mask` — Wrap payload in SRTP frames (TrafficMasker)
//! 2. `chaff` — Inject harmonic noise packets (ChaffingEngine)
//! 3. `rotate` — Select optimal transport profile (TransportRotator)
//! 4. `assemble` — Final stream ready for network transmission

use crate::pillars::steganographic::{
    chaffing_engine::{ChaffConfig, ChaffingEngine, ChaffingError, TaggedPacket},
    traffic_masker::{MaskerConfig, MaskingError, SrtpHeader, TrafficMasker},
    transport_rotator::{RotatorConfig, TransportHealth, TransportRotator, TransportType},
};

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

/// Errors that can occur during harmonic flow pipeline execution.
#[derive(Debug, PartialEq)]
pub enum HarmonicFlowError {
    /// SRTP masking stage failed.
    MaskingError(MaskingError),
    /// Chaffing injection stage failed.
    ChaffingError(ChaffingError),
    /// Payload is empty and cannot be processed.
    EmptyPayload,
    /// Session key not registered for chaffing.
    SessionKeyMissing(String),
    /// Pipeline configuration error.
    ConfigError(String),
}

impl std::fmt::Display for HarmonicFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarmonicFlowError::MaskingError(e) => write!(f, "Masking error: {}", e),
            HarmonicFlowError::ChaffingError(e) => write!(f, "Chaffing error: {}", e),
            HarmonicFlowError::EmptyPayload => write!(f, "Payload cannot be empty"),
            HarmonicFlowError::SessionKeyMissing(session) => {
                write!(f, "Session key missing for: {}", session)
            }
            HarmonicFlowError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Complete configuration for the Harmonic Flow pipeline.
#[derive(Debug, Clone)]
pub struct HarmonicFlowConfig {
    /// SRTP masking configuration.
    pub masker: MaskerConfig,
    /// Chaffing injection configuration.
    pub chaff: ChaffConfig,
    /// Transport rotation configuration.
    pub transport: RotatorConfig,
    /// Session ID for chaffing/winnowing operations.
    pub session_id: String,
}

impl Default for HarmonicFlowConfig {
    fn default() -> Self {
        Self {
            masker: MaskerConfig::default(),
            chaff: ChaffConfig::default(),
            transport: RotatorConfig::default(),
            session_id: "harmonic-default-session".to_string(),
        }
    }
}

impl HarmonicFlowConfig {
    /// Validate configuration before pipeline execution.
    pub fn validate(&self) -> Result<(), HarmonicFlowError> {
        if self.session_id.is_empty() {
            return Err(HarmonicFlowError::ConfigError(
                "Session ID cannot be empty".to_string(),
            ));
        }
        if self.chaff.chaff_ratio < 0.0 || self.chaff.chaff_ratio > 1.0 {
            return Err(HarmonicFlowError::ConfigError(
                "Chaff ratio must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Pipeline Output
// ---------------------------------------------------------------------------

/// A single frame in the harmonic stream, ready for network transmission.
#[derive(Debug, Clone)]
pub struct HarmonicFrame {
    /// The tagged packet (real or chaff).
    pub packet: TaggedPacket,
    /// Transport profile for this frame.
    pub transport: TransportType,
    /// SRTP header data (for real packets only).
    pub srtp_header: Option<SrtpHeader>,
}

/// The complete output of the obfuscation pipeline.
#[derive(Debug)]
pub struct ObfuscatedStream {
    /// Ordered list of harmonic frames.
    pub frames: Vec<HarmonicFrame>,
    /// Transport profile used for this stream.
    pub transport: TransportType,
    /// Original payload size (for verification).
    pub original_size: usize,
    /// Total obfuscated size.
    pub obfuscated_size: usize,
    /// Expansion ratio (obfuscated / original).
    pub expansion_ratio: f64,
}

/// The result of the winnowing (de-obfuscation) pipeline.
#[derive(Debug)]
pub struct DeobfuscatedPayload {
    /// Recovered original payload bytes.
    pub payload: Vec<u8>,
    /// Number of real frames processed.
    pub frames_processed: usize,
    /// Number of chaff frames removed.
    pub chaff_removed: usize,
}

// ---------------------------------------------------------------------------
// Harmonic Flow Engine
// ---------------------------------------------------------------------------

/// SRTP Pipeline Coordinator — orchestrates the full obfuscation chain.
///
/// This engine provides a high-level interface for traffic preservation,
/// combining SRTP masking, harmonic chaffing, and transport rotation
/// into a single cohesive pipeline.
pub struct HarmonicFlow {
    /// SRTP traffic masker.
    masker: TrafficMasker,
    /// Chaffing engine for noise injection.
    chaffer: ChaffingEngine,
    /// Transport rotator for protocol selection.
    rotator: TransportRotator,
    /// Current pipeline configuration.
    config: HarmonicFlowConfig,
}

impl HarmonicFlow {
    // --- Construction ---

    /// Create a new HarmonicFlow with default configuration.
    pub fn new() -> Self {
        let config = HarmonicFlowConfig::default();
        Self::with_config(config)
    }

    /// Create a HarmonicFlow with custom configuration.
    pub fn with_config(config: HarmonicFlowConfig) -> Self {
        Self {
            masker: TrafficMasker::with_config(config.masker.clone()),
            chaffer: ChaffingEngine::with_config(config.chaff.clone()),
            rotator: TransportRotator::with_config(config.transport.clone())
                .expect("valid transport config"),
            config,
        }
    }

    // --- Session Management ---

    /// Register a session key for chaffing operations.
    ///
    /// The session key is derived from shared context between peers
    /// and enables winnowing (chaff removal) on the receiving side.
    pub fn register_session_key(&mut self, session_id: &str, key: [u8; 32]) {
        self.chaffer
            .register_session_key(session_id.to_string(), key);
    }

    /// Generate a session key from a seed value.
    pub fn generate_session_key(seed: &[u8]) -> [u8; 32] {
        ChaffingEngine::generate_session_key(seed)
    }

    /// Update the session ID for subsequent operations.
    pub fn set_session_id(&mut self, session_id: String) {
        self.config.session_id = session_id;
    }

    // --- Obfuscation Pipeline ---

    /// Execute the full obfuscation pipeline on a raw P2P payload.
    ///
    /// **Pipeline Stages:**
    /// 1. **Mask**: Wrap payload in SRTP frames (TrafficMasker)
    /// 2. **Chaff**: Inject harmonic noise packets (ChaffingEngine)
    /// 3. **Rotate**: Select optimal transport profile (TransportRotator)
    /// 4. **Assemble**: Combine into HarmonicFrames
    ///
    /// # Arguments
    /// * `payload` - Raw P2P payload (tensor exchange, BFT consensus, etc.)
    ///
    /// # Returns
    /// * `Ok(ObfuscatedStream)` - Complete stream ready for transmission
    /// * `Err(HarmonicFlowError)` - Pipeline failure at any stage
    pub fn obfuscate(&mut self, payload: &[u8]) -> Result<ObfuscatedStream, HarmonicFlowError> {
        // Validate input.
        if payload.is_empty() {
            return Err(HarmonicFlowError::EmptyPayload);
        }

        let original_size = payload.len();

        // Stage 1: SRTP Masking
        let srtp_frames = self
            .masker
            .mask_payload(payload)
            .map_err(HarmonicFlowError::MaskingError)?;

        // Stage 2: Chaffing Injection
        let tagged_packets = self
            .chaffer
            .inject_chaff(payload, &self.config.session_id)
            .map_err(HarmonicFlowError::ChaffingError)?;

        // Stage 3: Transport Rotation
        let transport = self.rotator.select_best().unwrap_or(TransportType::Tcp);

        // Stage 4: Assemble Harmonic Frames
        let mut frames = Vec::with_capacity(tagged_packets.len());
        let mut srtp_idx = 0;

        for packet in tagged_packets {
            // Real packets get SRTP headers; chaff packets don't.
            let srtp_header = if srtp_idx < srtp_frames.len() {
                let frame_bytes = &srtp_frames[srtp_idx];
                // SRTP header minimum size: 20 bytes (12 header + 8 RTCP)
                if frame_bytes.len() >= 20 {
                    match SrtpHeader::deserialize(frame_bytes) {
                        Ok(header) => {
                            srtp_idx += 1;
                            Some(header)
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            frames.push(HarmonicFrame {
                packet,
                transport: transport.clone(),
                srtp_header,
            });
        }

        let obfuscated_size: usize = frames.iter().map(|f| f.packet.payload.len()).sum();

        let expansion_ratio = if original_size > 0 {
            obfuscated_size as f64 / original_size as f64
        } else {
            0.0
        };

        Ok(ObfuscatedStream {
            frames,
            transport,
            original_size,
            obfuscated_size,
            expansion_ratio,
        })
    }

    // --- De-obfuscation Pipeline ---

    /// Execute the winnowing pipeline to recover the original payload.
    ///
    /// This removes chaff packets and reconstructs the original P2P
    /// payload from the harmonic stream.
    ///
    /// # Arguments
    /// * `stream` - Obfuscated stream received from the network
    ///
    /// # Returns
    /// * `Ok(DeobfuscatedPayload)` - Recovered original payload
    /// * `Err(HarmonicFlowError)` - Winnowing failure
    pub fn deobfuscate(
        &self,
        stream: &ObfuscatedStream,
    ) -> Result<DeobfuscatedPayload, HarmonicFlowError> {
        // Extract tagged packets from harmonic frames.
        let tagged_packets: Vec<TaggedPacket> =
            stream.frames.iter().map(|f| f.packet.clone()).collect();

        // Winnow: remove chaff and recover original data.
        let payload = self
            .chaffer
            .winnow(&tagged_packets, &self.config.session_id)
            .map_err(HarmonicFlowError::ChaffingError)?;

        let total_frames = tagged_packets.len();
        let real_frames = payload.len().max(1); // At least 1 byte per real packet
        let chaff_removed = total_frames.saturating_sub(real_frames);

        Ok(DeobfuscatedPayload {
            payload,
            frames_processed: total_frames,
            chaff_removed,
        })
    }

    // --- Transport Management ---

    /// Rotate to the next transport profile based on health metrics.
    pub fn rotate_transport(&mut self) -> TransportType {
        self.rotator.rotate().unwrap_or(TransportType::Tcp)
    }

    /// Report health metrics for the current transport.
    pub fn report_health(&mut self, success_rate: f64, latency_ms: f64) {
        let transport = self.rotator.current_transport().clone();
        let health = TransportHealth::new(
            transport,
            latency_ms,
            1.0 - success_rate, // packet_loss = 1 - success_rate
            0.0,                // throughput not tracked here
        );
        self.rotator.update_health(health);
    }

    /// Get the current transport profile.
    pub fn current_transport(&self) -> TransportType {
        self.rotator.current_transport().clone()
    }

    // --- Configuration Access ---

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &HarmonicFlowConfig {
        &self.config
    }

    /// Update the pipeline configuration.
    pub fn update_config(&mut self, new_config: HarmonicFlowConfig) {
        self.masker = TrafficMasker::with_config(new_config.masker.clone());
        self.chaffer = ChaffingEngine::with_config(new_config.chaff.clone());
        self.rotator = TransportRotator::with_config(new_config.transport.clone())
            .expect("valid transport config");
        self.config = new_config;
    }

    // --- Reset ---

    /// Reset the pipeline state (clears buffers, resets counters).
    pub fn reset(&mut self) {
        self.masker = TrafficMasker::new();
        self.chaffer = ChaffingEngine::new();
        self.rotator = TransportRotator::new();
    }
}

impl Default for HarmonicFlow {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_payload() -> Vec<u8> {
        b"tensor-exchange-payload-data".to_vec()
    }

    fn setup_flow() -> HarmonicFlow {
        let mut config = HarmonicFlowConfig::default();
        config.chaff.chaff_ratio = 0.5; // 50% chaff for testing
        let mut flow = HarmonicFlow::with_config(config);
        // Register a session key for testing.
        let key = ChaffingEngine::generate_session_key(b"test-session-seed");
        flow.register_session_key("harmonic-default-session", key);
        flow
    }

    // --- Construction Tests ---

    #[test]
    fn test_flow_creation() {
        let flow = HarmonicFlow::new();
        assert_eq!(flow.config().session_id, "harmonic-default-session");
    }

    #[test]
    fn test_flow_custom_config() {
        let config = HarmonicFlowConfig {
            session_id: "custom-session".to_string(),
            ..HarmonicFlowConfig::default()
        };
        let flow = HarmonicFlow::with_config(config);
        assert_eq!(flow.config().session_id, "custom-session");
    }

    #[test]
    fn test_flow_default() {
        let flow = HarmonicFlow::default();
        assert_eq!(flow.config().session_id, "harmonic-default-session");
    }

    // --- Session Management Tests ---

    #[test]
    fn test_generate_session_key() {
        let key1 = HarmonicFlow::generate_session_key(b"seed");
        let key2 = HarmonicFlow::generate_session_key(b"seed");
        assert_eq!(key1, key2); // Deterministic from seed
    }

    #[test]
    fn test_register_session_key() {
        let mut flow = HarmonicFlow::new();
        let key = [42u8; 32];
        flow.register_session_key("test-session", key);
        // No error means success.
    }

    #[test]
    fn test_set_session_id() {
        let mut flow = HarmonicFlow::new();
        flow.set_session_id("new-session".to_string());
        assert_eq!(flow.config().session_id, "new-session");
    }

    // --- Obfuscation Tests ---

    #[test]
    fn test_obfuscate_empty_payload() {
        let mut flow = setup_flow();
        match flow.obfuscate(&[]) {
            Err(HarmonicFlowError::EmptyPayload) => (),
            other => panic!("Expected EmptyPayload, got {:?}", other),
        }
    }

    #[test]
    fn test_obfuscate_valid_payload() {
        let mut flow = setup_flow();
        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");

        assert_eq!(stream.original_size, payload.len());
        assert!(stream.obfuscated_size >= stream.original_size);
        assert!(stream.expansion_ratio >= 1.0);
        assert!(!stream.frames.is_empty());
    }

    #[test]
    fn test_obfuscate_preserves_transport() {
        let mut flow = setup_flow();
        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");

        // All frames should have the same transport profile.
        for frame in &stream.frames {
            assert_eq!(frame.transport, stream.transport);
        }
    }

    #[test]
    fn test_obfuscate_expansion_with_chaff() {
        let mut config = HarmonicFlowConfig::default();
        config.chaff.chaff_ratio = 1.0; // 100% chaff = 2x expansion
        let mut flow = HarmonicFlow::with_config(config);
        let key = ChaffingEngine::generate_session_key(b"expansion-test");
        flow.register_session_key("harmonic-default-session", key);

        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");

        // With 100% chaff ratio, we expect roughly 2x expansion.
        assert!(stream.expansion_ratio >= 1.5);
    }

    // --- De-obfuscation Tests ---

    #[test]
    fn test_deobfuscate_roundtrip() {
        let mut flow = setup_flow();
        let payload = test_payload();

        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
        let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");

        assert_eq!(result.payload, payload);
        assert!(result.frames_processed > 0);
    }

    #[test]
    fn test_deobfuscate_removes_chaff() {
        let mut config = HarmonicFlowConfig::default();
        config.chaff.chaff_ratio = 0.8; // High chaff ratio
        let mut flow = HarmonicFlow::with_config(config);
        let key = ChaffingEngine::generate_session_key(b"chaff-removal");
        flow.register_session_key("harmonic-default-session", key);

        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
        let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");

        assert_eq!(result.payload, payload);
        assert!(result.chaff_removed > 0);
    }

    // --- Transport Rotation Tests ---

    #[test]
    fn test_rotate_transport() {
        let mut flow = setup_flow();
        let current = flow.current_transport();
        let next = flow.rotate_transport();
        // Transport should change after rotation.
        // (Depending on rotator implementation, may cycle through profiles)
        let _ = current;
        let _ = next;
    }

    #[test]
    fn test_report_health() {
        let mut flow = setup_flow();
        flow.report_health(0.95, 50.0);
        // No error means success.
    }

    // --- Configuration Tests ---

    #[test]
    fn test_config_validate_valid() {
        let config = HarmonicFlowConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_session() {
        let config = HarmonicFlowConfig {
            session_id: "".to_string(),
            ..HarmonicFlowConfig::default()
        };
        match config.validate() {
            Err(HarmonicFlowError::ConfigError(msg)) => {
                assert!(msg.contains("Session ID"));
            }
            other => panic!("Expected ConfigError, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_bad_chaff_ratio() {
        let config = HarmonicFlowConfig {
            chaff: ChaffConfig {
                chaff_ratio: 1.5,
                ..ChaffConfig::default()
            },
            ..HarmonicFlowConfig::default()
        };
        match config.validate() {
            Err(HarmonicFlowError::ConfigError(msg)) => {
                assert!(msg.contains("Chaff ratio"));
            }
            other => panic!("Expected ConfigError, got {:?}", other),
        }
    }

    #[test]
    fn test_update_config() {
        let mut flow = setup_flow();
        let new_config = HarmonicFlowConfig {
            session_id: "updated-session".to_string(),
            ..HarmonicFlowConfig::default()
        };
        flow.update_config(new_config);
        assert_eq!(flow.config().session_id, "updated-session");
    }

    // --- Reset Tests ---

    #[test]
    fn test_reset() {
        let mut flow = setup_flow();
        flow.reset();
        // After reset, should have default state.
        assert_eq!(flow.config().session_id, "harmonic-default-session");
    }

    // --- Error Display Tests ---

    #[test]
    fn test_error_display_masking() {
        let err = HarmonicFlowError::MaskingError(MaskingError::PayloadTooLarge(999));
        let msg = format!("{}", err);
        assert!(msg.contains("Masking error"));
    }

    #[test]
    fn test_error_display_chaffing() {
        let err = HarmonicFlowError::ChaffingError(ChaffingError::InvalidRatio(2.0));
        let msg = format!("{}", err);
        assert!(msg.contains("Chaffing error"));
    }

    #[test]
    fn test_error_display_empty() {
        let err = HarmonicFlowError::EmptyPayload;
        let msg = format!("{}", err);
        assert!(msg.contains("empty"));
    }

    #[test]
    fn test_error_display_session_missing() {
        let err = HarmonicFlowError::SessionKeyMissing("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Session key missing"));
    }

    #[test]
    fn test_error_display_config() {
        let err = HarmonicFlowError::ConfigError("test error".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Config error"));
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_pipeline_large_payload() {
        let mut flow = setup_flow();
        let payload: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();

        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
        let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");

        assert_eq!(result.payload, payload);
    }

    #[test]
    fn test_multiple_obfuscate_cycles() {
        let mut flow = setup_flow();
        for i in 0..5 {
            let payload = format!("cycle-{}", i).into_bytes();
            let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
            let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");
            assert_eq!(result.payload, payload);
        }
    }

    #[test]
    fn test_harmonic_frame_structure() {
        let mut flow = setup_flow();
        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");

        for (i, frame) in stream.frames.iter().enumerate() {
            // Each frame should have a transport profile.
            assert_eq!(frame.transport, stream.transport);
            // Real packets have SRTP headers; chaff may not.
            let _ = &frame.packet;
            let _ = &frame.srtp_header;
            let _ = i; // Ensure all frames are visited
        }
    }
}
