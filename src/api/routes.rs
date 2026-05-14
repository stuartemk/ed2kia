//! API Routes v2 - Handlers Axum para /api/v2/*
//!
//! Endpoints de la API v2 para Fase 6: interoperabilidad, federación,
//! staking, gobernanza y análisis SAE. Sigue el patrón de callbacks
//! del servidor web existente para evitar acoplamiento directo.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::openapi::{Info, OpenApiSpec, Paths};

// ============================================================
// Request/Response Types
// ============================================================

/// Response genérico de la API v2
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub version: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            version: "v2".to_string(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            version: "v2".to_string(),
        }
    }
}

/// Request para análisis SAE
#[derive(Debug, Clone, Deserialize)]
pub struct SaeAnalyzeRequest {
    /// Tensor de activaciones (vec de f32)
    pub activations: Vec<f32>,
    /// ID de capa origen
    pub layer_id: u32,
    /// Modelo origen (Llama, Mistral, Qwen, etc.)
    #[serde(default = "default_source_model")]
    pub source_model: String,
    /// Dimensionalidad esperada
    #[serde(default)]
    pub expected_dim: Option<usize>,
}

fn default_source_model() -> String {
    "Llama".to_string()
}

/// Response de análisis SAE
#[derive(Debug, Serialize)]
pub struct SaeAnalyzeResponse {
    pub layer_id: u32,
    pub source_model: String,
    pub feature_count: usize,
    pub top_features: Vec<FeatureActivation>,
    pub anomaly_detected: bool,
    pub confidence: f32,
    pub normalized_dim: usize,
}

#[derive(Debug, Serialize)]
pub struct FeatureActivation {
    pub feature_index: u32,
    pub activation_value: f32,
    pub concept_label: Option<String>,
}

/// Request para iniciar ronda de federación
#[derive(Debug, Deserialize)]
pub struct FederationRoundRequest {
    /// Capa a agregar
    pub layer_id: u32,
    /// Número mínimo de participantes
    #[serde(default = "default_min_participants")]
    pub min_participants: usize,
    /// Timeout en segundos
    #[serde(default = "default_round_timeout")]
    pub timeout_seconds: u64,
}

fn default_min_participants() -> usize {
    3
}

fn default_round_timeout() -> u64 {
    60
}

/// Response de ronda de federación
#[derive(Debug, Serialize)]
pub struct FederationRoundResponse {
    pub round_id: String,
    pub layer_id: u32,
    pub status: String,
    pub participants: usize,
    pub updates_received: usize,
    pub estimated_completion: u64,
}

/// Request para propuesta de gobernanza
#[derive(Debug, Deserialize)]
pub struct GovernanceProposalRequest {
    pub title: String,
    pub description: String,
    #[serde(default = "default_proposal_type")]
    pub proposal_type: String,
    pub payload: serde_json::Value,
    #[serde(default = "default_voting_period")]
    pub voting_period_seconds: u64,
}

fn default_proposal_type() -> String {
    "model_update".to_string()
}

fn default_voting_period() -> u64 {
    604800 // 7 days in seconds
}

fn default_voting_period_seconds() -> u64 {
    86400
}

/// Response de propuesta
#[derive(Debug, Serialize)]
pub struct GovernanceProposalResponse {
    pub proposal_id: String,
    pub title: String,
    pub state: String,
    pub created_at: u64,
    pub voting_ends_at: u64,
    pub votes_for: usize,
    pub votes_against: usize,
}

// ============================================================
// API v2 State
// ============================================================

/// Estado compartido del API v2
#[derive(Clone)]
pub struct ApiV2State {
    /// Callback para health check
    pub health_fn: std::sync::Arc<dyn Fn() -> (bool, String) + Send + Sync>,
    /// Callback para info de red
    pub network_fn: std::sync::Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para análisis SAE
    pub sae_analyze_fn: std::sync::Arc<dyn Fn(SaeAnalyzeRequest) -> Result<SaeAnalyzeResponse, String> + Send + Sync>,
    /// Callback para info de federación
    pub federation_info_fn: std::sync::Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para iniciar ronda de federación
    pub federation_start_fn: std::sync::Arc<dyn Fn(FederationRoundRequest) -> Result<FederationRoundResponse, String> + Send + Sync>,
    /// Callback para registro de staking
    pub staking_registry_fn: std::sync::Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para propuestas de gobernanza
    pub governance_proposals_fn: std::sync::Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para crear propuesta
    pub governance_create_fn: std::sync::Arc<dyn Fn(GovernanceProposalRequest) -> Result<GovernanceProposalResponse, String> + Send + Sync>,
    /// Callback para generar OpenAPI spec
    pub openapi_spec_fn: std::sync::Arc<dyn Fn() -> OpenApiSpec + Send + Sync>,
}

