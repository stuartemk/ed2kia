//! SSE Metrics Stream — Streaming de métricas vía Server-Sent Events (Sprint 4)
//!
//! LP-33: Real-time UI/Backend
//! Proporciona streaming de métricas de rendimiento, alineación y federación
//! vía Server-Sent Events con filtrado por categoría, rate limiting y
//! reconexión automática con Last-Event-ID.
//!
//! Características:
//! - Streaming SSE de métricas de alineación, federación y sistema
//! - Filtrado por categoría de métrica
//! - Rate limiting por sesión con ventanas deslizantes
//! - Reconexión automática vía Last-Event-ID
//! - Formato SSE estándar (event, data, id, retry)
//! - Estadísticas de latencia, throughput y utilization
//!
//! Protegido con `#[cfg(feature = "v1.1-sprint4")]`.

#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint4")]
use std::time::Instant;
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::info;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Error, Debug)]
pub enum SseMetricsError {
    #[error("Sesión no encontrada: {0}")]
    SessionNotFound(String),

    #[error("Rate limit excedido: {current}/{max} events/s")]
    RateLimitExceeded { current: usize, max: usize },

    #[error("Categoría de métrica inválida: {0}")]
    InvalidMetricCategory(String),

    #[error("Sesión ya existe: {0}")]
    SessionAlreadyExists(String),

    #[error("Máximo de sesiones alcanzado: {0}")]
    MaxSessionsReached(usize),

    #[error("Last-Event-ID inválido: {0}")]
    InvalidLastEventId(String),

    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

// ─── Metric Categories ────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MetricCategory {
    /// Métricas de alineación (drift, steering, confidence)
    Alignment,
    /// Métricas de federación (gradient sync, trust, divergence)
    Federation,
    /// Métricas de rendimiento (latencia, throughput)
    Performance,
    /// Métricas de recursos (CPU, memoria, GPU)
    Resources,
    /// Métricas de gobernanza (votos, SLO)
    Governance,
    /// Métricas de seguridad (ZKP, Sybil)
    Security,
}

#[cfg(feature = "v1.1-sprint4")]
impl std::fmt::Display for MetricCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricCategory::Alignment => write!(f, "alignment"),
            MetricCategory::Federation => write!(f, "federation"),
            MetricCategory::Performance => write!(f, "performance"),
            MetricCategory::Resources => write!(f, "resources"),
            MetricCategory::Governance => write!(f, "governance"),
            MetricCategory::Security => write!(f, "security"),
        }
    }
}

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub labels: HashMap<String, String>,
}

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseMetricEvent {
    pub event_id: String,
    pub category: MetricCategory,
    pub timestamp_ms: u64,
    pub metrics: Vec<MetricPoint>,
    pub sequence: u64,
    pub source_node: Option<String>,
}

#[cfg(feature = "v1.1-sprint4")]
impl SseMetricEvent {
    pub fn new(
        category: MetricCategory,
        metrics: Vec<MetricPoint>,
        sequence: u64,
        source_node: Option<String>,
    ) -> Self {
        Self {
            event_id: format!(
                "metric-{}-{}-{}",
                category,
                current_timestamp_ms(),
                sequence
            ),
            category,
            timestamp_ms: current_timestamp_ms(),
            metrics,
            sequence,
            source_node,
        }
    }
}

