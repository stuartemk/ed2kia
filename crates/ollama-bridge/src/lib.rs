//! Ollama/LM Studio Bridge — Sprint 86: The Epistemic Annihilation & Pure Engineering Core
//!
//! HTTP API wrapper intercepting Ollama/LM Studio local inferences,
//! injecting SAE audit pipeline + TCM Z-axis calculation.

mod config;
mod inference;
mod sae_audit;
mod tcm;

pub use config::*;
pub use inference::*;
pub use sae_audit::*;
pub use tcm::*;

/// TCM Z-axis formula: Z = (z_node - μ_centroid) / σ_spread
pub fn calculate_tcm_z_score(z_node: f64, mu_centroid: f64, sigma_spread: f64) -> f64 {
    if sigma_spread < 1e-9 {
        return 0.0;
    }
    (z_node - mu_centroid) / sigma_spread
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcm_z_score_normal() {
        let z = calculate_tcm_z_score(5.0, 3.0, 2.0);
        assert!((z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tcm_z_score_zero_spread() {
        let z = calculate_tcm_z_score(5.0, 3.0, 0.0);
        assert!((z - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_tcm_z_score_identical() {
        let z = calculate_tcm_z_score(3.0, 3.0, 2.0);
        assert!((z - 0.0).abs() < 1e-6);
    }
}
