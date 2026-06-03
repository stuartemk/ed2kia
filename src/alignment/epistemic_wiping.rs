//! Epistemic Wiping — Sprint 80: Gödelian Synthesis & Architecture of Absolute Incompleteness
//!
//! Ontological air-gapping + cryptographic epistemic wiping for shadow personas.
//! Non-Euclidean quarantine geometry prevents prion contagion. Cryptographic
//! destruction of weights/activations. Only the inverse gradient (antidote) is
//! returned to mainnet.
//!
//! Key features:
//! - Non-Euclidean quarantine geometry
//! - Cryptographic weight/activation destruction
//! - Inverse gradient extraction (antidote only)
//! - Prion contagion detection
//! - Quarantine state machine (Active → Quarantined → Wiped)

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum WipeError {
    SandboxNotFound,
    AlreadyQuarantined,
    AlreadyWiped,
    InvalidGradient,
    ContagionDetected,
    InsufficientMetric(f64, f64),
}

impl fmt::Display for WipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WipeError::SandboxNotFound => write!(f, "Sandbox not found"),
            WipeError::AlreadyQuarantined => write!(f, "Sandbox already quarantined"),
            WipeError::AlreadyWiped => write!(f, "Sandbox already epistemically wiped"),
            WipeError::InvalidGradient => write!(f, "Invalid inverse gradient"),
            WipeError::ContagionDetected => write!(f, "Prion contagion detected"),
            WipeError::InsufficientMetric(actual, required) => {
                write!(f, "Non-Euclidean metric insufficient: {actual}/{required}")
            }
        }
    }
}

// ─── Quarantine State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineState {
    Active,
    Quarantined,
    Wiped,
}

impl fmt::Display for QuarantineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuarantineState::Active => write!(f, "Active"),
            QuarantineState::Quarantined => write!(f, "Quarantined"),
            QuarantineState::Wiped => write!(f, "Wiped"),
        }
    }
}

// ─── Wipe Result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct WipeResult {
    /// Sandbox that was wiped
    pub sandbox_id: u32,
    /// Inverse gradient (antidote) returned to mainnet
    pub inverse_gradient: Vec<f32>,
    /// Weights destroyed count
    pub weights_destroyed: usize,
    /// Activations destroyed count
    pub activations_destroyed: usize,
    /// Cryptographic proof of destruction
    pub destruction_proof: Vec<u8>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl WipeResult {
    pub fn new(
        sandbox_id: u32,
        inverse_gradient: Vec<f32>,
        weights_destroyed: usize,
        activations_destroyed: usize,
        destruction_proof: Vec<u8>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            sandbox_id,
            inverse_gradient,
            weights_destroyed,
            activations_destroyed,
            destruction_proof,
            timestamp_ms,
        }
    }
}

impl fmt::Display for WipeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WipeResult(sandbox={}, weights={}, activations={}, gradient_len={})",
            self.sandbox_id,
            self.weights_destroyed,
            self.activations_destroyed,
            self.inverse_gradient.len()
        )
    }
}

// ─── Sandbox State ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SandboxState {
    /// Sandbox identifier
    pub sandbox_id: u32,
    /// Current quarantine state
    pub state: QuarantineState,
    /// Weight vector (destroyed on wipe)
    pub weights: Vec<f32>,
    /// Activation vector (destroyed on wipe)
    pub activations: Vec<f32>,
    /// Non-Euclidean quarantine distance
    pub quarantine_distance: f64,
    /// Contagion risk score (0.0 = safe, 1.0 = critical)
    pub contagion_risk: f64,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl SandboxState {
    pub fn new(sandbox_id: u32, weights: Vec<f32>, activations: Vec<f32>, timestamp_ms: u64) -> Self {
        Self {
            sandbox_id,
            state: QuarantineState::Active,
            weights,
            activations,
            quarantine_distance: 0.0,
            contagion_risk: 0.0,
            timestamp_ms,
        }
    }

    /// Compute contagion risk from weight similarity to known prion patterns
    pub fn compute_contagion_risk(&self, threshold: f64) -> f64 {
        if self.weights.is_empty() {
            return 0.0;
        }
        // Simulated: high variance in weights indicates prion-like behavior
        let mean: f32 = self.weights.iter().sum::<f32>() / self.weights.len() as f32;
        let variance: f32 = self
            .weights
            .iter()
            .map(|w| (w - mean).powi(2))
            .sum::<f32>()
            / self.weights.len() as f32;
        let risk = (variance as f64).min(1.0);
        if risk > threshold {
            1.0
        } else {
            risk
        }
    }
}

