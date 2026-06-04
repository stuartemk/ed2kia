//! Streaming Symbolic Filter â€” Sprint 72: Asymptotic Optimization & Hard Sybil Resistance
//!
//! Async rejection sampling, priority queue, autoregressive fallback.
//! Post-hoc filtering with GEI alignment bounds.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

// â”€â”€â”€ Error Types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    EmptyInput,
    AllRejected,
    Timeout(u64),
    InvalidThreshold(f64),
    QueueFull(usize),
    InvalidConfig,
    AutoregressiveFailed,
    GEIMismatch,
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::EmptyInput => write!(f, "Input sequence is empty"),
            FilterError::AllRejected => write!(f, "All candidates rejected by filter"),
            FilterError::Timeout(ms) => write!(f, "Filter timeout after {}ms", ms),
            FilterError::InvalidThreshold(t) => {
                write!(f, "Invalid threshold: {}", t)
            }
            FilterError::QueueFull(n) => write!(f, "Priority queue full (max={})", n),
            FilterError::InvalidConfig => write!(f, "Invalid filter configuration"),
            FilterError::AutoregressiveFailed => {
                write!(f, "Autoregressive fallback failed")
            }
            FilterError::GEIMismatch => write!(f, "GEI vector dimension mismatch"),
        }
    }
}

impl std::error::Error for FilterError {}

// â”€â”€â”€ Configuration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct FilterConfig {
    /// GEI alignment threshold for acceptance
    pub alignment_threshold: f64,
    /// Maximum candidates in priority queue
    pub max_queue_size: usize,
    /// TTFT (Time To First Token) timeout in ms
    pub ttft_timeout_ms: u64,
    /// Enable autoregressive fallback
    pub autoregressive_fallback: bool,
    /// Maximum rejection samples before fallback
    pub max_rejection_samples: usize,
    /// Symbolic constraint weight
    pub symbolic_weight: f64,
    /// Statistical weight
    pub statistical_weight: f64,
}

impl FilterConfig {
    pub fn default_topological() -> Self {
        Self {
            alignment_threshold: 0.3,
            max_queue_size: 256,
            ttft_timeout_ms: 5000,
            autoregressive_fallback: true,
            max_rejection_samples: 100,
            symbolic_weight: 0.6,
            statistical_weight: 0.4,
        }
    }

    pub fn validate(&self) -> Result<(), FilterError> {
        if self.alignment_threshold < 0.0 || self.alignment_threshold > 1.0 {
            return Err(FilterError::InvalidThreshold(self.alignment_threshold));
        }
        if self.max_queue_size == 0 {
            return Err(FilterError::InvalidConfig);
        }
        if self.ttft_timeout_ms == 0 {
            return Err(FilterError::InvalidConfig);
        }
        if self.max_rejection_samples == 0 {
            return Err(FilterError::InvalidConfig);
        }
        if self.symbolic_weight < 0.0 || self.symbolic_weight > 1.0 {
            return Err(FilterError::InvalidConfig);
        }
        if self.statistical_weight < 0.0 || self.statistical_weight > 1.0 {
            return Err(FilterError::InvalidConfig);
        }
        if (self.symbolic_weight + self.statistical_weight - 1.0).abs() > 0.01 {
            return Err(FilterError::InvalidConfig);
        }
        Ok(())
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// â”€â”€â”€ Candidate Token â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct CandidateToken {
    pub token_id: u32,
    pub logit: f32,
    pub gei_alignment: f64,
    pub symbolic_score: f64,
    pub statistical_score: f64,
    pub combined_score: f64,
    pub timestamp_ms: u64,
}

impl CandidateToken {
    pub fn new(
        token_id: u32,
        logit: f32,
        gei_alignment: f64,
        symbolic_score: f64,
        statistical_score: f64,
        timestamp_ms: u64,
    ) -> Self {
        let combined_score = 0.6 * symbolic_score + 0.4 * statistical_score;
        Self {
            token_id,
            logit,
            gei_alignment: gei_alignment.clamp(0.0, 1.0),
            symbolic_score: symbolic_score.clamp(0.0, 1.0),
            statistical_score: statistical_score.clamp(0.0, 1.0),
            combined_score,
            timestamp_ms,
        }
    }

