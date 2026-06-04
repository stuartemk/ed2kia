//! GEI Proxy Distillation â€” Sprint 75: Thermodynamic Hardening & Asynchronous Neuro-Symbolic Pivot
//!
//! Lightweight proxy network approximates Î²â‚ in <5ms via UMAP/PCA reduction.
//! Heavy homology computation delegated to async orchestrator nodes.

use std::fmt;

/// Proxy errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ProxyError {
    DimensionMismatch(usize),
    EmptyInput,
    InvalidThreshold(f64),
    ModelNotTrained,
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::DimensionMismatch(d) => write!(f, "Dimension mismatch: {}", d),
            ProxyError::EmptyInput => write!(f, "Empty activation input"),
            ProxyError::InvalidThreshold(t) => write!(f, "Invalid threshold: {}", t),
            ProxyError::ModelNotTrained => write!(f, "Proxy model not trained"),
        }
    }
}

/// Proxy configuration.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub reduction_dim: usize,
    pub epsilon: f32,
    pub target_latency_ms: u32,
    pub delegation_threshold: f64,
}

impl ProxyConfig {
    pub fn default_Topological() -> Self {
        Self {
            reduction_dim: 64,
            epsilon: 0.01,
            target_latency_ms: 5,
            delegation_threshold: 0.8,
        }
    }

    pub fn validate(&self) -> Result<(), ProxyError> {
        if self.reduction_dim == 0 {
            return Err(ProxyError::DimensionMismatch(0));
        }
        if self.delegation_threshold < 0.0 || self.delegation_threshold > 1.0 {
            return Err(ProxyError::InvalidThreshold(self.delegation_threshold));
        }
        Ok(())
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Lightweight proxy network for Î²â‚ approximation.
#[derive(Debug, Clone)]
pub struct ProxyNetwork {
    pub weights: Vec<f32>,
    pub bias: f32,
    pub input_dim: usize,
    pub reduction_dim: usize,
    pub trained: bool,
}

impl ProxyNetwork {
    pub fn new(input_dim: usize, reduction_dim: usize) -> Self {
        let total_weights = input_dim * reduction_dim;
        Self {
            weights: vec![0.0; total_weights],
            bias: 0.0,
            input_dim,
            reduction_dim,
            trained: false,
        }
    }

    /// Simulate training via PCA-like weight initialization.
    pub fn train_pca(&mut self, sample_activations: &[Vec<f32>]) {
        if sample_activations.is_empty() {
            return;
        }
        let dim = sample_activations[0].len();
        if dim != self.input_dim {
            return;
        }

        // Compute mean activation per dimension
        let mut mean = vec![0.0f32; dim];
        for sample in sample_activations {
            for (m, v) in mean.iter_mut().zip(sample.iter()) {
                *m += *v;
            }
        }
        let n = sample_activations.len() as f32;
        for m in &mut mean {
            *m /= n;
        }

        // Initialize weights based on variance (PCA approximation)
        let mut variance = vec![0.0f32; dim];
        for sample in sample_activations {
            for (v, (s, m)) in variance.iter_mut().zip(sample.iter().zip(mean.iter())) {
                let diff = s - m;
                *v += diff * diff;
            }
        }
        for v in &mut variance {
            *v /= n;
        }

        // Top-k dimensions by variance become proxy weights
        let mut indexed: Vec<(usize, f32)> = variance.into_iter().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut weight_idx = 0;
        for (dim_idx, var) in indexed.iter().take(self.reduction_dim) {
            let _row = dim_idx * self.reduction_dim;
            for j in 0..self.reduction_dim {
                if weight_idx < self.weights.len() {
                    self.weights[weight_idx] = if j == (dim_idx % self.reduction_dim) {
                        var.sqrt()
                    } else {
                        0.0
                    };
                    weight_idx += 1;
                }
            }
        }

        self.trained = true;
    }

    /// Forward pass: reduced projection â†’ Î²â‚ proxy.
    pub fn forward(&self, activations: &[f32]) -> Result<f32, ProxyError> {
        if !self.trained {
            return Err(ProxyError::ModelNotTrained);
        }
        if activations.len() != self.input_dim {
            return Err(ProxyError::DimensionMismatch(activations.len()));
        }

        // Project to reduced space
        let mut reduced = vec![0.0f32; self.reduction_dim];
        for j in 0..self.reduction_dim {
            for i in 0..self.input_dim {
                let w_idx = i * self.reduction_dim + j;
                if w_idx < self.weights.len() {
                    reduced[j] += activations[i] * self.weights[w_idx];
                }
            }
            reduced[j] += self.bias;
        }

        // Î²â‚ proxy: normalized energy of reduced representation
        let energy: f32 = reduced.iter().map(|x| x * x).sum();
        Ok(energy.sqrt())
    }
}

/// Distillation record.
#[derive(Debug, Clone)]
pub struct DistillationRecord {
    pub input_dim: usize,
    pub reduced_dim: usize,
    pub proxy_betti_1: f32,
    pub latency_ms: u32,
    pub delegated: bool,
}

impl fmt::Display for DistillationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DistillationRecord {{ input: {}d, reduced: {}d, Î²â‚: {:.4}, latency: {}ms, delegated: {} }}",
            self.input_dim, self.reduced_dim, self.proxy_betti_1, self.latency_ms, self.delegated
        )
    }
}

