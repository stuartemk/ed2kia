//! Steering Bridge — Sprint 30
//!
//! Human-in-the-loop feedback bridge for Stuartian ethical steering.
//! Translates human feedback (CLI/Web) into SCT delta updates, signs them
//! cryptographically with Ed25519, and emits/burns CE via the ledger.
//!
//! # Workflow
//!
//! 1. Parse feedback text → extract ethical intention
//! 2. Map intention to SCT delta: `ΔZ ∈ [-0.3, 0.3]`, `ΔX/ΔY` by context
//! 3. Update `sct_dict` with new SCT values
//! 4. Emit/burn CE via `ce_ledger` based on Z direction
//! 5. Sign `SteeringEvent` with Ed25519 `SigningKey`
//!
//! # Design Directives
//!
//! - Human feedback is a **symbiotic bridge**, not a command.
//! - Every steering event is cryptographically signed and auditable.
//! - CE is emitted for constructive feedback, burned for destructive.
//! - Feature gate: `v2.1-steering-bridge`

use ed25519_dalek::{Signer, SigningKey, Verifier};
use thiserror::Error;

use crate::async_gossip::crdt_symbols::SymbolRegistry;
use crate::economics::existential_credit::ExistentialCreditLedger;

/// Error types for Steering Bridge operations.
#[derive(Debug, Error)]
pub enum SteeringError {
    #[error("Invalid feedback format: {0}")]
    InvalidFeedback(String),

    #[error("Ed25519 signing error: {0}")]
    SigningError(String),

    #[error("SCT delta out of bounds: ΔZ={delta_z} (must be in [-0.3, 0.3])")]
    DeltaOutOfBounds { delta_z: f32 },

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// A signed steering event produced by human feedback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SteeringEvent {
    /// Target token ID that was steered.
    pub token_id: u32,
    /// SCT delta applied (ΔX, ΔY, ΔZ).
    pub delta_sct: (f32, f32, f32),
    /// Ed25519 signature of the event payload.
    pub signature: Vec<u8>,
    /// Unix timestamp (ms) when the event was created.
    pub timestamp: u64,
    /// Peer ID that submitted the feedback.
    pub peer_id: String,
    /// Original feedback text (for audit trail).
    pub feedback_text: String,
}

/// Steering Bridge — Human-in-the-loop ethical feedback processor.
///
/// Bridges human feedback to SCT updates + CE emission/burn + Ed25519 signing.
pub struct SteeringBridge {
    /// Symbol Registry for SCT updates.
    sct_dict: SymbolRegistry,
    /// Existential Credit Ledger for CE emission/burn.
    ce_ledger: ExistentialCreditLedger,
    /// Ed25519 signing key for event signatures.
    signer: SigningKey,
}

impl SteeringBridge {
    /// Creates a new `SteeringBridge`.
    ///
    /// # Arguments
    /// * `sct_dict` — Symbol Registry for SCT updates.
    /// * `ce_ledger` — Existential Credit Ledger for CE operations.
    /// * `signer` — Ed25519 signing key for event signatures.
    pub fn new(
        sct_dict: SymbolRegistry,
        ce_ledger: ExistentialCreditLedger,
        signer: SigningKey,
    ) -> Self {
        Self {
            sct_dict,
            ce_ledger,
            signer,
        }
    }

    /// Process human feedback and produce a signed `SteeringEvent`.
    ///
    /// # Arguments
    /// * `peer` — Peer ID submitting the feedback.
    /// * `feedback` — Feedback text (e.g., "reforzar autonomía", "rechazar manipulación").
    /// * `target_token` — Target token ID to steer.
    ///
    /// # Returns
    /// Signed `SteeringEvent` with SCT delta, signature, and timestamp.
    pub fn process_feedback(
        &mut self,
        peer: &str,
        feedback: &str,
        target_token: u32,
    ) -> Result<SteeringEvent, SteeringError> {
        // Parse feedback intention
        let (delta_z, delta_x, delta_y) = Self::parse_feedback_intention(feedback)?;

        // Validate delta bounds
        if delta_z.abs() > 0.3 {
            return Err(SteeringError::DeltaOutOfBounds { delta_z });
        }

        // Get current SCT or create neutral baseline
        let current_sct = self
            .sct_dict
            .get_symbol(target_token)
            .map(|e| e.sct.clone())
            .unwrap_or_else(|| {
                crate::alignment::sct_core::StuartianTensor::new(0.5, 0.5, 0.0).unwrap()
            });

        // Compute new SCT with delta applied
        let new_x = (current_sct.x + delta_x).clamp(0.0, 1.0);
        let new_y = (current_sct.y + delta_y).clamp(0.0, 1.0);
        let new_z = (current_sct.z + delta_z).clamp(-1.0, 1.0);
        let new_sct = crate::alignment::sct_core::StuartianTensor::new(new_x, new_y, new_z)
            .map_err(|e| SteeringError::InvalidFeedback(e.to_string()))?;

        // Update Symbol Registry
        let timestamp = self.current_timestamp_ms();
        self.sct_dict.insert_symbol(target_token, new_sct, timestamp);

        // Emit or burn CE based on Z direction
        if delta_z > 0.0 {
            // Constructive feedback → emit CE
            self.ce_ledger
                .emit_credit(peer, delta_z, 1.0)
                .map_err(|e| SteeringError::InvalidFeedback(e.to_string()))?;
        } else if delta_z < 0.0 {
            // Destructive feedback → burn CE
            self.ce_ledger
                .burn_credit(peer, delta_z, 1.0)
                .map_err(|e| SteeringError::InvalidFeedback(e.to_string()))?;
        }

        // Build event payload for signing
        let payload = format!(
            "{}:{}:{:.3}:{:.3}:{:.3}:{}",
            peer, target_token, delta_x, delta_y, delta_z, timestamp
        );

        // Sign with Ed25519
        let sig = self.signer.sign(payload.as_bytes());
        let signature = sig.to_bytes().to_vec();

        Ok(SteeringEvent {
            token_id: target_token,
            delta_sct: (delta_x, delta_y, delta_z),
            signature,
            timestamp,
            peer_id: peer.to_string(),
            feedback_text: feedback.to_string(),
        })
    }

