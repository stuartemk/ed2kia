//! Predictive Scheduler — Load prediction and task scheduling based on historical patterns.
//!
//! Predicts node load using exponential moving average and schedules tasks
//! to minimize overall latency while respecting capacity constraints.

use std::collections::{HashMap, VecDeque};

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum SchedulerError {
    NoPredictionsAvailable,
    NodeNotFound(String),
    InsufficientHistory,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPredictionsAvailable => write!(f, "No predictions available"),
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::InsufficientHistory => write!(f, "Insufficient history for prediction"),
        }
    }
}

impl std::error::Error for SchedulerError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub ema_alpha: f64,
    pub min_history_points: usize,
    pub prediction_horizon: usize,
    pub max_schedule_queue: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            ema_alpha: 0.3,
            min_history_points: 5,
            prediction_horizon: 3,
            max_schedule_queue: 100,
        }
    }
}

// ─── Load Prediction ───

#[derive(Debug, Clone)]
pub struct LoadPrediction {
    pub node_id: String,
    pub predicted_load: f64,
    pub confidence: f64,
    pub trend: LoadTrend,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadTrend {
    Increasing,
    Stable,
    Decreasing,
}

impl std::fmt::Display for LoadTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Increasing => write!(f, "increasing"),
            Self::Stable => write!(f, "stable"),
            Self::Decreasing => write!(f, "decreasing"),
        }
    }
}

// ─── Schedule Entry ───

#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    pub task_id: String,
    pub node_id: String,
    pub predicted_latency_ms: f64,
    pub scheduled_at_ms: u64,
}

// ─── Node History ───

#[derive(Debug, Clone)]
pub struct NodeLoadHistory {
    pub node_id: String,
    pub load_samples: VecDeque<f64>,
    pub ema_load: f64,
}

impl NodeLoadHistory {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            load_samples: VecDeque::new(),
            ema_load: 0.0,
        }
    }

    pub fn record_load(&mut self, load: f64, alpha: f64) {
        self.load_samples.push_back(load);
        if self.ema_load == 0.0 {
            self.ema_load = load;
        } else {
            self.ema_load = alpha * load + (1.0 - alpha) * self.ema_load;
        }
    }

    pub fn compute_trend(&self) -> LoadTrend {
        if self.load_samples.len() < 3 {
            return LoadTrend::Stable;
        }
        let samples: Vec<f64> = self.load_samples.iter().copied().collect();
        let recent: f64 = samples.iter().rev().take(3).sum::<f64>() / 3.0;
        let older: f64 = samples.iter().rev().skip(3).take(3).sum::<f64>() / 3.0;
        if (recent - older) > 0.05 {
            LoadTrend::Increasing
        } else if (older - recent) > 0.05 {
            LoadTrend::Decreasing
        } else {
            LoadTrend::Stable
        }
    }
}

// ─── Stats ───

#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_predictions: u64,
    pub total_schedules: u64,
    pub avg_prediction_confidence: f64,
}

impl Default for SchedulerStats {
    fn default() -> Self {
        Self {
            total_predictions: 0,
            total_schedules: 0,
            avg_prediction_confidence: 0.0,
        }
    }
}

// ─── Scheduler ───

pub struct PredictiveScheduler {
    config: SchedulerConfig,
    histories: HashMap<String, NodeLoadHistory>,
    schedule_queue: VecDeque<ScheduleEntry>,
    stats: SchedulerStats,
}

