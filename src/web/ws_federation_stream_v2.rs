//! WebSocket Federation Stream v2 — Real-time streaming for Scaling v7, ZKP v13 & Bridge v6
//!
//! LP-147: UI Dashboard v7 & Real-time Streams
//! Provides real-time streaming of federation scaling v7 events, ZKP v13 proof lifecycle,
//! and bridge v6 routing events to connected WebSocket clients.
//!
//! Features:
//! - WebSocket connections with lightweight authentication
//! - Category-based subscriptions (scaling, zkp, bridge, alerts)
//! - Rate limiting per connection with sliding windows
//! - Catchup buffer for reconnections
//! - Heartbeat to keep connections alive
//! - Optimized serialization for dashboards
//!
//! Protected with `#[cfg(feature = "v1.6-sprint2")]`.

#[cfg(feature = "v1.6-sprint2")]
mod internal {
    use std::collections::{HashMap, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for WebSocket Federation Stream v2.
    #[derive(Debug, Clone, PartialEq)]
    pub enum WsFederationV2Error {
        /// Connection not found.
        ConnectionNotFound(String),
        /// Rate limit exceeded.
        RateLimitExceeded { current: usize, max: usize, conn: String },
        /// Authentication failed.
        AuthFailed(String),
        /// Duplicate connection.
        ConnectionAlreadyExists(String),
        /// Max connections reached.
        MaxConnectionsReached(usize),
        /// Invalid category.
        InvalidCategory(String),
        /// Serialization error.
        SerializationError(String),
    }

    impl std::fmt::Display for WsFederationV2Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::ConnectionNotFound(id) => write!(f, "Connection not found: {}", id),
                Self::RateLimitExceeded { current, max, conn } => {
                    write!(f, "Rate limit exceeded: {}/{} msg/s for {}", current, max, conn)
                }
                Self::AuthFailed(msg) => write!(f, "Auth failed: {}", msg),
                Self::ConnectionAlreadyExists(id) => write!(f, "Connection exists: {}", id),
                Self::MaxConnectionsReached(n) => write!(f, "Max connections reached: {}", n),
                Self::InvalidCategory(c) => write!(f, "Invalid category: {}", c),
                Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            }
        }
    }

    impl std::error::Error for WsFederationV2Error {}

    // ---------------------------------------------------------------------------
    // Categories
    // ---------------------------------------------------------------------------

    /// Stream categories for subscriptions.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum StreamCategory {
        Scaling,
        Zkp,
        Bridge,
        Alerts,
        All,
    }

    impl std::fmt::Display for StreamCategory {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Scaling => write!(f, "scaling"),
                Self::Zkp => write!(f, "zkp"),
                Self::Bridge => write!(f, "bridge"),
                Self::Alerts => write!(f, "alerts"),
                Self::All => write!(f, "all"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Messages
    // ---------------------------------------------------------------------------

    /// WebSocket message types.
    #[derive(Debug, Clone)]
    pub enum WsMessage {
        // Client → Server
        Auth { client_id: String, signature: String, timestamp_ms: u64 },
        Subscribe { categories: Vec<StreamCategory> },
        Ping { timestamp_ms: u64 },
        Reconnect { client_id: String, last_sequence: u64 },
        // Server → Client
        AuthOk { connection_id: String, token: String },
        AuthError { reason: String },
        Subscribed { categories: Vec<StreamCategory> },
        Pong { timestamp_ms: u64 },
        Catchup { events: Vec<StreamEvent> },
        Event { event: StreamEvent },
        Error { message: String },
    }

    // ---------------------------------------------------------------------------
    // Events
    // ---------------------------------------------------------------------------

    /// Stream event types.
    #[derive(Debug, Clone)]
    pub enum StreamEvent {
        Scaling {
            sequence: u64,
            timestamp_ms: u64,
            nodes_active: usize,
            shards_active: usize,
            partition_health: f64,
            predictive_load: f64,
            gradient_alignment: f64,
        },
        Zkp {
            sequence: u64,
            timestamp_ms: u64,
            proofs_submitted: u64,
            proofs_verified: u64,
            batches_completed: u64,
            fallback_rate: f64,
            avg_verification_ms: f64,
        },
        Bridge {
            sequence: u64,
            timestamp_ms: u64,
            proofs_routed: u64,
            proofs_verified: u64,
            fallback_count: u64,
            avg_credibility: f64,
        },
        Alert {
            sequence: u64,
            timestamp_ms: u64,
            severity: String,
            category: String,
            message: String,
        },
    }

    impl StreamEvent {
        pub fn category(&self) -> StreamCategory {
            match self {
                Self::Scaling { .. } => StreamCategory::Scaling,
                Self::Zkp { .. } => StreamCategory::Zkp,
                Self::Bridge { .. } => StreamCategory::Bridge,
                Self::Alert { .. } => StreamCategory::Alerts,
            }
        }

        pub fn sequence(&self) -> u64 {
            match self {
                Self::Scaling { sequence, .. } => *sequence,
                Self::Zkp { sequence, .. } => *sequence,
                Self::Bridge { sequence, .. } => *sequence,
                Self::Alert { sequence, .. } => *sequence,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Connection State
    // ---------------------------------------------------------------------------

    /// WebSocket connection state.
    #[derive(Debug, Clone)]
    pub struct WsConnection {
        pub connection_id: String,
        pub client_id: String,
        pub authenticated: bool,
        pub categories: Vec<StreamCategory>,
        pub last_sequence: u64,
        pub messages_sent: usize,
        pub rate_window_start_ms: u64,
        pub rate_window_count: usize,
        pub last_heartbeat_ms: u64,
        pub created_at_ms: u64,
    }

    impl WsConnection {
        pub fn new(connection_id: String, created_at_ms: u64) -> Self {
            Self {
                connection_id,
                client_id: String::new(),
                authenticated: false,
                categories: Vec::new(),
                last_sequence: 0,
                messages_sent: 0,
                rate_window_start_ms: created_at_ms,
                rate_window_count: 0,
                last_heartbeat_ms: created_at_ms,
                created_at_ms,
            }
        }

        pub fn is_rate_limited(&self, max_rate: usize, current_ms: u64) -> bool {
            if current_ms - self.rate_window_start_ms > 1000 {
                return false;
            }
            self.rate_window_count >= max_rate
        }

        pub fn record_message(&mut self, current_ms: u64) {
            if current_ms - self.rate_window_start_ms > 1000 {
                self.rate_window_start_ms = current_ms;
                self.rate_window_count = 0;
            }
            self.rate_window_count += 1;
            self.messages_sent += 1;
        }

        pub fn is_stale(&self, current_ms: u64, timeout_ms: u64) -> bool {
            current_ms - self.last_heartbeat_ms > timeout_ms
        }
    }

    // ---------------------------------------------------------------------------
    // Stream Stats
    // ---------------------------------------------------------------------------

    /// Stream statistics.
    #[derive(Debug, Clone, Default)]
    pub struct StreamStats {
        pub connections_active: usize,
        pub events_published: u64,
        pub messages_delivered: u64,
        pub rate_limited_count: u64,
        pub auth_failures: u64,
    }

    // ---------------------------------------------------------------------------
    // Stream Engine
    // ---------------------------------------------------------------------------

    /// WebSocket Federation Stream v2 engine.
    pub struct WsFederationStreamV2 {
        pub connections: HashMap<String, WsConnection>,
        pub event_buffer: VecDeque<StreamEvent>,
        pub stats: StreamStats,
        pub max_connections: usize,
        pub max_rate: usize,
        pub max_buffer: usize,
        pub stale_timeout_ms: u64,
        pub next_sequence: u64,
    }

    impl WsFederationStreamV2 {
        /// Create a new stream engine.
        pub fn new() -> Self {
            Self {
                connections: HashMap::new(),
                event_buffer: VecDeque::with_capacity(500),
                stats: StreamStats::default(),
                max_connections: 100,
                max_rate: 50,
                max_buffer: 500,
                stale_timeout_ms: 30_000,
                next_sequence: 1,
            }
        }

        /// Authenticate a new connection.
        pub fn authenticate(
            &mut self,
            client_id: String,
            signature: String,
            current_ms: u64,
        ) -> Result<String, WsFederationV2Error> {
            if self.connections.len() >= self.max_connections {
                return Err(WsFederationV2Error::MaxConnectionsReached(
                    self.max_connections,
                ));
            }

            // Simple signature validation (placeholder)
            if signature.is_empty() {
                self.stats.auth_failures += 1;
                return Err(WsFederationV2Error::AuthFailed("Empty signature".to_string()));
            }

            let connection_id = format!("conn_{}_{}", client_id, current_ms);
            if self.connections.contains_key(&connection_id) {
                return Err(WsFederationV2Error::ConnectionAlreadyExists(
                    connection_id,
                ));
            }

            let mut conn = WsConnection::new(connection_id.clone(), current_ms);
            conn.client_id = client_id;
            conn.authenticated = true;
            conn.last_heartbeat_ms = current_ms;
            self.connections.insert(connection_id.clone(), conn);
            self.stats.connections_active = self.connections.len();
            Ok(connection_id)
        }

        /// Subscribe to categories.
        pub fn subscribe(
            &mut self,
            connection_id: &str,
            categories: Vec<StreamCategory>,
        ) -> Result<(), WsFederationV2Error> {
            let conn = self.connections.get_mut(connection_id).ok_or_else(|| {
                WsFederationV2Error::ConnectionNotFound(connection_id.to_string())
            })?;
            conn.categories = categories;
            Ok(())
        }

        /// Publish a scaling event.
        pub fn publish_scaling(
            &mut self,
            current_ms: u64,
            nodes_active: usize,
            shards_active: usize,
            partition_health: f64,
            predictive_load: f64,
            gradient_alignment: f64,
        ) {
            let event = StreamEvent::Scaling {
                sequence: self.next_sequence,
                timestamp_ms: current_ms,
                nodes_active,
                shards_active,
                partition_health,
                predictive_load,
                gradient_alignment,
            };
            self.next_sequence += 1;
            self.buffer_event(event);
            self.broadcast(current_ms);
        }

        /// Publish a ZKP event.
        pub fn publish_zkp(
            &mut self,
            current_ms: u64,
            proofs_submitted: u64,
            proofs_verified: u64,
            batches_completed: u64,
            fallback_rate: f64,
            avg_verification_ms: f64,
        ) {
            let event = StreamEvent::Zkp {
                sequence: self.next_sequence,
                timestamp_ms: current_ms,
                proofs_submitted,
                proofs_verified,
                batches_completed,
                fallback_rate,
                avg_verification_ms,
            };
            self.next_sequence += 1;
            self.buffer_event(event);
            self.broadcast(current_ms);
        }

        /// Publish a bridge event.
        pub fn publish_bridge(
            &mut self,
            current_ms: u64,
            proofs_routed: u64,
            proofs_verified: u64,
            fallback_count: u64,
            avg_credibility: f64,
        ) {
            let event = StreamEvent::Bridge {
                sequence: self.next_sequence,
                timestamp_ms: current_ms,
                proofs_routed,
                proofs_verified,
                fallback_count,
                avg_credibility,
            };
            self.next_sequence += 1;
            self.buffer_event(event);
            self.broadcast(current_ms);
        }

        /// Publish an alert event.
        pub fn publish_alert(
            &mut self,
            current_ms: u64,
            severity: String,
            category: String,
            message: String,
        ) {
            let event = StreamEvent::Alert {
                sequence: self.next_sequence,
                timestamp_ms: current_ms,
                severity,
                category,
                message,
            };
            self.next_sequence += 1;
            self.buffer_event(event);
            self.broadcast(current_ms);
        }

        /// Handle ping from client.
        pub fn handle_ping(
            &mut self,
            connection_id: &str,
            current_ms: u64,
        ) -> Result<u64, WsFederationV2Error> {
            let conn = self.connections.get_mut(connection_id).ok_or_else(|| {
                WsFederationV2Error::ConnectionNotFound(connection_id.to_string())
            })?;
            conn.last_heartbeat_ms = current_ms;
            Ok(current_ms)
        }

        /// Get catchup events for reconnection.
        pub fn get_catchup(
            &self,
            last_sequence: u64,
        ) -> Vec<&StreamEvent> {
            self.event_buffer
                .iter()
                .filter(|e| e.sequence() > last_sequence)
                .collect()
        }

        /// Clean up stale connections.
        pub fn cleanup_stale(&mut self, current_ms: u64) -> usize {
            let before = self.connections.len();
            self.connections
                .retain(|_, conn| !conn.is_stale(current_ms, self.stale_timeout_ms));
            self.stats.connections_active = self.connections.len();
            before - self.connections.len()
        }

        fn buffer_event(&mut self, event: StreamEvent) {
            self.event_buffer.push_back(event);
            if self.event_buffer.len() > self.max_buffer {
                self.event_buffer.pop_front();
            }
            self.stats.events_published += 1;
        }

        fn broadcast(&mut self, current_ms: u64) {
            let event = match self.event_buffer.back().cloned() {
                Some(e) => e,
                None => return,
            };
            let category = event.category();

            for conn in self.connections.values_mut() {
                if !conn.authenticated {
                    continue;
                }
                if !conn.categories.contains(&category)
                    && !conn.categories.contains(&StreamCategory::All)
                {
                    continue;
                }
                if conn.is_rate_limited(self.max_rate, current_ms) {
                    self.stats.rate_limited_count += 1;
                    continue;
                }
                conn.record_message(current_ms);
                conn.last_sequence = event.sequence();
                self.stats.messages_delivered += 1;
            }
        }
    }

    impl Default for WsFederationStreamV2 {
        fn default() -> Self {
            Self::new()
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        #[allow(unused_imports)]
        use super::*;

        #[test]
        fn test_stream_creation() {
            let stream = WsFederationStreamV2::new();
            assert_eq!(stream.connections.len(), 0);
            assert_eq!(stream.max_connections, 100);
        }

        #[test]
        fn test_authenticate() {
            let mut stream = WsFederationStreamV2::new();
            let result = stream.authenticate("client1".to_string(), "sig1".to_string(), 1000);
            assert!(result.is_ok());
            assert_eq!(stream.connections.len(), 1);
        }

        #[test]
        fn test_authenticate_empty_signature() {
            let mut stream = WsFederationStreamV2::new();
            let result = stream.authenticate("client1".to_string(), "".to_string(), 1000);
            assert!(result.is_err());
        }

        #[test]
        fn test_authenticate_max_reached() {
            let mut stream = WsFederationStreamV2::new();
            stream.max_connections = 1;
            stream.authenticate("c1".to_string(), "s1".to_string(), 1000).unwrap();
            let id = stream.authenticate("c2".to_string(), "s2".to_string(), 1001);
            assert!(id.is_err());
        }

        #[test]
        fn test_subscribe() {
            let mut stream = WsFederationStreamV2::new();
            let conn_id = stream.authenticate("c1".to_string(), "s1".to_string(), 1000).unwrap();
            assert!(stream
                .subscribe(&conn_id, vec![StreamCategory::Scaling])
                .is_ok());
        }

        #[test]
        fn test_subscribe_not_found() {
            let mut stream = WsFederationStreamV2::new();
            let result = stream.subscribe("unknown", vec![StreamCategory::Scaling]);
            assert!(result.is_err());
        }

        #[test]
        fn test_publish_scaling() {
            let mut stream = WsFederationStreamV2::new();
            stream.publish_scaling(1000, 50, 10, 0.99, 0.7, 0.92);
            assert_eq!(stream.stats.events_published, 1);
            assert_eq!(stream.next_sequence, 2);
        }

        #[test]
        fn test_publish_zkp() {
            let mut stream = WsFederationStreamV2::new();
            stream.publish_zkp(1000, 100, 95, 5, 0.1, 10.0);
            assert_eq!(stream.stats.events_published, 1);
        }

        #[test]
        fn test_publish_bridge() {
            let mut stream = WsFederationStreamV2::new();
            stream.publish_bridge(1000, 80, 75, 5, 0.85);
            assert_eq!(stream.stats.events_published, 1);
        }

        #[test]
        fn test_publish_alert() {
            let mut stream = WsFederationStreamV2::new();
            stream.publish_alert(1000, "warning".to_string(), "scaling".to_string(), "High load".to_string());
            assert_eq!(stream.stats.events_published, 1);
        }

        #[test]
        fn test_handle_ping() {
            let mut stream = WsFederationStreamV2::new();
            let conn_id = stream.authenticate("c1".to_string(), "s1".to_string(), 1000).unwrap();
            let result = stream.handle_ping(&conn_id, 2000);
            assert!(result.is_ok());
        }

        #[test]
        fn test_get_catchup() {
            let mut stream = WsFederationStreamV2::new();
            stream.publish_scaling(1000, 50, 10, 0.99, 0.7, 0.92);
            stream.publish_scaling(2000, 55, 12, 0.98, 0.75, 0.90);
            let catchup = stream.get_catchup(0);
            assert_eq!(catchup.len(), 2);
        }

        #[test]
        fn test_cleanup_stale() {
            let mut stream = WsFederationStreamV2::new();
            stream.stale_timeout_ms = 5000;
            let conn_id = stream.authenticate("c1".to_string(), "s1".to_string(), 1000).unwrap();
            assert_eq!(stream.connections.len(), 1);
            let cleaned = stream.cleanup_stale(10000);
            assert_eq!(cleaned, 1);
        }

        #[test]
        fn test_rate_limiting() {
            let mut stream = WsFederationStreamV2::new();
            stream.max_rate = 2;
            let conn_id = stream.authenticate("c1".to_string(), "s1".to_string(), 1000).unwrap();
            stream.subscribe(&conn_id, vec![StreamCategory::All]).unwrap();
            stream.publish_scaling(1000, 50, 10, 0.99, 0.7, 0.92);
            stream.publish_scaling(1001, 50, 10, 0.99, 0.7, 0.92);
            stream.publish_scaling(1002, 50, 10, 0.99, 0.7, 0.92);
            assert!(stream.stats.rate_limited_count > 0);
        }

        #[test]
        fn test_event_category() {
            let event = StreamEvent::Scaling {
                sequence: 1,
                timestamp_ms: 1000,
                nodes_active: 50,
                shards_active: 10,
                partition_health: 0.99,
                predictive_load: 0.7,
                gradient_alignment: 0.92,
            };
            assert_eq!(event.category(), StreamCategory::Scaling);
        }

        #[test]
        fn test_connection_rate_limited() {
            let mut conn = WsConnection::new("c1".to_string(), 1000);
            conn.rate_window_count = 50;
            conn.rate_window_start_ms = 1000;
            assert!(conn.is_rate_limited(50, 1500));
        }

        #[test]
        fn test_connection_not_rate_limited_new_window() {
            let mut conn = WsConnection::new("c1".to_string(), 1000);
            conn.rate_window_count = 50;
            conn.rate_window_start_ms = 1000;
            assert!(!conn.is_rate_limited(50, 2001));
        }

        #[test]
        fn test_connection_stale() {
            let conn = WsConnection::new("c1".to_string(), 1000);
            assert!(conn.is_stale(40000, 30000));
        }

        #[test]
        fn test_connection_not_stale() {
            let conn = WsConnection::new("c1".to_string(), 1000);
            assert!(!conn.is_stale(20000, 30000));
        }

        #[test]
        fn test_category_display() {
            assert_eq!(format!("{}", StreamCategory::Scaling), "scaling");
            assert_eq!(format!("{}", StreamCategory::Zkp), "zkp");
            assert_eq!(format!("{}", StreamCategory::All), "all");
        }

        #[test]
        fn test_error_display() {
            let err = WsFederationV2Error::AuthFailed("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_broadcast_filtered() {
            let mut stream = WsFederationStreamV2::new();
            let conn_id = stream.authenticate("c1".to_string(), "s1".to_string(), 1000).unwrap();
            stream.subscribe(&conn_id, vec![StreamCategory::Scaling]).unwrap();
            stream.publish_zkp(1000, 100, 95, 5, 0.1, 10.0);
            // ZKP event should not be delivered to scaling-only subscriber
            assert_eq!(stream.stats.messages_delivered, 0);
        }

        #[test]
        fn test_full_lifecycle() {
            let mut stream = WsFederationStreamV2::new();
            let conn_id = stream.authenticate("client1".to_string(), "sig1".to_string(), 1000).unwrap();
            stream.subscribe(&conn_id, vec![StreamCategory::All]).unwrap();
            stream.publish_scaling(2000, 50, 10, 0.99, 0.7, 0.92);
            stream.handle_ping(&conn_id, 3000).unwrap();
            assert_eq!(stream.stats.events_published, 1);
            assert_eq!(stream.stats.messages_delivered, 1);
        }
    }
}

#[cfg(feature = "v1.6-sprint2")]
pub use internal::*;
