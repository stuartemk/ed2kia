//! Advanced Metrics — High-resolution metrics collection with percentiles and histograms.
//!
//! Provides histogram-based latency tracking, rate counters, and gauge metrics
//! with minimal overhead. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::collections::HashMap;

/// Error types for advanced metrics operations.
#[derive(Debug)]
pub enum MetricsError {
    /// Metric name already exists with different type.
    TypeMismatch(String),
    /// Invalid metric name.
    InvalidName(String),
    /// Bucket count out of range.
    InvalidBuckets(usize),
}

impl std::fmt::Display for MetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricsError::TypeMismatch(name) => write!(f, "Type mismatch: {}", name),
            MetricsError::InvalidName(name) => write!(f, "Invalid name: {}", name),
            MetricsError::InvalidBuckets(count) => write!(f, "Invalid bucket count: {}", count),
        }
    }
}

/// Metric type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Counter => write!(f, "counter"),
            MetricType::Gauge => write!(f, "gauge"),
            MetricType::Histogram => write!(f, "histogram"),
        }
    }
}

/// Histogram bucket boundaries.
#[derive(Debug, Clone)]
pub struct HistogramBuckets {
    pub boundaries: Vec<f64>,
    pub counts: Vec<u64>,
}

impl HistogramBuckets {
    pub fn new(boundaries: Vec<f64>) -> Self {
        let counts = vec![0; boundaries.len() + 1];
        Self { boundaries, counts }
    }

    /// Record an observation in the histogram.
    pub fn observe(&mut self, value: f64) {
        for (i, &boundary) in self.boundaries.iter().enumerate() {
            if value <= boundary {
                self.counts[i] += 1;
                return;
            }
        }
        // +Inf bucket
        let idx = self.counts.len() - 1;
        self.counts[idx] += 1;
    }

    /// Get the count at or below a given percentile.
    pub fn percentile(&self, p: f64) -> f64 {
        let total: u64 = self.counts.iter().sum();
        if total == 0 {
            return 0.0;
        }
        let target = (p / 100.0) * total as f64;
        let mut cumulative = 0u64;
        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative as f64 >= target {
                if i < self.boundaries.len() {
                    return self.boundaries[i];
                }
                return f64::INFINITY;
            }
        }
        0.0
    }

    /// Total observations.
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }
}

/// Counter metric — monotonically increasing value.
#[derive(Debug, Clone)]
pub struct Counter {
    pub name: String,
    pub help: String,
    pub value: u64,
    pub labels: HashMap<String, String>,
}

impl Counter {
    pub fn new(name: String, help: String) -> Self {
        Self {
            name,
            help,
            value: 0,
            labels: HashMap::new(),
        }
    }

    pub fn inc(&mut self) {
        self.value += 1;
    }

    pub fn add(&mut self, amount: u64) {
        self.value += amount;
    }

    pub fn with_label(&mut self, key: &str, value: &str) {
        self.labels.insert(key.to_string(), value.to_string());
    }
}

/// Gauge metric — value that can go up and down.
#[derive(Debug, Clone)]
pub struct Gauge {
    pub name: String,
    pub help: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

impl Gauge {
    pub fn new(name: String, help: String) -> Self {
        Self {
            name,
            help,
            value: 0.0,
            labels: HashMap::new(),
        }
    }

    pub fn set(&mut self, value: f64) {
        self.value = value;
    }

    pub fn inc(&mut self, amount: f64) {
        self.value += amount;
    }

    pub fn dec(&mut self, amount: f64) {
        self.value -= amount;
    }

    pub fn with_label(&mut self, key: &str, value: &str) {
        self.labels.insert(key.to_string(), value.to_string());
    }
}

/// Histogram metric — distribution of observations.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub name: String,
    pub help: String,
    pub buckets: HistogramBuckets,
    pub sum: f64,
    pub count: u64,
    pub labels: HashMap<String, String>,
}

impl Histogram {
    pub fn new(name: String, help: String, boundaries: Vec<f64>) -> Self {
        Self {
            name,
            help,
            buckets: HistogramBuckets::new(boundaries),
            sum: 0.0,
            count: 0,
            labels: HashMap::new(),
        }
    }

    pub fn observe(&mut self, value: f64) {
        self.buckets.observe(value);
        self.sum += value;
        self.count += 1;
    }

    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / self.count as f64
    }

    pub fn with_label(&mut self, key: &str, value: &str) {
        self.labels.insert(key.to_string(), value.to_string());
    }
}

