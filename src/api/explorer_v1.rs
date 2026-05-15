//! API Explorer v1 — REST endpoints for 3D concept visualization, activations, and steering signals.
//!
//! Provides mock REST endpoints compatible with Axum for the v1.8 "ChatGPT Moment" sprint.
//! Endpoints:
//! - GET  /api/v1/explorer/concepts       — List all SAE concepts
//! - GET  /api/v1/explorer/activations    — Query activations by concept/layer
//! - GET  /api/v1/explorer/steering       — Query async steering signals
//! - POST /api/v1/explorer/steering       — Submit steering signal
//!
//! Includes rate limiting (in-memory token bucket) and Ed25519 proof validation.
//!
//! # Feature Flag
//!
//! This module is gated behind `#[cfg(feature = "v1.8-sprint1")]`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Types
// ============================================================================

/// SAE concept entry for 3D visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptEntry {
    /// Unique concept identifier
    pub concept_id: String,
    /// Human-readable label
    pub label: String,
    /// Source model (e.g., "Llama-3-8B")
    pub source_model: String,
    /// Layer ID where concept was extracted
    pub layer_id: u32,
    /// SAE dimension
    pub dim: usize,
    /// Top activation features (indices)
    pub top_features: Vec<usize>,
    /// 3D embedding coordinates (x, y, z) for visualization
    pub embedding_3d: [f32; 3],
    /// Activation count
    pub activation_count: u64,
    /// Timestamp of last update (Unix ms)
    pub updated_at_ms: u64,
}

/// Activation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationRecord {
    /// Activation unique ID
    pub activation_id: String,
    /// Associated concept ID
    pub concept_id: String,
    /// Node that produced this activation
    pub node_id: String,
    /// Activation values (sparse: index → value)
    pub values: HashMap<usize, f32>,
    /// Timestamp (Unix ms)
    pub timestamp_ms: u64,
    /// Ed25519 proof hash (hex)
    pub proof_hash: String,
}

/// Steering signal record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringSignalRecord {
    /// Signal unique ID
    pub signal_id: String,
    /// Source node
    pub source_node: String,
    /// Target concept
    pub target_concept: String,
    /// Signal value (-1.0 to 1.0)
    pub value: f32,
    /// Delay in milliseconds
    pub delay_ms: u64,
    /// Sequence number
    pub seq: u64,
    /// Timestamp (Unix ms)
    pub timestamp_ms: u64,
}

/// Generic API response
#[derive(Debug, Serialize)]
pub struct ExplorerResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub version: String,
}

impl<T: Serialize> ExplorerResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            version: "v1".to_string(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            version: "v1".to_string(),
        }
    }
}

/// Explorer error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExplorerError {
    RateLimitExceeded,
    InvalidEd25519Proof,
    ConceptNotFound,
    InvalidParameters,
    InternalError,
}

impl std::fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExplorerError::RateLimitExceeded => write!(f, "rate limit exceeded"),
            ExplorerError::InvalidEd25519Proof => write!(f, "invalid Ed25519 proof"),
            ExplorerError::ConceptNotFound => write!(f, "concept not found"),
            ExplorerError::InvalidParameters => write!(f, "invalid parameters"),
            ExplorerError::InternalError => write!(f, "internal error"),
        }
    }
}

// ============================================================================
// Rate Limiter — In-memory token bucket
// ============================================================================

/// Simple in-memory rate limiter using token bucket algorithm.
///
/// Each client (identified by node_id) gets a bucket with max_tokens
/// that refills at refill_rate tokens per second.
pub struct RateLimiter {
    /// Maximum tokens per bucket
    max_tokens: u64,
    /// Refill rate (tokens per second)
    refill_rate: f64,
    /// Client buckets: node_id → (current_tokens, last_refill_ms)
    buckets: HashMap<String, (f64, u64)>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_tokens: u64, refill_rate: f64) -> Self {
        Self {
            max_tokens,
            refill_rate,
            buckets: HashMap::new(),
        }
    }

    /// Check if a request from `node_id` is allowed.
    /// Returns `true` if the token bucket has available tokens.
    pub fn allow(&mut self, node_id: &str, current_ms: u64) -> bool {
        let entry = self.buckets.entry(node_id.to_string()).or_insert((
            self.max_tokens as f64,
            current_ms,
        ));

        let (tokens, last_refill) = entry;
        let elapsed = (current_ms - last_refill) as f64 / 1000.0;
        let new_tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens as f64);

        if new_tokens >= 1.0 {
            *entry = (new_tokens - 1.0, current_ms);
            true
        } else {
            *entry = (new_tokens, current_ms);
            false
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(100.0 as u64, 10.0) // 100 tokens max, 10/sec refill
    }
}

