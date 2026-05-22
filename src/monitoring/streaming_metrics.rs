//! Streaming Metrics — Collector y emisor de métricas en tiempo real
//!
//! Recolecta métricas de rendimiento, SLO, gobernanza y red en ventanas
//! deslizantes y las emite al backend de telemetría en tiempo real.
//! Soporta agregación por intervalos y exportación SSE-compatible.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::debug;

/// Tipo de métrica streaming
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum MetricType {
    /// Contador (monótono creciente)
    Counter,
    /// Gauge (sube y baja)
    Gauge,
    /// Histograma (distribución)
    Histogram,
    /// Tasa (eventos por segundo)
    Rate,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Counter => write!(f, "counter"),
            MetricType::Gauge => write!(f, "gauge"),
            MetricType::Histogram => write!(f, "histogram"),
            MetricType::Rate => write!(f, "rate"),
        }
    }
}

/// Punto de métrica individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    /// Nombre de la métrica
    pub name: String,
    /// Valor
    pub value: f64,
    /// Timestamp en milisegundos UNIX
    pub timestamp_ms: u64,
    /// Etiquetas clave-valor
    pub labels: HashMap<String, String>,
}

impl MetricPoint {
    /// Crea nuevo punto de métrica
    pub fn new(name: String, value: f64, labels: HashMap<String, String>) -> Self {
        Self {
            name,
            value,
            timestamp_ms: current_timestamp_ms(),
            labels,
        }
    }
}

/// Bucket de histograma
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// Límite superior del bucket
    pub upper_bound: f64,
    /// Conteo acumulado
    pub count: u64,
}

/// Resumen de histograma
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramSummary {
    /// Nombre de la métrica
    pub name: String,
    /// Buckets
    pub buckets: Vec<HistogramBucket>,
    /// Suma total de valores
    pub sum: f64,
    /// Conteo total
    pub count: u64,
    /// Mínimo
    pub min: f64,
    /// Máximo
    pub max: f64,
    /// Promedio
    pub avg: f64,
}

/// Métrica agregada en ventana
#[derive(Debug, Clone)]
pub struct WindowedMetric {
    /// Nombre de la métrica
    pub name: String,
    /// Tipo de métrica
    pub metric_type: MetricType,
    /// Ventana de tiempo en segundos
    pub window_seconds: u64,
    /// Puntos en ventana actual
    pub points: VecDeque<MetricPoint>,
    /// Valor actual agregado
    pub current_value: f64,
    /// Último cálculo de agregación
    pub last_aggregation: Instant,
    /// Activa
    pub active: bool,
}

impl WindowedMetric {
    /// Crea nueva métrica en ventana
    pub fn new(name: String, metric_type: MetricType, window_seconds: u64) -> Self {
        Self {
            name,
            metric_type,
            window_seconds,
            points: VecDeque::new(),
            current_value: 0.0,
            last_aggregation: Instant::now(),
            active: true,
        }
    }

    /// Agrega punto a la ventana
    pub fn record(&mut self, point: MetricPoint) {
        self.points.push_back(point);
        self.evict_expired();
        self.recalculate();
    }

    /// Elimina puntos expirados de la ventana
    fn evict_expired(&mut self) {
        let cutoff = current_timestamp_ms().saturating_sub(self.window_seconds * 1000);
        while let Some(front) = self.points.front() {
            if front.timestamp_ms < cutoff {
                self.points.pop_front();
            } else {
                break;
            }
        }
    }

    /// Recalcula valor agregado según tipo
    fn recalculate(&mut self) {
        if self.points.is_empty() {
            self.current_value = 0.0;
            return;
        }

        self.current_value = match self.metric_type {
            MetricType::Counter => {
                // Último valor para contadores
                self.points.back().map(|p| p.value).unwrap_or(0.0)
            }
            MetricType::Gauge => {
                // Último valor para gauges
                self.points.back().map(|p| p.value).unwrap_or(0.0)
            }
            MetricType::Histogram => {
                // Promedio para histogramas
                let sum: f64 = self.points.iter().map(|p| p.value).sum();
                sum / self.points.len() as f64
            }
            MetricType::Rate => {
                // Tasa: puntos por segundo en ventana
                let count = self.points.len() as f64;
                if self.window_seconds > 0 {
                    count / self.window_seconds as f64
                } else {
                    count
                }
            }
        };
        self.last_aggregation = Instant::now();
    }

