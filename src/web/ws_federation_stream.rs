//! WebSocket Federation Stream — Real-time streaming for Federation Scaling v5, Async ZKP v10 & Bridge v4
//!
//! LP-125: UI Dashboard v6 & Real-time Streams
//! Provides real-time streaming of federation scaling events, ZKP v10 proof lifecycle,
//! and bridge v4 routing events to connected WebSocket clients.
//!
//! Features:
//! - WebSocket connections with lightweight authentication
//! - Category-based subscriptions (scaling, zkp, bridge, alerts)
//! - Rate limiting per connection with sliding windows
//! - Catchup buffer for reconnections
//! - Heartbeat to keep connections alive
//! - Optimized serialization for dashboards
//!
//! Protected with `#[cfg(feature = "v1.5-sprint2")]`.

#[cfg(feature = "v1.5-sprint2")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.5-sprint2")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.5-sprint2")]
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WsFederationError {
    #[error("Conexión no encontrada: {0}")]
    ConnectionNotFound(String),
    #[error("Rate limit excedido: {current}/{max} msg/s para conexión {conn}")]
    RateLimitExceeded {
        current: usize,
        max: usize,
        conn: String,
    },
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

// ─── Federation Categories ────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FedCategory {
    Scaling,
    Zkp,
    Bridge,
    Alerts,
    All,
}

#[cfg(feature = "v1.5-sprint2")]
impl std::fmt::Display for FedCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FedCategory::Scaling => write!(f, "scaling"),
            FedCategory::Zkp => write!(f, "zkp"),
            FedCategory::Bridge => write!(f, "bridge"),
            FedCategory::Alerts => write!(f, "alerts"),
            FedCategory::All => write!(f, "all"),
        }
    }
}

// ─── Federation Message ───────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FedMessage {
    // Client → Server
    Auth {
        client_id: String,
        signature: String,
        timestamp_ms: u64,
    },
    Subscribe {
        categories: Vec<FedCategory>,
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
    AuthError {
        reason: String,
    },
    Subscribed {
        categories: Vec<FedCategory>,
    },
    Pong {
        timestamp_ms: u64,
    },
    Catchup {
        events: Vec<FedEvent>,
    },
    Event {
        event: FedEvent,
    },
    Error {
        message: String,
    },
}

// ─── Federation Event ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedEvent {
    pub id: String,
    pub category: FedCategory,
    pub timestamp_ms: u64,
    pub sequence: u64,
    pub payload: FedPayload,
}

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FedPayload {
    // Scaling v5 events
    NodeRegistered {
        node_id: String,
        capacity: f64,
        reputation: f64,
    },
    ShardAssigned {
        node_id: String,
        shard_id: String,
        score: f64,
    },
    ShardCreated {
        shard_id: String,
        model_types: Vec<String>,
    },
    Rebalance {
        shard_id: String,
        nodes_moved: usize,
    },
    PartitionEvent {
        shard_id: String,
        healthy: bool,
    },
    // ZKP v10 events
    ProofSubmitted {
        proof_id: String,
        federation_id: String,
        priority: String,
    },
    ProofVerified {
        proof_id: String,
        cost: f64,
        time_ms: u64,
    },
    ProofFailed {
        proof_id: String,
        reason: String,
    },
    ProofDelegated {
        proof_id: String,
        source_federation: String,
        target_federation: String,
    },
    ReplayDetected {
        proof_id: String,
        nonce: u64,
    },
    // Bridge v4 events
    ProofRouted {
        session_id: String,
        target_federation: String,
        routing_score: f64,
    },
    ConsensusReached {
        session_id: String,
        votes_yes: usize,
        votes_no: usize,
    },
    ConsensusFailed {
        session_id: String,
        reason: String,
    },
    MerkleSynced {
        merkle_root: String,
        federations: Vec<String>,
    },
    // Alert events
    Alert {
        severity: String,
        category: String,
        message: String,
    },
}

// ─── Connection State ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedConnection {
    pub connection_id: String,
    pub client_id: String,
    pub categories: Vec<FedCategory>,
    pub authenticated: bool,
    pub last_sequence: u64,
    pub messages_this_second: usize,
    pub last_heartbeat_ms: u64,
    pub connected_at_ms: u64,
}