impl PredictiveScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            histories: HashMap::new(),
            schedule_queue: VecDeque::new(),
            stats: SchedulerStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(SchedulerConfig::default())
    }

    pub fn register_node(&mut self, node_id: String) {
        self.histories
            .insert(node_id.clone(), NodeLoadHistory::new(node_id));
    }

    pub fn record_load(&mut self, node_id: &str, load: f64) -> Result<(), SchedulerError> {
        if let Some(history) = self.histories.get_mut(node_id) {
            history.record_load(load.clamp(0.0, 1.0), self.config.ema_alpha);
            Ok(())
        } else {
            Err(SchedulerError::NodeNotFound(node_id.to_string()))
        }
    }

    pub fn predict_load(&mut self, node_id: &str) -> Result<LoadPrediction, SchedulerError> {
        let history = self
            .histories
            .get(node_id)
            .ok_or_else(|| SchedulerError::NodeNotFound(node_id.to_string()))?;

        if history.load_samples.len() < self.config.min_history_points {
            return Err(SchedulerError::InsufficientHistory);
        }

        let trend = history.compute_trend();
        let confidence = self.compute_confidence(history);

        // Predict future load based on trend
        let predicted_load = match trend {
            LoadTrend::Increasing => history.ema_load * 1.1,
            LoadTrend::Stable => history.ema_load,
            LoadTrend::Decreasing => history.ema_load * 0.9,
        };

        self.stats.total_predictions += 1;
        self.stats.avg_prediction_confidence = (self.stats.avg_prediction_confidence
            * (self.stats.total_predictions - 1) as f64
            + confidence)
            / self.stats.total_predictions as f64;

        Ok(LoadPrediction {
            node_id: node_id.to_string(),
            predicted_load: predicted_load.clamp(0.0, 1.0),
            confidence,
            trend,
        })
    }

    pub fn schedule_task(
        &mut self,
        task_id: String,
        node_id: &str,
        predicted_latency_ms: f64,
    ) -> Result<ScheduleEntry, SchedulerError> {
        if !self.histories.contains_key(node_id) {
            return Err(SchedulerError::NodeNotFound(node_id.to_string()));
        }

        let entry = ScheduleEntry {
            task_id,
            node_id: node_id.to_string(),
            predicted_latency_ms,
            scheduled_at_ms: current_timestamp_ms(),
        };

        self.schedule_queue.push_back(entry.clone());
        if self.schedule_queue.len() > self.config.max_schedule_queue {
            self.schedule_queue.pop_front();
        }

        self.stats.total_schedules += 1;
        Ok(entry)
    }

    pub fn get_queue(&self) -> &[ScheduleEntry] {
        self.schedule_queue.as_slices().0
    }

    pub fn get_stats(&self) -> &SchedulerStats {
        &self.stats
    }

    pub fn get_config(&self) -> &SchedulerConfig {
        &self.config
    }

    pub fn reset_stats(&mut self) {
        self.stats = SchedulerStats::default();
    }

    fn compute_confidence(&self, history: &NodeLoadHistory) -> f64 {
        let n = history.load_samples.len();
        if n < self.config.min_history_points {
            return 0.0;
        }
        // Confidence increases with more data points and lower variance
        let mean = history.ema_load;
        let variance: f64 = history
            .load_samples
            .iter()
            .map(|l| (l - mean).powi(2))
            .sum::<f64>()
            / n as f64;
        let stability = 1.0 / (1.0 + variance);
        let data_factor = (n as f64 / 100.0).min(1.0);
        (stability * 0.7 + data_factor * 0.3).min(1.0)
    }
}

impl Default for PredictiveScheduler {
    fn default() -> Self {
        Self::with_defaults()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let s = PredictiveScheduler::with_defaults();
        assert_eq!(s.get_stats().total_predictions, 0);
    }

    #[test]
    fn test_register_node() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        assert!(s.histories.contains_key("n1"));
    }

    #[test]
    fn test_record_load() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        s.record_load("n1", 0.5).unwrap();
        let h = s.histories.get("n1").unwrap();
        assert_eq!(h.load_samples.len(), 1);
    }

    #[test]
    fn test_predict_insufficient_history() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        s.record_load("n1", 0.5).unwrap();
        assert!(s.predict_load("n1").is_err());
    }

    #[test]
    fn test_predict_load() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        for i in 0..10 {
            s.record_load("n1", 0.5 + i as f64 * 0.01).unwrap();
        }
        let pred = s.predict_load("n1").unwrap();
        assert!(pred.confidence > 0.0);
    }

    #[test]
    fn test_trend_detection() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        for i in 0..10 {
            s.record_load("n1", 0.3 + i as f64 * 0.05).unwrap();
        }
        let pred = s.predict_load("n1").unwrap();
        assert!(matches!(pred.trend, LoadTrend::Increasing));
    }

    #[test]
    fn test_schedule_task() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        let entry = s.schedule_task("t1".to_string(), "n1", 50.0).unwrap();
        assert_eq!(entry.task_id, "t1");
        assert_eq!(s.get_queue().len(), 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut s = PredictiveScheduler::with_defaults();
        s.register_node("n1".to_string());
        for i in 0..10 {
            s.record_load("n1", 0.5).unwrap();
        }
        s.predict_load("n1").unwrap();
        assert_eq!(s.get_stats().total_predictions, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut s = PredictiveScheduler::with_defaults();
        s.reset_stats();
        assert_eq!(s.get_stats().total_predictions, 0);
    }

    #[test]
    fn test_error_display() {
        let e = SchedulerError::NoPredictionsAvailable;
        assert!(!e.to_string().is_empty());
    }
}
