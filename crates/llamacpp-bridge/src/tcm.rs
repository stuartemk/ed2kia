//! TCM Z-Axis Metrics — Sprint 88: The Reality Engine & Empirical Proof Core
//!
//! Computes topological divergence metrics for SAE activation auditing.
//! Formula: `Z = (z_node - μ_centroid) / σ_spread`

/// TCM Z-axis metric result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TcmMetric {
    /// Node activation value.
    pub z_node: f64,
    /// Centroid mean.
    pub mu_centroid: f64,
    /// Spread standard deviation.
    pub sigma_spread: f64,
    /// Computed Z-score.
    pub z_score: f64,
}

/// Calculate the TCM Z-axis score.
///
/// Returns `0.0` when `sigma_spread` is zero to avoid division by zero.
pub fn calculate_tcm_z_score(z_node: f64, mu_centroid: f64, sigma_spread: f64) -> f64 {
    if sigma_spread.abs() < f64::EPSILON {
        return 0.0;
    }
    (z_node - mu_centroid) / sigma_spread
}

/// Compute TCM metrics for a given activation.
pub fn compute_tcm_metric(z_node: f64, mu_centroid: f64, sigma_spread: f64) -> TcmMetric {
    let z_score = calculate_tcm_z_score(z_node, mu_centroid, sigma_spread);
    TcmMetric {
        z_node,
        mu_centroid,
        sigma_spread,
        z_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcm_z_score_normal() {
        let z = calculate_tcm_z_score(2.0, 0.0, 1.0);
        assert!((z - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tcm_z_score_zero_spread() {
        let z = calculate_tcm_z_score(1.0, 0.0, 0.0);
        assert!((z - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tcm_z_score_identical() {
        let z = calculate_tcm_z_score(5.0, 5.0, 1.0);
        assert!((z - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_tcm_metric() {
        let metric = compute_tcm_metric(3.0, 1.0, 2.0);
        assert!((metric.z_score - 1.0).abs() < f64::EPSILON);
        assert!((metric.z_node - 3.0).abs() < f64::EPSILON);
    }
}
