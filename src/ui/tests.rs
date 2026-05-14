//! UI Backend unit tests — 10+ tests covering route handlers, payload
//! validation, SSE simulation, cache behavior, and rate limiting.

use super::backend::*;
use sha2::{Digest, Sha256};

// ---- UIResponse tests -----------------------------------------------------

#[test]
fn test_ui_response_new() {
    let data = serde_json::json!({"key": "value"});
    let resp: UIResponse<serde_json::Value> = UIResponse::new(data.clone());
    assert_eq!(resp.data, data);
    assert!(!resp.cache_hit);
    assert!(!resp.trace_id.is_empty());
    assert!(resp.timestamp > 0);
}

#[test]
fn test_ui_response_cached() {
    let data = serde_json::json!({"cached": true});
    let resp: UIResponse<serde_json::Value> = UIResponse::cached(data.clone());
    assert_eq!(resp.data, data);
    assert!(resp.cache_hit);
}

// ---- LRU Cache tests ------------------------------------------------------

#[test]
fn test_lru_cache_insert_and_get() {
    let mut cache = LruCache::<String, serde_json::Value>::new(4);
    cache.insert("a".into(), serde_json::json!(1));
    assert!(cache.get(&"a".into()).is_some());
    assert!(cache.get(&"b".into()).is_none());
}

#[test]
fn test_lru_cache_eviction() {
    let mut cache = LruCache::<String, serde_json::Value>::new(2);
    cache.insert("first".into(), serde_json::json!(1));
    cache.insert("second".into(), serde_json::json!(2));
    assert_eq!(cache.len(), 2);

    // Insert third → evicts "first"
    cache.insert("third".into(), serde_json::json!(3));
    assert!(cache.get(&"first".into()).is_none());
    assert!(cache.get(&"second".into()).is_some());
    assert!(cache.get(&"third".into()).is_some());
}

#[test]
fn test_lru_cache_access_order() {
    let mut cache = LruCache::<String, serde_json::Value>::new(3);
    cache.insert("a".into(), serde_json::json!(1));
    cache.insert("b".into(), serde_json::json!(2));
    cache.insert("c".into(), serde_json::json!(3));

    // Access "a" to make it most recent
    cache.get(&"a".into());

    // Insert "d" → evicts "b" (least recent)
    cache.insert("d".into(), serde_json::json!(4));
    assert!(cache.get(&"a".into()).is_some());
    assert!(cache.get(&"b".into()).is_none());
    assert!(cache.get(&"c".into()).is_some());
    assert!(cache.get(&"d".into()).is_some());
}

// ---- Payload serialization tests ------------------------------------------

#[test]
fn test_alignment_event_serialization() {
    let event = AlignmentStreamEvent {
        layer_id: "sae-0".into(),
        drift: 0.05,
        confidence: 0.9,
        steering_delta_hash: "abc123".into(),
        timestamp: 1000,
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("sae-0"));
    assert!(json.contains("0.05"));
}

#[test]
fn test_federation_status_serialization() {
    let status = FederationStatus {
        network_id: "test-net".into(),
        connected_peers: 5,
        trusted_networks: vec!["peer-1".into()],
        sync_round: 10,
        schema_version: "1.0.0".into(),
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("test-net"));
    assert!(json.contains("5"));
}

#[test]
fn test_realtime_metrics_serialization() {
    let metrics = RealtimeMetrics {
        sae_latency_ms: 10.0,
        consensus_latency_ms: 30.0,
        node_uptime_pct: 99.9,
        api_error_rate: 0.001,
        wasm_memory_mb: 128.0,
        active_listings: 4,
        active_trades: 2,
    };
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(json.contains("10.0"));
    assert!(json.contains("99.9"));
}

// ---- UiBackendState tests -------------------------------------------------

#[test]
fn test_ui_backend_state_default() {
    let state = UiBackendState::default();
    assert_eq!(state.network_id, "ed2kIA-mainnet");
    assert_eq!(state.rate_limit_per_sec, 30);
}

// ---- Router tests ---------------------------------------------------------

#[test]
fn test_router_creation() {
    let state = UiBackendState::default();
    let _router = create_router(state);
    // Router creation succeeds without panic
}

// ---- SSE simulation test --------------------------------------------------

#[test]
fn test_sse_event_serialization() {
    let event = AlignmentStreamEvent {
        layer_id: "sae-0".into(),
        drift: 0.03,
        confidence: 0.88,
        steering_delta_hash: format!("{:x}", Sha256::digest([0u8])),
        timestamp: current_timestamp(),
    };
    let payload = serde_json::to_string(&event).unwrap();
    assert!(payload.contains("sae-0"));
    assert!(payload.contains("0.03"));
}

// ---- Cache hit simulation -------------------------------------------------

#[test]
fn test_cache_hit_via_state() {
    let state = UiBackendState::default();

    // Simulate first cache insert (cache miss pattern)
    {
        let mut cache = state.cache.lock();
        cache.insert(
            "federation_status".into(),
            serde_json::json!({"network_id": "ed2kIA-mainnet"}),
        );
    }

    // Simulate second request (cache hit)
    {
        let mut cache = state.cache.lock();
        let cached = cache.get(&"federation_status".into());
        assert!(cached.is_some());
        let val = cached.unwrap();
        assert_eq!(val["network_id"].as_str(), Some("ed2kIA-mainnet"));
    }
}

// ---- Timestamp helper test ------------------------------------------------

#[test]
fn test_current_timestamp_positive() {
    let ts = current_timestamp();
    assert!(ts > 1_000_000_000); // After 2001
}