impl ApiV2State {
    /// Crea estado con valores por defecto (para testing)
    pub fn default_state() -> Self {
        let healthy = std::sync::Arc::new(|| (true, "ok".to_string()));
        let empty_json = std::sync::Arc::new(|| serde_json::json!({}));
        let sae_default = std::sync::Arc::new(|_: SaeAnalyzeRequest| {
            Ok(SaeAnalyzeResponse {
                layer_id: 0,
                source_model: "Unknown".to_string(),
                feature_count: 0,
                top_features: vec![],
                anomaly_detected: false,
                confidence: 0.0,
                normalized_dim: 0,
            })
        });
        let federation_default = std::sync::Arc::new(|_: FederationRoundRequest| {
            Err("Federation not initialized".to_string())
        });
        let governance_default = std::sync::Arc::new(|_: GovernanceProposalRequest| {
            Err("Governance not initialized".to_string())
        });
        let openapi_default = std::sync::Arc::new(|| OpenApiSpec {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: "ed2kIA API".to_string(),
                description: "API v2".to_string(),
                version: "v2".to_string(),
                contact: super::openapi::Contact {
                    name: "ed2kIA Team".to_string(),
                    url: "https://ed2k.ai".to_string(),
                    email: "contact@ed2k.ai".to_string(),
                },
            },
            servers: vec![],
            paths: Paths::default(),
            components: super::openapi::Components::default(),
        });

        Self {
            health_fn: healthy,
            network_fn: empty_json.clone(),
            sae_analyze_fn: sae_default,
            federation_info_fn: empty_json.clone(),
            federation_start_fn: federation_default,
            staking_registry_fn: empty_json.clone(),
            governance_proposals_fn: empty_json.clone(),
            governance_create_fn: governance_default,
            openapi_spec_fn: openapi_default,
        }
    }
}

// ============================================================
// Route Handlers
// ============================================================

/// GET /api/v2/health
/// Health check del endpoint v2
pub async fn get_health_v2(
    State(state): State<ApiV2State>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let (healthy, message) = (state.health_fn)();

    let status_code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = serde_json::json!({
        "status": if healthy { "healthy" } else { "unhealthy" },
        "message": message,
        "version": env!("CARGO_PKG_VERSION"),
        "api_version": "v2",
    });

    (status_code, Json(ApiResponse::ok(response)))
}

/// GET /api/v2/network
/// Estado de la red P2P
pub async fn get_network_v2(
    State(state): State<ApiV2State>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let network_info = (state.network_fn)();
    (StatusCode::OK, Json(ApiResponse::ok(network_info)))
}

/// POST /api/v2/sae/analyze
/// Analizar activaciones SAE con normalización cross-model
pub async fn post_sae_analyze(
    State(state): State<ApiV2State>,
    Json(request): Json<SaeAnalyzeRequest>,
) -> (StatusCode, Json<ApiResponse<SaeAnalyzeResponse>>) {
    if request.activations.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Activations array cannot be empty".to_string())),
        );
    }

    match (state.sae_analyze_fn)(request.clone()) {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::ok(response))),
        Err(e) => {
            error!("SAE analysis failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e)))
        }
    }
}

/// GET /api/v2/federation/rounds
/// Estado actual de rondas de federación
pub async fn get_federation_rounds(
    State(state): State<ApiV2State>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let info = (state.federation_info_fn)();
    (StatusCode::OK, Json(ApiResponse::ok(info)))
}

/// POST /api/v2/federation/rounds
/// Iniciar nueva ronda de agregación FedAvg
pub async fn post_federation_round(
    State(state): State<ApiV2State>,
    Json(request): Json<FederationRoundRequest>,
) -> (StatusCode, Json<ApiResponse<FederationRoundResponse>>) {
    if request.min_participants == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("min_participants must be > 0".to_string())),
        );
    }

    match (state.federation_start_fn)(request) {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::ok(response))),
        Err(e) => {
            error!("Federation round start failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e)))
        }
    }
}

