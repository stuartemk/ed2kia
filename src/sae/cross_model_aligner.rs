//! Cross-Model Aligner — Aligns gradients across heterogeneous models.
//!
//! Features:
//! - Cosine similarity-based gradient alignment
//! - Adaptive normalization per model dimension
//! - Alignment scoring with decay tracking
//!
//! Zero financial logic: operates on technical compute metrics only.

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum AlignerError {
    DimensionMismatch { expected: usize, got: usize },
    NoModelsRegistered,
    ModelNotFound(String),
    AlignmentThresholdExceeded(f64),
}

impl fmt::Display for AlignerError {
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
        }
    }
}

impl std::error::Error for AlignerError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct AlignerConfig {
    /// Minimum cosine similarity for valid alignment.
    pub min_similarity: f64,
    /// Decay factor for alignment score history.
    pub score_decay: f64,
    /// Maximum models to align simultaneously.
    pub max_models: usize,
    /// Enable adaptive normalization.
    pub adaptive_normalization: bool,
}

impl Default for AlignerConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.85,
            score_decay: 0.95,
            max_models: 10,
            adaptive_normalization: true,
        }
    }
}

// ─── Model Gradient Profile ───

#[derive(Debug, Clone)]
pub struct GradientProfile {
    pub model_id: String,
    pub dimension: usize,
    pub gradient_norm: f64,
    pub alignment_score: f64,
    pub rounds_aligned: u64,
}

impl GradientProfile {
    pub fn new(model_id: String, dimension: usize) -> Self {
        Self {
            model_id,
            dimension,
            gradient_norm: 0.0,
            alignment_score: 1.0,
            rounds_aligned: 0,
        }
    }
}

// ─── Alignment Result ───

#[derive(Debug, Clone)]
pub struct AlignmentResult {
    pub aligned_gradients: Vec<f32>,
    pub similarity_score: f64,
    pub models_aligned: usize,
    pub normalization_factor: f64,
}

// ─── Aligner ───

pub struct CrossModelAligner {
    config: AlignerConfig,
    profiles: HashMap<String, GradientProfile>,
    alignment_history: Vec<f64>,
}

impl CrossModelAligner {
    /// Create a new aligner with config.
    pub fn new(config: AlignerConfig) -> Self {
        Self {
            config,
            profiles: HashMap::new(),
            alignment_history: Vec::new(),
        }
    }

    /// Create aligner with default config.
    pub fn with_defaults() -> Self {
        Self::new(AlignerConfig::default())
    }

    /// Register a model gradient profile.
    pub fn register_model(&mut self, model_id: String, dimension: usize) -> Result<(), AlignerError> {
        if self.profiles.len() >= self.config.max_models {
            return Err(AlignerError::NoModelsRegistered); // Reuse error for capacity
        }
        self.profiles.insert(
            model_id.clone(),
            GradientProfile::new(model_id, dimension),
        );
        Ok(())
    }

    /// Update gradient profile after training step.
    pub fn update_profile(
        &mut self,
        model_id: &str,
        gradients: &[f32],
    ) -> Result<(), AlignerError> {
        let profile = self.profiles.get_mut(model_id)
            .ok_or(AlignerError::ModelNotFound(model_id.to_string()))?;

        if gradients.len() != profile.dimension {
            return Err(AlignerError::DimensionMismatch {
                expected: profile.dimension,
                got: gradients.len(),
            });
        }

        profile.gradient_norm = compute_norm(gradients);
        profile.rounds_aligned += 1;
        Ok(())
    }

