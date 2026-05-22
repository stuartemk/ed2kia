//! Realtime Telemetry — WebSocket/SSE streaming backend for real-time telemetry
//!
//! Provides Server-Sent Events (SSE) and WebSocket-like session management
//! for streaming telemetry data to connected clients. Supports event filtering,
//! rate limiting, and automatic reconnection handling.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Tipo de evento de telemetría en tiempo real
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum TelemetryEventType {
    /// Métricas de rendimiento (latencia, throughput)
    Metrics,
    /// Eventos de gobernanza (votos, propuestas)
    Governance,
    /// Eventos de red (peers, mensajes)
    Network,
    /// Eventos de SLO/SLA (breaches, recovery)
    Slo,
    /// Eventos de seguridad (alertas, auditoría)
    Security,
    /// Eventos de sistema (CPU, memoria)
    System,
}

impl std::fmt::Display for TelemetryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryEventType::Metrics => write!(f, "metrics"),
            TelemetryEventType::Governance => write!(f, "governance"),
            TelemetryEventType::Network => write!(f, "network"),
            TelemetryEventType::Slo => write!(f, "slo"),
            TelemetryEventType::Security => write!(f, "security"),
            TelemetryEventType::System => write!(f, "system"),
        }
    }
}

/// Evento de telemetría para streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Identificador único del evento
    pub event_id: String,
    /// Tipo de evento
    pub event_type: TelemetryEventType,
    /// Timestamp en milisegundos UNIX
    pub timestamp_ms: u64,
    /// Payload del evento (JSON)
    pub payload: serde_json::Value,
    /// Nodo fuente (opcional)
    pub source_node: Option<String>,
    /// Secuencia para ordenamiento
    pub sequence: u64,
}

impl TelemetryEvent {
    /// Crea un nuevo evento de telemetría
    pub fn new(
        event_type: TelemetryEventType,
        payload: serde_json::Value,
        source_node: Option<String>,
        sequence: u64,
    ) -> Self {
        Self {
            event_id: format!("evt-{}", current_timestamp_ms()),
            event_type,
            timestamp_ms: current_timestamp_ms(),
            payload,
            source_node,
            sequence,
        }
    }
}

/// Resultado de operación SSE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseResult {
    /// ID de sesión
    pub session_id: String,
    /// Eventos enviados
    pub events_sent: usize,
    /// Eventos filtrados
    pub events_filtered: usize,
    /// Rate limited
    pub rate_limited: bool,
}

impl SseResult {
    /// Crea resultado exitoso
    pub fn success(session_id: String, events_sent: usize, events_filtered: usize) -> Self {
        Self {
            session_id,
            events_sent,
            events_filtered,
            rate_limited: false,
        }
    }

    /// Crea resultado con rate limit
    pub fn rate_limited(session_id: String) -> Self {
        Self {
            session_id,
            events_sent: 0,
            events_filtered: 0,
            rate_limited: true,
        }
    }
}

/// Estado de sesión SSE
#[derive(Debug, Clone)]
pub struct SseSession {
    /// ID de sesión
    pub session_id: String,
    /// Tipos de evento suscritos
    pub subscribed_types: Vec<TelemetryEventType>,
    /// Timestamp de última actividad
    pub last_activity: Instant,
    /// Eventos enviados totales
    pub total_events_sent: usize,
    /// Eventos en cola pendientes
    pub pending_events: VecDeque<TelemetryEvent>,
    /// Límite de eventos por segundo
    pub rate_limit_per_sec: usize,
    /// Contador de eventos en ventana actual
    pub current_window_count: usize,
    /// Inicio de ventana actual
    pub window_start: Instant,
    /// Activa
    pub active: bool,
}

impl SseSession {
    /// Crea nueva sesión SSE
    pub fn new(
        session_id: String,
        subscribed_types: Vec<TelemetryEventType>,
        rate_limit_per_sec: usize,
    ) -> Self {
        Self {
            session_id,
            subscribed_types,
            last_activity: Instant::now(),
            total_events_sent: 0,
            pending_events: VecDeque::new(),
            rate_limit_per_sec,
            current_window_count: 0,
            window_start: Instant::now(),
            active: true,
        }
    }

