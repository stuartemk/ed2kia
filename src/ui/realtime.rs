//! Realtime UI Backend — WebSocket upgrade, session management, rate limiting, event broadcasting

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RealtimeError {
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("rate limit exceeded: {current}/{max} msg/s")]
    RateLimitExceeded { current: usize, max: usize },
    #[error("websocket upgrade failed: {0}")]
    UpgradeFailed(String),
    #[error("invalid event type: {0}")]
    InvalidEventType(String),
}

// ─── Event Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    GovernanceVote,
    AlignmentDrift,
    FederationSync,
    SloBreach,
    MarketplaceTrade,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::GovernanceVote => write!(f, "governance_vote"),
            EventType::AlignmentDrift => write!(f, "alignment_drift"),
            EventType::FederationSync => write!(f, "federation_sync"),
            EventType::SloBreach => write!(f, "slo_breach"),
            EventType::MarketplaceTrade => write!(f, "marketplace_trade"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeEvent {
    pub event_type: EventType,
    pub payload: serde_json::Value,
    pub timestamp_ms: u64,
    pub source_node: String,
}

impl RealtimeEvent {
    pub fn new(event_type: EventType, payload: serde_json::Value, source_node: String) -> Self {
        Self {
            event_type,
            payload,
            timestamp_ms: current_timestamp_ms(),
            source_node,
        }
    }
}

// ─── Session Management ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WsResult {
    pub session_id: String,
    pub messages_sent: usize,
    pub rate_limited: bool,
    pub active_sessions: usize,
}

impl WsResult {
    pub fn new(
        session_id: String,
        messages_sent: usize,
        rate_limited: bool,
        active_sessions: usize,
    ) -> Self {
        Self {
            session_id,
            messages_sent,
            rate_limited,
            active_sessions,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub connected_at: Instant,
    pub messages_sent: usize,
    pub message_timestamps: VecDeque<Instant>,
    pub rate_limit_per_sec: usize,
}

impl SessionState {
    pub fn new(session_id: String, rate_limit_per_sec: usize) -> Self {
        Self {
            session_id,
            connected_at: Instant::now(),
            messages_sent: 0,
            message_timestamps: VecDeque::new(),
            rate_limit_per_sec,
        }
    }

    /// Check rate limit and prune old timestamps. Returns true if allowed.
    pub fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        // Remove timestamps older than 1 second
        while let Some(&old) = self.message_timestamps.front() {
            if now.duration_since(old).as_secs() >= 1 {
                self.message_timestamps.pop_front();
            } else {
                break;
            }
        }
        self.message_timestamps.len() < self.rate_limit_per_sec
    }

    pub fn record_message(&mut self) {
        self.messages_sent += 1;
        self.message_timestamps.push_back(Instant::now());
    }

    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        self.connected_at.elapsed().as_secs() > max_age_secs
    }
}

// ─── RealtimeUIBackend ───────────────────────────────────────────────────────

pub struct RealtimeUIBackend {
    sessions: Arc<DashMap<String, SessionState>>,
    rate_limit_per_sec: usize,
    event_buffer: Arc<DashMap<String, VecDeque<RealtimeEvent>>>,
    max_session_age_secs: u64,
}

impl RealtimeUIBackend {
    pub fn new() -> Self {
        Self::with_rate_limit(50)
    }