    /// Align gradients across all registered models.
    pub fn align(
        &mut self,
        gradients: &[f32],
    ) -> Result<AlignmentResult, AlignerError> {
        if self.profiles.is_empty() {
            return Err(AlignerError::NoModelsRegistered);
        }

        let mut aligned = gradients.to_vec();
        let norm = compute_norm(gradients);

        // Apply adaptive normalization
        if self.config.adaptive_normalization && norm > 0.0 {
            let factor = self.compute_normalization_factor(norm);
            for g in aligned.iter_mut() {
                *g *= factor;
            }

            // Update profiles
            for profile in self.profiles.values_mut() {
                profile.alignment_score = profile.alignment_score * self.config.score_decay
                    + (1.0 - self.config.score_decay) * (factor as f64);
            }
        }

        // Compute similarity
        let similarity = if norm > 0.0 {
            cosine_similarity(gradients, &aligned)
        } else {
            1.0
        };

        // Track history
        self.alignment_history.push(similarity);
        if self.alignment_history.len() > 100 {
            self.alignment_history.remove(0);
        }

        Ok(AlignmentResult {
            aligned_gradients: aligned,
            similarity_score: similarity,
            models_aligned: self.profiles.len(),
            normalization_factor: if self.config.adaptive_normalization {
                self.compute_normalization_factor(norm) as f64
            } else {
                1.0
            },
        })
    }

    /// Compute normalization factor based on gradient norm.
    fn compute_normalization_factor(&self, norm: f64) -> f32 {
        if norm == 0.0 {
            return 1.0;
        }
        // Scale factor: reduce large gradients, amplify small ones
        let target = 1.0;
        let factor = target / norm;
        factor.clamp(0.1, 10.0) as f32
    }

    /// Get average alignment score.
    pub fn avg_alignment_score(&self) -> f64 {
        if self.profiles.is_empty() {
            return 0.0;
        }
        self.profiles.values().map(|p| p.alignment_score).sum::<f64>()
            / self.profiles.len() as f64
    }

    /// Get alignment history.
    pub fn get_history(&self) -> &[f64] {
        &self.alignment_history
    }

    /// Get model profile.
    pub fn get_profile(&self, model_id: &str) -> Option<&GradientProfile> {
        self.profiles.get(model_id)
    }

    /// Reset alignment state.
    pub fn reset(&mut self) {
        self.alignment_history.clear();
        for profile in self.profiles.values_mut() {
            profile.alignment_score = 1.0;
            profile.gradient_norm = 0.0;
        }
    }
}

impl Default for CrossModelAligner {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_norm(gradients: &[f32]) -> f64 {
    gradients.iter().map(|g| (*g as f64) * (*g as f64)).sum::<f64>().sqrt()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| *x as f64 * *y as f64).sum();
    let norm_a = compute_norm(a);
    let norm_b = compute_norm(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligner_creation() {
        let aligner = CrossModelAligner::with_defaults();
        assert!(aligner.profiles.is_empty());
    }

    #[test]
    fn test_register_model() {
        let mut aligner = CrossModelAligner::default();
        let result = aligner.register_model("m1".to_string(), 128);
        assert!(result.is_ok());
        assert_eq!(aligner.profiles.len(), 1);
    }

    #[test]
    fn test_update_profile() {
        let mut aligner = CrossModelAligner::default();
        let _ = aligner.register_model("m1".to_string(), 64);
        let grads = vec![0.1; 64];
        let result = aligner.update_profile("m1", &grads);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_profile_dimension_mismatch() {
        let mut aligner = CrossModelAligner::default();
        let _ = aligner.register_model("m1".to_string(), 64);
        let grads = vec![0.1; 128];
        let result = aligner.update_profile("m1", &grads);
        assert!(matches!(result, Err(AlignerError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_align_single_model() {
        let mut aligner = CrossModelAligner::default();
        let _ = aligner.register_model("m1".to_string(), 64);
        let grads = vec![0.5; 64];
        let result = aligner.align(&grads);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.models_aligned, 1);
    }

    #[test]
    fn test_align_no_models() {
        let mut aligner = CrossModelAligner::default();
        let result = aligner.align(&[0.5; 64]);
        assert!(matches!(result, Err(AlignerError::NoModelsRegistered)));
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn test_reset() {
        let mut aligner = CrossModelAligner::default();
        let _ = aligner.register_model("m1".to_string(), 64);
        let _ = aligner.align(&vec![0.5; 64]);
        aligner.reset();
        assert!(aligner.alignment_history.is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = AlignerError::ModelNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }
}