    /// Verifica si el evento debe ser enviado a esta sesión
    pub fn is_subscribed(&self, event_type: &TelemetryEventType) -> bool {
        self.subscribed_types.contains(event_type)
    }

    /// Verifica rate limit y resetea ventana si es necesario
    pub fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= Duration::from_secs(1) {
            self.current_window_count = 0;
            self.window_start = now;
        }
        self.current_window_count < self.rate_limit_per_sec
    }

    /// Registra envío de evento
    pub fn record_event(&mut self) {
        self.current_window_count += 1;
        self.total_events_sent += 1;
        self.last_activity = Instant::now();
    }

    /// Agrega evento a cola pendiente
    pub fn queue_event(&mut self, event: TelemetryEvent) {
        self.pending_events.push_back(event);
    }

    /// Marca sesión como inactiva
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Verifica si la sesión expiró
    pub fn is_expired(&self, max_idle_secs: u64) -> bool {
        self.last_activity.elapsed() > Duration::from_secs(max_idle_secs)
    }
}

/// Configuración del backend de telemetría
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Límite de eventos por segundo por sesión
    pub rate_limit_per_sec: usize,
    /// Máximo de sesiones concurrentes
    pub max_sessions: usize,
    /// Tiempo máximo de inactividad antes de expirar (segundos)
    pub session_timeout_secs: u64,
    /// Tamaño máximo del buffer de eventos históricos
    pub max_history_size: usize,
    /// Habilitar compresión SSE
    pub enable_compression: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_sec: 100,
            max_sessions: 50,
            session_timeout_secs: 300,
            max_history_size: 1000,
            enable_compression: false,
        }
    }
}

/// Estadísticas del backend de telemetría
#[derive(Debug, Clone)]
pub struct TelemetryStats {
    /// Total de sesiones activas
    pub active_sessions: usize,
    /// Total de eventos enviados
    pub total_events_sent: usize,
    /// Total de eventos filtrados
    pub total_events_filtered: usize,
    /// Total de eventos rate-limited
    pub total_rate_limited: usize,
    /// Eventos históricos en buffer
    pub history_size: usize,
    /// Secuencia actual
    pub current_sequence: u64,
    /// Latencia promedio de broadcast (ms)
    pub avg_broadcast_latency_ms: f64,
}

/// Backend de telemetría en tiempo real
pub struct TelemetryBackend {
    /// Configuración
    config: TelemetryConfig,
    /// Sesiones SSE activas
    sessions: HashMap<String, SseSession>,
    /// Buffer de eventos históricos
    event_history: VecDeque<TelemetryEvent>,
    /// Contador de secuencia
    sequence_counter: u64,
    /// Estadísticas
    total_events_sent: usize,
    total_events_filtered: usize,
    total_rate_limited: usize,
    /// Latencia de broadcast acumulada (ms)
    broadcast_latency_sum: f64,
    broadcast_count: usize,
}

