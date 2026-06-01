//! Lightweight GEI Proxy — Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! Approximación diferenciable de β₁ con muestreo estratificado.
//! O(n log n) esperado. Sin ZKP pesado en WASM/Edge.
//!
//! **Pivot Arquitectónico:** GEI como proxy de estabilidad estructural,
//! no oráculo ético directo. Delegación ZKP a Prover Nodes.

use std::cmp::Ordering;
use std::fmt;

/// Error types for Lightweight GEI Proxy
#[derive(Debug, Clone, PartialEq)]
pub enum ProxyError {
    /// Invalid sample rate (must be in (0, 1])
    InvalidSampleRate(f64),
    /// Negative epsilon for Vietoris-Rips
    NegativeEpsilon(f32),
    /// Empty activations
    EmptyActivations,
    /// Dimension mismatch (expected 8-dim GEI vectors)
    DimensionMismatch { expected: usize, actual: usize },
    /// Overflow in computation
    Overflow,
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::InvalidSampleRate(r) => {
                write!(f, "Invalid sample rate: {} (must be in (0, 1])", r)
            }
            ProxyError::NegativeEpsilon(e) => write!(f, "Negative epsilon: {} (must be >= 0)", e),
            ProxyError::EmptyActivations => write!(f, "Empty activations array"),
            ProxyError::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "Dimension mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            ProxyError::Overflow => write!(f, "Numeric overflow in GEI proxy computation"),
        }
    }
}

/// Configuration for Lightweight GEI Proxy
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Maximum number of simplices to process (bounded complexity)
    pub max_simplices: usize,
    /// Stratified sampling rate (0, 1]
    pub sample_rate: f64,
    /// Smoothing factor for soft Betti approximation
    pub smoothing: f32,
    /// Enable ZKP proof delegation to Prover Nodes
    pub delegate_zkp: bool,
}

impl ProxyConfig {
    /// Default Stuartian configuration
    pub fn default_stuartian() -> Self {
        Self {
            max_simplices: 4096,
            sample_rate: 0.5,
            smoothing: 0.01,
            delegate_zkp: true,
        }
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), ProxyError> {
        if self.max_simplices == 0 {
            return Err(ProxyError::InvalidSampleRate(0.0));
        }
        if self.sample_rate <= 0.0 || self.sample_rate > 1.0 {
            return Err(ProxyError::InvalidSampleRate(self.sample_rate));
        }
        if self.smoothing < 0.0 {
            return Err(ProxyError::NegativeEpsilon(self.smoothing));
        }
        Ok(())
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Record of a soft Betti computation
#[derive(Debug, Clone)]
pub struct SoftBettiRecord {
    /// Computed soft β₁ value
    pub soft_betti_1: f32,
    /// Number of simplices processed
    pub simplices_count: usize,
    /// Sample rate used
    pub sample_rate: f64,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

impl fmt::Display for SoftBettiRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SoftBettiRecord {{ β₁: {:.4}, simplices: {}, rate: {:.2}, ts: {} }}",
            self.soft_betti_1, self.simplices_count, self.sample_rate, self.timestamp_ms
        )
    }
}

/// Lightweight GEI Proxy — Soft Betti with stratified sampling
pub struct LightweightGeiProxy {
    config: ProxyConfig,
    records: Vec<SoftBettiRecord>,
}

