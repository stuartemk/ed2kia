//! SAE Audit Pipeline — Sprint 88: The Reality Engine & Empirical Proof Core
//!
//! Sparse Autoencoder audit metrics for inference interception.

/// SAE audit result for a single inference.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaeAuditResult {
    /// Number of active neurons.
    pub active_neurons: usize,
    /// Total neurons in the SAE.
    pub total_neurons: usize,
    /// Sparsity ratio (active / total).
    pub sparsity: f64,
    /// L0 norm of activations.
    pub l0_norm: f64,
    /// L2 norm of activations.
    pub l2_norm: f64,
    /// Maximum activation value.
    pub max_activation: f64,
    /// Mean activation value.
    pub mean_activation: f64,
}

/// SAE auditor that computes metrics from activation vectors.
#[derive(Debug, Clone)]
pub struct SaeAuditor {
    /// Total number of neurons in the SAE.
    pub total_neurons: usize,
}

impl SaeAuditor {
    /// Create a new auditor with the given total neuron count.
    pub fn new(total_neurons: usize) -> Self {
        Self { total_neurons }
    }

    /// Audit an activation vector and return metrics.
    pub fn audit(&self, activations: &[f64]) -> SaeAuditResult {
        let active_neurons = activations.iter().filter(|&&a| a > 0.0).count();
        let sparsity = if self.total_neurons > 0 {
            active_neurons as f64 / self.total_neurons as f64
        } else {
            0.0
        };
        let l0_norm = active_neurons as f64;
        let l2_norm: f64 = activations.iter().map(|a| a * a).sum::<f64>().sqrt();
        let max_activation = activations.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mean_activation = if activations.is_empty() {
            0.0
        } else {
            activations.iter().sum::<f64>() / activations.len() as f64
        };

        SaeAuditResult {
            active_neurons,
            total_neurons: self.total_neurons,
            sparsity,
            l0_norm,
            l2_norm,
            max_activation,
            mean_activation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auditor_creation() {
        let auditor = SaeAuditor::new(16384);
        assert_eq!(auditor.total_neurons, 16384);
    }

    #[test]
    fn test_audit_sparsity() {
        let auditor = SaeAuditor::new(100);
        let activations = vec![1.0; 10];
        let result = auditor.audit(&activations);
        assert_eq!(result.active_neurons, 10);
        assert!((result.sparsity - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_audit_empty() {
        let auditor = SaeAuditor::new(100);
        let result = auditor.audit(&[]);
        assert_eq!(result.active_neurons, 0);
        assert!((result.sparsity - 0.0).abs() < f64::EPSILON);
    }
}
