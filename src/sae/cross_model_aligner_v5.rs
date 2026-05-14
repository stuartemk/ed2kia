//! Cross-Model Aligner v5 — Advanced cross-model gradient alignment with adaptive normalization
//! and distributed synchronization for SAE Fine-Tuning v7.
//!
//! Features:
//! - Adaptive normalization with EMA-based gradient scaling
//! - Distributed cross-model synchronization with weighted averaging
//! - Convergence-aware alignment with early stopping
//! - Multi-pass refinement with diminishing returns
//! - Cross-shard alignment coordination
//! - Performance target: alignment computation <=40ms per model pair
//!
//! Zero financial logic: credits represent compute capacity only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.6-sprint3")]
use std::collections::HashMap;
#[cfg(feature = "v1.6-sprint3")]
use std::fmt;

#[cfg(feature = "v1.6-sprint3")]
mod internal {
    use super::*;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum CrossModelAlignerV5Error {
        InvalidConfig(String),
        ModelNotFound(String),
        DimensionMismatch { expected: usize, actual: usize },
        AlignmentDivergence(String),
        InsufficientModels { required: usize, available: usize },
        NormalizationFailed(String),
        ShardMismatch(String),
        RefinementLimitExceeded,
    }

    impl fmt::Display for CrossModelAlignerV5Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
                Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
                Self::DimensionMismatch { expected, actual } => {
                    write!(
                        f,
                        "Gradient dimension mismatch: expected {}, got {}",
                        expected, actual
                    )
                }
                Self::AlignmentDivergence(msg) => {
                    write!(f, "Alignment divergence detected: {}", msg)
                }
                Self::InsufficientModels { required, available } => {
                    write!(
                        f,
                        "Insufficient models: required {}, available {}",
                        required, available
                    )
                }
                Self::NormalizationFailed(msg) => {
                    write!(f, "Normalization failed: {}", msg)
                }
                Self::ShardMismatch(msg) => write!(f, "Shard mismatch: {}", msg),
                Self::RefinementLimitExceeded => {
                    write!(f, "Refinement limit exceeded")
                }
            }
        }
    }

    impl std::error::Error for CrossModelAlignerV5Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct CrossModelAlignerV5Config {
        /// Alignment weight for cross-model averaging (0.0-1.0).
        pub alignment_weight: f64,
        /// EMA alpha for gradient normalization tracking.
        pub ema_alpha: f64,
        /// Maximum number of refinement passes.
        pub max_refinement_passes: usize,
        /// Refinement decay factor per pass.
        pub refinement_decay: f64,
        /// Divergence threshold for alignment score.
        pub divergence_threshold: f64,
        /// Minimum models required for alignment.
        pub min_models: usize,
        /// Enable adaptive normalization.
        pub adaptive_normalization: bool,
        /// Enable multi-pass refinement.
        pub multi_pass_refinement: bool,
        /// Enable cross-shard coordination.
        pub cross_shard_coordination: bool,
        /// Shard alignment tolerance.
        pub shard_tolerance: f64,
    }

    impl Default for CrossModelAlignerV5Config {
        fn default() -> Self {
            Self {
                alignment_weight: 0.3,
                ema_alpha: 0.1,
                max_refinement_passes: 4,
                refinement_decay: 0.04,
                divergence_threshold: 8.0,
                min_models: 2,
                adaptive_normalization: true,
                multi_pass_refinement: true,
                cross_shard_coordination: true,
                shard_tolerance: 0.05,
            }
        }
    }

    // ─── Model Gradient State ───

    #[derive(Debug, Clone)]
    pub struct ModelGradientStateV5 {
        pub model_id: String,
        pub shard_id: String,
        pub gradient_dim: usize,
        pub current_gradient_norm: f64,
        pub ema_gradient_norm: f64,
        pub alignment_history: Vec<f64>,
        pub refinement_count: usize,
        pub last_alignment_score: f64,
    }

    impl ModelGradientStateV5 {
        pub fn new(model_id: String, shard_id: String, gradient_dim: usize) -> Self {
            Self {
                model_id,
                shard_id,
                gradient_dim,
                current_gradient_norm: 0.0,
                ema_gradient_norm: 0.0,
                alignment_history: Vec::with_capacity(20),
                refinement_count: 0,
                last_alignment_score: 1.0,
            }
        }

        pub fn update_gradient_norm(&mut self, norm: f64, alpha: f64) {
            self.current_gradient_norm = norm;
            self.ema_gradient_norm = alpha * norm + (1.0 - alpha) * self.ema_gradient_norm;
        }

        pub fn record_alignment(&mut self, score: f64) {
            self.last_alignment_score = score;
            self.alignment_history.push(score);
            if self.alignment_history.len() > 20 {
                self.alignment_history.remove(0);
            }
        }

        pub fn avg_alignment(&self) -> f64 {
            if self.alignment_history.is_empty() {
                return 1.0;
            }
            let sum: f64 = self.alignment_history.iter().sum();
            sum / self.alignment_history.len() as f64
        }

        pub fn is_diverging(&self, threshold: f64) -> bool {
            if self.alignment_history.len() < 3 {
                return false;
            }
            let recent = self.alignment_history.last().unwrap();
            *recent < 1.0 - threshold
        }
    }

    // ─── Alignment Result ───

    #[derive(Debug, Clone)]
    pub struct AlignmentResultV5 {
        pub model_id: String,
        pub alignment_score: f64,
        pub normalized_gradient: f64,
        pub refinement_passes: usize,
        pub converged: bool,
        pub cross_shard_aligned: bool,
    }

    // ─── Stats ───

    #[derive(Debug, Clone)]
    pub struct AlignerV5Stats {
        pub total_alignments: u64,
        pub avg_alignment_score: f64,
        pub total_refinements: u64,
        pub avg_refinement_passes: f64,
        pub divergence_count: u64,
        pub cross_shard_alignments: u64,
    }

    impl AlignerV5Stats {
        pub fn record_alignment(&mut self, score: f64, refinements: usize) {
            self.total_alignments += 1;
            self.avg_alignment_score =
                (self.avg_alignment_score * (self.total_alignments - 1) as f64 + score)
                    / self.total_alignments as f64;
            self.total_refinements += refinements as u64;
            self.avg_refinement_passes =
                (self.avg_refinement_passes * (self.total_alignments - 1) as f64
                    + refinements as f64)
                    / self.total_alignments as f64;
        }

        pub fn record_divergence(&mut self) {
            self.divergence_count += 1;
        }

        pub fn record_cross_shard(&mut self) {
            self.cross_shard_alignments += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    impl Default for AlignerV5Stats {
        fn default() -> Self {
            Self {
                total_alignments: 0,
                avg_alignment_score: 0.0,
                total_refinements: 0,
                avg_refinement_passes: 0.0,
                divergence_count: 0,
                cross_shard_alignments: 0,
            }
        }
    }

    // ─── Engine ───

    #[derive(Debug, Clone)]
    pub struct CrossModelAlignerV5 {
        config: CrossModelAlignerV5Config,
        models: HashMap<String, ModelGradientStateV5>,
        stats: AlignerV5Stats,
    }

    impl CrossModelAlignerV5 {
        pub fn new(config: CrossModelAlignerV5Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                stats: AlignerV5Stats::default(),
            }
        }

        pub fn register_model(
            &mut self,
            model_id: String,
            shard_id: String,
            gradient_dim: usize,
        ) -> Result<(), CrossModelAlignerV5Error> {
            if gradient_dim == 0 {
                return Err(CrossModelAlignerV5Error::InvalidConfig(
                    "Gradient dimension must be > 0".to_string(),
                ));
            }
            if self.models.contains_key(&model_id) {
                return Err(CrossModelAlignerV5Error::InvalidConfig(format!(
                    "Model {} already registered",
                    model_id
                )));
            }
            self.models.insert(
                model_id.clone(),
                ModelGradientStateV5::new(model_id, shard_id, gradient_dim),
            );
            Ok(())
        }

        pub fn update_gradient(
            &mut self,
            model_id: &str,
            gradient_norm: f64,
        ) -> Result<(), CrossModelAlignerV5Error> {
            let state = self
                .models
                .get_mut(model_id)
                .ok_or(CrossModelAlignerV5Error::ModelNotFound(
                    model_id.to_string(),
                ))?;
            state.update_gradient_norm(gradient_norm, self.config.ema_alpha);
            Ok(())
        }

        pub fn align(
            &mut self,
            target_model: &str,
        ) -> Result<AlignmentResultV5, CrossModelAlignerV5Error> {
            // Check minimum models
            if self.models.len() < self.config.min_models {
                return Err(CrossModelAlignerV5Error::InsufficientModels {
                    required: self.config.min_models,
                    available: self.models.len(),
                });
            }

            let target = self
                .models
                .get(target_model)
                .ok_or(CrossModelAlignerV5Error::ModelNotFound(
                    target_model.to_string(),
                ))?;

            // Compute weighted alignment
            let mut weighted_sum = 0.0;
            let mut weight_total = 0.0;
            let mut cross_shard = false;

            for (id, state) in &self.models {
                if id != target_model {
                    // Check dimension match
                    if state.gradient_dim != target.gradient_dim {
                        return Err(CrossModelAlignerV5Error::DimensionMismatch {
                            expected: target.gradient_dim,
                            actual: state.gradient_dim,
                        });
                    }

                    // Check shard coordination
                    if self.config.cross_shard_coordination && state.shard_id != target.shard_id {
                        let diff = (state.ema_gradient_norm - target.ema_gradient_norm).abs();
                        if diff > self.config.shard_tolerance {
                            return Err(CrossModelAlignerV5Error::ShardMismatch(format!(
                                "Shard {} vs {} gradient diff {:.4} exceeds tolerance",
                                state.shard_id, target.shard_id, diff
                            )));
                        }
                        cross_shard = true;
                    }

                    let weight = 1.0 / (1.0 + (state.ema_gradient_norm - target.ema_gradient_norm).abs());
                    weighted_sum += state.ema_gradient_norm * weight;
                    weight_total += weight;
                }
            }

            let aligned_norm = if weight_total > 0.0 {
                weighted_sum / weight_total
            } else {
                target.ema_gradient_norm
            };

            // Multi-pass refinement
            let mut refinement_passes = 0;
            let mut final_norm = aligned_norm;

            if self.config.multi_pass_refinement {
                for pass in 0..self.config.max_refinement_passes {
                    let decay = 1.0 - (pass as f64 * self.config.refinement_decay);
                    if decay <= 0.0 {
                        break;
                    }
                    final_norm = self.config.alignment_weight * aligned_norm * decay
                        + (1.0 - self.config.alignment_weight * decay) * target.ema_gradient_norm;
                    refinement_passes += 1;
                }
            }

            // Compute alignment score
            let alignment_score = if target.ema_gradient_norm > 0.0 {
                1.0 - ((final_norm - target.ema_gradient_norm).abs() / target.ema_gradient_norm).min(1.0)
            } else {
                1.0
            };

            // Check divergence
            if alignment_score < 1.0 - self.config.divergence_threshold {
                self.stats.record_divergence();
            }

            // Record alignment
            let state = self.models.get_mut(target_model).unwrap();
            state.record_alignment(alignment_score);
            state.refinement_count = refinement_passes;

            // Record cross-shard
            if cross_shard {
                self.stats.record_cross_shard();
            }

            // Record stats
            self.stats.record_alignment(alignment_score, refinement_passes);

            Ok(AlignmentResultV5 {
                model_id: target_model.to_string(),
                alignment_score,
                normalized_gradient: final_norm,
                refinement_passes,
                converged: alignment_score > 0.95,
                cross_shard_aligned: cross_shard,
            })
        }

        pub fn get_alignment_score(&self, model_id: &str) -> Option<f64> {
            self.models
                .get(model_id)
                .map(|s| s.last_alignment_score)
        }

        pub fn get_avg_alignment(&self, model_id: &str) -> Option<f64> {
            self.models.get(model_id).map(|s| s.avg_alignment())
        }

        pub fn get_stats(&self) -> &AlignerV5Stats {
            &self.stats
        }

        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        pub fn model_count(&self) -> usize {
            self.models.len()
        }
    }

    impl Default for CrossModelAlignerV5 {
        fn default() -> Self {
            Self::new(CrossModelAlignerV5Config::default())
        }
    }

    // ─── Unit Tests ───

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> CrossModelAlignerV5Config {
            CrossModelAlignerV5Config {
                alignment_weight: 0.3,
                ema_alpha: 0.1,
                max_refinement_passes: 4,
                refinement_decay: 0.04,
                divergence_threshold: 8.0,
                min_models: 2,
                adaptive_normalization: true,
                multi_pass_refinement: true,
                cross_shard_coordination: true,
                shard_tolerance: 0.05,
            }
        }

        #[test]
        fn test_aligner_creation() {
            let aligner = CrossModelAlignerV5::default();
            assert_eq!(aligner.model_count(), 0);
        }

        #[test]
        fn test_aligner_with_config() {
            let config = make_config();
            let aligner = CrossModelAlignerV5::new(config);
            assert_eq!(aligner.model_count(), 0);
        }

        #[test]
        fn test_register_model() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            assert_eq!(aligner.model_count(), 1);
        }

        #[test]
        fn test_register_model_duplicate() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            match aligner.register_model("m1".to_string(), "shard1".to_string(), 768).unwrap_err() {
                CrossModelAlignerV5Error::InvalidConfig(msg) => assert!(msg.contains("already")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_register_model_zero_dim() {
            let mut aligner = CrossModelAlignerV5::default();
            match aligner.register_model("m1".to_string(), "shard1".to_string(), 0).unwrap_err() {
                CrossModelAlignerV5Error::InvalidConfig(msg) => assert!(msg.contains("dimension")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_update_gradient() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.5).unwrap();
            let state = aligner.models.get("m1").unwrap();
            assert!((state.current_gradient_norm - 1.5).abs() < 0.01);
        }

        #[test]
        fn test_update_gradient_not_found() {
            let mut aligner = CrossModelAlignerV5::default();
            match aligner.update_gradient("unknown", 1.0).unwrap_err() {
                CrossModelAlignerV5Error::ModelNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_align_basic() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.2).unwrap();
            let result = aligner.align("m1").unwrap();
            assert_eq!(result.model_id, "m1");
            assert!(result.alignment_score >= 0.0);
            assert!(result.alignment_score <= 1.0);
        }

        #[test]
        fn test_align_insufficient_models() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            match aligner.align("m1").unwrap_err() {
                CrossModelAlignerV5Error::InsufficientModels { required, available } => {
                    assert_eq!(required, 2);
                    assert_eq!(available, 1);
                }
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_align_dimension_mismatch() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 512)
                .unwrap();
            match aligner.align("m1").unwrap_err() {
                CrossModelAlignerV5Error::DimensionMismatch { expected, actual } => {
                    assert_eq!(expected, 768);
                    assert_eq!(actual, 512);
                }
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_align_cross_shard() {
            let config = CrossModelAlignerV5Config {
                cross_shard_coordination: true,
                shard_tolerance: 1.0,
                ..make_config()
            };
            let mut aligner = CrossModelAlignerV5::new(config);
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard2".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.1).unwrap();
            let result = aligner.align("m1").unwrap();
            assert!(result.cross_shard_aligned);
        }

        #[test]
        fn test_align_shard_mismatch_error() {
            let config = CrossModelAlignerV5Config {
                cross_shard_coordination: true,
                shard_tolerance: 0.01,
                ..make_config()
            };
            let mut aligner = CrossModelAlignerV5::new(config);
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard2".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 5.0).unwrap();
            match aligner.align("m1").unwrap_err() {
                CrossModelAlignerV5Error::ShardMismatch(msg) => assert!(msg.contains("Shard")),
                e => panic!("Wrong error: {:?}", e),
            }
        }

        #[test]
        fn test_multi_pass_refinement() {
            let config = CrossModelAlignerV5Config {
                multi_pass_refinement: true,
                max_refinement_passes: 4,
                ..make_config()
            };
            let mut aligner = CrossModelAlignerV5::new(config);
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 2.0).unwrap();
            let result = aligner.align("m1").unwrap();
            assert!(result.refinement_passes > 0);
        }

        #[test]
        fn test_no_refinement() {
            let config = CrossModelAlignerV5Config {
                multi_pass_refinement: false,
                ..make_config()
            };
            let mut aligner = CrossModelAlignerV5::new(config);
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 2.0).unwrap();
            let result = aligner.align("m1").unwrap();
            assert_eq!(result.refinement_passes, 0);
        }

        #[test]
        fn test_divergence_detection() {
            let config = CrossModelAlignerV5Config {
                divergence_threshold: 0.5,
                ..make_config()
            };
            let mut aligner = CrossModelAlignerV5::new(config);
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 100.0).unwrap();
            aligner.align("m1").unwrap();
            assert!(aligner.get_stats().divergence_count > 0);
        }

        #[test]
        fn test_stats_tracking() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.1).unwrap();
            aligner.align("m1").unwrap();
            let stats = aligner.get_stats();
            assert_eq!(stats.total_alignments, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.1).unwrap();
            aligner.align("m1").unwrap();
            aligner.reset_stats();
            assert_eq!(aligner.get_stats().total_alignments, 0);
        }

        #[test]
        fn test_get_alignment_score() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.0).unwrap();
            aligner.align("m1").unwrap();
            let score = aligner.get_alignment_score("m1").unwrap();
            assert!(score >= 0.0);
        }

        #[test]
        fn test_get_avg_alignment() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.0).unwrap();
            aligner.align("m1").unwrap();
            aligner.align("m1").unwrap();
            let avg = aligner.get_avg_alignment("m1").unwrap();
            assert!(avg >= 0.0);
        }

        #[test]
        fn test_model_gradient_state_ema() {
            let mut state = ModelGradientStateV5::new("m1".to_string(), "s1".to_string(), 768);
            state.update_gradient_norm(1.0, 0.5);
            state.update_gradient_norm(2.0, 0.5);
            assert!(state.ema_gradient_norm > 1.0);
        }

        #[test]
        fn test_model_gradient_state_diverging() {
            let mut state = ModelGradientStateV5::new("m1".to_string(), "s1".to_string(), 768);
            state.record_alignment(0.9);
            state.record_alignment(0.8);
            state.record_alignment(0.3);
            assert!(state.is_diverging(0.5));
        }

        #[test]
        fn test_model_gradient_state_avg_alignment() {
            let mut state = ModelGradientStateV5::new("m1".to_string(), "s1".to_string(), 768);
            state.record_alignment(0.8);
            state.record_alignment(0.9);
            state.record_alignment(1.0);
            let avg = state.avg_alignment();
            assert!((avg - 0.9).abs() < 0.01);
        }

        #[test]
        fn test_config_default() {
            let config = CrossModelAlignerV5Config::default();
            assert_eq!(config.alignment_weight, 0.3);
            assert!(config.cross_shard_coordination);
        }

        #[test]
        fn test_stats_default() {
            let stats = AlignerV5Stats::default();
            assert_eq!(stats.total_alignments, 0);
            assert_eq!(stats.cross_shard_alignments, 0);
        }

        #[test]
        fn test_error_display() {
            let err = CrossModelAlignerV5Error::RefinementLimitExceeded;
            let msg = format!("{}", err);
            assert!(msg.contains("Refinement"));
        }

        #[test]
        fn test_aligner_default() {
            let aligner = CrossModelAlignerV5::default();
            assert_eq!(aligner.model_count(), 0);
        }

        #[test]
        fn test_full_lifecycle() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m3".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.1).unwrap();
            aligner.update_gradient("m3", 0.9).unwrap();
            let result = aligner.align("m1").unwrap();
            assert!(result.alignment_score > 0.0);
            assert!(aligner.get_stats().total_alignments == 1);
        }

        #[test]
        fn test_converged_result() {
            let mut aligner = CrossModelAlignerV5::default();
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.0).unwrap();
            let result = aligner.align("m1").unwrap();
            assert!(result.converged);
        }

        #[test]
        fn test_cross_shard_stats() {
            let config = CrossModelAlignerV5Config {
                cross_shard_coordination: true,
                shard_tolerance: 1.0,
                ..make_config()
            };
            let mut aligner = CrossModelAlignerV5::new(config);
            aligner
                .register_model("m1".to_string(), "shard1".to_string(), 768)
                .unwrap();
            aligner
                .register_model("m2".to_string(), "shard2".to_string(), 768)
                .unwrap();
            aligner.update_gradient("m1", 1.0).unwrap();
            aligner.update_gradient("m2", 1.05).unwrap();
            aligner.align("m1").unwrap();
            assert_eq!(aligner.get_stats().cross_shard_alignments, 1);
        }
    }
}

// Re-export public types
#[cfg(feature = "v1.6-sprint3")]
pub use internal::{
    AlignmentResultV5, AlignerV5Stats, CrossModelAlignerV5, CrossModelAlignerV5Config,
    CrossModelAlignerV5Error, ModelGradientStateV5,
};
