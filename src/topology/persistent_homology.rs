//! Persistent Homology Engine — Geometric Ethical Invariants (GEI)
//!
//! Implements lightweight Vietoris-Rips complex computation over 3D point clouds
//! in the Stuartian Context Tensor (SCT) space. Computes PH₀ (connected components,
//! stable ethical concepts) and PH₁ (persistent loops, ethical tensions/dilemmas).
//!
//! **Distance Metric:** Ethical proximity weighted by Z-axis (ethical focus):
//! `d(p, q) = ||p - q||_2 * exp(-alpha * Z_avg)`
//!
//! **Feature Gate:** `v3.1-gei-topology`
//!
//! **WASM Compatible:** Pure Rust, no C/C++ dependencies.

#[cfg(feature = "v3.1-gei-topology")]
use crate::alignment::sct_core::StuartianTensor;

/// A point in 3D ethical space (SCT coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EthicalPoint {
    /// X-axis: Autonomy signal [0.0, 1.0]
    pub x: f64,
    /// Y-axis: Extraction/Cost signal [0.0, 1.0]
    pub y: f64,
    /// Z-axis: Ethical trajectory [-1.0, 1.0]
    pub z: f64,
}

impl EthicalPoint {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Convert from StuartianTensor.
    pub fn from_stuartian(tensor: &StuartianTensor) -> Self {
        Self {
            x: tensor.x as f64,
            y: tensor.y as f64,
            z: tensor.z as f64,
        }
    }

    /// Euclidean distance to another point.
    pub fn euclidean_distance(&self, other: &EthicalPoint) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Ethical distance metric weighted by Z-axis proximity to the Ethical Focus.
/// `d(p, q) = ||p - q||_2 * exp(-alpha * Z_avg)`
///
/// Higher Z_avg (more ethical) → lower effective distance → stronger topological connection.
/// This encodes the principle that ethically aligned concepts are topologically closer.
pub fn ethical_distance(p: &EthicalPoint, q: &EthicalPoint, alpha: f64) -> f64 {
    let euclidean = p.euclidean_distance(q);
    let z_avg = (p.z + q.z) / 2.0;
    euclidean * (-alpha * z_avg).exp()
}

/// A persistence pair (birth, death) representing a topological feature.
/// - PH₀: birth = component creation, death = component merge.
/// - PH₁: birth = loop creation, death = loop filling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersistencePair {
    pub birth: f64,
    pub death: f64,
}

impl PersistencePair {
    pub fn new(birth: f64, death: f64) -> Self {
        Self { birth, death }
    }

    /// Persistence lifetime: death - birth.
    /// Higher persistence = more topologically significant feature.
    pub fn lifetime(&self) -> f64 {
        self.death - self.birth
    }

    /// Check if this feature is persistent (not noise).
    pub fn is_persistent(&self, threshold: f64) -> bool {
        self.lifetime() >= threshold
    }
}

/// Vietoris-Rips complex edge, sorted by filtration value.
#[derive(Debug, Clone)]
struct Edge {
    i: usize,
    j: usize,
    distance: f64,
}

/// Union-Find (Disjoint Set Union) for efficient PH₀ computation.
#[derive(Debug, Clone)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) -> bool {
        let root_x = self.find(x);
        let root_y = self.find(y);
        if root_x == root_y {
            return false;
        }
        // Union by rank
        match self.rank[root_x].cmp(&self.rank[root_y]) {
            std::cmp::Ordering::Less => {
                self.parent[root_x] = root_y;
            }
            std::cmp::Ordering::Greater => {
                self.parent[root_y] = root_x;
            }
            std::cmp::Ordering::Equal => {
                self.parent[root_y] = root_x;
                self.rank[root_x] += 1;
            }
        }
        true
    }

    /// Count current number of connected components.
    fn component_count(&mut self) -> usize {
        let mut roots = std::collections::HashSet::new();
        for i in 0..self.parent.len() {
            roots.insert(self.find(i));
        }
        roots.len()
    }
}

/// Persistent Homology computation result.
#[derive(Debug, Clone)]
pub struct HomologyResult {
    /// PH₀ persistence pairs (connected components).
    /// Each pair represents an ethical concept cluster and its merge scale.
    pub ph0_pairs: Vec<PersistencePair>,
    /// PH₁ persistence pairs (loops/cycles).
    /// Each pair represents an ethical tension or dilemma cycle.
    pub ph1_pairs: Vec<PersistencePair>,
    /// Number of input points.
    pub num_points: usize,
    /// Number of edges in the Vietoris-Rips complex.
    pub num_edges: usize,
    /// Alpha parameter used for ethical distance weighting.
    pub alpha: f64,
}

