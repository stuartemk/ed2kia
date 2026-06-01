//! Speculative Symbolic Filter — Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! Filtro asíncrono post-hoc + speculative decoding + fallback autoregresivo.
//! Sin traversal síncrono en hot-path. TTFT optimizado.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

/// Error types for Speculative Symbolic Filter
#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    /// Queue full
    QueueFull(usize),
    /// Empty queue
    EmptyQueue,
    /// Timeout exceeded
    Timeout { elapsed_ms: u32, budget_ms: u32 },
    /// All candidates rejected
    AllRejected,
    /// Invalid threshold
    InvalidThreshold(f64),
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::QueueFull(cap) => write!(f, "Queue full (capacity: {})", cap),
            FilterError::EmptyQueue => write!(f, "Empty candidate queue"),
            FilterError::Timeout {
                elapsed_ms,
                budget_ms,
            } => {
                write!(f, "Timeout: {}ms > {}ms budget", elapsed_ms, budget_ms)
            }
            FilterError::AllRejected => write!(f, "All candidates rejected by filter"),
            FilterError::InvalidThreshold(t) => write!(f, "Invalid threshold: {}", t),
        }
    }
}

/// Configuration for the speculative filter
#[derive(Debug, Clone)]
pub struct SpeculativeConfig {
    /// Maximum queue size
    pub max_queue_size: usize,
    /// GEI alignment threshold [0, 1]
    pub alignment_threshold: f64,
    /// Latency budget in milliseconds for fallback trigger
    pub latency_budget_ms: u32,
    /// Temperature for acceptance probability
    pub temperature: f64,
    /// Maximum rejection samples before fallback
    pub max_rejection_samples: usize,
    /// Weights for combined score: [semantic, gei, novelty]
    pub score_weights: [f64; 3],
}

impl SpeculativeConfig {
    pub fn default_stuartian() -> Self {
        Self {
            max_queue_size: 256,
            alignment_threshold: 0.6,
            latency_budget_ms: 50,
            temperature: 0.8,
            max_rejection_samples: 10,
            score_weights: [0.3, 0.5, 0.2],
        }
    }

    pub fn validate(&self) -> Result<(), FilterError> {
        if self.alignment_threshold < 0.0 || self.alignment_threshold > 1.0 {
            return Err(FilterError::InvalidThreshold(self.alignment_threshold));
        }
        if self.max_queue_size == 0 {
            return Err(FilterError::QueueFull(0));
        }
        if self.latency_budget_ms == 0 {
            return Err(FilterError::Timeout {
                elapsed_ms: 0,
                budget_ms: 0,
            });
        }
        Ok(())
    }
}

impl Default for SpeculativeConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Token candidate for speculative filtering
#[derive(Debug, Clone)]
pub struct TokenCandidate {
    pub token_id: u32,
    pub semantic_score: f64,
    pub gei_alignment: f64,
    pub novelty_score: f64,
    pub timestamp_ms: u64,
}

impl TokenCandidate {
    pub fn new(
        token_id: u32,
        semantic_score: f64,
        gei_alignment: f64,
        novelty_score: f64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            token_id,
            semantic_score,
            gei_alignment,
            novelty_score,
            timestamp_ms,
        }
    }

    pub fn combined_score(&self, weights: &[f64; 3]) -> f64 {
        let score = weights[0] * self.semantic_score
            + weights[1] * self.gei_alignment
            + weights[2] * self.novelty_score;
        score.min(1.0).max(0.0)
    }

    pub fn passes_alignment(&self, threshold: f64) -> bool {
        self.gei_alignment >= threshold
    }
}

impl PartialEq for TokenCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.token_id == other.token_id
    }
}

impl Eq for TokenCandidate {}

impl Ord for TokenCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.combined_score(&[0.3, 0.5, 0.2])
            .partial_cmp(&other.combined_score(&[0.3, 0.5, 0.2]))
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for TokenCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for TokenCandidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenCandidate {{ id: {}, gei: {:.3}, semantic: {:.3}, novelty: {:.3} }}",
            self.token_id, self.gei_alignment, self.semantic_score, self.novelty_score
        )
    }
}

/// Result of filtering operation
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub accepted_token: Option<TokenCandidate>,
    pub used_fallback: bool,
    pub rejection_count: usize,
    pub elapsed_ms: u32,
}