/// Configuration for the advanced metrics registry.
#[derive(Debug, Clone)]
pub struct AdvancedMetricsConfig {
    /// Enable metrics collection.
    pub enabled: bool,
    /// Default histogram boundaries (milliseconds).
    pub default_boundaries: Vec<f64>,
    /// Maximum metric names.
    pub max_metrics: usize,
    /// Collection interval in milliseconds.
    pub collection_interval_ms: u64,
}

impl Default for AdvancedMetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_boundaries: vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0],
            max_metrics: 4096,
            collection_interval_ms: 1000,
        }
    }
}

/// Metrics registry snapshot.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub counters: Vec<(String, u64)>,
    pub gauges: Vec<(String, f64)>,
    pub histograms: Vec<(String, f64, f64, u64)>, // (name, avg, p99, count)
    pub timestamp_ms: u64,
}

/// Advanced metrics registry.
pub struct AdvancedMetrics {
    config: AdvancedMetricsConfig,
    counters: HashMap<String, Counter>,
    gauges: HashMap<String, Gauge>,
    histograms: HashMap<String, Histogram>,
    type_registry: HashMap<String, MetricType>,
    current_time_ms: u64,
}

impl AdvancedMetrics {
    pub fn new(config: AdvancedMetricsConfig) -> Self {
        Self {
            config,
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            type_registry: HashMap::new(),
            current_time_ms: 0,
        }
    }

    /// Set current time (for testing).
    pub fn set_time(&mut self, now_ms: u64) {
        self.current_time_ms = now_ms;
    }

    /// Register a new counter.
    pub fn register_counter(
        &mut self,
        name: String,
        help: String,
    ) -> Result<(), MetricsError> {
        self.check_registration(&name, MetricType::Counter)?;
        self.counters.insert(name.clone(), Counter::new(name, help));
        Ok(())
    }

    /// Register a new gauge.
    pub fn register_gauge(
        &mut self,
        name: String,
        help: String,
    ) -> Result<(), MetricsError> {
        self.check_registration(&name, MetricType::Gauge)?;
        self.gauges.insert(name.clone(), Gauge::new(name, help));
        Ok(())
    }

    /// Register a new histogram.
    pub fn register_histogram(
        &mut self,
        name: String,
        help: String,
        boundaries: Option<Vec<f64>>,
    ) -> Result<(), MetricsError> {
        self.check_registration(&name, MetricType::Histogram)?;
        let boundaries = boundaries.unwrap_or_else(|| self.config.default_boundaries.clone());
        if boundaries.is_empty() {
            return Err(MetricsError::InvalidBuckets(0));
        }
        self.histograms
            .insert(name.clone(), Histogram::new(name, help, boundaries));
        Ok(())
    }

    /// Increment a counter.
    pub fn counter_inc(&mut self, name: &str) -> Result<(), MetricsError> {
        if let Some(counter) = self.counters.get_mut(name) {
            counter.inc();
            Ok(())
        } else {
            Err(MetricsError::InvalidName(name.to_string()))
        }
    }

    /// Add to a counter.
    pub fn counter_add(&mut self, name: &str, amount: u64) -> Result<(), MetricsError> {
        if let Some(counter) = self.counters.get_mut(name) {
            counter.add(amount);
            Ok(())
        } else {
            Err(MetricsError::InvalidName(name.to_string()))
        }
    }

    /// Get counter value.
    pub fn counter_get(&self, name: &str) -> Option<u64> {
        self.counters.get(name).map(|c| c.value)
    }

    /// Set gauge value.
    pub fn gauge_set(&mut self, name: &str, value: f64) -> Result<(), MetricsError> {
        if let Some(gauge) = self.gauges.get_mut(name) {
            gauge.set(value);
            Ok(())
        } else {
            Err(MetricsError::InvalidName(name.to_string()))
        }
    }