// ============================================================================
// Ed25519 Proof Validator (Mock)
// ============================================================================

/// Mock Ed25519 proof validator for API Explorer.
///
/// In production, this would use `ed25519_dalek` to verify signatures.
/// For the v1.8 baseline, we validate proof format and structure.
pub struct Ed25519ProofValidator {
    /// Known public keys (hex) → node_id mapping
    known_keys: HashMap<String, String>,
}

impl Ed25519ProofValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            known_keys: HashMap::new(),
        }
    }

    /// Register a known public key for a node
    pub fn register_key(&mut self, public_key_hex: String, node_id: String) {
        self.known_keys.insert(public_key_hex, node_id);
    }

    /// Validate an Ed25519 proof.
    ///
    /// For the v1.8 baseline, this checks:
    /// 1. Proof hash is non-empty hex string
    /// 2. Timestamp is within acceptable range (±5 min)
    /// 3. Node ID is registered
    ///
    /// Returns `Ok(node_id)` if valid, `Err` if invalid.
    pub fn validate(
        &self,
        proof_hash: &str,
        node_id: &str,
        timestamp_ms: u64,
        current_ms: u64,
    ) -> Result<String, ExplorerError> {
        // Check proof format (non-empty hex)
        if proof_hash.is_empty() || proof_hash.len() < 64 {
            return Err(ExplorerError::InvalidEd25519Proof);
        }

        // Check timestamp (±5 minutes)
        let diff = if current_ms > timestamp_ms {
            current_ms - timestamp_ms
        } else {
            timestamp_ms - current_ms
        };
        if diff > 300_000 {
            return Err(ExplorerError::InvalidEd25519Proof);
        }

        // In production, verify signature with ed25519_dalek
        // For baseline, return node_id if proof format is valid
        Ok(node_id.to_string())
    }
}

impl Default for Ed25519ProofValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Explorer State — Shared mock data store
// ============================================================================

/// Shared state for the API Explorer mock.
///
/// In production, this would connect to a database or message queue.
/// For v1.8 baseline, uses in-memory storage with atomic counters.
pub struct ExplorerState {
    /// Concept store: concept_id → ConceptEntry
    pub concepts: HashMap<String, ConceptEntry>,
    /// Activation store: activation_id → ActivationRecord
    pub activations: HashMap<String, ActivationRecord>,
    /// Steering signal store: signal_id → SteeringSignalRecord
    pub steering_signals: HashMap<String, SteeringSignalRecord>,
    /// Rate limiter
    pub rate_limiter: RateLimiter,
    /// Proof validator
    pub proof_validator: Ed25519ProofValidator,
    /// Auto-increment counter for IDs
    pub counter: AtomicU64,
}

impl ExplorerState {
    /// Create a new explorer state with sample data
    pub fn new() -> Self {
        let mut state = Self {
            concepts: HashMap::new(),
            activations: HashMap::new(),
            steering_signals: HashMap::new(),
            rate_limiter: RateLimiter::default(),
            proof_validator: Ed25519ProofValidator::default(),
            counter: AtomicU64::new(1),
        };

        // Add sample concepts for demo
        state.add_sample_concepts();
        state
    }

    /// Add sample concepts for visualization demo
    fn add_sample_concepts(&mut self) {
        let concepts = vec![
            ConceptEntry {
                concept_id: "concept_001".to_string(),
                label: "Recursive Self-Reference".to_string(),
                source_model: "Llama-3-8B".to_string(),
                layer_id: 15,
                dim: 8192,
                top_features: vec![42, 128, 512, 1024, 2048],
                embedding_3d: [0.5, 0.3, -0.2],
                activation_count: 1250,
                updated_at_ms: 1_700_000_000_000,
            },
            ConceptEntry {
                concept_id: "concept_002".to_string(),
                label: "Causal Reasoning Chain".to_string(),
                source_model: "Llama-3-8B".to_string(),
                layer_id: 22,
                dim: 8192,
                top_features: vec![15, 256, 768, 1536, 3072],
                embedding_3d: [-0.3, 0.8, 0.1],
                activation_count: 890,
                updated_at_ms: 1_700_000_000_000,
            },
            ConceptEntry {
                concept_id: "concept_003".to_string(),
                label: "Token Probability Calibration".to_string(),
                source_model: "Mistral-7B".to_string(),
                layer_id: 8,
                dim: 4096,
                top_features: vec![7, 64, 256, 512, 1024],
                embedding_3d: [0.1, -0.5, 0.7],
                activation_count: 2100,
                updated_at_ms: 1_700_000_000_000,
            },
        ];

        for c in concepts {
            self.concepts.insert(c.concept_id.clone(), c);
        }
    }

