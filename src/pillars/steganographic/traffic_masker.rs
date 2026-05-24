//! Traffic Masker — SRTP Frame Simulation for Traffic Preservation.
//!
//! Wraps ed2kIA payloads in simulated SRTP/WebRTC frames, altering headers
//! and timestamps to harmonize with standard streaming/video traffic patterns.
//! This ensures cooperative network preservation by making ed2kIA traffic
//! indistinguishable from legitimate media streams.
//!
//! **Design Principles:**
//! - Zero data loss — only traffic pattern obfuscation.
//! - Deterministic masking for testability (configurable noise_seed).
//! - WASM-compatible control layer (no std::fs, no std::net).
//!
//! **Reference:** RFC 003 (Steganographic Survival), Sprint 45


/// Errors specific to traffic masking operations.
#[derive(Debug, Clone, PartialEq)]
pub enum MaskingError {
    /// Payload exceeds maximum SRTP frame size (1400 bytes).
    PayloadTooLarge(usize),
    /// Masking operation failed due to invalid configuration.
    InvalidConfig(String),
    /// Frame encoding failed.
    EncodingFailed(String),
    /// Frame decoding failed — corrupted or tampered cover traffic.
    DecodingFailed(String),
}

impl std::fmt::Display for MaskingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaskingError::PayloadTooLarge(size) => {
                write!(f, "Payload too large for SRTP frame: {} bytes (max 1400)", size)
            }
            MaskingError::InvalidConfig(msg) => write!(f, "Invalid masking configuration: {}", msg),
            MaskingError::EncodingFailed(msg) => write!(f, "Frame encoding failed: {}", msg),
            MaskingError::DecodingFailed(msg) => write!(f, "Frame decoding failed: {}", msg),
        }
    }
}

/// SRTP frame header simulation for traffic preservation.
///
/// Simulates standard SRTP header structure (RFC 3711) to harmonize
/// ed2kIA traffic with legitimate WebRTC/media streams.
#[derive(Debug, Clone)]
pub struct SrtpHeader {
    /// RTP version (always 2 for standard compliance).
    pub version: u8,
    /// Padding flag (reserved for future use).
    pub padding: u8,
    /// Extension flag (indicates header extension presence).
    pub extension: u8,
    /// Sequence number for frame ordering simulation.
    pub sequence_number: u16,
    /// Timestamp simulating media clock (90kHz for video).
    pub timestamp: u32,
    /// Synchronization source identifier (simulated camera/stream ID).
    pub ssrc: u32,
    /// Payload type (96-127 dynamic range, simulating H.264/VP8).
    pub payload_type: u8,
}

impl SrtpHeader {
    /// Create a new simulated SRTP header.
    pub fn new(sequence_number: u16, timestamp: u32, ssrc: u32) -> Self {
        Self {
            version: 2,
            padding: 0,
            extension: 0,
            sequence_number,
            timestamp,
            ssrc,
            payload_type: 96, // Dynamic range — H.264 simulation
        }
    }

    /// Serialize header to bytes (12 bytes standard SRTP header).
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(12);
        // Byte 0: V(2) | P(1) | X(1) | CC(4)
        buf.push((self.version << 6) | (self.padding << 5) | (self.extension << 4));
        // Byte 1: Marker(1) | Payload Type(7)
        buf.push(self.payload_type);
        // Bytes 2-3: Sequence Number (big-endian)
        buf.extend_from_slice(&self.sequence_number.to_be_bytes());
        // Bytes 4-7: Timestamp (big-endian)
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        // Bytes 8-11: SSRC (big-endian)
        buf.extend_from_slice(&self.ssrc.to_be_bytes());
        buf
    }

    /// Deserialize header from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, MaskingError> {
        if data.len() < 12 {
            return Err(MaskingError::DecodingFailed(
                "Insufficient data for SRTP header".into(),
            ));
        }
        let version = (data[0] >> 6) & 0x03;
        let padding = (data[0] >> 5) & 0x01;
        let extension = (data[0] >> 4) & 0x01;
        let payload_type = data[1];
        let sequence_number = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        if version != 2 {
            return Err(MaskingError::DecodingFailed(
                format!("Invalid SRTP version: {}", version),
            ));
        }

        Ok(Self {
            version,
            padding,
            extension,
            sequence_number,
            timestamp,
            ssrc,
            payload_type,
        })
    }
}

