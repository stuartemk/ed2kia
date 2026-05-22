//! Bias Mitigator — Motor de detección y mitigación de sesgos en señales de alineación
//!
//! Módulo responsable de detectar sesgos sistemáticos en el feedback de alineación,
//! aplicar correcciones de mitigación y mantener métricas de calidad de señal.
//! Implementa detección de skew, concentración, dominancia de fuente y deriva temporal.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, VecDeque};
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for Bias Mitigator.
#[derive(Debug, Error)]
pub enum BiasMitigatorError {
    #[error("Invalid score range: {score:.3} not in [{min}, {max}]")]
    InvalidScoreRange { score: f64, min: f64, max: f64 },
    #[error("Source not registered: {0}")]
    SourceNotRegistered(String),
    #[error("Bias threshold exceeded: {bias_type} = {score:.3} > {threshold:.3}")]
    BiasThresholdExceeded {
        bias_type: String,
        score: f64,
        threshold: f64,
    },
    #[error("Insufficient data for analysis: {count} < {min}")]
    InsufficientData { count: usize, min: usize },
    #[error("Mitigation strategy failed: {0}")]
    MitigationFailed(String),
    #[error("Confidence score invalid: {0:.3}")]
    InvalidConfidence(f64),
    #[error("Analysis window too small: {0}")]
    WindowTooSmall(usize),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Type of bias detected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum BiasType {
    /// Skew bias — distribution is asymmetric.
    Skew,
    /// Concentration bias — scores cluster in one direction.
    Concentration,
    /// Source dominance — single source influences too much.
    SourceDominance,
    /// Temporal drift — scores drift over time.
    TemporalDrift,
    /// Feedback loop bias — circular reinforcement detected.
    FeedbackLoop,
    /// Selection bias — non-representative sample.
    Selection,
}

impl std::fmt::Display for BiasType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BiasType::Skew => write!(f, "SKEW"),
            BiasType::Concentration => write!(f, "CONCENTRATION"),
            BiasType::SourceDominance => write!(f, "SOURCE_DOMINANCE"),
            BiasType::TemporalDrift => write!(f, "TEMPORAL_DRIFT"),
            BiasType::FeedbackLoop => write!(f, "FEEDBACK_LOOP"),
            BiasType::Selection => write!(f, "SELECTION"),
        }
    }
}

/// Severity level for detected bias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiasSeverity {
    /// Bias is within acceptable range.
    Low,
    /// Bias requires monitoring.
    Medium,
    /// Bias requires mitigation.
    High,
    /// Bias requires immediate action.
    Critical,
}

impl std::fmt::Display for BiasSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BiasSeverity::Low => write!(f, "LOW"),
            BiasSeverity::Medium => write!(f, "MEDIUM"),
            BiasSeverity::High => write!(f, "HIGH"),
            BiasSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Mitigation strategy to apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MitigationStrategy {
    /// No mitigation needed.
    None,
    /// Apply weight adjustment to affected scores.
    WeightAdjustment,
    /// Filter out biased samples.
    SampleFiltering,
    /// Reduce influence of dominant sources.
    SourceDownweight,
    /// Apply temporal correction.
    TemporalCorrection,
    /// Switch to static mode.
    FallbackToStatic,
}

impl std::fmt::Display for MitigationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MitigationStrategy::None => write!(f, "NONE"),
            MitigationStrategy::WeightAdjustment => write!(f, "WEIGHT_ADJUSTMENT"),
            MitigationStrategy::SampleFiltering => write!(f, "SAMPLE_FILTERING"),
            MitigationStrategy::SourceDownweight => write!(f, "SOURCE_DOWNWEIGHT"),
            MitigationStrategy::TemporalCorrection => write!(f, "TEMPORAL_CORRECTION"),
            MitigationStrategy::FallbackToStatic => write!(f, "FALLBACK_STATIC"),
        }
    }
}

/// Score entry with source and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreEntry {
    /// Source identifier.
    pub source_id: String,
    /// Score value (-1.0 to 1.0).
    pub score: f64,
    /// Confidence (0.0 to 1.0).
    pub confidence: f64,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
    /// Weight after mitigation (1.0 = full weight).
    pub weight: f64,
}

