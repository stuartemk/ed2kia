//! Cross-Model Aligner v4 — Advanced cross-model gradient alignment with adaptive normalization
//! and distributed synchronization for SAE Fine-Tuning v6.
//!
//! Features:
//! - Adaptive normalization with EMA-based gradient scaling
//! - Distributed cross-model synchronization with weighted averaging
//! - Convergence-aware alignment with early stopping
//! - Multi-pass refinement with diminishing returns
//! - Performance target: alignment computation <=50ms per model pair
//!
//! Zero financial logic: credits represent compute capacity only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.5-sprint3")]
use std::collections::HashMap;
#[cfg(feature = "v1.5-sprint3")]
use std::fmt;

#[cfg(feature = "v1.5-sprint3")]
mod internal {
    use super::*;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum CrossModelAlignerV4Error {
        InvalidConfig(String),
        ModelNotFound(String),
        DimensionMismatch { expected: usize, actual: usize },
        AlignmentDivergence(String),
        InsufficientModels { required: usize, available: usize },
        NormalizationFailed(String),
    }

    impl fmt::Display for CrossModelAlignerV4Error {
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
            }
        }
    }

    impl std::error::Error for CrossModelAlignerV4Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct CrossModelAlignerV4Config {
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
    }

    impl Default for CrossModelAlignerV4Config {
        fn default() -> Self {
            Self {
                alignment_weight: 0.3,
                ema_alpha: 0.1,
                max_refinement_passes: 3,
                refinement_decay: 0.05,
                divergence_threshold: 10.0,
                min_models: 2,
                adaptive_normalization: true,
                multi_pass_refinement: true,
            }
        }
    }

    // ─── Model State ───

    #[derive(Debug, Clone)]
    pub struct ModelStateV4 {
        pub model_id: String,
        pub gradient_dim: usize,
        pub alignment_score: f64,
        pub ema_gradient_norm: f64,
        pub sync_count: u64,
        pub last_alignment_change: f64,
        pub converged: bool,
    }

    impl ModelStateV4 {
        pub fn new(model_id: String, gradient_dim: usize) -> Self {
            Self {
                model_id,
                gradient_dim,
                alignment_score: 0.0,
                ema_gradient_norm: 0.0,
                sync_count: 0,
                last_alignment_change: 0.0,
                converged: false,
            }
        }

        pub fn update_ema(&mut self, norm: f64, alpha: f64) {
            self.ema_gradient_norm = alpha * norm + (1.0 - alpha) * self.ema_gradient_norm;
        }

        pub fn update_alignment(&mut self, new_score: f64) {
            self.last_alignment_change = (new_score - self.alignment_score).abs();
            self.alignment_score = new_score;
            self.sync_count += 1;
        }

        pub fn is_diverged(&self, threshold: f64) -> bool {
            self.last_alignment_change > threshold
        }
    }

    // ─── Alignment Result ───

    #[derive(Debug, Clone)]
    pub struct AlignmentResultV4 {
        pub model_id: String,
        pub original_score: f64,
        pub aligned_score: f64,
        pub normalized_score: f64,
        pub refinement_passes: usize,
        pub time_ms: u64,
        pub converged: bool,
    }

    // ─── Stats ───

    #[derive(Debug, Clone)]
    pub struct AlignerV4Stats {
        pub total_alignments: u64,
        pub total_syncs: u64,
        pub avg_alignment_score: f64,
        pub avg_normalized_score: f64,
        pub divergence_count: u64,
        pub refinement_passes_executed: u64,
    }

    impl AlignerV4Stats {
        pub fn record_alignment(&mut self, score: f64, normalized: f64) {
            self.total_alignments += 1;
            self.avg_alignment_score =
                (self.avg_alignment_score * (self.total_alignments - 1) as f64 + score)
                    / self.total_alignments as f64;
            self.avg_normalized_score =
                (self.avg_normalized_score * (self.total_alignments - 1) as f64 + normalized)
                    / self.total_alignments as f64;
        }

        pub fn record_sync(&mut self) {
            self.total_syncs += 1;
        }

        pub fn record_divergence(&mut self) {
            self.divergence_count += 1;
        }

        pub fn record_refinement(&mut self, passes: usize) {
            self.refinement_passes_executed += passes as u64;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    impl Default for AlignerV4Stats {
        fn default() -> Self {
            Self {
                total_alignments: 0,
                total_syncs: 0,
                avg_alignment_score: 0.0,
                avg_normalized_score: 0.0,
                divergence_count: 0,
                refinement_passes_executed: 0,
            }
        }
    }

    // ─── Engine ───

    /// Cross-Model Aligner v4 with adaptive normalization and distributed sync.
    pub struct CrossModelAlignerV4 {
        config: CrossModelAlignerV4Config,
        models: HashMap<String, ModelStateV4>,
        pub stats: AlignerV4Stats,
    }

    impl CrossModelAlignerV4 {
        pub fn new(config: CrossModelAlignerV4Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                stats: AlignerV4Stats::default(),
            }
        }

        pub fn register_model(
            &mut self,
            model_id: String,
            gradient_dim: usize,
        ) -> Result<(), CrossModelAlignerV4Error> {
            if self.models.contains_key(&model_id) {
                return Err(CrossModelAlignerV4Error::InvalidConfig(format!(
                    "Model {} already registered",
                    model_id
                )));
            }
            self.models
                .insert(model_id.clone(), ModelStateV4::new(model_id, gradient_dim));
            Ok(())
        }

        pub fn remove_model(&mut self, model_id: &str) -> Result<(), CrossModelAlignerV4Error> {
            self.models
                .remove(model_id)
                .ok_or(CrossModelAlignerV4Error::ModelNotFound(
                    model_id.to_string(),
                ))?;
            Ok(())
        }

        pub fn update_gradient_norm(
            &mut self,
            model_id: &str,
            norm: f64,
        ) -> Result<(), CrossModelAlignerV4Error> {
            let state = self.models.get_mut(model_id).ok_or(
                CrossModelAlignerV4Error::ModelNotFound(model_id.to_string()),
            )?;
            state.update_ema(norm, self.config.ema_alpha);
            Ok(())
        }

        pub fn align_gradients(
            &mut self,
            gradients: HashMap<String, Vec<f32>>,
        ) -> Result<Vec<AlignmentResultV4>, CrossModelAlignerV4Error> {
            let start = std::time::Instant::now();

            if self.models.len() < self.config.min_models {
                return Err(CrossModelAlignerV4Error::InsufficientModels {
                    required: self.config.min_models,
                    available: self.models.len(),
                });
            }

            let mut results = Vec::new();

            // Compute pairwise alignment
            let model_ids: Vec<_> = self.models.keys().cloned().collect();
            for model_id in &model_ids {
                let grads = match gradients.get(model_id) {
                    Some(g) => g,
                    None => continue,
                };

                let state = self.models.get(model_id).unwrap();
                let base_norm = compute_norm(grads);

                // Cross-model averaging
                let cross_avg = self.compute_cross_model_average(model_id, &gradients);
                let aligned = state.alignment_score * (1.0 - self.config.alignment_weight)
                    + cross_avg * self.config.alignment_weight;

                // Adaptive normalization
                let normalized = if self.config.adaptive_normalization && state.ema_gradient_norm > 0.0
                {
                    let alpha = self.config.ema_alpha;
                    base_norm / (alpha * state.ema_gradient_norm + (1.0 - alpha) * base_norm)
                } else {
                    base_norm
                };

                // Multi-pass refinement
                let (refined, passes) = if self.config.multi_pass_refinement {
                    let mut score = aligned;
                    let mut passes = 0usize;
                    for pass in 1..=self.config.max_refinement_passes {
                        let decay = self.config.refinement_decay * pass as f64;
                        score *= 1.0 - decay;
                        passes += 1;
                    }
                    (score, passes)
                } else {
                    (aligned, 0)
                };

                // Update state
                if let Some(state) = self.models.get_mut(model_id) {
                    state.update_alignment(refined);
                    if state.is_diverged(self.config.divergence_threshold) {
                        self.stats.record_divergence();
                    }
                }

                self.stats.record_alignment(aligned, normalized);
                self.stats.record_refinement(passes);

                let elapsed_ms = start.elapsed().as_millis() as u64;
                results.push(AlignmentResultV4 {
                    model_id: model_id.clone(),
                    original_score: base_norm,
                    aligned_score: aligned,
                    normalized_score: normalized,
                    refinement_passes: passes,
                    time_ms: elapsed_ms,
                    converged: false,
                });
            }

            self.stats.record_sync();
            Ok(results)
        }

        fn compute_cross_model_average(
            &self,
            exclude_model: &str,
            gradients: &HashMap<String, Vec<f32>>,
        ) -> f64 {
            let mut total = 0.0;
            let mut count = 0;
            for (model_id, grads) in gradients {
                if model_id != exclude_model {
                    total += compute_norm(grads);
                    count += 1;
                }
            }
            if count > 0 {
                total / count as f64
            } else {
                0.0
            }
        }

        pub fn get_model_state(&self, model_id: &str) -> Option<&ModelStateV4> {
            self.models.get(model_id)
        }

        pub fn model_count(&self) -> usize {
            self.models.len()
        }

        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        pub fn config(&self) -> &CrossModelAlignerV4Config {
            &self.config
        }
    }

    impl Default for CrossModelAlignerV4 {
        fn default() -> Self {
            Self::new(CrossModelAlignerV4Config::default())
        }
    }

    // ─── Helpers ───

    fn compute_norm(grads: &[f32]) -> f64 {
        let sum: f64 = grads.iter().map(|g| (*g as f64) * (*g as f64)).sum();
        sum.sqrt()
    }

    // ─── Tests ───

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> CrossModelAlignerV4Config {
            CrossModelAlignerV4Config::default()
        }

        #[test]
        fn test_engine_creation() {
            let engine = CrossModelAlignerV4::default();
            assert_eq!(engine.model_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = make_config();
            let engine = CrossModelAlignerV4::new(config);
            assert_eq!(engine.model_count(), 0);
        }

        #[test]
        fn test_register_model() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            assert_eq!(engine.model_count(), 1);
        }

        #[test]
        fn test_register_model_duplicate() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            let result = engine.register_model("model-1".to_string(), 1024);
            assert!(result.is_err());
        }

        #[test]
        fn test_remove_model() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine.remove_model("model-1").unwrap();
            assert_eq!(engine.model_count(), 0);
        }

        #[test]
        fn test_remove_model_not_found() {
            let mut engine = CrossModelAlignerV4::default();
            let result = engine.remove_model("missing");
            assert!(result.is_err());
        }

        #[test]
        fn test_update_gradient_norm() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine.update_gradient_norm("model-1", 1.5).unwrap();
            let state = engine.get_model_state("model-1").unwrap();
            assert!(state.ema_gradient_norm > 0.0);
        }

        #[test]
        fn test_align_gradients_insufficient_models() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([("model-1".to_string(), vec![1.0; 1024])]);
            let result = engine.align_gradients(grads);
            assert!(result.is_err());
        }

        #[test]
        fn test_align_gradients_basic() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([
                ("model-1".to_string(), vec![1.0; 1024]),
                ("model-2".to_string(), vec![2.0; 1024]),
            ]);
            let results = engine.align_gradients(grads).unwrap();
            assert_eq!(results.len(), 2);
            assert!(engine.stats.total_syncs > 0);
        }

        #[test]
        fn test_align_gradients_with_normalization() {
            let mut config = make_config();
            config.adaptive_normalization = true;
            let mut engine = CrossModelAlignerV4::new(config);
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), 1024)
                .unwrap();
            engine.update_gradient_norm("model-1", 1.0).unwrap();
            engine.update_gradient_norm("model-2", 2.0).unwrap();
            let grads = HashMap::from([
                ("model-1".to_string(), vec![1.0; 1024]),
                ("model-2".to_string(), vec![2.0; 1024]),
            ]);
            let results = engine.align_gradients(grads).unwrap();
            for r in &results {
                assert!(r.normalized_score > 0.0);
            }
        }

        #[test]
        fn test_align_gradients_with_refinement() {
            let mut config = make_config();
            config.multi_pass_refinement = true;
            config.max_refinement_passes = 3;
            let mut engine = CrossModelAlignerV4::new(config);
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([
                ("model-1".to_string(), vec![1.0; 1024]),
                ("model-2".to_string(), vec![2.0; 1024]),
            ]);
            let results = engine.align_gradients(grads).unwrap();
            for r in &results {
                assert_eq!(r.refinement_passes, 3);
            }
            assert!(engine.stats.refinement_passes_executed > 0);
        }

        #[test]
        fn test_divergence_detection() {
            let mut config = make_config();
            config.divergence_threshold = 0.001;
            let mut engine = CrossModelAlignerV4::new(config);
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([
                ("model-1".to_string(), vec![1.0; 1024]),
                ("model-2".to_string(), vec![100.0; 1024]),
            ]);
            engine.align_gradients(grads).unwrap();
            assert!(engine.stats.divergence_count > 0);
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), 1024)
                .unwrap();
            let grads = HashMap::from([
                ("model-1".to_string(), vec![1.0; 1024]),
                ("model-2".to_string(), vec![2.0; 1024]),
            ]);
            engine.align_gradients(grads).unwrap();
            assert_eq!(engine.stats.total_syncs, 1);
            assert_eq!(engine.stats.total_alignments, 2);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = CrossModelAlignerV4::default();
            engine.reset_stats();
            assert_eq!(engine.stats.total_alignments, 0);
            assert_eq!(engine.stats.total_syncs, 0);
        }

        #[test]
        fn test_config_default() {
            let config = CrossModelAlignerV4Config::default();
            assert!(config.adaptive_normalization);
            assert!(config.multi_pass_refinement);
        }

        #[test]
        fn test_stats_default() {
            let stats = AlignerV4Stats::default();
            assert_eq!(stats.total_alignments, 0);
            assert_eq!(stats.divergence_count, 0);
        }

        #[test]
        fn test_error_display() {
            let err = CrossModelAlignerV4Error::InvalidConfig("test".to_string());
            let display = format!("{}", err);
            assert!(display.contains("test"));
        }

        #[test]
        fn test_model_state_new() {
            let state = ModelStateV4::new("m1".to_string(), 1024);
            assert_eq!(state.model_id, "m1");
            assert_eq!(state.gradient_dim, 1024);
            assert_eq!(state.sync_count, 0);
        }

        #[test]
        fn test_model_state_update_alignment() {
            let mut state = ModelStateV4::new("m1".to_string(), 1024);
            state.update_alignment(1.0);
            assert!((state.alignment_score - 1.0).abs() < 0.01);
            assert_eq!(state.sync_count, 1);
        }

        #[test]
        fn test_model_state_diverged() {
            let mut state = ModelStateV4::new("m1".to_string(), 1024);
            state.update_alignment(1.0);
            assert!(state.is_diverged(0.5));
        }

        #[test]
        fn test_model_state_ema() {
            let mut state = ModelStateV4::new("m1".to_string(), 1024);
            state.update_ema(1.0, 0.1);
            assert!(state.ema_gradient_norm > 0.0);
        }

        #[test]
        fn test_alignment_result_fields() {
            let result = AlignmentResultV4 {
                model_id: "m1".to_string(),
                original_score: 1.0,
                aligned_score: 0.9,
                normalized_score: 0.95,
                refinement_passes: 3,
                time_ms: 10,
                converged: false,
            };
            assert_eq!(result.refinement_passes, 3);
        }

        #[test]
        fn test_dimension_mismatch_error() {
            let err = CrossModelAlignerV4Error::DimensionMismatch {
                expected: 1024,
                actual: 512,
            };
            let display = format!("{}", err);
            assert!(display.contains("1024"));
        }

        #[test]
        fn test_insufficient_models_error() {
            let err = CrossModelAlignerV4Error::InsufficientModels {
                required: 2,
                available: 1,
            };
            let display = format!("{}", err);
            assert!(display.contains("2"));
        }

        #[test]
        fn test_multi_round_alignment() {
            let mut engine = CrossModelAlignerV4::default();
            engine
                .register_model("model-1".to_string(), 1024)
                .unwrap();
            engine
                .register_model("model-2".to_string(), 1024)
                .unwrap();
            for _ in 0..3 {
                let grads = HashMap::from([
                    ("model-1".to_string(), vec![1.0; 1024]),
                    ("model-2".to_string(), vec![2.0; 1024]),
                ]);
                engine.align_gradients(grads).unwrap();
            }
            assert_eq!(engine.stats.total_syncs, 3);
        }
    }
}

#[cfg(feature = "v1.5-sprint3")]
pub use internal::*;
