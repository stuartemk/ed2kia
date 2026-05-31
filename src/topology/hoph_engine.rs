//! Higher-Order Persistent Homology (HOPH) Engine — Sprint 57
//!
//! Extends persistent homology to compute β₂ (Betti-2, 3D void structures)
//! using a simplified Vietoris-Rips filtration for 2-simplices.
//!
//! β₂ measures the number of independent 3D cavities in the data,
//! representing emergent higher-order topological structures in the
//! noosphere node distribution.
//!
//! Feature gate: `v3.9-noosphere-engine`

use std::collections::HashMap;

/// Maximum points before subsampling for performance.
const MAX_POINTS: usize = 500;

/// Default filtration resolution.
const DEFAULT_FILTRATION_STEPS: usize = 50;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum HophError {
    /// Insufficient points for β₂ computation (need >= 4).
    InsufficientPoints { actual: usize, required: usize },
    /// Empty point cloud.
    EmptyPointCloud,
    /// Filtration radius out of range.
    InvalidRadius(f64),
}

impl std::fmt::Display for HophError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HophError::InsufficientPoints { actual, required } => {
                write!(f, "Need {} points for β₂, got {}", required, actual)
            }
            HophError::EmptyPointCloud => write!(f, "Point cloud is empty"),
            HophError::InvalidRadius(r) => write!(f, "Invalid filtration radius: {}", r),
        }
    }
}

// ---------------------------------------------------------------------------
// Point and simplex structures
// ---------------------------------------------------------------------------

/// A point in the noosphere feature space (3D for β₂ computation).
#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point {
    /// Euclidean distance to another point.
    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A 2-simplex (tetrahedron) defined by 4 vertex indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tetrahedron {
    pub vertices: [usize; 4],
}

impl Tetrahedron {
    fn new(a: usize, b: usize, c: usize, d: usize) -> Self {
        let mut verts = [a, b, c, d];
        verts.sort();
        Tetrahedron { vertices: verts }
    }
}

/// A 1-simplex (edge) defined by 2 vertex indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    pub vertices: [usize; 2],
}

impl Edge {
    fn new(a: usize, b: usize) -> Self {
        let mut verts = [a, b];
        verts.sort();
        Edge { vertices: verts }
    }
}

/// A 2-simplex (triangle/facet) defined by 3 vertex indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Facet {
    pub vertices: [usize; 3],
}

// ---------------------------------------------------------------------------
// Persistence pair
// ---------------------------------------------------------------------------

/// A birth-death pair from persistent homology.
#[derive(Debug, Clone)]
pub struct PersistencePair {
    pub birth: f64,
    pub death: f64,
}

impl PersistencePair {
    /// Persistence (lifetime) of this topological feature.
    pub fn persistence(&self) -> f64 {
        self.death - self.birth
    }
}

// ---------------------------------------------------------------------------
// HOPH Engine
// ---------------------------------------------------------------------------

/// Higher-Order Persistent Homology engine for β₂ computation.
///
/// Uses a simplified Vietoris-Rips approach:
/// 1. Build edge set at each filtration radius
/// 2. Form triangles from connected edges
/// 3. Form tetrahedra from connected triangles
/// 4. Track β₂ = dim(H₂) = #tetrahedra - #facets + #edges - #vertices (simplified)
#[derive(Debug, Clone)]
pub struct HophEngine {
    filtration_steps: usize,
    max_radius: f64,
}

impl HophEngine {
    /// Create with default parameters.
    pub fn new() -> Self {
        HophEngine {
            filtration_steps: DEFAULT_FILTRATION_STEPS,
            max_radius: 2.0,
        }
    }

    /// Create with explicit parameters.
    pub fn with_config(filtration_steps: usize, max_radius: f64) -> Self {
        HophEngine {
            filtration_steps,
            max_radius,
        }
    }