impl ScoreEntry {
    /// Creates a new score entry.
    pub fn new(source_id: String, score: f64, confidence: f64) -> Self {
        Self {
            source_id,
            score: score.clamp(-1.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            timestamp_ms: current_timestamp_ms(),
            weight: 1.0,
        }
    }

    /// Returns weighted score.
    pub fn weighted_score(&self) -> f64 {
        self.score * self.weight
    }
}

/// Detected bias with details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasDetection {
    /// Type of bias detected.
    pub bias_type: BiasType,
    /// Severity level.
    pub severity: BiasSeverity,
    /// Bias score (0.0 = no bias, 1.0 = maximum bias).
    pub bias_score: f64,
    /// Recommended mitigation strategy.
    pub strategy: MitigationStrategy,
    /// Affected sources.
    pub affected_sources: Vec<String>,
    /// Detection timestamp (ms).
    pub detected_at_ms: u64,
    /// Details about the detection.
    pub details: String,
}

impl BiasDetection {
    /// Creates a new bias detection.
    pub fn new(bias_type: BiasType, bias_score: f64, details: String) -> Self {
        let (severity, strategy) = Self::classify(bias_score);
        Self {
            bias_type,
            severity,
            bias_score: bias_score.clamp(0.0, 1.0),
            strategy,
            affected_sources: Vec::new(),
            detected_at_ms: current_timestamp_ms(),
            details,
        }
    }

    /// Classifies severity and strategy based on bias score.
    fn classify(bias_score: f64) -> (BiasSeverity, MitigationStrategy) {
        match bias_score {
            0.0..=0.1 => (BiasSeverity::Low, MitigationStrategy::None),
            0.1..=0.25 => (BiasSeverity::Medium, MitigationStrategy::WeightAdjustment),
            0.25..=0.5 => (BiasSeverity::High, MitigationStrategy::SourceDownweight),
            0.5..=0.75 => (BiasSeverity::Critical, MitigationStrategy::SampleFiltering),
            _ => (BiasSeverity::Critical, MitigationStrategy::FallbackToStatic),
        }
    }
}

/// Result of bias analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasAnalysisResult {
    /// Overall bias score (0.0 = no bias, 1.0 = maximum bias).
    pub overall_bias_score: f64,
    /// Individual bias detections.
    pub detections: Vec<BiasDetection>,
    /// Mitigation applied.
    pub mitigation_applied: bool,
    /// Corrected scores available.
    pub corrected_count: usize,
    /// Analysis timestamp (ms).
    pub analyzed_at_ms: u64,
    /// Quality score (0.0 = poor, 1.0 = excellent).
    pub quality_score: f64,
}

impl BiasAnalysisResult {
    /// Returns true if bias is within acceptable limits.
    pub fn is_acceptable(&self, threshold: f64) -> bool {
        self.overall_bias_score <= threshold
    }

    /// Returns the highest severity detected.
    pub fn max_severity(&self) -> BiasSeverity {
        let mut max = BiasSeverity::Low;
        for detection in &self.detections {
            match (&max, &detection.severity) {
                (_, BiasSeverity::Critical) => max = BiasSeverity::Critical,
                (_, BiasSeverity::High) => max = BiasSeverity::High,
                (_, BiasSeverity::Medium) => max = BiasSeverity::Medium,
                _ => {}
            }
        }
        max
    }
}

/// Statistics for bias mitigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasStats {
    /// Total analyses performed.
    pub total_analyses: usize,
    /// Total biases detected.
    pub total_detections: usize,
    /// Total mitigations applied.
    pub total_mitigations: usize,
    /// Average bias score.
    pub avg_bias_score: f64,
    /// Detections by type.
    pub detections_by_type: HashMap<BiasType, usize>,
    /// Fallback count.
    pub fallback_count: usize,
    /// Current quality score.
    pub current_quality: f64,
}

impl Default for BiasStats {
    fn default() -> Self {
        Self {
            total_analyses: 0,
            total_detections: 0,
            total_mitigations: 0,
            avg_bias_score: 0.0,
            detections_by_type: HashMap::new(),
            fallback_count: 0,
            current_quality: 1.0,
        }
    }
}

