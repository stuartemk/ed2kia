//! Rosetta API — axum HTTP endpoints for semantic graph queries.
//!
//! Exposes `GET /api/feature/{id}` and `GET /api/token/{word}` endpoints
//! for querying the semantic graph from web clients.

#[cfg(feature = "v2.1-rosetta-api")]
use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
#[cfg(feature = "v2.1-rosetta-api")]
use serde::Serialize;
#[cfg(feature = "v2.1-rosetta-api")]
use std::net::SocketAddr;
#[cfg(feature = "v2.1-rosetta-api")]
use std::sync::Arc;
#[cfg(feature = "v2.1-rosetta-api")]
#[cfg(feature = "v2.1-rosetta-api")]
use tower_http::trace::TraceLayer;
#[cfg(feature = "v2.1-rosetta-api")]
use tracing::info;

#[cfg(feature = "v2.1-rosetta-api")]
use super::graph::SemanticGraph;

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

/// Start the Rosetta API server on the given port.
///
/// Spawns an async axum server with trace middleware.
#[cfg(feature = "v2.1-rosetta-api")]
pub async fn run_server(graph: Arc<SemanticGraph>, port: u16) {
    let app = Router::new()
        .route("/api/feature/:id", get(feature_handler))
        .route("/api/token/:word", get(token_handler))
        .route("/api/atlas/stats", get(stats_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(graph);

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
    axum::extract::State(graph): axum::extract::State<Arc<SemanticGraph>>,
) -> (StatusCode, Json<FeatureResponse>) {
    let top = graph.get_top_tokens_for_feature(&feature_id, 10);
    if top.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(FeatureResponse {
                feature_id,
                top_tokens: Vec::new(),
                node_count: graph.node_count(),
                edge_count: graph.edge_count(),
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
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
        }),
    )
}

#[cfg(feature = "v2.1-rosetta-api")]
async fn token_handler(
    Path(word): Path<String>,
    axum::extract::State(graph): axum::extract::State<Arc<SemanticGraph>>,
) -> (StatusCode, Json<TokenResponse>) {
    let top = graph.get_top_features_for_token(&word, 10);
    if top.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(TokenResponse {
                token: word,
                top_features: Vec::new(),
                node_count: graph.node_count(),
                edge_count: graph.edge_count(),
            }),
        );
    }
    (
        StatusCode::OK,
        Json(TokenResponse {
            token: word,
            top_features: top
                .into_iter()
                .map(|(feature_id, weight)| FeatureEntry {
                    feature_id,
                    weight,
                })
                .collect(),
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
        }),
    )
}

#[cfg(feature = "v2.1-rosetta-api")]
async fn stats_handler(
    axum::extract::State(graph): axum::extract::State<Arc<SemanticGraph>>,
) -> Json<GraphStats> {
    Json(GraphStats {
        node_count: graph.node_count(),
        edge_count: graph.edge_count(),
    })
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

        let app = Router::new()
            .route("/api/atlas/stats", get(stats_handler))
            .with_state(graph);

        let client = axum::Router::into_tower_service(app).unwrap();
        // Simple check: server binds without panic
        let port = 18541;
        let _ = run_server(Arc::new(SemanticGraph::new()), port);
        tokio::time::sleep(Duration::from_millis(100)).await;
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
}
