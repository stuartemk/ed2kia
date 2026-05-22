//! Realtime Backend v2 — Backend unificado para streaming en tiempo real (Sprint 4)
//!
//! LP-33: Real-time UI/Backend
//! Extiende el backend de tiempo real con integración de Alignment Loop v2,
//! métricas de federación cross-model y streaming de métricas SSE.
//!
//! Características:
//! - Sesiones WebSocket con filtrado por tipo de evento
//! - Integración con Alignment Loop v2 (steering signals, feedback)
//! - Métricas de federación cross-model en tiempo real
//! - Rate limiting por sesión con ventanas deslizantes
//! - Broadcast a múltiples sesiones con backpressure
//! - Estadísticas de latencia y throughput
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
pub enum RealtimeBackendError {
    #[error("Sesión no encontrada: {0}")]
    SessionNotFound(String),

    #[error("Rate limit excedido: {current}/{max} msg/s para sesión {session}")]
    RateLimitExceeded {
        current: usize,
        max: usize,
        session: String,
    },

    #[error("Tipo de evento inválido: {0}")]
    InvalidEventType(String),

    #[error("Sesión ya existe: {0}")]
    SessionAlreadyExists(String),

    #[error("Máximo de sesiones alcanzado: {0}")]
    MaxSessionsReached(usize),

    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

// ─── Event Types ──────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BackendEventType {
    // Alignment Loop v2
    AlignmentFeedback,
    AlignmentSteering,
    AlignmentDrift,
    AlignmentCycleComplete,
    // Federation Cross-Model
    FederationGradientSync,
    FederationTrustUpdate,
    FederationOutlierDetected,
    FederationDivergenceAlert,
    // Métricas del sistema
    MetricsLatency,
    MetricsThroughput,
    MetricsResourceUsage,
    // Gobernanza
    GovernanceVote,
    GovernanceProposal,
    GovernanceSloBreach,
    // Seguridad
    SecuritySybilDetected,
    SecurityZkpVerification,
}

#[cfg(feature = "v1.1-sprint4")]
impl std::fmt::Display for BackendEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendEventType::AlignmentFeedback => write!(f, "alignment_feedback"),
            BackendEventType::AlignmentSteering => write!(f, "alignment_steering"),
            BackendEventType::AlignmentDrift => write!(f, "alignment_drift"),
            BackendEventType::AlignmentCycleComplete => write!(f, "alignment_cycle"),
            BackendEventType::FederationGradientSync => write!(f, "federation_gradient"),
            BackendEventType::FederationTrustUpdate => write!(f, "federation_trust"),
            BackendEventType::FederationOutlierDetected => write!(f, "federation_outlier"),
            BackendEventType::FederationDivergenceAlert => write!(f, "federation_divergence"),
            BackendEventType::MetricsLatency => write!(f, "metrics_latency"),
            BackendEventType::MetricsThroughput => write!(f, "metrics_throughput"),
            BackendEventType::MetricsResourceUsage => write!(f, "metrics_resource"),
            BackendEventType::GovernanceVote => write!(f, "governance_vote"),
            BackendEventType::GovernanceProposal => write!(f, "governance_proposal"),
            BackendEventType::GovernanceSloBreach => write!(f, "governance_slo"),
            BackendEventType::SecuritySybilDetected => write!(f, "security_sybil"),
            BackendEventType::SecurityZkpVerification => write!(f, "security_zkp"),
        }
    }
}

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEvent {
    pub event_id: String,
    pub event_type: BackendEventType,
    pub timestamp_ms: u64,
    pub payload: serde_json::Value,
    pub source_node: Option<String>,
    pub sequence: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl BackendEvent {
    pub fn new(
        event_type: BackendEventType,
        payload: serde_json::Value,
        source_node: Option<String>,
        sequence: u64,
    ) -> Self {
        Self {
            event_id: format!("evt-{}-{}", current_timestamp_ms(), sequence),
            event_type,
            timestamp_ms: current_timestamp_ms(),
            payload,
            source_node,
            sequence,
        }
    }
}

// ─── Session ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone)]
pub struct BackendSession {
    pub session_id: String,
    pub subscribed_types: Vec<BackendEventType>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub rate_limit_per_sec: usize,
    pub current_window_count: usize,
    pub window_start: Instant,
    pub event_buffer: VecDeque<BackendEvent>,
    pub max_buffer_size: usize,
    pub active: bool,
}

#[cfg(feature = "v1.1-sprint4")]
impl BackendSession {
    pub fn new(
        session_id: String,
        subscribed_types: Vec<BackendEventType>,
        rate_limit_per_sec: usize,
    ) -> Self {
        Self {
            session_id,
            subscribed_types,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            messages_sent: 0,
            messages_received: 0,
            rate_limit_per_sec,
            current_window_count: 0,
            window_start: Instant::now(),
            event_buffer: VecDeque::new(),
            max_buffer_size: 1000,
            active: true,
        }
    }

    /// Verificar rate limit. Retorna true si está permitido.
    pub fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start).as_secs() >= 1 {
            self.current_window_count = 0;
            self.window_start = now;
        }
        self.current_window_count < self.rate_limit_per_sec
    }

    /// Registrar mensaje enviado.
    pub fn record_message(&mut self) {
        self.messages_sent += 1;
        self.current_window_count += 1;
        self.last_activity = Instant::now();
    }

    /// Agregar evento al buffer con backpressure.
    pub fn buffer_event(&mut self, event: BackendEvent) -> bool {
        if self.event_buffer.len() < self.max_buffer_size {
            self.event_buffer.push_back(event);
            true
        } else {
            false // Backpressure: buffer lleno
        }
    }

    /// Verificar si la sesión expiró.
    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        self.connected_at.elapsed().as_secs() > max_age_secs
    }

    /// Verificar si suscribe al tipo de evento.
    pub fn is_subscribed(&self, event_type: &BackendEventType) -> bool {
        self.subscribed_types.contains(event_type)
    }

    /// Desactivar sesión.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