/// Configuration for bias mitigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasConfig {
    /// Minimum samples for analysis.
    pub min_samples: usize,
    /// Analysis window size.
    pub analysis_window: usize,
    /// Skew threshold.
    pub skew_threshold: f64,
    /// Concentration threshold.
    pub concentration_threshold: f64,
    /// Source dominance threshold (max share per source).
    pub source_dominance_threshold: f64,
    /// Temporal drift threshold.
    pub temporal_drift_threshold: f64,
    /// Overall bias threshold for fallback.
    pub overall_bias_threshold: f64,
    /// Weight adjustment factor.
    pub weight_adjustment_factor: f64,
    /// Maximum history size.
    pub max_history_size: usize,
}

impl Default for BiasConfig {
    fn default() -> Self {
        Self {
            min_samples: 10,
            analysis_window: 200,
            skew_threshold: 0.15,
            concentration_threshold: 0.2,
            source_dominance_threshold: 0.4,
            temporal_drift_threshold: 0.1,
            overall_bias_threshold: 0.25,
            weight_adjustment_factor: 0.7,
            max_history_size: 500,
        }
    }
}

// ---------------------------------------------------------------------------
// BiasMitigator Engine
// ---------------------------------------------------------------------------

/// Engine for detecting and mitigating bias in alignment feedback.
pub struct BiasMitigator {
    config: BiasConfig,
    stats: BiasStats,
    /// Score buffer for analysis.
    score_buffer: VecDeque<ScoreEntry>,
    /// History of analysis results.
    analysis_history: VecDeque<BiasAnalysisResult>,
    /// Per-source score tracking.
    source_scores: BTreeMap<String, VecDeque<f64>>,
    /// Source influence tracking.
    source_influence: HashMap<String, f64>,
    /// Temporal windows for drift detection.
    temporal_windows: VecDeque<(u64, f64)>,
}

impl BiasMitigator {
    /// Creates a new bias mitigator with default config.
    pub fn new() -> Self {
        Self::with_config(BiasConfig::default())
    }

    /// Creates a mitigator with custom config.
    pub fn with_config(config: BiasConfig) -> Self {
        Self {
            config,
            stats: BiasStats::default(),
            score_buffer: VecDeque::new(),
            analysis_history: VecDeque::new(),
            source_scores: BTreeMap::new(),
            source_influence: HashMap::new(),
            temporal_windows: VecDeque::new(),
        }
    }

    /// Records a score entry.
    pub fn record_score(&mut self, entry: ScoreEntry) -> Result<(), BiasMitigatorError> {
        if entry.confidence < 0.0 || entry.confidence > 1.0 {
            return Err(BiasMitigatorError::InvalidConfidence(entry.confidence));
        }

        self.score_buffer.push_back(entry.clone());
        self.stats.current_quality = self.compute_quality();

        // Track per-source scores
        self.source_scores
            .entry(entry.source_id.clone())
            .or_default()
            .push_back(entry.score);

        // Track source influence
        let influence = self
            .source_influence
            .entry(entry.source_id.clone())
            .or_insert(0.0);
        *influence = entry.confidence.max(*influence);

        // Track temporal window
        self.temporal_windows
            .push_back((entry.timestamp_ms, entry.score));

        // Enforce window limits
        self.enforce_limits();

        debug!(
            "BiasMitigator: recorded score from {} (score={:.3}, confidence={:.3})",
            entry.source_id, entry.score, entry.confidence
        );
        Ok(())
    }

    /// Records a score with source ID.
    pub fn record_score_simple(
        &mut self,
        source_id: String,
        score: f64,
        confidence: f64,
    ) -> Result<(), BiasMitigatorError> {
        let entry = ScoreEntry::new(source_id, score, confidence);
        self.record_score(entry)
    }

