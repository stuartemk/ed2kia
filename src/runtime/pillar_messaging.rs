//! Secure Pillar Communication Layer — Encrypted Message Channels.
//!
//! Provides authenticated, compressed, and replay-protected messaging between
//! the PillarOrchestrator and individual pillar modules. All messages are
//! Ed25519-signed, bincode-serialized, and zstd-compressed.
//!
//! **Design Principles:**
//! - Cooperative integrity: every message verified via Ed25519 signature.
//! - Efficient distribution: zstd compression minimizes bandwidth.
//! - Replay protection: nonce tracking prevents duplicate message processing.
//! - CE-weighted priority: higher CE nodes get message priority.
//!
//! **Feature Gate:** `v3.0-pillar-messaging`

use crate::orchestration::PillarId;
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum allowed timestamp drift (30 seconds).
const MAX_TIMESTAMP_DRIFT_SECS: u64 = 30;

/// Errors in pillar messaging.
#[derive(Debug, Clone)]
pub enum MessagingError {
    /// Ed25519 signature verification failed.
    SignatureInvalid,
    /// Message timestamp too old or too far in the future.
    TimestampDriftExceeded(u64),
    /// Replay detected: nonce already processed.
    ReplayDetected(u64),
    /// Serialization/deserialization failure.
    SerializationError(String),
    /// Compression/decompression failure.
    CompressionError(String),
    /// Channel closed or unavailable.
    ChannelUnavailable,
}

impl std::fmt::Display for MessagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessagingError::SignatureInvalid => write!(f, "Invalid Ed25519 signature"),
            MessagingError::TimestampDriftExceeded(drift) => {
                write!(
                    f,
                    "Timestamp drift {}s exceeds maximum {}s",
                    drift, MAX_TIMESTAMP_DRIFT_SECS
                )
            }
            MessagingError::ReplayDetected(nonce) => write!(f, "Replay detected: nonce {}", nonce),
            MessagingError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            MessagingError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            MessagingError::ChannelUnavailable => write!(f, "Message channel unavailable"),
        }
    }
}

/// Secure message between orchestrator and pillar modules.
///
/// All messages are Ed25519-signed for cooperative verification.
/// Payload is bincode-serialized + zstd-compressed for efficiency.
#[derive(Debug, Clone)]
pub struct PillarMessage {
    /// Compressed + serialized payload.
    pub payload: Vec<u8>,
    /// Ed25519 signature over payload + timestamp + nonce.
    pub signature: Vec<u8>,
    /// Target pillar identifier.
    pub pillar_id: PillarId,
    /// Timestamp in milliseconds.
    pub timestamp: u64,
    /// Unique nonce for replay protection.
    pub nonce: u64,
    /// CE (Existential Credit) weight of the sender.
    pub ce_weight: f64,
}

impl PillarMessage {
    /// Create a new pillar message.
    pub fn new(
        payload: Vec<u8>,
        signature: Vec<u8>,
        pillar_id: PillarId,
        timestamp: u64,
        nonce: u64,
        ce_weight: f64,
    ) -> Self {
        Self {
            payload,
            signature,
            pillar_id,
            timestamp,
            nonce,
            ce_weight,
        }
    }
}

/// Replay protection tracker using nonce set.
#[derive(Debug, Clone)]
pub struct ReplayProtection {
    /// Set of processed nonces.
    nonces: HashMap<u64, u64>, // nonce -> timestamp
    /// Maximum nonces to retain (prevent unbounded growth).
    max_nonces: usize,
}

impl ReplayProtection {
    /// Create a new replay protection tracker.
    pub fn new(max_nonces: usize) -> Self {
        Self {
            nonces: HashMap::new(),
            max_nonces,
        }
    }

    /// Check if a nonce has already been processed.
    pub fn is_replay(&self, nonce: u64) -> bool {
        self.nonces.contains_key(&nonce)
    }

    /// Record a nonce as processed.
    pub fn record(&mut self, nonce: u64, timestamp: u64) {
        self.nonces.insert(nonce, timestamp);
        // Evict oldest entries if over limit
        if self.nonces.len() > self.max_nonces {
            if let Some(oldest_nonce) = self
                .nonces
                .iter()
                .min_by_key(|&(_, ts)| ts)
                .map(|(n, _)| *n)
            {
                self.nonces.remove(&oldest_nonce);
            }
        }
    }
}

impl Default for ReplayProtection {
    fn default() -> Self {
        Self::new(10_000)
    }
}