    /// Get gauge value.
    pub fn gauge_get(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).map(|g| g.value)
    }

    /// Observe a histogram value.
    pub fn histogram_observe(&mut self, name: &str, value: f64) -> Result<(), MetricsError> {
        if let Some(histogram) = self.histograms.get_mut(name) {
            histogram.observe(value);
            Ok(())
        } else {
            Err(MetricsError::InvalidName(name.to_string()))
        }
    }

    /// Get histogram average.
    pub fn histogram_avg(&self, name: &str) -> Option<f64> {
        self.histograms.get(name).map(|h| h.avg())
    }

    /// Get histogram p99.
    pub fn histogram_p99(&self, name: &str) -> Option<f64> {
        self.histograms.get(name).map(|h| h.buckets.percentile(99.0))
    }

    /// Get histogram p95.
    pub fn histogram_p95(&self, name: &str) -> Option<f64> {
        self.histograms.get(name).map(|h| h.buckets.percentile(95.0))
    }

    /// Get histogram p50 (median).
    pub fn histogram_p50(&self, name: &str) -> Option<f64> {
        self.histograms.get(name).map(|h| h.buckets.percentile(50.0))
    }

    /// Get histogram count.
    pub fn histogram_count(&self, name: &str) -> Option<u64> {
        self.histograms.get(name).map(|h| h.count)
    }

    /// Take a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let counters = self
            .counters
            .iter()
            .map(|(name, c)| (name.clone(), c.value))
            .collect();
        let gauges = self
            .gauges
            .iter()
            .map(|(name, g)| (name.clone(), g.value))
            .collect();
        let histograms = self
            .histograms
            .iter()
            .map(|(name, h)| (name.clone(), h.avg(), h.buckets.percentile(99.0), h.count))
            .collect();
        MetricsSnapshot {
            counters,
            gauges,
            histograms,
            timestamp_ms: self.current_time_ms,
        }
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        self.counters.clear();
        self.gauges.clear();
        self.histograms.clear();
        self.type_registry.clear();
    }

    /// Get configuration.
    pub fn config(&self) -> &AdvancedMetricsConfig {
        &self.config
    }

    /// Get total metric count.
    pub fn metric_count(&self) -> usize {
        self.counters.len() + self.gauges.len() + self.histograms.len()
    }

    // ─── Internal ───

    fn check_registration(&mut self, name: &str, metric_type: MetricType) -> Result<(), MetricsError> {
        if name.is_empty() {
            return Err(MetricsError::InvalidName(name.to_string()));
        }
        if let Some(existing) = self.type_registry.get(name) {
            if existing != &metric_type {
                return Err(MetricsError::TypeMismatch(name.to_string()));
            }
            return Ok(()); // Same type, allow
        }
        if self.metric_count() >= self.config.max_metrics {
            return Err(MetricsError::InvalidName(format!(
                "Max metrics ({}) reached",
                self.config.max_metrics
            )));
        }
        self.type_registry.insert(name.to_string(), metric_type);
        Ok(())
    }
}

impl Default for AdvancedMetrics {
    fn default() -> Self {
        Self::new(AdvancedMetricsConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let metrics = AdvancedMetrics::default();
        assert_eq!(metrics.metric_count(), 0);
    }

    #[test]
    fn test_register_counter() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_counter("requests".to_string(), "Total requests".to_string()).unwrap();
        assert_eq!(metrics.metric_count(), 1);
    }