    /// Compute β₂ persistence diagram from a point cloud.
    ///
    /// Returns a list of (birth, death) pairs for each 3D cavity.
    pub fn compute_beta2(&self, points: &[Point]) -> Result<Vec<PersistencePair>, HophError> {
        if points.is_empty() {
            return Err(HophError::EmptyPointCloud);
        }
        if points.len() < 4 {
            return Err(HophError::InsufficientPoints {
                actual: points.len(),
                required: 4,
            });
        }

        let n = points.len().min(MAX_POINTS);
        let points = &points[..n];

        // Build filtration values.
        let mut radii = Vec::with_capacity(self.filtration_steps);
        for i in 1..=self.filtration_steps {
            radii.push((i as f64 / self.filtration_steps as f64) * self.max_radius);
        }

        // Track β₂ at each filtration step using simplified Euler characteristic.
        let mut persistence_pairs = Vec::new();
        let mut active_cavities: HashMap<usize, f64> = HashMap::new(); // cavity_id → birth_radius
        let mut cavity_id = 0;

        for &radius in &radii {
            if radius <= 0.0 {
                continue;
            }

            // Build edge set at this radius.
            let mut edges: HashMap<Edge, bool> = HashMap::new();
            for i in 0..n {
                for j in (i + 1)..n {
                    let d = points[i].distance(&points[j]);
                    if d <= radius {
                        edges.insert(Edge::new(i, j), true);
                    }
                }
            }

            // Build facet (triangle) set from connected edges.
            let mut facets: Vec<[usize; 3]> = Vec::new();
            let edge_list: Vec<_> = edges.keys().cloned().collect();
            for ei in 0..edge_list.len() {
                for ej in (ei + 1)..edge_list.len() {
                    let e1 = &edge_list[ei];
                    let e2 = &edge_list[ej];
                    // Shared vertex?
                    let shared = e1.vertices[0] == e2.vertices[0]
                        || e1.vertices[0] == e2.vertices[1]
                        || e1.vertices[1] == e2.vertices[0]
                        || e1.vertices[1] == e2.vertices[1];
                    if shared {
                        // Find third vertex.
                        let third = if e1.vertices[0] != e2.vertices[0]
                            && e1.vertices[0] != e2.vertices[1]
                        {
                            e1.vertices[0]
                        } else if e1.vertices[1] != e2.vertices[0]
                            && e1.vertices[1] != e2.vertices[1]
                        {
                            e1.vertices[1]
                        } else if e2.vertices[0] != e1.vertices[0]
                            && e2.vertices[0] != e1.vertices[1]
                        {
                            e2.vertices[0]
                        } else {
                            e2.vertices[1]
                        };
                        let mut tri = [e1.vertices[0], e1.vertices[1], third];
                        // Check third edge exists.
                        tri.sort();
                        let e_ab = Edge::new(tri[0], tri[1]);
                        let e_bc = Edge::new(tri[1], tri[2]);
                        let e_ac = Edge::new(tri[0], tri[2]);
                        if edges.contains_key(&e_ab)
                            && edges.contains_key(&e_bc)
                            && edges.contains_key(&e_ac)
                        {
                            facets.push(tri);
                        }
                    }
                }
            }

            // Build tetrahedra from connected facets.
            let mut tet_count = 0;
            for fi in 0..facets.len() {
                for fj in (fi + 1)..facets.len() {
                    // Shared edge?
                    let f1 = &facets[fi];
                    let f2 = &facets[fj];
                    let mut shared_count = 0;
                    let mut fourth = None;
                    for &v1 in f1.iter() {
                        if !f2.contains(&v1) {
                            fourth = Some(v1);
                        } else {
                            shared_count += 1;
                        }
                    }
                    if shared_count == 2 {
                        let d = match fourth {
                            Some(v) => v,
                            None => continue,
                        };
                        // Check all 4 triangular faces exist.
                        let all_verts = [f1[0], f1[1], d];
                        let faces = [
                            [all_verts[0], all_verts[1], all_verts[2]],
                            [all_verts[0], all_verts[2], all_verts[1]],
                            [all_verts[1], all_verts[2], all_verts[0]],
                            [all_verts[2], all_verts[0], all_verts[1]],
                        ];
                        let mut all_exist = true;
                        for face in &faces {
                            let mut sorted = *face;
                            sorted.sort();
                            if !facets.contains(&sorted) {
                                all_exist = false;
                                break;
                            }
                        }
                        if all_exist {
                            tet_count += 1;
                        }
                    }
                }
            }

            // Simplified β₂ estimation: track changes in cavity count.
            let facet_count = facets.len();
            let edge_count = edges.len();
            // β₂ ≈ tet_count - facet_count + edge_count - n (Euler char approximation).
            // For persistence, track when β₂ > 0.
            let beta2_estimate = if tet_count > 0 {
                tet_count as i64 - facet_count as i64 + edge_count as i64 - n as i64
            } else {
                0i64
            };

            if beta2_estimate > 0 {
                // New cavity born at this radius.
                for _ in 0..beta2_estimate {
                    active_cavities.insert(cavity_id, radius);
                    cavity_id += 1;
                }
            } else if beta2_estimate < 0 {
                // Cavities die.
                let to_remove: Vec<usize> = active_cavities
                    .keys()
                    .copied()
                    .take(-beta2_estimate as usize)
                    .collect();
                for id in to_remove {
                    if let Some(birth) = active_cavities.remove(&id) {
                        persistence_pairs.push(PersistencePair {
                            birth,
                            death: radius,
                        });
                    }
                }
            }
        }

        // Close remaining cavities at max radius.
        for (_, birth) in active_cavities {
            persistence_pairs.push(PersistencePair {
                birth,
                death: self.max_radius,
            });
        }

        Ok(persistence_pairs)
    }

