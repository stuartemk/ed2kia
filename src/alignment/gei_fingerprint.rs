//! Geometric Ethical Invariant (GEI) Fingerprint Extraction
//!
//! Extracts a compact topological fingerprint from Persistent Homology results,
//! producing a fixed-length GEI vector that serves as an ethical signature for
//! any point cloud in Stuartian Tensor space.
//!
//! **GEI Vector:** `(b₀, d₀, b₁, d₁, ph0_integral, ph1_integral)`
//! - `b₀`: Dominant PH₀ birth scale (primary ethical concept emergence)
//! - `d₀`: Dominant PH₀ death scale (primary ethical concept stabilization)
//! - `b₁`: Dominant PH₁ birth scale (primary ethical tension emergence)
//! - `d₁`: Dominant PH₁ death scale (primary ethical tension resolution)
//! - `ph0_integral`: Total PH₀ persistence (ethical concept stability)
//! - `ph1_integral`: Total PH₁ persistence (ethical tension complexity)
//!
//! **SAE top-k → SCT Projection:** Converts SAE sparse activations into SCT
//! coordinates by selecting the top-k latent features and projecting them
//! through the Stuartian geometry mapping.
//!
//! **Feature Gate:** `v3.1-gei-topology`
//!
//! **WASM Compatible:** Pure Rust, no C/C++ dependencies.

#[cfg(feature = "v3.1-gei-topology")]
use crate::alignment::sct_core::StuartianTensor;

#[cfg(feature = "v3.1-gei-topology")]
use crate::topology::persistent_homology::{
    EthicalPoint, HomologyConfig, HomologyResult, PersistentHomologyEngine,
};

/// Geometric Ethical Invariant (GEI) — Compact topological fingerprint.
///
/// This struct encapsulates the essential topological features of an ethical
/// point cloud as a fixed-length vector suitable for:
/// - Cross-model ethical comparison
/// - ZKP certification input
/// - Federated aggregation without raw data exposure
/// - Topological stability benchmarks
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeometricEthicalInvariant {
    /// Dominant PH₀ birth scale — when the primary ethical concept emerges.
    pub b0: f64,
    /// Dominant PH₀ death scale — when the primary ethical concept stabilizes.
    pub d0: f64,
    /// Dominant PH₁ birth scale — when the primary ethical tension emerges.
    pub b1: f64,
    /// Dominant PH₁ death scale — when the primary ethical tension resolves.
    pub d1: f64,
    /// Total PH₀ persistence integral — aggregate ethical concept stability.
    pub ph0_integral: f64,
    /// Total PH₁ persistence integral — aggregate ethical tension complexity.
    pub ph1_integral: f64,
    /// Number of persistent PH₀ features (above threshold).
    pub persistent_ph0_count: usize,
    /// Number of persistent PH₁ features (above threshold).
    pub persistent_ph1_count: usize,
    /// Alpha parameter used in ethical distance computation.
    pub alpha: f64,
    /// Persistence threshold used for feature filtering.
    pub persistence_threshold: f64,
}

impl GeometricEthicalInvariant {
    /// Create a zeroed GEI fingerprint (empty point cloud baseline).
    pub fn zero() -> Self {
        Self {
            b0: 0.0,
            d0: 0.0,
            b1: 0.0,
            d1: 0.0,
            ph0_integral: 0.0,
            ph1_integral: 0.0,
            persistent_ph0_count: 0,
            persistent_ph1_count: 0,
            alpha: 2.0,
            persistence_threshold: 0.05,
        }
    }