    pub fn passes_alignment(&self, threshold: f64) -> bool {
        self.gei_alignment >= threshold
    }

    pub fn is_valid(&self) -> bool {
        self.gei_alignment >= 0.0 && self.gei_alignment <= 1.0 && self.combined_score >= 0.0
    }
}

impl fmt::Display for CandidateToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Candidate(id={}, logit={:.3}, align={:.4}, score={:.4})",
            self.token_id, self.logit, self.gei_alignment, self.combined_score
        )
    }
}

// Priority queue ordering (highest combined score first)
impl Eq for CandidateToken {}

impl PartialEq for CandidateToken {
    fn eq(&self, other: &Self) -> bool {
        self.combined_score == other.combined_score
    }
}

impl Ord for CandidateToken {
    fn cmp(&self, other: &Self) -> Ordering {
        self.combined_score
            .partial_cmp(&other.combined_score)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for CandidateToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// â”€â”€â”€ Filter Result â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct FilterResult {
    pub accepted_token: CandidateToken,
    pub rejection_count: usize,
    pub used_fallback: bool,
    pub processing_time_ms: u64,
    pub timestamp_ms: u64,
}

impl fmt::Display for FilterResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FilterResult(token={}, rejections={}, fallback={}, time={}ms)",
            self.accepted_token.token_id,
            self.rejection_count,
            self.used_fallback,
            self.processing_time_ms
        )
    }
}

// â”€â”€â”€ Streaming Symbolic Filter â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug)]
pub struct StreamingSymbolicFilter {
    config: FilterConfig,
    queue: BinaryHeap<CandidateToken>,
    gei_cache: [f64; 8],
    gei_timestamp_ms: u64,
    total_processed: usize,
    total_rejected: usize,
    fallback_count: usize,
}

impl StreamingSymbolicFilter {
    pub fn new() -> Self {
        Self {
            config: FilterConfig::default_topological(),
            queue: BinaryHeap::new(),
            gei_cache: [0.0; 8],
            gei_timestamp_ms: 0,
            total_processed: 0,
            total_rejected: 0,
            fallback_count: 0,
        }
    }

    pub fn with_config(config: FilterConfig) -> Result<Self, FilterError> {
        config.validate()?;
        Ok(Self {
            config,
            queue: BinaryHeap::new(),
            gei_cache: [0.0; 8],
            gei_timestamp_ms: 0,
            total_processed: 0,
            total_rejected: 0,
            fallback_count: 0,
        })
    }

    /// Update GEI cache for alignment checks
    pub fn update_gei_cache(&mut self, gei: [f64; 8], timestamp_ms: u64) {
        self.gei_cache = gei;
        self.gei_timestamp_ms = timestamp_ms;
    }

    /// Enqueue a candidate token
    pub fn enqueue(&mut self, candidate: CandidateToken) -> Result<(), FilterError> {
        if self.queue.len() >= self.config.max_queue_size {
            // Remove lowest priority to make room
            self.queue.pop();
        }
        self.queue.push(candidate);
        Ok(())
    }

    /// Enqueue multiple candidates
    pub fn enqueue_batch(&mut self, candidates: &[CandidateToken]) -> Result<usize, FilterError> {
        let mut count = 0;
        for candidate in candidates {
            self.enqueue(candidate.clone())?;
            count += 1;
        }
        Ok(count)
    }