    /// Genera resumen de histograma desde los puntos
    pub fn histogram_summary(&self, buckets: &[f64]) -> HistogramSummary {
        let mut bucket_counts: Vec<HistogramBucket> = buckets
            .iter()
            .map(|b| HistogramBucket {
                upper_bound: *b,
                count: 0,
            })
            .collect();

        let mut sum = 0.0;
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut count = 0u64;

        for point in &self.points {
            sum += point.value;
            if point.value < min {
                min = point.value;
            }
            if point.value > max {
                max = point.value;
            }
            count += 1;

            for bucket in &mut bucket_counts {
                if point.value <= bucket.upper_bound {
                    bucket.count += 1;
                }
            }
        }

        HistogramSummary {
            name: self.name.clone(),
            buckets: bucket_counts,
            sum,
            count,
            min: if count > 0 { min } else { 0.0 },
            max: if count > 0 { max } else { 0.0 },
            avg: if count > 0 { sum / count as f64 } else { 0.0 },
        }
    }
}

/// Configuración del collector de métricas streaming
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Ventana de agregación en segundos
    pub aggregation_window_secs: u64,
    /// Intervalo de emisión en milisegundos
    pub emit_interval_ms: u64,
    /// Tamaño máximo de buffer por métrica
    pub max_buffer_size: usize,
    /// Buckets de histograma por defecto
    pub default_histogram_buckets: Vec<f64>,
    /// Habilitar métricas de tasa
    pub enable_rate_metrics: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            aggregation_window_secs: 60,
            emit_interval_ms: 1000,
            max_buffer_size: 5000,
            default_histogram_buckets: vec![
                1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
            ],
            enable_rate_metrics: true,
        }
    }
}

/// Estadísticas del collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingStats {
    /// Métricas registradas
    pub registered_metrics: usize,
    /// Métricas activas
    pub active_metrics: usize,
    /// Total de puntos recolectados
    pub total_points_collected: u64,
    /// Total de emisiones
    pub total_emissions: u64,
    /// Última emisión (ms UNIX)
    pub last_emit_timestamp_ms: u64,
    /// Latencia promedio de agregación (ms)
    pub avg_aggregation_latency_ms: f64,
}

/// Emisor de métricas (trait-like callback)
pub struct StreamingMetricsCollector {
    /// Configuración
    config: StreamingConfig,
    /// Métricas en ventana
    metrics: HashMap<String, WindowedMetric>,
    /// Puntos recolectados totales
    total_points_collected: u64,
    /// Total de emisiones
    total_emissions: u64,
    /// Última emisión
    last_emit_timestamp_ms: u64,
    /// Latencia de agregación acumulada
    aggregation_latency_sum: f64,
    aggregation_count: usize,
}

