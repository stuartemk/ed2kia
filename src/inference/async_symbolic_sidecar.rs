//! Async Symbolic Sidecar — Sprint 75: Thermodynamic Hardening & Asynchronous Neuro-Symbolic Pivot
//!
//! LLM generates locally, ed2kIA validates post-hoc in a parallel thread.
//! Distributes SAE weights/corrections, not per-token activations.
//! Priority queue with latency budget enforcement and autoregressive fallback.

use std::collections::{BinaryHeap, HashMap};
use std::fmt;
use std::time::Instant;

/// Sidecar errors.
#[derive(Debug, Clone, PartialEq)]
pub enum SidecarError {
    BudgetExceeded(u32),
    EmptyInput,
    InvalidWeights(usize),
    QueueFull(usize),
}

impl fmt::Display for SidecarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SidecarError::BudgetExceeded(ms) => write!(f, "Latency budget exceeded: {}ms", ms),
            SidecarError::EmptyInput => write!(f, "Empty token input"),
            SidecarError::InvalidWeights(d) => write!(f, "Invalid SAE weights dimension: {}", d),
            SidecarError::QueueFull(cap) => write!(f, "Validation queue full: capacity {}", cap),
        }
    }
}

/// SAE weights for symbolic validation.
#[derive(Debug, Clone)]
pub struct SAEWeights {
    pub dimensions: usize,
    pub weights: Vec<f32>,
    pub bias: f32,
}

impl SAEWeights {
    pub fn new(dimensions: usize, weights: Vec<f32>, bias: f32) -> Result<Self, SidecarError> {
        if dimensions == 0 {
            return Err(SidecarError::InvalidWeights(0));
        }
        if weights.len() != dimensions {
            return Err(SidecarError::InvalidWeights(weights.len()));
        }
        Ok(Self {
            dimensions,
            weights,
            bias,
        })
    }

    pub fn dot(&self, activations: &[f32]) -> f32 {
        if activations.len() != self.dimensions {
            return 0.0;
        }
        self.weights
            .iter()
            .zip(activations.iter())
            .map(|(w, a)| w * a)
            .sum::<f32>()
            + self.bias
    }
}

/// Generated token with metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub id: u32,
    pub text: String,
    pub logprob: f32,
    pub activations: Vec<f32>,
}

impl Token {
    pub fn new(id: u32, text: String, logprob: f32, activations: Vec<f32>) -> Self {
        Self {
            id,
            text,
            logprob,
            activations,
        }
    }
}

/// Validation result for a batch of tokens.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub accepted: Vec<Token>,
    pub rejected: Vec<Token>,
    pub corrections: HashMap<u32, f32>,
    pub latency_ms: u32,
    pub within_budget: bool,
    pub used_fallback: bool,
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ValidationResult {{ accepted: {}, rejected: {}, latency: {}ms, budget: {}, fallback: {} }}",
            self.accepted.len(),
            self.rejected.len(),
            self.latency_ms,
            if self.within_budget { "ok" } else { "exceeded" },
            self.used_fallback
        )
    }
}

/// Sidecar configuration.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub latency_budget_ms: u32,
    pub max_queue_size: usize,
    pub correction_threshold: f32,
    pub min_logprob: f32,
    pub enable_fallback: bool,
}

impl SidecarConfig {
    pub fn default_stuartian() -> Self {
        Self {
            latency_budget_ms: 50,
            max_queue_size: 256,
            correction_threshold: 0.1,
            min_logprob: -5.0,
            enable_fallback: true,
        }
    }

    pub fn validate(&self) -> Result<(), SidecarError> {
        if self.latency_budget_ms == 0 {
            return Err(SidecarError::BudgetExceeded(0));
        }
        if self.max_queue_size == 0 {
            return Err(SidecarError::QueueFull(0));
        }
        Ok(())
    }
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Priority item for validation queue.
#[derive(Debug, Clone)]
struct QueueItem {
    pub token: Token,
    pub priority: f32,
    pub timestamp: u128,
}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority.partial_cmp(&other.priority) == Some(std::cmp::Ordering::Equal)
    }
}

