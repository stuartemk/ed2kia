//! Metrics - Prometheus counters/gauges/histograms
//!
//! Contadores de mensajes, latencia de inferencia SAE, éxito/fracaso de consenso,
//! uso de memoria WASM, feedback procesado. Exporta en formato Prometheus text.

// CLEANUP: removed unused imports AtomicU64, Ordering, Arc, RwLock, GaugeVec, tracing::info
use std::time::Instant;

use prometheus::{
    Counter, CounterVec, Encoder, Gauge, Histogram, HistogramOpts, Opts, Registry, TextEncoder,
};

// CLEANUP: changed /// to // for lazy_static macros (rustdoc cannot document macro invocations)
// Registry global de métricas Prometheus
lazy_static::lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
}

// ─── Contadores (Counters) ───────────────────────────────────────────────

// Total de mensajes P2P recibidos
lazy_static::lazy_static! {
    static ref P2P_MESSAGES_RECEIVED: CounterVec = CounterVec::new(
        Opts::new("p2p_messages_received_total", "Total P2P messages received"),
        &["type"],
    ).unwrap();
}

// Total de mensajes P2P enviados
lazy_static::lazy_static! {
    static ref P2P_MESSAGES_SENT: CounterVec = CounterVec::new(
        Opts::new("p2p_messages_sent_total", "Total P2P messages sent"),
        &["type"],
    ).unwrap();
}

// Total de inferencias SAE ejecutadas
lazy_static::lazy_static! {
    static ref SAE_INFERENCE_TOTAL: Counter = Counter::new(
        "sae_inference_total", "Total SAE inferences executed"
    ).unwrap();
}

// Total de batches de consenso procesados
lazy_static::lazy_static! {
    static ref CONSENSUS_BATCHES_TOTAL: CounterVec = CounterVec::new(
        Opts::new("consensus_batches_total", "Total consensus batches processed"),
        &["result"],
    ).unwrap();
}

// Total de feedback humano procesado
lazy_static::lazy_static! {
    static ref FEEDBACK_PROCESSED_TOTAL: CounterVec = CounterVec::new(
        Opts::new("feedback_processed_total", "Total human feedback processed"),
        &["decision"],
    ).unwrap();
}

// Total de errores de sandbox WASM
lazy_static::lazy_static! {
    static ref WASM_SANDBOX_ERRORS_TOTAL: Counter = Counter::new(
        "wasm_sandbox_errors_total", "Total WASM sandbox errors"
    ).unwrap();
}

// ─── Gauges ──────────────────────────────────────────────────────────────

// Número de peers conectados
lazy_static::lazy_static! {
    static ref PEERS_CONNECTED: Gauge = Gauge::new(
        "peers_connected", "Number of connected peers"
    ).unwrap();
}

// Uso de memoria del sandbox WASM (bytes)
lazy_static::lazy_static! {
    static ref WASM_MEMORY_USAGE_BYTES: Gauge = Gauge::new(
        "wasm_memory_usage_bytes", "WASM sandbox memory usage in bytes"
    ).unwrap();
}

// Número de entradas en el feedback store
lazy_static::lazy_static! {
    static ref FEEDBACK_STORE_ENTRIES: Gauge = Gauge::new(
        "feedback_store_entries", "Number of entries in feedback store"
    ).unwrap();
}

// Número de batches de entrenamiento listos
lazy_static::lazy_static! {
    static ref TRAINING_BATCHES_READY: Gauge = Gauge::new(
        "training_batches_ready", "Number of training batches ready for export"
    ).unwrap();
}

// Score promedio de reputación de peers
lazy_static::lazy_static! {
    static ref PEER_REPUTATION_AVG: Gauge = Gauge::new(
        "peer_reputation_avg", "Average peer reputation score"
    ).unwrap();
}

// ─── Histogramas ─────────────────────────────────────────────────────────

// Latencia de inferencia SAE (milisegundos)
lazy_static::lazy_static! {
    static ref SAE_INFERENCE_LATENCY_MS: Histogram = Histogram::with_opts(
        HistogramOpts::new("sae_inference_latency_ms", "SAE inference latency in milliseconds")
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]),
    ).unwrap();
}

// Latencia de consenso (milisegundos)
lazy_static::lazy_static! {
    static ref CONSENSUS_LATENCY_MS: Histogram = Histogram::with_opts(
        HistogramOpts::new("consensus_latency_ms", "Consensus latency in milliseconds")
            .buckets(vec![10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0]),
    ).unwrap();
}

// Latencia de verificación ZKP (milisegundos)
lazy_static::lazy_static! {
    static ref ZKP_VERIFICATION_LATENCY_MS: Histogram = Histogram::with_opts(
        HistogramOpts::new("zkp_verification_latency_ms", "ZKP verification latency in milliseconds")
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]),
    ).unwrap();
}

// Tamaño de mensajes P2P (bytes)
lazy_static::lazy_static! {
    static ref P2P_MESSAGE_SIZE_BYTES: Histogram = Histogram::with_opts(
        HistogramOpts::new("p2p_message_size_bytes", "P2P message size in bytes")
            .buckets(vec![64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0]),
    ).unwrap();
}

/// Manager de métricas
pub struct MetricsManager {
    /// Registry Prometheus
    registry: Registry,
    /// Timestamp de inicio
    start_time: Instant,
}