/// Configuration for the TrafficMasker.
#[derive(Debug, Clone)]
pub struct MaskerConfig {
    /// Maximum payload size per SRTP frame (default 1400 bytes).
    pub max_payload_size: usize,
    /// Noise seed for deterministic timestamp generation.
    pub noise_seed: u64,
    /// Simulated SSRC for stream identification.
    pub ssrc: u32,
    /// Media clock rate for timestamp generation (default 90000 Hz).
    pub clock_rate: u32,
}

impl Default for MaskerConfig {
    fn default() -> Self {
        Self {
            max_payload_size: 1400,
            noise_seed: 42,
            ssrc: 0xDEAD_BEEF,
            clock_rate: 90_000,
        }
    }
}

/// Traffic Masker — SRTP Frame Simulation Engine.
///
/// Wraps ed2kIA payloads in simulated SRTP frames for traffic preservation,
/// ensuring cooperative harmony with the global digital ecosystem.
pub struct TrafficMasker {
    config: MaskerConfig,
    sequence_counter: u16,
    frame_count: usize,
}

impl TrafficMasker {
    /// Create a new TrafficMasker with default configuration.
    pub fn new() -> Self {
        Self {
            config: MaskerConfig::default(),
            sequence_counter: 0,
            frame_count: 0,
        }
    }

    /// Create a TrafficMasker with custom configuration.
    pub fn with_config(config: MaskerConfig) -> Self {
        Self {
            config,
            sequence_counter: 0,
            frame_count: 0,
        }
    }

    /// Generate a pseudo-random timestamp based on noise_seed and frame count.
    fn generate_timestamp(&self) -> u32 {
        // Simple LCG for deterministic timestamp generation
        let lcg = self.config.noise_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (lcg as u32).wrapping_add((self.frame_count as u32) * self.config.clock_rate / 30)
    }

    /// Mask a payload by wrapping it in a simulated SRTP frame.
    ///
    /// The original payload is fragmented into chunks ≤ max_payload_size,
    /// each wrapped in its own SRTP frame with simulated headers.
    /// Returns a vector of masked frames.
    pub fn mask_payload(&mut self, original: &[u8]) -> Result<Vec<Vec<u8>>, MaskingError> {
        if original.is_empty() {
            return Err(MaskingError::InvalidConfig("Empty payload cannot be masked".into()));
        }

        let max_payload = self.config.max_payload_size;
        let num_frames = original.len().div_ceil(max_payload);
        let mut frames = Vec::with_capacity(num_frames);

        for (chunk_idx, chunk) in original.chunks(max_payload).enumerate() {
            let header = SrtpHeader::new(
                self.sequence_counter.wrapping_add(chunk_idx as u16),
                self.generate_timestamp(),
                self.config.ssrc,
            );

            let mut frame = header.serialize();
            frame.extend_from_slice(chunk);

            // Add frame metadata: total_frames, chunk_index (4 bytes)
            frame.push(num_frames as u8);
            frame.push(chunk_idx as u8);
            // 2-byte checksum for integrity
            let checksum = self.compute_checksum(&frame);
            frame.extend_from_slice(&checksum.to_be_bytes());

            frames.push(frame);
        }

        self.sequence_counter = self.sequence_counter.wrapping_add(num_frames as u16);
        self.frame_count += num_frames;

        Ok(frames)
    }

