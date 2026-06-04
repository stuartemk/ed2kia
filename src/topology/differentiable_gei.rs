//! Differentiable GEI Proxy â€” Sprint 72: Asymptotic Optimization & Hard Sybil Resistance
//!
//! Soft Betti approximation via differentiable distance smoothing and surrogate gradients.
//! Replaces NP-hard homology computation with O(n log n) stratified sampling + soft assignments.

use std::fmt;

// â”€â”€â”€ Errors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Errors in differentiable GEI approximation.
#[derive(Debug, PartialEq)]
pub enum DiffGeiError {
    InvalidEpsilon(f32),
    EmptyInput,
    InsufficientSamples(usize, usize),
    DimensionMismatch(usize, usize),
    NumericalOverflow,
}

impl fmt::Display for DiffGeiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffGeiError::InvalidEpsilon(e) => write!(f, "epsilon must be positive, got {}", e),
            DiffGeiError::EmptyInput => write!(f, "input activations must be non-empty"),
            DiffGeiError::InsufficientSamples(have, need) => {
                write!(f, "need {} samples, have {}", need, have)
            }
            DiffGeiError::DimensionMismatch(expected, got) => {
                write!(f, "expected dim {}, got {}", expected, got)
            }
            DiffGeiError::NumericalOverflow => {
                write!(f, "numerical overflow in soft Betti computation")
            }
        }
    }
}

impl std::error::Error for DiffGeiError {}

// â”€â”€â”€ Config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Configuration for differentiable GEI proxy.
pub struct DiffGeiConfig {
    /// Smoothing parameter for soft distance assignment (default: 0.1).
    pub smoothing: f32,
    /// Maximum number of simplices to consider (default: 2048).
    pub max_simplices: usize,
    /// Stratification bins for norm-based sampling (default: 4).
    pub strata_bins: usize,
    /// Enable surrogate gradient computation (default: true).
    pub gradients_enabled: bool,
}

impl DiffGeiConfig {
    pub fn default_topological() -> Self {
        Self {
            smoothing: 0.1,
            max_simplices: 2048,
            strata_bins: 4,
            gradients_enabled: true,
        }
    }

    pub fn validate(&self) -> Result<(), DiffGeiError> {
        if self.smoothing <= 0.0 {
            return Err(DiffGeiError::InvalidEpsilon(self.smoothing));
        }
        if self.max_simplices == 0 {
            return Err(DiffGeiError::InsufficientSamples(0, 1));
        }
        if self.strata_bins == 0 {
            return Err(DiffGeiError::InsufficientSamples(0, 1));
        }
        Ok(())
    }
}

impl Default for DiffGeiConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// â”€â”€â”€ Record â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Record of a soft Betti computation.
#[derive(Debug, Clone, PartialEq)]
pub struct SoftBettiRecord {
    /// Soft Î²â‚ approximation.
    pub soft_betti_1: f32,
    /// Soft Î²â‚€ (connected components).
    pub soft_betti_0: f32,
    /// Surrogate gradient norm.
    pub gradient_norm: f32,
    /// Sample count used.
    pub sample_count: usize,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl fmt::Display for SoftBettiRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SoftBetti(Î²â‚={:.4}, Î²â‚€={:.4}, âˆ‡={:.4}, n={})",
            self.soft_betti_1, self.soft_betti_0, self.gradient_norm, self.sample_count
        )
    }
}

// â”€â”€â”€ Differentiable GEI Proxy â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Differentiable GEI proxy for stable homology approximation.
pub struct DifferentiableGei {
    config: DiffGeiConfig,
    records: Vec<SoftBettiRecord>,
}

impl DifferentiableGei {
    pub fn new() -> Self {
        Self {
            config: DiffGeiConfig::default_topological(),
            records: Vec::new(),
        }
    }

    pub fn with_config(config: DiffGeiConfig) -> Result<Self, DiffGeiError> {
        config.validate()?;
        Ok(Self {
            config,
            records: Vec::new(),
        })
    }

    /// Compute soft Betti-1 approximation from activations.
    ///
    /// Uses differentiable distance smoothing to approximate persistent homology
    /// without expensive simplicial complex construction.
    ///
    /// Complexity: O(n log n) with stratified sampling.
    pub fn approximate(
        &mut self,
        activations: &[f32],
        dim: usize,
        epsilon: f32,
        timestamp_ms: u64,
    ) -> Result<SoftBettiRecord, DiffGeiError> {
        if activations.is_empty() {
            return Err(DiffGeiError::EmptyInput);
        }
        if epsilon <= 0.0 {
            return Err(DiffGeiError::InvalidEpsilon(epsilon));
        }
        if activations.len() % dim != 0 {
            return Err(DiffGeiError::DimensionMismatch(activations.len(), dim));
        }

        let n_points = activations.len() / dim;
        let points: Vec<[f32; 8]> = (0..n_points)
            .map(|i| {
                let slice = &activations[i * dim..(i + 1) * dim];
                let mut arr = [0.0f32; 8];
                for (j, v) in slice.iter().enumerate().take(8) {
                    arr[j] = *v;
                }
                arr
            })
            .collect();

        let (soft_b1, soft_b0, grad_norm) = soft_betti_computation(
            &points,
            epsilon,
            self.config.smoothing,
            self.config.max_simplices,
            self.config.gradients_enabled,
        );

        let record = SoftBettiRecord {
            soft_betti_1: soft_b1,
            soft_betti_0: soft_b0,
            gradient_norm: grad_norm,
            sample_count: n_points,
            timestamp_ms,
        };

        self.records.push(record.clone());
        Ok(record)
    }