impl TelemetryBackend {
    /// Crea nuevo backend con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(TelemetryConfig::default())
    }

    /// Crea backend con configuración personalizada
    pub fn with_config(config: TelemetryConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            event_history: VecDeque::new(),
            sequence_counter: 0,
            total_events_sent: 0,
            total_events_filtered: 0,
            total_rate_limited: 0,
            broadcast_latency_sum: 0.0,
            broadcast_count: 0,
        }
    }

    /// Crea nueva sesión SSE
    pub fn create_session(
        &mut self,
        session_id: String,
        subscribed_types: Vec<TelemetryEventType>,
    ) -> Result<SseResult, String> {
        if self.sessions.len() >= self.config.max_sessions {
            return Err(format!(
                "Maximum sessions reached ({})",
                self.config.max_sessions
            ));
        }

        let session = SseSession::new(
            session_id.clone(),
            subscribed_types,
            self.config.rate_limit_per_sec,
        );

        self.sessions.insert(session_id.clone(), session);
        info!(session = %session_id, "Created SSE session");

        Ok(SseResult::success(session_id, 0, 0))
    }

    /// Cierra sesión SSE
    pub fn close_session(&mut self, session_id: &str) -> Result<(), String> {
        match self.sessions.remove(session_id) {
            Some(_) => {
                info!(session = %session_id, "Closed SSE session");
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Publica evento a todas las sesiones suscritas
    pub fn publish_event(
        &mut self,
        event_type: TelemetryEventType,
        payload: serde_json::Value,
        source_node: Option<String>,
    ) -> SseResult {
        let start = Instant::now();

        self.sequence_counter += 1;
        let event = TelemetryEvent::new(event_type, payload, source_node, self.sequence_counter);

        let mut events_sent = 0;
        let mut events_filtered = 0;
        let mut has_rate_limited = false;

        // Agregar al buffer histórico
        self.event_history.push_back(event.clone());
        while self.event_history.len() > self.config.max_history_size {
            self.event_history.pop_front();
        }

        // Broadcast a sesiones activas
        let session_ids: Vec<String> = self.sessions.keys().cloned().collect();
        for session_id in session_ids {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                if !session.active {
                    continue;
                }

                if !session.is_subscribed(&event_type) {
                    events_filtered += 1;
                    continue;
                }

                if !session.check_rate_limit() {
                    session.queue_event(event.clone());
                    has_rate_limited = true;
                    self.total_rate_limited += 1;
                    continue;
                }

                session.record_event();
                events_sent += 1;
                self.total_events_sent += 1;
                debug!(
                    session = %session_id,
                    event_type = %event_type,
                    "Sent telemetry event"
                );
            }
        }

        let latency = start.elapsed().as_micros() as f64 / 1000.0;
        self.broadcast_latency_sum += latency;
        self.broadcast_count += 1;

        if has_rate_limited {
            SseResult::rate_limited("broadcast".to_string())
        } else {
            SseResult::success("broadcast".to_string(), events_sent, events_filtered)
        }
    }

    /// Obtiene eventos históricos para catch-up de sesión
    pub fn get_catchup_events(
        &self,
        from_sequence: u64,
        event_types: &[TelemetryEventType],
    ) -> Vec<TelemetryEvent> {
        self.event_history
            .iter()
            .filter(|e| e.sequence > from_sequence && event_types.contains(&e.event_type))
            .cloned()
            .collect()
    }

    /// Genera payload SSE formateado
    pub fn format_sse_event(event: &TelemetryEvent) -> String {
        let payload_json = serde_json::to_string(event).unwrap_or_default();
        format!(
            "id: {}\nevent: {}\ndata: {}\n\n",
            event.event_id, event.event_type, payload_json
        )
    }

    /// Limpia sesiones expiradas
    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|_id, session| {
            if session.is_expired(self.config.session_timeout_secs) {
                session.deactivate();
                false
            } else {
                true
            }
        });
        let removed = before.saturating_sub(self.sessions.len());
        if removed > 0 {
            info!(removed, "Cleaned up expired SSE sessions");
        }
        removed
    }

    /// Obtiene estadísticas
    pub fn get_stats(&self) -> TelemetryStats {
        let active_sessions = self.sessions.values().filter(|s| s.active).count();
        TelemetryStats {
            active_sessions,
            total_events_sent: self.total_events_sent,
            total_events_filtered: self.total_events_filtered,
            total_rate_limited: self.total_rate_limited,
            history_size: self.event_history.len(),
            current_sequence: self.sequence_counter,
            avg_broadcast_latency_ms: if self.broadcast_count > 0 {
                self.broadcast_latency_sum / self.broadcast_count as f64
            } else {
                0.0
            },
        }
    }

    /// Resetea backend
    pub fn reset(&mut self) {
        self.sessions.clear();
        self.event_history.clear();
        self.sequence_counter = 0;
        self.total_events_sent = 0;
        self.total_events_filtered = 0;
        self.total_rate_limited = 0;
        self.broadcast_latency_sum = 0.0;
        self.broadcast_count = 0;
    }
}