    /// Unmask a single SRTP frame, extracting the original payload chunk.
    pub fn unmask_frame(&self, frame: &[u8]) -> Result<(Vec<u8>, usize, usize), MaskingError> {
        // Minimum frame: 12 (header) + 1 (payload) + 2 (metadata) + 2 (checksum) = 17
        if frame.len() < 17 {
            return Err(MaskingError::DecodingFailed(
                "Frame too small for SRTP header + metadata".into(),
            ));
        }

        // Verify checksum
        let stored_checksum = u16::from_be_bytes([frame[frame.len() - 2], frame[frame.len() - 1]]);
        let computed_checksum = self.compute_checksum(&frame[..frame.len() - 2]);
        if stored_checksum != computed_checksum {
            return Err(MaskingError::DecodingFailed(
                "Checksum mismatch — frame integrity compromised".into(),
            ));
        }

        // Parse header to skip it
        let _header = SrtpHeader::deserialize(frame)?;

        // Extract payload (skip 12-byte header, 2-byte metadata, 2-byte checksum)
        let payload_start = 12;
        let payload_end = frame.len() - 4;
        let payload = frame[payload_start..payload_end].to_vec();

        // Metadata: [N-4] = total_frames, [N-3] = chunk_index, [N-2..N] = checksum
        let total_frames = frame[frame.len() - 4] as usize;
        let chunk_index = frame[frame.len() - 3] as usize;

        Ok((payload, total_frames, chunk_index))
    }

    /// Reassemble original payload from a vector of unmasked frames.
    pub fn unmask_payload(&self, frames: &[Vec<u8>]) -> Result<Vec<u8>, MaskingError> {
        if frames.is_empty() {
            return Err(MaskingError::DecodingFailed("No frames to unmask".into()));
        }

        let mut chunks: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut expected_total = None;

        for frame in frames {
            let (payload, total, idx) = self.unmask_frame(frame)?;
            match expected_total {
                None => expected_total = Some(total),
                Some(exp) => {
                    if total != exp {
                        return Err(MaskingError::DecodingFailed(
                            "Inconsistent total frame count across frames".into(),
                        ));
                    }
                }
            }
            chunks.push((idx, payload));
        }

        // Sort by chunk index
        chunks.sort_by_key(|(idx, _)| *idx);

        // Verify completeness
        let total = expected_total.unwrap_or(chunks.len());
        if chunks.len() != total {
            return Err(MaskingError::DecodingFailed(
                format!("Missing frames: expected {}, got {}", total, chunks.len()),
            ));
        }

        // Reassemble
        let mut result = Vec::with_capacity(chunks.iter().map(|(_, p)| p.len()).sum());
        for (_, payload) in &chunks {
            result.extend_from_slice(payload);
        }

        Ok(result)
    }

    /// Compute a simple checksum for frame integrity.
    fn compute_checksum(&self, data: &[u8]) -> u16 {
        let mut hash: u32 = 5381;
        for &byte in data {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
        }
        (hash ^ (hash >> 16)) as u16
    }

    /// Get the current frame count.
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Get the current configuration.
    pub fn config(&self) -> &MaskerConfig {
        &self.config
    }
}

