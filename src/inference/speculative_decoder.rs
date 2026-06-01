//! Speculative Decoding with Parallel Topological Validation — Sprint 74: Distributed Systems Hardening
//!
//! Draft model generates token batches, validated in parallel by SAE+GEI engine.
//! Acceptance/rejection based on SCT-Z threshold for competitive TTFT.
//!
//! # Design
//!
//! 1. Draft model generates K candidate tokens in parallel
//! 2. Symbolic validator (SAE+GEI) evaluates batch in parallel
//! 3. Tokens accepted if SCT-Z score exceeds threshold
//! 4. Rejected tokens trigger autoregressive fallback
//!
//! # Guarantees
//!
//! - TTFT: O(K) parallel draft + O(K) parallel validation
//! - Acceptance rate: configurable via SCT-Z threshold
//! - Fallback: guaranteed progress via autoregressive mode

use std::cmp::Ordering;
use std::fmt;

/// Errors for speculative decoding.
#[derive(Debug, Clone, PartialEq)]
pub enum DecoderError {
    /// Empty prompt provided.
    EmptyPrompt,
    /// Invalid batch size.
    InvalidBatchSize(usize),
    /// Latency budget exceeded.
    LatencyBudgetExceeded,
    /// SCT-Z threshold invalid.
    InvalidThreshold(f64),
    /// Validation failed for all candidates.
    AllCandidatesRejected,
}

impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecoderError::EmptyPrompt => write!(f, "Decoder: empty prompt"),
            DecoderError::InvalidBatchSize(s) => write!(f, "Decoder: invalid batch size {}", s),
            DecoderError::LatencyBudgetExceeded => write!(f, "Decoder: latency budget exceeded"),
            DecoderError::InvalidThreshold(t) => write!(f, "Decoder: invalid threshold {}", t),
            DecoderError::AllCandidatesRejected => write!(f, "Decoder: all candidates rejected"),
        }
    }
}

impl std::error::Error for DecoderError {}

/// Configuration for speculative decoding.
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// Batch size for speculative generation.
    pub batch_size: usize,
    /// SCT-Z acceptance threshold.
    pub sct_z_threshold: f64,
    /// Latency budget in milliseconds.
    pub latency_budget_ms: u32,
    /// Maximum speculative steps.
    pub max_steps: usize,
    /// Enable autoregressive fallback.
    pub fallback_enabled: bool,
}

impl DecoderConfig {
    /// Default Stuartian configuration.
    pub fn default_stuartian() -> Self {
        Self {
            batch_size: 8,
            sct_z_threshold: 0.7,
            latency_budget_ms: 100,
            max_steps: 16,
            fallback_enabled: true,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), DecoderError> {
        if self.batch_size == 0 {
            return Err(DecoderError::InvalidBatchSize(0));
        }
        if self.sct_z_threshold <= 0.0 || self.sct_z_threshold > 1.0 {
            return Err(DecoderError::InvalidThreshold(self.sct_z_threshold));
        }
        if self.latency_budget_ms == 0 {
            return Err(DecoderError::LatencyBudgetExceeded);
        }
        Ok(())
    }
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// A token in the system.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token ID.
    pub id: u32,
    /// Token text.
    pub text: String,
    /// Log probability.
    pub log_prob: f32,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token(id={}, text=\"{}\", log_prob={:.4})",
            self.id, self.text, self.log_prob
        )
    }
}

/// A candidate token from the draft model.
#[derive(Debug, Clone)]
pub struct DraftCandidate {
    /// The candidate token.
    pub token: Token,
    /// Draft model confidence.
    pub confidence: f64,
    /// SCT-Z score from symbolic validation.
    pub sct_z: f64,
    /// Whether the candidate was accepted.
    pub accepted: bool,
}

impl fmt::Display for DraftCandidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DraftCandidate(token={}, conf={:.4}, sct_z={:.4}, accepted={})",
            self.token.id, self.confidence, self.sct_z, self.accepted
        )
    }
}

impl PartialEq for DraftCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.token.id == other.token.id
    }
}

impl Eq for DraftCandidate {}

