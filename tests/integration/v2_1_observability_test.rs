//! v2.1 Observability Integration Tests
//! Feature-gated: v2.1-observability
//! Validates Observable trait structure, simulated metrics, data format assertions.
//! No network calls, no real runtime — scaffold validation only.

#![cfg(feature = "v2.1-observability")]

/// Simulated metric entry for observability testing.
#[derive(Debug, Clone, PartialEq)]
struct MetricEntry {
    name: String,
    value: f64,
    timestamp_ms: u64,
    labels: Vec<(String, String)>,
}

impl MetricEntry {
    fn new(name: &str, value: f64, timestamp_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            value,
            timestamp_ms,
            labels: Vec::new(),
        }
    }

    fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.push((key.to_string(), value.to_string()));
        self
    }
}

/// Simulated Observable trait for scaffold validation.
trait Observable {
    fn collect_metrics(&self) -> Vec<MetricEntry>;
    fn metric_count(&self) -> usize;
    fn has_metric(&self, name: &str) -> bool;
}

/// Simulated NodeMetrics collector.
struct NodeMetrics {
    metrics: Vec<MetricEntry>,
}

impl NodeMetrics {
    fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }

    fn add_metric(&mut self, metric: MetricEntry) {
        self.metrics.push(metric);
    }
}

impl Observable for NodeMetrics {
    fn collect_metrics(&self) -> Vec<MetricEntry> {
        self.metrics.clone()
    }

    fn metric_count(&self) -> usize {
        self.metrics.len()
    }

    fn has_metric(&self, name: &str) -> bool {
        self.metrics.iter().any(|m| m.name == name)
    }
}

/// Simulated HealthEndpoint for scaffold validation.
struct HealthEndpoint {
    status: String,
    uptime_seconds: u64,
    checks_passed: usize,
    checks_failed: usize,
}

impl HealthEndpoint {
    fn new(status: &str, uptime_seconds: u64) -> Self {
        Self {
            status: status.to_string(),
            uptime_seconds,
            checks_passed: 0,
            checks_failed: 0,
        }
    }

    fn check(&mut self, passed: bool) {
        if passed {
            self.checks_passed += 1;
        } else {
            self.checks_failed += 1;
        }
    }

    fn is_healthy(&self) -> bool {
        self.checks_failed == 0 && self.status == "ok"
    }
}

impl Observable for HealthEndpoint {
    fn collect_metrics(&self) -> Vec<MetricEntry> {
        vec![
            MetricEntry::new("health_status", if self.is_healthy() { 1.0 } else { 0.0 }, 0)
                .with_label("status", &self.status),
            MetricEntry::new("uptime_seconds", self.uptime_seconds as f64, 0),
            MetricEntry::new("checks_passed", self.checks_passed as f64, 0),
            MetricEntry::new("checks_failed", self.checks_failed as f64, 0),
        ]
    }

    fn metric_count(&self) -> usize {
        4
    }

    fn has_metric(&self, name: &str) -> bool {
        ["health_status", "uptime_seconds", "checks_passed", "checks_failed"]
            .iter()
            .any(|n| *n == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_entry_creation() {
        let metric = MetricEntry::new("cpu_usage", 45.5, 1000);
        assert_eq!(metric.name, "cpu_usage");
        assert!((metric.value - 45.5).abs() < f64::EPSILON);
        assert_eq!(metric.timestamp_ms, 1000);
        assert!(metric.labels.is_empty());
    }

    #[test]
    fn test_metric_entry_with_label() {
        let metric = MetricEntry::new("request_count", 100.0, 2000)
            .with_label("method", "GET")
            .with_label("path", "/health");
        assert_eq!(metric.labels.len(), 2);
        assert_eq!(metric.labels[0].0, "method");
        assert_eq!(metric.labels[0].1, "GET");
    }

    #[test]
    fn test_node_metrics_empty() {
        let metrics = NodeMetrics::new();
        assert_eq!(metrics.metric_count(), 0);
        assert!(metrics.collect_metrics().is_empty());
        assert!(!metrics.has_metric("anything"));
    }

    #[test]
    fn test_node_metrics_add_and_collect() {
        let mut metrics = NodeMetrics::new();
        metrics.add_metric(MetricEntry::new("cpu", 25.0, 100));
        metrics.add_metric(MetricEntry::new("memory", 512.0, 100));

        assert_eq!(metrics.metric_count(), 2);
        assert!(metrics.has_metric("cpu"));
        assert!(metrics.has_metric("memory"));
        assert!(!metrics.has_metric("disk"));

        let collected = metrics.collect_metrics();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].name, "cpu");
        assert_eq!(collected[1].name, "memory");
    }

    #[test]
    fn test_health_endpoint_healthy() {
        let mut endpoint = HealthEndpoint::new("ok", 3600);
        endpoint.check(true);
        endpoint.check(true);
        endpoint.check(true);

        assert!(endpoint.is_healthy());
        assert_eq!(endpoint.checks_passed, 3);
        assert_eq!(endpoint.checks_failed, 0);
    }

    #[test]
    fn test_health_endpoint_unhealthy() {
        let mut endpoint = HealthEndpoint::new("ok", 3600);
        endpoint.check(true);
        endpoint.check(false);

        assert!(!endpoint.is_healthy());
        assert_eq!(endpoint.checks_passed, 1);
        assert_eq!(endpoint.checks_failed, 1);
    }

    #[test]
    fn test_health_endpoint_status_not_ok() {
        let endpoint = HealthEndpoint::new("degraded", 3600);
        assert!(!endpoint.is_healthy());
    }

    #[test]
    fn test_health_endpoint_observable() {
        let endpoint = HealthEndpoint::new("ok", 7200);
        let metrics = endpoint.collect_metrics();

        assert_eq!(endpoint.metric_count(), 4);
        assert!(endpoint.has_metric("health_status"));
        assert!(endpoint.has_metric("uptime_seconds"));
        assert!(endpoint.has_metric("checks_passed"));
        assert!(endpoint.has_metric("checks_failed"));

        // Verify metric values
        let uptime_metric = metrics.iter().find(|m| m.name == "uptime_seconds").unwrap();
        assert!((uptime_metric.value - 7200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_observable_trait_consistency() {
        let mut node = NodeMetrics::new();
        node.add_metric(MetricEntry::new("latency", 150.0, 500));

        let collected = node.collect_metrics();
        assert_eq!(collected.len(), node.metric_count());
        assert!(node.has_metric("latency"));
    }

    #[test]
    fn test_metric_equality() {
        let m1 = MetricEntry::new("test", 1.0, 100);
        let m2 = MetricEntry::new("test", 1.0, 100);
        let m3 = MetricEntry::new("test", 2.0, 100);

        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }
}
