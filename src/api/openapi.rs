//! OpenAPI Spec - Generador de especificación OpenAPI 3.0 para ed2kIA
//!
//! Genera la especificación API REST para integración con clientes
//! externos y documentación automática.

use serde::{Deserialize, Serialize};

/// Especificación OpenAPI 3.0 para ed2kIA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: Paths,
    pub components: Components,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub description: String,
    pub version: String,
    pub contact: Contact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub url: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Paths {
    #[serde(flatten)]
    pub endpoints: std::collections::HashMap<String, PathItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub summary: String,
    pub description: String,
    pub operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: Responses,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub in_: String,
    pub description: String,
    pub required: bool,
    pub schema: SchemaRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub description: String,
    pub required: bool,
    pub content: std::collections::HashMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema: SchemaRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Responses {
    #[serde(flatten)]
    pub codes: std::collections::HashMap<String, Response>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<std::collections::HashMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Components {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<std::collections::HashMap<String, SchemaRef>>,
}

impl OpenApiSpec {
    /// Generar especificación completa para ed2kIA v2 API
    pub fn generate() -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: "ed2kIA API".to_string(),
                description: "API REST para la red descentralizada de interpretabilidad ed2kIA. Proporciona endpoints para gestión de nodos P2P, análisis SAE, federación, staking y gobernanza.".to_string(),
                version: "2.0.0".to_string(),
                contact: Contact {
                    name: "ed2kIA Team".to_string(),
                    url: "https://github.com/ed2kia/ed2kia".to_string(),
                    email: "dev@ed2kia.org".to_string(),
                },
            },
            servers: vec![
                Server {
                    url: "http://localhost:3030".to_string(),
                    description: "Development server".to_string(),
                },
                Server {
                    url: "https://api.ed2kia.network".to_string(),
                    description: "Production gateway".to_string(),
                },
            ],
            paths: Self::generate_paths(),
            components: Self::generate_components(),
        }
    }

    fn generate_paths() -> Paths {
        let mut paths = Paths::default();

        // Health & Status
        paths.endpoints.insert(
            "/api/v2/health".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: "Health check".to_string(),
                    description: "Verifica el estado de salud del nodo ed2kIA.".to_string(),
                    operation_id: "getHealth".to_string(),
                    tags: Some(vec!["system".to_string()]),
                    parameters: None,
                    request_body: None,
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "200".to_string(),
                            Response {
                                description: "Node is healthy".to_string(),
                                content: Some(std::collections::HashMap::from_iter([(
                                    "application/json".to_string(),
                                    MediaType {
                                        schema: SchemaRef {
                                            r#type: Some("object".to_string()),
                                            ..Default::default()
                                        },
                                    },
                                )])),
                            },
                        )]),
                    },
                }),
                ..Default::default()
            },
        );

        // Network Status
        paths.endpoints.insert(
            "/api/v2/network".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: "Network status".to_string(),
                    description: "Obtiene información del estado de la red P2P.".to_string(),
                    operation_id: "getNetworkStatus".to_string(),
                    tags: Some(vec!["network".to_string()]),
                    parameters: None,
                    request_body: None,
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "200".to_string(),
                            Response {
                                description: "Network status retrieved".to_string(),
                                content: Some(std::collections::HashMap::from_iter([(
                                    "application/json".to_string(),
                                    MediaType {
                                        schema: SchemaRef {
                                            r#ref: Some(
                                                "#/components/schemas/NetworkStatus".to_string(),
                                            ),
                                            ..Default::default()
                                        },
                                    },
                                )])),
                            },
                        )]),
                    },
                }),
                ..Default::default()
            },
        );

        // SAE Analysis
        paths.endpoints.insert(
            "/api/v2/sae/analyze".to_string(),
            PathItem {
                post: Some(Operation {
                    summary: "Analyze hidden state".to_string(),
                    description: "Envía un hidden state para análisis SAE distribuido.".to_string(),
                    operation_id: "analyzeHiddenState".to_string(),
                    tags: Some(vec!["sae".to_string()]),
                    parameters: None,
                    request_body: Some(RequestBody {
                        description: "Hidden state data".to_string(),
                        required: true,
                        content: std::collections::HashMap::from_iter([(
                            "application/json".to_string(),
                            MediaType {
                                schema: SchemaRef {
                                    r#ref: Some(
                                        "#/components/schemas/HiddenStateInput".to_string(),
                                    ),
                                    ..Default::default()
                                },
                            },
                        )]),
                    }),
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([
                            (
                                "200".to_string(),
                                Response {
                                    description: "Analysis complete".to_string(),
                                    content: Some(std::collections::HashMap::from_iter([(
                                        "application/json".to_string(),
                                        MediaType {
                                            schema: SchemaRef {
                                                r#ref: Some(
                                                    "#/components/schemas/AnalysisResult"
                                                        .to_string(),
                                                ),
                                                ..Default::default()
                                            },
                                        },
                                    )])),
                                },
                            ),
                            (
                                "400".to_string(),
                                Response {
                                    description: "Invalid input".to_string(),
                                    content: None,
                                },
                            ),
                        ]),
                    },
                }),
                ..Default::default()
            },
        );

        // Federation
        paths.endpoints.insert(
            "/api/v2/federation/rounds".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: "List federation rounds".to_string(),
                    description: "Lista los rounds de federación activos y completados."
                        .to_string(),
                    operation_id: "listFederationRounds".to_string(),
                    tags: Some(vec!["federation".to_string()]),
                    parameters: None,
                    request_body: None,
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "200".to_string(),
                            Response {
                                description: "Rounds list".to_string(),
                                content: None,
                            },
                        )]),
                    },
                }),
                post: Some(Operation {
                    summary: "Start federation round".to_string(),
                    description: "Inicia un nuevo round de entrenamiento federado.".to_string(),
                    operation_id: "startFederationRound".to_string(),
                    tags: Some(vec!["federation".to_string()]),
                    parameters: None,
                    request_body: Some(RequestBody {
                        description: "Round configuration".to_string(),
                        required: true,
                        content: std::collections::HashMap::from_iter([(
                            "application/json".to_string(),
                            MediaType {
                                schema: SchemaRef {
                                    r#ref: Some("#/components/schemas/RoundConfig".to_string()),
                                    ..Default::default()
                                },
                            },
                        )]),
                    }),
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "201".to_string(),
                            Response {
                                description: "Round started".to_string(),
                                content: None,
                            },
                        )]),
                    },
                }),
                ..Default::default()
            },
        );

        // Staking
        paths.endpoints.insert(
            "/api/v2/staking/registry".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: "Staking registry".to_string(),
                    description: "Obtiene el registro de nodos con staking activo.".to_string(),
                    operation_id: "getStakingRegistry".to_string(),
                    tags: Some(vec!["staking".to_string()]),
                    parameters: None,
                    request_body: None,
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "200".to_string(),
                            Response {
                                description: "Registry data".to_string(),
                                content: None,
                            },
                        )]),
                    },
                }),
                ..Default::default()
            },
        );

        // Governance
        paths.endpoints.insert(
            "/api/v2/governance/proposals".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: "List proposals".to_string(),
                    description: "Lista las propuestas de gobernanza activas.".to_string(),
                    operation_id: "listProposals".to_string(),
                    tags: Some(vec!["governance".to_string()]),
                    parameters: None,
                    request_body: None,
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "200".to_string(),
                            Response {
                                description: "Proposals list".to_string(),
                                content: None,
                            },
                        )]),
                    },
                }),
                post: Some(Operation {
                    summary: "Submit proposal".to_string(),
                    description: "Envía una nueva propuesta de gobernanza.".to_string(),
                    operation_id: "submitProposal".to_string(),
                    tags: Some(vec!["governance".to_string()]),
                    parameters: None,
                    request_body: Some(RequestBody {
                        description: "Proposal data".to_string(),
                        required: true,
                        content: std::collections::HashMap::from_iter([(
                            "application/json".to_string(),
                            MediaType {
                                schema: SchemaRef {
                                    r#ref: Some("#/components/schemas/Proposal".to_string()),
                                    ..Default::default()
                                },
                            },
                        )]),
                    }),
                    responses: Responses {
                        codes: std::collections::HashMap::from_iter([(
                            "201".to_string(),
                            Response {
                                description: "Proposal created".to_string(),
                                content: None,
                            },
                        )]),
                    },
                }),
                ..Default::default()
            },
        );

        paths
    }

    fn generate_components() -> Components {
        let mut schemas = std::collections::HashMap::new();

        schemas.insert(
            "NetworkStatus".to_string(),
            SchemaRef {
                r#type: Some("object".to_string()),
                ..Default::default()
            },
        );

        schemas.insert(
            "HiddenStateInput".to_string(),
            SchemaRef {
                r#type: Some("object".to_string()),
                ..Default::default()
            },
        );

        schemas.insert(
            "AnalysisResult".to_string(),
            SchemaRef {
                r#type: Some("object".to_string()),
                ..Default::default()
            },
        );

        schemas.insert(
            "RoundConfig".to_string(),
            SchemaRef {
                r#type: Some("object".to_string()),
                ..Default::default()
            },
        );

        schemas.insert(
            "Proposal".to_string(),
            SchemaRef {
                r#type: Some("object".to_string()),
                ..Default::default()
            },
        );

        Components {
            schemas: Some(schemas),
        }
    }

    /// Serializar a JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Serializar a YAML (requiere serde_yaml)
    pub fn to_yaml_string(&self) -> String {
        // Fallback a JSON si serde_yaml no disponible
        self.to_json().unwrap_or_default()
    }
}