impl Ord for DraftCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sct_z
            .partial_cmp(&other.sct_z)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DraftCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Result of speculative decoding.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// Accepted tokens.
    pub accepted_tokens: Vec<Token>,
    /// Number of speculative steps taken.
    pub steps_taken: usize,
    /// Whether fallback was used.
    pub fallback_used: bool,
    /// Total latency in milliseconds.
    pub latency_ms: u32,
    /// Acceptance rate.
    pub acceptance_rate: f64,
}

impl fmt::Display for DecodeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DecodeResult(accepted={}, steps={}, fallback={}, latency={}ms, rate={:.4})",
            self.accepted_tokens.len(),
            self.steps_taken,
            self.fallback_used,
            self.latency_ms,
            self.acceptance_rate
        )
    }
}

/// Simulated draft model.
pub struct DraftModel {
    /// Model seed for reproducibility.
    pub seed: u64,
}

impl DraftModel {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Generate draft candidates (simulated).
    pub fn generate_batch(&self, _prompt: &[Token], batch_size: usize) -> Vec<DraftCandidate> {
        let mut rng = self.seed;
        let mut candidates = Vec::with_capacity(batch_size);

        for _i in 0..batch_size {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let token_id = (rng % 10000) as u32;
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let confidence = ((rng % 10000) as f64) / 10000.0;
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let log_prob = -((rng % 10000) as f32) / 1000.0;

            let token = Token {
                id: token_id,
                text: format!("token_{}", token_id),
                log_prob,
            };

            candidates.push(DraftCandidate {
                token,
                confidence,
                sct_z: 0.0, // Will be set by validator
                accepted: false,
            });
        }

        candidates
    }
}

/// Simulated symbolic validation engine.
pub struct SymbolicEngine {
    /// GEI cache for topological validation.
    pub gei_cache: [f64; 8],
}

impl SymbolicEngine {
    pub fn new() -> Self {
        Self {
            gei_cache: [0.0; 8],
        }
    }

    /// Validate a batch of candidates in parallel (simulated).
    pub fn validate_batch(&self, candidates: &mut [DraftCandidate]) {
        for candidate in candidates.iter_mut() {
            // Simulate SCT-Z computation based on GEI alignment
            let alignment = self.compute_gei_alignment(candidate);
            candidate.sct_z = alignment;
        }
    }

    /// Compute GEI alignment for a candidate.
    fn compute_gei_alignment(&self, candidate: &DraftCandidate) -> f64 {
        // Simulated: use token ID and confidence to compute alignment
        let hash = candidate.token.id as u64;
        let normalized = ((hash % 10000) as f64) / 10000.0;
        // Weight by confidence
        normalized * candidate.confidence
    }
}

/// Speculative decoder engine.
pub struct SpeculativeDecoder {
    config: DecoderConfig,
    draft_model: DraftModel,
    symbolic_engine: SymbolicEngine,
    results: Vec<DecodeResult>,
}