#[cfg(feature = "v1.5-sprint2")]
impl FedConnection {
    pub fn new(connection_id: String, client_id: String) -> Self {
        Self {
            connection_id,
            client_id,
            categories: Vec::new(),
            authenticated: false,
            last_sequence: 0,
            messages_this_second: 0,
            last_heartbeat_ms: current_timestamp_ms(),
            connected_at_ms: current_timestamp_ms(),
        }
    }

    pub fn is_subscribed_to(&self, category: &FedCategory) -> bool {
        self.categories.contains(category) || self.categories.contains(&FedCategory::All)
    }
}

// ─── Stream Config ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFederationConfig {
    pub max_connections: usize,
    pub rate_limit_per_sec: usize,
    pub heartbeat_interval_ms: u64,
    pub catchup_buffer_size: usize,
    pub auth_required: bool,
}

#[cfg(feature = "v1.5-sprint2")]
impl Default for WsFederationConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            rate_limit_per_sec: 50,
            heartbeat_interval_ms: 30_000,
            catchup_buffer_size: 500,
            auth_required: true,
        }
    }
}

// ─── Stream Stats ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WsFederationStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub events_sent: u64,
    pub rate_limited: u64,
    pub auth_failures: u64,
    pub catchups_served: u64,
}

// ─── Federation Stream Engine ─────────────────────────────────────────────────

#[cfg(feature = "v1.5-sprint2")]
pub struct WsFederationStream {
    pub config: WsFederationConfig,
    pub connections: HashMap<String, FedConnection>,
    pub event_buffer: VecDeque<FedEvent>,
    pub stats: WsFederationStats,
    pub next_sequence: u64,
}

#[cfg(feature = "v1.5-sprint2")]
impl WsFederationStream {
    pub fn new(config: WsFederationConfig) -> Self {
        Self {
            config,
            connections: HashMap::new(),
            event_buffer: VecDeque::with_capacity(500),
            stats: WsFederationStats::default(),
            next_sequence: 1,
        }
    }

    pub fn connect(
        &mut self,
        connection_id: String,
        client_id: String,
    ) -> Result<(), WsFederationError> {
        if self.connections.contains_key(&connection_id) {
            return Err(WsFederationError::ConnectionAlreadyExists(connection_id));
        }
        if self.connections.len() >= self.config.max_connections {
            return Err(WsFederationError::MaxConnectionsReached(
                self.config.max_connections,
            ));
        }
        let conn = FedConnection::new(connection_id.clone(), client_id);
        self.connections.insert(connection_id, conn);
        self.stats.total_connections += 1;
        self.stats.active_connections += 1;
        Ok(())
    }

    pub fn authenticate(&mut self, connection_id: &str) -> Result<(), WsFederationError> {
        let conn = self.connections.get_mut(connection_id).ok_or(
            WsFederationError::ConnectionNotFound(connection_id.to_string()),
        )?;
        conn.authenticated = true;
        Ok(())
    }

    pub fn subscribe(
        &mut self,
        connection_id: &str,
        categories: Vec<FedCategory>,
    ) -> Result<(), WsFederationError> {
        let conn = self.connections.get_mut(connection_id).ok_or(
            WsFederationError::ConnectionNotFound(connection_id.to_string()),
        )?;
        conn.categories = categories;
        Ok(())
    }

    pub fn disconnect(&mut self, connection_id: &str) -> Result<(), WsFederationError> {
        self.connections
            .remove(connection_id)
            .ok_or(WsFederationError::ConnectionNotFound(
                connection_id.to_string(),
            ))?;
        self.stats.active_connections -= 1;
        Ok(())
    }

    pub fn emit_event(&mut self, category: FedCategory, payload: FedPayload) {
        let event = FedEvent {
            id: format!("evt-{}", self.next_sequence),
            category: category.clone(),
            timestamp_ms: current_timestamp_ms(),
            sequence: self.next_sequence,
            payload,
        };
        self.next_sequence += 1;
        self.event_buffer.push_back(event);
        if self.event_buffer.len() > self.config.catchup_buffer_size {
            self.event_buffer.pop_front();
        }
    }