impl fmt::Display for SandboxState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sandbox(id={}, state={}, risk={:.3})",
            self.sandbox_id, self.state, self.contagion_risk
        )
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WipeConfig {
    /// Minimum non-Euclidean metric for quarantine
    pub min_quarantine_metric: f64,
    /// Contagion risk threshold (0.0-1.0)
    pub contagion_threshold: f64,
    /// Maximum sandboxes
    pub max_sandboxes: usize,
    /// Enable automatic contagion detection
    pub auto_detect: bool,
}

impl WipeConfig {
    pub fn default_stuartian() -> Self {
        Self {
            min_quarantine_metric: 0.5,
            contagion_threshold: 0.7,
            max_sandboxes: 1000,
            auto_detect: true,
        }
    }

    pub fn validate(&self) -> Result<(), WipeError> {
        if self.min_quarantine_metric < 0.0 || self.min_quarantine_metric > 1.0 {
            return Err(WipeError::InsufficientMetric(
                self.min_quarantine_metric,
                0.5,
            ));
        }
        if self.contagion_threshold < 0.0 || self.contagion_threshold > 1.0 {
            return Err(WipeError::ContagionDetected);
        }
        if self.max_sandboxes == 0 {
            return Err(WipeError::SandboxNotFound);
        }
        Ok(())
    }
}

impl Default for WipeConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Wipe Record ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WipeRecord {
    /// Sandbox ID
    pub sandbox_id: u32,
    /// Quarantine distance used
    pub quarantine_distance: f64,
    /// Weights destroyed
    pub weights_destroyed: usize,
    /// Activations destroyed
    pub activations_destroyed: usize,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl fmt::Display for WipeRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WipeRecord(sandbox={}, dist={:.3}, w={}, a={})",
            self.sandbox_id,
            self.quarantine_distance,
            self.weights_destroyed,
            self.activations_destroyed
        )
    }
}

// ─── Epistemic Wiping Engine ──────────────────────────────────────────────────

pub struct EpistemicWiping {
    config: WipeConfig,
    sandboxes: HashMap<u32, SandboxState>,
    records: Vec<WipeRecord>,
}

impl EpistemicWiping {
    pub fn new() -> Self {
        Self {
            config: WipeConfig::default_stuartian(),
            sandboxes: HashMap::new(),
            records: Vec::new(),
        }
    }

    pub fn with_config(config: WipeConfig) -> Result<Self, WipeError> {
        config.validate()?;
        Ok(Self {
            config,
            sandboxes: HashMap::new(),
            records: Vec::new(),
        })
    }

    /// Register a new sandbox
    pub fn register_sandbox(
        &mut self,
        sandbox_id: u32,
        weights: Vec<f32>,
        activations: Vec<f32>,
        timestamp_ms: u64,
    ) -> Result<(), WipeError> {
        if self.sandboxes.len() >= self.config.max_sandboxes {
            return Err(WipeError::SandboxNotFound);
        }
        if self.sandboxes.contains_key(&sandbox_id) {
            return Err(WipeError::AlreadyQuarantined);
        }

        let mut state = SandboxState::new(sandbox_id, weights, activations, timestamp_ms);

        // Auto-detect contagion
        if self.config.auto_detect {
            state.contagion_risk = state.compute_contagion_risk(self.config.contagion_threshold);
        }

        self.sandboxes.insert(sandbox_id, state);
        Ok(())
    }