/// Obtiene timestamp actual en milisegundos UNIX
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Default for TelemetryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = TelemetryBackend::new();
        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.current_sequence, 0);
    }

    #[test]
    fn test_backend_with_config() {
        let config = TelemetryConfig {
            rate_limit_per_sec: 50,
            max_sessions: 10,
            ..Default::default()
        };
        let backend = TelemetryBackend::with_config(config);
        assert_eq!(backend.config.rate_limit_per_sec, 50);
    }

    #[test]
    fn test_create_session() {
        let mut backend = TelemetryBackend::new();
        let result =
            backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        assert!(result.is_ok());
        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 1);
    }

    #[test]
    fn test_create_session_max_reached() {
        let config = TelemetryConfig {
            max_sessions: 1,
            ..Default::default()
        };
        let mut backend = TelemetryBackend::with_config(config);
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        let result =
            backend.create_session("sess-2".to_string(), vec![TelemetryEventType::Metrics]);
        assert!(result.is_err());
    }

    #[test]
    fn test_close_session() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        assert!(backend.close_session("sess-1").is_ok());
        assert!(backend.close_session("nonexistent").is_err());
    }

    #[test]
    fn test_publish_event_to_subscribed() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        let result = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"cpu": 42.0}),
            Some("node-1".to_string()),
        );
        assert_eq!(result.events_sent, 1);
        assert_eq!(result.events_filtered, 0);
    }

    #[test]
    fn test_publish_event_filtered() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        let result = backend.publish_event(
            TelemetryEventType::Governance,
            serde_json::json!({"vote": true}),
            None,
        );
        assert_eq!(result.events_sent, 0);
        assert_eq!(result.events_filtered, 1);
    }

    #[test]
    fn test_publish_event_increases_sequence() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 2}),
            None,
        );
        let stats = backend.get_stats();
        assert_eq!(stats.current_sequence, 2);
    }

    #[test]
    fn test_rate_limiting() {
        let config = TelemetryConfig {
            rate_limit_per_sec: 2,
            ..Default::default()
        };
        let mut backend = TelemetryBackend::with_config(config);
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        let r1 = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        let r2 = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 2}),
            None,
        );
        let r3 = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 3}),
            None,
        );
        assert_eq!(r1.events_sent, 1);
        assert_eq!(r2.events_sent, 1);
        assert!(r3.rate_limited);
    }

    #[test]
    fn test_get_catchup_events() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session(
            "sess-1".to_string(),
            vec![TelemetryEventType::Metrics, TelemetryEventType::Governance],
        );
        backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        backend.publish_event(
            TelemetryEventType::Governance,
            serde_json::json!({"v": 2}),
            None,
        );
        backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 3}),
            None,
        );
        let catchup = backend.get_catchup_events(1, &[TelemetryEventType::Metrics]);
        assert_eq!(catchup.len(), 1);
        assert_eq!(catchup[0].sequence, 3);
    }

    #[test]
    fn test_format_sse_event() {
        let event = TelemetryEvent::new(
            TelemetryEventType::Metrics,
            serde_json::json!({"cpu": 50.0}),
            Some("node-1".to_string()),
            1,
        );
        let sse = TelemetryBackend::format_sse_event(&event);
        assert!(sse.contains("event: metrics"));
        assert!(sse.contains("data:"));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let config = TelemetryConfig {
            session_timeout_secs: 0,
            ..Default::default()
        };
        let mut backend = TelemetryBackend::with_config(config);
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        let removed = backend.cleanup_expired_sessions();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 1);
        assert_eq!(stats.total_events_sent, 1);
        assert_eq!(stats.history_size, 1);
        assert!(stats.avg_broadcast_latency_ms >= 0.0);
    }

    #[test]
    fn test_reset() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        backend.reset();
        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_events_sent, 0);
        assert_eq!(stats.current_sequence, 0);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(TelemetryEventType::Metrics.to_string(), "metrics");
        assert_eq!(TelemetryEventType::Governance.to_string(), "governance");
        assert_eq!(TelemetryEventType::Network.to_string(), "network");
        assert_eq!(TelemetryEventType::Slo.to_string(), "slo");
        assert_eq!(TelemetryEventType::Security.to_string(), "security");
        assert_eq!(TelemetryEventType::System.to_string(), "system");
    }

    #[test]
    fn test_sse_result_success() {
        let result = SseResult::success("s1".to_string(), 5, 2);
        assert_eq!(result.events_sent, 5);
        assert_eq!(result.events_filtered, 2);
        assert!(!result.rate_limited);
    }

    #[test]
    fn test_sse_result_rate_limited() {
        let result = SseResult::rate_limited("s1".to_string());
        assert!(result.rate_limited);
        assert_eq!(result.events_sent, 0);
    }

    #[test]
    fn test_session_subscription_check() {
        let session = SseSession::new(
            "s1".to_string(),
            vec![TelemetryEventType::Metrics, TelemetryEventType::Network],
            100,
        );
        assert!(session.is_subscribed(&TelemetryEventType::Metrics));
        assert!(!session.is_subscribed(&TelemetryEventType::Governance));
    }

    #[test]
    fn test_session_rate_limit_reset() {
        let mut session = SseSession::new("s1".to_string(), vec![TelemetryEventType::Metrics], 2);
        assert!(session.check_rate_limit());
        session.record_event();
        session.record_event();
        assert!(!session.check_rate_limit());
    }

    #[test]
    fn test_session_deactivate() {
        let mut session = SseSession::new("s1".to_string(), vec![TelemetryEventType::Metrics], 100);
        assert!(session.active);
        session.deactivate();
        assert!(!session.active);
    }

    #[test]
    fn test_session_queue_event() {
        let mut session = SseSession::new("s1".to_string(), vec![TelemetryEventType::Metrics], 100);
        let event = TelemetryEvent::new(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
            1,
        );
        session.queue_event(event);
        assert_eq!(session.pending_events.len(), 1);
    }

    #[test]
    fn test_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.rate_limit_per_sec, 100);
        assert_eq!(config.max_sessions, 50);
        assert_eq!(config.session_timeout_secs, 300);
        assert_eq!(config.max_history_size, 1000);
    }

    #[test]
    fn test_telemetry_event_creation() {
        let event = TelemetryEvent::new(
            TelemetryEventType::Metrics,
            serde_json::json!({"cpu": 50.0}),
            Some("node-1".to_string()),
            42,
        );
        assert_eq!(event.event_type, TelemetryEventType::Metrics);
        assert_eq!(event.sequence, 42);
        assert!(event.source_node.is_some());
        assert!(event.event_id.starts_with("evt-"));
    }

    #[test]
    fn test_history_buffer_size_limit() {
        let config = TelemetryConfig {
            max_history_size: 3,
            ..Default::default()
        };
        let mut backend = TelemetryBackend::with_config(config);
        let _ = backend.create_session("sess-1".to_string(), vec![TelemetryEventType::Metrics]);
        for i in 0..5 {
            backend.publish_event(
                TelemetryEventType::Metrics,
                serde_json::json!({"v": i}),
                None,
            );
        }
        assert_eq!(backend.event_history.len(), 3);
        assert_eq!(backend.event_history.front().unwrap().sequence, 3);
    }

    #[test]
    fn test_multiple_sessions_different_subscriptions() {
        let mut backend = TelemetryBackend::new();
        let _ = backend.create_session("s1".to_string(), vec![TelemetryEventType::Metrics]);
        let _ = backend.create_session("s2".to_string(), vec![TelemetryEventType::Governance]);
        let result = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        assert_eq!(result.events_sent, 1);
        assert_eq!(result.events_filtered, 1);
    }

    #[test]
    fn test_publish_to_no_sessions() {
        let mut backend = TelemetryBackend::new();
        let result = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"v": 1}),
            None,
        );
        assert_eq!(result.events_sent, 0);
        assert_eq!(result.events_filtered, 0);
    }

    #[test]
    fn test_default() {
        let backend = TelemetryBackend::default();
        assert_eq!(backend.get_stats().active_sessions, 0);
    }
}