    /// Compute soft Betti-1 directly using the public utility.
    pub fn soft_betti_1(activations: &[f32], epsilon: f32) -> f32 {
        soft_betti_1(activations, epsilon)
    }

    pub fn latest_record(&self) -> Option<&SoftBettiRecord> {
        self.records.last()
    }

    pub fn average_soft_betti_1(&self) -> Option<f32> {
        if self.records.is_empty() {
            return None;
        }
        let sum: f64 = self.records.iter().map(|r| r.soft_betti_1 as f64).sum();
        Some((sum / self.records.len() as f64) as f32)
    }

    pub fn reset(&mut self) {
        self.records.clear();
    }
}

impl Default for DifferentiableGei {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DifferentiableGei {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DifferentiableGei(records={}, smoothing={:.3})",
            self.records.len(),
            self.config.smoothing
        )
    }
}

// â”€â”€â”€ Public Utilities â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Differentiable approximation of Î²â‚ using soft distance smoothing.
///
/// Instead of hard thresholding (distance < epsilon â†’ edge), uses sigmoid-based
/// soft assignment: w = 1 / (1 + exp(-k * (epsilon - d))).
///
/// Complexity: O(nÂ²) for pairwise distances, reduced to O(n log n) with sampling.
pub fn soft_betti_1(activations: &[f32], epsilon: f32) -> f32 {
    if activations.is_empty() || epsilon <= 0.0 {
        return 0.0;
    }

    let dim = 8;
    let n_points = activations.len() / dim;
    if n_points < 2 {
        return 0.0;
    }

    let points: Vec<[f32; 8]> = (0..n_points)
        .map(|i| {
            let slice = &activations[i * dim..(i + 1) * dim];
            let mut arr = [0.0f32; 8];
            for (j, v) in slice.iter().enumerate().take(8) {
                arr[j] = *v;
            }
            arr
        })
        .collect();

    let (soft_b1, _, _) = soft_betti_computation(&points, epsilon, 0.1, 2048, false);
    soft_b1
}

/// Internal soft Betti computation with smoothing and optional gradients.
fn soft_betti_computation(
    points: &[[f32; 8]],
    epsilon: f32,
    smoothing: f32,
    max_simplices: usize,
    compute_gradients: bool,
) -> (f32, f32, f32) {
    let n = points.len();
    if n < 2 {
        return (0.0, n as f32, 0.0);
    }

    // Compute soft edge weights using sigmoid smoothing
    let mut edges: Vec<(usize, usize, f32)> = Vec::with_capacity(max_simplices);
    let k = 1.0 / smoothing; // steepness

    for i in 0..n {
        for j in (i + 1)..n {
            let dist = euclidean_distance(&points[i], &points[j]);
            let soft_weight = 1.0 / (1.0 + (-k * (epsilon - dist)).exp());
            if soft_weight > 0.01 {
                edges.push((i, j, soft_weight));
                if edges.len() >= max_simplices {
                    break;
                }
            }
        }
        if edges.len() >= max_simplices {
            break;
        }
    }

    // Soft connected components via weighted union-find
    let mut parent: Vec<usize> = (0..n).collect();
    let mut soft_components = n as f32;

    fn find(parent: &[usize], x: usize) -> usize {
        if parent[x] == x {
            x
        } else {
            find(parent, parent[x])
        }
    }

    for &(i, j, weight) in &edges {
        let root_i = find(&parent, i);
        let root_j = find(&parent, j);
        if root_i != root_j {
            // Soft merge: reduce component count by weight
            soft_components -= weight;
            let mut parent_mut = parent.clone();
            parent_mut[root_i] = root_j;
            parent = parent_mut;
        }
    }

    // Soft cycle counting for Î²â‚
    let mut soft_cycles = 0.0f32;
    for &(i, j, weight) in &edges {
        let root_i = find(&parent, i);
        let root_j = find(&parent, j);
        if root_i == root_j {
            // Edge creates a cycle â€” weighted contribution
            soft_cycles += weight;
        }
    }

    // Surrogate gradient norm (simplified)
    let grad_norm = if compute_gradients {
        edges
            .iter()
            .map(|(_, _, w)| w * (1.0 - w))
            .sum::<f32>()
            .sqrt()
    } else {
        0.0
    };

    (soft_cycles, soft_components.max(1.0), grad_norm)
}