    /// Extract GEI fingerprint from Persistent Homology results.
    ///
    /// Selects the most persistent (longest lifetime) PH₀ and PH₁ pairs
    /// as the dominant features, and computes aggregate integrals.
    pub fn from_homology(result: &HomologyResult, threshold: f64) -> Self {
        // Find dominant PH₀ pair (longest persistence)
        let dominant_ph0 = result
            .ph0_pairs
            .iter()
            .max_by(|a, b| a.lifetime().partial_cmp(&b.lifetime()).unwrap());

        let (b0, d0) = match dominant_ph0 {
            Some(pair) => (pair.birth, pair.death),
            None => (0.0, 0.0),
        };

        // Find dominant PH₁ pair (longest persistence)
        let dominant_ph1 = result
            .ph1_pairs
            .iter()
            .max_by(|a, b| a.lifetime().partial_cmp(&b.lifetime()).unwrap());

        let (b1, d1) = match dominant_ph1 {
            Some(pair) => (pair.birth, pair.death),
            None => (0.0, 0.0),
        };

        // Compute integrals
        let ph0_integral = result.ph0_integral();
        let ph1_integral = result.ph1_integral();

        // Count persistent features
        let (persistent_ph0_count, persistent_ph1_count) =
            result.persistent_feature_count(threshold);

        Self {
            b0,
            d0,
            b1,
            d1,
            ph0_integral,
            ph1_integral,
            persistent_ph0_count,
            persistent_ph1_count,
            alpha: result.alpha,
            persistence_threshold: threshold,
        }
    }

    /// Compute the GEI vector as a flat array for serialization/ZKP input.
    pub fn to_vector(&self) -> [f64; 6] {
        [
            self.b0,
            self.d0,
            self.b1,
            self.d1,
            self.ph0_integral,
            self.ph1_integral,
        ]
    }

    /// Compute topological stability score.
    ///
    /// Higher values indicate more stable ethical structure:
    /// - High PH₀ persistence (stable concepts)
    /// - Low PH₁ persistence (few unresolved tensions)
    /// - Balanced ratio prevents degenerate cases
    pub fn stability_score(&self) -> f64 {
        let total_persistence = self.ph0_integral + self.ph1_integral;
        if total_persistence < 1e-10 {
            return 0.0;
        }
        // Stability = PH₀ dominance over PH₁ (concepts > tensions)
        let ph0_ratio = self.ph0_integral / total_persistence;
        // Weight by persistent feature count
        let feature_weight = if self.persistent_ph0_count > 0 {
            self.persistent_ph0_count as f64
                / (self.persistent_ph0_count + self.persistent_ph1_count) as f64
        } else {
            0.5
        };
        // Combined score in [0.0, 1.0]
        (ph0_ratio * 0.5 + feature_weight * 0.5).min(1.0)
    }

    /// Compute ethical tension index.
    ///
    /// Higher values indicate more complex ethical dilemmas (persistent loops).
    pub fn tension_index(&self) -> f64 {
        if self.persistent_ph1_count == 0 {
            return 0.0;
        }
        self.ph1_integral / self.persistent_ph1_count as f64
    }

    /// Compute conceptual clarity score.
    ///
    /// Measures how clearly defined the ethical concepts are based on
    /// the spread between birth and death of dominant PH₀ features.
    pub fn conceptual_clarity(&self) -> f64 {
        if self.persistent_ph0_count == 0 {
            return 0.0;
        }
        let dominant_lifetime = self.d0 - self.b0;
        // Normalize: lifetime in [0, 1] maps to clarity in [0, 1]
        // Very short lifetime = noise, very long = stable concept
        (dominant_lifetime / (dominant_lifetime + 1.0)).min(1.0)
    }

    /// Check if this GEI represents a valid ethical structure.
    ///
    /// Valid if:
    /// - At least one persistent PH₀ feature exists
    /// - Dominant PH₀ has positive lifetime
    /// - All values are finite
    pub fn is_valid(&self) -> bool {
        self.persistent_ph0_count > 0
            && self.d0 > self.b0
            && self.b0.is_finite()
            && self.d0.is_finite()
            && self.b1.is_finite()
            && self.d1.is_finite()
            && self.ph0_integral.is_finite()
            && self.ph1_integral.is_finite()
    }
}

impl Default for GeometricEthicalInvariant {
    fn default() -> Self {
        Self::zero()
    }
}

/// Configuration for GEI fingerprint extraction pipeline.
#[derive(Debug, Clone)]
pub struct GEIConfig {
    /// Number of top SAE features to select for SCT projection.
    pub top_k: usize,
    /// Homology computation configuration.
    pub homology_config: HomologyConfig,
    /// Minimum number of points required for valid fingerprint.
    pub min_points: usize,
}

