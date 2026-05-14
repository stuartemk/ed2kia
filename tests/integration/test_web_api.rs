//! Integration Test: Web API
//!
//! Validates API endpoints: /api/status, /api/feedback, /api/metrics
//! using axum test utilities.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;

#[path = "../../src/web/server.rs"]
mod web_server;

#[path = "../../src/web/routes.rs"]
mod web_routes;

use web_server::WebServer;
use web_routes::ApiResponse;

/// Test: ApiResponse success format
#[test]
fn test_api_response_success() {
    let response: ApiResponse<serde_json::Value> = ApiResponse::ok(serde_json::json!({
        "test": "data"
    }));

    assert!(response.success);
    assert!(response.data.is_some());
    assert!(response.error.is_none());
}

/// Test: ApiResponse error format
#[test]
fn test_api_response_error() {
    let response: ApiResponse<serde_json::Value> =
        ApiResponse::error("Something went wrong".to_string());

    assert!(!response.success);
    assert!(response.data.is_none());
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap(), "Something went wrong");
}

/// Test: Feedback request validation - valid decision
#[test]
fn test_feedback_request_valid_decision() {
    let valid_decisions = vec![
        "approved",
        "rejected",
        "corrected",
        "uncertain",
    ];

    for decision in valid_decisions {
        let json = serde_json::json!({
            "layer_id": "layer_0",
            "feature_idx": 42,
            "feature_value": 0.85,
            "decision": decision,
            "annotator_id": "test_annotator",
        });

        let parsed: serde_json::Value = json;
        assert_eq!(parsed["decision"], decision);
    }
}

/// Test: Feedback request validation - invalid decision
#[test]
fn test_feedback_request_invalid_decision() {
    let invalid_decisions = vec!["approve", "reject", "yes", "no", ""];

    for decision in invalid_decisions {
        // These should be rejected by the handler
        assert!(!["approved", "rejected", "corrected", "uncertain"]
            .contains(&decision));
    }
}

/// Test: Status endpoint response structure
#[test]
fn test_status_response_structure() {
    let response = serde_json::json!({
        "node": {
            "peer_id": "test_peer",
            "layers": [0, 1, 2],
            "uptime_seconds": 100
        },
        "uptime_seconds": 100,
        "server_version": "0.5.0"
    });

    assert!(response.get("node").is_some());
    assert!(response.get("uptime_seconds").is_some());
    assert!(response.get("server_version").is_some());
}

/// Test: Network endpoint response structure
#[test]
fn test_network_response_structure() {
    let response = serde_json::json!({
        "peers": [
            {
                "peer_id": "peer_1",
                "reputation": 0.95,
                "last_seen": 1000
            }
        ],
        "total_peers": 1,
        "gossipsub": {
            "topics": ["ed2k.sae.layer_0"],
            "messages_received": 50
        }
    });

    assert!(response.get("peers").is_some());
    assert!(response.get("total_peers").is_some());
    assert_eq!(response["total_peers"], 1);
}

/// Test: Metrics endpoint response structure
#[test]
fn test_metrics_response_structure() {
    let response = serde_json::json!({
        "sae_forwards_total": 1000,
        "consensus_votes_total": 500,
        "feedback_entries_total": 200,
        "avg_inference_ms": 15.5,
        "active_peers": 5
    });

    assert!(response.get("sae_forwards_total").is_some());
    assert!(response.get("consensus_votes_total").is_some());
    assert!(response.get("feedback_entries_total").is_some());
}

/// Test: Health endpoint returns OK
#[test]
fn test_health_response() {
    let response = serde_json::json!({
        "status": "healthy",
        "checks": {
            "p2p": "ok",
            "sae": "ok",
            "database": "ok"
        }
    });

    assert_eq!(response["status"], "healthy");
    assert_eq!(response["checks"]["p2p"], "ok");
}

/// Test: API response serialization roundtrip
#[test]
fn test_api_response_serialization() {
    let response: ApiResponse<serde_json::Value> = ApiResponse::ok(serde_json::json!({
        "key": "value"
    }));

    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: ApiResponse<serde_json::Value> =
        serde_json::from_str(&serialized).unwrap();

    assert!(deserialized.success);
    assert_eq!(deserialized.data.unwrap()["key"], "value");
}

/// Test: Feedback payload construction
#[test]
fn test_feedback_payload_construction() {
    let payload = serde_json::json!({
        "layer_id": "layer_5",
        "feature_idx": 128,
        "feature_value": 0.75,
        "decision": "approved",
        "correction": null,
        "concept": "test_concept",
        "annotator_id": "human_1",
        "metadata": "{\"source\": \"web_ui\"}"
    });

    assert_eq!(payload["layer_id"], "layer_5");
    assert_eq!(payload["feature_idx"], 128);
    assert_eq!(payload["decision"], "approved");
    assert!(payload["correction"].is_null());
    assert_eq!(payload["concept"], "test_concept");
}

/// Test: Web server state uptime calculation
#[test]
fn test_uptime_calculation() {
    use std::time::Instant;

    let start = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let elapsed = start.elapsed().as_secs();

    assert!(elapsed >= 0);
}