impl Eq for QueueItem {}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .partial_cmp(&other.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Async Symbolic Sidecar engine.
pub struct AsyncSymbolicSidecar {
    pub config: SidecarConfig,
    queue: BinaryHeap<QueueItem>,
    sae_weights: Option<SAEWeights>,
    total_validated: usize,
    total_rejected: usize,
    total_fallback: usize,
}

impl AsyncSymbolicSidecar {
    pub fn new() -> Self {
        Self {
            config: SidecarConfig::default_stuartian(),
            queue: BinaryHeap::new(),
            sae_weights: None,
            total_validated: 0,
            total_rejected: 0,
            total_fallback: 0,
        }
    }

    pub fn with_config(config: SidecarConfig) -> Result<Self, SidecarError> {
        config.validate()?;
        Ok(Self {
            config,
            queue: BinaryHeap::new(),
            sae_weights: None,
            total_validated: 0,
            total_rejected: 0,
            total_fallback: 0,
        })
    }

    pub fn set_sae_weights(&mut self, weights: SAEWeights) {
        self.sae_weights = Some(weights);
    }

    /// Enqueue tokens for post-hoc validation.
    pub fn enqueue(&mut self, tokens: &[Token]) -> Result<usize, SidecarError> {
        if tokens.is_empty() {
            return Err(SidecarError::EmptyInput);
        }
        let mut enqueued = 0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        for token in tokens {
            if self.queue.len() >= self.config.max_queue_size {
                break;
            }
            self.queue.push(QueueItem {
                token: token.clone(),
                priority: -token.logprob,
                timestamp: now,
            });
            enqueued += 1;
        }
        Ok(enqueued)
    }

    /// Validate post-hoc: SAE correction + logprob filter + latency budget.
    pub fn validate_post_hoc(
        &mut self,
        generated_tokens: &[Token],
        sae_weights: &SAEWeights,
        latency_budget_ms: u32,
    ) -> ValidationResult {
        let start = Instant::now();
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        let mut corrections = HashMap::new();

        for token in generated_tokens {
            let elapsed = start.elapsed().as_millis() as u32;
            if elapsed > latency_budget_ms {
                // Budget exceeded — fallback to accept remaining
                if self.config.enable_fallback {
                    self.total_fallback += 1;
                    accepted.push(token.clone());
                } else {
                    rejected.push(token.clone());
                }
                continue;
            }

            // SAE correction
            let correction = sae_weights.dot(&token.activations);
            let corrected_logprob = token.logprob + correction;

            if (correction).abs() > self.config.correction_threshold {
                corrections.insert(token.id, correction);
            }

            // Acceptance: corrected logprob above threshold
            if corrected_logprob >= self.config.min_logprob {
                accepted.push(token.clone());
                self.total_validated += 1;
            } else {
                rejected.push(token.clone());
                self.total_rejected += 1;
            }
        }

        let latency_ms = start.elapsed().as_millis() as u32;
        let within_budget = latency_ms <= latency_budget_ms;
        let used_fallback = !within_budget && self.config.enable_fallback;

        ValidationResult {
            accepted,
            rejected,
            corrections,
            latency_ms,
            within_budget,
            used_fallback,
        }
    }

    /// Process queue: drain and validate in priority order.
    pub fn process_queue(&mut self, sae_weights: &SAEWeights) -> ValidationResult {
        let tokens: Vec<Token> = self.queue.drain().map(|item| item.token).collect();
        self.validate_post_hoc(&tokens, sae_weights, self.config.latency_budget_ms)
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.total_validated,
            self.total_rejected,
            self.total_fallback,
        )
    }

    pub fn reset(&mut self) {
        self.queue.clear();
        self.total_validated = 0;
        self.total_rejected = 0;
        self.total_fallback = 0;
    }
}

impl Default for AsyncSymbolicSidecar {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AsyncSymbolicSidecar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (validated, rejected, fallback) = self.stats();
        write!(
            f,
            "AsyncSymbolicSidecar {{ queue: {}, validated: {}, rejected: {}, fallback: {}, budget: {}ms }}",
            self.queue_len(),
            validated,
            rejected,
            fallback,
            self.config.latency_budget_ms
        )
    }
}