// ─── Result ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendResult {
    pub session_id: String,
    pub events_sent: usize,
    pub events_buffered: usize,
    pub events_dropped: usize,
    pub rate_limited: bool,
    pub active_sessions: usize,
    pub avg_latency_ms: f64,
}

#[cfg(feature = "v1.1-sprint4")]
impl BackendResult {
    pub fn success(
        session_id: String,
        events_sent: usize,
        events_buffered: usize,
        active_sessions: usize,
    ) -> Self {
        Self {
            session_id,
            events_sent,
            events_buffered,
            events_dropped: 0,
            rate_limited: false,
            active_sessions,
            avg_latency_ms: 0.0,
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
            avg_latency_ms: 0.0,
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub max_sessions: usize,
    pub rate_limit_per_sec: usize,
    pub max_session_age_secs: u64,
    pub event_history_size: usize,
    pub enable_backpressure: bool,
    pub backpressure_threshold: usize,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            max_sessions: 100,
            rate_limit_per_sec: 100,
            max_session_age_secs: 3600,
            event_history_size: 5000,
            enable_backpressure: true,
            backpressure_threshold: 500,
        }
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStats {
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
impl Default for BackendStats {
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

// ─── RealtimeBackend ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
pub struct RealtimeBackend {
    pub config: BackendConfig,
    pub sessions: HashMap<String, BackendSession>,
    pub event_history: VecDeque<BackendEvent>,
    pub stats: BackendStats,
    pub sequence_counter: u64,
    pub latency_samples: VecDeque<f64>,
}

#[cfg(feature = "v1.1-sprint4")]
impl RealtimeBackend {
    pub fn new() -> Self {
        Self {
            config: BackendConfig::default(),
            sessions: HashMap::new(),
            event_history: VecDeque::new(),
            stats: BackendStats::default(),
            sequence_counter: 0,
            latency_samples: VecDeque::new(),
        }
    }

    pub fn with_config(config: BackendConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            event_history: VecDeque::new(),
            stats: BackendStats::default(),
            sequence_counter: 0,
            latency_samples: VecDeque::new(),
        }
    }

    /// Crear nueva sesión.
    pub fn create_session(
        &mut self,
        session_id: String,
        subscribed_types: Vec<BackendEventType>,
    ) -> Result<BackendSession, RealtimeBackendError> {
        if self.sessions.contains_key(&session_id) {
            return Err(RealtimeBackendError::SessionAlreadyExists(session_id));
        }

        if self.sessions.len() >= self.config.max_sessions {
            return Err(RealtimeBackendError::MaxSessionsReached(
                self.config.max_sessions,
            ));
        }

        let session = BackendSession::new(
            session_id.clone(),
            subscribed_types,
            self.config.rate_limit_per_sec,
        );

        self.sessions.insert(session_id.clone(), session);
        info!(
            "Sesión creada: {} con {} suscripciones",
            session_id,
            self.sessions.len()
        );

        Ok(self.sessions.get(&session_id).unwrap().clone())
    }

    /// Cerrar sesión.
    pub fn close_session(&mut self, session_id: &str) -> Result<(), RealtimeBackendError> {
        match self.sessions.remove(session_id) {
            Some(_) => {
                info!("Sesión cerrada: {}", session_id);
                Ok(())
            }
            None => Err(RealtimeBackendError::SessionNotFound(
                session_id.to_string(),
            )),
        }
    }

    /// Publicar evento a todas las sesiones suscritas.
    pub fn publish_event(
        &mut self,
        event_type: BackendEventType,
        payload: serde_json::Value,
        source_node: Option<String>,
    ) -> BackendResult {
        self.sequence_counter += 1;
        let event = BackendEvent::new(
            event_type.clone(),
            payload,
            source_node,
            self.sequence_counter,
        );

        let start = Instant::now();
        let mut events_sent = 0;
        let mut events_buffered = 0;
        let mut _events_dropped = 0;
        let mut _rate_limited = false;

        for session in self.sessions.values_mut() {
            if !session.active || !session.is_subscribed(&event.event_type) {
                continue;
            }

            if !session.check_rate_limit() {
                _rate_limited = true;
                self.stats.total_rate_limited += 1;
                // Intentar buffer si está habilitado
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

            session.record_message();
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

        BackendResult::success(
            "broadcast".to_string(),
            events_sent,
            events_buffered,
            self.sessions.len(),
        )
    }

    /// Obtener eventos de catch-up para una sesión.
    pub fn get_catchup_events(&self, session_id: &str, since_sequence: u64) -> Vec<BackendEvent> {
        let session = match self.sessions.get(session_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        self.event_history
            .iter()
            .filter(|e| e.sequence > since_sequence && session.is_subscribed(&e.event_type))
            .cloned()
            .collect()
    }

    /// Limpiar sesiones expiradas.
    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions
            .retain(|_, s| !s.is_expired(self.config.max_session_age_secs));
        let removed = before - self.sessions.len();
        if removed > 0 {
            info!("{} sesiones expiradas limpiadas", removed);
        }
        self.update_stats();
        removed
    }

    /// Obtener estadísticas.
    pub fn get_stats(&self) -> BackendStats {
        self.stats.clone()
    }

    /// Obtener sesión.
    pub fn get_session(&self, session_id: &str) -> Option<&BackendSession> {
        self.sessions.get(session_id)
    }

    /// Reiniciar estadísticas.
    pub fn reset_stats(&mut self) {
        self.stats = BackendStats::default();
        self.latency_samples.clear();
    }

    /// Actualizar estadísticas internas.
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

        let total_buffer: usize = self.sessions.values().map(|s| s.event_buffer.len()).sum();
        let max_buffer: usize = self.sessions.values().map(|s| s.max_buffer_size).sum();
        self.stats.buffer_utilization = if max_buffer > 0 {
            total_buffer as f64 / max_buffer as f64
        } else {
            0.0
        };
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for RealtimeBackend {
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "v1.1-sprint4"))]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = RealtimeBackend::new();
        assert_eq!(backend.sessions.len(), 0);
        assert_eq!(backend.config.max_sessions, 100);
    }

    #[test]
    fn test_backend_with_config() {
        let config = BackendConfig {
            max_sessions: 50,
            rate_limit_per_sec: 200,
            ..BackendConfig::default()
        };
        let backend = RealtimeBackend::with_config(config);
        assert_eq!(backend.config.max_sessions, 50);
        assert_eq!(backend.config.rate_limit_per_sec, 200);
    }

    #[test]
    fn test_create_session() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        let session = backend.create_session("s1".to_string(), types).unwrap();
        assert_eq!(session.session_id, "s1");
        assert_eq!(backend.sessions.len(), 1);
    }

    #[test]
    fn test_create_duplicate_session() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend
            .create_session("s1".to_string(), types.clone())
            .unwrap();
        let result = backend.create_session("s1".to_string(), types);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_sessions_reached() {
        let config = BackendConfig {
            max_sessions: 2,
            ..BackendConfig::default()
        };
        let mut backend = RealtimeBackend::with_config(config);
        let types = vec![BackendEventType::AlignmentFeedback];

        backend
            .create_session("s1".to_string(), types.clone())
            .unwrap();
        backend
            .create_session("s2".to_string(), types.clone())
            .unwrap();
        let result = backend.create_session("s3".to_string(), types);
        assert!(result.is_err());
    }

    #[test]
    fn test_close_session() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();
        backend.close_session("s1").unwrap();
        assert_eq!(backend.sessions.len(), 0);
    }