    pub fn with_rate_limit(rate_limit_per_sec: usize) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            rate_limit_per_sec,
            event_buffer: Arc::new(DashMap::new()),
            max_session_age_secs: 3600, // 1 hour
        }
    }

    /// Upgrade HTTP connection to WebSocket.
    pub fn upgrade_to_ws(&self, ws: WebSocketUpgrade) -> impl IntoResponse {
        let sessions = Arc::clone(&self.sessions);
        let event_buffer = Arc::clone(&self.event_buffer);
        let rate_limit = self.rate_limit_per_sec;
        let max_age = self.max_session_age_secs;
        ws.on_upgrade(move |socket| {
            Self::handle_ws_connection(socket, sessions, event_buffer, rate_limit, max_age)
        })
    }

    /// Handle a WebSocket connection lifecycle (free function for static lifetime).
    async fn handle_ws_connection(
        socket: WebSocket,
        sessions: Arc<DashMap<String, SessionState>>,
        event_buffer: Arc<DashMap<String, VecDeque<RealtimeEvent>>>,
        rate_limit_per_sec: usize,
        _max_session_age_secs: u64,
    ) {
        let session_id = generate_session_id();

        let session = SessionState::new(session_id.clone(), rate_limit_per_sec);
        sessions.insert(session_id.clone(), session);

        let (mut sender, mut receiver) = socket.split();

        // Outbound: broadcast events to this session
        let sessions_clone = Arc::clone(&sessions);
        let event_buffer_clone = Arc::clone(&event_buffer);
        let session_id_clone = session_id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
            loop {
                interval.tick().await;

                // Check if session still exists
                if !sessions_clone.contains_key(&session_id_clone) {
                    break;
                }

                // Check rate limit
                let allowed = {
                    let mut sess = match sessions_clone.get_mut(&session_id_clone) {
                        Some(s) => s,
                        None => break,
                    };
                    sess.check_rate_limit()
                };

                if !allowed {
                    // Rate limited, skip this tick
                    continue;
                }

                // Drain events for this session
                let events: Vec<RealtimeEvent> = {
                    match event_buffer_clone.get_mut(&session_id_clone) {
                        Some(mut buffer) => {
                            let mut evts = Vec::new();
                            while let Some(event) = buffer.pop_front() {
                                evts.push(event);
                            }
                            evts
                        }
                        None => Vec::new(),
                    }
                };

                for event in events {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    if sender.send(Message::Text(json)).await.is_err() {
                        // Client disconnected
                        sessions_clone.remove(&session_id_clone);
                        return;
                    }
                    if let Some(mut sess) = sessions_clone.get_mut(&session_id_clone) {
                        sess.record_message();
                    }
                }
            }
        });

        // Inbound: receive messages from client (ping/pong, commands)
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(_) | Message::Binary(_) => {
                    // Process client messages if needed
                }
                Message::Ping(_) | Message::Pong(_) => {
                    // Keepalive
                }
                Message::Close(_) => {
                    break;
                }
            }
        }

        // Cleanup on disconnect
        sessions.remove(&session_id);
        event_buffer.remove(&session_id);
    }

    /// Broadcast an event to all active sessions or a specific session.
    pub fn broadcast_event(&self, event: RealtimeEvent) -> WsResult {
        let mut total_sent = 0;
        let mut rate_limited_count = 0;

        for mut session in self.sessions.iter_mut() {
            if !session.value_mut().check_rate_limit() {
                rate_limited_count += 1;
                continue;
            }

            // Add event to session buffer
            let session_id = session.key().clone();
            self.event_buffer
                .entry(session_id)
                .or_default() // CLEANUP: or_insert_with -> or_default
                .push_back(event.clone());

            session.value_mut().record_message();
            total_sent += 1;
        }

        let any_rate_limited = rate_limited_count > 0;
        WsResult::new(
            "broadcast".to_string(),
            total_sent,
            any_rate_limited,
            self.sessions.len(),
        )
    }

    /// Generate a sync state snapshot for a session.
    pub fn sync_state(&self, session_id: String) -> Result<serde_json::Value, RealtimeError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| RealtimeError::SessionNotFound(session_id.clone()))?;

        let state = serde_json::json!({
            "session_id": session_id,
            "connected_at_ms": session.connected_at.elapsed().as_millis(),
            "messages_sent": session.messages_sent,
            "active_sessions": self.sessions.len(),
            "rate_limit_per_sec": session.rate_limit_per_sec,
        });

        Ok(state)
    }

    /// Apply rate limiting check for a session. Returns result with rate_limited flag.
    pub fn rate_limit_session(&self, session_id: String) -> Result<WsResult, RealtimeError> {
        let mut session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| RealtimeError::SessionNotFound(session_id.clone()))?;

        let allowed = session.check_rate_limit();
        Ok(WsResult::new(
            session_id,
            session.messages_sent,
            !allowed,
            self.sessions.len(),
        ))
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired_sessions(&self) -> usize {
        let mut expired = Vec::new();
        for session in self.sessions.iter() {
            if session.is_expired(self.max_session_age_secs) {
                expired.push(session.key().clone());
            }
        }

        for id in &expired {
            self.sessions.remove(id);
            self.event_buffer.remove(id);
        }

        expired.len()
    }

    /// Get the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get stats for all sessions.
    pub fn get_stats(&self) -> RealtimeStats {
        let total_messages: usize = self.sessions.iter().map(|s| s.messages_sent).sum();
        RealtimeStats {
            active_sessions: self.sessions.len(),
            total_messages_sent: total_messages,
            rate_limit_per_sec: self.rate_limit_per_sec,
        }
    }

    /// Reset all state.
    pub fn reset(&self) {
        self.sessions.clear();
        self.event_buffer.clear();
    }
}