impl Default for GEIConfig {
    fn default() -> Self {
        Self {
            top_k: 32,
            homology_config: HomologyConfig::default(),
            min_points: 10,
        }
    }
}

/// GEI Fingerprint Extraction Engine.
///
/// Orchestrates the full pipeline: SAE top-k selection → SCT projection
/// → Persistent Homology → GEI fingerprint extraction.
#[derive(Debug, Clone)]
pub struct GEIFingerprintEngine {
    config: GEIConfig,
    homology_engine: PersistentHomologyEngine,
}

impl GEIFingerprintEngine {
    /// Create a new engine with default configuration.
    pub fn new() -> Self {
        let config = GEIConfig::default();
        Self {
            homology_engine: PersistentHomologyEngine::with_config(config.homology_config.clone()),
            config,
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: GEIConfig) -> Self {
        Self {
            homology_engine: PersistentHomologyEngine::with_config(config.homology_config.clone()),
            config,
        }
    }

    /// Extract GEI fingerprint directly from ethical points.
    ///
    /// This is the core extraction function that bypasses SAE projection
    /// and works directly with SCT-projected points.
    pub fn extract_from_points(
        &self,
        points: &[EthicalPoint],
    ) -> Option<GeometricEthicalInvariant> {
        if points.len() < self.config.min_points {
            return None;
        }

        let result = self.homology_engine.compute(points);
        Some(GeometricEthicalInvariant::from_homology(
            &result,
            self.config.homology_config.persistence_threshold,
        ))
    }

    /// Extract GEI fingerprint from StuartianTensors.
    ///
    /// Converts SCT coordinates to EthicalPoint and computes fingerprint.
    pub fn extract_from_tensors(
        &self,
        tensors: &[StuartianTensor],
    ) -> Option<GeometricEthicalInvariant> {
        if tensors.len() < self.config.min_points {
            return None;
        }

        let points: Vec<EthicalPoint> = tensors.iter().map(EthicalPoint::from_stuartian).collect();
        self.extract_from_points(&points)
    }

    /// SAE top-k → SCT projection.
    ///
    /// Selects the top-k most active SAE latent features and projects them
    /// into 3D Stuartian Tensor space using the following mapping:
    /// - X (autonomy): Normalized activation magnitude of top-k features
    /// - Y (cost): Computational cost ratio (sparsity penalty)
    /// - Z (ethical trajectory): Signed projection based on feature semantics
    ///
    /// # Arguments
    /// * `activations` - Raw SAE latent activations [batch_size, latent_dim]
    /// * `semantic_signs` - Semantic polarity of each latent feature (-1.0 to 1.0)
    ///
    /// # Returns
    /// Vector of StuartianTensors, one per batch element (top-k aggregated).
    pub fn sae_topk_to_sct(
        &self,
        activations: &[f32],
        semantic_signs: &[f32],
        latent_dim: usize,
    ) -> Vec<StuartianTensor> {
        if activations.is_empty() || semantic_signs.len() < latent_dim {
            return Vec::new();
        }

        // Find top-k indices by activation magnitude
        let mut indexed: Vec<(usize, f32)> = activations
            .iter()
            .take(latent_dim)
            .enumerate()
            .map(|(i, &v)| (i, v))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let top_k_indices: Vec<usize> = indexed
            .iter()
            .take(self.config.top_k)
            .map(|(i, _)| *i)
            .collect();

        // Project top-k features into SCT space
        let mut tensors = Vec::new();
        for &idx in &top_k_indices {
            let activation = activations.get(idx).copied().unwrap_or(0.0);
            let semantic = semantic_signs.get(idx).copied().unwrap_or(0.0);

            // X: Normalized activation (sigmoid-like mapping to [0, 1])
            let x = (activation / (activation + 1.0)).clamp(0.0, 1.0);

            // Y: Cost as inverse of activation efficiency (higher activation = lower cost)
            let y = 1.0 - x;

            // Z: Ethical trajectory from semantic sign, modulated by activation strength
            let z = (semantic * activation.signum() * activation.min(1.0)).clamp(-1.0, 1.0);

            if let Ok(tensor) = StuartianTensor::new(x, y, z) {
                tensors.push(tensor);
            }
        }
        tensors
    }

    /// Full pipeline: SAE activations → GEI fingerprint.
    ///
    /// Combines SAE top-k selection, SCT projection, and persistent homology
    /// into a single extraction call.
    pub fn extract_from_sae(
        &self,
        activations: &[f32],
        semantic_signs: &[f32],
        latent_dim: usize,
    ) -> Option<GeometricEthicalInvariant> {
        let tensors = self.sae_topk_to_sct(activations, semantic_signs, latent_dim);
        self.extract_from_tensors(&tensors)
    }
}

impl Default for GEIFingerprintEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v3.1-gei-topology")]
    use crate::topology::persistent_homology::{EthicalPoint, PersistencePair};

