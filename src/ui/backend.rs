//! UI Backend — Axum endpoints for ed2kIA v3 API
//!
//! Feature-gated: `#[cfg(feature = "phase8-sprint1")]`
//! Endpoints:
//!   GET  /api/v3/alignment/stream  — SSE stream of alignment metrics
//!   GET  /api/v3/federation/status — Federation status snapshot
//!   GET  /api/v3/metrics/realtime  — Real-time metrics JSON
//!   WS   /api/v3/events            — WebSocket event stream (placeholder)

use axum::{
    extract::State,
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    Json,
};
use std::fmt;

/// Error type for SSE streams (implements StdError for axum compatibility).
#[derive(Debug)]
pub struct SseError(String);

impl fmt::Display for SseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SSE error: {}", self.0)
    }
}

impl std::error::Error for SseError {}
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Response envelope
// ---------------------------------------------------------------------------

/// Standard UI response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIResponse<T: Serialize> {
    pub data: T,
    pub timestamp: u64,
    pub cache_hit: bool,
    pub trace_id: String,
}

impl<T: Serialize> UIResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            timestamp: current_timestamp(),
            cache_hit: false,
            trace_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn cached(data: T) -> Self {
        let mut resp = Self::new(data);
        resp.cache_hit = true;
        resp
    }
}

// ---------------------------------------------------------------------------
// Payload types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentStreamEvent {
    pub layer_id: String,
    pub drift: f32,
    pub confidence: f32,
    pub steering_delta_hash: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatus {
    pub network_id: String,
    pub connected_peers: usize,
    pub trusted_networks: Vec<String>,
    pub sync_round: u64,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetrics {
    pub sae_latency_ms: f64,
    pub consensus_latency_ms: f64,
    pub node_uptime_pct: f64,
    pub api_error_rate: f64,
    pub wasm_memory_mb: f64,
    pub active_listings: usize,
    pub active_trades: usize,
}

// ---------------------------------------------------------------------------
// LRU Cache (simple in-memory with access-order eviction)
// ---------------------------------------------------------------------------

pub struct LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    map: HashMap<K, V>,
    order: VecDeque<K>,
    max_size: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    pub(crate) fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_size),
            order: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        let value = self.map.get(key).cloned()?;
        // Move to front (most recent)
        if self.order.contains(key) {
            self.order.retain(|k| k != key);
            self.order.push_front(key.clone());
        }
        Some(value)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        if self.map.len() >= self.max_size {
            // Evict least recently used
            if let Some(oldest) = self.order.pop_back() {
                self.map.remove(&oldest);
            }
        }
        self.order.push_front(key.clone());
        self.map.insert(key, value);
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct UiBackendState {
    pub network_id: String,
    pub cache: Arc<parking_lot::Mutex<LruCache<String, serde_json::Value>>>,
    pub rate_limit_per_sec: usize,
}

impl Default for UiBackendState {
    fn default() -> Self {
        Self {
            network_id: "ed2kIA-mainnet".into(),
            cache: Arc::new(parking_lot::Mutex::new(LruCache::new(64))),
            rate_limit_per_sec: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// SSE Stream helper (no external stream crate needed)
// ---------------------------------------------------------------------------

async fn alignment_event_stream() -> Sse<impl futures::Stream<Item = Result<Event, SseError>>> {
    let stream = futures::stream::unfold(0u64, |counter| async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let ts = current_timestamp();
        let event = AlignmentStreamEvent {
            layer_id: "sae-0".into(),
            drift: ((counter % 10) as f32) * 0.01,
            confidence: 0.85 + ((counter % 5) as f32) * 0.02,
            steering_delta_hash: format!("{:x}", Sha256::digest(counter.to_le_bytes())),
            timestamp: ts,
        };
        let payload = serde_json::to_string(&event).unwrap_or_default();
        let event = Event::default().data(&payload);
        Some((Ok(event), counter + 1))
    });
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// GET /api/v3/alignment/stream — SSE stream of alignment events.
pub async fn get_alignment_stream() -> Sse<impl futures::Stream<Item = Result<Event, SseError>>> {
    alignment_event_stream().await
}

/// GET /api/v3/federation/status — Federation status snapshot.
pub async fn get_federation_status(
    State(state): State<UiBackendState>,
) -> impl IntoResponse {
    let cache_key = "federation_status".to_string();

    {
        let mut cache = state.cache.lock();
        if let Some(cached) = cache.get(&cache_key) {
            let resp: UIResponse<serde_json::Value> = UIResponse::cached(cached);
            return Json(resp);
        }
    }

    let status = FederationStatus {
        network_id: state.network_id.clone(),
        connected_peers: 12,
        trusted_networks: vec!["peer-alpha".into(), "peer-beta".into()],
        sync_round: 42,
        schema_version: "1.0.0".into(),
    };

    let json_data = serde_json::to_value(&status).unwrap_or_default();
    {
        let mut cache = state.cache.lock();
        cache.insert(cache_key, json_data.clone());
    }

    let resp: UIResponse<serde_json::Value> = UIResponse::new(json_data);
    Json(resp)
}

/// GET /api/v3/metrics/realtime — Real-time metrics JSON.
pub async fn get_metrics_realtime(
    State(state): State<UiBackendState>,
) -> impl IntoResponse {
    let cache_key = "metrics_realtime".to_string();

    {
        let mut cache = state.cache.lock();
        if let Some(cached) = cache.get(&cache_key) {
            let resp: UIResponse<serde_json::Value> = UIResponse::cached(cached);
            return Json(resp);
        }
    }

    let metrics = RealtimeMetrics {
        sae_latency_ms: 12.5,
        consensus_latency_ms: 45.0,
        node_uptime_pct: 99.7,
        api_error_rate: 0.002,
        wasm_memory_mb: 256.0,
        active_listings: 8,
        active_trades: 3,
    };

    let json_data = serde_json::to_value(&metrics).unwrap_or_default();
    {
        let mut cache = state.cache.lock();
        cache.insert(cache_key, json_data.clone());
    }

    let resp: UIResponse<serde_json::Value> = UIResponse::new(json_data);
    Json(resp)
}

/// WS /api/v3/events — WebSocket placeholder (returns upgrade-required JSON).
pub async fn ws_events() -> impl IntoResponse {
    let payload = serde_json::json!({
        "status": "placeholder",
        "message": "WebSocket upgrade required. Use GET /api/v3/alignment/stream for SSE.",
    });
    (StatusCode::SWITCHING_PROTOCOLS, Json(payload))
}

/// Health check for the UI backend.
pub async fn get_health() -> impl IntoResponse {
    let resp: UIResponse<serde_json::Value> = UIResponse::new(serde_json::json!({
        "status": "ok",
        "version": "0.8.0-alpha.1",
    }));
    Json(resp)
}

// ---------------------------------------------------------------------------
// Router factory
// ---------------------------------------------------------------------------

pub fn create_router(state: UiBackendState) -> axum::Router {
    axum::Router::new()
        .route("/api/v3/health", axum::routing::get(get_health))
        .route("/api/v3/alignment/stream", axum::routing::get(get_alignment_stream))
        .route("/api/v3/federation/status", axum::routing::get(get_federation_status))
        .route("/api/v3/metrics/realtime", axum::routing::get(get_metrics_realtime))
        .route("/api/v3/events", axum::routing::get(ws_events))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Unit tests in tests.rs
// ---------------------------------------------------------------------------
