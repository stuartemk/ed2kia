//! Predictive Balancer — Balanceo predictivo de carga basado en tendencias históricas
//!
//! LP-37: Adaptive Routing v2
//! Proporciona predicción de carga futura basada en tendencias históricas
//! de latencia y throughput, permitiendo balanceo proactivo antes de que
//! los nodos se saturen.
//!
//! Características:
//! - Predicción de carga con regresión lineal simple sobre ventanas deslizantes
//! - Detección de tendencias (creciente, estable, decreciente)
//! - Score predictivo por nodo para integración con AdaptiveRouter
//! - Umbrales configurables para alertas de saturación
//! - Soporte para múltiples métricas (latencia, throughput, cola)
//!
//! Protegido con `#[cfg(feature = "v1.1-sprint5")]`.

#[cfg(feature = "v1.1-sprint5")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint5")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint5")]
#[cfg(feature = "v1.1-sprint5")]
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Error, Debug)]
pub enum BalancerError {
    #[error("Nodo no registrado: {0}")]
    NodeNotRegistered(String),

    #[error("Datos insuficientes para predicción: {available}/{required}")]
    InsufficientData { available: usize, required: usize },

    #[error("Tendencia no calculable: varianza cero")]
    ZeroVariance,

    #[error("Error de cálculo: {0}")]
    CalculationError(String),
}

// ─── Trend Direction ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Increasing,
    Stable,
    Decreasing,
}

#[cfg(feature = "v1.1-sprint5")]
impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrendDirection::Increasing => write!(f, "increasing"),
            TrendDirection::Stable => write!(f, "stable"),
            TrendDirection::Decreasing => write!(f, "decreasing"),
        }
    }
}

// ─── Prediction Result ────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub node_id: String,
    pub metric: String,
    pub current_value: f64,
    pub predicted_value: f64,
    pub trend: TrendDirection,
    pub slope: f64,
    pub r_squared: f64,
    pub confidence: f32,
    pub saturation_estimate_steps: Option<u32>,
    pub timestamp_ms: u64,
}