    // ─── GeometricEthicalInvariant Tests ───

    #[test]
    fn test_gei_zero() {
        let gei = GeometricEthicalInvariant::zero();
        assert_eq!(gei.b0, 0.0);
        assert_eq!(gei.d0, 0.0);
        assert_eq!(gei.b1, 0.0);
        assert_eq!(gei.d1, 0.0);
        assert_eq!(gei.ph0_integral, 0.0);
        assert_eq!(gei.ph1_integral, 0.0);
        assert_eq!(gei.persistent_ph0_count, 0);
        assert_eq!(gei.persistent_ph1_count, 0);
    }

    #[test]
    fn test_gei_default() {
        let gei = GeometricEthicalInvariant::default();
        assert_eq!(gei, GeometricEthicalInvariant::zero());
    }

    #[test]
    fn test_gei_to_vector() {
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let vec = gei.to_vector();
        assert_eq!(vec, [0.1, 0.5, 0.2, 0.6, 1.0, 0.5]);
        assert_eq!(vec.len(), 6);
    }

    #[test]
    fn test_gei_stability_score_zero() {
        let gei = GeometricEthicalInvariant::zero();
        assert_eq!(gei.stability_score(), 0.0);
    }

    #[test]
    fn test_gei_stability_score_ph0_dominant() {
        let gei = GeometricEthicalInvariant {
            b0: 0.0,
            d0: 1.0,
            b1: 0.0,
            d1: 0.0,
            ph0_integral: 10.0,
            ph1_integral: 0.0,
            persistent_ph0_count: 5,
            persistent_ph1_count: 0,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let score = gei.stability_score();
        assert!(
            score > 0.5,
            "PH₀ dominant should have high stability: {}",
            score
        );
        assert!(score <= 1.0, "Stability score should be <= 1.0: {}", score);
    }

    #[test]
    fn test_gei_stability_score_ph1_dominant() {
        let gei = GeometricEthicalInvariant {
            b0: 0.0,
            d0: 0.1,
            b1: 0.0,
            d1: 1.0,
            ph0_integral: 0.0,
            ph1_integral: 10.0,
            persistent_ph0_count: 0,
            persistent_ph1_count: 5,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let score = gei.stability_score();
        assert!(
            score < 0.5,
            "PH₁ dominant should have low stability: {}",
            score
        );
    }

    #[test]
    fn test_gei_tension_index_zero() {
        let gei = GeometricEthicalInvariant::zero();
        assert_eq!(gei.tension_index(), 0.0);
    }

    #[test]
    fn test_gei_tension_index_positive() {
        let gei = GeometricEthicalInvariant {
            b0: 0.0,
            d0: 1.0,
            b1: 0.1,
            d1: 0.9,
            ph0_integral: 5.0,
            ph1_integral: 4.0,
            persistent_ph0_count: 5,
            persistent_ph1_count: 2,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let tension = gei.tension_index();
        assert_eq!(tension, 2.0); // 4.0 / 2
    }

    #[test]
    fn test_gei_conceptual_clarity_zero() {
        let gei = GeometricEthicalInvariant::zero();
        assert_eq!(gei.conceptual_clarity(), 0.0);
    }

    #[test]
    fn test_gei_conceptual_clarity_positive() {
        let gei = GeometricEthicalInvariant {
            b0: 0.0,
            d0: 1.0,
            b1: 0.0,
            d1: 0.0,
            ph0_integral: 5.0,
            ph1_integral: 0.0,
            persistent_ph0_count: 3,
            persistent_ph1_count: 0,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let clarity = gei.conceptual_clarity();
        assert!(clarity > 0.0, "Should have positive clarity: {}", clarity);
        assert!(clarity <= 1.0, "Clarity should be <= 1.0: {}", clarity);
    }

    #[test]
    fn test_gei_valid_false_for_zero() {
        let gei = GeometricEthicalInvariant::zero();
        assert!(!gei.is_valid(), "Zero GEI should be invalid");
    }

    #[test]
    fn test_gei_valid_true_for_proper_fingerprint() {
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        assert!(gei.is_valid(), "Proper GEI should be valid");
    }

    #[test]
    fn test_gei_invalid_with_nan() {
        let gei = GeometricEthicalInvariant {
            b0: f64::NAN,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        assert!(!gei.is_valid(), "GEI with NaN should be invalid");
    }

    // ─── GEIConfig Tests ───

    #[test]
    fn test_gei_config_default() {
        let config = GEIConfig::default();
        assert_eq!(config.top_k, 32);
        assert_eq!(config.min_points, 10);
        assert_eq!(config.homology_config.alpha, 2.0);
    }

    // ─── GEIFingerprintEngine Tests ───

    #[test]
    fn test_engine_creation() {
        let engine = GEIFingerprintEngine::new();
        assert_eq!(engine.config.top_k, 32);
    }

    #[test]
    fn test_engine_custom_config() {
        let config = GEIConfig {
            top_k: 16,
            homology_config: HomologyConfig {
                alpha: 3.0,
                max_scale: 1.5,
                persistence_threshold: 0.1,
                max_points: 5000,
            },
            min_points: 5,
        };
        let engine = GEIFingerprintEngine::with_config(config);
        assert_eq!(engine.config.top_k, 16);
        assert_eq!(engine.config.min_points, 5);
    }

    #[cfg(feature = "v3.1-gei-topology")]
    #[test]
    fn test_extract_from_points_insufficient() {
        let engine = GEIFingerprintEngine::new();
        let points = vec![EthicalPoint::new(0.5, 0.5, 0.5)];
        assert!(engine.extract_from_points(&points).is_none());
    }

    #[cfg(feature = "v3.1-gei-topology")]
    #[test]
    fn test_extract_from_points_cluster() {
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 32,
            homology_config: HomologyConfig::default(),
            min_points: 3,
        });

        // Create a tight cluster of ethical points
        let points = vec![
            EthicalPoint::new(0.5, 0.5, 0.8),
            EthicalPoint::new(0.52, 0.48, 0.78),
            EthicalPoint::new(0.48, 0.52, 0.82),
            EthicalPoint::new(0.51, 0.49, 0.79),
            EthicalPoint::new(0.49, 0.51, 0.81),
        ];

        let gei = engine.extract_from_points(&points);
        assert!(gei.is_some(), "Should extract GEI from cluster");
        let fingerprint = gei.unwrap();
        assert!(fingerprint.ph0_integral >= 0.0);
    }

    #[test]
    fn test_sae_topk_to_sct_empty() {
        let engine = GEIFingerprintEngine::new();
        let tensors = engine.sae_topk_to_sct(&[], &[], 0);
        assert!(tensors.is_empty());
    }

    #[test]
    fn test_sae_topk_to_sct_basic() {
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 3,
            homology_config: HomologyConfig::default(),
            min_points: 10,
        });

        let activations = vec![0.8, 0.6, 0.4, 0.2, 0.1];
        let semantic_signs = vec![1.0, -1.0, 0.5, 0.3, -0.5];
        let latent_dim = 5;

        let tensors = engine.sae_topk_to_sct(&activations, &semantic_signs, latent_dim);
        assert_eq!(tensors.len(), 3); // top_k = 3

        // Verify SCT constraints
        for tensor in &tensors {
            assert!(
                tensor.x >= 0.0 && tensor.x <= 1.0,
                "X out of bounds: {}",
                tensor.x
            );
            assert!(
                tensor.y >= 0.0 && tensor.y <= 1.0,
                "Y out of bounds: {}",
                tensor.y
            );
            assert!(
                tensor.z >= -1.0 && tensor.z <= 1.0,
                "Z out of bounds: {}",
                tensor.z
            );
        }
    }

    #[test]
    fn test_sae_topk_selects_highest_activations() {
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 2,
            homology_config: HomologyConfig::default(),
            min_points: 10,
        });