/// GET /api/v2/staking/registry
/// Registro de nodos con staking y recursos comprometidos
pub async fn get_staking_registry(
    State(state): State<ApiV2State>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let registry = (state.staking_registry_fn)();
    (StatusCode::OK, Json(ApiResponse::ok(registry)))
}

/// GET /api/v2/governance/proposals
/// Listar propuestas de gobernanza activas
pub async fn get_governance_proposals(
    State(state): State<ApiV2State>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let proposals = (state.governance_proposals_fn)();
    (StatusCode::OK, Json(ApiResponse::ok(proposals)))
}

/// POST /api/v2/governance/proposals
/// Crear nueva propuesta de gobernanza
pub async fn post_governance_proposal(
    State(state): State<ApiV2State>,
    Json(request): Json<GovernanceProposalRequest>,
) -> (StatusCode, Json<ApiResponse<GovernanceProposalResponse>>) {
    if request.title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Title cannot be empty".to_string())),
        );
    }

    if request.description.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Description cannot be empty".to_string())),
        );
    }

    match (state.governance_create_fn)(request) {
        Ok(response) => (StatusCode::CREATED, Json(ApiResponse::ok(response))),
        Err(e) => {
            error!("Governance proposal creation failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e)))
        }
    }
}

/// GET /api/v2/openapi.json
/// Generar especificación OpenAPI 3.0.3
pub async fn get_openapi_spec(
    State(state): State<ApiV2State>,
) -> (StatusCode, Json<OpenApiSpec>) {
    let spec = (state.openapi_spec_fn)();
    (StatusCode::OK, Json(spec))
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let response = ApiResponse::<serde_json::Value>::ok(serde_json::json!({"test": true}));
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.version, "v2");
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<serde_json::Value>::error("test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap(), "test error");
    }

    #[test]
    fn test_sae_analyze_request_defaults() {
        let json = r#"{"activations": [1.0, 2.0, 3.0], "layer_id": 5}"#;
        let req: SaeAnalyzeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.source_model, "Llama");
        assert_eq!(req.layer_id, 5);
        assert_eq!(req.activations.len(), 3);
        assert!(req.expected_dim.is_none());
    }

    #[test]
    fn test_federation_round_request_defaults() {
        let json = r#"{"layer_id": 10}"#;
        let req: FederationRoundRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.layer_id, 10);
        assert_eq!(req.min_participants, 3);
        assert_eq!(req.timeout_seconds, 60);
    }

    #[test]
    fn test_governance_proposal_request_defaults() {
        let json = r#"{"title": "Test", "description": "Desc", "payload": {}}"#;
        let req: GovernanceProposalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.proposal_type, "model_update");
        assert_eq!(req.voting_period_seconds, 604800);
    }

    #[test]
    fn test_api_v2_state_default() {
        let state = ApiV2State::default_state();
        let (healthy, msg) = (state.health_fn)();
        assert!(healthy);
        assert_eq!(msg, "ok");
    }

    #[tokio::test]
    async fn test_get_health_v2() {
        let state = ApiV2State::default_state();
        let (status, Json(response)) = get_health_v2(State(state)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_post_sae_analyze_empty_activations() {
        let state = ApiV2State::default_state();
        let request = SaeAnalyzeRequest {
            activations: vec![],
            layer_id: 0,
            source_model: "Llama".to_string(),
            expected_dim: None,
        };
        let (status, Json(response)) = post_sae_analyze(State(state), Json(request)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!response.success);
    }

    #[tokio::test]
    async fn test_post_federation_round_zero_participants() {
        let state = ApiV2State::default_state();
        let request = FederationRoundRequest {
            layer_id: 0,
            min_participants: 0,
            timeout_seconds: 60,
        };
        let (status, Json(response)) = post_federation_round(State(state), Json(request)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!response.success);
    }

    #[tokio::test]
    async fn test_post_governance_proposal_empty_title() {
        let state = ApiV2State::default_state();
        let request = GovernanceProposalRequest {
            title: "".to_string(),
            description: "Test".to_string(),
            proposal_type: "model_update".to_string(),
            payload: serde_json::json!({}),
            voting_period_seconds: 86400,
        };
        let (status, Json(response)) = post_governance_proposal(State(state), Json(request)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!response.success);
    }

    #[tokio::test]
    async fn test_get_openapi_spec() {
        let state = ApiV2State::default_state();
        let (status, Json(spec)) = get_openapi_spec(State(state)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(spec.openapi, "3.0.3");
    }
}