impl Default for RealtimeUIBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RealtimeStats {
    pub active_sessions: usize,
    pub total_messages_sent: usize,
    pub rate_limit_per_sec: usize,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn generate_session_id() -> String {
    format!("sess_{}", uuid::Uuid::new_v4().simple())
}

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
    fn test_backend_creation() {
        let backend = RealtimeUIBackend::new();
        assert_eq!(backend.active_session_count(), 0);
    }

    #[test]
    fn test_backend_with_rate_limit() {
        let backend = RealtimeUIBackend::with_rate_limit(100);
        assert_eq!(backend.get_stats().rate_limit_per_sec, 100);
    }

    #[test]
    fn test_realtime_event_creation() {
        let event = RealtimeEvent::new(
            EventType::GovernanceVote,
            serde_json::json!({"vote": "yes"}),
            "node1".into(),
        );
        assert_eq!(event.event_type, EventType::GovernanceVote);
        assert_eq!(event.source_node, "node1");
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::GovernanceVote.to_string(), "governance_vote");
        assert_eq!(EventType::AlignmentDrift.to_string(), "alignment_drift");
        assert_eq!(EventType::FederationSync.to_string(), "federation_sync");
        assert_eq!(EventType::SloBreach.to_string(), "slo_breach");
        assert_eq!(EventType::MarketplaceTrade.to_string(), "marketplace_trade");
    }

    #[test]
    fn test_session_state_creation() {
        let session = SessionState::new("test".into(), 50);
        assert_eq!(session.session_id, "test");
        assert_eq!(session.messages_sent, 0);
    }

    #[test]
    fn test_rate_limit_check() {
        let mut session = SessionState::new("test".into(), 5);
        for _ in 0..5 {
            assert!(session.check_rate_limit());
            session.record_message();
        }
        // 6th message should be rate limited
        assert!(!session.check_rate_limit());
    }

    #[test]
    fn test_record_message() {
        let mut session = SessionState::new("test".into(), 50);
        session.record_message();
        session.record_message();
        assert_eq!(session.messages_sent, 2);
    }

    #[test]
    fn test_broadcast_event() {
        let backend = RealtimeUIBackend::new();
        let event = RealtimeEvent::new(
            EventType::GovernanceVote,
            serde_json::json!({}),
            "node1".into(),
        );
        let result = backend.broadcast_event(event);
        assert_eq!(result.messages_sent, 0); // No sessions yet
        assert!(!result.rate_limited);
    }

    #[test]
    fn test_sync_state_unknown_session() {
        let backend = RealtimeUIBackend::new();
        let result = backend.sync_state("unknown".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_session_unknown() {
        let backend = RealtimeUIBackend::new();
        let result = backend.rate_limit_session("unknown".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let backend = RealtimeUIBackend::new();
        let cleaned = backend.cleanup_expired_sessions();
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn test_get_stats() {
        let backend = RealtimeUIBackend::new();
        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_messages_sent, 0);
    }

    #[test]
    fn test_reset() {
        let backend = RealtimeUIBackend::new();
        backend.reset();
        assert_eq!(backend.active_session_count(), 0);
    }

    #[test]
    fn test_ws_result() {
        let result = WsResult::new("s1".into(), 10, false, 5);
        assert_eq!(result.session_id, "s1");
        assert_eq!(result.messages_sent, 10);
        assert!(!result.rate_limited);
        assert_eq!(result.active_sessions, 5);
    }

    #[test]
    fn test_session_expiry() {
        let session = SessionState::new("test".into(), 50);
        assert!(!session.is_expired(3600));
    }

    #[test]
    fn test_default() {
        let backend = RealtimeUIBackend::default();
        assert_eq!(backend.active_session_count(), 0);
    }

    #[test]
    fn test_realtime_error_session_not_found() {
        let err = RealtimeError::SessionNotFound("x".into());
        assert!(err.to_string().contains("x"));
    }

    #[test]
    fn test_realtime_error_rate_limit() {
        let err = RealtimeError::RateLimitExceeded {
            current: 60,
            max: 50,
        };
        assert!(err.to_string().contains("60"));
    }
}