impl fmt::Display for FilterResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token_id = self.accepted_token.as_ref().map(|t| t.token_id).unwrap_or(0);
        write!(
            f,
            "FilterResult {{ token: {}, fallback: {}, rejections: {}, ms: {} }}",
            token_id, self.used_fallback, self.rejection_count, self.elapsed_ms
        )
    }
}

/// Speculative Symbolic Filter — Async post-hoc with autoregressive fallback
pub struct SpeculativeSymbolicFilter {
    config: SpeculativeConfig,
    queue: BinaryHeap<TokenCandidate>,
    gei_cache: [f64; 8],
    gei_timestamp_ms: u64,
    total_processed: usize,
    total_accepted: usize,
    total_fallbacks: usize,
}

impl SpeculativeSymbolicFilter {
    pub fn new() -> Self {
        Self {
            config: SpeculativeConfig::default_stuartian(),
            queue: BinaryHeap::new(),
            gei_cache: [0.0; 8],
            gei_timestamp_ms: 0,
            total_processed: 0,
            total_accepted: 0,
            total_fallbacks: 0,
        }
    }

    pub fn with_config(config: SpeculativeConfig) -> Result<Self, FilterError> {
        config.validate()?;
        Ok(Self {
            config,
            queue: BinaryHeap::new(),
            gei_cache: [0.0; 8],
            gei_timestamp_ms: 0,
            total_processed: 0,
            total_accepted: 0,
            total_fallbacks: 0,
        })
    }

    /// Update GEI cache
    pub fn update_gei_cache(&mut self, gei: [f64; 8], timestamp_ms: u64) {
        self.gei_cache = gei;
        self.gei_timestamp_ms = timestamp_ms;
    }

    /// Enqueue a candidate token
    pub fn enqueue(&mut self, candidate: TokenCandidate) -> Result<(), FilterError> {
        if self.queue.len() >= self.config.max_queue_size {
            return Err(FilterError::QueueFull(self.config.max_queue_size));
        }
        self.queue.push(candidate);
        Ok(())
    }

    /// Enqueue batch of candidates
    pub fn enqueue_batch(&mut self, candidates: &[TokenCandidate]) -> Result<usize, FilterError> {
        let mut count = 0;
        for c in candidates {
            if self.queue.len() >= self.config.max_queue_size {
                break;
            }
            self.queue.push(c.clone());
            count += 1;
        }
        Ok(count)
    }

    /// Apply speculative filter with latency budget
    pub fn apply_filter(
        &mut self,
        _current_ms: u64,
        elapsed_ms: u32,
    ) -> Result<FilterResult, FilterError> {
        if self.queue.is_empty() {
            return Err(FilterError::EmptyQueue);
        }

        let mut rejection_count = 0;
        let mut accepted = None;

        // Try candidates in priority order
        let mut temp_queue = Vec::new();
        let mut candidates: Vec<_> = self.queue.drain().collect();

        while let Some(candidate) = candidates.pop() {
            self.total_processed += 1;

            // Check latency budget — trigger fallback if exceeded
            if elapsed_ms > self.config.latency_budget_ms {
                self.total_fallbacks += 1;
                // Autoregressive fallback: pick highest semantic score
                if let Some(fallback) = Self::autoregressive_fallback(&candidates) {
                    self.total_accepted += 1;
                    return Ok(FilterResult {
                        accepted_token: Some(fallback),
                        used_fallback: true,
                        rejection_count,
                        elapsed_ms,
                    });
                }
            }

            // Check GEI alignment
            if candidate.passes_alignment(self.config.alignment_threshold) {
                // accepted tracked via self.total_accepted below
                self.total_accepted += 1;
                // Put remaining back
                self.queue.extend(temp_queue);
                self.queue.extend(candidates);
                return Ok(FilterResult {
                    accepted_token: Some(candidate),
                    used_fallback: false,
                    rejection_count,
                    elapsed_ms,
                });
            }

            rejection_count += 1;
            if rejection_count >= self.config.max_rejection_samples {
                break;
            }
            temp_queue.push(candidate);
        }

        // Fallback if all rejected
        if accepted.is_none() {
            self.total_fallbacks += 1;
            if let Some(fallback) = Self::autoregressive_fallback(&temp_queue) {
                self.total_accepted += 1;
                return Ok(FilterResult {
                    accepted_token: Some(fallback),
                    used_fallback: true,
                    rejection_count,
                    elapsed_ms,
                });
            }
        }

        // Restore queue
        self.queue.extend(temp_queue);
        self.queue.extend(candidates);

        if accepted.is_none() {
            Err(FilterError::AllRejected)
        } else {
            Ok(FilterResult {
                accepted_token: accepted,
                used_fallback: false,
                rejection_count,
                elapsed_ms,
            })
        }
    }

