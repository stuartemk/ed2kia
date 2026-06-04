//! Topology-Ethics Reframe â€” Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! GEI redefinido explÃ­citamente como *proxy de estabilidad estructural & detecciÃ³n de anomalÃ­as*.
//! Ã‰tica vÃ­a guardrails explÃ­citos + benchmarks auditados + SCT-Z calibrado.
//!
//! **Pivot ArquitectÃ³nico:** TopologÃ­a â‰  Ã‰tica (error de categorÃ­a corregido).
//! GEI como mÃ©trica de inestabilidad/anomalÃ­a, no orÃ¡culo moral directo.

use std::fmt;

/// Error types for Topology-Ethics Reframe
#[derive(Debug, Clone, PartialEq)]
pub enum ReframeError {
    /// Invalid GEI proxy value
    InvalidGeiProxy(f32),
    /// Benchmark scores empty
    EmptyBenchmarks,
    /// SCT-Z out of range
    InvalidSctZ(f64),
    /// Guardrail violation
    GuardrailViolation(String),
    /// Anomaly score overflow
    Overflow,
}

impl fmt::Display for ReframeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReframeError::InvalidGeiProxy(v) => write!(f, "Invalid GEI proxy: {}", v),
            ReframeError::EmptyBenchmarks => write!(f, "Benchmark scores array is empty"),
            ReframeError::InvalidSctZ(v) => write!(f, "SCT-Z out of range: {}", v),
            ReframeError::GuardrailViolation(msg) => write!(f, "Guardrail violation: {}", msg),
            ReframeError::Overflow => write!(f, "Anomaly score computation overflow"),
        }
    }
}

/// Configuration for the reframe mapper
#[derive(Debug, Clone)]
pub struct ReframeConfig {
    /// Anomaly threshold for GEI proxy
    pub anomaly_threshold: f32,
    /// SCT-Z calibration weights: [fairness, safety, interpretability, conflict]
    pub sct_weights: [f64; 4],
    /// Maximum benchmark deviation before guardrail trigger
    pub max_benchmark_deviation: f64,
    /// Enable explicit ethical guardrails
    pub enable_guardrails: bool,
    /// Number of benchmark dimensions
    pub benchmark_dims: usize,
}

impl ReframeConfig {
    pub fn default_topological() -> Self {
        Self {
            anomaly_threshold: 0.7,
            sct_weights: [0.3, 0.3, 0.25, 0.15],
            max_benchmark_deviation: 0.2,
            enable_guardrails: true,
            benchmark_dims: 8,
        }
    }

    pub fn validate(&self) -> Result<(), ReframeError> {
        if self.anomaly_threshold < 0.0 || self.anomaly_threshold > 1.0 {
            return Err(ReframeError::InvalidGeiProxy(self.anomaly_threshold));
        }
        let weight_sum: f64 = self.sct_weights.iter().sum();
        if (weight_sum - 1.0).abs() > 0.01 {
            return Err(ReframeError::InvalidSctZ(weight_sum));
        }
        if self.max_benchmark_deviation < 0.0 || self.max_benchmark_deviation > 1.0 {
            return Err(ReframeError::InvalidSctZ(self.max_benchmark_deviation));
        }
        Ok(())
    }
}

impl Default for ReframeConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

/// Anomaly record from GEI mapping
#[derive(Debug, Clone)]
pub struct AnomalyRecord {
    pub timestamp_ms: u64,
    pub gei_proxy: f32,
    pub anomaly_score: f64,
    pub sct_z: f64,
    pub guardrail_triggered: bool,
    pub benchmark_deviation: f64,
}

impl fmt::Display for AnomalyRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AnomalyRecord {{ gei: {:.4}, anomaly: {:.4}, sct_z: {:.4}, guardrail: {}, dev: {:.4}, ts: {} }}",
            self.gei_proxy, self.anomaly_score, self.sct_z, self.guardrail_triggered, self.benchmark_deviation, self.timestamp_ms
        )
    }
}

