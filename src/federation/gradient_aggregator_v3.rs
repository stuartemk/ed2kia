//! Gradient Aggregation v3 — Agregación de gradientes FedAvg con tasas adaptativas
//!
//! Implementa agregación federada de gradientes con compresión, detección de
//! outliers, ponderación por reputación y corrección de divergencia.

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum AggregatorError {
    InsufficientParticipants(usize),
    DimensionMismatch { expected: usize, got: usize },
    NoGradientsSubmitted,
    CompressionFailed(String),
    InvalidConfig(String),
}

impl fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregatorError::InsufficientParticipants(min) => {
                write!(f, "Insufficient participants (min: {})", min)
            }
            AggregatorError::DimensionMismatch { expected, got } => {
                write!(
                    f,
                    "Dimension mismatch: expected {}, got {}",
                    expected, got
                )
            }
            AggregatorError::NoGradientsSubmitted => write!(f, "No gradients submitted"),
            AggregatorError::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
            AggregatorError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

impl std::error::Error for AggregatorError {}

// ============================================================================
// Aggregator Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    pub compression_ratio: f32,
    pub outlier_threshold: f32,
    pub min_participants: usize,
    pub use_reputation_weights: bool,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            compression_ratio: 0.8,
            outlier_threshold: 2.0,
            min_participants: 2,
            use_reputation_weights: true,
        }
    }
}

// ============================================================================
// Aggregation Result
// ============================================================================

#[derive(Debug, Clone)]
pub struct AggregationResult {
    pub round: u64,
    pub aggregated_gradient: Vec<f32>,
    pub participant_count: usize,
    pub excluded_outliers: Vec<String>,
    pub gradient_norm: f32,
}

impl AggregationResult {
    pub fn new(
        round: u64,
        aggregated_gradient: Vec<f32>,
        participant_count: usize,
        excluded_outliers: Vec<String>,
    ) -> Self {
        let norm = Self::compute_norm(&aggregated_gradient);
        Self {
            round,
            aggregated_gradient,
            participant_count,
            excluded_outliers,
            gradient_norm: norm,
        }
    }

    fn compute_norm(gradient: &[f32]) -> f32 {
        gradient.iter().map(|v| v * v).sum::<f32>().sqrt()
    }
}

// ============================================================================
// Gradient Submission
// ============================================================================

#[derive(Debug, Clone)]
struct GradientSubmission {
    node_id: String,
    gradient: Vec<f32>,
    reputation: f32,
}

// ============================================================================
// Gradient Aggregator v3
// ============================================================================

pub struct GradientAggregatorV3 {
    config: AggregatorConfig,
    round: u64,
    submissions: Vec<GradientSubmission>,
    participant_weights: HashMap<String, f32>,
    aggregated_gradients: Option<Vec<f32>>,
}

impl GradientAggregatorV3 {
    pub fn new(config: AggregatorConfig) -> Self {
        Self {
            config,
            round: 0,
            submissions: Vec::new(),
            participant_weights: HashMap::new(),
            aggregated_gradients: None,
        }
    }

    /// Submit a gradient from a node
    pub fn submit_gradient(
        &mut self,
        node_id: String,
        gradient: Vec<f32>,
    ) -> Result<(), AggregatorError> {
        // Check dimension consistency
        if let Some(first) = self.submissions.first() {
            if gradient.len() != first.gradient.len() {
                return Err(AggregatorError::DimensionMismatch {
                    expected: first.gradient.len(),
                    got: gradient.len(),
                });
            }
        }

        let reputation = *self.participant_weights.get(&node_id).unwrap_or(&1.0);
        self.submissions.push(GradientSubmission {
            node_id,
            gradient,
            reputation,
        });

        Ok(())
    }

    /// Set reputation weight for a participant
    pub fn set_participant_weight(&mut self, node_id: String, weight: f32) {
        self.participant_weights.insert(node_id, weight);
    }