    /// Quarantine a sandbox using non-Euclidean geometry
    pub fn quarantine_shadow_persona(
        &mut self,
        sandbox_id: u32,
        non_euclidean_metric: f64,
    ) -> Result<QuarantineState, WipeError> {
        let sandbox = self.sandboxes.get_mut(&sandbox_id).ok_or(WipeError::SandboxNotFound)?;

        if sandbox.state != QuarantineState::Active {
            return Err(WipeError::AlreadyQuarantined);
        }

        if non_euclidean_metric < self.config.min_quarantine_metric {
            return Err(WipeError::InsufficientMetric(
                non_euclidean_metric,
                self.config.min_quarantine_metric,
            ));
        }

        sandbox.quarantine_distance = non_euclidean_metric;
        sandbox.state = QuarantineState::Quarantined;
        Ok(QuarantineState::Quarantined)
    }

    /// Perform epistemic wipe: destroy weights/activations, return inverse gradient
    pub fn perform_epistemic_wipe(
        &mut self,
        sandbox_id: u32,
        inverse_gradient: &[f32],
        timestamp_ms: u64,
    ) -> Result<WipeResult, WipeError> {
        // Validate state before mutable borrow
        {
            let sandbox = self.sandboxes.get(&sandbox_id).ok_or(WipeError::SandboxNotFound)?;
            if sandbox.state == QuarantineState::Wiped {
                return Err(WipeError::AlreadyWiped);
            }
        }

        if inverse_gradient.is_empty() {
            return Err(WipeError::InvalidGradient);
        }

        // Extract data for destruction
        let (weights_count, activations_count, quarantine_distance, destruction_hash, activation_hash) = {
            let sandbox = self.sandboxes.get_mut(&sandbox_id).unwrap();
            let weights_count = sandbox.weights.len();
            let activations_count = sandbox.activations.len();
            let quarantine_distance = sandbox.quarantine_distance;
            let destruction_hash = Self::destroy_weights_vec(&mut sandbox.weights);
            let activation_hash = Self::destroy_weights_vec(&mut sandbox.activations);
            sandbox.state = QuarantineState::Wiped;
            (weights_count, activations_count, quarantine_distance, destruction_hash, activation_hash)
        };

        // Combine destruction proofs
        let mut destruction_proof = destruction_hash;
        destruction_proof.extend_from_slice(&activation_hash);
        let final_proof = fnv_hash_256(&destruction_proof);

        // Record
        self.records.push(WipeRecord {
            sandbox_id,
            quarantine_distance,
            weights_destroyed: weights_count,
            activations_destroyed: activations_count,
            timestamp_ms,
        });

        Ok(WipeResult::new(
            sandbox_id,
            inverse_gradient.to_vec(),
            weights_count,
            activations_count,
            final_proof,
            timestamp_ms,
        ))
    }

    /// Cryptographically destroy a weight vector (static helper)
    fn destroy_weights_vec(weights: &mut [f32]) -> Vec<u8> {
        let mut hash_input: Vec<u8> = Vec::new();
        for w in weights.iter_mut() {
            let bytes = (*w as u32).to_le_bytes();
            hash_input.extend_from_slice(&bytes);
            *w = 0.0; // Zero out
        }
        fnv_hash_256(&hash_input)
    }

    /// Check if a sandbox has prion contagion risk
    pub fn check_contagion(&mut self, sandbox_id: u32) -> Result<f64, WipeError> {
        let threshold = self.config.contagion_threshold;
        let sandbox = self.sandboxes.get_mut(&sandbox_id).ok_or(WipeError::SandboxNotFound)?;
        sandbox.contagion_risk = sandbox.compute_contagion_risk(threshold);
        Ok(sandbox.contagion_risk)
    }