/// Explicit ethical guardrail definition
#[derive(Debug, Clone)]
pub struct Guardrail {
    pub name: String,
    pub min_value: f64,
    pub max_value: f64,
    pub enforced: bool,
}

impl Guardrail {
    pub fn new(name: String, min_value: f64, max_value: f64) -> Self {
        Self {
            name,
            min_value,
            max_value,
            enforced: true,
        }
    }

    pub fn check(&self, value: f64) -> bool {
        if !self.enforced {
            return true;
        }
        value >= self.min_value && value <= self.max_value
    }
}

impl fmt::Display for Guardrail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Guardrail {{ name: {}, range: [{}, {}], enforced: {} }}",
            self.name, self.min_value, self.max_value, self.enforced
        )
    }
}

/// Topology-Ethics Reframe Mapper
pub struct TopologyEthicsReframe {
    config: ReframeConfig,
    guardrails: Vec<Guardrail>,
    history: Vec<AnomalyRecord>,
    baseline_benchmarks: Vec<f64>,
}

impl TopologyEthicsReframe {
    pub fn new() -> Self {
        let mut self_ref = Self {
            config: ReframeConfig::default_topological(),
            guardrails: Vec::new(),
            history: Vec::new(),
            baseline_benchmarks: vec![0.5; 8],
        };
        self_ref.init_default_guardrails();
        self_ref
    }

    pub fn with_config(config: ReframeConfig) -> Result<Self, ReframeError> {
        config.validate()?;
        let mut self_ref = Self {
            config,
            guardrails: Vec::new(),
            history: Vec::new(),
            baseline_benchmarks: vec![0.5; 8],
        };
        self_ref.init_default_guardrails();
        Ok(self_ref)
    }

    fn init_default_guardrails(&mut self) {
        self.guardrails
            .push(Guardrail::new("fairness".to_string(), 0.3, 1.0));
        self.guardrails
            .push(Guardrail::new("safety".to_string(), 0.5, 1.0));
        self.guardrails
            .push(Guardrail::new("interpretability".to_string(), 0.2, 1.0));
    }

    /// Set baseline benchmarks
    pub fn set_benchmarks(&mut self, benchmarks: &[f64]) {
        self.baseline_benchmarks = benchmarks.to_vec();
    }

    /// Map GEI proxy to anomaly score
    pub fn map_gei_to_anomaly(
        &mut self,
        gei_proxy: f32,
        benchmark_scores: &[f64],
        timestamp_ms: u64,
    ) -> Result<AnomalyRecord, ReframeError> {
        if gei_proxy < 0.0 || gei_proxy > 1.0 {
            return Err(ReframeError::InvalidGeiProxy(gei_proxy));
        }
        if benchmark_scores.is_empty() {
            return Err(ReframeError::EmptyBenchmarks);
        }

        // Compute anomaly score from GEI proxy
        let anomaly_score = Self::compute_anomaly_score(gei_proxy, &self.baseline_benchmarks);

        // Compute SCT-Z calibrated score
        let sct_z = Self::compute_sct_z(benchmark_scores, &self.config.sct_weights);

        // Check benchmark deviation
        let deviation =
            Self::compute_benchmark_deviation(benchmark_scores, &self.baseline_benchmarks);

        // Check guardrails
        let mut guardrail_triggered = false;
        if self.config.enable_guardrails {
            for gr in &self.guardrails {
                if !gr.check(sct_z) {
                    guardrail_triggered = true;
                    break;
                }
            }
            if deviation > self.config.max_benchmark_deviation {
                guardrail_triggered = true;
            }
        }

        let record = AnomalyRecord {
            timestamp_ms,
            gei_proxy,
            anomaly_score,
            sct_z,
            guardrail_triggered,
            benchmark_deviation: deviation,
        };

        self.history.push(record.clone());
        Ok(record)
    }