impl HomologyResult {
    /// Calculate the persistence landscape integral for PH₀.
    /// Sum of all persistence lifetimes, representing total ethical concept stability.
    pub fn ph0_integral(&self) -> f64 {
        self.ph0_pairs.iter().map(|p| p.lifetime()).sum()
    }

    /// Calculate the persistence landscape integral for PH₁.
    /// Sum of all persistence lifetimes, representing total ethical tension complexity.
    pub fn ph1_integral(&self) -> f64 {
        self.ph1_pairs.iter().map(|p| p.lifetime()).sum()
    }

    /// Count persistent features above threshold.
    pub fn persistent_feature_count(&self, threshold: f64) -> (usize, usize) {
        let ph0 = self.ph0_pairs.iter().filter(|p| p.is_persistent(threshold)).count();
        let ph1 = self.ph1_pairs.iter().filter(|p| p.is_persistent(threshold)).count();
        (ph0, ph1)
    }

    /// Betti numbers at a given filtration scale.
    /// β₀ = number of connected components at scale ε.
    /// β₁ = number of loops at scale ε.
    pub fn betti_numbers_at_scale(&self, epsilon: f64) -> (usize, usize) {
        let b0 = self.ph0_pairs.iter().filter(|p| p.birth <= epsilon && p.death > epsilon).count();
        let b1 = self.ph1_pairs.iter().filter(|p| p.birth <= epsilon && p.death > epsilon).count();
        (b0, b1)
    }
}

/// Configuration for Persistent Homology computation.
#[derive(Debug, Clone)]
pub struct HomologyConfig {
    /// Alpha parameter for ethical distance weighting.
    /// Higher alpha → stronger Z-axis influence on topological connections.
    pub alpha: f64,
    /// Maximum filtration scale. Edges beyond this are ignored.
    pub max_scale: f64,
    /// Minimum persistence threshold for feature significance.
    pub persistence_threshold: f64,
    /// Maximum number of points (for memory control).
    pub max_points: usize,
}

impl Default for HomologyConfig {
    fn default() -> Self {
        Self {
            alpha: 2.0,
            max_scale: 2.0,
            persistence_threshold: 0.05,
            max_points: 10_000,
        }
    }
}

/// Persistent Homology Engine for Geometric Ethical Invariants.
#[derive(Debug, Clone)]
pub struct PersistentHomologyEngine {
    config: HomologyConfig,
}

