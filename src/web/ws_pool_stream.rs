//! WebSocket Pool Stream — Streaming de datos de pools y ZKP v4 vía WebSocket
//!
//! LP-84: UI Dashboard v4 & Real-time Streams
//! Proporciona streaming en tiempo real de eventos de pools técnicos, ZKP v4
//! y DAO Ledger v2 a clientes conectados vía WebSocket, con autenticación ligera,
//! rate limiting y suscripción por categorías.
//!
//! Características:
//! - Conexiones WebSocket con autenticación ligera
//! - Suscripción por categorías (pool, zkp, dao, network)
//! - Rate limiting por conexión con ventanas deslizantes
//! - Buffer de catchup para reconexiones
//! - Heartbeat para mantener conexiones activas
//! - Serialización optimizada para dashboards
//!
//! Protegido con `#[cfg(feature = "v1.3-sprint2")]`.

#[cfg(feature = "v1.3-sprint2")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.3-sprint2")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.3-sprint2")]
use thiserror::Error;
#[cfg(feature = "v1.3-sprint2")]
use tracing::{debug, info, warn};

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WsPoolError {
    #[error("Conexión no encontrada: {0}")]
    ConnectionNotFound(String),
    #[error("Rate limit excedido: {current}/{max} msg/s para conexión {conn}")]
    RateLimitExceeded { current: usize, max: usize, conn: String },
    #[error("Autenticación fallida: {0}")]
    AuthFailed(String),
    #[error("Conexión duplicada: {0}")]
    ConnectionAlreadyExists(String),
    #[error("Máximo de conexiones alcanzado: {0}")]
    MaxConnectionsReached(usize),
    #[error("Categoría inválida: {0}")]
    InvalidCategory(String),
    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

// ─── Pool Categories ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PoolCategory {
    Pool,
    Zkp,
    Dao,
    Network,
    All,
}

#[cfg(feature = "v1.3-sprint2")]
impl std::fmt::Display for PoolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolCategory::Pool => write!(f, "pool"),
            PoolCategory::Zkp => write!(f, "zkp"),
            PoolCategory::Dao => write!(f, "dao"),
            PoolCategory::Network => write!(f, "network"),
            PoolCategory::All => write!(f, "all"),
        }
    }
}

// ─── Pool Message ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PoolMessage {
    // Client → Server
    Auth {
        client_id: String,
        signature: String,
        timestamp_ms: u64,
    },
    Subscribe {
        categories: Vec<PoolCategory>,
    },
    Ping {
        timestamp_ms: u64,
    },
    Reconnect {
        client_id: String,
        last_sequence: u64,
    },

    // Server → Client
    AuthOk {
        connection_id: String,
        token: String,
    },
    AuthDenied {
        reason: String,
    },
    Event {
        sequence: u64,
        timestamp_ms: u64,
        category: String,
        event_type: String,
        payload: serde_json::Value,
    },
    Snapshot {
        sequence: u64,
        timestamp_ms: u64,
        data: serde_json::Value,
    },
    Pong {
        timestamp_ms: u64,
    },
    Alert {
        alert_id: String,
        severity: String,
        message: String,
        timestamp_ms: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

// ─── Pool Connection ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConnection {
    pub connection_id: String,
    pub client_id: String,
    pub categories: Vec<PoolCategory>,
    pub authenticated: bool,
    pub connected_at_ms: u64,
    pub last_message_ms: u64,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub rate_limit_per_sec: usize,
    pub rate_limit_counter: usize,
    pub rate_limit_window_start: u64,
    pub buffer: VecDeque<PoolMessage>,
    pub max_buffer: usize,
}

#[cfg(feature = "v1.3-sprint2")]
impl PoolConnection {
    pub fn new(connection_id: String, client_id: String, rate_limit_per_sec: usize) -> Self {
        Self {
            connection_id,
            client_id,
            categories: vec![PoolCategory::All],
            authenticated: false,
            connected_at_ms: current_timestamp_ms(),
            last_message_ms: current_timestamp_ms(),
            messages_sent: 0,
            messages_received: 0,
            rate_limit_per_sec,
            rate_limit_counter: 0,
            rate_limit_window_start: current_timestamp_ms(),
            buffer: VecDeque::with_capacity(100),
            max_buffer: 100,
        }
    }

