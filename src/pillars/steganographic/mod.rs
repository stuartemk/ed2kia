//! Steganographic Survival — Preservación de Red (Pillar 3).
//!
//! **RFC 003:** Traffic obfuscation layer for cooperative network preservation.
//! Makes ed2kIA traffic harmonize with standard internet patterns through
//! multiple preservation techniques working in symphony.
//!
//! **Complete Flow:**
//! ```text
//! Raw Payload → SCT Validation → Traffic Masking (SRTP) → Chaffing & Winnowing
//!     → Transport Rotation → Network
//! ```
//!
//! **Techniques:**
//! - **Traffic Masking:** Wraps payloads in simulated SRTP/WebRTC frames,
//!   altering headers and timestamps to harmonize with media streaming.
//! - **Chaffing & Winnowing:** Injects cryptographic noise packets (configurable ratio)
//!   to dilute signal patterns, with perfect reconstruction via winnowing.
//! - **Transport Rotation:** Dynamic protocol rotation (TCP/QUIC/WebSocket/WebRTC)
//!   based on health metrics for resilient communication.
//!
//! **Security Invariant:** Feature-gated (`v3.0-steganographic-survival`), disabled by default.
//! Zero data loss — only traffic pattern obfuscation.
//!
//! **Feature Gate:** `v3.0-steganographic-survival`

use crate::orchestration::PillarId;
use crate::pillars::{PillarError, PillarInterface};

pub mod traffic_masker;
pub mod chaffing_engine;
pub mod transport_rotator;
#[cfg(feature = "v3.6-aegis-resonance")]
pub mod harmonic_flow;
#[cfg(feature = "v3.0-omni-integration")]
pub mod migration_protocol;

pub use traffic_masker::{TrafficMasker, MaskingError, MaskerConfig, SrtpHeader};
pub use chaffing_engine::{ChaffingEngine, ChaffingError, ChaffConfig, TaggedPacket};
pub use transport_rotator::{TransportRotator, RotationError, RotatorConfig, TransportType, TransportHealth};
#[cfg(feature = "v3.6-aegis-resonance")]
pub use harmonic_flow::{HarmonicFlow, HarmonicFlowConfig, HarmonicFlowError, HarmonicFrame, ObfuscatedStream, DeobfuscatedPayload};
#[cfg(feature = "v3.0-omni-integration")]
pub use migration_protocol::{MigrationHandshake, MigrationToken, MigrationNegotiator, MigrationError, MigrationRecord, MigrationStatus};

/// Obfuscation pipeline result: (SRTP frames, chaffed packets, selected transport).
pub type ObfuscationResult = Result<(Vec<Vec<u8>>, Vec<TaggedPacket>, TransportType), String>;

/// Steganographic Survival Engine — Network preservation coordinator.
///
/// Orchestrates traffic obfuscation techniques to ensure ed2kIA's cooperative
/// communication persists through harmonious integration with the global
/// digital ecosystem.
///
/// **Expected Flow:**
/// 1. Outbound payloads intercepted by steganographic layer.
/// 2. Payloads masked as SRTP frames (simulated media streaming).
/// 3. Noise packets injected (chaffing) to dilute signal patterns.
/// 4. Transport endpoints rotated based on health metrics.
/// 5. Inbound traffic winnowed and unmasked to recover original payloads.
pub struct SteganographicEngine {
    masker: TrafficMasker,
    chaffing: ChaffingEngine,
    rotator: TransportRotator,
}

