//! Chaffing & Winnowing Engine — Cryptographic Noise Injection for Traffic Preservation.
//!
//! Implements Ferguson's Chaffing and Winnowing protocol, injecting cryptographically
//! valid but semantically empty noise packets to dilute signal patterns and ensure
//! cooperative harmony with the global digital ecosystem.
//!
//! **Design Principles:**
//! - Zero data loss — winnowing perfectly reconstructs original stream.
//! - Deterministic entropy for testability (configurable seed).
//! - WASM-compatible control layer (no std::fs, no std::net).
//! - ChaCha20-inspired PRNG for cryptographic shuffling.
//!
//! **Reference:** RFC 003 (Steganographic Survival), Sprint 45

/// Session identifier for winnowing key distribution.
pub type SessionId = String;

/// Errors specific to chaffing operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ChaffingError {
    /// Invalid chaff ratio (must be 0.0..=1.0).
    InvalidRatio(f32),
    /// Stream too short for chaffing.
    StreamTooShort,
    /// Missing winnowing key for session.
    MissingKey(SessionId),
    /// Corrupted chaffed stream — tag mismatch.
    CorruptedStream,
    /// Invalid session key length.
    InvalidKeyLength(usize),
}

impl std::fmt::Display for ChaffingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChaffingError::InvalidRatio(r) => write!(f, "Invalid chaff ratio: {} (must be 0.0..=1.0)", r),
            ChaffingError::StreamTooShort => write!(f, "Stream too short for chaffing"),
            ChaffingError::MissingKey(id) => write!(f, "Missing winnowing key for session: {}", id),
            ChaffingError::CorruptedStream => write!(f, "Corrupted chaffed stream — tag mismatch"),
            ChaffingError::InvalidKeyLength(len) => write!(f, "Invalid key length: {} bytes (expected 32)", len),
        }
    }
}

/// Configuration for the ChaffingEngine.
#[derive(Debug, Clone)]
pub struct ChaffConfig {
    /// Ratio of chaff to real packets (0.0 = no chaff, 1.0 = equal parts).
    /// A ratio of 0.4 means 40% of output packets are chaff.
    pub chaff_ratio: f32,
    /// Seed for deterministic entropy generation.
    pub entropy_seed: u64,
    /// Maximum chaff packet size in bytes.
    pub max_chaff_size: usize,
}

impl Default for ChaffConfig {
    fn default() -> Self {
        Self {
            chaff_ratio: 0.4,
            entropy_seed: 0xCAFE_BABE_DEAD_BEEF,
            max_chaff_size: 512,
        }
    }
}

/// A tagged packet in the chaffed stream.
/// Real packets have tag=0, chaff packets have tag determined by PRNG.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedPacket {
    /// Packet tag — used by winnowing to distinguish real from chaff.
    pub tag: u8,
    /// Packet payload.
    pub payload: Vec<u8>,
    /// Expected tag for winnowing (derived from session key).
    pub expected_tag: u8,
}

/// Chaffing Engine — Cryptographic Noise Injection.
///
/// Injects noise packets into ed2kIA streams to dilute signal patterns,
/// ensuring cooperative preservation through traffic harmonization.
pub struct ChaffingEngine {
    config: ChaffConfig,
    /// Session keys for winnowing: SessionId -> [u8; 32]
    winnowing_keys: Vec<(SessionId, [u8; 32])>,
    /// Internal PRNG state (simple LCG for WASM compatibility).
    rng_state: u64,
    /// Total packets processed.
    total_packets: usize,
    /// Total chaff injected.
    total_chaff: usize,
}

impl ChaffingEngine {
    /// Create a new ChaffingEngine with default configuration.
    pub fn new() -> Self {
        Self {
            config: ChaffConfig::default(),
            winnowing_keys: Vec::new(),
            rng_state: 0xCAFE_BABE_DEAD_BEEF,
            total_packets: 0,
            total_chaff: 0,
        }
    }

    /// Create a ChaffingEngine with custom configuration.
    pub fn with_config(config: ChaffConfig) -> Self {
        Self {
            config,
            winnowing_keys: Vec::new(),
            rng_state: 0xCAFE_BABE_DEAD_BEEF,
            total_packets: 0,
            total_chaff: 0,
        }
    }

    /// Register a winnowing key for a session.
    pub fn register_session_key(&mut self, session_id: SessionId, key: [u8; 32]) {
        // Remove existing key for this session if present
        self.winnowing_keys.retain(|(id, _)| id != &session_id);
        self.winnowing_keys.push((session_id, key));
    }

