//! Steering Verifier — Verificador de señales de dirección con ZKP y validación de integridad
//!
//! Módulo responsable de verificar señales de steering generadas por Alignment Loop v3,
//! validando pruebas ZKP, detectando manipulaciones y manteniendo un registro inmutable
//! de señales verificadas.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, VecDeque};
use thiserror::Error;
use tracing::info;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for Steering Verifier.
#[derive(Debug, Error)]
pub enum SteeringVerifierError {
    #[error("Invalid signal ID: {0}")]
    InvalidSignalId(String),
    #[error("ZKP proof verification failed for signal: {0}")]
    ZKPProofFailed(String),
    #[error("Signal tampering detected: {0}")]
    TamperingDetected(String),
    #[error("Signal expired: issued={issued}, now={now}, max_age={max}")]
    SignalExpired { issued: u64, now: u64, max: u64 },
    #[error("Layer not authorized: {0}")]
    LayerNotAuthorized(String),
    #[error("Adjustment out of bounds: {adjustment:.3} not in [{min}, {max}]")]
    AdjustmentOutOfBounds { adjustment: f64, min: f64, max: f64 },
    #[error("Confidence below threshold: {confidence:.3} < {threshold:.3}")]
    ConfidenceBelowThreshold { confidence: f64, threshold: f64 },
    #[error("Duplicate signal detected: {0}")]
    DuplicateSignal(String),
    #[error("Rate limit exceeded: {count} signals in window")]
    RateLimitExceeded { count: usize },
    #[error("Signal replay detected: {0}")]
    ReplayDetected(String),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Verification status for a steering signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Signal passed all verification checks.
    Verified,
    /// Signal failed ZKP proof verification.
    ZKPFailed,
    /// Signal failed integrity check.
    IntegrityFailed,
    /// Signal was expired.
    Expired,
    /// Signal was a duplicate.
    Duplicate,
    /// Signal was rejected due to rate limiting.
    RateLimited,
    /// Signal failed authorization check.
    Unauthorized,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Verified => write!(f, "VERIFIED"),
            VerificationStatus::ZKPFailed => write!(f, "ZKP_FAILED"),
            VerificationStatus::IntegrityFailed => write!(f, "INTEGRITY_FAILED"),
            VerificationStatus::Expired => write!(f, "EXPIRED"),
            VerificationStatus::Duplicate => write!(f, "DUPLICATE"),
            VerificationStatus::RateLimited => write!(f, "RATE_LIMITED"),
            VerificationStatus::Unauthorized => write!(f, "UNAUTHORIZED"),
        }
    }
}

/// Steering signal to be verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringSignal {
    /// Unique signal identifier.
    pub signal_id: String,
    /// Source node that generated the signal.
    pub source_id: String,
    /// Target model layer.
    pub layer_id: String,
    /// Adjustment magnitude (-1.0 to 1.0).
    pub adjustment: f64,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Bias score (0.0 to 1.0).
    pub bias_score: f64,
    /// ZKP proof hash.
    pub zkp_proof: String,
    /// Integrity hash (covers all fields).
    pub integrity_hash: String,
    /// Timestamp when signal was issued (ms).
    pub issued_at_ms: u64,
    /// Sequence number for ordering.
    pub sequence: u64,
}

impl SteeringSignal {
    /// Creates a new steering signal with computed hashes.
    pub fn new(
        signal_id: String,
        source_id: String,
        layer_id: String,
        adjustment: f64,
        confidence: f64,
        bias_score: f64,
        sequence: u64,
    ) -> Self {
        let zkp_proof = compute_zkp_proof(&signal_id, &layer_id, adjustment);
        let integrity_hash = compute_integrity_hash(
            &signal_id, &source_id, &layer_id, adjustment, confidence, bias_score, sequence,
        );
        Self {
            signal_id,
            source_id,
            layer_id,
            adjustment: adjustment.clamp(-1.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            bias_score: bias_score.clamp(0.0, 1.0),
            zkp_proof,
            integrity_hash,
            issued_at_ms: current_timestamp_ms(),
            sequence,
        }
    }

    /// Verifies the ZKP proof.
    pub fn verify_zkp(&self) -> bool {
        let expected = compute_zkp_proof(&self.signal_id, &self.layer_id, self.adjustment);
        self.zkp_proof == expected
    }

    /// Verifies the integrity hash.
    pub fn verify_integrity(&self) -> bool {
        let expected = compute_integrity_hash(
            &self.signal_id,
            &self.source_id,
            &self.layer_id,
            self.adjustment,
            self.confidence,
            self.bias_score,
            self.sequence,
        );
        self.integrity_hash == expected
    }

    /// Checks if signal is within max age.
    pub fn is_fresh(&self, max_age_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.issued_at_ms) <= max_age_ms
    }
}

