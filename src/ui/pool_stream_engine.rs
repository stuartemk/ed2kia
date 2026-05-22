//! Pool Stream Engine — Motor de streaming en tiempo real para pools técnicos y ZKP v4
//!
//! LP-84: UI Dashboard v4 & Real-time Streams
//! Extiende el backend de tiempo real con soporte para eventos de pools técnicos
//! de recursos cross-chain, Async ZKP v4 y DAO Ledger v2.
//!
//! Características:
//! - Sesiones con filtrado por categoría de evento (pool, zkp, dao, network)
//! - Broadcast a múltiples sesiones con backpressure
//! - Rate limiting por sesión con ventanas deslizantes
//! - Buffer de catchup para reconexiones
//! - Estadísticas de latencia y throughput
//! - Integración con Dashboard v4 snapshots
//!
//! Protegido con `#[cfg(feature = "v1.3-sprint2")]`.

#[cfg(feature = "v1.3-sprint2")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.3-sprint2")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.3-sprint2")]
use std::time::Instant;
#[cfg(feature = "v1.3-sprint2")]
use thiserror::Error;
#[cfg(feature = "v1.3-sprint2")]
use tracing::{debug, info};

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum StreamEngineError {
    #[error("Sesión no encontrada: {0}")]
    SessionNotFound(String),
    #[error("Rate limit excedido: {current}/{max} msg/s para sesión {session}")]
    RateLimitExceeded {
        current: usize,
        max: usize,
        session: String,
    },
    #[error("Categoría inválida: {0}")]
    InvalidCategory(String),
    #[error("Sesión ya existe: {0}")]
    SessionAlreadyExists(String),
    #[error("Máximo de sesiones alcanzado: {0}")]
    MaxSessionsReached(usize),
    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

// ─── Event Categories ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StreamCategory {
    Pool,
    Zkp,
    Dao,
    Network,
    All,
}

#[cfg(feature = "v1.3-sprint2")]
impl std::fmt::Display for StreamCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamCategory::Pool => write!(f, "pool"),
            StreamCategory::Zkp => write!(f, "zkp"),
            StreamCategory::Dao => write!(f, "dao"),
            StreamCategory::Network => write!(f, "network"),
            StreamCategory::All => write!(f, "all"),
        }
    }
}

// ─── Event Types ──────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PoolStreamEvent {
    // Pool events
    PoolShardRegistered,
    PoolShardRemoved,
    PoolAllocationCreated,
    PoolAllocationCompleted,
    PoolCreditDecay,
    PoolLatencyUpdate,
    // ZKP v4 events
    ZkpStatementSubmitted,
    ZkpBatchGenerated,
    ZkpProofVerified,
    ZkpCrossPoolVerified,
    ZkpFallbackTriggered,
    // DAO Ledger v2 events
    DaoProposalCreated,
    DaoVoteCast,
    DaoProposalExecuted,
    DaoStakePlaced,
    DaoStakeWithdrawn,
    DaoEpochAdvanced,
    // Network events
    NetworkLatencySpike,
    NetworkThroughputChange,
    NetworkErrorRateAlert,
}