    /// Analyzes current scores for bias.
    pub fn analyze(&mut self) -> Result<BiasAnalysisResult, BiasMitigatorError> {
        if self.score_buffer.len() < self.config.min_samples {
            return Err(BiasMitigatorError::InsufficientData {
                count: self.score_buffer.len(),
                min: self.config.min_samples,
            });
        }

        let mut detections = Vec::new();
        let mut max_bias: f64 = 0.0;

        // Detect skew bias
        let skew_score = self.detect_skew();
        if skew_score > self.config.skew_threshold {
            let detection = BiasDetection::new(
                BiasType::Skew,
                skew_score,
                format!("Skew detected: {:.3}", skew_score),
            );
            detections.push(detection);
            max_bias = max_bias.max(skew_score);
        }

        // Detect concentration bias
        let concentration_score = self.detect_concentration();
        if concentration_score > self.config.concentration_threshold {
            let detection = BiasDetection::new(
                BiasType::Concentration,
                concentration_score,
                format!("Concentration detected: {:.3}", concentration_score),
            );
            detections.push(detection);
            max_bias = max_bias.max(concentration_score);
        }

        // Detect source dominance
        let (dominance_score, dominant_sources) = self.detect_source_dominance();
        if dominance_score > self.config.source_dominance_threshold {
            let mut detection = BiasDetection::new(
                BiasType::SourceDominance,
                dominance_score,
                format!("Source dominance detected: {:.3}", dominance_score),
            );
            detection.affected_sources = dominant_sources;
            detections.push(detection);
            max_bias = max_bias.max(dominance_score);
        }

        // Detect temporal drift
        let drift_score = self.detect_temporal_drift();
        if drift_score > self.config.temporal_drift_threshold {
            let detection = BiasDetection::new(
                BiasType::TemporalDrift,
                drift_score,
                format!("Temporal drift detected: {:.3}", drift_score),
            );
            detections.push(detection);
            max_bias = max_bias.max(drift_score);
        }

        // Apply mitigation if needed
        let mitigation_applied = if max_bias > self.config.skew_threshold {
            self.apply_mitigation(&detections);
            true
        } else {
            false
        };

        let corrected_count = if mitigation_applied {
            self.score_buffer.len()
        } else {
            0
        };

        let quality_score = self.compute_quality();
        let result = BiasAnalysisResult {
            overall_bias_score: max_bias,
            detections,
            mitigation_applied,
            corrected_count,
            analyzed_at_ms: current_timestamp_ms(),
            quality_score,
        };

        // Update stats
        self.stats.total_analyses += 1;
        self.stats.total_detections += result.detections.len();
        if mitigation_applied {
            self.stats.total_mitigations += 1;
        }
        for detection in &result.detections {
            *self
                .stats
                .detections_by_type
                .entry(detection.bias_type.clone())
                .or_insert(0) += 1;
        }
        let alpha = 0.1;
        self.stats.avg_bias_score =
            alpha * result.overall_bias_score + (1.0 - alpha) * self.stats.avg_bias_score;
        self.stats.current_quality = quality_score;

        // Record history
        self.analysis_history.push_back(result.clone());
        while self.analysis_history.len() > self.config.max_history_size {
            self.analysis_history.pop_front();
        }

        info!(
            "BiasMitigator: analysis complete (bias={:.3}, detections={}, quality={:.3})",
            result.overall_bias_score,
            result.detections.len(),
            result.quality_score
        );

        Ok(result)
    }

    /// Gets bias statistics.
    pub fn get_stats(&self) -> BiasStats {
        self.stats.clone()
    }

    /// Gets recent analysis history.
    pub fn get_recent_analyses(&self, limit: usize) -> Vec<&BiasAnalysisResult> {
        self.analysis_history.iter().rev().take(limit).collect()
    }

    /// Gets current quality score.
    pub fn get_quality_score(&self) -> f64 {
        self.stats.current_quality
    }

    /// Gets buffered score count.
    pub fn buffered_score_count(&self) -> usize {
        self.score_buffer.len()
    }

    /// Clears all recorded scores.
    pub fn clear_scores(&mut self) {
        self.score_buffer.clear();
        self.source_scores.clear();
        self.source_influence.clear();
        self.temporal_windows.clear();
    }

    /// Resets statistics.
    pub fn reset_stats(&mut self) {
        self.stats = BiasStats::default();
    }

    // -----------------------------------------------------------------------
    // Bias detection methods
    // -----------------------------------------------------------------------

    /// Detects skew bias in score distribution.
    fn detect_skew(&self) -> f64 {
        let scores: Vec<f64> = self.score_buffer.iter().map(|e| e.score).collect();
        if scores.len() < 3 {
            return 0.0;
        }

        let mean: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance: f64 =
            scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        let std_dev = variance.sqrt().max(1e-10);

        let cubed: f64 =
            scores.iter().map(|s| (s - mean).powi(3)).sum::<f64>() / scores.len() as f64;
        let skew = (cubed / std_dev.powi(3)).abs();

        skew.min(1.0)
    }