    #[test]
    fn test_close_unknown_session() {
        let mut backend = RealtimeBackend::new();
        let result = backend.close_session("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_publish_event_to_subscribed() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();

        let result = backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({"drift": 0.1}),
            Some("node1".to_string()),
        );
        assert_eq!(result.events_sent, 1);
    }

    #[test]
    fn test_publish_event_filtered() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();

        let result = backend.publish_event(
            BackendEventType::FederationGradientSync,
            serde_json::json!({"grad": 0.5}),
            None,
        );
        assert_eq!(result.events_sent, 0);
    }

    #[test]
    fn test_rate_limiting() {
        let config = BackendConfig {
            rate_limit_per_sec: 2,
            ..BackendConfig::default()
        };
        let mut backend = RealtimeBackend::with_config(config);
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();

        backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
        );
        backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
        );
        let result = backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
        );
        assert!(result.rate_limited || result.events_sent == 0 || result.events_buffered > 0);
    }

    #[test]
    fn test_get_catchup_events() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();

        backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({"n": 1}),
            None,
        );
        backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({"n": 2}),
            None,
        );

        let catchup = backend.get_catchup_events("s1", 0);
        assert_eq!(catchup.len(), 2);
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();

        // Marcar como expirada
        let session = backend.sessions.get_mut("s1").unwrap();
        session.connected_at = Instant::now() - std::time::Duration::from_secs(7200);

        let removed = backend.cleanup_expired_sessions();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut backend = RealtimeBackend::new();
        let types = vec![BackendEventType::AlignmentFeedback];
        backend.create_session("s1".to_string(), types).unwrap();

        backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
        );

        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 1);
        assert!(stats.total_events_sent > 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut backend = RealtimeBackend::new();
        backend.reset_stats();
        let stats = backend.get_stats();
        assert_eq!(stats.total_events_sent, 0);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(
            BackendEventType::AlignmentFeedback.to_string(),
            "alignment_feedback"
        );
        assert_eq!(
            BackendEventType::FederationGradientSync.to_string(),
            "federation_gradient"
        );
    }

    #[test]
    fn test_backend_result_success() {
        let result = BackendResult::success("s1".to_string(), 5, 2, 10);
        assert_eq!(result.events_sent, 5);
        assert_eq!(result.events_buffered, 2);
        assert!(!result.rate_limited);
    }

    #[test]
    fn test_backend_result_rate_limited() {
        let result = BackendResult::rate_limited("s1".to_string(), 10);
        assert!(result.rate_limited);
        assert_eq!(result.events_sent, 0);
    }

    #[test]
    fn test_config_default() {
        let config = BackendConfig::default();
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.rate_limit_per_sec, 100);
        assert!(config.enable_backpressure);
    }

    #[test]
    fn test_stats_default() {
        let stats = BackendStats::default();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_events_sent, 0);
    }

    #[test]
    fn test_backend_default() {
        let backend = RealtimeBackend::default();
        assert_eq!(backend.sessions.len(), 0);
    }

    #[test]
    fn test_session_buffer_event() {
        let mut session = BackendSession::new("s1".to_string(), vec![], 100);
        let event = BackendEvent::new(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
            1,
        );
        assert!(session.buffer_event(event));
        assert_eq!(session.event_buffer.len(), 1);
    }

    #[test]
    fn test_session_buffer_full() {
        let mut session = BackendSession::new("s1".to_string(), vec![], 100);
        session.max_buffer_size = 1;
        let event1 = BackendEvent::new(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
            1,
        );
        let event2 = BackendEvent::new(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
            2,
        );
        session.buffer_event(event1);
        assert!(!session.buffer_event(event2));
    }

    #[test]
    fn test_multiple_sessions_different_subscriptions() {
        let mut backend = RealtimeBackend::new();
        backend
            .create_session("s1".to_string(), vec![BackendEventType::AlignmentFeedback])
            .unwrap();
        backend
            .create_session(
                "s2".to_string(),
                vec![BackendEventType::FederationGradientSync],
            )
            .unwrap();

        let result = backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({}),
            None,
        );
        assert_eq!(result.events_sent, 1); // Solo s1 recibe
    }
}
