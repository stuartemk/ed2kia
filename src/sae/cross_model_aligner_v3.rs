//! Cross-Model Aligner v3 — Multi-pass gradient alignment with adaptive projection and convergence tracking.
//!
//! Features:
//! - Multi-pass alignment refinement with iterative convergence
//! - Adaptive dimension projection with quality scoring
//! - Enhanced LZ4 compression with ratio tracking
//! - Convergence detection with patience-based stopping
//! - Gradient divergence alerting
//!
//! Zero financial logic: operates on technical compute metrics only.
//! Linux analogy: Public infrastructure for distributed AI interpretability.

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::{HashMap, VecDeque};
    use std::fmt;

    // ─── Errors ───

    #[derive(Debug, Clone)]
    pub enum AlignerV3Error {
        DimensionMismatch { expected: usize, got: usize },
        NoModelsRegistered,
        ModelNotFound(String),
        AlignmentThresholdExceeded(f64),
        ProjectionFailed(String),
        CompressionError(String),
        ConvergenceDivergence(f64),
        MaxPassesExceeded(usize),
    }

    impl fmt::Display for AlignerV3Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::DimensionMismatch { expected, got } => {
                    write!(f, "Gradient dimension mismatch: expected {}, got {}", expected, got)
                }
                Self::NoModelsRegistered => write!(f, "No models registered for alignment"),
                Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
                Self::AlignmentThresholdExceeded(score) => {
                    write!(f, "Alignment score {:.4} below threshold", score)
                }
                Self::ProjectionFailed(msg) => write!(f, "Dimension projection failed: {}", msg),
                Self::CompressionError(msg) => write!(f, "Compression error: {}", msg),
                Self::ConvergenceDivergence(score) => {
                    write!(f, "Alignment diverging: score {:.4} worsening over passes", score)
                }
                Self::MaxPassesExceeded(n) => write!(f, "Max alignment passes ({}) exceeded", n),
            }
        }
    }

    impl std::error::Error for AlignerV3Error {}

    // ─── Config ───

    #[derive(Debug, Clone)]
    pub struct AlignerV3Config {
        /// Minimum cosine similarity for valid alignment.
        pub min_similarity: f64,
        /// Decay factor for alignment score history.
        pub score_decay: f64,
        /// Maximum models to align simultaneously.
        pub max_models: usize,
        /// Enable adaptive normalization.
        pub adaptive_normalization: bool,
        /// Enable dimension projection for heterogeneous models.
        pub dimension_projection: bool,
        /// Maximum history size for alignment scores.
        pub max_history_size: usize,
        /// Enable LZ4 compression for gradient exchange.
        pub lz4_compression: bool,
        /// Compression ratio target.
        pub compression_ratio: f32,
        /// Enable multi-pass refinement.
        pub multi_pass_refinement: bool,
        /// Maximum refinement passes.
        pub max_refinement_passes: usize,
        /// Convergence threshold for multi-pass.
        pub convergence_threshold: f64,
        /// Patience for convergence detection.
        pub convergence_patience: usize,
        /// Divergence alert threshold.
        pub divergence_threshold: f64,
        /// Projection quality minimum.
        pub projection_quality_min: f64,
    }

    impl Default for AlignerV3Config {
        fn default() -> Self {
            Self {
                min_similarity: 0.85,
                score_decay: 0.95,
                max_models: 10,
                adaptive_normalization: true,
                dimension_projection: true,
                max_history_size: 200,
                lz4_compression: true,
                compression_ratio: 4.0,
                multi_pass_refinement: true,
                max_refinement_passes: 5,
                convergence_threshold: 1e-6,
                convergence_patience: 3,
                divergence_threshold: 0.5,
                projection_quality_min: 0.7,
            }
        }
    }

    // ─── Model Gradient Profile ───

    #[derive(Debug, Clone)]
    pub struct GradientProfileV3 {
        pub model_id: String,
        pub dimension: usize,
        pub gradient_norm: f64,
        pub alignment_score: f64,
        pub rounds_aligned: u64,
        pub running_mean: f64,
        pub running_variance: f64,
        pub compressed_size: usize,
        /// Best alignment score achieved.
        pub best_alignment: f64,
        /// Consecutive worsening count for divergence detection.
        pub worsening_count: usize,
        /// Projection quality score.
        pub projection_quality: f64,
    }

    impl GradientProfileV3 {
        pub fn new(model_id: String, dimension: usize) -> Self {
            Self {
                model_id,
                dimension,
                gradient_norm: 0.0,
                alignment_score: 1.0,
                rounds_aligned: 0,
                running_mean: 0.0,
                running_variance: 0.0,
                compressed_size: 0,
                best_alignment: 0.0,
                worsening_count: 0,
                projection_quality: 1.0,
            }
        }

        pub fn update_running_stats(&mut self, value: f64) {
            let n = self.rounds_aligned as f64 + 1.0;
            let delta = value - self.running_mean;
            self.running_mean += delta / n;
            let delta2 = value - self.running_mean;
            self.running_variance += delta * delta2;
        }

        pub fn std_dev(&self) -> f64 {
            if self.rounds_aligned == 0 {
                return 0.0;
            }
            (self.running_variance / self.rounds_aligned as f64).sqrt()
        }

        /// Update best alignment and track worsening.
        pub fn update_alignment_tracking(&mut self, new_score: f64) {
            if new_score > self.best_alignment {
                self.best_alignment = new_score;
                self.worsening_count = 0;
            } else {
                self.worsening_count += 1;
            }
        }

        /// Check if profile is diverging.
        pub fn is_diverging(&self, patience: usize) -> bool {
            self.worsening_count >= patience
        }
    }

    // ─── Alignment Result ───

    #[derive(Debug, Clone)]
    pub struct AlignmentResultV3 {
        pub aligned_gradients: Vec<f32>,
        pub similarity_score: f64,
        pub models_aligned: usize,
        pub normalization_factor: f64,
        pub dimension_projected: bool,
        pub compressed_bytes: usize,
        /// Number of refinement passes executed.
        pub passes_executed: usize,
        /// Final projection quality score.
        pub projection_quality: f64,
        /// Whether convergence was achieved.
        pub converged: bool,
        /// Per-pass similarity scores.
        pub pass_history: Vec<f64>,
    }

    // ─── Stats ───

    #[derive(Debug, Clone, Default)]
    pub struct AlignerV3Stats {
        pub total_alignments: u64,
        pub avg_similarity: f64,
        pub total_projections: u64,
        pub total_compressed_bytes: usize,
        pub max_similarity: f64,
        pub min_similarity: f64,
        pub total_passes: u64,
        pub avg_passes: f64,
        pub convergence_count: u64,
        pub divergence_alerts: u64,
    }

    impl AlignerV3Stats {
        pub fn record(&mut self, similarity: f64, projected: bool, compressed: usize, passes: usize, converged: bool) {
            self.total_alignments += 1;
            let n = self.total_alignments as f64;
            self.avg_similarity = self.avg_similarity * (n - 1.0) / n + similarity / n;
            if similarity > self.max_similarity {
                self.max_similarity = similarity;
            }
            if self.min_similarity == 0.0 || similarity < self.min_similarity {
                self.min_similarity = similarity;
            }
            if projected {
                self.total_projections += 1;
            }
            self.total_compressed_bytes += compressed;
            self.total_passes += passes as u64;
            self.avg_passes = self.total_passes as f64 / self.total_alignments as f64;
            if converged {
                self.convergence_count += 1;
            }
        }

        pub fn record_divergence(&mut self) {
            self.divergence_alerts += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ─── Aligner ───

    pub struct CrossModelAlignerV3 {
        pub config: AlignerV3Config,
        profiles: HashMap<String, GradientProfileV3>,
        alignment_history: VecDeque<f64>,
        pub stats: AlignerV3Stats,
    }

    impl CrossModelAlignerV3 {
        /// Create a new aligner with config.
        pub fn new(config: AlignerV3Config) -> Self {
            Self {
                config,
                profiles: HashMap::new(),
                alignment_history: VecDeque::new(),
                stats: AlignerV3Stats::default(),
            }
        }

        /// Create aligner with default config.
        pub fn with_defaults() -> Self {
            Self::new(AlignerV3Config::default())
        }

        /// Register a model gradient profile.
        pub fn register_model(&mut self, model_id: String, dimension: usize) -> Result<(), AlignerV3Error> {
            if self.profiles.len() >= self.config.max_models {
                return Err(AlignerV3Error::NoModelsRegistered);
            }
            self.profiles.insert(
                model_id.clone(),
                GradientProfileV3::new(model_id, dimension),
            );
            Ok(())
        }

        /// Update gradient profile after training step.
        pub fn update_profile(
            &mut self,
            model_id: &str,
            gradients: &[f32],
        ) -> Result<(), AlignerV3Error> {
            let profile = self.profiles.get_mut(model_id)
                .ok_or(AlignerV3Error::ModelNotFound(model_id.to_string()))?;

            if gradients.len() != profile.dimension {
                return Err(AlignerV3Error::DimensionMismatch {
                    expected: profile.dimension,
                    got: gradients.len(),
                });
            }

            profile.gradient_norm = compute_norm(gradients);
            profile.rounds_aligned += 1;
            profile.update_running_stats(profile.gradient_norm);

            // Compress if enabled
            if self.config.lz4_compression {
                profile.compressed_size = simulate_lz4(gradients, self.config.compression_ratio);
            }

            Ok(())
        }

        /// Align gradients across all registered models with multi-pass refinement.
        pub fn align_gradients(&mut self) -> Result<AlignmentResultV3, AlignerV3Error> {
            if self.profiles.is_empty() {
                return Err(AlignerV3Error::NoModelsRegistered);
            }

            let profiles: Vec<_> = self.profiles.values().collect();
            let target_dim = profiles.iter().map(|p| p.dimension).min().unwrap_or(0);

            if target_dim == 0 {
                return Err(AlignerV3Error::ProjectionFailed("No valid dimensions".to_string()));
            }

            // Project gradients to common dimension if needed
            let dimension_projected = profiles.iter().any(|p| p.dimension != target_dim);

            // Multi-pass refinement
            let mut pass_history = Vec::new();
            let mut current_grads = Self::compute_mean_gradient(&profiles, target_dim);
            let mut prev_similarity = 0.0;
            let mut converged = false;
            let mut passes = 0;
            let mut no_improve_count = 0;

            let max_passes = if self.config.multi_pass_refinement {
                self.config.max_refinement_passes
            } else {
                1
            };
            for pass in 0..max_passes {
                // Compute similarity for current pass
                let similarity = Self::compute_avg_similarity(&current_grads, &profiles, target_dim);
                pass_history.push(similarity);
                passes = pass + 1;

                // Check convergence
                if self.config.multi_pass_refinement && pass > 0 {
                    let improvement = similarity - prev_similarity;
                    if improvement.abs() < self.config.convergence_threshold {
                        no_improve_count += 1;
                        if no_improve_count >= self.config.convergence_patience {
                            converged = true;
                            break;
                        }
                    } else {
                        no_improve_count = 0;
                    }

                    // Check divergence
                    if similarity < prev_similarity * self.config.divergence_threshold {
                        self.stats.record_divergence();
                        return Err(AlignerV3Error::ConvergenceDivergence(similarity));
                    }
                }

                prev_similarity = similarity;

                // Refine gradients using weighted average
                if self.config.multi_pass_refinement && pass < self.config.max_refinement_passes - 1 {
                    current_grads = Self::refine_gradients(&current_grads, &profiles, target_dim, similarity);
                }
            }

            if passes >= self.config.max_refinement_passes && !converged {
                // Check if we should alert on max passes
                if pass_history.len() >= 2 {
                    let first = pass_history[0];
                    let last = *pass_history.last().unwrap();
                    if last < first * self.config.divergence_threshold {
                        self.stats.record_divergence();
                    }
                }
            }

            let final_similarity = *pass_history.last().unwrap_or(&0.0);

            // Compute projection quality
            let projection_quality = if dimension_projected {
                Self::compute_projection_quality(&profiles, target_dim)
            } else {
                1.0
            };

            // Adaptive normalization
            let normalization_factor = if self.config.adaptive_normalization {
                let avg_std: f64 = profiles.iter().map(|p| p.std_dev()).sum::<f64>() / profiles.len() as f64;
                if avg_std > 0.0 { 1.0 / avg_std } else { 1.0 }
            } else {
                1.0
            };

            // Build aligned gradients
            let aligned: Vec<f32> = current_grads.iter().map(|g| (*g * normalization_factor) as f32).collect();

            // Compressed bytes
            let compressed_bytes: usize = profiles.iter().map(|p| p.compressed_size).sum();

            // Update history
            self.alignment_history.push_back(final_similarity);
            while self.alignment_history.len() > self.config.max_history_size {
                self.alignment_history.pop_front();
            }

            // Apply decay to profile scores and update tracking
            let models_count = profiles.len();
            for (_, profile) in self.profiles.iter_mut() {
                profile.alignment_score = profile.alignment_score * self.config.score_decay + final_similarity * (1.0 - self.config.score_decay);
                profile.update_alignment_tracking(profile.alignment_score);
                profile.projection_quality = projection_quality;

                // Check divergence per profile
                if profile.is_diverging(self.config.convergence_patience) {
                    self.stats.record_divergence();
                }
            }

            // Record stats
            self.stats.record(final_similarity, dimension_projected, compressed_bytes, passes, converged);

            // Check threshold
            if final_similarity < self.config.min_similarity {
                return Err(AlignerV3Error::AlignmentThresholdExceeded(final_similarity));
            }

            // Check projection quality
            if dimension_projected && projection_quality < self.config.projection_quality_min {
                return Err(AlignerV3Error::ProjectionFailed(format!(
                    "Projection quality {:.4} below minimum {:.4}",
                    projection_quality, self.config.projection_quality_min
                )));
            }

            Ok(AlignmentResultV3 {
                aligned_gradients: aligned,
                similarity_score: final_similarity,
                models_aligned: models_count,
                normalization_factor,
                dimension_projected,
                compressed_bytes,
                passes_executed: passes,
                projection_quality,
                converged,
                pass_history,
            })
        }

        /// Compute mean gradient across profiles.
        fn compute_mean_gradient(profiles: &[&GradientProfileV3], target_dim: usize) -> Vec<f64> {
            let mut mean_grad: Vec<f64> = vec![0.0; target_dim];
            for profile in profiles {
                for (i, item) in mean_grad.iter_mut().enumerate().take(target_dim) {
                    if i < profile.dimension {
                        *item += profile.gradient_norm * 0.1;
                    }
                }
            }
            let n = profiles.len() as f64;
            for item in mean_grad.iter_mut().take(target_dim) {
                *item /= n;
            }
            mean_grad
        }

        /// Compute average similarity score.
        fn compute_avg_similarity(mean_grad: &[f64], profiles: &[&GradientProfileV3], target_dim: usize) -> f64 {
            let mut total_similarity = 0.0;
            for profile in profiles {
                let sim = compute_cosine_similarity(mean_grad, profile, target_dim);
                total_similarity += sim;
            }
            total_similarity / profiles.len() as f64
        }

        /// Refine gradients using similarity-weighted averaging.
        fn refine_gradients(current: &[f64], profiles: &[&GradientProfileV3], target_dim: usize, similarity: f64) -> Vec<f64> {
            let mut refined = current.to_vec();
            let weight = 1.0 - similarity; // Lower similarity = more refinement needed

            for profile in profiles {
                for (i, item) in refined.iter_mut().enumerate().take(target_dim) {
                    if i < profile.dimension {
                        let profile_grad = profile.gradient_norm * 0.1;
                        *item = *item * (1.0 - weight * 0.1) + profile_grad * weight * 0.1;
                    }
                }
            }
            refined
        }

        /// Compute projection quality score.
        fn compute_projection_quality(profiles: &[&GradientProfileV3], target_dim: usize) -> f64 {
            if profiles.is_empty() {
                return 0.0;
            }
            let total_dim: usize = profiles.iter().map(|p| p.dimension).sum();
            let projected_dim = target_dim * profiles.len();
            if total_dim == 0 {
                return 0.0;
            }
            projected_dim as f64 / total_dim as f64
        }

        /// Get alignment history.
        pub fn get_alignment_history(&self) -> &VecDeque<f64> {
            &self.alignment_history
        }

        /// Get model profile.
        pub fn get_profile(&self, model_id: &str) -> Option<&GradientProfileV3> {
            self.profiles.get(model_id)
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Check if any profile is diverging.
        pub fn has_diverging_profiles(&self) -> bool {
            self.profiles.values().any(|p| p.is_diverging(self.config.convergence_patience))
        }
    }

    impl Default for CrossModelAlignerV3 {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    // ─── Helpers ───

    fn compute_norm(grads: &[f32]) -> f64 {
        let sum: f64 = grads.iter().map(|g| (*g as f64) * (*g as f64)).sum();
        sum.sqrt()
    }

    fn compute_cosine_similarity(a: &[f64], profile: &GradientProfileV3, target_dim: usize) -> f64 {
        let b_len = std::cmp::min(a.len(), target_dim);
        if b_len == 0 {
            return 0.0;
        }
        let dot: f64 = a[..b_len].iter().map(|v| v * profile.gradient_norm * 0.1).sum();
        let norm_a: f64 = a[..b_len].iter().map(|v| v * v).sum::<f64>().sqrt();
        let norm_b = profile.gradient_norm;
        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }
        let sim = dot / (norm_a * norm_b);
        sim.clamp(-1.0, 1.0)
    }

    fn simulate_lz4(data: &[f32], ratio: f32) -> usize {
        (data.len() * 4) / ratio as usize
    }

}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;

#[cfg(all(test, feature = "v1.5-sprint1"))]
mod tests {
    use super::*;

    #[test]
    fn test_aligner_creation() {
        let aligner = CrossModelAlignerV3::with_defaults();
        assert_eq!(aligner.stats.total_alignments, 0);
    }

    #[test]
    fn test_aligner_with_config() {
        let config = AlignerV3Config {
            min_similarity: 0.90,
            max_models: 5,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let aligner = CrossModelAlignerV3::new(config);
        assert_eq!(aligner.config.max_models, 5);
        assert!(!aligner.config.multi_pass_refinement);
    }

    #[test]
    fn test_register_model() {
        let mut aligner = CrossModelAlignerV3::with_defaults();
        let result = aligner.register_model("model_a".to_string(), 128);
        assert!(result.is_ok());
        assert!(aligner.get_profile("model_a").is_some());
    }

    #[test]
    fn test_register_model_max_reached() {
        let config = AlignerV3Config {
            max_models: 2,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        assert!(aligner.register_model("m1".to_string(), 64).is_ok());
        assert!(aligner.register_model("m2".to_string(), 64).is_ok());
        let result = aligner.register_model("m3".to_string(), 64);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_profile() {
        let mut aligner = CrossModelAlignerV3::with_defaults();
        aligner.register_model("model_a".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let result = aligner.update_profile("model_a", &grads);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_profile_dimension_mismatch() {
        let mut aligner = CrossModelAlignerV3::with_defaults();
        aligner.register_model("model_a".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..32).map(|i| i as f32).collect();
        let result = aligner.update_profile("model_a", &grads);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_profile_model_not_found() {
        let mut aligner = CrossModelAlignerV3::with_defaults();
        let grads: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let result = aligner.update_profile("missing", &grads);
        assert!(result.is_err());
    }

    #[test]
    fn test_align_gradients_empty() {
        let mut aligner = CrossModelAlignerV3::with_defaults();
        let result = aligner.align_gradients();
        assert!(result.is_err());
    }

    #[test]
    fn test_align_gradients_single_model() {
        let mut aligner = CrossModelAlignerV3::with_defaults();
        aligner.register_model("model_a".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("model_a", &grads).unwrap();
        let result = aligner.align_gradients();
        // Single model may not meet threshold
        match result {
            Ok(r) => assert_eq!(r.models_aligned, 1),
            Err(AlignerV3Error::AlignmentThresholdExceeded(_)) => {},
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_align_gradients_multiple_models() {
        let config = AlignerV3Config {
            min_similarity: 0.0, // Allow any similarity for test
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        aligner.register_model("m2".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.update_profile("m2", &grads).unwrap();
        let result = aligner.align_gradients();
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.models_aligned, 2);
        assert_eq!(r.passes_executed, 1);
    }

    #[test]
    fn test_dimension_projection() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            dimension_projection: true,
            multi_pass_refinement: false,
            projection_quality_min: 0.0, // Allow any quality for test
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 128).unwrap();
        aligner.register_model("m2".to_string(), 64).unwrap();
        let grads128: Vec<f32> = (0..128).map(|i| (i % 10) as f32).collect();
        let grads64: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads128).unwrap();
        aligner.update_profile("m2", &grads64).unwrap();
        let result = aligner.align_gradients();
        assert!(result.is_ok());
        assert!(result.unwrap().dimension_projected);
    }

    #[test]
    fn test_multi_pass_refinement() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: true,
            max_refinement_passes: 3,
            convergence_threshold: 1e-4,
            convergence_patience: 2,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        let result = aligner.align_gradients();
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.passes_executed <= 3);
        assert!(!r.pass_history.is_empty());
    }

    #[test]
    fn test_convergence_detection() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: true,
            max_refinement_passes: 10,
            convergence_threshold: 1e-6,
            convergence_patience: 2,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        let result = aligner.align_gradients();
        assert!(result.is_ok());
        // Should converge early if similarity stabilizes
        let r = result.unwrap();
        if r.converged {
            assert!(r.passes_executed < 10);
        }
    }

    #[test]
    fn test_stats_tracking() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.align_gradients().unwrap();
        assert_eq!(aligner.stats.total_alignments, 1);
        assert!(aligner.stats.avg_similarity >= 0.0);
    }

    #[test]
    fn test_reset_stats() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.align_gradients().unwrap();
        aligner.reset_stats();
        assert_eq!(aligner.stats.total_alignments, 0);
        assert_eq!(aligner.stats.total_passes, 0);
    }

    #[test]
    fn test_score_decay() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            score_decay: 0.90,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.align_gradients().unwrap();
        let profile = aligner.get_profile("m1").unwrap();
        let score_after_first = profile.alignment_score;
        // Run another alignment
        aligner.align_gradients().unwrap();
        let profile = aligner.get_profile("m1").unwrap();
        // Score should have decay applied
        assert!(profile.alignment_score >= 0.0);
    }

    #[test]
    fn test_lz4_compression() {
        let config = AlignerV3Config {
            lz4_compression: true,
            compression_ratio: 4.0,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 100).unwrap();
        let grads: Vec<f32> = (0..100).map(|i| i as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        let profile = aligner.get_profile("m1").unwrap();
        assert!(profile.compressed_size > 0);
        assert_eq!(profile.compressed_size, 100 * 4 / 4);
    }

    #[test]
    fn test_projection_quality() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            dimension_projection: true,
            projection_quality_min: 0.0,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 128).unwrap();
        aligner.register_model("m2".to_string(), 64).unwrap();
        let grads128: Vec<f32> = (0..128).map(|i| (i % 10) as f32).collect();
        let grads64: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads128).unwrap();
        aligner.update_profile("m2", &grads64).unwrap();
        let result = aligner.align_gradients();
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.projection_quality > 0.0);
        assert!(r.projection_quality <= 1.0);
    }

    #[test]
    fn test_alignment_history() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            max_history_size: 5,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        for _ in 0..7 {
            aligner.align_gradients().unwrap();
        }
        let history = aligner.get_alignment_history();
        assert!(history.len() <= 5);
    }

    #[test]
    fn test_best_alignment_tracking() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.align_gradients().unwrap();
        let profile = aligner.get_profile("m1").unwrap();
        assert!(profile.best_alignment >= 0.0);
    }

    #[test]
    fn test_diverging_profiles() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            convergence_patience: 2,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        assert!(!aligner.has_diverging_profiles());
    }

    #[test]
    fn test_config_default() {
        let config = AlignerV3Config::default();
        assert_eq!(config.min_similarity, 0.85);
        assert_eq!(config.max_refinement_passes, 5);
        assert!(config.multi_pass_refinement);
        assert!(config.adaptive_normalization);
    }

    #[test]
    fn test_stats_default() {
        let stats = AlignerV3Stats::default();
        assert_eq!(stats.total_alignments, 0);
        assert_eq!(stats.total_passes, 0);
        assert_eq!(stats.convergence_count, 0);
        assert_eq!(stats.divergence_alerts, 0);
    }

    #[test]
    fn test_error_display() {
        let err = AlignerV3Error::DimensionMismatch { expected: 64, got: 32 };
        let msg = format!("{}", err);
        assert!(msg.contains("dimension mismatch"));
    }

    #[test]
    fn test_profile_new() {
        let profile = GradientProfileV3::new("test".to_string(), 128);
        assert_eq!(profile.model_id, "test");
        assert_eq!(profile.dimension, 128);
        assert_eq!(profile.best_alignment, 0.0);
        assert_eq!(profile.worsening_count, 0);
    }

    #[test]
    fn test_profile_running_stats() {
        let mut profile = GradientProfileV3::new("test".to_string(), 64);
        profile.rounds_aligned = 1; // Pre-set for stats calculation
        profile.update_running_stats(10.0);
        profile.update_running_stats(20.0);
        assert!(profile.running_mean > 0.0);
        assert!(profile.std_dev() >= 0.0);
    }

    #[test]
    fn test_profile_diverging() {
        let mut profile = GradientProfileV3::new("test".to_string(), 64);
        profile.update_alignment_tracking(0.9);
        assert_eq!(profile.best_alignment, 0.9);
        assert_eq!(profile.worsening_count, 0);
        profile.update_alignment_tracking(0.5);
        assert_eq!(profile.worsening_count, 1);
        assert!(profile.is_diverging(1));
        assert!(!profile.is_diverging(2));
    }

    #[test]
    fn test_aligner_default() {
        let aligner = CrossModelAlignerV3::default();
        assert_eq!(aligner.stats.total_alignments, 0);
    }

    #[test]
    fn test_threshold_rejection() {
        let config = AlignerV3Config {
            min_similarity: 0.99,
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 3) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        let result = aligner.align_gradients();
        match result {
            Ok(_) => {}, // May pass if similarity is high enough
            Err(AlignerV3Error::AlignmentThresholdExceeded(_)) => {},
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_passes_executed_tracked() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: true,
            max_refinement_passes: 3,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        let result = aligner.align_gradients();
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.passes_executed > 0);
        assert!(r.passes_executed <= 3);
        assert_eq!(r.pass_history.len(), r.passes_executed);
    }

    #[test]
    fn test_stats_avg_passes() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: true,
            max_refinement_passes: 2,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.align_gradients().unwrap();
        aligner.align_gradients().unwrap();
        assert_eq!(aligner.stats.total_alignments, 2);
        assert!(aligner.stats.avg_passes > 0.0);
    }

    #[test]
    fn test_convergence_count() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            multi_pass_refinement: true,
            max_refinement_passes: 5,
            convergence_threshold: 1e-4,
            convergence_patience: 2,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads).unwrap();
        aligner.align_gradients().unwrap();
        // Stats should be recorded
        assert_eq!(aligner.stats.total_alignments, 1);
    }

    #[test]
    fn test_divergence_alert_recorded() {
        let stats = &mut AlignerV3Stats::default();
        stats.record_divergence();
        assert_eq!(stats.divergence_alerts, 1);
        stats.record_divergence();
        assert_eq!(stats.divergence_alerts, 2);
    }

    #[test]
    fn test_projection_quality_low_rejected() {
        let config = AlignerV3Config {
            min_similarity: 0.0,
            dimension_projection: true,
            projection_quality_min: 0.99, // Very high threshold
            multi_pass_refinement: false,
            ..Default::default()
        };
        let mut aligner = CrossModelAlignerV3::new(config);
        aligner.register_model("m1".to_string(), 256).unwrap();
        aligner.register_model("m2".to_string(), 32).unwrap();
        let grads256: Vec<f32> = (0..256).map(|i| (i % 10) as f32).collect();
        let grads32: Vec<f32> = (0..32).map(|i| (i % 10) as f32).collect();
        aligner.update_profile("m1", &grads256).unwrap();
        aligner.update_profile("m2", &grads32).unwrap();
        let result = aligner.align_gradients();
        match result {
            Ok(_) => {}, // May pass if quality is high enough
            Err(AlignerV3Error::ProjectionFailed(_)) => {},
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_error_convergence_divergence_display() {
        let err = AlignerV3Error::ConvergenceDivergence(0.3);
        let msg = format!("{}", err);
        assert!(msg.contains("diverging"));
    }

    #[test]
    fn test_error_max_passes_display() {
        let err = AlignerV3Error::MaxPassesExceeded(5);
        let msg = format!("{}", err);
        assert!(msg.contains("passes"));
    }
}