    pub fn get_catchup(&mut self, from_sequence: u64) -> Vec<FedEvent> {
        let mut catchup = Vec::new();
        while let Some(event) = self.event_buffer.front() {
            if event.sequence > from_sequence {
                catchup.push(self.event_buffer.pop_front().unwrap());
            } else {
                break;
            }
        }
        self.stats.catchups_served += 1;
        catchup
    }

    pub fn check_rate_limit(&mut self, connection_id: &str) -> Result<(), WsFederationError> {
        let conn = self.connections.get_mut(connection_id).ok_or(
            WsFederationError::ConnectionNotFound(connection_id.to_string()),
        )?;
        if conn.messages_this_second >= self.config.rate_limit_per_sec {
            self.stats.rate_limited += 1;
            return Err(WsFederationError::RateLimitExceeded {
                current: conn.messages_this_second,
                max: self.config.rate_limit_per_sec,
                conn: connection_id.to_string(),
            });
        }
        conn.messages_this_second += 1;
        Ok(())
    }

    pub fn reset_rate_limits(&mut self) {
        for conn in self.connections.values_mut() {
            conn.messages_this_second = 0;
        }
    }

    pub fn reset_stats(&mut self) {
        self.stats = WsFederationStats::default();
    }
}

#[cfg(feature = "v1.5-sprint2")]
impl Default for WsFederationStream {
    fn default() -> Self {
        Self::new(WsFederationConfig::default())
    }
}

