//! Rosetta API — axum HTTP endpoints for semantic graph queries.
//!
//! Exposes `GET /api/feature/{id}` and `GET /api/token/{word}` endpoints
//! for querying the semantic graph from web clients.
//!
//! **Sprint9 RLHF Bridge** (feature `v2.1-rlhf-bridge`):
//! - `POST /api/feedback` — Receives human corrections for semantic alignment
//! - `GET /api/feedback/export` — Exports feedback dataset for ethical re-training
//! - Rate limiting per `node_id` to prevent abuse

#[cfg(feature = "v2.1-rosetta-api")]
use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
#[cfg(feature = "v2.1-rosetta-api")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v2.1-rosetta-api")]
use std::collections::HashMap;
#[cfg(feature = "v2.1-rosetta-api")]
use std::net::SocketAddr;
#[cfg(feature = "v2.1-rosetta-api")]
use std::sync::Arc;
#[cfg(feature = "v2.1-rosetta-api")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(feature = "v2.1-rosetta-api")]
#[cfg(feature = "v2.1-rosetta-api")]
use tower_http::trace::TraceLayer;
#[cfg(feature = "v2.1-rosetta-api")]
use tracing::info;

#[cfg(feature = "v2.1-rosetta-api")]
use super::graph::SemanticGraph;

// ─── RLHF Feedback Bridge (Sprint9) ───────────────────────────────────────────

#[cfg(feature = "v2.1-rlhf-bridge")]
#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    /// SAE feature ID being corrected
    pub feature_id: String,
    /// Natural language token associated with the feature
    pub token: String,
    /// Human-explained reason for the correction
    pub reason: String,
    /// Submitting node ID (for rate limiting)
    pub node_id: String,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
}

#[cfg(feature = "v2.1-rlhf-bridge")]
#[derive(Debug, Serialize, Clone)]
pub struct FeedbackEntry {
    pub feature_id: String,
    pub token: String,
    pub reason: String,
    pub node_id: String,
    pub timestamp: u64,
    pub accepted: bool,
}

#[cfg(feature = "v2.1-rlhf-bridge")]
#[derive(Debug, Serialize)]
pub struct FeedbackResponse {
    pub accepted: bool,
    pub message: String,
}

#[cfg(feature = "v2.1-rlhf-bridge")]
#[derive(Debug, Serialize)]
pub struct FeedbackExport {
    pub total_entries: usize,
    pub entries: Vec<FeedbackEntry>,
}

/// In-memory feedback store with per-node rate limiting.
///
/// Uses a bounded Vec for entries and a HashMap tracking
/// submission counts per node_id with time-window decay.
#[cfg(feature = "v2.1-rlhf-bridge")]
pub struct FeedbackStore {
    entries: Arc<parking_lot::RwLock<Vec<FeedbackEntry>>>,
    rate_limits: Arc<parking_lot::RwLock<HashMap<String, (u64, usize)>>>,
    max_submissions: usize,
    window_duration: Duration,
}

#[cfg(feature = "v2.1-rlhf-bridge")]
impl FeedbackStore {
    pub fn new(max_submissions: usize, window_secs: u64) -> Self {
        Self {
            entries: Arc::new(parking_lot::RwLock::new(Vec::new())),
            rate_limits: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            max_submissions,
            window_duration: Duration::from_secs(window_secs),
        }
    }

    /// Check if a node_id has exceeded the rate limit window.
    /// Returns true if the submission is allowed.
    fn is_allowed(&self, node_id: &str) -> bool {
        let mut limits = self.rate_limits.write();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let window_ms = self.window_duration.as_millis() as u64;

        let entry = limits.entry(node_id.to_string()).or_insert((now_ms, 0));

        // Reset window if expired
        if now_ms - entry.0 > window_ms {
            entry.0 = now_ms;
            entry.1 = 0;
        }

        if entry.1 >= self.max_submissions {
            false
        } else {
            entry.1 += 1;
            true
        }
    }

    /// Insert a feedback entry. Returns false if rate limited.
    fn submit(&self, req: &FeedbackRequest) -> bool {
        if !self.is_allowed(&req.node_id) {
            return false;
        }
        let entry = FeedbackEntry {
            feature_id: req.feature_id.clone(),
            token: req.token.clone(),
            reason: req.reason.clone(),
            node_id: req.node_id.clone(),
            timestamp: req.timestamp,
            accepted: true,
        };
        self.entries.write().push(entry);
        true
    }

    /// Export all feedback entries as a dataset.
    fn export(&self) -> FeedbackExport {
        let entries = self.entries.read().clone();
        FeedbackExport {
            total_entries: entries.len(),
            entries,
        }
    }
}

#[cfg(feature = "v2.1-rlhf-bridge")]
impl Default for FeedbackStore {
    fn default() -> Self {
        // Default: 10 submissions per 300-second window per node
        Self::new(10, 300)
    }
}

// ─── Existing Types ──────────────────────────────────────────────────────────