    /// Aggregate all submitted gradients
    pub fn aggregate(&mut self) -> Result<AggregationResult, AggregatorError> {
        if self.submissions.len() < self.config.min_participants {
            return Err(AggregatorError::InsufficientParticipants(
                self.config.min_participants,
            ));
        }

        let gradients: Vec<&[f32]> = self.submissions.iter().map(|s| s.gradient.as_slice()).collect();

        // Detect outliers
        let outlier_indices = self.detect_outliers(&gradients);

        // Filter out outliers
        let mut filtered_gradients = Vec::new();
        let mut excluded_outliers = Vec::new();
        for (i, submission) in self.submissions.iter().enumerate() {
            if outlier_indices.contains(&i) {
                excluded_outliers.push(submission.node_id.clone());
            } else {
                filtered_gradients.push(submission);
            }
        }

        if filtered_gradients.is_empty() {
            return Err(AggregatorError::NoGradientsSubmitted);
        }

        // Compute weights
        let weights: Vec<f32> = if self.config.use_reputation_weights {
            filtered_gradients
                .iter()
                .map(|s| s.reputation.max(0.01))
                .collect()
        } else {
            vec![1.0; filtered_gradients.len()]
        };

        // Compute weighted average
        let aggregated = self.compute_weighted_average(
            &filtered_gradients.iter().map(|s| s.gradient.as_slice()).collect::<Vec<_>>(),
            &weights,
        );

        // Apply compression if configured
        let compressed = if self.config.compression_ratio < 1.0 {
            self.apply_compression(&aggregated, self.config.compression_ratio)
        } else {
            aggregated
        };

        let result = AggregationResult::new(
            self.round,
            compressed.clone(),
            filtered_gradients.len(),
            excluded_outliers,
        );

        self.aggregated_gradients = Some(compressed);
        Ok(result)
    }

    /// Detect outlier gradients using median absolute deviation
    pub fn detect_outliers(&self, gradients: &[&[f32]]) -> Vec<usize> {
        if gradients.is_empty() {
            return Vec::new();
        }

        let dim = gradients[0].len();
        let n = gradients.len();

        // Compute per-dimension median
        let mut medians = vec![0.0f32; dim];
        for d in 0..dim {
            let mut vals: Vec<f32> = gradients.iter().map(|g| g[d]).collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            medians[d] = vals[n / 2];
        }

        // Compute MAD per dimension
        let mut mad = vec![0.0f32; dim];
        for d in 0..dim {
            let mut diffs: Vec<f32> = gradients
                .iter()
                .map(|g| (g[d] - medians[d]).abs())
                .collect();
            diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            mad[d] = diffs[n / 2];
        }

        // Score each gradient
        let mut outliers = Vec::new();
        for (i, gradient) in gradients.iter().enumerate() {
            let mut max_deviation = 0.0f32;
            for d in 0..dim {
                let deviation = if mad[d] > 0.0 {
                    (gradient[d] - medians[d]).abs() / mad[d]
                } else {
                    0.0
                };
                if deviation > max_deviation {
                    max_deviation = deviation;
                }
            }
            if max_deviation > self.config.outlier_threshold {
                outliers.push(i);
            }
        }

        outliers
    }

    /// Compute weighted average of gradients
    pub fn compute_weighted_average(&self, gradients: &[&[f32]], weights: &[f32]) -> Vec<f32> {
        if gradients.is_empty() || weights.is_empty() {
            return Vec::new();
        }

        let dim = gradients[0].len();
        let total_weight: f32 = weights.iter().sum();

        let mut result = vec![0.0f32; dim];
        for (i, gradient) in gradients.iter().enumerate() {
            let w = weights[i].max(0.0);
            for d in 0..dim {
                result[d] += gradient[d] * w;
            }
        }

        if total_weight > 0.0 {
            for val in result.iter_mut() {
                *val /= total_weight;
            }
        }

        result
    }