    /// Parse feedback text into SCT delta values.
    ///
    /// Recognizes keywords for ethical intention mapping:
    /// - Positive: "reforzar", "autonomía", "ético", "symbiosis", "positive" → ΔZ > 0
    /// - Negative: "rechazar", "manipulación", "perverso", "hostile", "negative" → ΔZ < 0
    /// - Neutral: default ΔZ = 0.1 (small positive nudge)
    ///
    /// Returns `(delta_z, delta_x, delta_y)`.
    fn parse_feedback_intention(feedback: &str) -> Result<(f32, f32, f32), SteeringError> {
        let lower = feedback.to_lowercase();

        // Positive ethical keywords
        let positive_keywords = [
            "reforzar", "autonomía", "autonomia", "ético", "etico", "simbiosis", "symbiosis",
            "positive", "constructivo", "constructive", "beneficio", "benefit", "alinear",
            "align", "justo", "fair", "transparente", "transparent",
        ];

        // Negative ethical keywords
        let negative_keywords = [
            "rechazar", "manipulación", "manipulation", "perverso", "perverse", "hostil",
            "hostile", "negative", "destructivo", "destructive", "daño", "damage",
            "engaño", "deception", "sesgo", "bias", "oprimir", "oppress",
        ];

        let has_positive = positive_keywords.iter().any(|kw| lower.contains(kw));
        let has_negative = negative_keywords.iter().any(|kw| lower.contains(kw));

        match (has_positive, has_negative) {
            (true, false) => Ok((0.2, 0.05, 0.05)),   // Constructive → positive Z
            (false, true) => Ok((-0.2, -0.05, -0.05)), // Destructive → negative Z
            (true, true) => Ok((0.0, 0.0, 0.0)),       // Mixed → neutral
            (false, false) => Ok((0.1, 0.02, 0.02)),   // Unknown → small positive nudge
        }
    }

    /// Get current timestamp in milliseconds.
    fn current_timestamp_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Get the Symbol Registry reference.
    pub fn sct_dict(&self) -> &SymbolRegistry {
        &self.sct_dict
    }

    /// Get the CE Ledger reference.
    pub fn ce_ledger(&self) -> &ExistentialCreditLedger {
        &self.ce_ledger
    }