#[cfg(feature = "v2.1-rosetta-api")]
#[derive(Debug, Serialize)]
pub struct FeatureResponse {
    pub feature_id: String,
    pub top_tokens: Vec<TokenEntry>,
    pub node_count: usize,
    pub edge_count: usize,
}

#[cfg(feature = "v2.1-rosetta-api")]
#[derive(Debug, Serialize)]
pub struct TokenEntry {
    pub token: String,
    pub weight: f64,
}

#[cfg(feature = "v2.1-rosetta-api")]
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub top_features: Vec<FeatureEntry>,
    pub node_count: usize,
    pub edge_count: usize,
}

#[cfg(feature = "v2.1-rosetta-api")]
#[derive(Debug, Serialize)]
pub struct FeatureEntry {
    pub feature_id: String,
    pub weight: f64,
}

#[cfg(feature = "v2.1-rosetta-api")]
#[derive(Debug, Serialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
}

/// Shared application state for the Rosetta API server.
#[cfg(feature = "v2.1-rosetta-api")]
#[derive(Clone)]
pub struct AppState {
    pub graph: Arc<SemanticGraph>,
    #[cfg(feature = "v2.1-rlhf-bridge")]
    pub feedback: Arc<FeedbackStore>,
}

/// Start the Rosetta API server on the given port (with RLHF feedback).
#[cfg(all(feature = "v2.1-rosetta-api", feature = "v2.1-rlhf-bridge"))]
pub async fn run_server(graph: Arc<SemanticGraph>, port: u16) {
    let state = AppState {
        graph,
        feedback: Arc::new(FeedbackStore::default()),
    };

    let app = Router::new()
        .route("/api/feature/:id", get(feature_handler))
        .route("/api/token/:word", get(token_handler))
        .route("/api/atlas/stats", get(stats_handler))
        .route("/api/feedback", post(feedback_handler))
        .route("/api/feedback/export", get(feedback_export_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    info!(address = %addr, "Rosetta API listening");
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("Server error: {}", e);
    }
}

/// Start the Rosetta API server on the given port (without RLHF feedback).
#[cfg(all(feature = "v2.1-rosetta-api", not(feature = "v2.1-rlhf-bridge")))]
pub async fn run_server(graph: Arc<SemanticGraph>, port: u16) {
    let app = Router::new()
        .route("/api/feature/:id", get(feature_handler))
        .route("/api/token/:word", get(token_handler))
        .route("/api/atlas/stats", get(stats_handler))
        .with_state(graph)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    info!(address = %addr, "Rosetta API listening");
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("Server error: {}", e);
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.unwrap_or(());
    info!("Shutdown signal received");
}

#[cfg(feature = "v2.1-rosetta-api")]
async fn feature_handler(
    Path(feature_id): Path<String>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> (StatusCode, Json<FeatureResponse>) {
    let top = state.graph.get_top_tokens_for_feature(&feature_id, 10);
    if top.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(FeatureResponse {
                feature_id,
                top_tokens: Vec::new(),
                node_count: state.graph.node_count(),
                edge_count: state.graph.edge_count(),
            }),
        );
    }
    (
        StatusCode::OK,
        Json(FeatureResponse {
            feature_id,
            top_tokens: top
                .into_iter()
                .map(|(token, weight)| TokenEntry { token, weight })
                .collect(),
            node_count: state.graph.node_count(),
            edge_count: state.graph.edge_count(),
        }),
    )
}

#[cfg(feature = "v2.1-rosetta-api")]
async fn token_handler(
    Path(word): Path<String>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> (StatusCode, Json<TokenResponse>) {
    let top = state.graph.get_top_features_for_token(&word, 10);
    if top.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(TokenResponse {
                token: word,
                top_features: Vec::new(),
                node_count: state.graph.node_count(),
                edge_count: state.graph.edge_count(),
            }),
        );
    }
    (
        StatusCode::OK,
        Json(TokenResponse {
            token: word,
            top_features: top
                .into_iter()
                .map(|(feature_id, weight)| FeatureEntry { feature_id, weight })
                .collect(),
            node_count: state.graph.node_count(),
            edge_count: state.graph.edge_count(),
        }),
    )
}

#[cfg(feature = "v2.1-rosetta-api")]
async fn stats_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<GraphStats> {
    Json(GraphStats {
        node_count: state.graph.node_count(),
        edge_count: state.graph.edge_count(),
    })
}

// ─── RLHF Feedback Handlers (Sprint9) ────────────────────────────────────────

#[cfg(feature = "v2.1-rlhf-bridge")]
async fn feedback_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<FeedbackRequest>,
) -> (StatusCode, Json<FeedbackResponse>) {
    // Validate required fields
    if req.feature_id.is_empty() || req.token.is_empty() || req.reason.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(FeedbackResponse {
                accepted: false,
                message: "Missing required fields: feature_id, token, reason".to_string(),
            }),
        );
    }

    if state.feedback.submit(&req) {
        (
            StatusCode::OK,
            Json(FeedbackResponse {
                accepted: true,
                message: "Feedback recorded successfully".to_string(),
            }),
        )
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(FeedbackResponse {
                accepted: false,
                message: "Rate limit exceeded. Try again later.".to_string(),
            }),
        )
    }
}