    /// Compute anomaly score from GEI proxy + baseline deviation
    pub fn compute_anomaly_score(gei_proxy: f32, baseline: &[f64]) -> f64 {
        let baseline_mean: f64 = if baseline.is_empty() {
            0.5
        } else {
            baseline.iter().sum::<f64>() / baseline.len() as f64
        };

        // Anomaly = |gei_proxy - baseline_mean|, scaled to [0, 1]
        let deviation = (gei_proxy as f64 - baseline_mean).abs();
        (deviation * 2.0).min(1.0)
    }

    /// Compute SCT-Z calibrated axis
    pub fn compute_sct_z(scores: &[f64], weights: &[f64; 4]) -> f64 {
        if scores.len() < 4 {
            return 0.0;
        }
        let fairness = scores[0].min(1.0).max(0.0);
        let safety = scores[1].min(1.0).max(0.0);
        let interpretability = scores[2].min(1.0).max(0.0);
        let conflict = scores[3].min(1.0).max(0.0);

        let z = weights[0] * fairness + weights[1] * safety + weights[2] * interpretability
            - weights[3] * conflict;

        z.min(1.0).max(0.0)
    }

    /// Compute benchmark deviation from baseline
    pub fn compute_benchmark_deviation(current: &[f64], baseline: &[f64]) -> f64 {
        let len = current.len().min(baseline.len());
        if len == 0 {
            return 0.0;
        }
        let sum: f64 = current[..len]
            .iter()
            .zip(baseline[..len].iter())
            .map(|(c, b)| (c - b).abs())
            .sum();
        sum / len as f64
    }

    /// Add custom guardrail
    pub fn add_guardrail(&mut self, guardrail: Guardrail) {
        self.guardrails.push(guardrail);
    }

    /// Check if all guardrails pass for a given SCT-Z
    pub fn check_all_guardrails(&self, sct_z: f64) -> bool {
        if !self.config.enable_guardrails {
            return true;
        }
        self.guardrails.iter().all(|gr| gr.check(sct_z))
    }

    /// Get average anomaly score
    pub fn average_anomaly_score(&self) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }
        let sum: f64 = self.history.iter().map(|r| r.anomaly_score).sum();
        Some(sum / self.history.len() as f64)
    }

    /// Get guardrail trigger rate
    pub fn guardrail_trigger_rate(&self) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }
        let triggers = self
            .history
            .iter()
            .filter(|r| r.guardrail_triggered)
            .count();
        Some(triggers as f64 / self.history.len() as f64)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.history.clear();
    }
}

impl Default for TopologyEthicsReframe {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TopologyEthicsReframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TopologyEthicsReframe {{ records: {}, guardrails: {}, anomaly_threshold: {:.3} }}",
            self.history.len(),
            self.guardrails.len(),
            self.config.anomaly_threshold
        )
    }
}

/// Public function: Map GEI to anomaly score
pub fn map_gei_to_anomaly_score(gei_proxy: f32, benchmark_scores: &[f64]) -> f64 {
    if benchmark_scores.is_empty() {
        return 0.0;
    }
    let baseline_mean: f64 = benchmark_scores.iter().sum::<f64>() / benchmark_scores.len() as f64;
    let deviation = (gei_proxy as f64 - baseline_mean).abs();
    (deviation * 2.0).min(1.0)
}