impl MetricsManager {
    pub fn new() -> Self {
        let registry = Registry::new();

        // FIX: prometheus 0.13 - Use registry.register() instead of .register_clone() | prometheus API
        // Registrar todas las métricas
        let _ = registry.register(Box::new(P2P_MESSAGES_RECEIVED.clone()));
        let _ = registry.register(Box::new(P2P_MESSAGES_SENT.clone()));
        let _ = registry.register(Box::new(SAE_INFERENCE_TOTAL.clone()));
        let _ = registry.register(Box::new(CONSENSUS_BATCHES_TOTAL.clone()));
        let _ = registry.register(Box::new(FEEDBACK_PROCESSED_TOTAL.clone()));
        let _ = registry.register(Box::new(WASM_SANDBOX_ERRORS_TOTAL.clone()));
        let _ = registry.register(Box::new(PEERS_CONNECTED.clone()));
        let _ = registry.register(Box::new(WASM_MEMORY_USAGE_BYTES.clone()));
        let _ = registry.register(Box::new(FEEDBACK_STORE_ENTRIES.clone()));
        let _ = registry.register(Box::new(TRAINING_BATCHES_READY.clone()));
        let _ = registry.register(Box::new(PEER_REPUTATION_AVG.clone()));
        let _ = registry.register(Box::new(SAE_INFERENCE_LATENCY_MS.clone()));
        let _ = registry.register(Box::new(CONSENSUS_LATENCY_MS.clone()));
        let _ = registry.register(Box::new(ZKP_VERIFICATION_LATENCY_MS.clone()));
        let _ = registry.register(Box::new(P2P_MESSAGE_SIZE_BYTES.clone()));

        Self {
            registry,
            start_time: Instant::now(),
        }
    }

    /// Exporta métricas en formato Prometheus text
    // FIX: E0599 - Use encode() with proper prometheus 0.13 API
    pub fn encode_metrics(&self) -> String {
        let encoder = TextEncoder::new();
        let metrics_families = self.registry.gather();
        let mut buffer = Vec::new();

        match encoder.encode(&metrics_families, &mut buffer) {
            Ok(_) => String::from_utf8_lossy(&buffer).to_string(),
            Err(e) => format!("# ERROR encoding metrics: {}\n", e),
        }
    }

    /// Obtiene uptime en segundos
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

// ─── Funciones de conveniencia para actualizar métricas ──────────────────

/// Registra mensaje P2P recibido
pub fn inc_p2p_messages_received(message_type: &str) {
    P2P_MESSAGES_RECEIVED
        .with_label_values(&[message_type])
        .inc();
}

/// Registra mensaje P2P enviado
pub fn inc_p2p_messages_sent(message_type: &str) {
    P2P_MESSAGES_SENT.with_label_values(&[message_type]).inc();
}

/// Registra tamaño de mensaje P2P
pub fn record_p2p_message_size(bytes: u64) {
    P2P_MESSAGE_SIZE_BYTES.observe(bytes as f64);
}

/// Registra inferencia SAE completada
pub fn record_sae_inference(latency_ms: f64) {
    SAE_INFERENCE_TOTAL.inc();
    SAE_INFERENCE_LATENCY_MS.observe(latency_ms);
}

/// Registra batch de consenso
pub fn record_consensus_batch(result: &str, latency_ms: f64) {
    CONSENSUS_BATCHES_TOTAL.with_label_values(&[result]).inc();
    CONSENSUS_LATENCY_MS.observe(latency_ms);
}

/// Registra feedback procesado
pub fn record_feedback_processed(decision: &str) {
    FEEDBACK_PROCESSED_TOTAL
        .with_label_values(&[decision])
        .inc();
}

/// Registra error de sandbox WASM
pub fn inc_wasm_sandbox_errors() {
    WASM_SANDBOX_ERRORS_TOTAL.inc();
}

/// Actualiza número de peers conectados
pub fn set_peers_connected(count: f64) {
    PEERS_CONNECTED.set(count);
}

/// Actualiza uso de memoria WASM
pub fn set_wasm_memory_usage(bytes: u64) {
    WASM_MEMORY_USAGE_BYTES.set(bytes as f64);
}

/// Actualiza entradas del feedback store
pub fn set_feedback_store_entries(count: u64) {
    FEEDBACK_STORE_ENTRIES.set(count as f64);
}

/// Actualiza batches de entrenamiento listos
pub fn set_training_batches_ready(count: u64) {
    TRAINING_BATCHES_READY.set(count as f64);
}

/// Actualiza score promedio de reputación
pub fn set_peer_reputation_avg(score: f64) {
    PEER_REPUTATION_AVG.set(score);
}

/// Registra latencia de verificación ZKP
pub fn record_zkp_verification_latency(latency_ms: f64) {
    ZKP_VERIFICATION_LATENCY_MS.observe(latency_ms);
}

impl Default for MetricsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_manager_creation() {
        let manager = MetricsManager::new();
        assert!(manager.uptime_seconds() >= 0);
    }

    #[test]
    fn test_encode_metrics() {
        let manager = MetricsManager::new();
        let metrics = manager.encode_metrics();
        // Debe contener al menos algún texto de métricas
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_counter_increment() {
        let before = SAE_INFERENCE_TOTAL.get();
        record_sae_inference(10.0);
        let after = SAE_INFERENCE_TOTAL.get();
        assert_eq!(after, before + 1.0);
    }

    #[test]
    fn test_gauge_update() {
        set_peers_connected(42.0);
        assert_eq!(PEERS_CONNECTED.get(), 42.0);
    }
}