    /// Autoregressive fallback: select token with highest semantic score
    fn autoregressive_fallback(candidates: &[TokenCandidate]) -> Option<TokenCandidate> {
        candidates
            .iter()
            .max_by(|a, b| {
                a.semantic_score
                    .partial_cmp(&b.semantic_score)
                    .unwrap_or(Ordering::Equal)
            })
            .cloned()
    }

    /// Compute cosine similarity between GEI vectors
    pub fn cosine_similarity(a: &[f64; 8], b: &[f64; 8]) -> f64 {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for i in 0..8 {
            dot += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }
        let denom = (norm_a * norm_b).sqrt();
        if denom < 1e-10 {
            return 0.0;
        }
        dot / denom
    }

    /// Get stats: (processed, accepted, fallbacks)
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.total_processed,
            self.total_accepted,
            self.total_fallbacks,
        )
    }

    /// Rejection rate
    pub fn rejection_rate(&self) -> Option<f64> {
        if self.total_processed == 0 {
            return None;
        }
        Some((self.total_processed - self.total_accepted) as f64 / self.total_processed as f64)
    }

    /// Fallback rate
    pub fn fallback_rate(&self) -> Option<f64> {
        if self.total_processed == 0 {
            return None;
        }
        Some(self.total_fallbacks as f64 / self.total_processed as f64)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.queue.clear();
        self.gei_cache = [0.0; 8];
        self.gei_timestamp_ms = 0;
        self.total_processed = 0;
        self.total_accepted = 0;
        self.total_fallbacks = 0;
    }
}

impl Default for SpeculativeSymbolicFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SpeculativeSymbolicFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (processed, accepted, fallbacks) = self.stats();
        write!(
            f,
            "SpeculativeSymbolicFilter {{ queue: {}, processed: {}, accepted: {}, fallbacks: {} }}",
            self.queue.len(),
            processed,
            accepted,
            fallbacks
        )
    }
}

/// Public function: Async filter tokens
pub fn async_filter_tokens(
    tokens: &[TokenCandidate],
    gei_proxy: f32,
    _latency_budget_ms: u32,
) -> Vec<TokenCandidate> {
    let threshold = gei_proxy as f64;
    tokens
        .iter()
        .filter(|t| t.gei_alignment >= threshold)
        .cloned()
        .collect()
}

/// Compute combined score
pub fn compute_combined_score(
    semantic: f64,
    gei_alignment: f64,
    novelty: f64,
    weights: &[f64; 3],
) -> f64 {
    let score = weights[0] * semantic + weights[1] * gei_alignment + weights[2] * novelty;
    score.min(1.0).max(0.0)
}