// ─── Session State ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone)]
pub struct SseMetricsSession {
    pub session_id: String,
    pub subscribed_categories: Vec<MetricCategory>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub last_event_id: Option<String>,
    pub last_sequence_seen: u64,
    pub events_sent: usize,
    pub rate_limit_per_sec: usize,
    pub current_window_count: usize,
    pub window_start: Instant,
    pub pending_events: VecDeque<SseMetricEvent>,
    pub max_buffer_size: usize,
    pub active: bool,
    pub retry_interval_ms: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl SseMetricsSession {
    pub fn new(
        session_id: String,
        subscribed_categories: Vec<MetricCategory>,
        rate_limit_per_sec: usize,
    ) -> Self {
        Self {
            session_id,
            subscribed_categories,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            last_event_id: None,
            last_sequence_seen: 0,
            events_sent: 0,
            rate_limit_per_sec,
            current_window_count: 0,
            window_start: Instant::now(),
            pending_events: VecDeque::new(),
            max_buffer_size: 1000,
            active: true,
            retry_interval_ms: 3000,
        }
    }

    /// Verificar rate limit.
    pub fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start).as_secs() >= 1 {
            self.current_window_count = 0;
            self.window_start = now;
        }
        self.current_window_count < self.rate_limit_per_sec
    }

    /// Registrar evento enviado.
    pub fn record_event(&mut self) {
        self.events_sent += 1;
        self.current_window_count += 1;
        self.last_activity = Instant::now();
    }

    /// Buffer event con backpressure.
    pub fn buffer_event(&mut self, event: SseMetricEvent) -> bool {
        if self.pending_events.len() < self.max_buffer_size {
            self.pending_events.push_back(event);
            true
        } else {
            false
        }
    }

    /// Verificar suscripción.
    pub fn is_subscribed(&self, category: &MetricCategory) -> bool {
        self.subscribed_categories.contains(category)
    }

    /// Obtener eventos pendientes desde secuencia.
    pub fn get_pending_since(&mut self, since_sequence: u64) -> Vec<SseMetricEvent> {
        let mut result = Vec::new();
        while let Some(event) = self.pending_events.front() {
            if event.sequence > since_sequence {
                result.push(self.pending_events.pop_front().unwrap());
            } else if event.sequence <= since_sequence {
                // Saltar eventos ya vistos
                self.pending_events.pop_front();
            } else {
                break;
            }
        }
        result
    }

    /// Verificar expiración.
    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        self.connected_at.elapsed().as_secs() > max_age_secs
    }

    /// Desactivar sesión.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