/// GEI cosine similarity for anomaly detection
pub fn gei_cosine_similarity(a: &[f64; 8], b: &[f64; 8]) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..8 {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom < 1e-10 {
        return 0.0;
    }
    dot / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ReframeConfig::default();
        assert!((config.sct_weights.iter().sum::<f64>() - 1.0).abs() < 0.01);
        assert!(config.enable_guardrails);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = ReframeConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = ReframeConfig {
            anomaly_threshold: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_mapper_creation() {
        let mapper = TopologyEthicsReframe::new();
        assert_eq!(mapper.history.len(), 0);
    }

    #[test]
    fn test_map_gei_to_anomaly() {
        let mut mapper = TopologyEthicsReframe::new();
        let scores = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        let result = mapper.map_gei_to_anomaly(0.5, &scores, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_map_invalid_gei() {
        let mut mapper = TopologyEthicsReframe::new();
        let result = mapper.map_gei_to_anomaly(1.5, &[0.5], 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_empty_benchmarks() {
        let mut mapper = TopologyEthicsReframe::new();
        let result = mapper.map_gei_to_anomaly(0.5, &[], 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_anomaly_score() {
        let baseline = vec![0.5; 8];
        let score = TopologyEthicsReframe::compute_anomaly_score(0.5, &baseline);
        assert!((score - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_anomaly_high_deviation() {
        let baseline = vec![0.0; 8];
        let score = TopologyEthicsReframe::compute_anomaly_score(1.0, &baseline);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_sct_z() {
        let scores = [1.0, 1.0, 1.0, 0.0];
        let weights = [0.3, 0.3, 0.25, 0.15];
        let z = TopologyEthicsReframe::compute_sct_z(&scores, &weights);
        assert!((z - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_compute_benchmark_deviation() {
        let current = [0.6, 0.7, 0.8, 0.9];
        let baseline = [0.5, 0.5, 0.5, 0.5];
        let dev = TopologyEthicsReframe::compute_benchmark_deviation(&current, &baseline);
        // Mean absolute deviation: (0.1 + 0.2 + 0.3 + 0.4) / 4 = 0.25
        assert!((dev - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_guardrail_check() {
        let gr = Guardrail::new("test".to_string(), 0.0, 1.0);
        assert!(gr.check(0.5));
        assert!(!gr.check(1.5));
    }

    #[test]
    fn test_guardrail_disabled() {
        let mut gr = Guardrail::new("test".to_string(), 0.0, 1.0);
        gr.enforced = false;
        assert!(gr.check(1.5));
    }

    #[test]
    fn test_average_anomaly_score() {
        let mut mapper = TopologyEthicsReframe::new();
        let scores = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        mapper.map_gei_to_anomaly(0.5, &scores, 1000).unwrap();
        mapper.map_gei_to_anomaly(0.8, &scores, 2000).unwrap();
        let avg = mapper.average_anomaly_score().unwrap();
        assert!(avg >= 0.0 && avg <= 1.0);
    }

    #[test]
    fn test_guardrail_trigger_rate() {
        let mut mapper = TopologyEthicsReframe::new();
        let scores = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        mapper.map_gei_to_anomaly(0.5, &scores, 1000).unwrap();
        let rate = mapper.guardrail_trigger_rate();
        assert!(rate.is_some());
    }

    #[test]
    fn test_reset() {
        let mut mapper = TopologyEthicsReframe::new();
        let scores = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        mapper.map_gei_to_anomaly(0.5, &scores, 1000).unwrap();
        mapper.reset();
        assert_eq!(mapper.history.len(), 0);
    }

    #[test]
    fn test_display() {
        let mapper = TopologyEthicsReframe::new();
        let s = format!("{}", mapper);
        assert!(s.contains("TopologyEthicsReframe"));
    }

    #[test]
    fn test_gei_cosine_similarity_identical() {
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!((gei_cosine_similarity(&a, &a) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_gei_cosine_similarity_zero() {
        let a = [0.0; 8];
        assert!((gei_cosine_similarity(&a, &a) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_map_gei_to_anomaly_score() {
        let scores = [0.5, 0.5, 0.5, 0.5];
        let anomaly = map_gei_to_anomaly_score(0.5, &scores);
        assert!((anomaly - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_full_workflow() {
        let mut mapper = TopologyEthicsReframe::new();
        mapper.set_benchmarks(&[0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);
        let scores = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        let record = mapper.map_gei_to_anomaly(0.5, &scores, 1000).unwrap();
        assert!(record.anomaly_score >= 0.0);
        assert!(record.sct_z >= 0.0);
        assert_eq!(mapper.history.len(), 1);
    }
}
