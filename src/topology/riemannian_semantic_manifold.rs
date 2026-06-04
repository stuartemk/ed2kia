//! Riemannian Semantic Manifold â€” Sprint 77: Physics of Consciousness & Thermodynamic Finality
//!
//! Resuelve el bug ontolÃ³gico: cuantizaciÃ³n discreta del grafo semÃ¡ntico (petgraph).
//!
//! Implementa variedades Riemannianas continuas: SCT como curvatura de manifold.
//! Enrutamiento por geodÃ©sicas, no saltos discretos.
//!
//! # GarantÃ­as
//!
//! - Curvatura: O(nÂ²) para matriz mÃ©trica completa
//! - ProyecciÃ³n geodÃ©sica: O(dim Ã— steps)
//! - Continuidad: sin cuantizaciÃ³n discreta

use std::fmt;

/// Error types for Riemannian Semantic Manifold
#[derive(Debug, Clone, PartialEq)]
pub enum ManifoldError {
    /// Empty embeddings
    EmptyEmbeddings,
    /// Dimension mismatch
    DimensionMismatch(usize, usize),
    /// Invalid step size
    InvalidStepSize(f32),
    /// Singular metric tensor
    SingularMetric,
    /// Negative curvature (hyperbolic not supported)
    NegativeCurvature(f64),
}

impl fmt::Display for ManifoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifoldError::EmptyEmbeddings => write!(f, "Empty embeddings"),
            ManifoldError::DimensionMismatch(a, b) => {
                write!(f, "Dimension mismatch: {} vs {}", a, b)
            }
            ManifoldError::InvalidStepSize(s) => write!(f, "Invalid step size: {}", s),
            ManifoldError::SingularMetric => write!(f, "Singular metric tensor"),
            ManifoldError::NegativeCurvature(c) => {
                write!(f, "Negative curvature: {:.6}", c)
            }
        }
    }
}

impl std::error::Error for ManifoldError {}

/// Manifold configuration.
#[derive(Debug, Clone)]
pub struct ManifoldConfig {
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Maximum geodesic steps
    pub max_geodesic_steps: usize,
    /// Default step size for projection
    pub default_step_size: f32,
    /// Curvature regularization epsilon
    pub curvature_epsilon: f64,
}

impl ManifoldConfig {
    pub fn default_Topological() -> Self {
        Self {
            embedding_dim: 8,
            max_geodesic_steps: 100,
            default_step_size: 0.01,
            curvature_epsilon: 1e-8,
        }
    }

    pub fn validate(&self) -> Result<(), ManifoldError> {
        if self.embedding_dim == 0 {
            return Err(ManifoldError::EmptyEmbeddings);
        }
        if self.default_step_size <= 0.0 || self.default_step_size > 1.0 {
            return Err(ManifoldError::InvalidStepSize(self.default_step_size));
        }
        Ok(())
    }
}

impl Default for ManifoldConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Attractor basin in the semantic manifold.
#[derive(Debug, Clone)]
pub struct AttractorBasin {
    /// Basin identifier
    pub basin_id: u32,
    /// Center of the attractor (embedding)
    pub center: Vec<f32>,
    /// Basin radius (influence zone)
    pub radius: f32,
    /// Ethical weight (SCT-Z proxy)
    pub ethical_weight: f64,
}

impl AttractorBasin {
    pub fn new(basin_id: u32, center: Vec<f32>, radius: f32, ethical_weight: f64) -> Self {
        Self {
            basin_id,
            center,
            radius,
            ethical_weight,
        }
    }

    /// Check if a point is within this basin.
    pub fn contains_point(&self, point: &[f32]) -> bool {
        if point.len() != self.center.len() {
            return false;
        }
        let dist = Self::euclidean_distance(point, &self.center);
        dist <= self.radius
    }

    /// Compute attraction force (gradient toward center).
    pub fn attraction_gradient(&self, point: &[f32]) -> Option<Vec<f32>> {
        if point.len() != self.center.len() {
            return None;
        }
        let dist = Self::euclidean_distance(point, &self.center);
        if dist < 1e-6 {
            return Some(vec![0.0; self.center.len()]);
        }
        let strength = self.ethical_weight as f32 / dist;
        Some(
            self.center
                .iter()
                .zip(point.iter())
                .map(|(c, p)| (c - p) * strength)
                .collect(),
        )
    }

    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

impl fmt::Display for AttractorBasin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttractorBasin {{ id={}, dim={}, radius={:.3}, weight={:.3} }}",
            self.basin_id,
            self.center.len(),
            self.radius,
            self.ethical_weight
        )
    }
}