    /// Apply gradient compression via thresholding
    fn apply_compression(&self, gradient: &[f32], ratio: f32) -> Vec<f32> {
        if ratio >= 1.0 {
            return gradient.to_vec();
        }

        // Sort absolute values to find threshold
        let mut abs_values: Vec<f32> = gradient.iter().map(|v| v.abs()).collect();
        abs_values.sort_by(|a, b| b.partial_cmp(a).unwrap());

        let keep_count = (gradient.len() as f32 * ratio) as usize;
        let threshold = if keep_count > 0 && keep_count < abs_values.len() {
            abs_values[keep_count]
        } else {
            0.0
        };

        gradient
            .iter()
            .map(|v| {
                if v.abs() >= threshold {
                    *v
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Reset for a new aggregation round
    pub fn reset_round(&mut self) {
        self.round += 1;
        self.submissions.clear();
        self.aggregated_gradients = None;
    }

    /// Get current round number
    pub fn current_round(&self) -> u64 {
        self.round
    }

    /// Get number of submissions in current round
    pub fn submission_count(&self) -> usize {
        self.submissions.len()
    }

    /// Get last aggregated gradient
    pub fn get_last_aggregated(&self) -> Option<&Vec<f32>> {
        self.aggregated_gradients.as_ref()
    }

    /// Get configuration
    pub fn get_config(&self) -> &AggregatorConfig {
        &self.config
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> AggregatorConfig {
        AggregatorConfig::default()
    }

    fn make_gradient(dim: usize, value: f32) -> Vec<f32> {
        vec![value; dim]
    }

    #[test]
    fn test_aggregator_creation() {
        let config = make_config();
        let aggregator = GradientAggregatorV3::new(config);
        assert_eq!(aggregator.current_round(), 0);
        assert_eq!(aggregator.submission_count(), 0);
    }

    #[test]
    fn test_submit_gradient() {
        let config = make_config();
        let mut aggregator = GradientAggregatorV3::new(config);
        let gradient = make_gradient(10, 1.0);
        assert!(aggregator.submit_gradient("node-1".to_string(), gradient).is_ok());
        assert_eq!(aggregator.submission_count(), 1);
    }

    #[test]
    fn test_dimension_mismatch() {
        let config = make_config();
        let mut aggregator = GradientAggregatorV3::new(config);
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(10, 1.0))
            .unwrap();
        match aggregator.submit_gradient("node-2".to_string(), make_gradient(5, 1.0)) {
            Err(AggregatorError::DimensionMismatch { expected, got }) => {
                assert_eq!(expected, 10);
                assert_eq!(got, 5);
            }
            other => panic!("Expected DimensionMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_aggregate_basic() {
        let config = AggregatorConfig {
            compression_ratio: 1.0, // No compression
            ..Default::default()
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(4, 1.0))
            .unwrap();
        aggregator
            .submit_gradient("node-2".to_string(), make_gradient(4, 3.0))
            .unwrap();
        let result = aggregator.aggregate().unwrap();
        assert_eq!(result.aggregated_gradient, vec![2.0, 2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_aggregate_insufficient_participants() {
        let config = AggregatorConfig {
            min_participants: 3,
            ..Default::default()
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(4, 1.0))
            .unwrap();
        match aggregator.aggregate() {
            Err(AggregatorError::InsufficientParticipants(min)) => {
                assert_eq!(min, 3);
            }
            other => panic!("Expected InsufficientParticipants, got {:?}", other),
        }
    }

    #[test]
    fn test_outlier_detection() {
        let config = make_config();
        let aggregator = GradientAggregatorV3::new(config);
        let g1 = make_gradient(4, 1.0);
        let g2 = make_gradient(4, 1.1);
        let g3 = make_gradient(4, 1.0);
        let g4 = make_gradient(4, 100.0); // Outlier
        let gradients = [&g1[..], &g2[..], &g3[..], &g4[..]];
        let outliers = aggregator.detect_outliers(&gradients);
        assert!(outliers.contains(&3));
    }

    #[test]
    fn test_weighted_average() {
        let config = make_config();
        let aggregator = GradientAggregatorV3::new(config);
        let g1 = vec![1.0, 2.0];
        let g2 = vec![3.0, 4.0];
        let gradients = [&g1[..], &g2[..]];
        let weights = vec![1.0, 3.0];
        let result = aggregator.compute_weighted_average(&gradients, &weights);
        // (1*1 + 3*3) / 4 = 10/4 = 2.5
        assert!((result[0] - 2.5).abs() < 0.01);
        // (2*1 + 4*3) / 4 = 14/4 = 3.5
        assert!((result[1] - 3.5).abs() < 0.01);
    }

    #[test]
    fn test_reputation_weighted_aggregation() {
        let config = AggregatorConfig {
            compression_ratio: 1.0,
            use_reputation_weights: true,
            ..Default::default()
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        aggregator.set_participant_weight("node-1".to_string(), 1.0);
        aggregator.set_participant_weight("node-2".to_string(), 3.0);
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(2, 1.0))
            .unwrap();
        aggregator
            .submit_gradient("node-2".to_string(), make_gradient(2, 5.0))
            .unwrap();
        let result = aggregator.aggregate().unwrap();
        // (1*1 + 3*5) / 4 = 16/4 = 4.0
        assert!((result.aggregated_gradient[0] - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_compression() {
        let config = AggregatorConfig {
            compression_ratio: 0.5,
            min_participants: 1,
            ..Default::default()
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        let gradient = vec![1.0, 5.0, 3.0, 7.0, 2.0];
        aggregator
            .submit_gradient("node-1".to_string(), gradient)
            .unwrap();
        let result = aggregator.aggregate().unwrap();
        // Should have some zeros from compression
        let zero_count = result.aggregated_gradient.iter().filter(|v| **v == 0.0).count();
        assert!(zero_count > 0);
    }

    #[test]
    fn test_reset_round() {
        let config = make_config();
        let mut aggregator = GradientAggregatorV3::new(config);
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(4, 1.0))
            .unwrap();
        aggregator.reset_round();
        assert_eq!(aggregator.current_round(), 1);
        assert_eq!(aggregator.submission_count(), 0);
    }

    #[test]
    fn test_aggregation_result_norm() {
        let result = AggregationResult::new(0, vec![3.0, 4.0], 1, Vec::new());
        assert!((result.gradient_norm - 5.0).abs() < 0.01); // sqrt(9+16) = 5
    }

    #[test]
    fn test_get_last_aggregated() {
        let config = AggregatorConfig {
            compression_ratio: 1.0,
            min_participants: 1,
            ..Default::default()
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        assert!(aggregator.get_last_aggregated().is_none());
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(2, 1.0))
            .unwrap();
        aggregator.aggregate().unwrap();
        assert!(aggregator.get_last_aggregated().is_some());
    }

    #[test]
    fn test_config_default() {
        let config = AggregatorConfig::default();
        assert_eq!(config.compression_ratio, 0.8);
        assert_eq!(config.outlier_threshold, 2.0);
        assert!(config.use_reputation_weights);
    }

    #[test]
    fn test_error_display() {
        let err = AggregatorError::NoGradientsSubmitted;
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_multiple_rounds() {
        let config = AggregatorConfig {
            compression_ratio: 1.0,
            min_participants: 1,
            ..Default::default()
        };
        let mut aggregator = GradientAggregatorV3::new(config);

        // Round 0
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(2, 1.0))
            .unwrap();
        aggregator.aggregate().unwrap();
        aggregator.reset_round();

        // Round 1
        aggregator
            .submit_gradient("node-1".to_string(), make_gradient(2, 2.0))
            .unwrap();
        let result = aggregator.aggregate().unwrap();
        assert_eq!(result.round, 1);
    }

    #[test]
    fn test_empty_outlier_detection() {
        let config = make_config();
        let aggregator = GradientAggregatorV3::new(config);
        let gradients: Vec<&[f32]> = Vec::new();
        let outliers = aggregator.detect_outliers(&gradients);
        assert!(outliers.is_empty());
    }
}