    /// Apply rejection sampling filter
    pub fn apply_filter(
        &mut self,
        start_ms: u64,
        current_ms: u64,
    ) -> Result<FilterResult, FilterError> {
        if self.queue.is_empty() {
            // Try autoregressive fallback
            if self.config.autoregressive_fallback {
                return self.autoregressive_fallback(start_ms, current_ms);
            }
            return Err(FilterError::EmptyInput);
        }

        let mut rejection_count = 0;
        let mut best_candidate = None;

        // Extract candidates and check alignment
        let mut candidates: Vec<CandidateToken> = self.queue.drain().collect();
        // Sort by combined score (highest first)
        candidates.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(Ordering::Equal)
        });

        for candidate in &candidates {
            // Check GEI alignment first
            if candidate.passes_alignment(self.config.alignment_threshold) {
                best_candidate = Some(candidate.clone());
                break; // Accept first valid candidate
            }

            rejection_count += 1;
            self.total_rejected += 1;

            // Check max rejection samples
            if rejection_count >= self.config.max_rejection_samples {
                break;
            }

            // Check timeout after each rejection
            if current_ms.saturating_sub(start_ms) > self.config.ttft_timeout_ms {
                if let Some(best) = &best_candidate {
                    // Return best found so far
                    let accepted = best.clone();
                    self.total_processed += 1;
                    return Ok(FilterResult {
                        accepted_token: accepted,
                        rejection_count,
                        used_fallback: false,
                        processing_time_ms: current_ms.saturating_sub(start_ms),
                        timestamp_ms: current_ms,
                    });
                }
                return Err(FilterError::Timeout(self.config.ttft_timeout_ms));
            }
        }

        // Restore remaining candidates to queue
        let accepted = if let Some(best) = &best_candidate {
            // Put back candidates that weren't checked
            let best_id = best.token_id;
            for c in candidates.iter().filter(|c| c.token_id != best_id) {
                self.enqueue(c.clone()).ok();
            }
            best.clone()
        } else {
            // All rejected â€” restore and fallback
            for c in &candidates {
                self.enqueue(c.clone()).ok();
            }
            if self.config.autoregressive_fallback {
                let fallback = self.autoregressive_fallback(start_ms, current_ms)?;
                return Ok(fallback);
            }
            return Err(FilterError::AllRejected);
        };

        self.total_processed += 1;

        Ok(FilterResult {
            accepted_token: accepted,
            rejection_count,
            used_fallback: false,
            processing_time_ms: current_ms.saturating_sub(start_ms),
            timestamp_ms: current_ms,
        })
    }

    /// Autoregressive fallback when all candidates rejected
    fn autoregressive_fallback(
        &mut self,
        start_ms: u64,
        current_ms: u64,
    ) -> Result<FilterResult, FilterError> {
        if !self.config.autoregressive_fallback {
            return Err(FilterError::AutoregressiveFailed);
        }

        // Simulate autoregressive generation with safe defaults
        self.fallback_count += 1;
        self.total_processed += 1;

        let fallback_token = CandidateToken::new(
            0, // Safe token ID
            0.0,
            self.config.alignment_threshold + 0.1, // Above threshold
            0.5,
            0.5,
            current_ms,
        );

        Ok(FilterResult {
            accepted_token: fallback_token,
            rejection_count: 0,
            used_fallback: true,
            processing_time_ms: current_ms.saturating_sub(start_ms),
            timestamp_ms: current_ms,
        })
    }

    /// Compute GEI alignment for a candidate
    pub fn compute_gei_alignment(&self, candidate_gei: &[f64; 8]) -> f64 {
        Self::cosine_similarity(&self.gei_cache, candidate_gei)
    }

    /// Cosine similarity between two GEI vectors
    pub fn cosine_similarity(a: &[f64; 8], b: &[f64; 8]) -> f64 {
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }
        (dot / (norm_a * norm_b)).min(1.0).max(-1.0)
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    /// Get processing stats
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.total_processed,
            self.total_rejected,
            self.fallback_count,
        )
    }

    /// Rejection rate
    pub fn rejection_rate(&self) -> Option<f64> {
        let total = self.total_processed + self.total_rejected;
        if total == 0 {
            return None;
        }
        Some(self.total_rejected as f64 / total as f64)
    }

    /// Fallback rate
    pub fn fallback_rate(&self) -> Option<f64> {
        if self.total_processed == 0 {
            return None;
        }
        Some(self.fallback_count as f64 / self.total_processed as f64)
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.queue.clear();
        self.gei_cache = [0.0; 8];
        self.gei_timestamp_ms = 0;
        self.total_processed = 0;
        self.total_rejected = 0;
        self.fallback_count = 0;
    }
}