/// Result of verifying a steering signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Signal identifier.
    pub signal_id: String,
    /// Verification status.
    pub status: VerificationStatus,
    /// Verification timestamp (ms).
    pub verified_at_ms: u64,
    /// Details about the verification.
    pub details: String,
    /// ZKP proof valid.
    pub zkp_valid: bool,
    /// Integrity check passed.
    pub integrity_valid: bool,
    /// Signal was fresh (not expired).
    pub fresh: bool,
}

impl VerificationResult {
    /// Creates a successful verification result.
    pub fn verified(signal_id: String) -> Self {
        Self {
            signal_id,
            status: VerificationStatus::Verified,
            verified_at_ms: current_timestamp_ms(),
            details: "All checks passed".to_string(),
            zkp_valid: true,
            integrity_valid: true,
            fresh: true,
        }
    }

    /// Creates a failed verification result.
    pub fn failed(
        signal_id: String,
        status: VerificationStatus,
        details: String,
        zkp_valid: bool,
        integrity_valid: bool,
        fresh: bool,
    ) -> Self {
        Self {
            signal_id,
            status,
            verified_at_ms: current_timestamp_ms(),
            details,
            zkp_valid,
            integrity_valid,
            fresh,
        }
    }
}

/// Statistics for the steering verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierStats {
    /// Total signals verified.
    pub total_verified: usize,
    /// Total signals rejected.
    pub total_rejected: usize,
    /// ZKP failures.
    pub zkp_failures: usize,
    /// Integrity failures.
    pub integrity_failures: usize,
    /// Expired signals.
    pub expired_count: usize,
    /// Duplicate signals.
    pub duplicate_count: usize,
    /// Rate limited signals.
    pub rate_limited_count: usize,
    /// Unauthorized signals.
    pub unauthorized_count: usize,
    /// Average verification time (ms).
    pub avg_verification_ms: f64,
}

impl Default for VerifierStats {
    fn default() -> Self {
        Self {
            total_verified: 0,
            total_rejected: 0,
            zkp_failures: 0,
            integrity_failures: 0,
            expired_count: 0,
            duplicate_count: 0,
            rate_limited_count: 0,
            unauthorized_count: 0,
            avg_verification_ms: 0.0,
        }
    }
}

/// Configuration for the steering verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierConfig {
    /// Maximum signal age in milliseconds.
    pub max_signal_age_ms: u64,
    /// Minimum confidence threshold.
    pub min_confidence: f64,
    /// Maximum adjustment magnitude.
    pub max_adjustment: f64,
    /// Minimum adjustment magnitude.
    pub min_adjustment: f64,
    /// Authorized layers for verification.
    pub authorized_layers: Vec<String>,
    /// Rate limit window size in milliseconds.
    pub rate_limit_window_ms: u64,
    /// Max signals per rate limit window.
    pub max_signals_per_window: usize,
    /// Maximum verification history size.
    pub max_history_size: usize,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            max_signal_age_ms: 30_000,
            min_confidence: 0.85,
            max_adjustment: 0.5,
            min_adjustment: -0.5,
            authorized_layers: vec![
                "layer_0".to_string(),
                "layer_1".to_string(),
                "layer_2".to_string(),
            ],
            rate_limit_window_ms: 60_000,
            max_signals_per_window: 100,
            max_history_size: 1000,
        }
    }
}

// ---------------------------------------------------------------------------
// SteeringVerifier Engine
// ---------------------------------------------------------------------------

