//! TCM Z-axis metrics computation.

/// TCM Z-axis result.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TcmMetric {
    /// Raw z-node value from SAE activation.
    pub z_node: f64,
    /// Centroid mean (μ).
    pub mu_centroid: f64,
    /// Spread σ.
    pub sigma_spread: f64,
    /// Computed Z-score: (z_node - μ) / σ.
    pub z_score: f64,
}

impl TcmMetric {
    /// Compute TCM Z-score from raw activation and centroid parameters.
    pub fn compute(z_node: f64, mu_centroid: f64, sigma_spread: f64) -> Self {
        let z_score = if sigma_spread < 1e-9 {
            0.0
        } else {
            (z_node - mu_centroid) / sigma_spread
        };
        Self {
            z_node,
            mu_centroid,
            sigma_spread,
            z_score,
        }
    }

    /// Returns true if the Z-score indicates an anomaly (|Z| > threshold).
    pub fn is_anomaly(&self, threshold: f64) -> bool {
        self.z_score.abs() > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcm_normal() {
        let m = TcmMetric::compute(5.0, 3.0, 2.0);
        assert!((m.z_score - 1.0).abs() < 1e-6);
        assert!(!m.is_anomaly(2.0));
    }

    #[test]
    fn test_tcm_zero_spread() {
        let m = TcmMetric::compute(5.0, 3.0, 0.0);
        assert!((m.z_score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_tcm_anomaly() {
        let m = TcmMetric::compute(10.0, 0.0, 1.0);
        assert!(m.is_anomaly(2.0));
    }
}