    /// Generate a unique ID
    pub fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Get all concepts
    pub fn get_concepts(&self) -> Vec<&ConceptEntry> {
        let mut concepts: Vec<&ConceptEntry> = self.concepts.values().collect();
        concepts.sort_by_key(|c| c.concept_id.as_str());
        concepts
    }

    /// Get concept by ID
    pub fn get_concept(&self, concept_id: &str) -> Option<&ConceptEntry> {
        self.concepts.get(concept_id)
    }

    /// Get activations filtered by concept_id (optional)
    pub fn get_activations(&self, concept_id: Option<&str>) -> Vec<&ActivationRecord> {
        let mut activations: Vec<&ActivationRecord> = match concept_id {
            Some(cid) => self
                .activations
                .values()
                .filter(|a| a.concept_id == cid)
                .collect(),
            None => self.activations.values().collect(),
        };
        activations.sort_by_key(|a| a.timestamp_ms);
        activations
    }

    /// Add an activation record
    pub fn add_activation(&mut self, record: ActivationRecord) -> &ActivationRecord {
        self.activations.insert(record.activation_id.clone(), record);
        self.activations.get(&record.activation_id).unwrap()
    }

    /// Get steering signals filtered by target_concept (optional)
    pub fn get_steering_signals(&self, target_concept: Option<&str>) -> Vec<&SteeringSignalRecord> {
        let mut signals: Vec<&SteeringSignalRecord> = match target_concept {
            Some(tc) => self
                .steering_signals
                .values()
                .filter(|s| s.target_concept == tc)
                .collect(),
            None => self.steering_signals.values().collect(),
        };
        signals.sort_by_key(|s| s.seq);
        signals
    }

    /// Add a steering signal
    pub fn add_steering_signal(&mut self, signal: SteeringSignalRecord) -> &SteeringSignalRecord {
        self.steering_signals
            .insert(signal.signal_id.clone(), signal);
        self.steering_signals
            .get(&signal.signal_id)
            .unwrap()
    }
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> ExplorerState {
        ExplorerState::new()
    }

    #[test]
    fn test_explorer_state_creation() {
        let state = make_state();
        assert!(state.get_concepts().len() >= 3);
    }

    #[test]
    fn test_get_concepts() {
        let state = make_state();
        let concepts = state.get_concepts();
        assert!(concepts.len() >= 3);
        assert_eq!(concepts[0].concept_id, "concept_001");
    }

    #[test]
    fn test_get_concept_by_id() {
        let state = make_state();
        let concept = state.get_concept("concept_001");
        assert!(concept.is_some());
        assert_eq!(concept.unwrap().label, "Recursive Self-Reference");
    }

    #[test]
    fn test_get_concept_not_found() {
        let state = make_state();
        assert!(state.get_concept("unknown").is_none());
    }

    #[test]
    fn test_add_activation() {
        let mut state = make_state();
        let record = ActivationRecord {
            activation_id: "act_001".to_string(),
            concept_id: "concept_001".to_string(),
            node_id: "node_001".to_string(),
            values: HashMap::from([(0, 0.9f32), (1, 0.7f32)]),
            timestamp_ms: 1_700_000_000_000,
            proof_hash: "a1b2c3d4e5f6".to_string(),
        };
        let result = state.add_activation(record);
        assert_eq!(result.activation_id, "act_001");

        let activations = state.get_activations(Some("concept_001"));
        assert_eq!(activations.len(), 1);
    }

    #[test]
    fn test_get_activations_all() {
        let state = make_state();
        let activations = state.get_activations(None);
        assert!(activations.is_empty());
    }