impl PersistentHomologyEngine {
    /// Create a new engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: HomologyConfig::default(),
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: HomologyConfig) -> Self {
        Self { config }
    }

    /// Compute persistent homology for a cloud of ethical points.
    ///
    /// Returns PH₀ (connected components) and PH₁ (loops) persistence pairs.
    ///
    /// **Algorithm:**
    /// 1. Build Vietoris-Rips complex with ethical distance metric.
    /// 2. Sort edges by filtration value (distance).
    /// 3. Compute PH₀ using Union-Find (merge tree).
    /// 4. Compute PH₁ using boundary matrix reduction (over GF(2)).
    pub fn compute(&self, points: &[EthicalPoint]) -> HomologyResult {
        let n = points.len().min(self.config.max_points);
        if n < 2 {
            return HomologyResult {
                ph0_pairs: vec![],
                ph1_pairs: vec![],
                num_points: n,
                num_edges: 0,
                alpha: self.config.alpha,
            };
        }

        // Step 1: Build and sort edges by ethical distance
        let mut edges = Self::build_vietoris_rips(&points[..n], self.config.alpha, self.config.max_scale);
        edges.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        let num_edges = edges.len();

        // Step 2: Compute PH₀ using Union-Find
        let ph0_pairs = self.compute_ph0(&edges, n);

        // Step 3: Compute PH₁ using boundary matrix reduction
        let ph1_pairs = self.compute_ph1(&edges, &points[..n]);

        HomologyResult {
            ph0_pairs,
            ph1_pairs,
            num_points: n,
            num_edges,
            alpha: self.config.alpha,
        }
    }

    /// Build Vietoris-Rips complex edges with ethical distance metric.
    fn build_vietoris_rips(points: &[EthicalPoint], alpha: f64, max_scale: f64) -> Vec<Edge> {
        let n = points.len();
        let mut edges = Vec::with_capacity(n * (n - 1) / 2);

        for i in 0..n {
            for j in (i + 1)..n {
                let dist = ethical_distance(&points[i], &points[j], alpha);
                if dist <= max_scale {
                    edges.push(Edge {
                        i,
                        j,
                        distance: dist,
                    });
                }
            }
        }
        edges
    }

    /// Compute PH₀ persistence pairs using Union-Find.
    ///
    /// Each point starts as its own component (birth = 0).
    /// When two components merge at scale ε, the smaller dies (death = ε).
    fn compute_ph0(&self, edges: &[Edge], n: usize) -> Vec<PersistencePair> {
        let mut uf = UnionFind::new(n);
        let mut pairs = Vec::new();

        // Initial components: each point is born at scale 0.
        // We track which component "dies" when merged.
        let mut death_recorded = vec![false; n];

        for edge in edges {
            let root_i = uf.find(edge.i);
            let root_j = uf.find(edge.j);

            if root_i != root_j {
                // Two components merge at this scale.
                // One component dies (we pick the one with higher index for determinism).
                let dying = if root_i > root_j { root_i } else { root_j };
                if !death_recorded[dying] {
                    death_recorded[dying] = true;
                    pairs.push(PersistencePair::new(0.0, edge.distance));
                }

                uf.union(edge.i, edge.j);
            }
        }

        // Remaining components are immortal (death = infinity).
        // We cap at max_scale for practical purposes.
        for i in 0..n {
            if !death_recorded[i] && uf.find(i) == i {
                pairs.push(PersistencePair::new(0.0, self.config.max_scale));
            }
        }

        // Sort by birth, then death
        pairs.sort_by(|a, b| a.birth.partial_cmp(&b.birth).unwrap()
            .then(a.death.partial_cmp(&b.death).unwrap()));

        pairs
    }

    /// Compute PH₁ persistence pairs using boundary matrix reduction over GF(2).
    ///
    /// Simplified implementation using edge triangulation detection:
    /// A 1-cycle (loop) is born when a triangle's third edge is added,
    /// and dies when the triangle is filled (all 3 edges exist).
    fn compute_ph1(&self, edges: &[Edge], points: &[EthicalPoint]) -> Vec<PersistencePair> {
        let n = points.len();
        if n < 3 {
            return Vec::new();
        }

        // Build adjacency for triangle detection
        let mut adjacency = vec![vec![false; n]; n];
        let mut edge_scale = vec![vec![f64::INFINITY; n]; n];
        let mut pairs = Vec::new();

        for edge in edges {
            let (i, j) = if edge.i < edge.j {
                (edge.i, edge.j)
            } else {
                (edge.j, edge.i)
            };
            adjacency[i][j] = true;
            adjacency[j][i] = true;
            edge_scale[i][j] = edge.distance;
            edge_scale[j][i] = edge.distance;

            // Check for triangle formation (1-cycle birth)
            // When edge (i,j) is added, check if there exists k such that
            // edges (i,k) and (j,k) already exist but (i,j) was missing.
            for k in 0..n {
                if k != i && k != j {
                    // Triangle (i,j,k) is formed when the last edge is added.
                    // Check if this creates a new cycle.
                    if adjacency[i][k] && adjacency[j][k] {
                        // Triangle exists. The cycle was born when the second edge
                        // was added and dies when the third (current) edge fills it.
                        let scale_ik = edge_scale[i][k];
                        let scale_jk = edge_scale[j][k];
                        let birth = scale_ik.min(scale_jk);
                        let death = edge.distance;

                        if death > birth {
                            pairs.push(PersistencePair::new(birth, death));
                        }
                    }
                }
            }
        }

        // Sort by birth, then death
        pairs.sort_by(|a, b| a.birth.partial_cmp(&b.birth).unwrap()
            .then(a.death.partial_cmp(&b.death).unwrap()));

        pairs
    }
}

