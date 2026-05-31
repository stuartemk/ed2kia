//! Spectral Coherence — Sprint 68: Academic Formalization & Validation Layer
//!
//! Translates "Morphic Resonance" into measurable spectral graph theory metrics.
//! Computes eigenvalues of the graph Laplacian, federated gradient synchronization rate,
//! and cross-correlation coefficient to produce a `spectral_coherence_score`.
//!
//! # Key Concepts
//! - **Laplacian Eigenvalues**: The algebraic connectivity (λ₂) measures network robustness.
//! - **Synchronization Rate**: How quickly federated gradients converge across nodes.
//! - **Cross-Correlation**: Pairwise correlation of node activation patterns.

#[cfg(feature = "v9.4-validation-layer")]
use std::f64;

    const EPSILON: f64 = 1e-9;

    /// Result of spectral coherence analysis.
    #[derive(Debug, Clone)]
    pub struct SpectralCoherenceResult {
        /// Algebraic connectivity (second smallest Laplacian eigenvalue).
        pub algebraic_connectivity: f64,
        /// Federated gradient synchronization rate.
        pub sync_rate: f64,
        /// Cross-correlation coefficient across nodes.
        pub cross_correlation: f64,
        /// Composite spectral coherence score in [0, 1].
        pub coherence_score: f64,
    }

    impl SpectralCoherenceResult {
        /// Returns true when the network exhibits strong morphic resonance.
        pub fn is_resonant(&self) -> bool {
            self.coherence_score > 0.7
        }

        /// Returns true when the network is connected (algebraic connectivity > 0).
        pub fn is_connected(&self) -> bool {
            self.algebraic_connectivity > EPSILON
        }
    }

    /// Compute spectral coherence from adjacency matrix and node activations.
    ///
    /// # Arguments
    /// * `adjacency` — Square adjacency matrix (1.0 = connected, 0.0 = not connected).
    /// * `activations` — Per-node activation vectors for cross-correlation.
    ///
    /// # Returns
    /// `SpectralCoherenceResult` with all spectral metrics.
    pub fn compute_spectral_coherence(
        adjacency: &[Vec<f64>],
        activations: &[Vec<f64>],
    ) -> SpectralCoherenceResult {
        let n = adjacency.len();
        if n == 0 {
            return empty_result();
        }

        // Compute Laplacian eigenvalues (power iteration approximation for λ₂)
        let algebraic_conn = algebraic_connectivity(adjacency);

        // Compute synchronization rate from activations
        let sync = sync_rate(activations);

        // Compute cross-correlation
        let corr = cross_correlation(activations);

        // Composite score: weighted combination
        let score = 0.4 * algebraic_conn.min(1.0) + 0.3 * sync + 0.3 * corr.clamp(0.0, 1.0);

        SpectralCoherenceResult {
            algebraic_connectivity: algebraic_conn,
            sync_rate: sync,
            cross_correlation: corr,
            coherence_score: score.clamp(0.0, 1.0),
        }
    }

    /// Estimate algebraic connectivity (λ₂ of Laplacian).
    ///
    /// Uses Fiedler value approximation via degree-based heuristic for efficiency.
    /// For a connected graph, λ₂ > 0.
    pub fn algebraic_connectivity(adjacency: &[Vec<f64>]) -> f64 {
        let n = adjacency.len();
        if n <= 1 {
            return 0.0;
        }

        // Compute degree matrix
        let degrees: Vec<f64> = (0..n).map(|i| adjacency[i].iter().sum()).collect();

        // Build Laplacian L = D - A
        let mut laplacian = vec![vec![0.0; n]; n];
        for i in 0..n {
            laplacian[i][i] = degrees[i];
            for j in 0..n {
                if i != j {
                    laplacian[i][j] = -adjacency[i][j];
                }
            }
        }

        // Power iteration to find smallest non-zero eigenvalue
        // Start with random vector orthogonal to all-ones
        let mut v = vec![1.0; n];
        v[0] -= n as f64; // Make sum ≈ 0

        // Inverse power iteration (simplified: use Rayleigh quotient)
        for _ in 0..50 {
            // Matrix-vector multiply
            let mut lv = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    lv[i] += laplacian[i][j] * v[j];
                }
            }
            // Orthogonalize against all-ones vector
            let mean: f64 = lv.iter().sum::<f64>() / n as f64;
            for val in lv.iter_mut() {
                *val -= mean;
            }
            // Normalize
            let norm: f64 = lv.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < EPSILON {
                return 0.0;
            }
            for val in lv.iter_mut() {
                *val /= norm;
            }
            v = lv;
        }

        // Rayleigh quotient: v^T L v / v^T v
        let mut lv = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                lv[i] += laplacian[i][j] * v[j];
            }
        }
        let num: f64 = v.iter().zip(lv.iter()).map(|(a, b)| a * b).sum();
        let den: f64 = v.iter().map(|x| x * x).sum();
        if den < EPSILON {
            0.0
        } else {
            (num / den).max(0.0)
        }
    }

    /// Compute federated gradient synchronization rate.
    ///
    /// Measures how quickly node activations converge toward the mean.
    /// Returns value in [0, 1] where 1.0 = perfect synchronization.
    pub fn sync_rate(activations: &[Vec<f64>]) -> f64 {
        if activations.len() <= 1 {
            return 1.0;
        }
        let dim = activations[0].len();
        // Compute mean activation
        let mut mean = vec![0.0; dim];
        for a in activations {
            for (j, v) in a.iter().enumerate() {
                mean[j] += v;
            }
        }
        for v in mean.iter_mut() {
            *v /= activations.len() as f64;
        }
        // Compute mean squared deviation
        let mse: f64 = activations
            .iter()
            .map(|a| {
                a.iter()
                    .zip(mean.iter())
                    .map(|(av, mv)| (av - mv).powi(2))
                    .sum::<f64>()
            })
            .sum::<f64>()
            / activations.len() as f64;

        // Sync rate = 1 / (1 + mse) — bounded in (0, 1]
        1.0 / (1.0 + mse)
    }

    /// Compute average pairwise cross-correlation of activation vectors.
    ///
    /// Returns value in [-1, 1] where 1.0 = perfect positive correlation.
    pub fn cross_correlation(activations: &[Vec<f64>]) -> f64 {
        let mut sum = 0.0;
        let mut count = 0;
        for (i, a1) in activations.iter().enumerate() {
            for a2 in activations.iter().skip(i + 1) {
                let corr = pearson_correlation(a1, a2);
                sum += corr;
                count += 1;
            }
        }
        if count == 0 {
            0.0
        } else {
            sum / count as f64
        }
    }

    /// Compute Pearson correlation coefficient between two vectors.
    pub fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let mean_a: f64 = a.iter().sum::<f64>() / n;
        let mean_b: f64 = b.iter().sum::<f64>() / n;

        let mut cov = 0.0;
        let mut var_a = 0.0;
        let mut var_b = 0.0;

        for (av, bv) in a.iter().zip(b.iter()) {
            let da = av - mean_a;
            let db = bv - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }

        let denom = (var_a * var_b).sqrt();
        if denom < EPSILON {
            0.0
        } else {
            (cov / denom).clamp(-1.0, 1.0)
        }
    }

    fn empty_result() -> SpectralCoherenceResult {
        SpectralCoherenceResult {
            algebraic_connectivity: 0.0,
            sync_rate: 0.0,
            cross_correlation: 0.0,
            coherence_score: 0.0,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Fully connected 3-node graph.
        fn full_adjacency_3() -> Vec<Vec<f64>> {
            vec![
                vec![0.0, 1.0, 1.0],
                vec![1.0, 0.0, 1.0],
                vec![1.0, 1.0, 0.0],
            ]
        }

        /// Identical activations for all nodes.
        fn identical_activations() -> Vec<Vec<f64>> {
            vec![
                vec![1.0, 2.0, 3.0],
                vec![1.0, 2.0, 3.0],
                vec![1.0, 2.0, 3.0],
            ]
        }

        #[test]
        fn test_fully_connected_is_resonant() {
            let adj = full_adjacency_3();
            let act = identical_activations();
            let result = compute_spectral_coherence(&adj, &act);
            assert!(
                result.is_connected(),
                "Fully connected graph should be connected (λ₂={})",
                result.algebraic_connectivity
            );
            assert!(
                result.is_resonant(),
                "Identical activations in connected graph should be resonant (score={})",
                result.coherence_score
            );
        }

        #[test]
        fn test_disconnected_graph_not_connected() {
            let adj = vec![
                vec![0.0, 0.0, 0.0],
                vec![0.0, 0.0, 0.0],
                vec![0.0, 0.0, 0.0],
            ];
            let act = identical_activations();
            let result = compute_spectral_coherence(&adj, &act);
            assert!(
                !result.is_connected(),
                "Disconnected graph should not be connected"
            );
        }

        #[test]
        fn test_sync_rate_identical() {
            let act = identical_activations();
            let rate = sync_rate(&act);
            assert!(
                (rate - 1.0).abs() < 1e-9,
                "Identical activations should have sync rate 1.0 (got {})",
                rate
            );
        }

        #[test]
        fn test_sync_rate_divergent() {
            let act = vec![vec![0.0, 0.0], vec![10.0, 10.0], vec![20.0, 20.0]];
            let rate = sync_rate(&act);
            assert!(
                rate < 0.5,
                "Divergent activations should have low sync rate (got {})",
                rate
            );
        }

        #[test]
        fn test_pearson_identical() {
            let a = vec![1.0, 2.0, 3.0, 4.0];
            let b = vec![1.0, 2.0, 3.0, 4.0];
            let corr = pearson_correlation(&a, &b);
            assert!(
                (corr - 1.0).abs() < 1e-9,
                "Identical vectors should have correlation 1.0"
            );
        }

        #[test]
        fn test_pearson_anti_correlated() {
            let a = vec![1.0, 2.0, 3.0, 4.0];
            let b = vec![4.0, 3.0, 2.0, 1.0];
            let corr = pearson_correlation(&a, &b);
            assert!(
                (corr + 1.0).abs() < 1e-9,
                "Anti-correlated vectors should have correlation -1.0"
            );
        }

        #[test]
        fn test_empty_inputs() {
            let adj: Vec<Vec<f64>> = vec![];
            let act: Vec<Vec<f64>> = vec![];
            let result = compute_spectral_coherence(&adj, &act);
            assert_eq!(result.coherence_score, 0.0);
            assert!(!result.is_resonant());
            assert!(!result.is_connected());
        }

        #[test]
        fn test_coherence_score_bounded() {
            let adj = full_adjacency_3();
            let act = vec![
                vec![0.0, 0.0, 0.0],
                vec![100.0, 100.0, 100.0],
                vec![50.0, 50.0, 50.0],
            ];
            let result = compute_spectral_coherence(&adj, &act);
            assert!(
                result.coherence_score >= 0.0 && result.coherence_score <= 1.0,
                "Coherence score should be in [0, 1] (got {})",
                result.coherence_score
            );
        }
    }