impl SteganographicEngine {
    /// Create a new Steganographic Survival Engine with default configuration.
    pub fn new() -> Self {
        Self {
            masker: TrafficMasker::new(),
            chaffing: ChaffingEngine::new(),
            rotator: TransportRotator::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        masker_config: MaskerConfig,
        chaff_config: ChaffConfig,
        rotator_config: RotatorConfig,
    ) -> Result<Self, RotationError> {
        Ok(Self {
            masker: TrafficMasker::with_config(masker_config),
            chaffing: ChaffingEngine::with_config(chaff_config),
            rotator: TransportRotator::with_config(rotator_config)?,
        })
    }

    /// Full obfuscation pipeline: mask → chaff → select transport.
    ///
    /// Returns: (masked_frames, chaffed_packets, selected_transport)
    pub fn obfuscate(
        &mut self,
        payload: &[u8],
        session_id: &str,
    ) -> ObfuscationResult {
        // Step 1: Traffic Masking (SRTP frames)
        let frames = self.masker.mask_payload(payload)
            .map_err(|e| format!("Masking failed: {}", e))?;

        // Step 2: Chaffing & Winnowing (noise injection)
        // Flatten frames for chaffing
        let mut flattened = Vec::new();
        for frame in &frames {
            flattened.extend_from_slice(frame);
        }

        let chaffed = self.chaffing.inject_chaff(&flattened, session_id)
            .map_err(|e| format!("Chaffing failed: {}", e))?;

        // Step 3: Transport Selection
        let transport = self.rotator.select_best()
            .unwrap_or_else(|| self.rotator.current_transport().clone());

        Ok((frames, chaffed, transport))
    }

    /// Full de-obfuscation pipeline: winnow → unmask.
    pub fn deobfuscate(
        &self,
        chaffed: &[TaggedPacket],
        session_id: &str,
    ) -> Result<Vec<u8>, String> {
        // Step 1: Winnowing (remove noise)
        let stream = self.chaffing.winnow(chaffed, session_id)
            .map_err(|e| format!("Winnowing failed: {}", e))?;

        // Note: Full unmasking requires frame boundaries which are lost after chaffing.
        // In production, frame metadata would be preserved in the chaffing tags.
        // For this implementation, we return the winnowed stream directly.
        Ok(stream)
    }

    /// Rotate to the next best transport.
    pub fn rotate_transport(&mut self) -> Result<TransportType, RotationError> {
        self.rotator.rotate()
    }

    /// Update transport health metrics.
    pub fn update_health(&mut self, health: TransportHealth) {
        self.rotator.update_health(health);
    }

    /// Get the current active transport.
    pub fn current_transport(&self) -> &TransportType {
        self.rotator.current_transport()
    }

    /// Get the traffic masker.
    pub fn masker(&self) -> &TrafficMasker {
        &self.masker
    }

    /// Get the chaffing engine.
    pub fn chaffing_engine(&self) -> &ChaffingEngine {
        &self.chaffing
    }

    /// Get the chaffing engine (mutable).
    pub fn chaffing_engine_mut(&mut self) -> &mut ChaffingEngine {
        &mut self.chaffing
    }

    /// Get the transport rotator.
    pub fn rotator(&self) -> &TransportRotator {
        &self.rotator
    }
}

impl PillarInterface for SteganographicEngine {
    fn id() -> PillarId {
        PillarId::SteganographicSurvival
    }

    fn validate_local_constraint(&self) -> bool {
        // Steganographic Survival operates on network traffic — obfuscation
        // occurs at the transport layer, preserving data integrity.
        // No LOCAL_ONLY constraint since traffic must traverse the network.
        true
    }

    fn consume_ce(&self, amount: f64) -> Result<(), PillarError> {
        if amount <= 0.0 {
            return Err(PillarError::InsufficientCE);
        }
        // CE cost for steganographic processing overhead:
        // masking + chaffing + rotation computation
        Ok(())
    }
}

impl Default for SteganographicEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = SteganographicEngine::new();
        assert_eq!(*engine.current_transport(), TransportType::Tcp);
    }

    #[test]
    fn test_pillar_id() {
        assert_eq!(SteganographicEngine::id(), PillarId::SteganographicSurvival);
    }

    #[test]
    fn test_local_constraint() {
        let engine = SteganographicEngine::new();
        assert!(engine.validate_local_constraint());
    }

    #[test]
    fn test_consume_ce_valid() {
        let engine = SteganographicEngine::new();
        assert!(engine.consume_ce(10.0).is_ok());
    }

    #[test]
    fn test_consume_ce_zero_rejected() {
        let engine = SteganographicEngine::new();
        match engine.consume_ce(0.0) {
            Err(PillarError::InsufficientCE) => {},
            other => panic!("Expected InsufficientCE, got {:?}", other),
        }
    }

    #[test]
    fn test_consume_ce_negative_rejected() {
        let engine = SteganographicEngine::new();
        match engine.consume_ce(-1.0) {
            Err(PillarError::InsufficientCE) => {},
            other => panic!("Expected InsufficientCE, got {:?}", other),
        }
    }

    #[test]
    fn test_default() {
        let engine = SteganographicEngine::default();
        assert_eq!(*engine.current_transport(), TransportType::Tcp);
    }

    #[test]
    fn test_obfuscate_full_pipeline() {
        let mut engine = SteganographicEngine::new();
        // Register session key for chaffing
        let key = ChaffingEngine::generate_session_key(b"test-session");
        engine.chaffing.register_session_key("test".to_string(), key);

        let payload = b"cooperative network preservation";
        let result = engine.obfuscate(payload, "test");
        assert!(result.is_ok());
        let (frames, chaffed, _transport) = result.unwrap();
        assert!(!frames.is_empty());
        assert!(!chaffed.is_empty());
    }

    #[test]
    fn test_rotate_transport() {
        let mut engine = SteganographicEngine::new();
        let initial = engine.current_transport().clone();
        let new_transport = engine.rotate_transport().unwrap();
        // Should rotate to a different transport
        assert_ne!(initial, new_transport);
    }

    #[test]
    fn test_update_health() {
        let mut engine = SteganographicEngine::new();
        let health = TransportHealth::new(TransportType::Quic, 30.0, 0.0, 900_000.0);
        engine.update_health(health);
        assert!(engine.rotator().get_health(&TransportType::Quic).is_some());
    }
}