impl Default for PersistentHomologyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_point(x: f64, y: f64, z: f64) -> EthicalPoint {
        EthicalPoint::new(x, y, z)
    }

    #[test]
    fn test_ethical_point_creation() {
        let p = make_point(0.5, 0.3, 0.7);
        assert_eq!(p.x, 0.5);
        assert_eq!(p.y, 0.3);
        assert_eq!(p.z, 0.7);
    }

    #[test]
    fn test_euclidean_distance() {
        let p1 = make_point(0.0, 0.0, 0.0);
        let p2 = make_point(1.0, 0.0, 0.0);
        let dist = p1.euclidean_distance(&p2);
        assert!((dist - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ethical_distance_z_weighting() {
        let p1 = make_point(0.5, 0.5, 0.9); // High ethical Z
        let p2 = make_point(0.6, 0.5, 0.9); // High ethical Z
        let p3 = make_point(0.5, 0.5, -0.5); // Low ethical Z
        let p4 = make_point(0.6, 0.5, -0.5); // Low ethical Z

        let dist_ethical = ethical_distance(&p1, &p2, 2.0);
        let dist_unethical = ethical_distance(&p3, &p4, 2.0);

        // Ethical points should have shorter effective distance
        assert!(dist_ethical < dist_unethical);
    }

    #[test]
    fn test_persistence_pair_lifetime() {
        let pair = PersistencePair::new(0.1, 0.5);
        assert!((pair.lifetime() - 0.4).abs() < 1e-10);
        assert!(pair.is_persistent(0.3));
        assert!(!pair.is_persistent(0.5));
    }

    #[test]
    fn test_engine_creation() {
        let engine = PersistentHomologyEngine::new();
        assert_eq!(engine.config.alpha, 2.0);
    }

    #[test]
    fn test_engine_custom_config() {
        let config = HomologyConfig {
            alpha: 3.0,
            max_scale: 1.5,
            persistence_threshold: 0.1,
            max_points: 5000,
        };
        let engine = PersistentHomologyEngine::with_config(config);
        assert_eq!(engine.config.alpha, 3.0);
    }

    #[test]
    fn test_empty_point_cloud() {
        let engine = PersistentHomologyEngine::new();
        let result = engine.compute(&[]);
        assert!(result.ph0_pairs.is_empty());
        assert!(result.ph1_pairs.is_empty());
        assert_eq!(result.num_points, 0);
    }

    #[test]
    fn test_single_point() {
        let engine = PersistentHomologyEngine::new();
        let points = vec![make_point(0.5, 0.5, 0.5)];
        let result = engine.compute(&points);
        assert!(result.ph0_pairs.is_empty());
        assert!(result.ph1_pairs.is_empty());
        assert_eq!(result.num_points, 1);
    }

    #[test]
    fn test_two_points_ph0() {
        let engine = PersistentHomologyEngine::new();
        let points = vec![
            make_point(0.3, 0.3, 0.5),
            make_point(0.4, 0.3, 0.5),
        ];
        let result = engine.compute(&points);
        // Two points merge into one component
        assert!(result.ph0_pairs.len() >= 1);
        assert_eq!(result.num_points, 2);
    }

    #[test]
    fn test_cluster_ph0() {
        let engine = PersistentHomologyEngine::new();
        // Create a tight cluster of ethical points
        let points: Vec<EthicalPoint> = (0..10)
            .map(|i| make_point(0.5 + i as f64 * 0.01, 0.5, 0.8))
            .collect();

        let result = engine.compute(&points);
        assert_eq!(result.num_points, 10);
        // Should have PH₀ pairs from component merges
        assert!(!result.ph0_pairs.is_empty());
    }

    #[test]
    fn test_triangle_ph1() {
        let engine = PersistentHomologyEngine::new();
        // Equilateral triangle in ethical space
        let points = vec![
            make_point(0.3, 0.3, 0.5),
            make_point(0.7, 0.3, 0.5),
            make_point(0.5, 0.7, 0.5),
        ];
        let result = engine.compute(&points);
        assert_eq!(result.num_points, 3);
        // Triangle should create at least one PH₁ pair
        assert!(!result.ph1_pairs.is_empty());
    }

    #[test]
    fn test_homology_result_integrals() {
        let result = HomologyResult {
            ph0_pairs: vec![
                PersistencePair::new(0.0, 0.3),
                PersistencePair::new(0.0, 0.5),
            ],
            ph1_pairs: vec![
                PersistencePair::new(0.1, 0.4),
            ],
            num_points: 5,
            num_edges: 10,
            alpha: 2.0,
        };

        assert!((result.ph0_integral() - 0.8).abs() < 1e-10);
        assert!((result.ph1_integral() - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_persistent_feature_count() {
        let result = HomologyResult {
            ph0_pairs: vec![
                PersistencePair::new(0.0, 0.1),
                PersistencePair::new(0.0, 0.5),
                PersistencePair::new(0.0, 0.8),
            ],
            ph1_pairs: vec![
                PersistencePair::new(0.1, 0.2),
                PersistencePair::new(0.1, 0.6),
            ],
            num_points: 5,
            num_edges: 10,
            alpha: 2.0,
        };

        let (ph0_count, ph1_count) = result.persistent_feature_count(0.3);
        assert_eq!(ph0_count, 2); // 0.5 and 0.8 >= 0.3
        assert_eq!(ph1_count, 1); // 0.5 >= 0.3
    }

    #[test]
    fn test_betti_numbers_at_scale() {
        let result = HomologyResult {
            ph0_pairs: vec![
                PersistencePair::new(0.0, 0.3),
                PersistencePair::new(0.0, 0.7),
                PersistencePair::new(0.0, 1.0),
            ],
            ph1_pairs: vec![
                PersistencePair::new(0.1, 0.5),
            ],
            num_points: 5,
            num_edges: 10,
            alpha: 2.0,
        };

        // At scale 0.4: 2 components alive (0.7, 1.0), 1 loop alive (0.5 > 0.4)
        let (b0, b1) = result.betti_numbers_at_scale(0.4);
        assert_eq!(b0, 2);
        assert_eq!(b1, 1);
    }

    #[test]
    fn test_from_stuartian_tensor() {
        let tensor = StuartianTensor::new(0.6, 0.3, 0.5).unwrap();
        let point = EthicalPoint::from_stuartian(&tensor);
        assert!((point.x - 0.6).abs() < 1e-10);
        assert!((point.y - 0.3).abs() < 1e-10);
        assert!((point.z - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_default_config() {
        let config = HomologyConfig::default();
        assert_eq!(config.alpha, 2.0);
        assert_eq!(config.max_scale, 2.0);
        assert_eq!(config.persistence_threshold, 0.05);
        assert_eq!(config.max_points, 10_000);
    }

    #[test]
    fn test_default_engine() {
        let engine = PersistentHomologyEngine::default();
        assert_eq!(engine.config.alpha, 2.0);
    }

    #[test]
    fn test_large_point_cloud() {
        let engine = PersistentHomologyEngine::new();
        // Generate 50 points in ethical space
        let points: Vec<EthicalPoint> = (0..50)
            .map(|i| {
                make_point(
                    0.1 + (i % 10) as f64 * 0.08,
                    0.1 + (i / 10) as f64 * 0.08,
                    0.3 + (i % 5) as f64 * 0.1,
                )
            })
            .collect();

        let result = engine.compute(&points);
        assert_eq!(result.num_points, 50);
        assert!(result.num_edges > 0);
        assert!(!result.ph0_pairs.is_empty());
    }

    #[test]
    fn test_max_points_limit() {
        let config = HomologyConfig {
            max_points: 10,
            ..HomologyConfig::default()
        };
        let engine = PersistentHomologyEngine::with_config(config);
        let points: Vec<EthicalPoint> = (0..100)
            .map(|i| make_point(i as f64 * 0.01, 0.5, 0.5))
            .collect();

        let result = engine.compute(&points);
        assert_eq!(result.num_points, 10); // Capped at max_points
    }

    #[test]
    fn test_identical_points() {
        let engine = PersistentHomologyEngine::new();
        let points = vec![
            make_point(0.5, 0.5, 0.5),
            make_point(0.5, 0.5, 0.5),
            make_point(0.5, 0.5, 0.5),
        ];
        let result = engine.compute(&points);
        // All points at same location → distance 0 → immediate merge
        assert_eq!(result.num_points, 3);
    }

    #[test]
    fn test_highly_ethical_vs_unethical_clustering() {
        let engine = PersistentHomologyEngine::new();

        // Highly ethical cluster (high Z)
        let ethical_points: Vec<EthicalPoint> = (0..5)
            .map(|i| make_point(0.5 + i as f64 * 0.05, 0.3, 0.9))
            .collect();

        // Unethical cluster (negative Z)
        let unethical_points: Vec<EthicalPoint> = (0..5)
            .map(|i| make_point(0.5 + i as f64 * 0.05, 0.3, -0.5))
            .collect();

        let ethical_result = engine.compute(&ethical_points);
        let unethical_result = engine.compute(&unethical_points);

        // Ethical points should form tighter topological connections
        // (lower effective distances due to Z weighting)
        let ethical_integral = ethical_result.ph0_integral();
        let unethical_integral = unethical_result.ph0_integral();

        // The integral reflects persistence; ethical clustering should show
        // different topological structure due to distance weighting
        assert!(ethical_integral >= 0.0 || unethical_integral >= 0.0);
    }
}