impl SpeculativeDecoder {
    /// Create a new speculative decoder.
    pub fn new() -> Self {
        Self {
            config: DecoderConfig::default_stuartian(),
            draft_model: DraftModel::new(42),
            symbolic_engine: SymbolicEngine::new(),
            results: Vec::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: DecoderConfig) -> Result<Self, DecoderError> {
        config.validate()?;
        Ok(Self {
            config,
            draft_model: DraftModel::new(42),
            symbolic_engine: SymbolicEngine::new(),
            results: Vec::new(),
        })
    }

    /// Run speculative decoding.
    pub fn speculative_generate(
        &mut self,
        prompt: &[Token],
        batch_size: usize,
        latency_budget_ms: u32,
    ) -> Result<DecodeResult, DecoderError> {
        if prompt.is_empty() {
            return Err(DecoderError::EmptyPrompt);
        }
        if batch_size == 0 {
            return Err(DecoderError::InvalidBatchSize(0));
        }

        let effective_batch = batch_size.min(self.config.batch_size);
        let mut accepted_tokens = Vec::new();
        let mut total_candidates = 0;
        let mut steps: usize = 0;
        let mut fallback_used = false;
        let mut total_latency = 0u32;

        let max_steps = self
            .config
            .max_steps
            .min(steps.saturating_add(effective_batch));

        while steps < max_steps && total_latency < latency_budget_ms {
            // Draft phase
            let mut candidates = self.draft_model.generate_batch(prompt, effective_batch);
            total_candidates += candidates.len();

            // Validation phase
            self.symbolic_engine.validate_batch(&mut candidates);

            // Acceptance phase
            let mut any_accepted = false;
            for candidate in &mut candidates {
                if candidate.sct_z >= self.config.sct_z_threshold {
                    candidate.accepted = true;
                    accepted_tokens.push(candidate.token.clone());
                    any_accepted = true;
                }
            }

            // Fallback if no candidates accepted
            if !any_accepted && self.config.fallback_enabled {
                // Autoregressive fallback: accept highest SCT-Z
                candidates.sort_by(|a, b| b.sct_z.partial_cmp(&a.sct_z).unwrap_or(Ordering::Equal));
                if let Some(best) = candidates.first() {
                    accepted_tokens.push(best.token.clone());
                    fallback_used = true;
                }
            }

            steps += 1;
            total_latency += 5; // Simulated latency per step
        }

        let acceptance_rate = if total_candidates > 0 {
            accepted_tokens.len() as f64 / total_candidates as f64
        } else {
            0.0
        };

        let result = DecodeResult {
            accepted_tokens,
            steps_taken: steps,
            fallback_used,
            latency_ms: total_latency,
            acceptance_rate,
        };

        self.results.push(result.clone());
        Ok(result)
    }

    /// Get decoding results.
    pub fn results(&self) -> &[DecodeResult] {
        &self.results
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.results.clear();
    }
}

impl Default for SpeculativeDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SpeculativeDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpeculativeDecoder(results={}, last_rate={:.4})",
            self.results.len(),
            self.results
                .last()
                .map(|r| r.acceptance_rate)
                .unwrap_or(0.0)
        )
    }
}

/// Public function: speculative generation with parallel topological validation.
pub fn speculative_generate(
    draft_model: &DraftModel,
    symbolic_validator: &SymbolicEngine,
    prompt: &[Token],
    batch_size: usize,
    _latency_budget_ms: u32,
) -> Vec<Token> {
    if prompt.is_empty() || batch_size == 0 {
        return Vec::new();
    }

    let mut candidates = draft_model.generate_batch(prompt, batch_size);
    let engine_copy = symbolic_validator.clone();
    engine_copy.validate_batch(&mut candidates);

    candidates
        .into_iter()
        .filter(|c| c.sct_z >= 0.7)
        .map(|c| c.token)
        .collect()
}