/// Standalone post-hoc validation function.
pub fn validate_post_hoc(
    generated_tokens: &[Token],
    sae_weights: &SAEWeights,
    latency_budget_ms: u32,
) -> ValidationResult {
    let mut engine = AsyncSymbolicSidecar::new();
    engine.validate_post_hoc(generated_tokens, sae_weights, latency_budget_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_token(id: u32, logprob: f32, dim: usize) -> Token {
        Token::new(id, format!("token_{}", id), logprob, vec![1.0; dim])
    }

    fn make_weights(dim: usize) -> SAEWeights {
        SAEWeights::new(dim, vec![0.1; dim], 0.0).unwrap()
    }

    #[test]
    fn test_config_default() {
        let config = SidecarConfig::default_stuartian();
        assert_eq!(config.latency_budget_ms, 50);
        assert_eq!(config.max_queue_size, 256);
    }

    #[test]
    fn test_config_validate_ok() {
        assert!(SidecarConfig::default_stuartian().validate().is_ok());
    }

    #[test]
    fn test_config_zero_budget() {
        let mut config = SidecarConfig::default_stuartian();
        config.latency_budget_ms = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_weights_creation() {
        let w = SAEWeights::new(8, vec![1.0; 8], 0.0).unwrap();
        assert_eq!(w.dimensions, 8);
    }

    #[test]
    fn test_weights_dimension_mismatch() {
        assert!(SAEWeights::new(8, vec![1.0; 4], 0.0).is_err());
    }

    #[test]
    fn test_weights_dot() {
        let w = SAEWeights::new(4, vec![1.0, 2.0, 3.0, 4.0], 0.0).unwrap();
        let result = w.dot(&[1.0, 1.0, 1.0, 1.0]);
        assert!((result - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_engine_creation() {
        let engine = AsyncSymbolicSidecar::new();
        assert_eq!(engine.queue_len(), 0);
    }

    #[test]
    fn test_enqueue() {
        let mut engine = AsyncSymbolicSidecar::new();
        let tokens = vec![make_token(1, -1.0, 8)];
        assert_eq!(engine.enqueue(&tokens).unwrap(), 1);
        assert_eq!(engine.queue_len(), 1);
    }

    #[test]
    fn test_enqueue_empty() {
        let mut engine = AsyncSymbolicSidecar::new();
        assert!(engine.enqueue(&[]).is_err());
    }

    #[test]
    fn test_enqueue_queue_full() {
        let mut engine = AsyncSymbolicSidecar::with_config(SidecarConfig {
            max_queue_size: 2,
            ..SidecarConfig::default_stuartian()
        })
        .unwrap();
        let tokens = vec![
            make_token(1, -1.0, 8),
            make_token(2, -2.0, 8),
            make_token(3, -3.0, 8),
        ];
        assert_eq!(engine.enqueue(&tokens).unwrap(), 2);
    }

    #[test]
    fn test_validate_accepts() {
        let mut engine = AsyncSymbolicSidecar::new();
        let tokens = vec![make_token(1, -1.0, 8)];
        let weights = make_weights(8);
        let result = engine.validate_post_hoc(&tokens, &weights, 100);
        assert_eq!(result.accepted.len(), 1);
        assert!(result.within_budget);
    }

    #[test]
    fn test_validate_rejects_low_logprob() {
        let mut engine = AsyncSymbolicSidecar::new();
        let tokens = vec![make_token(1, -10.0, 8)];
        let weights = make_weights(8);
        let result = engine.validate_post_hoc(&tokens, &weights, 100);
        assert_eq!(result.rejected.len(), 1);
    }

    #[test]
    fn test_validate_budget_exceeded() {
        let mut engine = AsyncSymbolicSidecar::with_config(SidecarConfig {
            latency_budget_ms: 1000,
            enable_fallback: true,
            ..SidecarConfig::default_stuartian()
        })
        .unwrap();
        // Many tokens to ensure processing time exceeds the tiny budget
        let tokens: Vec<Token> = (0..10000).map(|i| make_token(i, -1.0, 8)).collect();
        let weights = make_weights(8);
        // Pass latency_budget_ms=0 to validate_post_hoc to force immediate budget exceed
        let result = engine.validate_post_hoc(&tokens, &weights, 0);
        // With zero budget passed to validate, all tokens hit fallback or rejected
        assert!(!result.within_budget);
    }

    #[test]
    fn test_process_queue() {
        let mut engine = AsyncSymbolicSidecar::new();
        let tokens = vec![make_token(1, -1.0, 8), make_token(2, -2.0, 8)];
        engine.enqueue(&tokens).unwrap();
        let weights = make_weights(8);
        let result = engine.process_queue(&weights);
        assert_eq!(result.accepted.len(), 2);
        assert_eq!(engine.queue_len(), 0);
    }

    #[test]
    fn test_stats() {
        let mut engine = AsyncSymbolicSidecar::new();
        let tokens = vec![make_token(1, -1.0, 8)];
        let weights = make_weights(8);
        engine.validate_post_hoc(&tokens, &weights, 100);
        let (validated, rejected, fallback) = engine.stats();
        assert_eq!(validated, 1);
        assert_eq!(rejected, 0);
        assert_eq!(fallback, 0);
    }

    #[test]
    fn test_reset() {
        let mut engine = AsyncSymbolicSidecar::new();
        engine.enqueue(&[make_token(1, -1.0, 8)]).unwrap();
        engine.reset();
        assert_eq!(engine.queue_len(), 0);
        assert_eq!(engine.stats(), (0, 0, 0));
    }

    #[test]
    fn test_display() {
        let engine = AsyncSymbolicSidecar::new();
        let s = format!("{}", engine);
        assert!(s.contains("AsyncSymbolicSidecar"));
    }

    #[test]
    fn test_standalone_validate() {
        let tokens = vec![make_token(1, -1.0, 8)];
        let weights = make_weights(8);
        let result = validate_post_hoc(&tokens, &weights, 100);
        assert_eq!(result.accepted.len(), 1);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = AsyncSymbolicSidecar::new();
        let weights = make_weights(8);
        engine.set_sae_weights(weights.clone());

        let tokens = vec![
            make_token(1, -1.0, 8),
            make_token(2, -2.0, 8),
            make_token(3, -10.0, 8),
        ];
        engine.enqueue(&tokens).unwrap();
        let result = engine.process_queue(&weights);

        assert!(result.accepted.len() >= 2);
        assert!(result.rejected.len() >= 1);
        assert!(result.within_budget);
    }

    #[test]
    fn test_error_display() {
        let err = SidecarError::BudgetExceeded(50);
        assert!(format!("{}", err).contains("50"));
    }

    #[test]
    fn test_result_display() {
        let result = ValidationResult {
            accepted: vec![],
            rejected: vec![],
            corrections: HashMap::new(),
            latency_ms: 5,
            within_budget: true,
            used_fallback: false,
        };
        let s = format!("{}", result);
        assert!(s.contains("ValidationResult"));
    }
}
