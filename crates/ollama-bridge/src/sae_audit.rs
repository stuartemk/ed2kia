//! SAE audit pipeline — Intercepts model activations and computes sparse metrics.

use serde::{Deserialize, Serialize};

/// Result of an SAE audit pass on a single inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaeAuditResult {
    /// Number of active neurons in the SAE layer.
    pub active_neurons: usize,
    /// Total neurons in the SAE layer.
    pub total_neurons: usize,
    /// Sparsity ratio: active / total.
    pub sparsity: f64,
    /// L0 norm of the activation vector.
    pub l0_norm: f64,
    /// L2 norm of the activation vector.
    pub l2_norm: f64,
    /// Maximum activation value.
    pub max_activation: f64,
    /// Mean activation value.
    pub mean_activation: f64,
}

impl SaeAuditResult {
    /// Create a new audit result from raw activation statistics.
    pub fn new(
        active_neurons: usize,
        total_neurons: usize,
        l0_norm: f64,
        l2_norm: f64,
        max_activation: f64,
        mean_activation: f64,
    ) -> Self {
        let sparsity = if total_neurons > 0 {
            active_neurons as f64 / total_neurons as f64
        } else {
            0.0
        };
        Self {
            active_neurons,
            total_neurons,
            sparsity,
            l0_norm,
            l2_norm,
            max_activation,
            mean_activation,
        }
    }

    /// Returns true if sparsity exceeds the threshold (indicating over-pruning).
    pub fn is_over_pruned(&self, threshold: f64) -> bool {
        self.sparsity > threshold
    }

    /// Returns true if sparsity is below the threshold (indicating under-utilization).
    pub fn is_under_utilized(&self, threshold: f64) -> bool {
        self.sparsity < threshold
    }
}

/// Lightweight SAE audit that runs on intercepted activations.
///
/// In production, this would interface with the actual SAE crate.
/// For the bridge, we provide a stub that can be replaced with real SAE inference.
pub struct SaeAuditor {
    pub total_neurons: usize,
}

impl SaeAuditor {
    pub fn new(total_neurons: usize) -> Self {
        Self { total_neurons }
    }

    /// Audit a vector of activations.
    pub fn audit(&self, activations: &[f64]) -> SaeAuditResult {
        let active_neurons = activations.iter().filter(|&&a| a > 1e-6).count();
        let l0_norm = active_neurons as f64;
        let l2_norm: f64 = activations.iter().map(|a| a * a).sum::<f64>().sqrt();
        let max_activation = activations
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or(0.0);
        let mean_activation = if activations.is_empty() {
            0.0
        } else {
            activations.iter().sum::<f64>() / activations.len() as f64
        };
        SaeAuditResult::new(
            active_neurons,
            self.total_neurons,
            l0_norm,
            l2_norm,
            max_activation,
            mean_activation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_basic() {
        let auditor = SaeAuditor::new(100);
        let activations = vec![0.0; 90];
        let result = auditor.audit(&activations);
        assert_eq!(result.active_neurons, 0);
        assert!((result.sparsity - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_audit_active() {
        let auditor = SaeAuditor::new(100);
        let activations: Vec<f64> = (0..100).map(|i| if i < 10 { 1.0 } else { 0.0 }).collect();
        let result = auditor.audit(&activations);
        assert_eq!(result.active_neurons, 10);
        assert!((result.sparsity - 0.1).abs() < 1e-6);
    }
}