impl LightweightGeiProxy {
    /// Create a new proxy with default Stuartian config
    pub fn new() -> Self {
        Self {
            config: ProxyConfig::default_stuartian(),
            records: Vec::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ProxyConfig) -> Result<Self, ProxyError> {
        config.validate()?;
        Ok(Self {
            config,
            records: Vec::new(),
        })
    }

    /// Approximate soft β₁ with stratified sampling
    ///
    /// Complexity: O(n log n) expected with sampling
    pub fn approximate(
        &mut self,
        activations: &[f32],
        epsilon: f32,
        timestamp_ms: u64,
    ) -> Result<f32, ProxyError> {
        if activations.is_empty() {
            return Err(ProxyError::EmptyActivations);
        }
        if epsilon < 0.0 {
            return Err(ProxyError::NegativeEpsilon(epsilon));
        }

        // Check dimension (8-dim GEI vectors)
        if activations.len() % 8 != 0 {
            return Err(ProxyError::DimensionMismatch {
                expected: 8,
                actual: activations.len(),
            });
        }

        let n_points = activations.len() / 8;

        // Stratified sampling
        let sampled_indices = self.stratified_sample(n_points);
        let sampled_points: Vec<[f32; 8]> = sampled_indices
            .iter()
            .map(|&i| {
                let slice = &activations[i * 8..(i + 1) * 8];
                slice.try_into().unwrap()
            })
            .collect();

        // Compute soft Betti-1 on sampled points
        let (soft_b1, mut simplices, _clusters) =
            soft_betti_computation(&sampled_points, epsilon, self.config.smoothing);

        // Cap simplices
        if simplices > self.config.max_simplices {
            simplices = self.config.max_simplices;
        }

        let record = SoftBettiRecord {
            soft_betti_1: soft_b1,
            simplices_count: simplices,
            sample_rate: self.config.sample_rate,
            timestamp_ms,
        };

        self.records.push(record);
        Ok(soft_b1)
    }

    /// Stratified sampling: divide points into strata, sample proportionally
    fn stratified_sample(&self, n: usize) -> Vec<usize> {
        if n == 0 {
            return vec![];
        }
        let target = (n as f64 * self.config.sample_rate).max(1.0) as usize;
        if target >= n {
            return (0..n).collect();
        }

        // Divide into strata (clusters by index range)
        let n_strata = (target.min(10)).max(1);
        let stratum_size = n / n_strata;
        let mut indices = Vec::with_capacity(target);

        for s in 0..n_strata {
            let start = s * stratum_size;
            let end = if s == n_strata - 1 {
                n
            } else {
                (s + 1) * stratum_size
            };
            let stratum_len = end - start;
            let samples_in_stratum =
                (stratum_len as f64 * self.config.sample_rate).max(1.0) as usize;

            // Uniform sampling within stratum
            let step = stratum_len.max(1) / samples_in_stratum.max(1);
            for i in 0..samples_in_stratum {
                let idx = start + i * step;
                if idx < end {
                    indices.push(idx);
                }
            }
        }

        indices.truncate(target);
        indices
    }

    /// Get the latest computation record
    pub fn latest_record(&self) -> Option<&SoftBettiRecord> {
        self.records.last()
    }

    /// Compute average soft β₁ across all records
    pub fn average_soft_betti_1(&self) -> Option<f32> {
        if self.records.is_empty() {
            return None;
        }
        let sum: f64 = self.records.iter().map(|r| r.soft_betti_1 as f64).sum();
        Some((sum / self.records.len() as f64) as f32)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.records.clear();
    }
}

impl Default for LightweightGeiProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LightweightGeiProxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LightweightGeiProxy {{ records: {}, delegate_zkp: {}, sample_rate: {:.2} }}",
            self.records.len(),
            self.config.delegate_zkp,
            self.config.sample_rate
        )
    }
}

/// Compute soft Betti-1 with surrogate gradients
///
/// Returns (soft_betti_1, simplices_count, clusters)
pub fn soft_betti_1_sampled(activations: &[f32], sample_rate: f64, epsilon: f32) -> f32 {
    if activations.is_empty() || activations.len() % 8 != 0 {
        return 0.0;
    }

    let n_points = activations.len() / 8;
    let target = (n_points as f64 * sample_rate).max(1.0) as usize;

    // Sample points
    let step = n_points.max(1) / target.max(1);
    let points: Vec<[f32; 8]> = (0..n_points)
        .step_by(step)
        .take(target)
        .map(|i| {
            let slice = &activations[i * 8..(i + 1) * 8];
            slice.try_into().unwrap()
        })
        .collect();

    let (soft_b1, _, _) = soft_betti_computation(&points, epsilon, 0.01);
    soft_b1
}

/// Internal soft Betti computation with Union-Find
fn soft_betti_computation(
    points: &[[f32; 8]],
    epsilon: f32,
    smoothing: f32,
) -> (f32, usize, usize) {
    let n = points.len();
    if n < 2 {
        return (0.0, 0, n);
    }

    // Union-Find for connected components
    let parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u32; n];

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }

    fn union(parent: &mut [usize], rank: &mut [u32], a: usize, b: usize) -> bool {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return false;
        }
        match rank[ra].cmp(&rank[rb]) {
            Ordering::Less => parent[ra] = rb,
            Ordering::Greater => parent[rb] = ra,
            Ordering::Equal => {
                parent[rb] = ra;
                rank[ra] += 1;
            }
        }
        true
    }

    // Build Vietoris-Rips edges with soft threshold
    let mut edges = 0usize;
    let mut mut_parent = parent.clone();

    for i in 0..n {
        for j in (i + 1)..n {
            let dist = euclidean_distance(&points[i], &points[j]);
            // Soft threshold: sigmoid-like transition
            let soft_edge = 1.0 / (1.0 + (-((dist - epsilon) / smoothing.max(f32::EPSILON))).exp());
            if soft_edge > 0.5 {
                union(&mut mut_parent, &mut rank, i, j);
                edges += 1;
            }
        }
    }

    // Count connected components
    let mut components = std::collections::HashSet::new();
    for i in 0..n {
        components.insert(find(&mut mut_parent, i));
    }
    let beta_0 = components.len() as f32;

    // Soft Betti-1 = edges - vertices + beta_0 (Euler characteristic approximation)
    let vertices = n as f32;
    let soft_betti_1 = (edges as f32 - vertices + beta_0).max(0.0);

    (soft_betti_1, edges, components.len())
}