impl Default for TrafficMasker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header() -> SrtpHeader {
        SrtpHeader::new(100, 900_000, 0xDEAD_BEEF)
    }

    #[test]
    fn test_header_serialization_roundtrip() {
        let header = make_header();
        let bytes = header.serialize();
        assert_eq!(bytes.len(), 12);
        let decoded = SrtpHeader::deserialize(&bytes).unwrap();
        assert_eq!(decoded.sequence_number, header.sequence_number);
        assert_eq!(decoded.timestamp, header.timestamp);
        assert_eq!(decoded.ssrc, header.ssrc);
        assert_eq!(decoded.payload_type, header.payload_type);
    }

    #[test]
    fn test_header_too_small() {
        let short = [0u8; 8];
        match SrtpHeader::deserialize(&short) {
            Err(MaskingError::DecodingFailed(msg)) => assert!(msg.contains("Insufficient")),
            other => panic!("Expected DecodingFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_header_invalid_version() {
        let mut bytes = vec![0u8; 12];
        bytes[0] = 0x80; // version = 2, but let's set version = 0
        bytes[0] = 0x00;
        match SrtpHeader::deserialize(&bytes) {
            Err(MaskingError::DecodingFailed(msg)) => assert!(msg.contains("version")),
            other => panic!("Expected DecodingFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_masker_creation() {
        let masker = TrafficMasker::new();
        assert_eq!(masker.frame_count(), 0);
        assert_eq!(masker.config().max_payload_size, 1400);
    }

    #[test]
    fn test_masker_custom_config() {
        let config = MaskerConfig {
            max_payload_size: 500,
            noise_seed: 123,
            ssrc: 0xCAFE_BABE,
            clock_rate: 48_000,
        };
        let masker = TrafficMasker::with_config(config);
        assert_eq!(masker.config().max_payload_size, 500);
        assert_eq!(masker.config().ssrc, 0xCAFE_BABE);
    }

    #[test]
    fn test_mask_empty_payload_rejected() {
        let mut masker = TrafficMasker::new();
        match masker.mask_payload(&[]) {
            Err(MaskingError::InvalidConfig(msg)) => assert!(msg.contains("Empty")),
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_mask_small_payload() {
        let mut masker = TrafficMasker::new();
        let payload = b"hello ed2kIA";
        let frames = masker.mask_payload(payload).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(masker.frame_count(), 1);
    }

    #[test]
    fn test_mask_large_payload_fragments() {
        let config = MaskerConfig {
            max_payload_size: 64,
            ..MaskerConfig::default()
        };
        let mut masker = TrafficMasker::with_config(config);
        let payload = vec![0xAB; 200];
        let frames = masker.mask_payload(&payload).unwrap();
        // 200 / 64 = 4 frames (3 full + 1 partial)
        assert_eq!(frames.len(), 4);
        assert_eq!(masker.frame_count(), 4);
    }

    #[test]
    fn test_unmask_single_frame() {
        let mut masker = TrafficMasker::new();
        let payload = b"preservacion cooperativa";
        let frames = masker.mask_payload(payload).unwrap();
        let (extracted, total, idx) = masker.unmask_frame(&frames[0]).unwrap();
        assert_eq!(extracted, payload);
        assert_eq!(total, 1);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_unmask_payload_roundtrip() {
        let mut masker = TrafficMasker::new();
        let payload = (0..255u8).collect::<Vec<_>>();
        let frames = masker.mask_payload(&payload).unwrap();
        let recovered = masker.unmask_payload(&frames).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn test_unmask_fragmented_payload_roundtrip() {
        let config = MaskerConfig {
            max_payload_size: 32,
            ..MaskerConfig::default()
        };
        let mut masker = TrafficMasker::with_config(config);
        let payload = vec![0xFF; 100];
        let frames = masker.mask_payload(&payload).unwrap();
        assert_eq!(frames.len(), 4);
        let recovered = masker.unmask_payload(&frames).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn test_unmask_corrupted_frame() {
        let mut masker = TrafficMasker::new();
        let payload = b"test data";
        let mut frames = masker.mask_payload(payload).unwrap();
        // Corrupt a byte in the middle
        frames[0][15] ^= 0xFF;
        match masker.unmask_frame(&frames[0]) {
            Err(MaskingError::DecodingFailed(msg)) => assert!(msg.contains("Checksum")),
            other => panic!("Expected DecodingFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_unmask_empty_frames() {
        let masker = TrafficMasker::new();
        match masker.unmask_payload(&[]) {
            Err(MaskingError::DecodingFailed(msg)) => assert!(msg.contains("No frames")),
            other => panic!("Expected DecodingFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_unmask_missing_frames() {
        let config = MaskerConfig {
            max_payload_size: 32,
            ..MaskerConfig::default()
        };
        let mut masker = TrafficMasker::with_config(config);
        let payload = vec![0xAA; 100];
        let frames = masker.mask_payload(&payload).unwrap();
        // Drop last frame
        let incomplete = &frames[..frames.len() - 1];
        match masker.unmask_payload(incomplete) {
            Err(MaskingError::DecodingFailed(msg)) => assert!(msg.contains("Missing")),
            other => panic!("Expected DecodingFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_sequence_counter_increments() {
        let mut masker = TrafficMasker::new();
        masker.mask_payload(b"first").unwrap();
        let count_after_first = masker.frame_count();
        masker.mask_payload(b"second").unwrap();
        assert!(masker.frame_count() > count_after_first);
    }

    #[test]
    fn test_error_display() {
        match MaskingError::PayloadTooLarge(2000) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_default() {
        let masker = TrafficMasker::default();
        assert_eq!(masker.frame_count(), 0);
    }
}