#[cfg(feature = "v1.3-sprint2")]
impl std::fmt::Display for PoolStreamEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolStreamEvent::PoolShardRegistered => write!(f, "pool_shard_registered"),
            PoolStreamEvent::PoolShardRemoved => write!(f, "pool_shard_removed"),
            PoolStreamEvent::PoolAllocationCreated => write!(f, "pool_allocation_created"),
            PoolStreamEvent::PoolAllocationCompleted => {
                write!(f, "pool_allocation_completed")
            }
            PoolStreamEvent::PoolCreditDecay => write!(f, "pool_credit_decay"),
            PoolStreamEvent::PoolLatencyUpdate => write!(f, "pool_latency_update"),
            PoolStreamEvent::ZkpStatementSubmitted => write!(f, "zkp_statement_submitted"),
            PoolStreamEvent::ZkpBatchGenerated => write!(f, "zkp_batch_generated"),
            PoolStreamEvent::ZkpProofVerified => write!(f, "zkp_proof_verified"),
            PoolStreamEvent::ZkpCrossPoolVerified => write!(f, "zkp_cross_pool_verified"),
            PoolStreamEvent::ZkpFallbackTriggered => write!(f, "zkp_fallback_triggered"),
            PoolStreamEvent::DaoProposalCreated => write!(f, "dao_proposal_created"),
            PoolStreamEvent::DaoVoteCast => write!(f, "dao_vote_cast"),
            PoolStreamEvent::DaoProposalExecuted => write!(f, "dao_proposal_executed"),
            PoolStreamEvent::DaoStakePlaced => write!(f, "dao_stake_placed"),
            PoolStreamEvent::DaoStakeWithdrawn => write!(f, "dao_stake_withdrawn"),
            PoolStreamEvent::DaoEpochAdvanced => write!(f, "dao_epoch_advanced"),
            PoolStreamEvent::NetworkLatencySpike => write!(f, "network_latency_spike"),
            PoolStreamEvent::NetworkThroughputChange => {
                write!(f, "network_throughput_change")
            }
            PoolStreamEvent::NetworkErrorRateAlert => write!(f, "network_error_alert"),
        }
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl PoolStreamEvent {
    pub fn category(&self) -> StreamCategory {
        match self {
            PoolStreamEvent::PoolShardRegistered
            | PoolStreamEvent::PoolShardRemoved
            | PoolStreamEvent::PoolAllocationCreated
            | PoolStreamEvent::PoolAllocationCompleted
            | PoolStreamEvent::PoolCreditDecay
            | PoolStreamEvent::PoolLatencyUpdate => StreamCategory::Pool,
            PoolStreamEvent::ZkpStatementSubmitted
            | PoolStreamEvent::ZkpBatchGenerated
            | PoolStreamEvent::ZkpProofVerified
            | PoolStreamEvent::ZkpCrossPoolVerified
            | PoolStreamEvent::ZkpFallbackTriggered => StreamCategory::Zkp,
            PoolStreamEvent::DaoProposalCreated
            | PoolStreamEvent::DaoVoteCast
            | PoolStreamEvent::DaoProposalExecuted
            | PoolStreamEvent::DaoStakePlaced
            | PoolStreamEvent::DaoStakeWithdrawn
            | PoolStreamEvent::DaoEpochAdvanced => StreamCategory::Dao,
            PoolStreamEvent::NetworkLatencySpike
            | PoolStreamEvent::NetworkThroughputChange
            | PoolStreamEvent::NetworkErrorRateAlert => StreamCategory::Network,
        }
    }
}

// ─── Stream Event ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub event_id: String,
    pub event_type: PoolStreamEvent,
    pub category: StreamCategory,
    pub timestamp_ms: u64,
    pub payload: serde_json::Value,
    pub source: Option<String>,
    pub sequence: u64,
}

#[cfg(feature = "v1.3-sprint2")]
impl StreamEvent {
    pub fn new(
        event_type: PoolStreamEvent,
        payload: serde_json::Value,
        source: Option<String>,
        sequence: u64,
    ) -> Self {
        let category = event_type.category();
        Self {
            event_id: format!(
                "evt-{}-{}-{}",
                current_timestamp_ms(),
                sequence,
                event_type.category()
            ),
            event_type,
            category,
            timestamp_ms: current_timestamp_ms(),
            payload,
            source,
            sequence,
        }
    }
}

// ─── Session ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone)]
pub struct StreamSession {
    pub session_id: String,
    pub categories: Vec<StreamCategory>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub rate_limit_per_sec: usize,
    pub current_window_count: usize,
    pub window_start: Instant,
    pub event_buffer: VecDeque<StreamEvent>,
    pub max_buffer_size: usize,
    pub active: bool,
}