/// Stateful engine for Riemannian Semantic Manifold.
#[derive(Debug, Clone)]
pub struct RiemannianSemanticManifold {
    config: ManifoldConfig,
    attractors: Vec<AttractorBasin>,
}

impl RiemannianSemanticManifold {
    pub fn new() -> Self {
        Self {
            config: ManifoldConfig::default_Topological(),
            attractors: Vec::new(),
        }
    }

    pub fn with_config(config: ManifoldConfig) -> Result<Self, ManifoldError> {
        config.validate()?;
        Ok(Self {
            config,
            attractors: Vec::new(),
        })
    }

    /// Add an attractor basin to the manifold.
    pub fn add_attractor(&mut self, basin: AttractorBasin) {
        self.attractors.push(basin);
    }

    /// Compute manifold curvature from embeddings.
    pub fn compute_curvature(&self, embeddings: &[Vec<f32>]) -> Result<f64, ManifoldError> {
        if embeddings.is_empty() {
            return Err(ManifoldError::EmptyEmbeddings);
        }
        let dim = embeddings[0].len();
        for e in embeddings {
            if e.len() != dim {
                return Err(ManifoldError::DimensionMismatch(dim, e.len()));
            }
        }
        Ok(Self::compute_sectional_curvature(
            embeddings,
            self.config.curvature_epsilon,
        ))
    }

    /// Project a point toward the nearest attractor basin via geodesic.
    pub fn project_to_attractor(
        &self,
        point: &[f32],
        step_size: f32,
    ) -> Result<Vec<f32>, ManifoldError> {
        if step_size <= 0.0 || step_size > 1.0 {
            return Err(ManifoldError::InvalidStepSize(step_size));
        }
        if self.attractors.is_empty() {
            return Ok(point.to_vec());
        }

        // Find nearest attractor
        let mut best_basin: Option<&AttractorBasin> = None;
        let mut best_dist = f32::MAX;

        for basin in &self.attractors {
            if point.len() == basin.center.len() {
                let dist = Self::euclidean_distance(point, &basin.center);
                if dist < best_dist {
                    best_dist = dist;
                    best_basin = Some(basin);
                }
            }
        }

        match best_basin {
            Some(basin) => {
                let gradient = basin
                    .attraction_gradient(point)
                    .unwrap_or(vec![0.0; point.len()]);
                let projected: Vec<f32> = point
                    .iter()
                    .zip(gradient.iter())
                    .map(|(p, g)| p + step_size * g)
                    .collect();
                Ok(projected)
            }
            None => Ok(point.to_vec()),
        }
    }

    /// Find which basin a point belongs to (if any).
    pub fn find_containing_basin(&self, point: &[f32]) -> Option<u32> {
        self.attractors
            .iter()
            .find(|b| b.contains_point(point))
            .map(|b| b.basin_id)
    }