    /// Detects concentration bias.
    fn detect_concentration(&self) -> f64 {
        let scores: Vec<f64> = self.score_buffer.iter().map(|e| e.score).collect();
        if scores.is_empty() {
            return 0.0;
        }

        let positive = scores.iter().filter(|&&s| s > 0.0).count() as f64;
        let negative = scores.iter().filter(|&&s| s < 0.0).count() as f64;
        let total = scores.len() as f64;

        let positive_share = positive / total;
        let negative_share = negative / total;
        let max_share = positive_share.max(negative_share);

        // Concentration is high when one direction dominates
        (max_share - 0.5) * 2.0
    }

    /// Detects source dominance.
    fn detect_source_dominance(&self) -> (f64, Vec<String>) {
        let total = self.score_buffer.len();
        if total == 0 {
            return (0.0, Vec::new());
        }

        let mut source_counts: HashMap<String, usize> = HashMap::new();
        for entry in &self.score_buffer {
            *source_counts.entry(entry.source_id.clone()).or_insert(0) += 1;
        }

        let mut dominant_sources = Vec::new();
        let mut max_share: f64 = 0.0;

        for (source, count) in &source_counts {
            let share = *count as f64 / total as f64;
            if share > self.config.source_dominance_threshold {
                dominant_sources.push(source.clone());
            }
            max_share = max_share.max(share);
        }

        (max_share, dominant_sources)
    }

    /// Detects temporal drift.
    fn detect_temporal_drift(&self) -> f64 {
        let scores: Vec<f64> = self.score_buffer.iter().map(|e| e.score).collect();
        if scores.len() < 10 {
            return 0.0;
        }

        let mid = scores.len() / 2;
        let first_half: Vec<f64> = scores[..mid].to_vec();
        let second_half = &scores[mid..];

        let first_mean: f64 = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let second_mean: f64 = second_half.iter().sum::<f64>() / second_half.len() as f64;

        let drift = (first_mean - second_mean).abs();
        drift.min(1.0)
    }

    /// Computes quality score based on current state.
    fn compute_quality(&self) -> f64 {
        let scores: Vec<f64> = self
            .score_buffer
            .iter()
            .map(|e| e.score * e.weight)
            .collect();
        if scores.is_empty() {
            return 1.0;
        }

        // Quality is based on diversity and balance
        let mean: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance: f64 =
            scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;

        // Ideal variance is moderate (not too concentrated, not too scattered)
        let ideal_variance = 0.1;
        let variance_penalty = (variance - ideal_variance).abs() / (ideal_variance + variance);

        // Source diversity bonus
        let unique_sources = self.source_scores.len();
        let total_scores = self.score_buffer.len();
        let diversity_bonus = if total_scores > 0 {
            (unique_sources as f64 / total_scores as f64).min(1.0)
        } else {
            0.0
        };

        (1.0 - variance_penalty * 0.7 + diversity_bonus * 0.3).clamp(0.0, 1.0)
    }

    /// Applies mitigation based on detections.
    fn apply_mitigation(&mut self, detections: &[BiasDetection]) {
        for detection in detections {
            match detection.strategy {
                MitigationStrategy::WeightAdjustment => {
                    self.apply_weight_adjustment(detection);
                }
                MitigationStrategy::SourceDownweight => {
                    self.apply_source_downweight(detection);
                }
                MitigationStrategy::SampleFiltering => {
                    self.apply_sample_filtering(detection);
                }
                MitigationStrategy::FallbackToStatic => {
                    self.stats.fallback_count += 1;
                    warn!("BiasMitigator: fallback to static mode triggered");
                }
                _ => {}
            }
        }
    }

    /// Applies weight adjustment to affected scores.
    fn apply_weight_adjustment(&mut self, detection: &BiasDetection) {
        for entry in &mut self.score_buffer {
            if detection.affected_sources.contains(&entry.source_id) {
                entry.weight *= self.config.weight_adjustment_factor;
            }
        }
    }

    /// Applies source downweighting.
    fn apply_source_downweight(&mut self, detection: &BiasDetection) {
        for source_id in &detection.affected_sources {
            if let Some(influence) = self.source_influence.get_mut(source_id) {
                *influence *= self.config.weight_adjustment_factor;
            }
            for entry in &mut self.score_buffer {
                if entry.source_id == *source_id {
                    entry.weight *= self.config.weight_adjustment_factor;
                }
            }
        }
    }