impl StreamingMetricsCollector {
    /// Crea nuevo collector con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(StreamingConfig::default())
    }

    /// Crea collector con configuración personalizada
    pub fn with_config(config: StreamingConfig) -> Self {
        Self {
            config,
            metrics: HashMap::new(),
            total_points_collected: 0,
            total_emissions: 0,
            last_emit_timestamp_ms: 0,
            aggregation_latency_sum: 0.0,
            aggregation_count: 0,
        }
    }

    /// Registra nueva métrica
    pub fn register_metric(&mut self, name: String, metric_type: MetricType) {
        let window = WindowedMetric::new(
            name.clone(),
            metric_type,
            self.config.aggregation_window_secs,
        );
        self.metrics.insert(name, window);
        debug!(metric_type = %metric_type, "Registered streaming metric");
    }

    /// Registra métrica con ventana personalizada
    pub fn register_metric_with_window(
        &mut self,
        name: String,
        metric_type: MetricType,
        window_seconds: u64,
    ) {
        let window = WindowedMetric::new(name.clone(), metric_type, window_seconds);
        self.metrics.insert(name, window);
    }

    /// Desregistra métrica
    pub fn unregister_metric(&mut self, name: &str) -> bool {
        self.metrics.remove(name).is_some()
    }

    /// Registra punto de métrica
    pub fn record(&mut self, name: String, value: f64, labels: HashMap<String, String>) {
        let point = MetricPoint::new(name.clone(), value, labels);
        self.total_points_collected += 1;

        if let Some(metric) = self.metrics.get_mut(&name) {
            // Limitar buffer
            if metric.points.len() >= self.config.max_buffer_size {
                metric.points.pop_front();
            }
            metric.record(point);
        }
    }

    /// Registra punto rápido (sin labels)
    pub fn record_simple(&mut self, name: &str, value: f64) {
        self.record(name.to_string(), value, HashMap::new());
    }

    /// Ejecuta ciclo de agregación y emisión
    pub fn aggregate_cycle(&mut self) -> Vec<(String, f64, MetricType)> {
        let start = Instant::now();

        let mut results = Vec::new();

        for (name, metric) in &mut self.metrics {
            if !metric.active {
                continue;
            }

            metric.evict_expired();
            metric.recalculate();

            results.push((name.clone(), metric.current_value, metric.metric_type));
        }

        self.total_emissions += 1;
        self.last_emit_timestamp_ms = current_timestamp_ms();

        let latency = start.elapsed().as_micros() as f64 / 1000.0;
        self.aggregation_latency_sum += latency;
        self.aggregation_count += 1;

        debug!(
            metrics = results.len(),
            latency_ms = latency,
            "Aggregation cycle completed"
        );

        results
    }

    /// Genera payload JSON para emisión SSE
    pub fn emit_payload(&self) -> serde_json::Value {
        let metrics: Vec<serde_json::Value> = self
            .metrics
            .values()
            .map(|m| {
                serde_json::json!({
                    "name": m.name,
                    "value": m.current_value,
                    "type": m.metric_type.to_string(),
                    "window_seconds": m.window_seconds,
                    "points_in_window": m.points.len(),
                })
            })
            .collect();

        serde_json::json!({
            "timestamp_ms": current_timestamp_ms(),
            "metrics": metrics,
            "stats": self.get_stats(),
        })
    }

    /// Genera resumen de histograma para métrica específica
    pub fn get_histogram_summary(&self, name: &str) -> Option<HistogramSummary> {
        self.metrics
            .get(name)
            .map(|m| m.histogram_summary(&self.config.default_histogram_buckets))
    }

    /// Obtiene valor actual de métrica
    pub fn get_metric_value(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).map(|m| m.current_value)
    }

    /// Obtiene nombres de métricas registradas
    pub fn get_registered_metrics(&self) -> Vec<String> {
        self.metrics.keys().cloned().collect()
    }

    /// Obtiene estadísticas
    pub fn get_stats(&self) -> StreamingStats {
        let active = self.metrics.values().filter(|m| m.active).count();
        StreamingStats {
            registered_metrics: self.metrics.len(),
            active_metrics: active,
            total_points_collected: self.total_points_collected,
            total_emissions: self.total_emissions,
            last_emit_timestamp_ms: self.last_emit_timestamp_ms,
            avg_aggregation_latency_ms: if self.aggregation_count > 0 {
                self.aggregation_latency_sum / self.aggregation_count as f64
            } else {
                0.0
            },
        }
    }

    /// Resetea collector
    pub fn reset(&mut self) {
        self.metrics.clear();
        self.total_points_collected = 0;
        self.total_emissions = 0;
        self.last_emit_timestamp_ms = 0;
        self.aggregation_latency_sum = 0.0;
        self.aggregation_count = 0;
    }
}