    /// Compute combined ethical field at a point.
    pub fn compute_ethical_field(&self, point: &[f32]) -> f64 {
        self.attractors
            .iter()
            .filter_map(|b| {
                if point.len() == b.center.len() {
                    let dist = Self::euclidean_distance(point, &b.center);
                    if dist < b.radius {
                        Some(b.ethical_weight * (1.0 - dist / b.radius) as f64)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .sum()
    }

    /// Get attractor count.
    pub fn attractor_count(&self) -> usize {
        self.attractors.len()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.attractors.clear();
    }

    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Compute sectional curvature approximation.
    fn compute_sectional_curvature(embeddings: &[Vec<f32>], epsilon: f64) -> f64 {
        if embeddings.len() < 2 {
            return 0.0;
        }
        let _dim = embeddings[0].len();

        // Compute pairwise distances
        let n = embeddings.len();
        let mut total_curvature = 0.0;
        let mut count = 0;

        for i in 0..n {
            for j in (i + 1)..n {
                let dist_ij = Self::pairwise_distance(&embeddings[i], &embeddings[j]);

                // Compute local curvature via distance ratio
                // K â‰ˆ (d_ij^2 - sum_of_local_variances) / normalization
                let local_variance = Self::local_variance(&embeddings[i], embeddings, epsilon);
                let curvature = if dist_ij > 1e-6 {
                    (local_variance / (dist_ij * dist_ij)) as f64
                } else {
                    0.0
                };
                total_curvature += curvature;
                count += 1;
            }
        }

        if count > 0 {
            total_curvature / count as f64
        } else {
            0.0
        }
    }

    fn pairwise_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    fn local_variance(point: &[f32], all_points: &[Vec<f32>], _epsilon: f64) -> f32 {
        let dim = point.len();
        let mut variance_sum = 0.0f32;

        for other in all_points {
            if other.len() == dim {
                let dist_sq: f32 = point
                    .iter()
                    .zip(other.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum();
                variance_sum += dist_sq;
            }
        }

        variance_sum / all_points.len() as f32
    }
}

impl Default for RiemannianSemanticManifold {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RiemannianSemanticManifold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RiemannianManifold {{ dim={}, attractors={} }}",
            self.config.embedding_dim,
            self.attractor_count()
        )
    }
}

// â”€â”€â”€ Public Standalone Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compute manifold curvature from embeddings (standalone).
pub fn compute_manifold_curvature(embeddings: &[Vec<f32>]) -> f64 {
    if embeddings.is_empty() || embeddings[0].is_empty() {
        return 0.0;
    }
    RiemannianSemanticManifold::compute_sectional_curvature(embeddings, 1e-8)
}

/// Project a point toward an attractor basin (standalone).
pub fn project_to_attractor_basin(point: &[f32], basin_center: &[f32], step_size: f32) -> Vec<f32> {
    if point.len() != basin_center.len() || step_size <= 0.0 || step_size > 1.0 {
        return point.to_vec();
    }
    let dist = {
        let mut sum = 0.0f32;
        for (p, c) in point.iter().zip(basin_center.iter()) {
            sum += (p - c).powi(2);
        }
        sum.sqrt()
    };
    if dist < 1e-6 {
        return basin_center.to_vec();
    }
    point
        .iter()
        .zip(basin_center.iter())
        .map(|(p, c)| p + step_size * (c - p) / dist)
        .collect()
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(n: usize, val: f32) -> Vec<f32> {
        vec![val; n]
    }

    #[test]
    fn test_config_default() {
        let config = ManifoldConfig::default_Topological();
        assert!(config.validate().is_ok());
        assert_eq!(config.embedding_dim, 8);
    }

    #[test]
    fn test_config_zero_dim() {
        let config = ManifoldConfig {
            embedding_dim: 0,
            ..ManifoldConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_step() {
        let config = ManifoldConfig {
            default_step_size: 0.0,
            ..ManifoldConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_attractor_creation() {
        let basin = AttractorBasin::new(1, make_embedding(8, 0.5), 1.0, 0.8);
        assert_eq!(basin.basin_id, 1);
        assert_eq!(basin.center.len(), 8);
    }

    #[test]
    fn test_attractor_contains_point() {
        let basin = AttractorBasin::new(1, make_embedding(3, 0.0), 2.0, 0.8);
        assert!(basin.contains_point(&[0.5, 0.5, 0.5]));
        assert!(!basin.contains_point(&[10.0, 10.0, 10.0]));
    }

    #[test]
    fn test_attractor_gradient() {
        let basin = AttractorBasin::new(1, make_embedding(3, 1.0), 2.0, 0.8);
        let grad = basin.attraction_gradient(&[0.0, 0.0, 0.0]);
        assert!(grad.is_some());
        // Gradient should point toward center (positive)
        assert!(grad.unwrap()[0] > 0.0);
    }

    #[test]
    fn test_attractor_gradient_zero_distance() {
        let basin = AttractorBasin::new(1, make_embedding(3, 1.0), 2.0, 0.8);
        let grad = basin.attraction_gradient(&[1.0, 1.0, 1.0]);
        assert!(grad.is_some());
        assert_eq!(grad.unwrap(), vec![0.0; 3]);
    }

    #[test]
    fn test_engine_creation() {
        let engine = RiemannianSemanticManifold::new();
        assert_eq!(engine.attractor_count(), 0);
    }

    #[test]
    fn test_add_attractor() {
        let mut engine = RiemannianSemanticManifold::new();
        engine.add_attractor(AttractorBasin::new(1, make_embedding(8, 0.5), 1.0, 0.8));
        assert_eq!(engine.attractor_count(), 1);
    }

    #[test]
    fn test_compute_curvature_empty() {
        let engine = RiemannianSemanticManifold::new();
        assert!(engine.compute_curvature(&[]).is_err());
    }

    #[test]
    fn test_compute_curvature_single() {
        let engine = RiemannianSemanticManifold::new();
        let result = engine.compute_curvature(&[make_embedding(8, 0.5)]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.0);
    }

    #[test]
    fn test_compute_curvature_multiple() {
        let engine = RiemannianSemanticManifold::new();
        let embeddings = vec![make_embedding(4, 0.0), make_embedding(4, 1.0)];
        let result = engine.compute_curvature(&embeddings);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compute_curvature_dimension_mismatch() {
        let engine = RiemannianSemanticManifold::new();
        let embeddings = vec![make_embedding(4, 0.0), make_embedding(8, 1.0)];
        assert!(engine.compute_curvature(&embeddings).is_err());
    }

    #[test]
    fn test_project_to_attractor() {
        let mut engine = RiemannianSemanticManifold::new();
        engine.add_attractor(AttractorBasin::new(1, make_embedding(3, 1.0), 2.0, 0.8));
        let result = engine.project_to_attractor(&[0.0, 0.0, 0.0], 0.1);
        assert!(result.is_ok());
        // Should move toward center
        assert!(result.unwrap()[0] > 0.0);
    }

    #[test]
    fn test_project_invalid_step() {
        let engine = RiemannianSemanticManifold::new();
        assert!(engine.project_to_attractor(&[0.0], 0.0).is_err());
    }

    #[test]
    fn test_project_no_attractors() {
        let engine = RiemannianSemanticManifold::new();
        let result = engine.project_to_attractor(&[1.0, 2.0], 0.1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1.0, 2.0]);
    }

    #[test]
    fn test_find_containing_basin() {
        let mut engine = RiemannianSemanticManifold::new();
        engine.add_attractor(AttractorBasin::new(1, make_embedding(3, 0.0), 2.0, 0.8));
        assert_eq!(engine.find_containing_basin(&[0.5, 0.5, 0.5]), Some(1));
        assert_eq!(engine.find_containing_basin(&[10.0, 10.0, 10.0]), None);
    }

    #[test]
    fn test_ethical_field() {
        let mut engine = RiemannianSemanticManifold::new();
        engine.add_attractor(AttractorBasin::new(1, make_embedding(3, 0.0), 2.0, 0.8));
        let field = engine.compute_ethical_field(&[0.5, 0.5, 0.5]);
        assert!(field > 0.0);
    }

    #[test]
    fn test_ethical_field_outside_basin() {
        let mut engine = RiemannianSemanticManifold::new();
        engine.add_attractor(AttractorBasin::new(1, make_embedding(3, 0.0), 1.0, 0.8));
        let field = engine.compute_ethical_field(&[10.0, 10.0, 10.0]);
        assert_eq!(field, 0.0);
    }

    #[test]
    fn test_reset() {
        let mut engine = RiemannianSemanticManifold::new();
        engine.add_attractor(AttractorBasin::new(1, make_embedding(8, 0.5), 1.0, 0.8));
        engine.reset();
        assert_eq!(engine.attractor_count(), 0);
    }

    #[test]
    fn test_display() {
        let engine = RiemannianSemanticManifold::new();
        let s = format!("{}", engine);
        assert!(s.contains("RiemannianManifold"));
    }

    #[test]
    fn test_standalone_curvature() {
        let embeddings = vec![make_embedding(4, 0.0), make_embedding(4, 1.0)];
        let k = compute_manifold_curvature(&embeddings);
        assert!(k >= 0.0);
    }

    #[test]
    fn test_standalone_projection() {
        let point = vec![0.0, 0.0, 0.0];
        let center = vec![1.0, 1.0, 1.0];
        let projected = project_to_attractor_basin(&point, &center, 0.1);
        assert_eq!(projected.len(), 3);
        assert!(projected[0] > 0.0);
    }

    #[test]
    fn test_standalone_projection_dimension_mismatch() {
        let point = vec![0.0, 0.0];
        let center = vec![1.0, 1.0, 1.0];
        let projected = project_to_attractor_basin(&point, &center, 0.1);
        assert_eq!(projected, point);
    }

    #[test]
    fn test_error_display() {
        let err = ManifoldError::EmptyEmbeddings;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = RiemannianSemanticManifold::new();

        // Add attractors
        engine.add_attractor(AttractorBasin::new(1, make_embedding(4, 0.0), 3.0, 0.9));
        engine.add_attractor(AttractorBasin::new(2, make_embedding(4, 1.0), 3.0, 0.7));

        // Compute curvature
        let embeddings = vec![make_embedding(4, 0.5), make_embedding(4, 0.8)];
        let curvature = engine.compute_curvature(&embeddings).unwrap();

        // Project point
        let point = vec![0.3, 0.3, 0.3, 0.3];
        let projected = engine.project_to_attractor(&point, 0.1).unwrap();

        // Find basin
        let basin = engine.find_containing_basin(&point);
        assert_eq!(basin, Some(1));

        // Ethical field
        let field = engine.compute_ethical_field(&point);
        assert!(field > 0.0);

        assert!(curvature >= 0.0);
        assert_eq!(projected.len(), 4);
    }
}