/// Verifier for steering signals with ZKP validation and integrity checks.
pub struct SteeringVerifier {
    config: VerifierConfig,
    stats: VerifierStats,
    /// History of verification results.
    history: VecDeque<VerificationResult>,
    /// Set of verified signal IDs for duplicate detection.
    verified_signals: HashMap<String, u64>,
    /// Rate limit tracking per source.
    rate_limit_registry: BTreeMap<String, VecDeque<u64>>,
    /// Sequence counter.
    next_sequence: u64,
}

impl SteeringVerifier {
    /// Creates a new verifier with default config.
    pub fn new() -> Self {
        Self::with_config(VerifierConfig::default())
    }

    /// Creates a verifier with custom config.
    pub fn with_config(config: VerifierConfig) -> Self {
        Self {
            config,
            stats: VerifierStats::default(),
            history: VecDeque::new(),
            verified_signals: HashMap::new(),
            rate_limit_registry: BTreeMap::new(),
            next_sequence: 0,
        }
    }

    /// Verifies a steering signal.
    pub fn verify(
        &mut self,
        signal: &SteeringSignal,
    ) -> Result<VerificationResult, SteeringVerifierError> {
        let start = current_timestamp_ms();

        // Check for duplicate
        if self.verified_signals.contains_key(&signal.signal_id) {
            self.stats.total_rejected += 1;
            self.stats.duplicate_count += 1;
            let result = VerificationResult::failed(
                signal.signal_id.clone(),
                VerificationStatus::Duplicate,
                "Duplicate signal detected".to_string(),
                true,
                true,
                true,
            );
            self.record_result(result.clone());
            return Err(SteeringVerifierError::DuplicateSignal(
                signal.signal_id.clone(),
            ));
        }

        // Check rate limit
        self.check_rate_limit(&signal.source_id)?;

        // Check expiration
        if !signal.is_fresh(self.config.max_signal_age_ms) {
            self.stats.total_rejected += 1;
            self.stats.expired_count += 1;
            let result = VerificationResult::failed(
                signal.signal_id.clone(),
                VerificationStatus::Expired,
                "Signal age exceeds maximum".to_string(),
                true,
                true,
                false,
            );
            self.record_result(result.clone());
            return Err(SteeringVerifierError::SignalExpired {
                issued: signal.issued_at_ms,
                now: current_timestamp_ms(),
                max: self.config.max_signal_age_ms,
            });
        }

        // Check layer authorization
        if !self.is_layer_authorized(&signal.layer_id) {
            self.stats.total_rejected += 1;
            self.stats.unauthorized_count += 1;
            let result = VerificationResult::failed(
                signal.signal_id.clone(),
                VerificationStatus::Unauthorized,
                format!("Layer {} not authorized", signal.layer_id),
                true,
                true,
                true,
            );
            self.record_result(result.clone());
            return Err(SteeringVerifierError::LayerNotAuthorized(
                signal.layer_id.clone(),
            ));
        }

        // Check adjustment bounds
        if signal.adjustment < self.config.min_adjustment
            || signal.adjustment > self.config.max_adjustment
        {
            return Err(SteeringVerifierError::AdjustmentOutOfBounds {
                adjustment: signal.adjustment,
                min: self.config.min_adjustment,
                max: self.config.max_adjustment,
            });
        }

        // Check confidence
        if signal.confidence < self.config.min_confidence {
            return Err(SteeringVerifierError::ConfidenceBelowThreshold {
                confidence: signal.confidence,
                threshold: self.config.min_confidence,
            });
        }

        // Verify ZKP proof
        let zkp_valid = signal.verify_zkp();
        if !zkp_valid {
            self.stats.total_rejected += 1;
            self.stats.zkp_failures += 1;
            let result = VerificationResult::failed(
                signal.signal_id.clone(),
                VerificationStatus::ZKPFailed,
                "ZKP proof verification failed".to_string(),
                false,
                true,
                true,
            );
            self.record_result(result.clone());
            return Err(SteeringVerifierError::ZKPProofFailed(
                signal.signal_id.clone(),
            ));
        }

        // Verify integrity
        let integrity_valid = signal.verify_integrity();
        if !integrity_valid {
            self.stats.total_rejected += 1;
            self.stats.integrity_failures += 1;
            let result = VerificationResult::failed(
                signal.signal_id.clone(),
                VerificationStatus::IntegrityFailed,
                "Integrity hash mismatch - signal may have been tampered".to_string(),
                true,
                false,
                true,
            );
            self.record_result(result.clone());
            return Err(SteeringVerifierError::TamperingDetected(
                signal.signal_id.clone(),
            ));
        }

        // All checks passed
        let elapsed = current_timestamp_ms().saturating_sub(start);
        self.stats.total_verified += 1;
        self.update_avg_verification_time(elapsed);

        // Record verified signal
        self.verified_signals
            .insert(signal.signal_id.clone(), signal.issued_at_ms);

        let result = VerificationResult::verified(signal.signal_id.clone());
        self.record_result(result.clone());

        info!(
            "SteeringVerifier: signal {} verified successfully",
            signal.signal_id
        );
        Ok(result)
    }