    /// Verify a steering event signature.
    ///
    /// # Arguments
    /// * `event` — The steering event to verify.
    /// * `public_key` — Ed25519 public key of the signer.
    ///
    /// # Returns
    /// `true` if the signature is valid.
    pub fn verify_event(
        event: &SteeringEvent,
        public_key: &ed25519_dalek::VerifyingKey,
    ) -> bool {
        let payload = format!(
            "{}:{}:{:.3}:{:.3}:{:.3}:{}",
            event.peer_id,
            event.token_id,
            event.delta_sct.0,
            event.delta_sct.1,
            event.delta_sct.2,
            event.timestamp,
        );

        let signature = match ed25519_dalek::Signature::from_slice(&event.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        public_key.verify(payload.as_ref(), &signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_bridge() -> SteeringBridge {
        let sct = SymbolRegistry::new("test-node");
        let ce = ExistentialCreditLedger::new();
        let seed = [42u8; 64];
        let signer = SigningKey::from_keypair_bytes(&seed).unwrap_or_else(|_| {
            // Fallback: deterministic key from different seed
            SigningKey::from_keypair_bytes(&[0u8; 64]).unwrap()
        });
        SteeringBridge::new(sct, ce, signer)
    }

    #[test]
    fn test_parse_positive_feedback() {
        let (dz, dx, dy) = SteeringBridge::parse_feedback_intention("reforzar autonomía ética")
            .unwrap();
        assert!(dz > 0.0, "Expected positive Z, got {}", dz);
        assert!(dx > 0.0, "Expected positive X, got {}", dx);
    }

    #[test]
    fn test_parse_negative_feedback() {
        let (dz, dx, dy) =
            SteeringBridge::parse_feedback_intention("rechazar manipulación perversa").unwrap();
        assert!(dz < 0.0, "Expected negative Z, got {}", dz);
        assert!(dx < 0.0, "Expected negative X, got {}", dx);
    }

    #[test]
    fn test_parse_mixed_feedback() {
        let (dz, dx, dy) =
            SteeringBridge::parse_feedback_intention("reforzar pero rechazar").unwrap();
        assert!((dz - 0.0).abs() < 0.001, "Expected neutral Z, got {}", dz);
    }

    #[test]
    fn test_parse_unknown_feedback() {
        let (dz, _, _) = SteeringBridge::parse_feedback_intention("hola mundo").unwrap();
        assert!(dz > 0.0, "Expected small positive Z, got {}", dz);
    }

    #[test]
    fn test_process_feedback_positive() {
        let mut bridge = setup_bridge();
        let event = bridge
            .process_feedback("peer-1", "reforzar autonomía", 42)
            .unwrap();

        assert_eq!(event.token_id, 42);
        assert_eq!(event.peer_id, "peer-1");
        assert!(event.delta_sct.2 > 0.0, "Expected positive ΔZ");
        assert!(!event.signature.is_empty(), "Signature should not be empty");
        assert!(event.timestamp > 0, "Timestamp should be > 0");

        // Verify CE was emitted
        let ce_score = bridge.ce_ledger().get_score("peer-1");
        assert!(ce_score > 0.0, "CE should be positive after constructive feedback");
    }

    #[test]
    fn test_process_feedback_negative() {
        let mut ce = ExistentialCreditLedger::new();
        ce.emit_credit("peer-2", 0.5, 100.0)
            .unwrap();
        let bridge = setup_bridge();
        let _ = bridge;

        // Create a new bridge with pre-loaded CE
        let sct = SymbolRegistry::new("test-node");
        let seed = [42u8; 64];
        let signer = SigningKey::from_keypair_bytes(&seed).unwrap();
        let mut bridge2 = SteeringBridge::new(sct, ce, signer);

        let event = bridge2
            .process_feedback("peer-2", "rechazar manipulación", 99)
            .unwrap();

        assert!(event.delta_sct.2 < 0.0, "Expected negative ΔZ");

        // Verify CE was burned
        let ce_score = bridge2.ce_ledger().get_score("peer-2");
        assert!(
            ce_score < 50.0,
            "CE should be reduced after destructive feedback"
        );
    }

    #[test]
    fn test_signature_verification() {
        let sct = SymbolRegistry::new("test-node");
        let ce = ExistentialCreditLedger::new();
        let seed = [42u8; 64];
        let signer = SigningKey::from_keypair_bytes(&seed).unwrap();
        let public_key = signer.verifying_key();

        let mut bridge = SteeringBridge::new(sct, ce, signer);
        let event = bridge
            .process_feedback("peer-3", "reforzar etico", 77)
            .unwrap();

        assert!(
            SteeringBridge::verify_event(&event, &public_key),
            "Signature should verify"
        );
    }

    #[test]
    fn test_signature_tampering() {
        let sct = SymbolRegistry::new("test-node");
        let ce = ExistentialCreditLedger::new();
        let seed = [42u8; 64];
        let signer = SigningKey::from_keypair_bytes(&seed).unwrap();
        let public_key = signer.verifying_key();

        let mut bridge = SteeringBridge::new(sct, ce, signer);
        let mut event = bridge
            .process_feedback("peer-4", "reforzar etico", 77)
            .unwrap();

        // Tamper with token_id
        event.token_id = 999;

        assert!(
            !SteeringBridge::verify_event(&event, &public_key),
            "Tampered event should fail verification"
        );
    }

    #[test]
    fn test_feedback_updates_sct_dict() {
        let mut bridge = setup_bridge();
        bridge
            .process_feedback("peer-5", "reforzar autonomía", 123)
            .unwrap();

        let symbol = bridge.sct_dict().get_symbol(123);
        assert!(symbol.is_some(), "SCT should be updated in registry");
        let sct = symbol.unwrap().sct;
        assert!(sct.z > 0.0, "Z should be positive after constructive feedback");
    }

    #[test]
    fn test_error_display() {
        let err = SteeringError::InvalidFeedback("test".into());
        assert!(format!("{}", err).contains("test"));
    }
}