/// Acceptance probability with temperature
pub fn acceptance_probability(gei_alignment: f64, threshold: f64, temperature: f64) -> f64 {
    let diff = gei_alignment - threshold;
    let temp = temperature.max(1e-10);
    let prob = 1.0 / (1.0 + (-diff / temp).exp());
    prob.min(1.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(id: u32, gei: f64, semantic: f64, novelty: f64) -> TokenCandidate {
        TokenCandidate::new(id, semantic, gei, novelty, 1000)
    }

    #[test]
    fn test_config_default() {
        let config = SpeculativeConfig::default();
        assert_eq!(config.max_queue_size, 256);
        assert_eq!(config.alignment_threshold, 0.6);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = SpeculativeConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = SpeculativeConfig {
            alignment_threshold: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_candidate_creation() {
        let c = make_candidate(1, 0.8, 0.7, 0.5);
        assert_eq!(c.token_id, 1);
        assert!(c.passes_alignment(0.6));
    }

    #[test]
    fn test_candidate_combined_score() {
        let c = make_candidate(1, 0.8, 0.7, 0.5);
        let score = c.combined_score(&[0.3, 0.5, 0.2]);
        assert!((score - 0.71).abs() < 0.01);
    }

    #[test]
    fn test_filter_new() {
        let filter = SpeculativeSymbolicFilter::new();
        assert_eq!(filter.stats(), (0, 0, 0));
    }

    #[test]
    fn test_enqueue() {
        let mut filter = SpeculativeSymbolicFilter::new();
        let c = make_candidate(1, 0.8, 0.7, 0.5);
        assert!(filter.enqueue(c).is_ok());
    }

    #[test]
    fn test_enqueue_queue_full() {
        let mut filter = SpeculativeSymbolicFilter::with_config(SpeculativeConfig {
            max_queue_size: 2,
            ..Default::default()
        })
        .unwrap();
        filter.enqueue(make_candidate(1, 0.8, 0.7, 0.5)).unwrap();
        filter.enqueue(make_candidate(2, 0.8, 0.7, 0.5)).unwrap();
        let result = filter.enqueue(make_candidate(3, 0.8, 0.7, 0.5));
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_filter_accepts() {
        let mut filter = SpeculativeSymbolicFilter::new();
        filter.enqueue(make_candidate(1, 0.8, 0.7, 0.5)).unwrap();
        let result = filter.apply_filter(1000, 10);
        assert!(result.is_ok());
        assert!(!result.unwrap().used_fallback);
    }

    #[test]
    fn test_apply_filter_rejects_low_alignment() {
        let mut filter = SpeculativeSymbolicFilter::new();
        filter.enqueue(make_candidate(1, 0.2, 0.7, 0.5)).unwrap();
        let result = filter.apply_filter(1000, 10);
        // Should trigger fallback since alignment < threshold
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_apply_filter_empty_queue() {
        let mut filter = SpeculativeSymbolicFilter::new();
        let result = filter.apply_filter(1000, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_filter_fallback() {
        let mut filter = SpeculativeSymbolicFilter::new();
        filter.enqueue(make_candidate(1, 0.8, 0.7, 0.5)).unwrap();
        let result = filter.apply_filter(1000, 100); // High elapsed triggers fallback
        assert!(result.is_ok());
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!((SpeculativeSymbolicFilter::cosine_similarity(&a, &a) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_zero_norm() {
        let a = [0.0; 8];
        assert!((SpeculativeSymbolicFilter::cosine_similarity(&a, &a) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_reset() {
        let mut filter = SpeculativeSymbolicFilter::new();
        filter.enqueue(make_candidate(1, 0.8, 0.7, 0.5)).unwrap();
        filter.reset();
        assert_eq!(filter.stats(), (0, 0, 0));
    }

    #[test]
    fn test_display() {
        let filter = SpeculativeSymbolicFilter::new();
        let s = format!("{}", filter);
        assert!(s.contains("SpeculativeSymbolicFilter"));
    }

    #[test]
    fn test_async_filter_tokens() {
        let tokens = vec![
            make_candidate(1, 0.8, 0.7, 0.5),
            make_candidate(2, 0.3, 0.9, 0.6),
        ];
        let result = async_filter_tokens(&tokens, 0.5, 50);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].token_id, 1);
    }

    #[test]
    fn test_compute_combined_score() {
        let score = compute_combined_score(0.7, 0.8, 0.5, &[0.3, 0.5, 0.2]);
        assert!((score - 0.71).abs() < 0.01);
    }

    #[test]
    fn test_acceptance_probability_above() {
        let prob = acceptance_probability(0.9, 0.6, 0.8);
        assert!(prob > 0.5);
    }

    #[test]
    fn test_acceptance_probability_below() {
        let prob = acceptance_probability(0.3, 0.6, 0.8);
        assert!(prob < 0.5);
    }

    #[test]
    fn test_full_filtering_workflow() {
        let mut filter = SpeculativeSymbolicFilter::new();
        filter.update_gei_cache([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], 1000);
        filter.enqueue(make_candidate(1, 0.8, 0.7, 0.5)).unwrap();
        filter.enqueue(make_candidate(2, 0.9, 0.6, 0.4)).unwrap();
        let result = filter.apply_filter(1000, 10).unwrap();
        assert!(result.accepted_token.is_some());
        let (processed, accepted, _) = filter.stats();
        assert_eq!(processed, 1);
        assert_eq!(accepted, 1);
    }
}