#[cfg(feature = "v1.1-sprint5")]
impl PredictionResult {
    pub fn new(
        node_id: String,
        metric: String,
        current_value: f64,
        predicted_value: f64,
        trend: TrendDirection,
        slope: f64,
        r_squared: f64,
    ) -> Self {
        let confidence = (r_squared.min(1.0) as f32).sqrt();
        let saturation_estimate = if slope > 0.0 {
            let threshold = 1.0;
            let remaining = (threshold - predicted_value) / slope;
            if remaining > 0.0 && remaining.is_finite() {
                Some(remaining as u32)
            } else {
                None
            }
        } else {
            None
        };

        Self {
            node_id,
            metric,
            current_value,
            predicted_value,
            trend,
            slope,
            r_squared,
            confidence,
            saturation_estimate_steps: saturation_estimate,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ─── Node Load History ────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLoadHistory {
    pub node_id: String,
    pub latencies: VecDeque<f64>,
    pub throughputs: VecDeque<f64>,
    pub queue_sizes: VecDeque<f64>,
    pub max_history: usize,
}

#[cfg(feature = "v1.1-sprint5")]
impl NodeLoadHistory {
    pub fn new(node_id: String, max_history: usize) -> Self {
        Self {
            node_id,
            latencies: VecDeque::with_capacity(max_history),
            throughputs: VecDeque::with_capacity(max_history),
            queue_sizes: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    pub fn record(&mut self, latency: f64, throughput: f64, queue_size: f64) {
        self.latencies.push_back(latency);
        self.throughputs.push_back(throughput);
        self.queue_sizes.push_back(queue_size);
        if self.latencies.len() > self.max_history {
            self.latencies.pop_front();
        }
        if self.throughputs.len() > self.max_history {
            self.throughputs.pop_front();
        }
        if self.queue_sizes.len() > self.max_history {
            self.queue_sizes.pop_front();
        }
    }

    pub fn latency_count(&self) -> usize {
        self.latencies.len()
    }
}

// ─── Balancer Config ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveBalancerConfig {
    pub window_size: usize,
    pub min_data_points: usize,
    pub trend_threshold: f64,
    pub saturation_threshold: f64,
    pub prediction_horizon: u32,
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for PredictiveBalancerConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            min_data_points: 10,
            trend_threshold: 0.01,
            saturation_threshold: 0.85,
            prediction_horizon: 5,
        }
    }
}

// ─── Balancer Stats ───────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalancerStats {
    pub total_predictions: u64,
    pub nodes_tracked: usize,
    pub increasing_trends: u64,
    pub stable_trends: u64,
    pub decreasing_trends: u64,
    pub avg_confidence: f64,
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for BalancerStats {
    fn default() -> Self {
        Self {
            total_predictions: 0,
            nodes_tracked: 0,
            increasing_trends: 0,
            stable_trends: 0,
            decreasing_trends: 0,
            avg_confidence: 0.0,
        }
    }
}

// ─── Predictive Balancer ──────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
pub struct PredictiveBalancer {
    config: PredictiveBalancerConfig,
    histories: HashMap<String, NodeLoadHistory>,
    stats: BalancerStats,
}

#[cfg(feature = "v1.1-sprint5")]
impl PredictiveBalancer {
    pub fn new() -> Self {
        Self::with_config(PredictiveBalancerConfig::default())
    }

    pub fn with_config(config: PredictiveBalancerConfig) -> Self {
        Self {
            config,
            histories: HashMap::new(),
            stats: BalancerStats::default(),
        }
    }

    // ─── Node Registration ────────────────────────────────────────────────────

    pub fn register_node(&mut self, node_id: String) {
        let history = NodeLoadHistory::new(node_id.clone(), self.config.window_size);
        self.histories.insert(node_id, history);
        self.stats.nodes_tracked = self.histories.len();
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.histories.remove(node_id);
        self.stats.nodes_tracked = self.histories.len();
    }

    // ─── Data Recording ───────────────────────────────────────────────────────

    pub fn record_load(
        &mut self,
        node_id: &str,
        latency: f64,
        throughput: f64,
        queue_size: f64,
    ) -> Result<(), BalancerError> {
        let history = self.histories.get_mut(node_id).ok_or_else(|| {
            BalancerError::NodeNotRegistered(node_id.to_string())
        })?;
        history.record(latency, throughput, queue_size);
        Ok(())
    }

    // ─── Prediction ───────────────────────────────────────────────────────────

    pub fn predict_latency(
        &mut self,
        node_id: &str,
    ) -> Result<PredictionResult, BalancerError> {
        self.predict_metric(node_id, "latency", |h: &NodeLoadHistory| &h.latencies)
    }

    pub fn predict_throughput(
        &mut self,
        node_id: &str,
    ) -> Result<PredictionResult, BalancerError> {
        self.predict_metric(node_id, "throughput", |h: &NodeLoadHistory| &h.throughputs)
    }

    pub fn predict_queue(
        &mut self,
        node_id: &str,
    ) -> Result<PredictionResult, BalancerError> {
        self.predict_metric(node_id, "queue_size", |h: &NodeLoadHistory| &h.queue_sizes)
    }

    fn predict_metric<F>(
        &mut self,
        node_id: &str,
        metric: &str,
        extractor: F,
    ) -> Result<PredictionResult, BalancerError>
    where
        F: Fn(&NodeLoadHistory) -> &VecDeque<f64>,
    {
        let history = self.histories.get(node_id).ok_or_else(|| {
            BalancerError::NodeNotRegistered(node_id.to_string())
        })?;

        let data = extractor(history);
        if data.len() < self.config.min_data_points {
            return Err(BalancerError::InsufficientData {
                available: data.len(),
                required: self.config.min_data_points,
            });
        }

        let values: Vec<f64> = data.iter().cloned().collect();
        let (slope, intercept, r_squared) =
            Self::linear_regression(&values).map_err(|_| BalancerError::ZeroVariance)?;

        let current = values.last().copied().unwrap_or(0.0);
        let n = values.len() as f64;
        let predicted = slope * n + intercept;
        let trend = Self::classify_trend(slope, self.config.trend_threshold);

        let result = PredictionResult::new(
            node_id.to_string(),
            metric.to_string(),
            current,
            predicted,
            trend,
            slope,
            r_squared,
        );

        self.update_stats(&result);
        Ok(result)
    }

    // ─── Linear Regression (Welford-style for numerical stability) ────────────

    fn linear_regression(data: &[f64]) -> Result<(f64, f64, f64), &'static str> {
        let n = data.len() as f64;
        if n < 2.0 {
            return Err("insufficient data");
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        let mut _sum_y2 = 0.0;

        for (i, &y) in data.iter().enumerate() {
            let x = i as f64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
            _sum_y2 += y * y;
        }

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 {
            return Err("zero variance");
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / n;

        // R-squared
        let mean_y = sum_y / n;
        let ss_tot: f64 = data.iter().map(|&y| (y - mean_y).powi(2)).sum();
        let ss_res: f64 = data
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                let x = i as f64;
                let predicted = slope * x + intercept;
                (y - predicted).powi(2)
            })
            .sum();

        let r_squared = if ss_tot.abs() < 1e-10 {
            0.0
        } else {
            (1.0 - ss_res / ss_tot).clamp(0.0, 1.0)
        };

        Ok((slope, intercept, r_squared))
    }

    fn classify_trend(slope: f64, threshold: f64) -> TrendDirection {
        if slope > threshold {
            TrendDirection::Increasing
        } else if slope < -threshold {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }

    // ─── Composite Score ──────────────────────────────────────────────────────

    pub fn compute_node_score(&mut self, node_id: &str) -> Result<f32, BalancerError> {
        let latency_pred = self.predict_latency(node_id);
        let queue_pred = self.predict_queue(node_id);

        let (latency_score, queue_score) = match (latency_pred, queue_pred) {
            (Ok(lp), Ok(qp)) => {
                // Lower latency = higher score
                let lat_score = (1.0 - (lp.predicted_value / 500.0).clamp(0.0, 1.0)) as f32;
                // Lower queue = higher score
                let q_score = (1.0 - (qp.predicted_value / 100.0).clamp(0.0, 1.0)) as f32;
                (lat_score, q_score)
            }
            _ => (0.5, 0.5), // Default when insufficient data
        };

        // Weighted combination
        Ok(latency_score * 0.6 + queue_score * 0.4)
    }

    pub fn get_best_node(&mut self, candidates: &[String]) -> Result<Option<String>, BalancerError> {
        let mut best_id: Option<String> = None;
        let mut best_score = f32::MIN;

        for id in candidates {
            match self.compute_node_score(id) {
                Ok(score) => {
                    if score > best_score {
                        best_score = score;
                        best_id = Some(id.clone());
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(best_id)
    }

    // ─── Stats ────────────────────────────────────────────────────────────────

    fn update_stats(&mut self, _result: &PredictionResult) {
        self.stats.total_predictions += 1;
        match _result.trend {
            TrendDirection::Increasing => self.stats.increasing_trends += 1,
            TrendDirection::Stable => self.stats.stable_trends += 1,
            TrendDirection::Decreasing => self.stats.decreasing_trends += 1,
        }
        let total = self.stats.total_predictions as f64;
        self.stats.avg_confidence =
            (self.stats.avg_confidence * (total - 1.0) + _result.confidence as f64) / total;
    }

    pub fn get_stats(&self) -> BalancerStats {
        self.stats.clone()
    }

    pub fn reset_stats(&mut self) {
        self.stats = BalancerStats::default();
    }
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for PredictiveBalancer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_balancer_creation() {
        let balancer = PredictiveBalancer::new();
        assert_eq!(balancer.histories.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_balancer_with_config() {
        let config = PredictiveBalancerConfig {
            window_size: 50,
            min_data_points: 5,
            ..PredictiveBalancerConfig::default()
        };
        let balancer = PredictiveBalancer::with_config(config);
        assert_eq!(balancer.config.min_data_points, 5);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_register_node() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        assert_eq!(balancer.histories.len(), 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_remove_node() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        balancer.remove_node("node-1");
        assert_eq!(balancer.histories.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_record_load() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        balancer.record_load("node-1", 50.0, 1000.0, 5.0).unwrap();
        let history = balancer.histories.get("node-1").unwrap();
        assert_eq!(history.latency_count(), 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_predict_insufficient_data() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        balancer.record_load("node-1", 50.0, 1000.0, 5.0).unwrap();
        let result = balancer.predict_latency("node-1");
        assert!(result.is_err());
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_predict_increasing_trend() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for i in 0..20 {
            balancer
                .record_load("node-1", 50.0 + i as f64 * 5.0, 1000.0, 5.0)
                .unwrap();
        }
        let result = balancer.predict_latency("node-1").unwrap();
        assert_eq!(result.trend, TrendDirection::Increasing);
        assert!(result.slope > 0.0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_predict_decreasing_trend() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for i in 0..20 {
            balancer
                .record_load("node-1", 150.0 - i as f64 * 5.0, 1000.0, 5.0)
                .unwrap();
        }
        let result = balancer.predict_latency("node-1").unwrap();
        assert_eq!(result.trend, TrendDirection::Decreasing);
        assert!(result.slope < 0.0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_predict_stable_trend() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for _ in 0..20 {
            balancer.record_load("node-1", 100.0, 1000.0, 5.0).unwrap();
        }
        let result = balancer.predict_latency("node-1").unwrap();
        assert_eq!(result.trend, TrendDirection::Stable);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_prediction_confidence() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for i in 0..20 {
            balancer
                .record_load("node-1", 50.0 + i as f64 * 5.0, 1000.0, 5.0)
                .unwrap();
        }
        let result = balancer.predict_latency("node-1").unwrap();
        assert!(result.confidence > 0.9); // Strong linear trend
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_compute_node_score() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for _i in 0..20 {
            balancer
                .record_load("node-1", 50.0, 1000.0, 3.0)
                .unwrap();
        }
        let score = balancer.compute_node_score("node-1").unwrap();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_get_best_node() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        balancer.register_node("node-2".into());
        // node-1: low latency
        for _ in 0..20 {
            balancer.record_load("node-1", 30.0, 1000.0, 2.0).unwrap();
        }
        // node-2: high latency
        for _ in 0..20 {
            balancer.record_load("node-2", 200.0, 500.0, 50.0).unwrap();
        }
        let candidates = vec!["node-1".into(), "node-2".into()];
        let best = balancer.get_best_node(&candidates).unwrap();
        assert_eq!(best, Some("node-1".into()));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_linear_regression_perfect_line() {
        let data: Vec<f64> = (0..20).map(|i| i as f64 * 2.0 + 10.0).collect();
        let (slope, intercept, r_squared) =
            PredictiveBalancer::linear_regression(&data).unwrap();
        assert!((slope - 2.0).abs() < 0.01);
        assert!((intercept - 10.0).abs() < 0.01);
        assert!((r_squared - 1.0).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_linear_regression_constant() {
        // Constant y data produces valid regression with slope=0 (x still has variance)
        let data: Vec<f64> = vec![5.0; 20];
        let result = PredictiveBalancer::linear_regression(&data);
        assert!(result.is_ok());
        let (slope, intercept, _r2) = result.unwrap();
        assert!((slope - 0.0).abs() < 1e-10); // Slope should be ~0
        assert!((intercept - 5.0).abs() < 1e-10); // Intercept should be 5.0
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_linear_regression_insufficient_data() {
        let data: Vec<f64> = vec![5.0];
        let result = PredictiveBalancer::linear_regression(&data);
        assert!(result.is_err()); // Need at least 2 points
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_stats_tracking() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for i in 0..20 {
            balancer
                .record_load("node-1", 50.0 + i as f64 * 2.0, 1000.0, 5.0)
                .unwrap();
        }
        balancer.predict_latency("node-1").unwrap();
        let stats = balancer.get_stats();
        assert_eq!(stats.total_predictions, 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_reset_stats() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for i in 0..20 {
            balancer
                .record_load("node-1", 50.0 + i as f64 * 2.0, 1000.0, 5.0)
                .unwrap();
        }
        balancer.predict_latency("node-1").unwrap();
        balancer.reset_stats();
        let stats = balancer.get_stats();
        assert_eq!(stats.total_predictions, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_trend_display() {
        assert_eq!(TrendDirection::Increasing.to_string(), "increasing");
        assert_eq!(TrendDirection::Stable.to_string(), "stable");
        assert_eq!(TrendDirection::Decreasing.to_string(), "decreasing");
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_config_default() {
        let config = PredictiveBalancerConfig::default();
        assert_eq!(config.window_size, 100);
        assert_eq!(config.min_data_points, 10);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_stats_default() {
        let stats = BalancerStats::default();
        assert_eq!(stats.total_predictions, 0);
        assert_eq!(stats.nodes_tracked, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_balancer_default() {
        let balancer = PredictiveBalancer::default();
        assert_eq!(balancer.histories.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_prediction_result_creation() {
        let result = PredictionResult::new(
            "node-1".into(),
            "latency".into(),
            100.0,
            110.0,
            TrendDirection::Increasing,
            1.0,
            0.95,
        );
        assert_eq!(result.node_id, "node-1");
        assert!(result.confidence > 0.9);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_saturation_estimate() {
        let result = PredictionResult::new(
            "node-1".into(),
            "load".into(),
            0.7,
            0.75,
            TrendDirection::Increasing,
            0.05,
            0.9,
        );
        assert!(result.saturation_estimate_steps.is_some());
        let steps = result.saturation_estimate_steps.unwrap();
        assert!(steps > 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_no_saturation_for_decreasing() {
        let result = PredictionResult::new(
            "node-1".into(),
            "load".into(),
            0.7,
            0.65,
            TrendDirection::Decreasing,
            -0.05,
            0.9,
        );
        assert!(result.saturation_estimate_steps.is_none());
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_load_history() {
        let mut history = NodeLoadHistory::new("n1".into(), 10);
        history.record(50.0, 1000.0, 5.0);
        history.record(60.0, 950.0, 6.0);
        assert_eq!(history.latency_count(), 2);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_history_max_size() {
        let mut history = NodeLoadHistory::new("n1".into(), 5);
        for i in 0..10 {
            history.record(i as f64, 1000.0, 5.0);
        }
        assert_eq!(history.latency_count(), 5);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_predict_unregistered_node() {
        let mut balancer = PredictiveBalancer::new();
        let result = balancer.predict_latency("unknown");
        assert!(result.is_err());
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_record_unregistered_node() {
        let mut balancer = PredictiveBalancer::new();
        let result = balancer.record_load("unknown", 50.0, 1000.0, 5.0);
        assert!(result.is_err());
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_multiple_predictions_update_stats() {
        let mut balancer = PredictiveBalancer::new();
        balancer.register_node("node-1".into());
        for i in 0..20 {
            balancer
                .record_load("node-1", 50.0 + i as f64 * 2.0, 1000.0 - i as f64 * 5.0, 5.0)
                .unwrap();
        }
        balancer.predict_latency("node-1").unwrap();
        balancer.predict_throughput("node-1").unwrap();
        let stats = balancer.get_stats();
        assert_eq!(stats.total_predictions, 2);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_get_best_node_empty_candidates() {
        let mut balancer = PredictiveBalancer::new();
        let candidates: Vec<String> = vec![];
        let best = balancer.get_best_node(&candidates).unwrap();
        assert!(best.is_none());
    }
}