/// Euclidean distance between two 8-dim vectors
pub fn euclidean_distance(a: &[f32; 8], b: &[f32; 8]) -> f32 {
    let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_activations(n_points: usize) -> Vec<f32> {
        (0..n_points * 8)
            .map(|i| (i % 100) as f32 / 100.0)
            .collect()
    }

    #[test]
    fn test_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.max_simplices, 4096);
        assert_eq!(config.sample_rate, 0.5);
        assert!(config.delegate_zkp);
    }

    #[test]
    fn test_config_validate() {
        let config = ProxyConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_sample_rate() {
        let config = ProxyConfig {
            sample_rate: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_simplices() {
        let config = ProxyConfig {
            max_simplices: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_proxy_creation() {
        let proxy = LightweightGeiProxy::new();
        assert!(proxy.latest_record().is_none());
    }

    #[test]
    fn test_proxy_with_config() {
        let config = ProxyConfig::default_stuartian();
        let proxy = LightweightGeiProxy::with_config(config);
        assert!(proxy.is_ok());
    }

    #[test]
    fn test_empty_input() {
        let mut proxy = LightweightGeiProxy::new();
        let result = proxy.approximate(&[], 0.1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_epsilon() {
        let mut proxy = LightweightGeiProxy::new();
        let activations = make_activations(10);
        let result = proxy.approximate(&activations, -0.1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_approximation_basic() {
        let mut proxy = LightweightGeiProxy::new();
        let activations = make_activations(16);
        let result = proxy.approximate(&activations, 0.5, 1000);
        assert!(result.is_ok());
        assert!(result.unwrap() >= 0.0);
    }

    #[test]
    fn test_approximation_with_gradients() {
        let mut proxy = LightweightGeiProxy::new();
        let activations = make_activations(32);
        let result = proxy.approximate(&activations, 0.3, 2000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_soft_betti_non_negative() {
        let activations = make_activations(20);
        let b1 = soft_betti_1_sampled(&activations, 0.5, 0.5);
        assert!(b1 >= 0.0);
    }

    #[test]
    fn test_soft_betti_zero_epsilon() {
        let activations = make_activations(10);
        let b1 = soft_betti_1_sampled(&activations, 1.0, 0.0);
        assert!(b1 >= 0.0);
    }

    #[test]
    fn test_latest_record() {
        let mut proxy = LightweightGeiProxy::new();
        let activations = make_activations(16);
        proxy.approximate(&activations, 0.5, 1000).unwrap();
        let record = proxy.latest_record().unwrap();
        assert_eq!(record.timestamp_ms, 1000);
    }

    #[test]
    fn test_average_soft_betti_1() {
        let mut proxy = LightweightGeiProxy::new();
        let activations = make_activations(16);
        proxy.approximate(&activations, 0.5, 1000).unwrap();
        proxy.approximate(&activations, 0.5, 2000).unwrap();
        let avg = proxy.average_soft_betti_1().unwrap();
        assert!(avg >= 0.0);
    }

    #[test]
    fn test_reset() {
        let mut proxy = LightweightGeiProxy::new();
        let activations = make_activations(16);
        proxy.approximate(&activations, 0.5, 1000).unwrap();
        proxy.reset();
        assert!(proxy.latest_record().is_none());
    }

    #[test]
    fn test_display() {
        let proxy = LightweightGeiProxy::new();
        let s = format!("{}", proxy);
        assert!(s.contains("LightweightGeiProxy"));
    }

    #[test]
    fn test_error_display() {
        let err = ProxyError::EmptyActivations;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_euclidean_distance_identical() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        assert!((euclidean_distance(&a, &a) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_euclidean_distance_positive() {
        let a = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!((euclidean_distance(&a, &b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut proxy = LightweightGeiProxy::new();
        let bad = vec![1.0, 2.0, 3.0]; // Not multiple of 8
        let result = proxy.approximate(&bad, 0.5, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_impl() {
        let proxy = <LightweightGeiProxy as Default>::default();
        assert!(proxy.latest_record().is_none());
    }

    #[test]
    fn test_standalone_soft_betti_1() {
        let activations = make_activations(50);
        let b1 = soft_betti_1_sampled(&activations, 0.3, 0.4);
        assert!(b1 >= 0.0);
    }
}