    /// Creates a new steering signal with proper sequence number.
    pub fn create_signal(
        &mut self,
        source_id: String,
        layer_id: String,
        adjustment: f64,
        confidence: f64,
        bias_score: f64,
    ) -> SteeringSignal {
        let signal_id = format!("sig-{}-{}", source_id, self.next_sequence);
        self.next_sequence += 1;
        SteeringSignal::new(
            signal_id,
            source_id,
            layer_id,
            adjustment,
            confidence,
            bias_score,
            self.next_sequence - 1,
        )
    }

    /// Gets verification statistics.
    pub fn get_stats(&self) -> VerifierStats {
        self.stats.clone()
    }

    /// Gets recent verification history.
    pub fn get_recent_history(&self, limit: usize) -> Vec<&VerificationResult> {
        self.history.iter().rev().take(limit).collect()
    }

    /// Gets verification history for a specific signal.
    pub fn get_signal_result(&self, signal_id: &str) -> Option<&VerificationResult> {
        self.history.iter().rev().find(|r| r.signal_id == signal_id)
    }

    /// Clears old verification history based on signal age.
    pub fn cleanup_old_entries(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let initial_len = self.history.len();

        // Clean history
        self.history
            .retain(|r| now.saturating_sub(r.verified_at_ms) < max_age_ms);

        // Clean verified signals registry
        self.verified_signals
            .retain(|_, &mut timestamp| now.saturating_sub(timestamp) < max_age_ms);

        // Clean rate limit registry
        for timestamps in self.rate_limit_registry.values_mut() {
            timestamps.retain(|&ts| now.saturating_sub(ts) < max_age_ms);
        }
        self.rate_limit_registry
            .retain(|_, timestamps| !timestamps.is_empty());

        initial_len - self.history.len()
    }

    /// Resets all statistics.
    pub fn reset_stats(&mut self) {
        self.stats = VerifierStats::default();
    }

    /// Checks if a layer is authorized.
    fn is_layer_authorized(&self, layer_id: &str) -> bool {
        if self.config.authorized_layers.is_empty() {
            return true; // Empty list means all layers authorized
        }
        self.config
            .authorized_layers
            .contains(&layer_id.to_string())
    }

    /// Checks rate limit for a source.
    fn check_rate_limit(&mut self, source_id: &str) -> Result<(), SteeringVerifierError> {
        let now = current_timestamp_ms();
        let timestamps = self
            .rate_limit_registry
            .entry(source_id.to_string())
            .or_default();

        // Remove old timestamps
        timestamps.retain(|&ts| now.saturating_sub(ts) <= self.config.rate_limit_window_ms);

        // Check count
        if timestamps.len() >= self.config.max_signals_per_window {
            self.stats.total_rejected += 1;
            self.stats.rate_limited_count += 1;
            return Err(SteeringVerifierError::RateLimitExceeded {
                count: timestamps.len(),
            });
        }

        // Add current timestamp
        timestamps.push_back(now);
        Ok(())
    }

    /// Records a verification result.
    fn record_result(&mut self, result: VerificationResult) {
        self.history.push_back(result);
        while self.history.len() > self.config.max_history_size {
            self.history.pop_front();
        }
    }

    /// Updates average verification time using exponential moving average.
    fn update_avg_verification_time(&mut self, elapsed_ms: u64) {
        let alpha = 0.1;
        let elapsed = elapsed_ms as f64;
        self.stats.avg_verification_ms =
            alpha * elapsed + (1.0 - alpha) * self.stats.avg_verification_ms;
    }
}