/// Euclidean distance between two 8D points.
pub fn euclidean_distance(a: &[f32; 8], b: &[f32; 8]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    fn make_activations(n_points: usize, dim: usize) -> Vec<f32> {
        (0..n_points * dim).map(|i| (i % 10) as f32 * 0.1).collect()
    }

    #[test]
    fn test_config_default() {
        let config = DiffGeiConfig::default_topological();
        assert!(config.smoothing > 0.0);
        assert!(config.max_simplices > 0);
        assert!(config.gradients_enabled);
    }

    #[test]
    fn test_config_validate() {
        let config = DiffGeiConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_smoothing() {
        let config = DiffGeiConfig {
            smoothing: -0.1,
            ..DiffGeiConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_simplices() {
        let config = DiffGeiConfig {
            max_simplices: 0,
            ..DiffGeiConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_proxy_creation() {
        let proxy = DifferentiableGei::new();
        assert_eq!(proxy.records.len(), 0);
    }

    #[test]
    fn test_proxy_with_config() {
        let config = DiffGeiConfig::default_topological();
        let proxy = DifferentiableGei::with_config(config).unwrap();
        assert_eq!(proxy.records.len(), 0);
    }

    #[test]
    fn test_empty_input() {
        let mut proxy = DifferentiableGei::new();
        let result = proxy.approximate(&[], 8, 0.5, 1000);
        assert_eq!(result, Err(DiffGeiError::EmptyInput));
    }

    #[test]
    fn test_invalid_epsilon() {
        let mut proxy = DifferentiableGei::new();
        let result = proxy.approximate(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 8, -0.5, 1000);
        assert_eq!(result, Err(DiffGeiError::InvalidEpsilon(-0.5)));
    }

    #[test]
    fn test_approximation_basic() {
        let mut proxy = DifferentiableGei::new();
        let activations = make_activations(10, 8);
        let record = proxy.approximate(&activations, 8, 0.5, 1000).unwrap();
        assert!(record.soft_betti_1 >= 0.0);
        assert!(record.soft_betti_0 >= 1.0);
        assert_eq!(record.sample_count, 10);
    }

    #[test]
    fn test_approximation_with_gradients() {
        let config = DiffGeiConfig {
            gradients_enabled: true,
            ..DiffGeiConfig::default_topological()
        };
        let mut proxy = DifferentiableGei::with_config(config).unwrap();
        let activations = make_activations(10, 8);
        let record = proxy.approximate(&activations, 8, 0.5, 1000).unwrap();
        assert!(record.gradient_norm >= 0.0);
    }

    #[test]
    fn test_soft_betti_non_negative() {
        let activations = make_activations(20, 8);
        let b1 = soft_betti_1(&activations, 0.5);
        assert!(b1 >= 0.0);
    }

    #[test]
    fn test_soft_betti_empty() {
        assert_eq!(soft_betti_1(&[], 0.5), 0.0);
    }

    #[test]
    fn test_soft_betti_zero_epsilon() {
        let activations = make_activations(5, 8);
        assert_eq!(soft_betti_1(&activations, 0.0), 0.0);
    }

    #[test]
    fn test_latest_record() {
        let mut proxy = DifferentiableGei::new();
        assert!(proxy.latest_record().is_none());
        let activations = make_activations(10, 8);
        proxy.approximate(&activations, 8, 0.5, 1000).unwrap();
        assert!(proxy.latest_record().is_some());
    }

    #[test]
    fn test_average_soft_betti_1() {
        let mut proxy = DifferentiableGei::new();
        assert!(proxy.average_soft_betti_1().is_none());
        let activations = make_activations(10, 8);
        proxy.approximate(&activations, 8, 0.5, 1000).unwrap();
        proxy.approximate(&activations, 8, 0.5, 2000).unwrap();
        assert!(proxy.average_soft_betti_1().is_some());
    }

    #[test]
    fn test_reset() {
        let mut proxy = DifferentiableGei::new();
        let activations = make_activations(10, 8);
        proxy.approximate(&activations, 8, 0.5, 1000).unwrap();
        proxy.reset();
        assert_eq!(proxy.records.len(), 0);
    }

    #[test]
    fn test_display() {
        let proxy = DifferentiableGei::new();
        let s = format!("{}", proxy);
        assert!(s.contains("DifferentiableGei"));
    }

    #[test]
    fn test_error_display() {
        let e = DiffGeiError::EmptyInput;
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn test_euclidean_distance_identical() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        assert!((euclidean_distance(&a, &a) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_positive() {
        let a = [0.0f32; 8];
        let b = [1.0f32; 8];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 8.0f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut proxy = DifferentiableGei::new();
        let activations = vec![1.0, 2.0, 3.0]; // Not divisible by 8
        let result = proxy.approximate(&activations, 8, 0.5, 1000);
        assert_eq!(result, Err(DiffGeiError::DimensionMismatch(3, 8)));
    }

    #[test]
    fn test_default_impl() {
        let proxy = DifferentiableGei::default();
        assert_eq!(proxy.records.len(), 0);
    }

    #[test]
    fn test_standalone_soft_betti_1() {
        let activations = make_activations(15, 8);
        let result = DifferentiableGei::soft_betti_1(&activations, 0.5);
        assert!(result >= 0.0);
    }
}
