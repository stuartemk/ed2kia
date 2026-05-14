//! Web Routes - Handlers para /api/status, /api/feedback, /api/network, /api/metrics
//!
//! Implementa los handlers HTTP para las rutas de la API REST del dashboard.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::server::WebServerState;

/// Request para enviar feedback
#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    pub layer_id: String,
    pub feature_idx: u32,
    pub feature_value: f64,
    pub decision: String,
    pub correction: Option<String>,
    pub concept: Option<String>,
    pub annotator_id: String,
    pub metadata: Option<String>,
}

/// Response genérico de la API
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// GET /api/status
/// Retorna estado del nodo, capas cargadas, uptime, recursos.
pub async fn get_status(State(state): State<WebServerState>) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let node_status = (state.node_status_fn)();
    let uptime = state.uptime_seconds();

    let response = serde_json::json!({
        "node": node_status,
        "uptime_seconds": uptime,
        "server_version": env!("CARGO_PKG_VERSION"),
    });

    (StatusCode::OK, Json(ApiResponse::ok(response)))
}

/// GET /api/network
/// Retorna pares activos, reputación, métricas gossipsub.
pub async fn get_network(State(state): State<WebServerState>) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let network_info = (state.network_info_fn)();

    (StatusCode::OK, Json(ApiResponse::ok(network_info)))
}

/// GET /api/feedback
/// Retorna estadísticas y entradas recientes de feedback.
pub async fn get_feedback(State(state): State<WebServerState>) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let feedback_data = (state.feedback_fn)();

    (StatusCode::OK, Json(ApiResponse::ok(feedback_data)))
}

/// POST /api/feedback
/// Recibe JSON de feedback humano, valida y almacena en redb.
pub async fn handle_feedback(
    State(state): State<WebServerState>,
    Json(request): Json<FeedbackRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // Validar decisión
    if !["approved", "rejected", "corrected", "uncertain"]
        .iter()
        .any(|d| *d == request.decision)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid decision. Must be: approved, rejected, corrected, uncertain".to_string())),
        );
    }

    let payload = serde_json::json!({
        "layer_id": request.layer_id,
        "feature_idx": request.feature_idx,
        "feature_value": request.feature_value,
        "decision": request.decision,
        "correction": request.correction,
        "concept": request.concept,
        "annotator_id": request.annotator_id,
        "metadata": request.metadata,
    });

    match (state.submit_feedback_fn)(payload) {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::ok(response))),
        Err(e) => {
            error!(error = %e, "Failed to process feedback");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to store feedback: {}", e))),
            )
        }
    }
}

/// GET /api/metrics
/// Endpoint Prometheus con métricas en formato texto.
pub async fn get_metrics(State(state): State<WebServerState>) -> (StatusCode, String) {
    let metrics = (state.metrics_fn)();
    (StatusCode::OK, metrics)
}

/// GET /api/health
/// Health check para orquestadores (K8s, systemd, etc.)
pub async fn get_health(State(state): State<WebServerState>) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let (healthy, message) = (state.health_fn)();

    let status_code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = serde_json::json!({
        "healthy": healthy,
        "message": message,
        "uptime_seconds": state.uptime_seconds(),
    });

    (status_code, Json(ApiResponse::ok(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, Method};
    use axum::routing::get;
    use axum::Router;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_get_status() {
        let state = WebServerState::default_state();
        let app = Router::new().route("/status", get(get_status)).with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_health() {
        let state = WebServerState::default_state();
        let app = Router::new().route("/health", get(get_health)).with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_api_response_ok() {
        let response: ApiResponse<String> = ApiResponse::ok("test".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse::error("error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("error".to_string()));
    }

    #[test]
    fn test_feedback_request_validation() {
        let valid_json = r#"{"layer_id":"l1","feature_idx":0,"feature_value":0.5,"decision":"approved","annotator_id":"a1"}"#;
        let request: FeedbackRequest = serde_json::from_str(valid_json).unwrap();
        assert_eq!(request.decision, "approved");
        assert_eq!(request.feature_idx, 0);
    }
}