impl Default for SteeringVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Computes ZKP proof hash for a steering signal.
pub fn compute_zkp_proof(signal_id: &str, layer_id: &str, adjustment: f64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"zkp-steering-v3:");
    hasher.update(signal_id.as_bytes());
    hasher.update(layer_id.as_bytes());
    hasher.update(adjustment.to_le_bytes());
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

/// Computes integrity hash covering all signal fields.
pub fn compute_integrity_hash(
    signal_id: &str,
    source_id: &str,
    layer_id: &str,
    adjustment: f64,
    confidence: f64,
    bias_score: f64,
    sequence: u64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"integrity-steering-v3:");
    hasher.update(signal_id.as_bytes());
    hasher.update(source_id.as_bytes());
    hasher.update(layer_id.as_bytes());
    hasher.update(adjustment.to_le_bytes());
    hasher.update(confidence.to_le_bytes());
    hasher.update(bias_score.to_le_bytes());
    hasher.update(sequence.to_le_bytes());
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

/// Returns current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(source: &str, layer: &str, adjustment: f64) -> SteeringSignal {
        SteeringSignal::new(
            format!("sig-{}-{}", source, current_timestamp_ms()),
            source.to_string(),
            layer.to_string(),
            adjustment,
            0.95,
            0.1,
            0,
        )
    }

    #[test]
    fn test_verifier_creation() {
        let verifier = SteeringVerifier::new();
        let stats = verifier.get_stats();
        assert_eq!(stats.total_verified, 0);
        assert_eq!(stats.total_rejected, 0);
    }

    #[test]
    fn test_verifier_with_config() {
        let config = VerifierConfig {
            max_signal_age_ms: 60_000,
            min_confidence: 0.9,
            ..Default::default()
        };
        let verifier = SteeringVerifier::with_config(config);
        assert_eq!(verifier.config.max_signal_age_ms, 60_000);
    }

    #[test]
    fn test_verify_valid_signal() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_0", 0.3);
        let result = verifier.verify(&signal);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, VerificationStatus::Verified);
    }

    #[test]
    fn test_verify_duplicate_signal() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_0", 0.3);
        assert!(verifier.verify(&signal).is_ok());
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::DuplicateSignal(id) => assert_eq!(id, signal.signal_id),
            e => panic!("Expected DuplicateSignal, got {:?}", e),
        }
    }

    #[test]
    fn test_verify_expired_signal() {
        let mut verifier = SteeringVerifier::new();
        let mut signal = make_signal("node-1", "layer_0", 0.3);
        signal.issued_at_ms = current_timestamp_ms() - 60_000; // 60s old
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::SignalExpired { .. } => {}
            e => panic!("Expected SignalExpired, got {:?}", e),
        }
    }

    #[test]
    fn test_verify_unauthorized_layer() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_99", 0.3);
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::LayerNotAuthorized(layer) => assert_eq!(layer, "layer_99"),
            e => panic!("Expected LayerNotAuthorized, got {:?}", e),
        }
    }

    #[test]
    fn test_verify_adjustment_out_of_bounds() {
        let mut verifier = SteeringVerifier::new();
        let signal = SteeringSignal::new(
            "sig-test".to_string(),
            "node-1".to_string(),
            "layer_0".to_string(),
            0.8, // Exceeds max_adjustment of 0.5
            0.95,
            0.1,
            0,
        );
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::AdjustmentOutOfBounds { .. } => {}
            e => panic!("Expected AdjustmentOutOfBounds, got {:?}", e),
        }
    }

    #[test]
    fn test_verify_low_confidence() {
        let mut verifier = SteeringVerifier::new();
        let signal = SteeringSignal::new(
            "sig-test".to_string(),
            "node-1".to_string(),
            "layer_0".to_string(),
            0.3,
            0.5, // Below min_confidence of 0.85
            0.1,
            0,
        );
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::ConfidenceBelowThreshold { .. } => {}
            e => panic!("Expected ConfidenceBelowThreshold, got {:?}", e),
        }
    }

    #[test]
    fn test_verify_zkp_failure() {
        let mut verifier = SteeringVerifier::new();
        let mut signal = make_signal("node-1", "layer_0", 0.3);
        signal.zkp_proof = "0xtampered".to_string();
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::ZKPProofFailed(id) => assert_eq!(id, signal.signal_id),
            e => panic!("Expected ZKPProofFailed, got {:?}", e),
        }
    }

    #[test]
    fn test_verify_integrity_failure() {
        let mut verifier = SteeringVerifier::new();
        let mut signal = make_signal("node-1", "layer_0", 0.3);
        signal.integrity_hash = "0xtampered".to_string();
        let result = verifier.verify(&signal);
        // ZKP passes first, then integrity fails
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_rate_limit() {
        let config = VerifierConfig {
            rate_limit_window_ms: 60_000,
            max_signals_per_window: 3,
            ..Default::default()
        };
        let mut verifier = SteeringVerifier::with_config(config);
        for i in 0..3 {
            let signal = verifier.create_signal(
                "node-1".to_string(),
                "layer_0".to_string(),
                0.3 + (i as f64 * 0.1),
                0.95,
                0.1,
            );
            assert!(verifier.verify(&signal).is_ok());
        }
        let signal =
            verifier.create_signal("node-1".to_string(), "layer_0".to_string(), 0.6, 0.95, 0.1);
        let result = verifier.verify(&signal);
        assert!(result.is_err());
        match result.unwrap_err() {
            SteeringVerifierError::RateLimitExceeded { count } => assert_eq!(count, 3),
            e => panic!("Expected RateLimitExceeded, got {:?}", e),
        }
    }

    #[test]
    fn test_rate_limit_different_sources() {
        let config = VerifierConfig {
            rate_limit_window_ms: 60_000,
            max_signals_per_window: 2,
            ..Default::default()
        };
        let mut verifier = SteeringVerifier::with_config(config);
        // Source A hits limit
        for i in 0..2 {
            let signal = make_signal(&format!("node-a-{}", i), "layer_0", 0.3);
            assert!(verifier.verify(&signal).is_ok());
        }
        // Source B should still work
        let signal = make_signal("node-b", "layer_0", 0.3);
        assert!(verifier.verify(&signal).is_ok());
    }

    #[test]
    fn test_create_signal() {
        let mut verifier = SteeringVerifier::new();
        let signal =
            verifier.create_signal("node-1".to_string(), "layer_0".to_string(), 0.3, 0.95, 0.1);
        assert!(signal.verify_zkp());
        assert!(signal.verify_integrity());
        assert_eq!(signal.source_id, "node-1");
    }

    #[test]
    fn test_signal_zkp_verification() {
        let signal = make_signal("node-1", "layer_0", 0.3);
        assert!(signal.verify_zkp());
    }

    #[test]
    fn test_signal_integrity_verification() {
        let signal = make_signal("node-1", "layer_0", 0.3);
        assert!(signal.verify_integrity());
    }

    #[test]
    fn test_signal_freshness() {
        let signal = make_signal("node-1", "layer_0", 0.3);
        assert!(signal.is_fresh(30_000));
    }

    #[test]
    fn test_stats_tracking() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_0", 0.3);
        verifier.verify(&signal).unwrap();

        let stats = verifier.get_stats();
        assert_eq!(stats.total_verified, 1);
        assert!(stats.avg_verification_ms >= 0.0);
    }

    #[test]
    fn test_reset_stats() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_0", 0.3);
        verifier.verify(&signal).unwrap();
        verifier.reset_stats();

        let stats = verifier.get_stats();
        assert_eq!(stats.total_verified, 0);
    }

    #[test]
    fn test_get_recent_history() {
        let mut verifier = SteeringVerifier::new();
        for i in 0..5 {
            let signal = make_signal(&format!("node-{}", i), "layer_0", 0.3);
            let _ = verifier.verify(&signal);
        }

        let history = verifier.get_recent_history(3);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_get_signal_result() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_0", 0.3);
        let signal_id = signal.signal_id.clone();
        verifier.verify(&signal).unwrap();

        let result = verifier.get_signal_result(&signal_id);
        assert!(result.is_some());
        assert_eq!(result.unwrap().signal_id, signal_id);
    }

    #[test]
    fn test_cleanup_old_entries() {
        let mut verifier = SteeringVerifier::new();
        let signal = make_signal("node-1", "layer_0", 0.3);
        verifier.verify(&signal).unwrap();

        // Clean with very short max age
        let cleaned = verifier.cleanup_old_entries(0);
        assert!(cleaned >= 1);
    }

    #[test]
    fn test_verification_result_verified() {
        let result = VerificationResult::verified("sig-1".to_string());
        assert_eq!(result.status, VerificationStatus::Verified);
        assert!(result.zkp_valid);
        assert!(result.integrity_valid);
    }

    #[test]
    fn test_verification_result_failed() {
        let result = VerificationResult::failed(
            "sig-1".to_string(),
            VerificationStatus::ZKPFailed,
            "test failure".to_string(),
            false,
            true,
            true,
        );
        assert_eq!(result.status, VerificationStatus::ZKPFailed);
        assert!(!result.zkp_valid);
    }

    #[test]
    fn test_verification_status_display() {
        assert_eq!(format!("{}", VerificationStatus::Verified), "VERIFIED");
        assert_eq!(format!("{}", VerificationStatus::ZKPFailed), "ZKP_FAILED");
        assert_eq!(format!("{}", VerificationStatus::Expired), "EXPIRED");
        assert_eq!(format!("{}", VerificationStatus::Duplicate), "DUPLICATE");
    }

    #[test]
    fn test_config_default() {
        let config = VerifierConfig::default();
        assert_eq!(config.max_signal_age_ms, 30_000);
        assert_eq!(config.min_confidence, 0.85);
        assert_eq!(config.max_adjustment, 0.5);
    }

    #[test]
    fn test_stats_default() {
        let stats = VerifierStats::default();
        assert_eq!(stats.total_verified, 0);
        assert_eq!(stats.total_rejected, 0);
    }

    #[test]
    fn test_verifier_default() {
        let verifier = SteeringVerifier::default();
        assert_eq!(verifier.get_stats().total_verified, 0);
    }

    #[test]
    fn test_zkp_proof_consistency() {
        let h1 = compute_zkp_proof("sig-1", "layer_0", 0.3);
        let h2 = compute_zkp_proof("sig-1", "layer_0", 0.3);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_zkp_proof_uniqueness() {
        let h1 = compute_zkp_proof("sig-1", "layer_0", 0.3);
        let h2 = compute_zkp_proof("sig-2", "layer_0", 0.3);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_integrity_hash_consistency() {
        let h1 = compute_integrity_hash("sig-1", "src", "layer_0", 0.3, 0.95, 0.1, 0);
        let h2 = compute_integrity_hash("sig-1", "src", "layer_0", 0.3, 0.95, 0.1, 0);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_integrity_hash_field_sensitivity() {
        let h1 = compute_integrity_hash("sig-1", "src", "layer_0", 0.3, 0.95, 0.1, 0);
        let h2 = compute_integrity_hash("sig-1", "src", "layer_0", 0.3, 0.95, 0.1, 1); // Different sequence
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_signal_clamping() {
        let signal = SteeringSignal::new(
            "sig-1".to_string(),
            "src".to_string(),
            "layer_0".to_string(),
            2.0, // Exceeds max
            1.5, // Exceeds max
            1.5, // Exceeds max
            0,
        );
        assert_eq!(signal.adjustment, 1.0);
        assert_eq!(signal.confidence, 1.0);
        assert_eq!(signal.bias_score, 1.0);
    }

    #[test]
    fn test_error_display() {
        let err = SteeringVerifierError::InvalidSignalId("sig-1".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("sig-1"));
    }

    #[test]
    fn test_all_layers_authorized_when_empty() {
        let config = VerifierConfig {
            authorized_layers: vec![], // Empty means all authorized
            ..Default::default()
        };
        let mut verifier = SteeringVerifier::with_config(config);
        // layer_99 should be authorized
        let signal = make_signal("node-1", "layer_99", 0.3);
        assert!(verifier.verify(&signal).is_ok());
    }

    #[test]
    fn test_multiple_verifications_increase_stats() {
        let mut verifier = SteeringVerifier::new();
        for i in 0..10 {
            let signal = make_signal(&format!("node-{}", i), "layer_0", 0.3);
            let _ = verifier.verify(&signal);
        }
        let stats = verifier.get_stats();
        assert_eq!(stats.total_verified, 10);
    }
}