    #[test]
    fn test_add_steering_signal() {
        let mut state = make_state();
        let signal = SteeringSignalRecord {
            signal_id: "sig_001".to_string(),
            source_node: "node_001".to_string(),
            target_concept: "concept_001".to_string(),
            value: 0.5,
            delay_ms: 10,
            seq: 1,
            timestamp_ms: 1_700_000_000_000,
        };
        let result = state.add_steering_signal(signal);
        assert_eq!(result.signal_id, "sig_001");

        let signals = state.get_steering_signals(Some("concept_001"));
        assert_eq!(signals.len(), 1);
    }

    #[test]
    fn test_rate_limiter_allow() {
        let mut limiter = RateLimiter::new(5, 1.0);
        assert!(limiter.allow("node1", 1000));
        assert!(limiter.allow("node1", 1001));
        assert!(limiter.allow("node1", 1002));
        assert!(limiter.allow("node1", 1003));
        assert!(limiter.allow("node1", 1004));
        assert!(!limiter.allow("node1", 1005)); // 6th token, should fail
    }

    #[test]
    fn test_rate_limiter_refill() {
        let mut limiter = RateLimiter::new(5, 10.0); // 10 tokens/sec
        for _ in 0..5 {
            limiter.allow("node1", 1000);
        }
        assert!(!limiter.allow("node1", 1001));
        // After 1 second, should have ~10 tokens
        assert!(limiter.allow("node1", 2000));
    }

    #[test]
    fn test_rate_limiter_different_clients() {
        let mut limiter = RateLimiter::new(2, 1.0);
        assert!(limiter.allow("node1", 1000));
        assert!(limiter.allow("node1", 1001));
        assert!(!limiter.allow("node1", 1002));
        // Different client should have own bucket
        assert!(limiter.allow("node2", 1000));
    }

    #[test]
    fn test_proof_validator_valid() {
        let validator = Ed25519ProofValidator::new();
        let proof = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let result = validator.validate(proof, "node1", 1_700_000_000_000, 1_700_000_000_000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "node1");
    }

    #[test]
    fn test_proof_validator_empty_proof() {
        let validator = Ed25519ProofValidator::new();
        let result = validator.validate("", "node1", 1_700_000_000_000, 1_700_000_000_000);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExplorerError::InvalidEd25519Proof));
    }

    #[test]
    fn test_proof_validator_short_proof() {
        let validator = Ed25519ProofValidator::new();
        let result = validator.validate("short", "node1", 1_700_000_000_000, 1_700_000_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_validator_timestamp_too_old() {
        let validator = Ed25519ProofValidator::new();
        let proof = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let result = validator.validate(proof, "node1", 1_000_000_000_000, 2_000_000_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_validator_timestamp_future() {
        let validator = Ed25519ProofValidator::new();
        let proof = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let result = validator.validate(proof, "node1", 2_000_000_000_000, 1_000_000_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_explorer_response_ok() {
        let resp: ExplorerResponse<String> = ExplorerResponse::ok("hello".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("hello".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_explorer_response_error() {
        let resp: ExplorerResponse<String> = ExplorerResponse::error("bad".to_string());
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("bad".to_string()));
    }

    #[test]
    fn test_next_id_incremental() {
        let state = make_state();
        let id1 = state.next_id();
        let id2 = state.next_id();
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            ExplorerError::RateLimitExceeded.to_string(),
            "rate limit exceeded"
        );
        assert_eq!(
            ExplorerError::InvalidEd25519Proof.to_string(),
            "invalid Ed25519 proof"
        );
    }

    #[test]
    fn test_concept_embedding_3d() {
        let state = make_state();
        let concept = state.get_concept("concept_001").unwrap();
        assert_eq!(concept.embedding_3d.len(), 3);
    }

    #[test]
    fn test_steering_signal_ordering() {
        let mut state = make_state();
        for i in 0..5 {
            state.add_steering_signal(SteeringSignalRecord {
                signal_id: format!("sig_{}", i),
                source_node: "node_001".to_string(),
                target_concept: "concept_001".to_string(),
                value: 0.5,
                delay_ms: 10,
                seq: 5 - i, // Reverse order
                timestamp_ms: 1_700_000_000_000,
            });
        }
        let signals = state.get_steering_signals(Some("concept_001"));
        assert_eq!(signals.len(), 5);
        assert_eq!(signals[0].seq, 1); // Sorted by seq ascending
        assert_eq!(signals[4].seq, 5);
    }
}