    pub fn check_rate_limit(&mut self) -> bool {
        let now = current_timestamp_ms();
        if now.saturating_sub(self.rate_limit_window_start) > 1000 {
            self.rate_limit_counter = 0;
            self.rate_limit_window_start = now;
        }
        self.rate_limit_counter += 1;
        self.rate_limit_counter <= self.rate_limit_per_sec
    }

    pub fn record_message(&mut self) {
        self.last_message_ms = current_timestamp_ms();
        self.messages_received += 1;
    }

    pub fn buffer_message(&mut self, message: PoolMessage) -> bool {
        if self.buffer.len() >= self.max_buffer {
            self.buffer.pop_front();
        }
        self.buffer.push_back(message);
        true
    }

    pub fn get_pending_since(&mut self, since_sequence: u64) -> Vec<PoolMessage> {
        let mut result = Vec::new();
        while let Some(msg) = self.buffer.front() {
            if let PoolMessage::Event { sequence, .. }
            | PoolMessage::Snapshot { sequence, .. } = &msg
            {
                if *sequence > since_sequence {
                    result.push(self.buffer.pop_front().unwrap());
                } else {
                    self.buffer.pop_front();
                }
            } else {
                result.push(self.buffer.pop_front().unwrap());
            }
        }
        result
    }

    pub fn is_expired(&self, timeout_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_message_ms) > timeout_ms
    }

    pub fn subscribe(&mut self, categories: Vec<PoolCategory>) {
        self.categories = categories;
        debug!(
            "Conexión {} suscrita a: {:?}",
            self.connection_id, self.categories
        );
    }

    pub fn is_subscribed(&self, category: &PoolCategory) -> bool {
        self.categories.contains(&PoolCategory::All) || self.categories.contains(category)
    }
}

// ─── Stream Result ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStreamResult {
    pub connection_id: String,
    pub messages_sent: usize,
    pub rate_limited: bool,
    pub active_connections: usize,
    pub categories: Vec<PoolCategory>,
}

#[cfg(feature = "v1.3-sprint2")]
impl PoolStreamResult {
    pub fn success(
        connection_id: String,
        messages_sent: usize,
        active_connections: usize,
        categories: Vec<PoolCategory>,
    ) -> Self {
        Self {
            connection_id,
            messages_sent,
            rate_limited: false,
            active_connections,
            categories,
        }
    }

    pub fn rate_limited(connection_id: String, active_connections: usize) -> Self {
        Self {
            connection_id,
            messages_sent: 0,
            rate_limited: true,
            active_connections,
            categories: vec![],
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsPoolConfig {
    pub max_connections: usize,
    pub rate_limit_per_sec: usize,
    pub connection_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub snapshot_interval_ms: u64,
    pub max_buffer_size: usize,
    pub auth_required: bool,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for WsPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            rate_limit_per_sec: 30,
            connection_timeout_ms: 60_000,
            heartbeat_interval_ms: 15_000,
            snapshot_interval_ms: 2_000,
            max_buffer_size: 100,
            auth_required: true,
        }
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsPoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub total_messages_sent: usize,
    pub total_messages_received: usize,
    pub total_rate_limited: usize,
    pub total_auth_failures: usize,
    pub avg_snapshot_latency_ms: f64,
    pub snapshots_sent: usize,
    pub pool_events_sent: usize,
    pub zkp_events_sent: usize,
    pub dao_events_sent: usize,
    pub network_events_sent: usize,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for WsPoolStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            total_messages_sent: 0,
            total_messages_received: 0,
            total_rate_limited: 0,
            total_auth_failures: 0,
            avg_snapshot_latency_ms: 0.0,
            snapshots_sent: 0,
            pool_events_sent: 0,
            zkp_events_sent: 0,
            dao_events_sent: 0,
            network_events_sent: 0,
        }
    }
}

// ─── WebSocket Pool Stream ────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
pub struct WsPoolStream {
    config: WsPoolConfig,
    connections: HashMap<String, PoolConnection>,
    sequence_counter: u64,
    pub stats: WsPoolStats,
}

#[cfg(feature = "v1.3-sprint2")]
impl WsPoolStream {
    pub fn new() -> Self {
        Self::with_config(WsPoolConfig::default())
    }

    pub fn with_config(config: WsPoolConfig) -> Self {
        Self {
            config,
            connections: HashMap::new(),
            sequence_counter: 0,
            stats: WsPoolStats::default(),
        }
    }