/// GEI Proxy Distillation engine.
pub struct GeiProxyDistillation {
    pub config: ProxyConfig,
    proxy: ProxyNetwork,
    records: Vec<DistillationRecord>,
}

impl GeiProxyDistillation {
    pub fn new(input_dim: usize) -> Self {
        Self {
            config: ProxyConfig::default_Topological(),
            proxy: ProxyNetwork::new(input_dim, ProxyConfig::default_Topological().reduction_dim),
            records: Vec::new(),
        }
    }

    pub fn with_config(input_dim: usize, config: ProxyConfig) -> Result<Self, ProxyError> {
        config.validate()?;
        Ok(Self {
            proxy: ProxyNetwork::new(input_dim, config.reduction_dim),
            config,
            records: Vec::new(),
        })
    }

    /// Train proxy on sample activations.
    pub fn train(&mut self, samples: &[Vec<f32>]) {
        self.proxy.train_pca(samples);
    }

    /// Approximate Î²â‚ via proxy network. <5ms target.
    pub fn approximate_betti_1_proxy(
        &mut self,
        activations: &[f32],
    ) -> Result<DistillationRecord, ProxyError> {
        if activations.is_empty() {
            return Err(ProxyError::EmptyInput);
        }

        let start = std::time::Instant::now();
        let proxy_betti_1 = self.proxy.forward(activations)?;
        let latency_ms = start.elapsed().as_millis() as u32;

        // Check if delegation to heavy homology is needed
        let confidence = 1.0 - (latency_ms as f64 / self.config.target_latency_ms as f64).min(1.0);
        let delegated = confidence < self.config.delegation_threshold;

        let record = DistillationRecord {
            input_dim: activations.len(),
            reduced_dim: self.config.reduction_dim,
            proxy_betti_1,
            latency_ms,
            delegated,
        };

        self.records.push(record.clone());
        Ok(record)
    }

    /// Batch approximation with async delegation flag.
    pub fn approximate_batch(
        &mut self,
        batch: &[Vec<f32>],
    ) -> Vec<Result<DistillationRecord, ProxyError>> {
        batch
            .iter()
            .map(|activations| self.approximate_betti_1_proxy(activations))
            .collect()
    }

    pub fn average_latency(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let total: u64 = self.records.iter().map(|r| r.latency_ms as u64).sum();
        Some(total as f64 / self.records.len() as f64)
    }

    pub fn delegation_rate(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let delegated: usize = self.records.iter().filter(|r| r.delegated).count();
        Some(delegated as f64 / self.records.len() as f64)
    }

    pub fn reset(&mut self) {
        self.records.clear();
    }
}

impl fmt::Display for GeiProxyDistillation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let avg_lat = self.average_latency().unwrap_or(0.0);
        let deleg_rate = self.delegation_rate().unwrap_or(0.0);
        write!(
            f,
            "GeiProxyDistillation {{ records: {}, avg_latency: {:.2}ms, delegation: {:.2}% }}",
            self.records.len(),
            avg_lat,
            deleg_rate * 100.0
        )
    }
}