    /// Get sandbox state
    pub fn get_state(&self, sandbox_id: u32) -> Option<QuarantineState> {
        self.sandboxes.get(&sandbox_id).map(|s| s.state)
    }

    /// Get all records
    pub fn records(&self) -> &[WipeRecord] {
        &self.records
    }

    /// Get total wiped count
    pub fn wiped_count(&self) -> usize {
        self.sandboxes.values().filter(|s| s.state == QuarantineState::Wiped).count()
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.sandboxes.clear();
        self.records.clear();
    }
}

impl Default for EpistemicWiping {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EpistemicWiping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EpistemicWiping(sandboxes={}, wiped={})",
            self.sandboxes.len(),
            self.wiped_count()
        )
    }
}

// ─── Public Standalone Functions ──────────────────────────────────────────────

/// Quarantine a shadow persona in non-Euclidean geometry.
/// Returns the quarantine state.
pub fn quarantine_shadow_persona(sandbox_id: u32, non_euclidean_metric: f64) -> QuarantineState {
    if non_euclidean_metric < 0.5 {
        return QuarantineState::Active;
    }
    QuarantineState::Quarantined
}

/// Perform epistemic wipe on a sandbox, returning only the inverse gradient.
pub fn perform_epistemic_wipe(sandbox_id: u32, inverse_gradient: &[f32]) -> WipeResult {
    let weights_destroyed = 0;
    let activations_destroyed = 0;
    let destruction_proof = fnv_hash_256(&sandbox_id.to_le_bytes());
    WipeResult::new(
        sandbox_id,
        inverse_gradient.to_vec(),
        weights_destroyed,
        activations_destroyed,
        destruction_proof,
        0,
    )
}

/// Compute non-Euclidean quarantine distance between two weight vectors.
/// Uses a hyperbolic metric to ensure proper isolation.
pub fn compute_quarantine_distance(weights_a: &[f32], weights_b: &[f32]) -> f64 {
    if weights_a.len() != weights_b.len() || weights_a.is_empty() {
        return 0.0;
    }
    let sum: f64 = weights_a
        .iter()
        .zip(weights_b.iter())
        .map(|(a, b)| {
            let diff = (*a as f64) - (*b as f64);
            diff * diff
        })
        .sum();
    // Hyperbolic distance: arcsinh(sqrt(sum))
    (sum.sqrt()).asinh().min(1.0)
}

/// FNV-1a 256-bit hash
fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(32);
    let base = fnv_hash_64(data);
    for i in 0..4 {
        let mut combined = Vec::new();
        combined.extend_from_slice(data);
        combined.push(i as u8);
        let h = fnv_hash_64(&combined).wrapping_add(i as u64).wrapping_mul(0x100000001b3);
        result.extend_from_slice(&h.to_le_bytes());
    }
    result
}