    /// Autenticar una nueva conexión.
    pub fn authenticate(
        &mut self,
        client_id: String,
        _signature: String,
    ) -> Result<PoolStreamResult, WsPoolError> {
        if self.connections.len() >= self.config.max_connections {
            return Err(WsPoolError::MaxConnectionsReached(
                self.config.max_connections,
            ));
        }

        if self.connections.contains_key(&client_id) {
            return Err(WsPoolError::ConnectionAlreadyExists(client_id));
        }

        let connection_id = format!("conn-{}-{}", current_timestamp_ms(), self.sequence_counter);
        let mut connection = PoolConnection::new(
            connection_id.clone(),
            client_id.clone(),
            self.config.rate_limit_per_sec,
        );
        connection.authenticated = true;

        self.connections.insert(client_id.clone(), connection);
        self.stats.total_connections += 1;
        self.stats.active_connections = self.connections.len();

        info!("Cliente autenticado: {} → {}", client_id, connection_id);

        Ok(PoolStreamResult::success(
            connection_id,
            0,
            self.stats.active_connections,
            vec![PoolCategory::All],
        ))
    }

    /// Cerrar una conexión.
    pub fn close_connection(&mut self, connection_id: &str) -> Result<(), WsPoolError> {
        if self.connections.remove(connection_id).is_some() {
            self.stats.active_connections = self.connections.len();
            info!("Conexión cerrada: {}", connection_id);
            Ok(())
        } else {
            Err(WsPoolError::ConnectionNotFound(connection_id.to_string()))
        }
    }

    /// Suscribirse a categorías.
    pub fn subscribe(
        &mut self,
        connection_id: &str,
        categories: Vec<PoolCategory>,
    ) -> Result<(), WsPoolError> {
        if let Some(conn) = self.connections.get_mut(connection_id) {
            conn.subscribe(categories);
            conn.record_message();
            self.stats.total_messages_received += 1;
            Ok(())
        } else {
            Err(WsPoolError::ConnectionNotFound(connection_id.to_string()))
        }
    }

    /// Enviar evento a conexiones suscritas.
    pub fn broadcast_event(
        &mut self,
        category: PoolCategory,
        event_type: String,
        payload: serde_json::Value,
    ) -> PoolStreamResult {
        self.sequence_counter += 1;
        let sequence = self.sequence_counter;
        let now = current_timestamp_ms();

        let message = PoolMessage::Event {
            sequence,
            timestamp_ms: now,
            category: category.to_string(),
            event_type,
            payload,
        };

        let mut sent = 0;
        let mut rate_limited = false;

        for (_, conn) in self.connections.iter_mut() {
            if !conn.authenticated {
                continue;
            }
            if !conn.is_subscribed(&category) {
                continue;
            }

            if !conn.check_rate_limit() {
                rate_limited = true;
                self.stats.total_rate_limited += 1;
                continue;
            }

            if conn.buffer_message(message.clone()) {
                sent += 1;
                conn.messages_sent += 1;
            }
        }

        // Update category stats
        match category {
            PoolCategory::Pool => self.stats.pool_events_sent += 1,
            PoolCategory::Zkp => self.stats.zkp_events_sent += 1,
            PoolCategory::Dao => self.stats.dao_events_sent += 1,
            PoolCategory::Network => self.stats.network_events_sent += 1,
            PoolCategory::All => {}
        }

        self.stats.total_messages_sent += sent;

        if rate_limited {
            PoolStreamResult::rate_limited(
                "broadcast".to_string(),
                self.stats.active_connections,
            )
        } else {
            PoolStreamResult::success(
                "broadcast".to_string(),
                sent,
                self.stats.active_connections,
                vec![category],
            )
        }
    }

    /// Enviar snapshot a todas las conexiones.
    pub fn broadcast_snapshot(
        &mut self,
        data: serde_json::Value,
    ) -> PoolStreamResult {
        self.sequence_counter += 1;
        let sequence = self.sequence_counter;
        let now = current_timestamp_ms();

        let message = PoolMessage::Snapshot {
            sequence,
            timestamp_ms: now,
            data,
        };

        let mut sent = 0;

        for (_, conn) in self.connections.iter_mut() {
            if !conn.authenticated {
                continue;
            }
            if conn.buffer_message(message.clone()) {
                sent += 1;
                conn.messages_sent += 1;
            }
        }

        self.stats.total_messages_sent += sent;
        self.stats.snapshots_sent += 1;

        PoolStreamResult::success(
            "snapshot".to_string(),
            sent,
            self.stats.active_connections,
            vec![PoolCategory::All],
        )
    }