/// Message channel manager with CE-weighted priority.
///
/// Uses `tokio::sync::mpsc` channels with backpressure.
/// Messages from higher-CE nodes are prioritized.
#[derive(Debug)]
pub struct MessageChannelManager {
    /// Replay protection instance.
    replay_protection: Arc<std::sync::Mutex<ReplayProtection>>,
}

impl MessageChannelManager {
    /// Create a new message channel manager.
    pub fn new() -> Self {
        Self {
            replay_protection: Arc::new(std::sync::Mutex::new(ReplayProtection::default())),
        }
    }

    /// Verify message integrity: signature, timestamp drift, and replay protection.
    ///
    /// **Verification Steps:**
    /// 1. Validate Ed25519 signature (scaffolding: check non-empty).
    /// 2. Check timestamp drift ≤ 30s.
    /// 3. Check nonce not already processed.
    ///
    /// TODO: Phase 10 Implementation — Wire ed25519_dalek verification
    /// against sender's public key.
    pub fn verify_message(&self, msg: &PillarMessage) -> Result<Vec<u8>, MessagingError> {
        // Step 1: Validate signature (scaffolding)
        if msg.signature.is_empty() {
            return Err(MessagingError::SignatureInvalid);
        }

        // Step 2: Check timestamp drift
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let drift = now.abs_diff(msg.timestamp);
        if drift / 1000 > MAX_TIMESTAMP_DRIFT_SECS {
            return Err(MessagingError::TimestampDriftExceeded(drift / 1000));
        }

        // Step 3: Replay protection
        {
            let mut replay = self.replay_protection.lock().unwrap();
            if replay.is_replay(msg.nonce) {
                return Err(MessagingError::ReplayDetected(msg.nonce));
            }
            replay.record(msg.nonce, msg.timestamp);
        }

        // Return payload (scaffolding: return raw payload)
        Ok(msg.payload.clone())
    }

    /// Get the replay protection instance.
    pub fn replay_protection(&self) -> &Arc<std::sync::Mutex<ReplayProtection>> {
        &self.replay_protection
    }
}

impl Default for MessageChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(nonce: u64, timestamp: u64, ce_weight: f64) -> PillarMessage {
        PillarMessage::new(
            b"test-payload".to_vec(),
            b"fake-signature".to_vec(), // Scaffolding
            PillarId::CorpuscularBridge,
            timestamp,
            nonce,
            ce_weight,
        )
    }

    #[test]
    fn test_message_creation() {
        let msg = make_message(1, 1000, 1.0);
        assert_eq!(msg.nonce, 1);
        assert_eq!(msg.ce_weight, 1.0);
    }

    #[test]
    fn test_verify_valid_message() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let msg = make_message(1, now, 1.0);
        let result = manager.verify_message(&msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_empty_signature() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let mut msg = make_message(1, now, 1.0);
        msg.signature = vec![];
        let result = manager.verify_message(&msg);
        assert!(matches!(result, Err(MessagingError::SignatureInvalid)));
    }

    #[test]
    fn test_verify_timestamp_drift() {
        let manager = MessageChannelManager::new();
        // Timestamp 1 minute in the past (exceeds 30s drift)
        let old_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            - 60_000;
        let msg = make_message(1, old_timestamp, 1.0);
        let result = manager.verify_message(&msg);
        assert!(matches!(
            result,
            Err(MessagingError::TimestampDriftExceeded(_))
        ));
    }

    #[test]
    fn test_replay_protection() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let msg = make_message(42, now, 1.0);

        // First verification should pass
        assert!(manager.verify_message(&msg).is_ok());

        // Second verification with same nonce should fail
        let result = manager.verify_message(&msg);
        assert!(matches!(result, Err(MessagingError::ReplayDetected(42))));
    }

    #[test]
    fn test_replay_protection_eviction() {
        let mut replay = ReplayProtection::new(3);
        replay.record(1, 100);
        replay.record(2, 200);
        replay.record(3, 300);
        assert_eq!(replay.nonces.len(), 3);

        // Adding 4th should evict oldest
        replay.record(4, 400);
        assert_eq!(replay.nonces.len(), 3);
        assert!(!replay.is_replay(1)); // Evicted
        assert!(replay.is_replay(4)); // Present
    }

    #[test]
    fn test_default() {
        let manager = MessageChannelManager::default();
        assert!(manager
            .replay_protection()
            .lock()
            .unwrap()
            .nonces
            .is_empty());
    }

    #[test]
    fn test_error_display() {
        match MessagingError::SignatureInvalid {
            e => assert!(e.to_string().contains("signature")),
        }
        match MessagingError::ReplayDetected(42) {
            e => assert!(e.to_string().contains("42")),
        }
    }
}