impl Default for StreamingSymbolicFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StreamingSymbolicFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (processed, rejected, fallback) = self.stats();
        write!(
            f,
            "StreamingFilter(queue={}, processed={}, rejected={}, fallback={})",
            self.queue.len(),
            processed,
            rejected,
            fallback
        )
    }
}

// â”€â”€â”€ Public Utility Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compute combined score from symbolic and statistical components
pub fn compute_combined_score(
    symbolic: f64,
    statistical: f64,
    symbolic_weight: f64,
    statistical_weight: f64,
) -> f64 {
    (symbolic_weight * symbolic + statistical_weight * statistical)
        .min(1.0)
        .max(0.0)
}

/// Check if a candidate passes GEI alignment threshold
pub fn passes_alignment(gei_alignment: f64, threshold: f64) -> bool {
    gei_alignment >= threshold
}

/// Compute rejection sampling acceptance probability
pub fn acceptance_probability(gei_alignment: f64, threshold: f64, temperature: f64) -> f64 {
    if gei_alignment < threshold {
        return 0.0;
    }
    // Sigmoid-based acceptance
    let x = (gei_alignment - threshold) / temperature.max(0.01);
    (1.0 / (1.0 + (-x).exp())).min(1.0).max(0.0)
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(
        id: u32,
        logit: f32,
        alignment: f64,
        symbolic: f64,
        statistical: f64,
    ) -> CandidateToken {
        CandidateToken::new(id, logit, alignment, symbolic, statistical, 1000)
    }

    // â”€â”€â”€ Config Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_config_default() {
        let config = FilterConfig::default_topological();
        assert_eq!(config.alignment_threshold, 0.3);
        assert!(config.autoregressive_fallback);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = FilterConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = FilterConfig {
            alignment_threshold: 1.5,
            ..FilterConfig::default_topological()
        };
        assert_eq!(config.validate(), Err(FilterError::InvalidThreshold(1.5)));
    }

    #[test]
    fn test_config_zero_queue() {
        let config = FilterConfig {
            max_queue_size: 0,
            ..FilterConfig::default_topological()
        };
        assert_eq!(config.validate(), Err(FilterError::InvalidConfig));
    }

    #[test]
    fn test_config_weights_mismatch() {
        let config = FilterConfig {
            symbolic_weight: 0.8,
            statistical_weight: 0.8,
            ..FilterConfig::default_topological()
        };
        assert_eq!(config.validate(), Err(FilterError::InvalidConfig));
    }

    // â”€â”€â”€ Candidate Token Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_candidate_creation() {
        let c = make_candidate(42, 1.5, 0.8, 0.9, 0.7);
        assert_eq!(c.token_id, 42);
        assert!(c.is_valid());
    }

    #[test]
    fn test_candidate_passes_alignment() {
        let c = make_candidate(1, 0.0, 0.5, 0.5, 0.5);
        assert!(c.passes_alignment(0.3));
        assert!(!c.passes_alignment(0.7));
    }

    #[test]
    fn test_candidate_combined_score() {
        let c = make_candidate(1, 0.0, 0.5, 0.8, 0.6);
        // 0.6 * 0.8 + 0.4 * 0.6 = 0.48 + 0.24 = 0.72
        assert!((c.combined_score - 0.72).abs() < 1e-10);
    }

    #[test]
    fn test_candidate_display() {
        let c = make_candidate(42, 1.5, 0.8, 0.9, 0.7);
        let s = format!("{}", c);
        assert!(s.contains("Candidate"));
        assert!(s.contains("id=42"));
    }

    #[test]
    fn test_candidate_ordering() {
        let c1 = make_candidate(1, 0.0, 0.5, 0.9, 0.9); // score = 0.9
        let c2 = make_candidate(2, 0.0, 0.5, 0.5, 0.5); // score = 0.5
        assert!(c1 > c2);
    }

    // â”€â”€â”€ Filter Creation Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_filter_new() {
        let filter = StreamingSymbolicFilter::new();
        assert_eq!(filter.queue_size(), 0);
    }

    #[test]
    fn test_filter_with_config() {
        let config = FilterConfig::default_topological();
        let filter = StreamingSymbolicFilter::with_config(config).unwrap();
        assert_eq!(filter.queue_size(), 0);
    }

    #[test]
    fn test_filter_with_bad_config() {
        let config = FilterConfig {
            max_queue_size: 0,
            ..FilterConfig::default_topological()
        };
        let result = StreamingSymbolicFilter::with_config(config);
        assert!(matches!(result, Err(FilterError::InvalidConfig)));
    }

    // â”€â”€â”€ GEI Cache Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_update_gei_cache() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.update_gei_cache([1.0; 8], 1000);
        assert_eq!(filter.gei_cache, [1.0; 8]);
        assert_eq!(filter.gei_timestamp_ms, 1000);
    }

    #[test]
    fn test_compute_gei_alignment_identical() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.update_gei_cache([1.0; 8], 1000);
        let alignment = filter.compute_gei_alignment(&[1.0; 8]);
        assert!((alignment - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_gei_alignment_orthogonal() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.update_gei_cache([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 1000);
        let alignment = filter.compute_gei_alignment(&[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(alignment.abs() < 1e-10);
    }

    // â”€â”€â”€ Enqueue Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_enqueue() {
        let mut filter = StreamingSymbolicFilter::new();
        let c = make_candidate(1, 0.0, 0.8, 0.9, 0.7);
        filter.enqueue(c).unwrap();
        assert_eq!(filter.queue_size(), 1);
    }

    #[test]
    fn test_enqueue_batch() {
        let mut filter = StreamingSymbolicFilter::new();
        let candidates = vec![
            make_candidate(1, 0.0, 0.8, 0.9, 0.7),
            make_candidate(2, 0.0, 0.6, 0.7, 0.5),
            make_candidate(3, 0.0, 0.4, 0.5, 0.3),
        ];
        let count = filter.enqueue_batch(&candidates).unwrap();
        assert_eq!(count, 3);
        assert_eq!(filter.queue_size(), 3);
    }

    #[test]
    fn test_enqueue_queue_full() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.max_queue_size = 2;
        filter
            .enqueue(make_candidate(1, 0.0, 0.8, 0.9, 0.7))
            .unwrap();
        filter
            .enqueue(make_candidate(2, 0.0, 0.6, 0.7, 0.5))
            .unwrap();
        filter
            .enqueue(make_candidate(3, 0.0, 0.4, 0.5, 0.3))
            .unwrap();
        assert_eq!(filter.queue_size(), 2); // Oldest removed
    }

    // â”€â”€â”€ Filter Application Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_apply_filter_accepts() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.3;
        filter
            .enqueue(make_candidate(1, 0.0, 0.8, 0.9, 0.7))
            .unwrap();

        let result = filter.apply_filter(1000, 1001).unwrap();
        assert_eq!(result.accepted_token.token_id, 1);
        assert_eq!(result.rejection_count, 0);
        assert!(!result.used_fallback);
    }

    #[test]
    fn test_apply_filter_rejects_low_alignment() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.9;
        filter.config.autoregressive_fallback = false;
        filter
            .enqueue(make_candidate(1, 0.0, 0.3, 0.9, 0.7))
            .unwrap();

        assert_eq!(
            filter.apply_filter(1000, 1001),
            Err(FilterError::AllRejected)
        );
    }

    #[test]
    fn test_apply_filter_empty_queue() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.autoregressive_fallback = false;
        assert_eq!(
            filter.apply_filter(1000, 1001),
            Err(FilterError::EmptyInput)
        );
    }

    #[test]
    fn test_apply_filter_fallback() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.9;
        filter.config.autoregressive_fallback = true;
        filter
            .enqueue(make_candidate(1, 0.0, 0.3, 0.9, 0.7))
            .unwrap();

        let result = filter.apply_filter(1000, 1001).unwrap();
        assert!(result.used_fallback);
        assert_eq!(result.accepted_token.token_id, 0); // Fallback token
    }

    #[test]
    fn test_apply_filter_timeout() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.ttft_timeout_ms = 100;
        filter.config.autoregressive_fallback = false;
        filter
            .enqueue(make_candidate(1, 0.0, 0.8, 0.9, 0.7))
            .unwrap();

        // start_ms far in the past
        let result = filter.apply_filter(0, 200).unwrap();
        assert_eq!(result.accepted_token.token_id, 1); // Still accepts first valid
    }

    #[test]
    fn test_apply_filter_selects_highest() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.3;
        // Lower alignment but higher score
        filter
            .enqueue(make_candidate(1, 0.0, 0.5, 0.5, 0.5))
            .unwrap(); // score = 0.5
                       // Higher alignment but lower score
        filter
            .enqueue(make_candidate(2, 0.0, 0.9, 0.3, 0.3))
            .unwrap(); // score = 0.3

        let result = filter.apply_filter(1000, 1001).unwrap();
        // Should pick highest score that passes threshold
        assert_eq!(result.accepted_token.token_id, 1);
    }

    // â”€â”€â”€ Stats Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_stats_initial() {
        let filter = StreamingSymbolicFilter::new();
        let (processed, rejected, fallback) = filter.stats();
        assert_eq!(processed, 0);
        assert_eq!(rejected, 0);
        assert_eq!(fallback, 0);
    }

    #[test]
    fn test_stats_after_processing() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.3;
        filter
            .enqueue(make_candidate(1, 0.0, 0.8, 0.9, 0.7))
            .unwrap();
        filter.apply_filter(1000, 1001).unwrap();

        let (processed, rejected, fallback) = filter.stats();
        assert_eq!(processed, 1);
        assert_eq!(rejected, 0);
        assert_eq!(fallback, 0);
    }

    #[test]
    fn test_rejection_rate() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.9;
        filter.config.autoregressive_fallback = true;

        // Enqueue low-alignment candidates
        for i in 0..5 {
            filter
                .enqueue(make_candidate(i, 0.0, 0.3, 0.5, 0.5))
                .unwrap();
        }

        filter.apply_filter(1000, 1001).unwrap();
        let rate = filter.rejection_rate();
        assert!(rate.is_some());
    }

    #[test]
    fn test_fallback_rate() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.9;
        filter.config.autoregressive_fallback = true;

        for _ in 0..3 {
            filter
                .enqueue(make_candidate(1, 0.0, 0.3, 0.5, 0.5))
                .unwrap();
            filter.apply_filter(1000, 1001).unwrap();
        }

        let rate = filter.fallback_rate();
        assert!(rate.is_some());
        assert!(*rate.as_ref().unwrap() > 0.0);
    }

    #[test]
    fn test_rates_empty() {
        let filter = StreamingSymbolicFilter::new();
        assert!(filter.rejection_rate().is_none());
        assert!(filter.fallback_rate().is_none());
    }

    // â”€â”€â”€ Reset Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_reset() {
        let mut filter = StreamingSymbolicFilter::new();
        filter
            .enqueue(make_candidate(1, 0.0, 0.8, 0.9, 0.7))
            .unwrap();
        filter.apply_filter(1000, 1001).unwrap();

        filter.reset();
        assert_eq!(filter.queue_size(), 0);
        assert_eq!(filter.stats(), (0, 0, 0));
    }

    // â”€â”€â”€ Display Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_filter_display() {
        let filter = StreamingSymbolicFilter::new();
        let s = format!("{}", filter);
        assert!(s.contains("StreamingFilter"));
    }

    #[test]
    fn test_filter_result_display() {
        let result = FilterResult {
            accepted_token: make_candidate(1, 0.0, 0.8, 0.9, 0.7),
            rejection_count: 2,
            used_fallback: false,
            processing_time_ms: 5,
            timestamp_ms: 1001,
        };
        let s = format!("{}", result);
        assert!(s.contains("FilterResult"));
    }

    // â”€â”€â”€ Error Display Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_error_display_empty() {
        let e = FilterError::EmptyInput;
        assert!(format!("{}", e).contains("empty"));
    }

    #[test]
    fn test_error_display_timeout() {
        let e = FilterError::Timeout(5000);
        let s = format!("{}", e);
        assert!(s.contains("5000"));
    }

    #[test]
    fn test_error_display_all_rejected() {
        let e = FilterError::AllRejected;
        assert!(format!("{}", e).contains("rejected"));
    }

    #[test]
    fn test_error_display_queue_full() {
        let e = FilterError::QueueFull(256);
        let s = format!("{}", e);
        assert!(s.contains("256"));
    }

    // â”€â”€â”€ Utility Function Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_compute_combined_score() {
        let score = compute_combined_score(0.8, 0.6, 0.6, 0.4);
        assert!((score - 0.72).abs() < 1e-10);
    }

    #[test]
    fn test_compute_combined_score_capped() {
        let score = compute_combined_score(1.5, 1.5, 0.6, 0.4);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_passes_alignment_above() {
        assert!(passes_alignment(0.8, 0.3));
    }

    #[test]
    fn test_passes_alignment_below() {
        assert!(!passes_alignment(0.2, 0.3));
    }

    #[test]
    fn test_passes_alignment_exact() {
        assert!(passes_alignment(0.3, 0.3));
    }

    #[test]
    fn test_acceptance_probability_above() {
        let p = acceptance_probability(0.8, 0.3, 0.1);
        assert!(p > 0.5);
    }

    #[test]
    fn test_acceptance_probability_below() {
        let p = acceptance_probability(0.2, 0.3, 0.1);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn test_acceptance_probability_at_threshold() {
        let p = acceptance_probability(0.3, 0.3, 0.1);
        assert!((p - 0.5).abs() < 0.01); // Sigmoid at 0 = 0.5
    }

    // â”€â”€â”€ Cosine Similarity Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_cosine_similarity_identical() {
        let a = [1.0; 8];
        let b = [1.0; 8];
        assert!((StreamingSymbolicFilter::cosine_similarity(&a, &b) - 1.0) < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_zero_norm() {
        let a = [0.0; 8];
        let b = [1.0; 8];
        assert_eq!(StreamingSymbolicFilter::cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_negative() {
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let sim = StreamingSymbolicFilter::cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-10);
    }

    // â”€â”€â”€ Workflow Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_full_filtering_workflow() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.3;
        filter.update_gei_cache([1.0; 8], 1000);

        // Enqueue diverse candidates
        let candidates = vec![
            make_candidate(1, 2.0, 0.9, 0.95, 0.9), // High alignment, high score
            make_candidate(2, 1.5, 0.5, 0.8, 0.7),  // Medium alignment
            make_candidate(3, 1.0, 0.2, 0.7, 0.6),  // Low alignment (rejected)
            make_candidate(4, 0.5, 0.4, 0.6, 0.5),  // Medium alignment
        ];
        filter.enqueue_batch(&candidates).unwrap();

        // Apply filter
        let result = filter.apply_filter(1000, 1001).unwrap();
        assert!(!result.used_fallback);
        assert!(result.accepted_token.gei_alignment >= 0.3);

        // Check stats
        let (processed, _, _) = filter.stats();
        assert_eq!(processed, 1);
    }

    #[test]
    fn test_max_rejection_samples() {
        let mut filter = StreamingSymbolicFilter::new();
        filter.config.alignment_threshold = 0.9;
        filter.config.max_rejection_samples = 3;
        filter.config.autoregressive_fallback = true;

        // Enqueue 5 low-alignment candidates
        for i in 0..5 {
            filter
                .enqueue(make_candidate(i, 0.0, 0.3, 0.5, 0.5))
                .unwrap();
        }

        let result = filter.apply_filter(1000, 1001).unwrap();
        assert!(result.used_fallback);
        assert!(result.rejection_count <= 3);
    }
}