impl Default for OpenApiSpec {
    fn default() -> Self {
        Self::generate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let spec = OpenApiSpec::generate();
        assert_eq!(spec.openapi, "3.0.3");
        assert_eq!(spec.info.title, "ed2kIA API");
        assert_eq!(spec.info.version, "2.0.0");
        assert_eq!(spec.servers.len(), 2);
    }

    #[test]
    fn test_openapi_paths() {
        let spec = OpenApiSpec::generate();
        assert!(spec.paths.endpoints.contains_key("/api/v2/health"));
        assert!(spec.paths.endpoints.contains_key("/api/v2/network"));
        assert!(spec.paths.endpoints.contains_key("/api/v2/sae/analyze"));
        assert!(spec
            .paths
            .endpoints
            .contains_key("/api/v2/federation/rounds"));
        assert!(spec
            .paths
            .endpoints
            .contains_key("/api/v2/staking/registry"));
        assert!(spec
            .paths
            .endpoints
            .contains_key("/api/v2/governance/proposals"));
    }

    #[test]
    fn test_openapi_json_serialization() {
        let spec = OpenApiSpec::generate();
        let json = spec.to_json().unwrap();
        assert!(json.contains("\"openapi\": \"3.0.3\""));
        assert!(json.contains("\"title\": \"ed2kIA API\""));
    }

    #[test]
    fn test_openapi_components() {
        let spec = OpenApiSpec::generate();
        let schemas = spec.components.schemas.as_ref().unwrap();
        assert!(schemas.contains_key("NetworkStatus"));
        assert!(schemas.contains_key("AnalysisResult"));
        assert!(schemas.contains_key("Proposal"));
    }
}