/// Obtiene timestamp actual en milisegundos UNIX
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Default for StreamingMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_creation() {
        let collector = StreamingMetricsCollector::new();
        let stats = collector.get_stats();
        assert_eq!(stats.registered_metrics, 0);
        assert_eq!(stats.total_points_collected, 0);
    }

    #[test]
    fn test_collector_with_config() {
        let config = StreamingConfig {
            aggregation_window_secs: 30,
            emit_interval_ms: 500,
            ..Default::default()
        };
        let collector = StreamingMetricsCollector::with_config(config);
        assert_eq!(collector.config.aggregation_window_secs, 30);
    }

    #[test]
    fn test_register_metric() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("cpu_usage".to_string(), MetricType::Gauge);
        assert_eq!(collector.get_registered_metrics().len(), 1);
    }

    #[test]
    fn test_register_metric_with_window() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric_with_window(
            "custom_metric".to_string(),
            MetricType::Counter,
            120,
        );
        assert_eq!(collector.get_registered_metrics().len(), 1);
    }

    #[test]
    fn test_unregister_metric() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("m1".to_string(), MetricType::Gauge);
        assert!(collector.unregister_metric("m1"));
        assert!(!collector.unregister_metric("nonexistent"));
        assert_eq!(collector.get_registered_metrics().len(), 0);
    }

    #[test]
    fn test_record_simple() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("latency".to_string(), MetricType::Gauge);
        collector.record_simple("latency", 42.5);
        let stats = collector.get_stats();
        assert_eq!(stats.total_points_collected, 1);
    }

    #[test]
    fn test_record_with_labels() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("http_requests".to_string(), MetricType::Counter);
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        labels.insert("status".to_string(), "200".to_string());
        collector.record("http_requests".to_string(), 100.0, labels);
        assert_eq!(collector.get_stats().total_points_collected, 1);
    }

    #[test]
    fn test_aggregate_cycle() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("cpu".to_string(), MetricType::Gauge);
        collector.record_simple("cpu", 50.0);
        collector.record_simple("cpu", 60.0);
        let results = collector.aggregate_cycle();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "cpu");
        assert_eq!(results[0].1, 60.0); // Gauge = último valor
    }

    #[test]
    fn test_counter_aggregation() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("requests".to_string(), MetricType::Counter);
        collector.record_simple("requests", 10.0);
        collector.record_simple("requests", 20.0);
        collector.record_simple("requests", 30.0);
        let results = collector.aggregate_cycle();
        assert_eq!(results[0].1, 30.0); // Counter = último valor
    }

    #[test]
    fn test_histogram_aggregation() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("latency".to_string(), MetricType::Histogram);
        collector.record_simple("latency", 10.0);
        collector.record_simple("latency", 20.0);
        collector.record_simple("latency", 30.0);
        let results = collector.aggregate_cycle();
        assert_eq!(results[0].1, 20.0); // Histogram = promedio
    }

    #[test]
    fn test_rate_aggregation() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("errors".to_string(), MetricType::Rate);
        collector.record_simple("errors", 1.0);
        collector.record_simple("errors", 1.0);
        collector.record_simple("errors", 1.0);
        let results = collector.aggregate_cycle();
        // Rate = puntos / window_seconds
        assert!(results[0].1 > 0.0);
    }

    #[test]
    fn test_emit_payload() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("cpu".to_string(), MetricType::Gauge);
        collector.record_simple("cpu", 75.0);
        let payload = collector.emit_payload();
        assert!(payload.get("metrics").is_some());
        assert!(payload.get("timestamp_ms").is_some());
        assert!(payload.get("stats").is_some());
    }

    #[test]
    fn test_histogram_summary() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("latency".to_string(), MetricType::Histogram);
        collector.record_simple("latency", 5.0);
        collector.record_simple("latency", 15.0);
        collector.record_simple("latency", 45.0);
        let summary = collector.get_histogram_summary("latency");
        assert!(summary.is_some());
        let summary = summary.unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.sum, 65.0);
        assert_eq!(summary.min, 5.0);
        assert_eq!(summary.max, 45.0);
        assert_eq!(summary.avg, 65.0 / 3.0);
    }

    #[test]
    fn test_get_metric_value() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("temp".to_string(), MetricType::Gauge);
        collector.record_simple("temp", 22.5);
        assert_eq!(collector.get_metric_value("temp"), Some(22.5));
        assert_eq!(collector.get_metric_value("nonexistent"), None);
    }

    #[test]
    fn test_stats_tracking() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("m1".to_string(), MetricType::Gauge);
        collector.register_metric("m2".to_string(), MetricType::Counter);
        collector.record_simple("m1", 1.0);
        collector.record_simple("m2", 2.0);
        collector.aggregate_cycle();
        let stats = collector.get_stats();
        assert_eq!(stats.registered_metrics, 2);
        assert_eq!(stats.active_metrics, 2);
        assert_eq!(stats.total_points_collected, 2);
        assert_eq!(stats.total_emissions, 1);
        assert!(stats.avg_aggregation_latency_ms >= 0.0);
    }

    #[test]
    fn test_reset() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("m1".to_string(), MetricType::Gauge);
        collector.record_simple("m1", 1.0);
        collector.aggregate_cycle();
        collector.reset();
        let stats = collector.get_stats();
        assert_eq!(stats.registered_metrics, 0);
        assert_eq!(stats.total_points_collected, 0);
        assert_eq!(stats.total_emissions, 0);
    }

    #[test]
    fn test_metric_type_display() {
        assert_eq!(MetricType::Counter.to_string(), "counter");
        assert_eq!(MetricType::Gauge.to_string(), "gauge");
        assert_eq!(MetricType::Histogram.to_string(), "histogram");
        assert_eq!(MetricType::Rate.to_string(), "rate");
    }

    #[test]
    fn test_metric_point_creation() {
        let mut labels = HashMap::new();
        labels.insert("host".to_string(), "node-1".to_string());
        let point = MetricPoint::new("cpu".to_string(), 50.0, labels);
        assert_eq!(point.name, "cpu");
        assert_eq!(point.value, 50.0);
        assert_eq!(point.labels.len(), 1);
    }

    #[test]
    fn test_windowed_metric_creation() {
        let metric = WindowedMetric::new("test".to_string(), MetricType::Gauge, 60);
        assert_eq!(metric.name, "test");
        assert_eq!(metric.metric_type, MetricType::Gauge);
        assert_eq!(metric.window_seconds, 60);
        assert!(metric.active);
    }

    #[test]
    fn test_windowed_metric_record() {
        let mut metric = WindowedMetric::new("test".to_string(), MetricType::Gauge, 60);
        let point = MetricPoint::new("test".to_string(), 42.0, HashMap::new());
        metric.record(point);
        assert_eq!(metric.current_value, 42.0);
        assert_eq!(metric.points.len(), 1);
    }

    #[test]
    fn test_windowed_metric_histogram_summary() {
        let mut metric = WindowedMetric::new("lat".to_string(), MetricType::Histogram, 60);
        metric.record(MetricPoint::new("lat".to_string(), 10.0, HashMap::new()));
        metric.record(MetricPoint::new("lat".to_string(), 20.0, HashMap::new()));
        let summary = metric.histogram_summary(&[10.0, 20.0, 30.0]);
        assert_eq!(summary.count, 2);
        assert_eq!(summary.sum, 30.0);
    }

    #[test]
    fn test_config_default() {
        let config = StreamingConfig::default();
        assert_eq!(config.aggregation_window_secs, 60);
        assert_eq!(config.emit_interval_ms, 1000);
        assert_eq!(config.max_buffer_size, 5000);
        assert!(config.enable_rate_metrics);
    }

    #[test]
    fn test_buffer_size_limit() {
        let config = StreamingConfig {
            max_buffer_size: 3,
            ..Default::default()
        };
        let mut collector = StreamingMetricsCollector::with_config(config);
        collector.register_metric("m".to_string(), MetricType::Gauge);
        for i in 0..5 {
            collector.record_simple("m", i as f64);
        }
        let metric = collector.metrics.get("m").unwrap();
        assert_eq!(metric.points.len(), 3);
    }

    #[test]
    fn test_aggregation_cycle_empty() {
        let mut collector = StreamingMetricsCollector::new();
        let results = collector.aggregate_cycle();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_metric_types() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("counter_m".to_string(), MetricType::Counter);
        collector.register_metric("gauge_m".to_string(), MetricType::Gauge);
        collector.register_metric("hist_m".to_string(), MetricType::Histogram);
        collector.register_metric("rate_m".to_string(), MetricType::Rate);
        collector.record_simple("counter_m", 100.0);
        collector.record_simple("gauge_m", 50.0);
        collector.record_simple("hist_m", 25.0);
        collector.record_simple("rate_m", 1.0);
        let results = collector.aggregate_cycle();
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_default() {
        let collector = StreamingMetricsCollector::default();
        assert_eq!(collector.get_stats().registered_metrics, 0);
    }

    #[test]
    fn test_record_unregistered_metric() {
        let mut collector = StreamingMetricsCollector::new();
        collector.record_simple("unregistered", 42.0);
        // Should not panic, just increment total_points_collected
        assert_eq!(collector.get_stats().total_points_collected, 1);
    }

    #[test]
    fn test_histogram_bucket_counts() {
        let mut collector = StreamingMetricsCollector::new();
        collector.register_metric("lat".to_string(), MetricType::Histogram);
        collector.record_simple("lat", 3.0);
        collector.record_simple("lat", 8.0);
        collector.record_simple("lat", 15.0);
        collector.record_simple("lat", 60.0);
        let summary = collector.get_histogram_summary("lat").unwrap();
        // Buckets: [1, 5, 10, 25, 50, 100, ...]
        // 3.0 <= 5, 10, 25, 50, 100...
        // 8.0 <= 10, 25, 50, 100...
        // 15.0 <= 25, 50, 100...
        // 60.0 <= 100...
        let bucket_5 = summary
            .buckets
            .iter()
            .find(|b| b.upper_bound == 5.0)
            .unwrap();
        assert_eq!(bucket_5.count, 1); // Only 3.0
        let bucket_10 = summary
            .buckets
            .iter()
            .find(|b| b.upper_bound == 10.0)
            .unwrap();
        assert_eq!(bucket_10.count, 2); // 3.0 and 8.0
        let bucket_25 = summary
            .buckets
            .iter()
            .find(|b| b.upper_bound == 25.0)
            .unwrap();
        assert_eq!(bucket_25.count, 3); // 3.0, 8.0, 15.0
    }
}