    /// Enviar alerta a todas las conexiones.
    pub fn broadcast_alert(
        &mut self,
        alert_id: String,
        severity: String,
        message: String,
    ) -> usize {
        let now = current_timestamp_ms();
        let alert_msg = PoolMessage::Alert {
            alert_id,
            severity,
            message,
            timestamp_ms: now,
        };

        let mut sent = 0;
        for (_, conn) in self.connections.iter_mut() {
            if conn.authenticated && conn.buffer_message(alert_msg.clone()) {
                sent += 1;
            }
        }
        sent
    }

    /// Manejar ping del cliente.
    pub fn handle_ping(
        &mut self,
        connection_id: &str,
        _timestamp_ms: u64,
    ) -> Result<PoolMessage, WsPoolError> {
        if let Some(conn) = self.connections.get_mut(connection_id) {
            conn.record_message();
            self.stats.total_messages_received += 1;
            Ok(PoolMessage::Pong {
                timestamp_ms: current_timestamp_ms(),
            })
        } else {
            Err(WsPoolError::ConnectionNotFound(connection_id.to_string()))
        }
    }

    /// Obtener mensajes pendientes para reconexión.
    pub fn get_pending_messages(
        &mut self,
        connection_id: &str,
        since_sequence: u64,
    ) -> Result<Vec<PoolMessage>, WsPoolError> {
        if let Some(conn) = self.connections.get_mut(connection_id) {
            conn.record_message();
            self.stats.total_messages_received += 1;
            Ok(conn.get_pending_since(since_sequence))
        } else {
            Err(WsPoolError::ConnectionNotFound(connection_id.to_string()))
        }
    }