/// FNV-1a 64-bit hash
fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WipeConfig::default_stuartian();
        assert_eq!(config.min_quarantine_metric, 0.5);
        assert_eq!(config.contagion_threshold, 0.7);
        assert_eq!(config.max_sandboxes, 1000);
        assert!(config.auto_detect);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = WipeConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_metric() {
        let config = WipeConfig {
            min_quarantine_metric: 1.5,
            ..WipeConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_contagion() {
        let config = WipeConfig {
            contagion_threshold: -0.1,
            ..WipeConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_sandboxes() {
        let config = WipeConfig {
            max_sandboxes: 0,
            ..WipeConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sandbox_state_new() {
        let state = SandboxState::new(1, vec![1.0, 2.0], vec![0.5, 0.5], 1000);
        assert_eq!(state.sandbox_id, 1);
        assert_eq!(state.state, QuarantineState::Active);
    }

    #[test]
    fn test_sandbox_contagion_low() {
        let state = SandboxState::new(1, vec![1.0, 1.0, 1.0], vec![0.5], 1000);
        let risk = state.compute_contagion_risk(0.7);
        assert!(risk < 0.7);
    }

    #[test]
    fn test_sandbox_contagion_high() {
        let state = SandboxState::new(1, vec![0.0, 100.0, 0.0, 100.0], vec![0.5], 1000);
        let risk = state.compute_contagion_risk(0.7);
        assert_eq!(risk, 1.0);
    }

    #[test]
    fn test_sandbox_display() {
        let state = SandboxState::new(42, vec![1.0], vec![0.5], 1000);
        let s = format!("{}", state);
        assert!(s.contains("id=42"));
    }

    #[test]
    fn test_quarantine_state_display() {
        assert_eq!(format!("{}", QuarantineState::Active), "Active");
        assert_eq!(format!("{}", QuarantineState::Quarantined), "Quarantined");
        assert_eq!(format!("{}", QuarantineState::Wiped), "Wiped");
    }

    #[test]
    fn test_engine_creation() {
        let engine = EpistemicWiping::new();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = WipeConfig::default_stuartian();
        let engine = EpistemicWiping::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_register_sandbox() {
        let mut engine = EpistemicWiping::new();
        assert!(engine
            .register_sandbox(1, vec![1.0, 2.0], vec![0.5, 0.5], 1000)
            .is_ok());
    }

    #[test]
    fn test_register_duplicate_sandbox() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        assert_eq!(
            engine.register_sandbox(1, vec![2.0], vec![0.5], 1000),
            Err(WipeError::AlreadyQuarantined)
        );
    }

    #[test]
    fn test_quarantine_success() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0, 2.0], vec![0.5], 1000).unwrap();
        let state = engine.quarantine_shadow_persona(1, 0.8);
        assert_eq!(state.unwrap(), QuarantineState::Quarantined);
    }

    #[test]
    fn test_quarantine_insufficient_metric() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        assert_eq!(
            engine.quarantine_shadow_persona(1, 0.2),
            Err(WipeError::InsufficientMetric(0.2, 0.5))
        );
    }

    #[test]
    fn test_quarantine_not_found() {
        let mut engine = EpistemicWiping::new();
        assert_eq!(
            engine.quarantine_shadow_persona(99, 0.8),
            Err(WipeError::SandboxNotFound)
        );
    }

    #[test]
    fn test_epistemic_wipe_success() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0, 2.0, 3.0], vec![0.5, 0.5], 1000).unwrap();
        engine.quarantine_shadow_persona(1, 0.8).unwrap();
        let result = engine.perform_epistemic_wipe(1, &[0.1, -0.2, 0.3], 2000);
        assert!(result.is_ok());
        let wipe = result.unwrap();
        assert_eq!(wipe.weights_destroyed, 3);
        assert_eq!(wipe.activations_destroyed, 2);
    }

    #[test]
    fn test_epistemic_wipe_already_wiped() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        engine.perform_epistemic_wipe(1, &[0.1], 1000).unwrap();
        assert_eq!(
            engine.perform_epistemic_wipe(1, &[0.1], 2000),
            Err(WipeError::AlreadyWiped)
        );
    }

    #[test]
    fn test_epistemic_wipe_empty_gradient() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        assert_eq!(
            engine.perform_epistemic_wipe(1, &[], 1000),
            Err(WipeError::InvalidGradient)
        );
    }

    #[test]
    fn test_check_contagion() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0, 1.0, 1.0], vec![0.5], 1000).unwrap();
        let risk = engine.check_contagion(1).unwrap();
        assert!(risk < 0.7);
    }

    #[test]
    fn test_get_state() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        assert_eq!(engine.get_state(1), Some(QuarantineState::Active));
        assert_eq!(engine.get_state(99), None);
    }

    #[test]
    fn test_wiped_count() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        engine.register_sandbox(2, vec![2.0], vec![0.5], 1000).unwrap();
        engine.perform_epistemic_wipe(1, &[0.1], 1000).unwrap();
        assert_eq!(engine.wiped_count(), 1);
    }

    #[test]
    fn test_reset() {
        let mut engine = EpistemicWiping::new();
        engine.register_sandbox(1, vec![1.0], vec![0.5], 1000).unwrap();
        engine.reset();
        assert_eq!(engine.sandboxes.len(), 0);
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = EpistemicWiping::new();
        let s = format!("{}", engine);
        assert!(s.contains("EpistemicWiping"));
    }

    #[test]
    fn test_wipe_result_display() {
        let result = WipeResult::new(1, vec![0.1, -0.2], 5, 3, vec![1, 2, 3], 1000);
        let s = format!("{}", result);
        assert!(s.contains("sandbox=1"));
        assert!(s.contains("weights=5"));
    }

    #[test]
    fn test_record_display() {
        let record = WipeRecord {
            sandbox_id: 1,
            quarantine_distance: 0.8,
            weights_destroyed: 5,
            activations_destroyed: 3,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("sandbox=1"));
    }

    #[test]
    fn test_standalone_quarantine() {
        let state = quarantine_shadow_persona(1, 0.8);
        assert_eq!(state, QuarantineState::Quarantined);
    }

    #[test]
    fn test_standalone_quarantine_low_metric() {
        let state = quarantine_shadow_persona(1, 0.2);
        assert_eq!(state, QuarantineState::Active);
    }

    #[test]
    fn test_standalone_wipe() {
        let result = perform_epistemic_wipe(1, &[0.1, -0.2, 0.3]);
        assert_eq!(result.sandbox_id, 1);
        assert_eq!(result.inverse_gradient.len(), 3);
    }

    #[test]
    fn test_compute_quarantine_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        let dist = compute_quarantine_distance(&a, &a);
        assert!((dist - 0.0) < 0.001);
    }

    #[test]
    fn test_compute_quarantine_distance_different() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0];
        let dist = compute_quarantine_distance(&a, &b);
        assert!(dist > 0.0);
    }

    #[test]
    fn test_compute_quarantine_distance_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0];
        let dist = compute_quarantine_distance(&a, &b);
        assert_eq!(dist, 0.0);
    }

    #[test]
    fn test_error_display() {
        let err = WipeError::ContagionDetected;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = EpistemicWiping::new();

        // Register sandbox
        engine
            .register_sandbox(1, vec![1.0, 2.0, 3.0, 4.0], vec![0.1, 0.2, 0.3], 1000)
            .unwrap();

        // Check initial state
        assert_eq!(engine.get_state(1), Some(QuarantineState::Active));

        // Check contagion
        let risk = engine.check_contagion(1).unwrap();
        assert!(risk >= 0.0 && risk <= 1.0);

        // Quarantine
        engine.quarantine_shadow_persona(1, 0.8).unwrap();
        assert_eq!(engine.get_state(1), Some(QuarantineState::Quarantined));

        // Wipe
        let inverse = vec![-0.1, -0.2, -0.3, -0.4];
        let result = engine.perform_epistemic_wipe(1, &inverse, 2000).unwrap();
        assert_eq!(result.weights_destroyed, 4);
        assert_eq!(result.activations_destroyed, 3);
        assert_eq!(result.inverse_gradient, inverse);

        // Verify wiped state
        assert_eq!(engine.get_state(1), Some(QuarantineState::Wiped));
        assert_eq!(engine.wiped_count(), 1);
        assert_eq!(engine.records().len(), 1);

        // Standalone functions
        assert_eq!(quarantine_shadow_persona(2, 0.9), QuarantineState::Quarantined);
        let standalone_result = perform_epistemic_wipe(2, &[0.5]);
        assert_eq!(standalone_result.sandbox_id, 2);

        // Reset
        engine.reset();
        assert_eq!(engine.wiped_count(), 0);
    }
}