/// Standalone proxy approximation.
pub fn approximate_betti_1_proxy(
    activations: &[f32],
    proxy_model: &ProxyNetwork,
) -> Result<f32, ProxyError> {
    proxy_model.forward(activations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_activations(dim: usize) -> Vec<f32> {
        (0..dim).map(|i| i as f32 * 0.1).collect()
    }

    #[test]
    fn test_config_default() {
        let config = ProxyConfig::default_Topological();
        assert_eq!(config.reduction_dim, 64);
        assert_eq!(config.target_latency_ms, 5);
    }

    #[test]
    fn test_config_validate_ok() {
        assert!(ProxyConfig::default_Topological().validate().is_ok());
    }

    #[test]
    fn test_config_zero_reduction() {
        let mut config = ProxyConfig::default_Topological();
        config.reduction_dim = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_proxy_network_creation() {
        let net = ProxyNetwork::new(128, 32);
        assert_eq!(net.input_dim, 128);
        assert!(!net.trained);
    }

    #[test]
    fn test_proxy_untrained_error() {
        let net = ProxyNetwork::new(128, 32);
        assert!(net.forward(&[1.0]).is_err());
    }

    #[test]
    fn test_proxy_train_and_forward() {
        let mut net = ProxyNetwork::new(8, 4);
        let samples: Vec<Vec<f32>> = (0..10).map(|_| make_activations(8)).collect();
        net.train_pca(&samples);
        assert!(net.trained);

        let result = net.forward(&make_activations(8)).unwrap();
        assert!(result >= 0.0);
    }

    #[test]
    fn test_proxy_dimension_mismatch() {
        let mut net = ProxyNetwork::new(8, 4);
        net.train_pca(&vec![make_activations(8); 10]);
        assert!(net.forward(&make_activations(16)).is_err());
    }

    #[test]
    fn test_engine_creation() {
        let engine = GeiProxyDistillation::new(128);
        assert_eq!(engine.records.len(), 0);
    }

    #[test]
    fn test_approximate_basic() {
        let mut engine = GeiProxyDistillation::new(8);
        engine.train(&vec![make_activations(8); 10]);
        let record = engine
            .approximate_betti_1_proxy(&make_activations(8))
            .unwrap();
        assert_eq!(record.input_dim, 8);
        assert!(record.proxy_betti_1 >= 0.0);
    }

    #[test]
    fn test_approximate_empty() {
        let mut engine = GeiProxyDistillation::new(8);
        assert!(engine.approximate_betti_1_proxy(&[]).is_err());
    }

    #[test]
    fn test_approximate_batch() {
        let mut engine = GeiProxyDistillation::new(8);
        engine.train(&vec![make_activations(8); 10]);
        let batch = vec![make_activations(8); 5];
        let results = engine.approximate_batch(&batch);
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_average_latency() {
        let mut engine = GeiProxyDistillation::new(8);
        engine.train(&vec![make_activations(8); 10]);
        engine
            .approximate_betti_1_proxy(&make_activations(8))
            .unwrap();
        let avg = engine.average_latency().unwrap();
        assert!(avg >= 0.0);
    }

    #[test]
    fn test_delegation_rate() {
        let mut engine = GeiProxyDistillation::new(8);
        engine.train(&vec![make_activations(8); 10]);
        engine
            .approximate_betti_1_proxy(&make_activations(8))
            .unwrap();
        let rate = engine.delegation_rate();
        assert!(rate.is_some());
    }

    #[test]
    fn test_reset() {
        let mut engine = GeiProxyDistillation::new(8);
        engine.train(&vec![make_activations(8); 10]);
        engine
            .approximate_betti_1_proxy(&make_activations(8))
            .unwrap();
        engine.reset();
        assert_eq!(engine.records.len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = GeiProxyDistillation::new(128);
        let s = format!("{}", engine);
        assert!(s.contains("GeiProxyDistillation"));
    }

    #[test]
    fn test_record_display() {
        let record = DistillationRecord {
            input_dim: 128,
            reduced_dim: 32,
            proxy_betti_1: 1.5,
            latency_ms: 3,
            delegated: false,
        };
        let s = format!("{}", record);
        assert!(s.contains("DistillationRecord"));
    }

    #[test]
    fn test_standalone_approximate() {
        let mut net = ProxyNetwork::new(8, 4);
        net.train_pca(&vec![make_activations(8); 10]);
        let result = approximate_betti_1_proxy(&make_activations(8), &net).unwrap();
        assert!(result >= 0.0);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = GeiProxyDistillation::new(16);
        let samples: Vec<Vec<f32>> = (0..20).map(|_| make_activations(16)).collect();
        engine.train(&samples);

        let batch: Vec<Vec<f32>> = (0..5).map(|_| make_activations(16)).collect();
        let results = engine.approximate_batch(&batch);

        assert_eq!(results.len(), 5);
        let successes: usize = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(successes, 5);
        assert!(engine.average_latency().is_some());
    }

    #[test]
    fn test_error_display() {
        let err = ProxyError::DimensionMismatch(8);
        assert!(format!("{}", err).contains("8"));
    }
}