#[cfg(feature = "v1.3-sprint2")]
impl StreamSession {
    pub fn new(
        session_id: String,
        categories: Vec<StreamCategory>,
        rate_limit_per_sec: usize,
    ) -> Self {
        Self {
            session_id,
            categories,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            messages_sent: 0,
            messages_received: 0,
            rate_limit_per_sec,
            current_window_count: 0,
            window_start: Instant::now(),
            event_buffer: VecDeque::new(),
            max_buffer_size: 500,
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
    pub fn buffer_event(&mut self, event: StreamEvent) -> bool {
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

    /// Verificar si suscribe a la categoría del evento.
    pub fn is_subscribed(&self, category: &StreamCategory) -> bool {
        self.categories.contains(&StreamCategory::All) || self.categories.contains(category)
    }

    /// Desactivar sesión.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Obtener eventos pendientes del buffer.
    pub fn drain_buffer(&mut self) -> Vec<StreamEvent> {
        self.event_buffer.drain(..).collect()
    }
}

// ─── Stream Result ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    pub session_id: String,
    pub events_sent: usize,
    pub events_buffered: usize,
    pub events_dropped: usize,
    pub rate_limited: bool,
    pub active_sessions: usize,
    pub avg_latency_ms: f64,
}

#[cfg(feature = "v1.3-sprint2")]
impl StreamResult {
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

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEngineConfig {
    pub max_sessions: usize,
    pub rate_limit_per_sec: usize,
    pub max_session_age_secs: u64,
    pub event_history_size: usize,
    pub enable_backpressure: bool,
    pub backpressure_threshold: usize,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for StreamEngineConfig {
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

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEngineStats {
    pub active_sessions: usize,
    pub total_events_sent: u64,
    pub total_events_dropped: u64,
    pub total_rate_limited: u64,
    pub total_broadcasts: u64,
    pub avg_latency_ms: f64,
    pub pool_events: u64,
    pub zkp_events: u64,
    pub dao_events: u64,
    pub network_events: u64,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for StreamEngineStats {
    fn default() -> Self {
        Self {
            active_sessions: 0,
            total_events_sent: 0,
            total_events_dropped: 0,
            total_rate_limited: 0,
            total_broadcasts: 0,
            avg_latency_ms: 0.0,
            pool_events: 0,
            zkp_events: 0,
            dao_events: 0,
            network_events: 0,
        }
    }
}

// ─── Pool Stream Engine ───────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
pub struct PoolStreamEngine {
    config: StreamEngineConfig,
    sessions: HashMap<String, StreamSession>,
    event_history: VecDeque<StreamEvent>,
    sequence_counter: u64,
    pub stats: StreamEngineStats,
}

#[cfg(feature = "v1.3-sprint2")]
impl PoolStreamEngine {
    pub fn new() -> Self {
        Self::with_config(StreamEngineConfig::default())
    }

    pub fn with_config(config: StreamEngineConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            event_history: VecDeque::new(),
            sequence_counter: 0,
            stats: StreamEngineStats::default(),
        }
    }

    /// Crear una nueva sesión de streaming.
    pub fn create_session(
        &mut self,
        session_id: String,
        categories: Vec<StreamCategory>,
    ) -> Result<StreamSession, StreamEngineError> {
        if self.sessions.contains_key(&session_id) {
            return Err(StreamEngineError::SessionAlreadyExists(session_id));
        }
        if self.sessions.len() >= self.config.max_sessions {
            return Err(StreamEngineError::MaxSessionsReached(
                self.config.max_sessions,
            ));
        }

        let session = StreamSession::new(
            session_id.clone(),
            categories,
            self.config.rate_limit_per_sec,
        );

        self.stats.active_sessions = self.sessions.len() + 1;
        self.sessions.insert(session_id.clone(), session);
        info!(
            "Sesión de streaming creada: {} con categorías: {:?}",
            session_id,
            self.sessions.get(&session_id).unwrap().categories
        );

        self.sessions
            .get(&session_id)
            .cloned()
            .ok_or(StreamEngineError::SessionNotFound(session_id))
    }

    /// Cerrar una sesión.
    pub fn close_session(&mut self, session_id: &str) -> Result<(), StreamEngineError> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.deactivate();
            self.stats.active_sessions = self.sessions.len();
            info!("Sesión cerrada: {}", session_id);
            Ok(())
        } else {
            Err(StreamEngineError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Eliminar una sesión.
    pub fn remove_session(&mut self, session_id: &str) -> Result<(), StreamEngineError> {
        if self.sessions.remove(session_id).is_some() {
            self.stats.active_sessions = self.sessions.len();
            Ok(())
        } else {
            Err(StreamEngineError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Publicar un evento a todas las sesiones suscritas.
    pub fn publish_event(
        &mut self,
        event_type: PoolStreamEvent,
        payload: serde_json::Value,
        source: Option<String>,
    ) -> StreamResult {
        self.sequence_counter += 1;
        let event = StreamEvent::new(event_type.clone(), payload, source, self.sequence_counter);

        let category = event.category.clone();
        let start = Instant::now();

        let mut sent = 0;
        let mut buffered = 0;
        let mut dropped = 0;
        for (_, session) in self.sessions.iter_mut() {
            if !session.active {
                continue;
            }
            if !session.is_subscribed(&category) {
                continue;
            }

            if !session.check_rate_limit() {
                self.stats.total_rate_limited += 1;
                dropped += 1;
                continue;
            }

            if session.buffer_event(event.clone()) {
                buffered += 1;
                sent += 1;
                session.record_message();
            } else {
                dropped += 1;
            }
        }

        // Update category stats
        match category {
            StreamCategory::Pool => self.stats.pool_events += 1,
            StreamCategory::Zkp => self.stats.zkp_events += 1,
            StreamCategory::Dao => self.stats.dao_events += 1,
            StreamCategory::Network => self.stats.network_events += 1,
            StreamCategory::All => {}
        }

        self.stats.total_events_sent += sent as u64;
        self.stats.total_events_dropped += dropped as u64;
        self.stats.total_broadcasts += 1;

        let latency = start.elapsed().as_micros() as f64 / 1000.0;
        self.stats.avg_latency_ms =
            (self.stats.avg_latency_ms * (self.stats.total_broadcasts - 1) as f64 + latency)
                / self.stats.total_broadcasts as f64;

        // Store in event history
        self.event_history.push_back(event);
        self.enforce_history_limit();

        StreamResult::success(
            "broadcast".to_string(),
            sent,
            buffered,
            self.active_session_count(),
        )
    }

    /// Obtener eventos pendientes para una sesión.
    pub fn get_pending_events(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<StreamEvent>, StreamEngineError> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.messages_received += 1;
            session.last_activity = Instant::now();
            Ok(session.drain_buffer())
        } else {
            Err(StreamEngineError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Obtener historial de eventos (para catchup).
    pub fn get_event_history(&self, limit: usize) -> Vec<&StreamEvent> {
        self.event_history.iter().rev().take(limit).collect()
    }

    /// Limpiar sesiones expiradas.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|_, session| {
            let active = session.active && !session.is_expired(self.config.max_session_age_secs);
            if !active {
                session.deactivate();
            }
            active
        });
        let cleaned = before - self.sessions.len();
        self.stats.active_sessions = self.sessions.len();
        if cleaned > 0 {
            info!("{} sesiones expiradas limpiadas", cleaned);
        }
        cleaned
    }

    /// Suscribirse a categorías adicionales.
    pub fn subscribe_categories(
        &mut self,
        session_id: &str,
        categories: Vec<StreamCategory>,
    ) -> Result<(), StreamEngineError> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            for cat in categories {
                if !session.categories.contains(&cat) {
                    session.categories.push(cat);
                }
            }
            debug!("Sesión {} suscrita a nuevas categorías", session_id);
            Ok(())
        } else {
            Err(StreamEngineError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Obtener sesión.
    pub fn get_session(&self, session_id: &str) -> Option<&StreamSession> {
        self.sessions.get(session_id)
    }

    /// Contar sesiones activas.
    pub fn active_session_count(&self) -> usize {
        self.sessions.values().filter(|s| s.active).count()
    }

    /// Resetear estadísticas.
    pub fn reset_stats(&mut self) {
        self.stats = StreamEngineStats::default();
        self.stats.active_sessions = self.active_session_count();
    }

    fn enforce_history_limit(&mut self) {
        if self.event_history.len() > self.config.event_history_size {
            let remove_count = self.event_history.len() - self.config.event_history_size;
            self.event_history.drain(0..remove_count);
        }
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for PoolStreamEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v1.3-sprint2")]
    fn make_engine() -> PoolStreamEngine {
        PoolStreamEngine::new()
    }

    #[cfg(feature = "v1.3-sprint2")]
    fn make_payload(key: &str, value: f64) -> serde_json::Value {
        serde_json::json!({ key: value })
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_engine_creation() {
        let engine = make_engine();
        assert_eq!(engine.stats.active_sessions, 0);
        assert_eq!(engine.stats.total_events_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_create_session() {
        let mut engine = make_engine();
        let result = engine.create_session(
            "s1".to_string(),
            vec![StreamCategory::Pool, StreamCategory::Zkp],
        );
        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.session_id, "s1");
        assert_eq!(session.categories.len(), 2);
        assert!(session.active);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_create_duplicate_session() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        let result = engine.create_session("s1".to_string(), vec![StreamCategory::Zkp]);
        assert!(result.is_err());
        match result.unwrap_err() {
            StreamEngineError::SessionAlreadyExists(id) => assert_eq!(id, "s1"),
            other => panic!("Expected SessionAlreadyExists, got {:?}", other),
        }
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_close_session() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        assert!(engine.close_session("s1").is_ok());
        let session = engine.get_session("s1").unwrap();
        assert!(!session.active);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_remove_session() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        assert!(engine.remove_session("s1").is_ok());
        assert!(engine.get_session("s1").is_none());
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_publish_event() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        let result = engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            Some("pool-1".to_string()),
        );
        assert_eq!(result.events_sent, 1);
        assert_eq!(result.events_dropped, 0);
        assert_eq!(engine.stats.pool_events, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_publish_event_no_subscription() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Dao]);
        let result = engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        assert_eq!(result.events_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_publish_event_all_category() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::All]);
        let result = engine.publish_event(
            PoolStreamEvent::ZkpBatchGenerated,
            make_payload("batch_id", 1.0),
            None,
        );
        assert_eq!(result.events_sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_publish_multiple_sessions() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        engine.create_session("s2".to_string(), vec![StreamCategory::Pool]);
        let result = engine.publish_event(
            PoolStreamEvent::PoolAllocationCreated,
            make_payload("credits", 100.0),
            None,
        );
        assert_eq!(result.events_sent, 2);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_pending_events() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        let events = engine.get_pending_events("s1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, PoolStreamEvent::PoolShardRegistered);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_pending_events_empty() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        let events = engine.get_pending_events("s1").unwrap();
        assert_eq!(events.len(), 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_pending_events_session_not_found() {
        let mut engine = make_engine();
        let result = engine.get_pending_events("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_event_history() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::All]);
        engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        engine.publish_event(
            PoolStreamEvent::ZkpBatchGenerated,
            make_payload("batch_id", 1.0),
            None,
        );
        let history = engine.get_event_history(10);
        assert_eq!(history.len(), 2);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_subscribe_categories() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        assert!(engine
            .subscribe_categories("s1", vec![StreamCategory::Zkp, StreamCategory::Dao])
            .is_ok());
        let session = engine.get_session("s1").unwrap();
        assert_eq!(session.categories.len(), 3);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stats_tracking() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::All]);
        engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        engine.publish_event(
            PoolStreamEvent::ZkpBatchGenerated,
            make_payload("batch_id", 1.0),
            None,
        );
        engine.publish_event(
            PoolStreamEvent::DaoProposalCreated,
            make_payload("proposal_id", 1.0),
            None,
        );
        assert_eq!(engine.stats.pool_events, 1);
        assert_eq!(engine.stats.zkp_events, 1);
        assert_eq!(engine.stats.dao_events, 1);
        assert_eq!(engine.stats.total_broadcasts, 3);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_reset_stats() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::All]);
        engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        engine.reset_stats();
        assert_eq!(engine.stats.total_events_sent, 0);
        assert_eq!(engine.stats.pool_events, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_event_category() {
        assert_eq!(
            PoolStreamEvent::PoolShardRegistered.category(),
            StreamCategory::Pool
        );
        assert_eq!(
            PoolStreamEvent::ZkpBatchGenerated.category(),
            StreamCategory::Zkp
        );
        assert_eq!(
            PoolStreamEvent::DaoProposalCreated.category(),
            StreamCategory::Dao
        );
        assert_eq!(
            PoolStreamEvent::NetworkLatencySpike.category(),
            StreamCategory::Network
        );
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_event_display() {
        assert_eq!(
            format!("{}", PoolStreamEvent::PoolShardRegistered),
            "pool_shard_registered"
        );
        assert_eq!(
            format!("{}", PoolStreamEvent::ZkpProofVerified),
            "zkp_proof_verified"
        );
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_category_display() {
        assert_eq!(format!("{}", StreamCategory::Pool), "pool");
        assert_eq!(format!("{}", StreamCategory::Zkp), "zkp");
        assert_eq!(format!("{}", StreamCategory::Dao), "dao");
        assert_eq!(format!("{}", StreamCategory::Network), "network");
        assert_eq!(format!("{}", StreamCategory::All), "all");
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_config_default() {
        let config = StreamEngineConfig::default();
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.rate_limit_per_sec, 100);
        assert_eq!(config.max_session_age_secs, 3600);
        assert!(config.enable_backpressure);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stats_default() {
        let stats = StreamEngineStats::default();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_events_sent, 0);
        assert_eq!(stats.pool_events, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_session_rate_limit() {
        let mut session = StreamSession::new("s1".to_string(), vec![StreamCategory::All], 2);
        assert!(session.check_rate_limit());
        session.record_message();
        session.record_message();
        assert!(!session.check_rate_limit());
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_session_buffer_backpressure() {
        let mut session = StreamSession::new("s1".to_string(), vec![StreamCategory::All], 100);
        session.max_buffer_size = 2;
        let event1 = StreamEvent::new(
            PoolStreamEvent::PoolShardRegistered,
            serde_json::json!({}),
            None,
            1,
        );
        let event2 = StreamEvent::new(
            PoolStreamEvent::PoolShardRegistered,
            serde_json::json!({}),
            None,
            2,
        );
        let event3 = StreamEvent::new(
            PoolStreamEvent::PoolShardRegistered,
            serde_json::json!({}),
            None,
            3,
        );
        assert!(session.buffer_event(event1));
        assert!(session.buffer_event(event2));
        assert!(!session.buffer_event(event3));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_active_session_count() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        engine.create_session("s2".to_string(), vec![StreamCategory::Zkp]);
        assert_eq!(engine.active_session_count(), 2);
        engine.close_session("s1");
        assert_eq!(engine.active_session_count(), 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_max_sessions() {
        let config = StreamEngineConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let mut engine = PoolStreamEngine::with_config(config);
        engine.create_session("s1".to_string(), vec![StreamCategory::Pool]);
        engine.create_session("s2".to_string(), vec![StreamCategory::Zkp]);
        let result = engine.create_session("s3".to_string(), vec![StreamCategory::Dao]);
        assert!(result.is_err());
        match result.unwrap_err() {
            StreamEngineError::MaxSessionsReached(2) => {}
            other => panic!("Expected MaxSessionsReached, got {:?}", other),
        }
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_publish_to_inactive_session() {
        let mut engine = make_engine();
        engine.create_session("s1".to_string(), vec![StreamCategory::All]);
        engine.close_session("s1");
        let result = engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        assert_eq!(result.events_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_error_display() {
        let err = StreamEngineError::SessionNotFound("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_engine_default() {
        let engine = PoolStreamEngine::default();
        assert_eq!(engine.stats.active_sessions, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stream_event_new() {
        let event = StreamEvent::new(
            PoolStreamEvent::PoolAllocationCreated,
            serde_json::json!({"credits": 50.0}),
            Some("pool-1".to_string()),
            1,
        );
        assert_eq!(event.category, StreamCategory::Pool);
        assert_eq!(event.sequence, 1);
        assert_eq!(event.source, Some("pool-1".to_string()));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_session_is_subscribed_all() {
        let session = StreamSession::new("s1".to_string(), vec![StreamCategory::All], 100);
        assert!(session.is_subscribed(&StreamCategory::Pool));
        assert!(session.is_subscribed(&StreamCategory::Zkp));
        assert!(session.is_subscribed(&StreamCategory::Dao));
        assert!(session.is_subscribed(&StreamCategory::Network));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_session_is_subscribed_specific() {
        let session = StreamSession::new("s1".to_string(), vec![StreamCategory::Pool], 100);
        assert!(session.is_subscribed(&StreamCategory::Pool));
        assert!(!session.is_subscribed(&StreamCategory::Zkp));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_full_stream_pipeline() {
        let mut engine = make_engine();
        // Create sessions
        engine.create_session("pool-watcher".to_string(), vec![StreamCategory::Pool]);
        engine.create_session("zkp-watcher".to_string(), vec![StreamCategory::Zkp]);
        engine.create_session("all-watcher".to_string(), vec![StreamCategory::All]);

        // Publish events
        engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            make_payload("shard_id", 1.0),
            None,
        );
        engine.publish_event(
            PoolStreamEvent::ZkpBatchGenerated,
            make_payload("batch_id", 1.0),
            None,
        );

        // Verify stats
        assert_eq!(engine.stats.pool_events, 1);
        assert_eq!(engine.stats.zkp_events, 1);
        assert_eq!(engine.stats.total_broadcasts, 2);
        assert_eq!(engine.active_session_count(), 3);

        // Get pending events
        let pool_events = engine.get_pending_events("pool-watcher").unwrap();
        assert_eq!(pool_events.len(), 1);
        assert_eq!(pool_events[0].category, StreamCategory::Pool);

        let zkp_events = engine.get_pending_events("zkp-watcher").unwrap();
        assert_eq!(zkp_events.len(), 1);
        assert_eq!(zkp_events[0].category, StreamCategory::Zkp);

        let all_events = engine.get_pending_events("all-watcher").unwrap();
        assert_eq!(all_events.len(), 2);
    }
}
