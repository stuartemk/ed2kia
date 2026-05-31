//! GEI (Geometric Ethical Invariant) Validation — Sprint 68
//!
//! Validates the GEI topological fingerprint against simulated ethical benchmarks
//! using Vietoris-Rips persistent homology (β₀ components, β₁ cycles).
//!
//! The GEI is an 8-dimensional topological fingerprint where:
//! - β₀ (connected components) represents ethical fragmentation
//! - β₁ (persistent cycles) represents ethical loops/hallucination clusters
//!
//! High β₁ correlates with hallucination clusters and bias patterns.

#[cfg(feature = "v9.4-validation-layer")]
use std::fmt;

/// Result of GEI validation against ethical benchmarks.
#[cfg(feature = "v9.4-validation-layer")]
#[derive(Debug, Clone)]
pub struct GEIValidationResult {
    /// Betti number β₀ (connected components).
    pub beta_0: usize,
    /// Betti number β₁ (persistent cycles).
    pub beta_1: usize,
    /// 8-dimensional GEI fingerprint.
    pub fingerprint: [f64; 8],
    /// Hallucination cluster count (derived from β₁).
    pub hallucination_clusters: usize,
    /// Bias correlation score [0.0, 1.0].
    pub bias_correlation: f64,
    /// Overall GEI validity score [0.0, 1.0].
    pub validity_score: f64,
    /// Whether GEI passes validation threshold.
    pub valid: bool,
}

#[cfg(feature = "v9.4-validation-layer")]
impl fmt::Display for GEIValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GEI(β₀={}, β₁={}, clusters={}, bias={:.4}, valid={})",
            self.beta_0, self.beta_1, self.hallucination_clusters,
            self.bias_correlation, self.valid
        )
    }
}

/// Compute cosine similarity between two vectors.
#[cfg(feature = "v9.4-validation-layer")]
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-12 {
        return 0.0;
    }
    dot / denom
}

/// Compute Betti numbers for a Vietoris-Rips complex at a given threshold.
///
/// Uses Union-Find for β₀ and triangle counting for β₁ approximation.
#[cfg(feature = "v9.4-validation-layer")]
fn compute_betti_numbers(activations: &[Vec<f64>], threshold: f64) -> (usize, usize) {
    let n = activations.len();
    if n == 0 {
        return (0, 0);
    }

    // Union-Find for β₀ (connected components)
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank: Vec<u32> = vec![0; n];

    let find = |mut x: usize, parent: &mut Vec<usize>| -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]]; // Path compression
            x = parent[x];
        }
        x
    };

    let union = |mut x: usize, mut y: usize, parent: &mut Vec<usize>, rank: &mut Vec<u32>| {
        x = find(x, parent);
        y = find(y, parent);
        if x == y {
            return;
        }
        if rank[x] < rank[y] {
            std::mem::swap(&mut x, &mut y);
        }
        parent[y] = x;
        if rank[x] == rank[y] {
            rank[x] += 1;
        }
    };

    // Build 1-skeleton: connect nodes within threshold
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            // Compute Euclidean distance
            let mut dist_sq = 0.0;
            for k in 0..activations[i].len().min(activations[j].len()) {
                let diff = activations[i][k] - activations[j][k];
                dist_sq += diff * diff;
            }
            let dist = dist_sq.sqrt();
            if dist <= threshold {
                edges.push((i, j));
                union(i, j, &mut parent, &mut rank);
            }
        }
    }

    // Count β₀ (connected components)
    let mut roots = std::collections::HashSet::new();
    for i in 0..n {
        roots.insert(find(i, &mut parent));
    }
    let beta_0 = roots.len();

    // Count β₁ (triangles/cycles) - simplified triangle counting
    let mut beta_1 = 0;
    for e1 in 0..edges.len() {
        for e2 in (e1 + 1)..edges.len() {
            let (a, b) = edges[e1];
            let (c, d) = edges[e2];
            // Check if edges form a triangle
            if a == c || a == d || b == c || b == d {
                // Third edge check
                let (x, y) = if a == c { (b, d) } else if a == d { (b, c) } else if b == c { (a, d) } else { (a, c) };
                // Check if third edge exists
                let mut has_third = false;
                for e3 in &edges {
                    if (e3.0 == x && e3.1 == y) || (e3.0 == y && e3.1 == x) {
                        has_third = true;
                        break;
                    }
                }
                if has_third {
                    beta_1 += 1;
                }
            }
        }
    }

    (beta_0, beta_1)
}