    #[test]
    fn test_counter_inc() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_counter("req".to_string(), "h".to_string()).unwrap();
        metrics.counter_inc("req").unwrap();
        metrics.counter_inc("req").unwrap();
        assert_eq!(metrics.counter_get("req"), Some(2));
    }

    #[test]
    fn test_counter_add() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_counter("bytes".to_string(), "h".to_string()).unwrap();
        metrics.counter_add("bytes", 100).unwrap();
        assert_eq!(metrics.counter_get("bytes"), Some(100));
    }

    #[test]
    fn test_counter_not_found() {
        let mut metrics = AdvancedMetrics::default();
        assert!(metrics.counter_inc("missing").is_err());
    }

    #[test]
    fn test_register_gauge() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_gauge("cpu".to_string(), "CPU usage".to_string()).unwrap();
        assert_eq!(metrics.metric_count(), 1);
    }

    #[test]
    fn test_gauge_set() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_gauge("temp".to_string(), "h".to_string()).unwrap();
        metrics.gauge_set("temp", 42.5).unwrap();
        assert_eq!(metrics.gauge_get("temp"), Some(42.5));
    }

    #[test]
    fn test_gauge_inc_dec() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_gauge("queue".to_string(), "h".to_string()).unwrap();
        metrics.gauge_set("queue", 10.0).unwrap();
        metrics.gauges.get_mut("queue").unwrap().inc(5.0);
        metrics.gauges.get_mut("queue").unwrap().dec(3.0);
        assert_eq!(metrics.gauge_get("queue"), Some(12.0));
    }

    #[test]
    fn test_register_histogram() {
        let mut metrics = AdvancedMetrics::default();
        metrics
            .register_histogram("latency".to_string(), "Latency ms".to_string(), None)
            .unwrap();
        assert_eq!(metrics.metric_count(), 1);
    }

    #[test]
    fn test_histogram_observe() {
        let mut metrics = AdvancedMetrics::default();
        metrics
            .register_histogram("lat".to_string(), "h".to_string(), None)
            .unwrap();
        metrics.histogram_observe("lat", 10.0).unwrap();
        metrics.histogram_observe("lat", 20.0).unwrap();
        assert_eq!(metrics.histogram_count("lat"), Some(2));
        assert_eq!(metrics.histogram_avg("lat"), Some(15.0));
    }

    #[test]
    fn test_histogram_percentiles() {
        let mut metrics = AdvancedMetrics::default();
        metrics
            .register_histogram("lat".to_string(), "h".to_string(), Some(vec![5.0, 10.0, 50.0, 100.0]))
            .unwrap();
        for _ in 0..95 {
            metrics.histogram_observe("lat", 10.0).unwrap();
        }
        for _ in 0..5 {
            metrics.histogram_observe("lat", 50.0).unwrap();
        }
        let p50 = metrics.histogram_p50("lat").unwrap();
        assert!(p50 <= 10.0);
    }

    #[test]
    fn test_type_mismatch() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_counter("x".to_string(), "h".to_string()).unwrap();
        let result = metrics.register_gauge("x".to_string(), "h".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_name() {
        let mut metrics = AdvancedMetrics::default();
        let result = metrics.register_counter("".to_string(), "h".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot() {
        let mut metrics = AdvancedMetrics::default();
        metrics.set_time(1000);
        metrics.register_counter("c".to_string(), "h".to_string()).unwrap();
        metrics.counter_inc("c").unwrap();
        let snap = metrics.snapshot();
        assert_eq!(snap.counters.len(), 1);
        assert_eq!(snap.timestamp_ms, 1000);
    }

    #[test]
    fn test_reset() {
        let mut metrics = AdvancedMetrics::default();
        metrics.register_counter("c".to_string(), "h".to_string()).unwrap();
        metrics.reset();
        assert_eq!(metrics.metric_count(), 0);
    }

    #[test]
    fn test_histogram_buckets_observe() {
        let mut buckets = HistogramBuckets::new(vec![10.0, 50.0, 100.0]);
        buckets.observe(5.0);
        buckets.observe(25.0);
        buckets.observe(75.0);
        buckets.observe(200.0);
        assert_eq!(buckets.counts[0], 1);
        assert_eq!(buckets.counts[1], 1);
        assert_eq!(buckets.counts[2], 1);
        assert_eq!(buckets.counts[3], 1); // +Inf
    }

    #[test]
    fn test_histogram_buckets_percentile() {
        let mut buckets = HistogramBuckets::new(vec![10.0, 50.0, 100.0]);
        for _ in 0..10 {
            buckets.observe(5.0);
        }
        assert_eq!(buckets.percentile(50.0), 10.0);
    }

    #[test]
    fn test_histogram_buckets_total() {
        let mut buckets = HistogramBuckets::new(vec![10.0]);
        buckets.observe(5.0);
        buckets.observe(15.0);
        assert_eq!(buckets.total(), 2);
    }

    #[test]
    fn test_metric_type_display() {
        assert_eq!(MetricType::Counter.to_string(), "counter");
        assert_eq!(MetricType::Gauge.to_string(), "gauge");
        assert_eq!(MetricType::Histogram.to_string(), "histogram");
    }

    #[test]
    fn test_config_default() {
        let config = AdvancedMetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_metrics, 4096);
    }

    #[test]
    fn test_max_metrics_limit() {
        let config = AdvancedMetricsConfig {
            max_metrics: 2,
            ..Default::default()
        };
        let mut metrics = AdvancedMetrics::new(config);
        metrics.register_counter("a".to_string(), "h".to_string()).unwrap();
        metrics.register_gauge("b".to_string(), "h".to_string()).unwrap();
        let result = metrics.register_counter("c".to_string(), "h".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let e = MetricsError::TypeMismatch("x".to_string());
        assert!(format!("{}", e).contains("x"));
    }

    #[test]
    fn test_histogram_empty_avg() {
        let h = Histogram::new("h".to_string(), "help".to_string(), vec![10.0]);
        assert_eq!(h.avg(), 0.0);
    }

    #[test]
    fn test_counter_labels() {
        let mut counter = Counter::new("req".to_string(), "h".to_string());
        counter.with_label("method", "GET");
        assert_eq!(counter.labels.get("method"), Some(&"GET".to_string()));
    }

    #[test]
    fn test_gauge_labels() {
        let mut gauge = Gauge::new("cpu".to_string(), "h".to_string());
        gauge.with_label("core", "0");
        assert_eq!(gauge.labels.get("core"), Some(&"0".to_string()));
    }
}