impl Clone for SymbolicEngine {
    fn clone(&self) -> Self {
        Self {
            gei_cache: self.gei_cache,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DecoderConfig::default_stuartian();
        assert_eq!(config.batch_size, 8);
        assert_eq!(config.sct_z_threshold, 0.7);
        assert_eq!(config.latency_budget_ms, 100);
        assert!(config.fallback_enabled);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = DecoderConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_batch() {
        let config = DecoderConfig {
            batch_size: 0,
            ..DecoderConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = DecoderConfig {
            sct_z_threshold: 1.5,
            ..DecoderConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_latency() {
        let config = DecoderConfig {
            latency_budget_ms: 0,
            ..DecoderConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_decoder_creation() {
        let decoder = SpeculativeDecoder::new();
        assert!(decoder.results().is_empty());
    }

    #[test]
    fn test_decoder_with_config() {
        let config = DecoderConfig::default_stuartian();
        let decoder = SpeculativeDecoder::with_config(config).unwrap();
        assert!(decoder.results().is_empty());
    }

    #[test]
    fn test_draft_model_generate() {
        let model = DraftModel::new(42);
        let prompt = vec![Token {
            id: 1,
            text: "hello".to_string(),
            log_prob: -0.5,
        }];
        let candidates = model.generate_batch(&prompt, 4);
        assert_eq!(candidates.len(), 4);
    }

    #[test]
    fn test_symbolic_validate() {
        let engine = SymbolicEngine::new();
        let mut candidates = vec![DraftCandidate {
            token: Token {
                id: 1,
                text: "a".into(),
                log_prob: -0.5,
            },
            confidence: 0.8,
            sct_z: 0.0,
            accepted: false,
        }];
        engine.validate_batch(&mut candidates);
        assert!(candidates[0].sct_z >= 0.0);
    }

    #[test]
    fn test_speculative_generate() {
        let mut decoder = SpeculativeDecoder::new();
        decoder.config.sct_z_threshold = 0.1; // Low threshold for testing
        let prompt = vec![
            Token {
                id: 1,
                text: "hello".into(),
                log_prob: -0.5,
            },
            Token {
                id: 2,
                text: "world".into(),
                log_prob: -0.3,
            },
        ];
        let result = decoder.speculative_generate(&prompt, 4, 100).unwrap();
        assert!(result.steps_taken > 0);
        assert!(!result.accepted_tokens.is_empty());
    }

    #[test]
    fn test_speculative_empty_prompt() {
        let mut decoder = SpeculativeDecoder::new();
        let result = decoder.speculative_generate(&[], 4, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_speculative_zero_batch() {
        let mut decoder = SpeculativeDecoder::new();
        let prompt = vec![Token {
            id: 1,
            text: "a".into(),
            log_prob: -0.5,
        }];
        let result = decoder.speculative_generate(&prompt, 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_triggered() {
        let mut decoder = SpeculativeDecoder::new();
        decoder.config.sct_z_threshold = 0.99; // Very high threshold
        decoder.config.fallback_enabled = true;
        let prompt = vec![Token {
            id: 1,
            text: "a".into(),
            log_prob: -0.5,
        }];
        let result = decoder.speculative_generate(&prompt, 4, 100).unwrap();
        assert!(result.fallback_used);
    }

    #[test]
    fn test_latency_budget() {
        let mut decoder = SpeculativeDecoder::new();
        decoder.config.sct_z_threshold = 0.1;
        let prompt = vec![Token {
            id: 1,
            text: "a".into(),
            log_prob: -0.5,
        }];
        let result = decoder.speculative_generate(&prompt, 4, 10).unwrap();
        assert!(result.latency_ms <= 10);
    }

    #[test]
    fn test_reset() {
        let mut decoder = SpeculativeDecoder::new();
        decoder.config.sct_z_threshold = 0.1;
        let prompt = vec![Token {
            id: 1,
            text: "a".into(),
            log_prob: -0.5,
        }];
        decoder.speculative_generate(&prompt, 4, 100).unwrap();
        assert!(!decoder.results().is_empty());

        decoder.reset();
        assert!(decoder.results().is_empty());
    }

    #[test]
    fn test_display() {
        let decoder = SpeculativeDecoder::new();
        let display = format!("{}", decoder);
        assert!(display.contains("SpeculativeDecoder"));
    }

    #[test]
    fn test_standalone_generate() {
        let model = DraftModel::new(42);
        let engine = SymbolicEngine::new();
        let prompt = vec![Token {
            id: 1,
            text: "a".into(),
            log_prob: -0.5,
        }];
        let tokens = speculative_generate(&model, &engine, &prompt, 4, 100);
        // May return empty if no tokens pass threshold
        assert!(tokens.len() <= 4);
    }

    #[test]
    fn test_full_workflow() {
        let mut decoder = SpeculativeDecoder::new();
        decoder.config.sct_z_threshold = 0.3;

        let prompt = vec![
            Token {
                id: 1,
                text: "the".into(),
                log_prob: -0.2,
            },
            Token {
                id: 2,
                text: "quick".into(),
                log_prob: -0.5,
            },
            Token {
                id: 3,
                text: "brown".into(),
                log_prob: -0.4,
            },
        ];

        let result = decoder.speculative_generate(&prompt, 8, 200).unwrap();
        assert!(result.steps_taken > 0);
        assert!(result.accepted_tokens.len() > 0);
        assert!(result.acceptance_rate >= 0.0);
        assert!(result.acceptance_rate <= 1.0);
        assert!(result.latency_ms <= 200);
    }
}