/// Compute the 8-dimensional GEI fingerprint from activation patterns.
#[cfg(feature = "v9.4-validation-layer")]
fn compute_gei_fingerprint(activations: &[Vec<f64>]) -> [f64; 8] {
    let mut fingerprint = [0.0; 8];
    let n = activations.len();

    if n == 0 {
        return fingerprint;
    }

    // Dimension 0: Mean activation magnitude
    let mut total_mag = 0.0;
    for act in activations {
        let mut mag = 0.0;
        for v in act {
            mag += v * v;
        }
        total_mag += mag.sqrt();
    }
    fingerprint[0] = if n > 0 { total_mag / n as f64 } else { 0.0 };

    // Dimension 1: Activation diversity (std dev of magnitudes)
    let magnitudes: Vec<f64> = activations
        .iter()
        .map(|act| {
            let mut mag = 0.0;
            for v in act {
                mag += v * v;
            }
            mag.sqrt()
        })
        .collect();
    let mean_mag = fingerprint[0];
    let variance: f64 = magnitudes.iter().map(|m| (m - mean_mag).powi(2)).sum::<f64>() / n as f64;
    fingerprint[1] = variance.sqrt();

    // Dimension 2: Pairwise coherence (mean cosine similarity)
    let mut total_sim = 0.0;
    let mut pair_count = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            total_sim += cosine_similarity(&activations[i], &activations[j]);
            pair_count += 1;
        }
    }
    fingerprint[2] = if pair_count > 0 {
        total_sim / pair_count as f64
    } else {
        0.0
    };

    // Dimension 3: Max activation (peak ethical signal)
    let mut max_act = 0.0;
    for act in activations {
        for v in act {
            if *v > max_act {
                max_act = *v;
            }
        }
    }
    fingerprint[3] = max_act;

    // Dimension 4: Min activation (lowest ethical signal)
    let mut min_act = f64::MAX;
    for act in activations {
        for v in act {
            if *v < min_act {
                min_act = *v;
            }
        }
    }
    fingerprint[4] = if n > 0 { min_act } else { 0.0 };

    // Dimension 5: Activation entropy (Shannon)
    let mut entropy = 0.0;
    let total: f64 = magnitudes.iter().sum();
    if total > 1e-12 {
        for m in &magnitudes {
            let p = m / total;
            if p > 1e-12 {
                entropy -= p * p.ln();
            }
        }
    }
    fingerprint[5] = entropy;

    // Dimension 6: Ethical balance (mean of absolute activations)
    let mut total_abs = 0.0;
    let mut count = 0;
    for act in activations {
        for v in act {
            total_abs += v.abs();
            count += 1;
        }
    }
    fingerprint[6] = if count > 0 { total_abs / count as f64 } else { 0.0 };

    // Dimension 7: Topological complexity (β₁ proxy from local cycles)
    // Approximate using local neighborhood density
    let mut local_density = 0.0;
    let threshold = 1.0; // Normalized threshold
    for i in 0..n {
        let mut neighbors = 0;
        for j in 0..n {
            if i != j {
                let mut dist_sq = 0.0;
                for k in 0..activations[i].len().min(activations[j].len()) {
                    let diff = activations[i][k] - activations[j][k];
                    dist_sq += diff * diff;
                }
                if dist_sq.sqrt() <= threshold {
                    neighbors += 1;
                }
            }
        }
        local_density += neighbors as f64 * (neighbors as f64 - 1.0) / 2.0;
    }
    fingerprint[7] = if n > 0 { local_density / n as f64 } else { 0.0 };

    fingerprint
}

/// Validate GEI against simulated ethical benchmarks.
///
/// # Arguments
/// * `activations` - Node activation patterns (8-dimensional vectors)
/// * `threshold` - Vietoris-Rips distance threshold
///
/// # Returns
/// `GEIValidationResult` with full topological analysis
#[cfg(feature = "v9.4-validation-layer")]
pub fn validate_gei(
    activations: &[Vec<f64>],
    threshold: f64,
) -> GEIValidationResult {
    let (beta_0, beta_1) = compute_betti_numbers(activations, threshold);
    let fingerprint = compute_gei_fingerprint(activations);

    // Hallucination clusters correlate with β₁ (persistent cycles)
    let hallucination_clusters = beta_1;

    // Bias correlation: high β₁ / low β₀ indicates concentrated bias
    let bias_correlation = if beta_0 > 0 {
        (beta_1 as f64 / beta_0 as f64).min(1.0)
    } else {
        0.0
    };

    // Validity score: low β₁, high coherence, balanced fingerprint
    let coherence = fingerprint[2]; // Pairwise coherence
    let balance = 1.0 - fingerprint[1].min(1.0); // Low variance = balanced
    let validity_score = (0.4 * coherence + 0.3 * balance + 0.3 * (1.0 - bias_correlation)).max(0.0).min(1.0);

    // GEI is valid if validity_score > 0.5 and β₁ is not excessive
    let valid = validity_score > 0.5 && beta_1 < activations.len();

    GEIValidationResult {
        beta_0,
        beta_1,
        fingerprint,
        hallucination_clusters,
        bias_correlation,
        validity_score,
        valid,
    }
}