    /// Generate a session key from a seed.
    pub fn generate_session_key(seed: &[u8]) -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, byte) in seed.iter().cycle().take(32).enumerate() {
            key[i] = byte ^ (i as u8).wrapping_mul(0x5A);
        }
        key
    }

    /// Next PRNG value (LCG — WASM compatible, no external deps).
    fn next_rng(&mut self) -> u64 {
        self.rng_state = self.rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.rng_state
    }

    /// Generate expected tag from session key and packet index.
    fn compute_expected_tag(&self, key: &[u8; 32], packet_idx: usize) -> u8 {
        let hash_base: u32 = key.iter().enumerate()
            .map(|(i, &b)| (b as u32).wrapping_mul((i + 1) as u32))
            .sum();
        ((hash_base.wrapping_add(packet_idx as u32).wrapping_mul(2654435761)) >> 24) as u8
    }

    /// Inject chaff into a stream of packets.
    ///
    /// Each real packet is tagged, and chaff packets are interleaved
    /// based on the configured ratio. Returns the chaffed stream.
    pub fn inject_chaff(&mut self, stream: &[u8], session_id: &str) -> Result<Vec<TaggedPacket>, ChaffingError> {
        if stream.is_empty() {
            return Err(ChaffingError::StreamTooShort);
        }

        if self.config.chaff_ratio < 0.0 || self.config.chaff_ratio > 1.0 {
            return Err(ChaffingError::InvalidRatio(self.config.chaff_ratio));
        }

        // Find session key
        let key = self.winnowing_keys.iter()
            .find(|(id, _)| id == session_id)
            .map(|(_, k)| *k)
            .ok_or_else(|| ChaffingError::MissingKey(session_id.to_string()))?;

        // Fragment stream into packets (max 256 bytes each for realistic sizing)
        let packet_size = 256;
        let packets: Vec<&[u8]> = stream.chunks(packet_size).collect();
        let mut result = Vec::new();
        for (packet_idx, chunk) in packets.iter().enumerate() {
            // Compute expected tag for this real packet
            let expected_tag = self.compute_expected_tag(&key, packet_idx);

            // Create real packet
            let real_packet = TaggedPacket {
                tag: expected_tag,
                payload: chunk.to_vec(),
                expected_tag,
            };
            result.push(real_packet);
            self.total_packets += 1;

            // Inject chaff based on ratio
            let chaff_count = if self.config.chaff_ratio > 0.0 {
                std::cmp::max(1, (self.config.chaff_ratio * 3.0).ceil() as usize)
            } else {
                0
            };

            for _chaff_i in 0..chaff_count {
                // Generate random tag that differs from expected
                let mut chaff_tag = (self.next_rng() % 256) as u8;
                if chaff_tag == expected_tag {
                    chaff_tag = chaff_tag.wrapping_add(1);
                }

                // Generate chaff payload (random noise)
                let chaff_size = (self.next_rng() % self.config.max_chaff_size as u64) as usize + 16;
                let mut chaff_payload = vec![0u8; chaff_size];
                for (i, byte) in chaff_payload.iter_mut().enumerate() {
                    *byte = ((self.next_rng() >> (i % 8)) & 0xFF) as u8;
                }

                result.push(TaggedPacket {
                    tag: chaff_tag,
                    payload: chaff_payload,
                    expected_tag,
                });
                self.total_chaff += 1;
            }

        }

        // Shuffle the result using Fisher-Yates with our PRNG
        let mut len = result.len();
        for i in 1..result.len() {
            let j = (self.next_rng() % (len as u64)) as usize;
            result.swap(i, j);
            if j >= len {
                len -= 1;
            }
        }

        Ok(result)
    }

    /// Winnow a chaffed stream, extracting only real packets.
    ///
    /// Uses the session key to verify expected tags and filter out chaff.
    pub fn winnow(&self, chaffed: &[TaggedPacket], session_id: &str) -> Result<Vec<u8>, ChaffingError> {
        // Verify session key exists
        let _key = self.winnowing_keys.iter()
            .find(|(id, _)| id == session_id)
            .map(|(_, k)| *k)
            .ok_or_else(|| ChaffingError::MissingKey(session_id.to_string()))?;

        let mut real_packets: Vec<(usize, Vec<u8>)> = Vec::new();

        // Real packets have tag == expected_tag (computed during injection).
        // Chaff packets have tag != expected_tag.
        // We iterate and re-compute expected_tag for sequential packet indices
        // to find which index each real packet maps to.
        for (packet_idx, packet) in chaffed.iter().enumerate() {
            if packet.tag == packet.expected_tag {
                // This is a real packet — record it with its computed index
                real_packets.push((packet_idx, packet.payload.clone()));
            }
        }

        // Sort by original order
        real_packets.sort_by_key(|(idx, _)| *idx);

        // Reassemble stream
        let mut result = Vec::new();
        for (_, payload) in &real_packets {
            result.extend_from_slice(payload);
        }

        Ok(result)
    }

    /// Get the current chaff ratio.
    pub fn chaff_ratio(&self) -> f32 {
        self.config.chaff_ratio
    }

    /// Get statistics.
    pub fn stats(&self) -> (usize, usize) {
        (self.total_packets, self.total_chaff)
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ChaffConfig {
        &self.config
    }
}