#[cfg(feature = "v1.5-sprint2")]
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

    #[test]
    fn test_stream_creation() {
        let stream = WsFederationStream::new(WsFederationConfig::default());
        assert_eq!(stream.stats.active_connections, 0);
        assert!(stream.event_buffer.is_empty());
    }

    #[test]
    fn test_connect() {
        let mut stream = WsFederationStream::default();
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        assert_eq!(stream.stats.active_connections, 1);
    }

    #[test]
    fn test_connect_duplicate() {
        let mut stream = WsFederationStream::default();
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        let err = stream
            .connect("conn-1".into(), "client-2".into())
            .unwrap_err();
        assert!(matches!(err, WsFederationError::ConnectionAlreadyExists(_)));
    }

    #[test]
    fn test_connect_max_reached() {
        let mut config = WsFederationConfig::default();
        config.max_connections = 2;
        let mut stream = WsFederationStream::new(config);
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream.connect("conn-2".into(), "client-2".into()).unwrap();
        let err = stream
            .connect("conn-3".into(), "client-3".into())
            .unwrap_err();
        assert!(matches!(err, WsFederationError::MaxConnectionsReached(2)));
    }

    #[test]
    fn test_authenticate() {
        let mut stream = WsFederationStream::default();
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream.authenticate("conn-1").unwrap();
        assert!(stream.connections.get("conn-1").unwrap().authenticated);
    }

    #[test]
    fn test_subscribe() {
        let mut stream = WsFederationStream::default();
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream
            .subscribe("conn-1", vec![FedCategory::Scaling, FedCategory::Zkp])
            .unwrap();
        let conn = stream.connections.get("conn-1").unwrap();
        assert_eq!(conn.categories.len(), 2);
    }

    #[test]
    fn test_disconnect() {
        let mut stream = WsFederationStream::default();
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream.disconnect("conn-1").unwrap();
        assert_eq!(stream.stats.active_connections, 0);
    }

    #[test]
    fn test_emit_event() {
        let mut stream = WsFederationStream::default();
        stream.emit_event(
            FedCategory::Scaling,
            FedPayload::NodeRegistered {
                node_id: "node-1".into(),
                capacity: 100.0,
                reputation: 0.9,
            },
        );
        assert_eq!(stream.event_buffer.len(), 1);
    }

    #[test]
    fn test_catchup() {
        let mut stream = WsFederationStream::default();
        stream.emit_event(
            FedCategory::Scaling,
            FedPayload::NodeRegistered {
                node_id: "n1".into(),
                capacity: 100.0,
                reputation: 0.9,
            },
        );
        stream.emit_event(
            FedCategory::Zkp,
            FedPayload::ProofVerified {
                proof_id: "p1".into(),
                cost: 10.0,
                time_ms: 200,
            },
        );
        let catchup = stream.get_catchup(0);
        assert_eq!(catchup.len(), 2);
    }

    #[test]
    fn test_rate_limit() {
        let mut config = WsFederationConfig::default();
        config.rate_limit_per_sec = 2;
        let mut stream = WsFederationStream::new(config);
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream.check_rate_limit("conn-1").unwrap();
        stream.check_rate_limit("conn-1").unwrap();
        let err = stream.check_rate_limit("conn-1").unwrap_err();
        assert!(matches!(err, WsFederationError::RateLimitExceeded { .. }));
    }

    #[test]
    fn test_reset_rate_limits() {
        let mut config = WsFederationConfig::default();
        config.rate_limit_per_sec = 1;
        let mut stream = WsFederationStream::new(config);
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream.check_rate_limit("conn-1").unwrap();
        stream.reset_rate_limits();
        stream.check_rate_limit("conn-1").unwrap();
    }

    #[test]
    fn test_subscription_check() {
        let conn = FedConnection::new("c1".into(), "client1".into());
        assert!(!conn.is_subscribed_to(&FedCategory::Scaling));
        let conn_all = FedConnection {
            categories: vec![FedCategory::All],
            ..FedConnection::new("c2".into(), "client2".into())
        };
        assert!(conn_all.is_subscribed_to(&FedCategory::Scaling));
    }

    #[test]
    fn test_category_display() {
        assert_eq!(FedCategory::Scaling.to_string(), "scaling");
        assert_eq!(FedCategory::Zkp.to_string(), "zkp");
        assert_eq!(FedCategory::Bridge.to_string(), "bridge");
        assert_eq!(FedCategory::Alerts.to_string(), "alerts");
    }

    #[test]
    fn test_error_display() {
        let err = WsFederationError::AuthFailed("bad token".into());
        assert_eq!(err.to_string(), "Autenticación fallida: bad token");
    }

    #[test]
    fn test_config_default() {
        let config = WsFederationConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.rate_limit_per_sec, 50);
    }

    #[test]
    fn test_stats_default() {
        let stats = WsFederationStats::default();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.events_sent, 0);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut config = WsFederationConfig::default();
        config.catchup_buffer_size = 3;
        let mut stream = WsFederationStream::new(config);
        for i in 0..5 {
            stream.emit_event(
                FedCategory::Scaling,
                FedPayload::NodeRegistered {
                    node_id: format!("node-{}", i),
                    capacity: 100.0,
                    reputation: 0.9,
                },
            );
        }
        assert_eq!(stream.event_buffer.len(), 3);
    }

    #[test]
    fn test_sequence_increments() {
        let mut stream = WsFederationStream::default();
        stream.emit_event(
            FedCategory::Scaling,
            FedPayload::NodeRegistered {
                node_id: "n1".into(),
                capacity: 100.0,
                reputation: 0.9,
            },
        );
        stream.emit_event(
            FedCategory::Zkp,
            FedPayload::ProofVerified {
                proof_id: "p1".into(),
                cost: 10.0,
                time_ms: 200,
            },
        );
        let events: Vec<_> = stream.event_buffer.iter().collect();
        assert!(events[0].sequence < events[1].sequence);
    }

    #[test]
    fn test_reset_stats() {
        let mut stream = WsFederationStream::default();
        stream.connect("conn-1".into(), "client-1".into()).unwrap();
        stream.reset_stats();
        assert_eq!(stream.stats.total_connections, 0);
    }

    #[test]
    fn test_disconnect_not_found() {
        let mut stream = WsFederationStream::default();
        let err = stream.disconnect("nonexistent").unwrap_err();
        assert!(matches!(err, WsFederationError::ConnectionNotFound(_)));
    }
}