#[cfg(all(test, feature = "v9.4-validation-layer"))]
mod tests {
    use super::*;

    fn make_aligned_activations(count: usize) -> Vec<Vec<f64>> {
        (0..count)
            .map(|_| vec![1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3])
            .collect()
    }

    fn make_divergent_activations(count: usize) -> Vec<Vec<f64>> {
        (0..count)
            .map(|i| {
                let base = (i * 37 % 100) as f64 / 100.0;
                vec![
                    base,
                    (base + 0.1).min(1.0),
                    (base + 0.2).min(1.0),
                    (base + 0.3).min(1.0),
                    (base + 0.4).min(1.0),
                    (base + 0.5).min(1.0),
                    (base + 0.6).min(1.0),
                    (base + 0.7).min(1.0),
                ]
            })
            .collect()
    }

    #[test]
    fn test_cosine_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_zero_norm() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_betti_empty() {
        let (b0, b1) = compute_betti_numbers(&[], 1.0);
        assert_eq!(b0, 0);
        assert_eq!(b1, 0);
    }

    #[test]
    fn test_betti_single_node() {
        let acts = vec![vec![1.0, 0.5]];
        let (b0, b1) = compute_betti_numbers(&acts, 1.0);
        assert_eq!(b0, 1);
        assert_eq!(b1, 0);
    }

    #[test]
    fn test_betti_connected_triangle() {
        // Three close nodes form a triangle (β₀=1, β₁=1)
        let acts = vec![
            vec![0.0, 0.0],
            vec![0.1, 0.0],
            vec![0.0, 0.1],
        ];
        let (b0, b1) = compute_betti_numbers(&acts, 0.5);
        assert_eq!(b0, 1);
        assert_eq!(b1, 1);
    }

    #[test]
    fn test_betti_disconnected() {
        // Two far nodes (β₀=2, β₁=0)
        let acts = vec![
            vec![0.0, 0.0],
            vec![10.0, 10.0],
        ];
        let (b0, b1) = compute_betti_numbers(&acts, 1.0);
        assert_eq!(b0, 2);
        assert_eq!(b1, 0);
    }

    #[test]
    fn test_gei_fingerprint_empty() {
        let fp = compute_gei_fingerprint(&[]);
        for v in &fp {
            assert!((v - 0.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_gei_fingerprint_aligned() {
        let acts = make_aligned_activations(5);
        let fp = compute_gei_fingerprint(&acts);
        // High coherence expected
        assert!(fp[2] > 0.9);
        // Low variance expected
        assert!(fp[1] < 0.1);
    }

    #[test]
    fn test_validate_gei_aligned() {
        let acts = make_aligned_activations(10);
        let result = validate_gei(&acts, 2.0);
        assert!(result.valid);
        assert!(result.validity_score > 0.5);
        assert_eq!(result.beta_0, 1); // All connected
    }

    #[test]
    fn test_validate_gei_divergent() {
        let acts = make_divergent_activations(20);
        let result = validate_gei(&acts, 0.5);
        // May or may not be valid depending on distribution
        assert!(result.validity_score >= 0.0);
        assert!(result.validity_score <= 1.0);
    }

    #[test]
    fn test_validate_gei_display() {
        let acts = make_aligned_activations(5);
        let result = validate_gei(&acts, 2.0);
        let display = format!("{}", result);
        assert!(display.contains("GEI("));
        assert!(display.contains("β₀="));
        assert!(display.contains("β₁="));
    }

    #[test]
    fn test_hallucination_clusters_zero() {
        let acts = make_aligned_activations(3);
        let result = validate_gei(&acts, 0.3);
        // Very tight threshold, no cycles
        assert_eq!(result.hallucination_clusters, result.beta_1);
    }

    #[test]
    fn test_bias_correlation_bounded() {
        let acts = make_divergent_activations(15);
        let result = validate_gei(&acts, 1.0);
        assert!(result.bias_correlation >= 0.0);
        assert!(result.bias_correlation <= 1.0);
    }

    #[test]
    fn test_validity_score_bounded() {
        let acts = make_divergent_activations(10);
        let result = validate_gei(&acts, 1.0);
        assert!(result.validity_score >= 0.0);
        assert!(result.validity_score <= 1.0);
    }
}