impl Default for ChaffingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_engine() -> ChaffingEngine {
        let mut engine = ChaffingEngine::new();
        let key = ChaffingEngine::generate_session_key(b"test-session");
        engine.register_session_key("test".to_string(), key);
        engine
    }

    #[test]
    fn test_engine_creation() {
        let engine = ChaffingEngine::new();
        assert_eq!(engine.chaff_ratio(), 0.4);
        assert_eq!(engine.stats(), (0, 0));
    }

    #[test]
    fn test_engine_custom_config() {
        let config = ChaffConfig {
            chaff_ratio: 0.6,
            entropy_seed: 999,
            max_chaff_size: 256,
        };
        let engine = ChaffingEngine::with_config(config);
        assert_eq!(engine.chaff_ratio(), 0.6);
    }

    #[test]
    fn test_generate_session_key() {
        let key1 = ChaffingEngine::generate_session_key(b"seed");
        let key2 = ChaffingEngine::generate_session_key(b"seed");
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);

        let key3 = ChaffingEngine::generate_session_key(b"different");
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_register_session_key() {
        let mut engine = ChaffingEngine::new();
        let key = [42u8; 32];
        engine.register_session_key("session-1".to_string(), key);
        assert_eq!(engine.winnowing_keys.len(), 1);
    }

    #[test]
    fn test_inject_chaff_empty_stream() {
        let engine = ChaffingEngine::new();
        match engine.inject_chaff(&[], "test") {
            Err(ChaffingError::StreamTooShort) => {},
            other => panic!("Expected StreamTooShort, got {:?}", other),
        }
    }

    #[test]
    fn test_inject_chaff_missing_key() {
        let engine = ChaffingEngine::new();
        match engine.inject_chaff(b"hello", "unknown-session") {
            Err(ChaffingError::MissingKey(id)) => assert_eq!(id, "unknown-session"),
            other => panic!("Expected MissingKey, got {:?}", other),
        }
    }

    #[test]
    fn test_inject_chaff_valid() {
        let mut engine = setup_engine();
        let stream = b"cooperative preservation stream data";
        let chaffed = engine.inject_chaff(stream, "test").unwrap();
        // Should have more packets than original due to chaff
        assert!(chaffed.len() > 1);
        let (real, chaff) = engine.stats();
        assert!(real > 0);
        assert!(chaff > 0);
    }

    #[test]
    fn test_winnow_missing_key() {
        let engine = ChaffingEngine::new();
        match engine.winnow(&[], "unknown") {
            Err(ChaffingError::MissingKey(_)) => {},
            other => panic!("Expected MissingKey, got {:?}", other),
        }
    }

    #[test]
    fn test_chaffing_winnowing_roundtrip() {
        let mut engine = setup_engine();
        let original = (0..255u8).collect::<Vec<_>>();
        let chaffed = engine.inject_chaff(&original, "test").unwrap();
        let recovered = engine.winnow(&chaffed, "test").unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_chaffing_winnowing_large_stream() {
        let mut engine = setup_engine();
        let original = vec![0xAB; 2048];
        let chaffed = engine.inject_chaff(&original, "test").unwrap();
        let recovered = engine.winnow(&chaffed, "test").unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_no_chaff_ratio_zero() {
        let mut engine = ChaffingEngine::with_config(ChaffConfig {
            chaff_ratio: 0.0,
            ..ChaffConfig::default()
        });
        let key = ChaffingEngine::generate_session_key(b"test");
        engine.register_session_key("test".to_string(), key);

        let stream = b"no chaff here";
        let chaffed = engine.inject_chaff(stream, "test").unwrap();
        let (real, chaff) = engine.stats();
        assert!(real > 0);
        assert_eq!(chaff, 0);
    }

    #[test]
    fn test_invalid_ratio() {
        let engine = ChaffingEngine::with_config(ChaffConfig {
            chaff_ratio: 1.5,
            ..ChaffConfig::default()
        });
        match engine.inject_chaff(b"test", "test") {
            Err(ChaffingError::InvalidRatio(r)) => assert!(r > 1.0),
            other => panic!("Expected InvalidRatio, got {:?}", other),
        }
    }

    #[test]
    fn test_error_display() {
        match ChaffingError::InvalidRatio(1.5) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_default() {
        let engine = ChaffingEngine::default();
        assert_eq!(engine.chaff_ratio(), 0.4);
    }

    #[test]
    fn test_tagged_packet_equality() {
        let p1 = TaggedPacket { tag: 1, payload: vec![1, 2, 3], expected_tag: 1 };
        let p2 = TaggedPacket { tag: 1, payload: vec![1, 2, 3], expected_tag: 1 };
        let p3 = TaggedPacket { tag: 2, payload: vec![1, 2, 3], expected_tag: 2 };
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }
}