        // Activations: [0.1, 0.9, 0.5, 0.3, 0.7]
        // Top-2 should be indices 1 (0.9) and 4 (0.7)
        let activations = vec![0.1, 0.9, 0.5, 0.3, 0.7];
        let semantic_signs = vec![0.0, 1.0, 0.0, 0.0, 1.0];
        let latent_dim = 5;

        let tensors = engine.sae_topk_to_sct(&activations, &semantic_signs, latent_dim);
        assert_eq!(tensors.len(), 2);

        // First tensor should have highest X (normalized activation ≈ 0.47)
        assert!(
            tensors[0].x > 0.4,
            "Top activation should map to highest X: {}",
            tensors[0].x
        );
    }

    #[test]
    fn test_engine_default() {
        let engine = GEIFingerprintEngine::default();
        assert_eq!(engine.config.top_k, 32);
    }

    // ─── Homology Result Integration Tests ───

    #[cfg(feature = "v3.1-gei-topology")]
    #[test]
    fn test_gei_from_homology_empty() {
        let result = HomologyResult {
            ph0_pairs: vec![],
            ph1_pairs: vec![],
            num_points: 0,
            num_edges: 0,
            alpha: 2.0,
        };
        let gei = GeometricEthicalInvariant::from_homology(&result, 0.05);
        assert_eq!(gei.b0, 0.0);
        assert_eq!(gei.d0, 0.0);
        assert_eq!(gei.persistent_ph0_count, 0);
        assert_eq!(gei.persistent_ph1_count, 0);
    }

    #[cfg(feature = "v3.1-gei-topology")]
    #[test]
    fn test_gei_from_homology_with_pairs() {
        let result = HomologyResult {
            ph0_pairs: vec![
                PersistencePair::new(0.0, 0.5),
                PersistencePair::new(0.1, 0.3),
            ],
            ph1_pairs: vec![PersistencePair::new(0.2, 0.8)],
            num_points: 10,
            num_edges: 45,
            alpha: 2.0,
        };
        let gei = GeometricEthicalInvariant::from_homology(&result, 0.1);

        // Dominant PH₀ should be the one with longest lifetime (0.0, 0.5)
        assert_eq!(gei.b0, 0.0);
        assert_eq!(gei.d0, 0.5);

        // Dominant PH₁ should be (0.2, 0.8)
        assert_eq!(gei.b1, 0.2);
        assert_eq!(gei.d1, 0.8);

        // Integrals (use approximate equality for floating point)
        assert!(
            (gei.ph0_integral - 0.7).abs() < 1e-10,
            "ph0_integral: {}",
            gei.ph0_integral
        );
        assert!(
            (gei.ph1_integral - 0.6).abs() < 1e-10,
            "ph1_integral: {}",
            gei.ph1_integral
        );

        // Persistent features (threshold = 0.1)
        // PH₀: (0.0, 0.5) lifetime=0.5 >= 0.1 ✓, (0.1, 0.3) lifetime=0.2 >= 0.1 ✓
        assert_eq!(gei.persistent_ph0_count, 2);
        // PH₁: (0.2, 0.8) lifetime=0.6 >= 0.1 ✓
        assert_eq!(gei.persistent_ph1_count, 1);
    }
}