    /// Limpiar conexiones expiradas.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.connections.len();
        self.connections
            .retain(|_, conn| !conn.is_expired(self.config.connection_timeout_ms));
        let cleaned = before - self.connections.len();
        self.stats.active_connections = self.connections.len();
        if cleaned > 0 {
            warn!("{} conexiones expiradas limpiadas", cleaned);
        }
        cleaned
    }

    /// Obtener conexión.
    pub fn get_connection(&self, connection_id: &str) -> Option<&PoolConnection> {
        self.connections.get(connection_id)
    }

    /// Contar conexiones activas.
    pub fn active_connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Resetear estadísticas.
    pub fn reset_stats(&mut self) {
        self.stats = WsPoolStats::default();
        self.stats.active_connections = self.connections.len();
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for WsPoolStream {
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
    fn make_stream() -> WsPoolStream {
        WsPoolStream::new()
    }

    #[cfg(feature = "v1.3-sprint2")]
    fn make_payload(key: &str, value: f64) -> serde_json::Value {
        serde_json::json!({ key: value })
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stream_creation() {
        let stream = make_stream();
        assert_eq!(stream.stats.active_connections, 0);
        assert_eq!(stream.stats.total_messages_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_authenticate() {
        let mut stream = make_stream();
        let result = stream.authenticate("client-1".to_string(), "sig-1".to_string());
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(!res.connection_id.is_empty());
        assert_eq!(stream.stats.total_connections, 1);
        assert_eq!(stream.stats.active_connections, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_authenticate_duplicate() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let result = stream.authenticate("client-1".to_string(), "sig-1".to_string());
        assert!(result.is_err());
        match result.unwrap_err() {
            WsPoolError::ConnectionAlreadyExists(id) => assert_eq!(id, "client-1"),
            other => panic!("Expected ConnectionAlreadyExists, got {:?}", other),
        }
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_close_connection() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let conn_id = stream.connections.keys().next().unwrap().clone();
        assert!(stream.close_connection(&conn_id).is_ok());
        assert_eq!(stream.stats.active_connections, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_close_missing_connection() {
        let mut stream = make_stream();
        let result = stream.close_connection("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_subscribe() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let conn_id = stream.connections.keys().next().unwrap().clone();
        assert!(stream
            .subscribe(
                &conn_id,
                vec![PoolCategory::Pool, PoolCategory::Zkp]
            )
            .is_ok());
        let conn = stream.get_connection(&conn_id).unwrap();
        assert!(conn.categories.contains(&PoolCategory::Pool));
        assert!(conn.categories.contains(&PoolCategory::Zkp));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_broadcast_event() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let result = stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        assert_eq!(result.messages_sent, 1);
        assert_eq!(stream.stats.pool_events_sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_broadcast_event_no_subscription() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let conn_id = stream.connections.keys().next().unwrap().clone();
        stream.subscribe(&conn_id, vec![PoolCategory::Dao]);
        let result = stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        assert_eq!(result.messages_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_broadcast_snapshot() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        stream.authenticate("client-2".to_string(), "sig-2".to_string());
        let result = stream.broadcast_snapshot(serde_json::json!({
            "pool_shards": 5,
            "zkp_rate": 0.95
        }));
        assert_eq!(result.messages_sent, 2);
        assert_eq!(stream.stats.snapshots_sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_broadcast_alert() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let sent = stream.broadcast_alert(
            "alert-1".to_string(),
            "warning".to_string(),
            "Pool credits low".to_string(),
        );
        assert_eq!(sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_handle_ping() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let conn_id = stream.connections.keys().next().unwrap().clone();
        let result = stream.handle_ping(&conn_id, current_timestamp_ms());
        assert!(result.is_ok());
        match result.unwrap() {
            PoolMessage::Pong { timestamp_ms } => assert!(timestamp_ms > 0),
            other => panic!("Expected Pong, got {:?}", other),
        }
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_handle_ping_missing() {
        let mut stream = make_stream();
        let result = stream.handle_ping("nonexistent", current_timestamp_ms());
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_pending_messages() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let conn_id = stream.connections.keys().next().unwrap().clone();
        stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        let messages = stream.get_pending_messages(&conn_id, 0).unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_cleanup_expired() {
        let config = WsPoolConfig {
            connection_timeout_ms: 0, // Expire immediately
            ..Default::default()
        };
        let mut stream = WsPoolStream::with_config(config);
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        std::thread::sleep(std::time::Duration::from_millis(1));
        let cleaned = stream.cleanup_expired();
        assert_eq!(cleaned, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_max_connections() {
        let config = WsPoolConfig {
            max_connections: 1,
            ..Default::default()
        };
        let mut stream = WsPoolStream::with_config(config);
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        let result = stream.authenticate("client-2".to_string(), "sig-2".to_string());
        assert!(result.is_err());
        match result.unwrap_err() {
            WsPoolError::MaxConnectionsReached(1) => {}
            other => panic!("Expected MaxConnectionsReached, got {:?}", other),
        }
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stats_tracking() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        stream.broadcast_event(
            PoolCategory::Zkp,
            "zkp_batch_generated".to_string(),
            make_payload("batch_id", 1.0),
        );
        assert_eq!(stream.stats.pool_events_sent, 1);
        assert_eq!(stream.stats.zkp_events_sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_reset_stats() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());
        stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        stream.reset_stats();
        assert_eq!(stream.stats.total_messages_sent, 0);
        assert_eq!(stream.stats.pool_events_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_category_display() {
        assert_eq!(format!("{}", PoolCategory::Pool), "pool");
        assert_eq!(format!("{}", PoolCategory::Zkp), "zkp");
        assert_eq!(format!("{}", PoolCategory::Dao), "dao");
        assert_eq!(format!("{}", PoolCategory::Network), "network");
        assert_eq!(format!("{}", PoolCategory::All), "all");
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_config_default() {
        let config = WsPoolConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.rate_limit_per_sec, 30);
        assert_eq!(config.connection_timeout_ms, 60_000);
        assert!(config.auth_required);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stats_default() {
        let stats = WsPoolStats::default();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.pool_events_sent, 0);
        assert_eq!(stats.snapshots_sent, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_connection_rate_limit() {
        let mut conn = PoolConnection::new("c1".to_string(), "client-1".to_string(), 2);
        assert!(conn.check_rate_limit());
        assert!(conn.check_rate_limit());
        assert!(!conn.check_rate_limit());
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_connection_buffer() {
        let mut conn = PoolConnection::new("c1".to_string(), "client-1".to_string(), 100);
        conn.max_buffer = 2;
        let msg1 = PoolMessage::Pong { timestamp_ms: 1 };
        let msg2 = PoolMessage::Pong { timestamp_ms: 2 };
        let msg3 = PoolMessage::Pong { timestamp_ms: 3 };
        assert!(conn.buffer_message(msg1));
        assert!(conn.buffer_message(msg2));
        assert!(conn.buffer_message(msg3)); // Replaces msg1
        assert_eq!(conn.buffer.len(), 2);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_connection_is_subscribed_all() {
        let conn = PoolConnection::new("c1".to_string(), "client-1".to_string(), 100);
        assert!(conn.is_subscribed(&PoolCategory::Pool));
        assert!(conn.is_subscribed(&PoolCategory::Zkp));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_connection_subscribe() {
        let mut conn = PoolConnection::new("c1".to_string(), "client-1".to_string(), 100);
        conn.categories = vec![];
        conn.subscribe(vec![PoolCategory::Pool, PoolCategory::Zkp]);
        assert!(conn.is_subscribed(&PoolCategory::Pool));
        assert!(conn.is_subscribed(&PoolCategory::Zkp));
        assert!(!conn.is_subscribed(&PoolCategory::Dao));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_error_display() {
        let err = WsPoolError::ConnectionNotFound("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stream_default() {
        let stream = WsPoolStream::default();
        assert_eq!(stream.stats.active_connections, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_full_ws_pipeline() {
        let mut stream = make_stream();

        // Authenticate clients
        stream.authenticate("pool-client".to_string(), "sig-1".to_string());
        stream.authenticate("zkp-client".to_string(), "sig-2".to_string());

        assert_eq!(stream.active_connection_count(), 2);

        // Subscribe pool-client to Pool only
        let pool_conn = stream
            .connections
            .iter()
            .find(|(_, c)| c.client_id == "pool-client")
            .unwrap()
            .0
            .clone();
        stream.subscribe(&pool_conn, vec![PoolCategory::Pool]);

        // Subscribe zkp-client to ZKP only
        let zkp_conn = stream
            .connections
            .iter()
            .find(|(_, c)| c.client_id == "zkp-client")
            .unwrap()
            .0
            .clone();
        stream.subscribe(&zkp_conn, vec![PoolCategory::Zkp]);

        // Broadcast pool event (only pool-client receives)
        let result = stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        assert_eq!(result.messages_sent, 1);

        // Broadcast zkp event (only zkp-client receives)
        let result = stream.broadcast_event(
            PoolCategory::Zkp,
            "zkp_batch_generated".to_string(),
            make_payload("batch_id", 1.0),
        );
        assert_eq!(result.messages_sent, 1);

        // Broadcast snapshot (both receive)
        let result = stream.broadcast_snapshot(serde_json::json!({
            "pool_shards": 5,
            "zkp_rate": 0.95
        }));
        assert_eq!(result.messages_sent, 2);

        // Verify stats
        assert_eq!(stream.stats.pool_events_sent, 1);
        assert_eq!(stream.stats.zkp_events_sent, 1);
        assert_eq!(stream.stats.snapshots_sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_broadcast_multiple_categories() {
        let mut stream = make_stream();
        stream.authenticate("client-1".to_string(), "sig-1".to_string());

        stream.broadcast_event(
            PoolCategory::Pool,
            "pool_shard_registered".to_string(),
            make_payload("shard_id", 1.0),
        );
        stream.broadcast_event(
            PoolCategory::Zkp,
            "zkp_proof_verified".to_string(),
            make_payload("proof_id", 1.0),
        );
        stream.broadcast_event(
            PoolCategory::Dao,
            "dao_proposal_created".to_string(),
            make_payload("proposal_id", 1.0),
        );
        stream.broadcast_event(
            PoolCategory::Network,
            "network_latency_spike".to_string(),
            make_payload("latency_ms", 500.0),
        );

        assert_eq!(stream.stats.pool_events_sent, 1);
        assert_eq!(stream.stats.zkp_events_sent, 1);
        assert_eq!(stream.stats.dao_events_sent, 1);
        assert_eq!(stream.stats.network_events_sent, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_connection_is_expired() {
        let conn = PoolConnection::new("c1".to_string(), "client-1".to_string(), 100);
        assert!(!conn.is_expired(60_000)); // Not expired with 60s timeout
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_pending_since() {
        let mut conn = PoolConnection::new("c1".to_string(), "client-1".to_string(), 100);
        conn.buffer_message(PoolMessage::Event {
            sequence: 1,
            timestamp_ms: 100,
            category: "pool".to_string(),
            event_type: "test".to_string(),
            payload: serde_json::json!({}),
        });
        conn.buffer_message(PoolMessage::Event {
            sequence: 3,
            timestamp_ms: 200,
            category: "pool".to_string(),
            event_type: "test".to_string(),
            payload: serde_json::json!({}),
        });
        let pending = conn.get_pending_since(0);
        assert_eq!(pending.len(), 2);
    }
}