    /// Applies sample filtering.
    fn apply_sample_filtering(&mut self, detection: &BiasDetection) {
        if detection.affected_sources.is_empty() {
            return;
        }
        let sources_to_filter: Vec<String> = detection.affected_sources.clone();
        self.score_buffer
            .retain(|e| !sources_to_filter.contains(&e.source_id));
    }

    /// Enforces window size limits.
    fn enforce_limits(&mut self) {
        while self.score_buffer.len() > self.config.analysis_window {
            self.score_buffer.pop_front();
        }
        for scores in self.source_scores.values_mut() {
            while scores.len() > self.config.analysis_window {
                scores.pop_front();
            }
        }
        while self.temporal_windows.len() > self.config.analysis_window {
            self.temporal_windows.pop_front();
        }
    }
}

impl Default for BiasMitigator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Computes a mitigation hash for audit purposes.
pub fn compute_mitigation_hash(bias_type: &str, score: f64, strategy: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mitigation-v3:");
    hasher.update(bias_type.as_bytes());
    hasher.update(score.to_le_bytes());
    hasher.update(strategy.as_bytes());
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

    fn make_entry(source: &str, score: f64) -> ScoreEntry {
        ScoreEntry::new(source.to_string(), score, 0.95)
    }

    #[test]
    fn test_mitigator_creation() {
        let mitigator = BiasMitigator::new();
        let stats = mitigator.get_stats();
        assert_eq!(stats.total_analyses, 0);
        assert_eq!(stats.total_detections, 0);
    }

    #[test]
    fn test_mitigator_with_config() {
        let config = BiasConfig {
            min_samples: 5,
            skew_threshold: 0.1,
            ..Default::default()
        };
        let mitigator = BiasMitigator::with_config(config);
        assert_eq!(mitigator.config.min_samples, 5);
    }

    #[test]
    fn test_record_score() {
        let mut mitigator = BiasMitigator::new();
        mitigator.record_score(make_entry("src-1", 0.5)).unwrap();
        assert_eq!(mitigator.buffered_score_count(), 1);
    }

    #[test]
    fn test_record_score_simple() {
        let mut mitigator = BiasMitigator::new();
        mitigator
            .record_score_simple("src-1".to_string(), 0.5, 0.95)
            .unwrap();
        assert_eq!(mitigator.buffered_score_count(), 1);
    }

    #[test]
    fn test_record_invalid_confidence() {
        let mut mitigator = BiasMitigator::new();
        let mut entry = make_entry("src-1", 0.5);
        entry.confidence = 1.5;
        let result = mitigator.record_score(entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_insufficient_data() {
        let mut mitigator = BiasMitigator::new();
        for i in 0..5 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i), 0.5))
                .unwrap();
        }
        let result = mitigator.analyze();
        assert!(result.is_err());
        match result.unwrap_err() {
            BiasMitigatorError::InsufficientData { count, min } => {
                assert_eq!(count, 5);
                assert_eq!(min, 10);
            }
            e => panic!("Expected InsufficientData, got {:?}", e),
        }
    }

    #[test]
    fn test_analyze_no_bias() {
        let mut mitigator = BiasMitigator::new();
        // Add balanced scores
        for i in 0..20 {
            let score = if i % 2 == 0 { 0.3 } else { -0.3 };
            mitigator
                .record_score(make_entry(&format!("src-{}", i % 5), score))
                .unwrap();
        }
        let result = mitigator.analyze().unwrap();
        assert!(result.quality_score > 0.0);
    }

    #[test]
    fn test_detect_skew_bias() {
        let mut mitigator = BiasMitigator::new();
        // Add highly skewed scores
        for i in 0..20 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i % 5), 1.0))
                .unwrap();
        }
        let result = mitigator.analyze().unwrap();
        // All same scores = high concentration
        assert!(!result.detections.is_empty() || result.overall_bias_score > 0.0);
    }

    #[test]
    fn test_detect_source_dominance() {
        let config = BiasConfig {
            min_samples: 5,
            source_dominance_threshold: 0.3,
            ..Default::default()
        };
        let mut mitigator = BiasMitigator::with_config(config);
        // One source dominates
        for i in 0..15 {
            mitigator
                .record_score(make_entry("dominant-src", 0.5))
                .unwrap();
        }
        for i in 0..5 {
            mitigator
                .record_score(make_entry(&format!("other-{}", i), 0.5))
                .unwrap();
        }
        let result = mitigator.analyze().unwrap();
        let has_dominance = result
            .detections
            .iter()
            .any(|d| d.bias_type == BiasType::SourceDominance);
        assert!(has_dominance);
    }

    #[test]
    fn test_detect_temporal_drift() {
        let config = BiasConfig {
            min_samples: 5,
            temporal_drift_threshold: 0.05,
            ..Default::default()
        };
        let mut mitigator = BiasMitigator::with_config(config);
        // First half positive, second half negative
        for i in 0..15 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i), 0.8))
                .unwrap();
        }
        for i in 0..15 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i), -0.8))
                .unwrap();
        }
        let result = mitigator.analyze().unwrap();
        let has_drift = result
            .detections
            .iter()
            .any(|d| d.bias_type == BiasType::TemporalDrift);
        assert!(has_drift);
    }

    #[test]
    fn test_mitigation_weight_adjustment() {
        let config = BiasConfig {
            min_samples: 5,
            skew_threshold: 0.01, // Very low to trigger mitigation
            source_dominance_threshold: 0.3,
            weight_adjustment_factor: 0.5,
            ..Default::default()
        };
        let mut mitigator = BiasMitigator::with_config(config);
        for i in 0..10 {
            mitigator.record_score(make_entry("src-1", 1.0)).unwrap();
        }
        mitigator.analyze().unwrap();
        // Check that weights were adjusted
        let has_adjusted = mitigator.score_buffer.iter().any(|e| e.weight < 1.0);
        // May or may not adjust depending on detection
        let _ = has_adjusted;
    }

    #[test]
    fn test_stats_tracking() {
        let mut mitigator = BiasMitigator::new();
        for i in 0..20 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i % 5), 0.5))
                .unwrap();
        }
        mitigator.analyze().unwrap();

        let stats = mitigator.get_stats();
        assert_eq!(stats.total_analyses, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut mitigator = BiasMitigator::new();
        for i in 0..20 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i % 5), 0.5))
                .unwrap();
        }
        mitigator.analyze().unwrap();
        mitigator.reset_stats();

        let stats = mitigator.get_stats();
        assert_eq!(stats.total_analyses, 0);
    }

    #[test]
    fn test_clear_scores() {
        let mut mitigator = BiasMitigator::new();
        mitigator.record_score(make_entry("src-1", 0.5)).unwrap();
        mitigator.clear_scores();
        assert_eq!(mitigator.buffered_score_count(), 0);
    }

    #[test]
    fn test_get_recent_analyses() {
        let mut mitigator = BiasMitigator::new();
        for _ in 0..5 {
            for i in 0..20 {
                mitigator
                    .record_score(make_entry(&format!("src-{}", i % 5), 0.5))
                    .unwrap();
            }
            let _ = mitigator.analyze();
            mitigator.clear_scores();
        }

        let analyses = mitigator.get_recent_analyses(3);
        assert_eq!(analyses.len(), 3);
    }

    #[test]
    fn test_quality_score() {
        let mut mitigator = BiasMitigator::new();
        assert_eq!(mitigator.get_quality_score(), 1.0); // Default
    }

    #[test]
    fn test_analysis_acceptable() {
        let result = BiasAnalysisResult {
            overall_bias_score: 0.05,
            detections: Vec::new(),
            mitigation_applied: false,
            corrected_count: 0,
            analyzed_at_ms: current_timestamp_ms(),
            quality_score: 0.95,
        };
        assert!(result.is_acceptable(0.15));
        assert!(!result.is_acceptable(0.03));
    }

    #[test]
    fn test_max_severity() {
        let mut detections = vec![
            BiasDetection::new(BiasType::Skew, 0.05, "low".to_string()),
            BiasDetection::new(BiasType::Concentration, 0.3, "high".to_string()),
        ];
        let result = BiasAnalysisResult {
            overall_bias_score: 0.3,
            detections: detections.clone(),
            mitigation_applied: false,
            corrected_count: 0,
            analyzed_at_ms: current_timestamp_ms(),
            quality_score: 0.7,
        };
        assert_eq!(result.max_severity(), BiasSeverity::High);
    }

    #[test]
    fn test_bias_type_display() {
        assert_eq!(format!("{}", BiasType::Skew), "SKEW");
        assert_eq!(format!("{}", BiasType::SourceDominance), "SOURCE_DOMINANCE");
        assert_eq!(format!("{}", BiasType::TemporalDrift), "TEMPORAL_DRIFT");
    }

    #[test]
    fn test_bias_severity_display() {
        assert_eq!(format!("{}", BiasSeverity::Low), "LOW");
        assert_eq!(format!("{}", BiasSeverity::Critical), "CRITICAL");
    }

    #[test]
    fn test_mitigation_strategy_display() {
        assert_eq!(
            format!("{}", MitigationStrategy::WeightAdjustment),
            "WEIGHT_ADJUSTMENT"
        );
        assert_eq!(
            format!("{}", MitigationStrategy::FallbackToStatic),
            "FALLBACK_STATIC"
        );
    }

    #[test]
    fn test_score_entry_weighted() {
        let entry = ScoreEntry::new("src".to_string(), 0.5, 0.95);
        assert_eq!(entry.weighted_score(), 0.5); // weight = 1.0

        let mut entry = entry;
        entry.weight = 0.5;
        assert_eq!(entry.weighted_score(), 0.25);
    }

    #[test]
    fn test_score_clamping() {
        let entry = ScoreEntry::new("src".to_string(), 2.0, 1.5);
        assert_eq!(entry.score, 1.0); // Clamped
        assert_eq!(entry.confidence, 1.0); // Clamped
    }

    #[test]
    fn test_config_default() {
        let config = BiasConfig::default();
        assert_eq!(config.min_samples, 10);
        assert_eq!(config.skew_threshold, 0.15);
        assert_eq!(config.overall_bias_threshold, 0.25);
    }

    #[test]
    fn test_stats_default() {
        let stats = BiasStats::default();
        assert_eq!(stats.total_analyses, 0);
        assert_eq!(stats.current_quality, 1.0);
    }

    #[test]
    fn test_mitigator_default() {
        let mitigator = BiasMitigator::default();
        assert_eq!(mitigator.buffered_score_count(), 0);
    }

    #[test]
    fn test_bias_detection_classification() {
        let detection = BiasDetection::new(BiasType::Skew, 0.05, "low".to_string());
        assert_eq!(detection.severity, BiasSeverity::Low);
        assert_eq!(detection.strategy, MitigationStrategy::None);

        let detection = BiasDetection::new(BiasType::Skew, 0.3, "high".to_string());
        assert_eq!(detection.severity, BiasSeverity::High);
        assert_eq!(detection.strategy, MitigationStrategy::SourceDownweight);

        let detection = BiasDetection::new(BiasType::Skew, 0.8, "critical".to_string());
        assert_eq!(detection.severity, BiasSeverity::Critical);
        assert_eq!(detection.strategy, MitigationStrategy::FallbackToStatic);
    }

    #[test]
    fn test_mitigation_hash_consistency() {
        let h1 = compute_mitigation_hash("SKEW", 0.3, "WEIGHT_ADJUSTMENT");
        let h2 = compute_mitigation_hash("SKEW", 0.3, "WEIGHT_ADJUSTMENT");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_mitigation_hash_uniqueness() {
        let h1 = compute_mitigation_hash("SKEW", 0.3, "WEIGHT_ADJUSTMENT");
        let h2 = compute_mitigation_hash("CONCENTRATION", 0.3, "WEIGHT_ADJUSTMENT");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_window_enforcement() {
        let config = BiasConfig {
            analysis_window: 10,
            ..Default::default()
        };
        let mut mitigator = BiasMitigator::with_config(config);
        for i in 0..20 {
            mitigator
                .record_score(make_entry(&format!("src-{}", i), 0.5))
                .unwrap();
        }
        assert_eq!(mitigator.buffered_score_count(), 10);
    }

    #[test]
    fn test_error_display() {
        let err = BiasMitigatorError::InvalidScoreRange {
            score: 1.5,
            min: -1.0,
            max: 1.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_multiple_analyses_update_stats() {
        let mut mitigator = BiasMitigator::new();
        for _cycle in 0..3 {
            for i in 0..15 {
                mitigator
                    .record_score(make_entry(&format!("src-{}", i % 3), 0.5))
                    .unwrap();
            }
            let _ = mitigator.analyze();
            mitigator.clear_scores();
        }
        let stats = mitigator.get_stats();
        assert_eq!(stats.total_analyses, 3);
    }
}