// ─── Stream Result ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseStreamResult {
    pub session_id: String,
    pub events_sent: usize,
    pub events_buffered: usize,
    pub events_dropped: usize,
    pub rate_limited: bool,
    pub active_sessions: usize,
    pub last_sequence: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl SseStreamResult {
    pub fn success(
        session_id: String,
        events_sent: usize,
        events_buffered: usize,
        active_sessions: usize,
        last_sequence: u64,
    ) -> Self {
        Self {
            session_id,
            events_sent,
            events_buffered,
            events_dropped: 0,
            rate_limited: false,
            active_sessions,
            last_sequence,
        }
    }

    pub fn rate_limited(session_id: String, active_sessions: usize) -> Self {
        Self {
            session_id,
            events_sent: 0,
            events_buffered: 0,
            events_dropped: 0,
            rate_limited: true,
            active_sessions,
            last_sequence: 0,
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseMetricsConfig {
    pub max_sessions: usize,
    pub rate_limit_per_sec: usize,
    pub max_session_age_secs: u64,
    pub event_history_size: usize,
    pub enable_backpressure: bool,
    pub default_retry_ms: u64,
    pub heartbeat_interval_ms: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for SseMetricsConfig {
    fn default() -> Self {
        Self {
            max_sessions: 100,
            rate_limit_per_sec: 50,
            max_session_age_secs: 7200,
            event_history_size: 5000,
            enable_backpressure: true,
            default_retry_ms: 3000,
            heartbeat_interval_ms: 15000,
        }
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseMetricsStats {
    pub active_sessions: usize,
    pub total_events_sent: u64,
    pub total_events_dropped: u64,
    pub total_rate_limited: u64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub events_per_second: f64,
    pub buffer_utilization: f64,
    pub last_updated_ms: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for SseMetricsStats {
    fn default() -> Self {
        Self {
            active_sessions: 0,
            total_events_sent: 0,
            total_events_dropped: 0,
            total_rate_limited: 0,
            avg_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            events_per_second: 0.0,
            buffer_utilization: 0.0,
            last_updated_ms: 0,
        }
    }
}

// ─── SseMetricsStream ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
pub struct SseMetricsStream {
    pub config: SseMetricsConfig,
    pub sessions: HashMap<String, SseMetricsSession>,
    pub event_history: VecDeque<SseMetricEvent>,
    pub stats: SseMetricsStats,
    pub sequence_counter: u64,
    pub latency_samples: VecDeque<f64>,
}

#[cfg(feature = "v1.1-sprint4")]
impl SseMetricsStream {
    pub fn new() -> Self {
        Self {
            config: SseMetricsConfig::default(),
            sessions: HashMap::new(),
            event_history: VecDeque::new(),
            stats: SseMetricsStats::default(),
            sequence_counter: 0,
            latency_samples: VecDeque::new(),
        }
    }

    pub fn with_config(config: SseMetricsConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            event_history: VecDeque::new(),
            stats: SseMetricsStats::default(),
            sequence_counter: 0,
            latency_samples: VecDeque::new(),
        }
    }

    /// Crear nueva sesión SSE.
    pub fn create_session(
        &mut self,
        session_id: String,
        subscribed_categories: Vec<MetricCategory>,
    ) -> Result<SseMetricsSession, SseMetricsError> {
        if self.sessions.contains_key(&session_id) {
            return Err(SseMetricsError::SessionAlreadyExists(session_id));
        }

        if self.sessions.len() >= self.config.max_sessions {
            return Err(SseMetricsError::MaxSessionsReached(
                self.config.max_sessions,
            ));
        }

        let session = SseMetricsSession::new(
            session_id.clone(),
            subscribed_categories,
            self.config.rate_limit_per_sec,
        );

        self.sessions.insert(session_id.clone(), session);
        info!(
            "Sesión SSE creada: {} ({} sesiones activas)",
            session_id,
            self.sessions.len()
        );

        Ok(self.sessions.get(&session_id).unwrap().clone())
    }

    /// Reconectar con Last-Event-ID.
    pub fn reconnect_with_last_event(
        &mut self,
        session_id: String,
        last_event_id: String,
        subscribed_categories: Vec<MetricCategory>,
    ) -> Result<SseMetricsSession, SseMetricsError> {
        // Parsear Last-Event-ID para obtener secuencia
        let last_sequence = parse_event_id(&last_event_id)
            .map_err(|e| SseMetricsError::InvalidLastEventId(e.to_string()))?;

        let mut session = self.create_session(session_id.clone(), subscribed_categories)?;
        session.last_event_id = Some(last_event_id);
        session.last_sequence_seen = last_sequence;

        info!(
            "Reconexión SSE: {} desde evento (secuencia {})",
            session_id, last_sequence
        );

        Ok(session)
    }

    /// Cerrar sesión.
    pub fn close_session(&mut self, session_id: &str) -> Result<(), SseMetricsError> {
        match self.sessions.remove(session_id) {
            Some(_) => {
                info!("Sesión SSE cerrada: {}", session_id);
                Ok(())
            }
            None => Err(SseMetricsError::SessionNotFound(session_id.to_string())),
        }
    }

    /// Publicar evento de métricas.
    pub fn publish_metrics(
        &mut self,
        category: MetricCategory,
        metrics: Vec<MetricPoint>,
        source_node: Option<String>,
    ) -> SseStreamResult {
        self.sequence_counter += 1;
        let event = SseMetricEvent::new(
            category.clone(),
            metrics,
            self.sequence_counter,
            source_node,
        );

        let start = Instant::now();
        let mut events_sent = 0;
        let mut events_buffered = 0;
        let mut _events_dropped = 0;
        let mut _rate_limited = false;

        for session in self.sessions.values_mut() {
            if !session.active || !session.is_subscribed(&event.category) {
                continue;
            }

            if !session.check_rate_limit() {
                _rate_limited = true;
                self.stats.total_rate_limited += 1;

                if self.config.enable_backpressure {
                    if session.buffer_event(event.clone()) {
                        events_buffered += 1;
                    } else {
                        _events_dropped += 1;
                        self.stats.total_events_dropped += 1;
                    }
                } else {
                    _events_dropped += 1;
                    self.stats.total_events_dropped += 1;
                }
                continue;
            }

            session.record_event();
            session.last_sequence_seen = event.sequence;
            session.last_event_id = Some(event.event_id.clone());
            events_sent += 1;
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.latency_samples.push_back(elapsed_ms);
        if self.latency_samples.len() > 1000 {
            self.latency_samples.pop_front();
        }

        // Guardar en historial
        self.event_history.push_back(event);
        while self.event_history.len() > self.config.event_history_size {
            self.event_history.pop_front();
        }

        self.stats.total_events_sent += events_sent as u64;
        self.update_stats();

        SseStreamResult::success(
            "broadcast".to_string(),
            events_sent,
            events_buffered,
            self.sessions.len(),
            self.sequence_counter,
        )
    }

    /// Obtener eventos de catch-up.
    pub fn get_catchup_events(&self, session_id: &str, since_sequence: u64) -> Vec<SseMetricEvent> {
        let session = match self.sessions.get(session_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        self.event_history
            .iter()
            .filter(|e| e.sequence > since_sequence && session.is_subscribed(&e.category))
            .cloned()
            .collect()
    }

    /// Formatear evento como SSE.
    pub fn format_sse_event(event: &SseMetricEvent) -> String {
        let data = serde_json::to_string(event).unwrap_or_default();
        format!(
            "event: {}\ndata: {}\nid: {}\nretry: {}\ntimestamp: {}\n\n",
            event.category,
            data,
            event.event_id,
            3000, // retry en ms
            event.timestamp_ms,
        )
    }

    /// Generar evento heartbeat.
    pub fn generate_heartbeat(&self) -> SseMetricEvent {
        SseMetricEvent {
            event_id: format!("heartbeat-{}", current_timestamp_ms()),
            category: MetricCategory::Performance,
            timestamp_ms: current_timestamp_ms(),
            metrics: vec![MetricPoint {
                metric_name: "heartbeat".to_string(),
                value: current_timestamp_ms() as f64,
                unit: "timestamp_ms".to_string(),
                labels: HashMap::new(),
            }],
            sequence: self.sequence_counter,
            source_node: None,
        }
    }

    /// Limpiar sesiones expiradas.
    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions
            .retain(|_, s| !s.is_expired(self.config.max_session_age_secs));
        let removed = before - self.sessions.len();
        if removed > 0 {
            info!("{} sesiones SSE expiradas limpiadas", removed);
        }
        self.update_stats();
        removed
    }

    /// Obtener estadísticas.
    pub fn get_stats(&self) -> SseMetricsStats {
        self.stats.clone()
    }

    /// Obtener sesión.
    pub fn get_session(&self, session_id: &str) -> Option<&SseMetricsSession> {
        self.sessions.get(session_id)
    }

    /// Reiniciar estadísticas.
    pub fn reset_stats(&mut self) {
        self.stats = SseMetricsStats::default();
        self.latency_samples.clear();
    }

    /// Actualizar estadísticas.
    fn update_stats(&mut self) {
        self.stats.active_sessions = self.sessions.len();
        self.stats.last_updated_ms = current_timestamp_ms();

        if !self.latency_samples.is_empty() {
            let sum: f64 = self.latency_samples.iter().sum();
            self.stats.avg_latency_ms = sum / self.latency_samples.len() as f64;

            let mut sorted: Vec<f64> = self.latency_samples.iter().cloned().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
            self.stats.p99_latency_ms = *sorted.get(p99_idx.min(sorted.len() - 1)).unwrap_or(&0.0);
        }

        let total_buffer: usize = self.sessions.values().map(|s| s.pending_events.len()).sum();
        let max_buffer: usize = self.sessions.values().map(|s| s.max_buffer_size).sum();
        self.stats.buffer_utilization = if max_buffer > 0 {
            total_buffer as f64 / max_buffer as f64
        } else {
            0.0
        };
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for SseMetricsStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "v1.1-sprint4")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(feature = "v1.1-sprint4")]
fn parse_event_id(event_id: &str) -> Result<u64, Box<dyn std::error::Error>> {
    // Formato: "metric-category-timestamp-sequence"
    let parts: Vec<&str> = event_id.split('-').collect();
    if parts.len() >= 4 {
        let sequence = parts.last().unwrap().parse::<u64>()?;
        Ok(sequence)
    } else {
        // Si no se puede parsear, retornar 0 (catch-up completo)
        Ok(0)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "v1.1-sprint4"))]
mod tests {
    use super::*;

    #[test]
    fn test_stream_creation() {
        let stream = SseMetricsStream::new();
        assert_eq!(stream.sessions.len(), 0);
        assert_eq!(stream.config.max_sessions, 100);
    }

    #[test]
    fn test_stream_with_config() {
        let config = SseMetricsConfig {
            max_sessions: 50,
            rate_limit_per_sec: 100,
            ..SseMetricsConfig::default()
        };
        let stream = SseMetricsStream::with_config(config);
        assert_eq!(stream.config.max_sessions, 50);
    }

    #[test]
    fn test_create_session() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        let session = stream.create_session("s1".to_string(), categories).unwrap();
        assert_eq!(session.session_id, "s1");
        assert_eq!(stream.sessions.len(), 1);
    }

    #[test]
    fn test_create_duplicate_session() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream
            .create_session("s1".to_string(), categories.clone())
            .unwrap();
        let result = stream.create_session("s1".to_string(), categories);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_sessions_reached() {
        let config = SseMetricsConfig {
            max_sessions: 1,
            ..SseMetricsConfig::default()
        };
        let mut stream = SseMetricsStream::with_config(config);
        let categories = vec![MetricCategory::Alignment];

        stream
            .create_session("s1".to_string(), categories.clone())
            .unwrap();
        let result = stream.create_session("s2".to_string(), categories);
        assert!(result.is_err());
    }

    #[test]
    fn test_close_session() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();
        stream.close_session("s1").unwrap();
        assert_eq!(stream.sessions.len(), 0);
    }

    #[test]
    fn test_close_unknown_session() {
        let mut stream = SseMetricsStream::new();
        let result = stream.close_session("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_publish_metrics_to_subscribed() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();

        let metrics = vec![MetricPoint {
            metric_name: "drift".to_string(),
            value: 0.1,
            unit: "score".to_string(),
            labels: HashMap::new(),
        }];

        let result = stream.publish_metrics(MetricCategory::Alignment, metrics, None);
        assert_eq!(result.events_sent, 1);
    }

    #[test]
    fn test_publish_metrics_filtered() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();

        let metrics = vec![MetricPoint {
            metric_name: "cpu".to_string(),
            value: 50.0,
            unit: "percent".to_string(),
            labels: HashMap::new(),
        }];

        let result = stream.publish_metrics(MetricCategory::Resources, metrics, None);
        assert_eq!(result.events_sent, 0);
    }

    #[test]
    fn test_get_catchup_events() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();

        let metrics = vec![MetricPoint {
            metric_name: "drift".to_string(),
            value: 0.1,
            unit: "score".to_string(),
            labels: HashMap::new(),
        }];

        stream.publish_metrics(MetricCategory::Alignment, metrics.clone(), None);
        stream.publish_metrics(MetricCategory::Alignment, metrics, None);

        let catchup = stream.get_catchup_events("s1", 0);
        assert_eq!(catchup.len(), 2);
    }

    #[test]
    fn test_format_sse_event() {
        let event = SseMetricEvent::new(
            MetricCategory::Alignment,
            vec![MetricPoint {
                metric_name: "drift".to_string(),
                value: 0.1,
                unit: "score".to_string(),
                labels: HashMap::new(),
            }],
            1,
            None,
        );
        let sse = SseMetricsStream::format_sse_event(&event);
        assert!(sse.contains("event: alignment"));
        assert!(sse.contains("data:"));
        assert!(sse.contains("id:"));
        assert!(sse.contains("retry:"));
    }

    #[test]
    fn test_generate_heartbeat() {
        let stream = SseMetricsStream::new();
        let heartbeat = stream.generate_heartbeat();
        assert_eq!(heartbeat.category, MetricCategory::Performance);
        assert!(heartbeat.metrics[0].metric_name == "heartbeat");
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();

        let session = stream.sessions.get_mut("s1").unwrap();
        session.connected_at = Instant::now() - std::time::Duration::from_secs(7201);

        let removed = stream.cleanup_expired_sessions();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();

        let metrics = vec![MetricPoint {
            metric_name: "drift".to_string(),
            value: 0.1,
            unit: "score".to_string(),
            labels: HashMap::new(),
        }];
        stream.publish_metrics(MetricCategory::Alignment, metrics, None);

        let stats = stream.get_stats();
        assert_eq!(stats.active_sessions, 1);
        assert!(stats.total_events_sent > 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut stream = SseMetricsStream::new();
        stream.reset_stats();
        let stats = stream.get_stats();
        assert_eq!(stats.total_events_sent, 0);
    }

    #[test]
    fn test_category_display() {
        assert_eq!(MetricCategory::Alignment.to_string(), "alignment");
        assert_eq!(MetricCategory::Federation.to_string(), "federation");
        assert_eq!(MetricCategory::Performance.to_string(), "performance");
    }

    #[test]
    fn test_config_default() {
        let config = SseMetricsConfig::default();
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.rate_limit_per_sec, 50);
        assert!(config.enable_backpressure);
    }

    #[test]
    fn test_stats_default() {
        let stats = SseMetricsStats::default();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_events_sent, 0);
    }

    #[test]
    fn test_stream_default() {
        let stream = SseMetricsStream::default();
        assert_eq!(stream.sessions.len(), 0);
    }

    #[test]
    fn test_session_buffer_event() {
        let mut session = SseMetricsSession::new("s1".to_string(), vec![], 100);
        let event = SseMetricEvent::new(MetricCategory::Alignment, vec![], 1, None);
        assert!(session.buffer_event(event));
        assert_eq!(session.pending_events.len(), 1);
    }

    #[test]
    fn test_session_buffer_full() {
        let mut session = SseMetricsSession::new("s1".to_string(), vec![], 100);
        session.max_buffer_size = 1;
        let event1 = SseMetricEvent::new(MetricCategory::Alignment, vec![], 1, None);
        let event2 = SseMetricEvent::new(MetricCategory::Alignment, vec![], 2, None);
        session.buffer_event(event1);
        assert!(!session.buffer_event(event2));
    }

    #[test]
    fn test_session_get_pending_since() {
        let mut session = SseMetricsSession::new("s1".to_string(), vec![], 100);
        for i in 1..=5 {
            session.pending_events.push_back(SseMetricEvent::new(
                MetricCategory::Alignment,
                vec![],
                i,
                None,
            ));
        }

        let pending = session.get_pending_since(2);
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].sequence, 3);
    }

    #[test]
    fn test_parse_event_id() {
        let seq = parse_event_id("metric-alignment-1234567890-42").unwrap();
        assert_eq!(seq, 42);
    }

    #[test]
    fn test_parse_event_id_short() {
        let seq = parse_event_id("short-id").unwrap();
        assert_eq!(seq, 0);
    }

    #[test]
    fn test_multiple_sessions_different_subscriptions() {
        let mut stream = SseMetricsStream::new();
        stream
            .create_session("s1".to_string(), vec![MetricCategory::Alignment])
            .unwrap();
        stream
            .create_session("s2".to_string(), vec![MetricCategory::Federation])
            .unwrap();

        let metrics = vec![MetricPoint {
            metric_name: "drift".to_string(),
            value: 0.1,
            unit: "score".to_string(),
            labels: HashMap::new(),
        }];
        let result = stream.publish_metrics(MetricCategory::Alignment, metrics, None);
        assert_eq!(result.events_sent, 1); // Solo s1
    }

    #[test]
    fn test_reconnect_with_last_event() {
        let mut stream = SseMetricsStream::new();
        let categories = vec![MetricCategory::Alignment];

        let session = stream
            .reconnect_with_last_event(
                "s1".to_string(),
                "metric-alignment-1234567890-10".to_string(),
                categories,
            )
            .unwrap();

        assert_eq!(session.last_sequence_seen, 10);
        assert!(session.last_event_id.is_some());
    }

    #[test]
    fn test_rate_limiting() {
        let config = SseMetricsConfig {
            rate_limit_per_sec: 2,
            ..SseMetricsConfig::default()
        };
        let mut stream = SseMetricsStream::with_config(config);
        let categories = vec![MetricCategory::Alignment];
        stream.create_session("s1".to_string(), categories).unwrap();

        let metrics = vec![MetricPoint {
            metric_name: "test".to_string(),
            value: 1.0,
            unit: "count".to_string(),
            labels: HashMap::new(),
        }];

        stream.publish_metrics(MetricCategory::Alignment, metrics.clone(), None);
        stream.publish_metrics(MetricCategory::Alignment, metrics.clone(), None);
        let result = stream.publish_metrics(MetricCategory::Alignment, metrics, None);

        assert!(result.rate_limited || result.events_sent == 0 || result.events_buffered > 0);
    }
}