#[cfg(feature = "v2.1-rlhf-bridge")]
async fn feedback_export_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<FeedbackExport> {
    Json(state.feedback.export())
}

#[cfg(all(test, feature = "v2.1-rosetta-api"))]
mod tests {
    use super::*;
    use crate::atlas::graph::SemanticGraph;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_stats_endpoint() {
        let graph = Arc::new(SemanticGraph::new());
        graph.insert_activation("neural", "feat-1", 0.9);

        // Simple check: graph has expected data
        assert_eq!(graph.node_count(), 2); // "neural" + "feat-1"
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_feature_response_serialize() {
        let resp = FeatureResponse {
            feature_id: "feat-1".to_string(),
            top_tokens: vec![TokenEntry {
                token: "neural".to_string(),
                weight: 0.9,
            }],
            node_count: 2,
            edge_count: 1,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("feat-1"));
        assert!(json.contains("neural"));
    }

    #[test]
    fn test_token_response_serialize() {
        let resp = TokenResponse {
            token: "neural".to_string(),
            top_features: vec![FeatureEntry {
                feature_id: "feat-1".to_string(),
                weight: 0.9,
            }],
            node_count: 2,
            edge_count: 1,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("neural"));
        assert!(json.contains("feat-1"));
    }

    // ─── RLHF Bridge Tests (Sprint9) ──────────────────────────────────────

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_store_creation() {
        let store = FeedbackStore::new(5, 60);
        assert_eq!(store.export().total_entries, 0);
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_submit_success() {
        let store = FeedbackStore::new(10, 300);
        let req = FeedbackRequest {
            feature_id: "feat-1".to_string(),
            token: "neural".to_string(),
            reason: "Incorrect mapping".to_string(),
            node_id: "node-a".to_string(),
            timestamp: 1000,
        };
        assert!(store.submit(&req));
        assert_eq!(store.export().total_entries, 1);
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_rate_limiting() {
        let store = FeedbackStore::new(2, 300); // Max 2 per 300s
        let req1 = FeedbackRequest {
            feature_id: "feat-1".to_string(),
            token: "neural".to_string(),
            reason: "Reason 1".to_string(),
            node_id: "node-x".to_string(),
            timestamp: 1000,
        };
        let req2 = FeedbackRequest {
            feature_id: "feat-2".to_string(),
            token: "cortex".to_string(),
            reason: "Reason 2".to_string(),
            node_id: "node-x".to_string(),
            timestamp: 1001,
        };
        let req3 = FeedbackRequest {
            feature_id: "feat-3".to_string(),
            token: "axon".to_string(),
            reason: "Reason 3".to_string(),
            node_id: "node-x".to_string(),
            timestamp: 1002,
        };
        assert!(store.submit(&req1));
        assert!(store.submit(&req2));
        assert!(!store.submit(&req3)); // Rate limited
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_different_nodes() {
        let store = FeedbackStore::new(1, 300);
        let req_a = FeedbackRequest {
            feature_id: "feat-1".to_string(),
            token: "neural".to_string(),
            reason: "From A".to_string(),
            node_id: "node-a".to_string(),
            timestamp: 1000,
        };
        let req_b = FeedbackRequest {
            feature_id: "feat-1".to_string(),
            token: "neural".to_string(),
            reason: "From B".to_string(),
            node_id: "node-b".to_string(),
            timestamp: 1001,
        };
        assert!(store.submit(&req_a));
        assert!(store.submit(&req_b)); // Different node, allowed
        assert_eq!(store.export().total_entries, 2);
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_export() {
        let store = FeedbackStore::new(10, 300);
        let req = FeedbackRequest {
            feature_id: "feat-1".to_string(),
            token: "neural".to_string(),
            reason: "Test".to_string(),
            node_id: "node-a".to_string(),
            timestamp: 5000,
        };
        store.submit(&req);
        let export = store.export();
        assert_eq!(export.total_entries, 1);
        assert_eq!(export.entries[0].feature_id, "feat-1");
        assert!(export.entries[0].accepted);
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_response_serialize() {
        let resp = FeedbackResponse {
            accepted: true,
            message: "OK".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("true"));
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_export_serialize() {
        let export = FeedbackExport {
            total_entries: 0,
            entries: vec![],
        };
        let json = serde_json::to_string(&export).unwrap();
        assert!(json.contains("total_entries"));
    }

    #[cfg(feature = "v2.1-rlhf-bridge")]
    #[test]
    fn test_feedback_store_default() {
        let store = FeedbackStore::default();
        assert_eq!(store.export().total_entries, 0);
    }
}