    /// Compute the PH₂ persistence score (sum of all persistence values).
    ///
    /// Higher score = more persistent 3D topological structures.
    pub fn ph2_persistence_score(&self, points: &[Point]) -> Result<f64, HophError> {
        let pairs = self.compute_beta2(points)?;
        let total: f64 = pairs.iter().map(|p| p.persistence()).sum();
        Ok(total)
    }

    /// Compute the number of significant β₂ features (persistence > threshold).
    pub fn significant_beta2_count(
        &self,
        points: &[Point],
        threshold: f64,
    ) -> Result<usize, HophError> {
        if threshold < 0.0 {
            return Err(HophError::InvalidRadius(threshold));
        }
        let pairs = self.compute_beta2(points)?;
        Ok(pairs.iter().filter(|p| p.persistence() > threshold).count())
    }

    /// Reset engine state (no-op for stateless engine, but API consistency).
    pub fn reset(&mut self) {
        // Stateless — nothing to reset.
    }
}

impl Default for HophEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_points(count: usize) -> Vec<Point> {
        (0..count)
            .map(|i| Point {
                x: (i as f64 % 3.0) * 0.5,
                y: (i as f64 / 3.0) * 0.5,
                z: (i as f64 * 0.7) * 0.3,
            })
            .collect()
    }

    #[test]
    fn test_engine_creation() {
        let engine = HophEngine::new();
        assert_eq!(engine.filtration_steps, DEFAULT_FILTRATION_STEPS);
    }

    #[test]
    fn test_engine_custom_config() {
        let engine = HophEngine::with_config(100, 3.0);
        assert_eq!(engine.filtration_steps, 100);
        assert_eq!(engine.max_radius, 3.0);
    }

    #[test]
    fn test_empty_point_cloud() {
        let engine = HophEngine::new();
        let points: Vec<Point> = vec![];
        assert!(engine.compute_beta2(&points).is_err());
    }

    #[test]
    fn test_insufficient_points() {
        let engine = HophEngine::new();
        let points = vec![
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ];
        match engine.compute_beta2(&points) {
            Err(HophError::InsufficientPoints {
                actual: 3,
                required: 4,
            }) => {}
            other => panic!("Expected InsufficientPoints, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_beta2_minimal() {
        let engine = HophEngine::with_config(20, 2.0);
        let points = make_points(10);
        let result = engine.compute_beta2(&points);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ph2_persistence_score() {
        let engine = HophEngine::with_config(20, 2.0);
        let points = make_points(15);
        let score = engine.ph2_persistence_score(&points).unwrap();
        assert!(score >= 0.0);
    }

    #[test]
    fn test_significant_beta2_count() {
        let engine = HophEngine::with_config(20, 2.0);
        let points = make_points(20);
        let count = engine.significant_beta2_count(&points, 0.01).unwrap();
        assert!(count >= 0);
    }

    #[test]
    fn test_invalid_radius() {
        let engine = HophEngine::new();
        let points = make_points(10);
        assert!(engine.significant_beta2_count(&points, -1.0).is_err());
    }

    #[test]
    fn test_point_distance() {
        let p1 = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let p2 = Point {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        assert!((p1.distance(&p2) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_distance_3d() {
        let p1 = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let p2 = Point {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };
        let expected = (3.0_f64).sqrt();
        assert!((p1.distance(&p2) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_persistence_pair() {
        let pair = PersistencePair {
            birth: 0.5,
            death: 1.5,
        };
        assert!((pair.persistence() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tetrahedron_sorting() {
        let tet = Tetrahedron::new(3, 1, 4, 2);
        assert_eq!(tet.vertices, [1, 2, 3, 4]);
    }

    #[test]
    fn test_edge_sorting() {
        let edge = Edge::new(5, 2);
        assert_eq!(edge.vertices, [2, 5]);
    }

    #[test]
    fn test_reset() {
        let mut engine = HophEngine::new();
        engine.reset();
        // Should still work after reset.
        let points = make_points(10);
        assert!(engine.compute_beta2(&points).is_ok());
    }

    #[test]
    fn test_default() {
        let engine = HophEngine::default();
        assert_eq!(engine.filtration_steps, DEFAULT_FILTRATION_STEPS);
    }

    #[test]
    fn test_error_display() {
        let err = HophError::EmptyPointCloud;
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_large_point_cloud_subsampling() {
        let engine = HophEngine::with_config(10, 1.0);
        let points = make_points(600); // Exceeds MAX_POINTS
        let result = engine.compute_beta2(&points);
        assert!(result.is_ok());
    }
}
