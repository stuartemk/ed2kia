//! Protocolo de mensajes P2P para ed2kIA
//!
//! Define los esquemas de serialización para comunicación entre nodos.
//! Usa Prost (Protobuf) para metadatos y FlatBuffers para tensores binarios.

// MIGRATION: ProtocolSupport and SendTransaction removed in libp2p 0.53 request_response
use serde::{Deserialize, Serialize};
use std::fmt;

/// Nombre del protocolo ed2kIA para libp2p request-response
pub const ED2K_PROTOCOL_NAME: &str = "/ed2kia.tensor/1.0.0";

/// Prioridad del protocolo (mayor = más prioritario)
pub const ED2K_PROTOCOL_PRIORITY: u32 = 100;

// ============================================================================
// Mensajes principales del protocolo
// ============================================================================

/// Mensaje principal del protocolo ed2kIA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ed2kMessage {
    /// Solicitud de inferencia SAE (Nodo A → Nodo B)
    TensorRequest(TensorRequest),
    /// Respuesta con sparse features (Nodo B → Nodo A)
    TensorResponse(TensorResponse),
    /// Solicitud de lease para capas SAE
    LeaseRequest(LeaseRequest),
    /// Respuesta de lease
    LeaseResponse(LeaseResponse),
    /// Señal de steering (atención, temperatura, etc.)
    SteeringSignal(SteeringSignal),
    /// Publicidad de recursos del nodo
    ResourceAdvertisement(NodeResources),
    // ─── Fase 2: Interpretación, Feedback & Consenso ───
    /// Batch de features para broadcast via gossipsub
    FeatureBatch(FeatureBatch),
    /// Voto de consenso para validación distribuida
    ConsensusVote(ConsensusVote),
    /// Resultado de análisis de features
    AnalysisResult(AnalysisResultPayload),
}

impl fmt::Display for Ed2kMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ed2kMessage::TensorRequest(_) => write!(f, "TensorRequest"),
            Ed2kMessage::TensorResponse(_) => write!(f, "TensorResponse"),
            Ed2kMessage::LeaseRequest(_) => write!(f, "LeaseRequest"),
            Ed2kMessage::LeaseResponse(_) => write!(f, "LeaseResponse"),
            Ed2kMessage::SteeringSignal(_) => write!(f, "SteeringSignal"),
            Ed2kMessage::ResourceAdvertisement(_) => write!(f, "ResourceAdvertisement"),
            Ed2kMessage::FeatureBatch(_) => write!(f, "FeatureBatch"),
            Ed2kMessage::ConsensusVote(_) => write!(f, "ConsensusVote"),
            Ed2kMessage::AnalysisResult(_) => write!(f, "AnalysisResult"),
        }
    }
}

// ============================================================================
// Tensor Request / Response
// ============================================================================

/// Solicitud de inferencia SAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorRequest {
    /// ID único de la solicitud
    pub request_id: String,
    /// ID de la capa SAE a ejecutar
    pub layer_id: u32,
    /// Tensor de entrada (hidden state del LLM)
    pub tensor_data: Vec<f32>,
    /// Shape del tensor (ej: [batch_size, seq_len, hidden_dim])
    pub tensor_shape: Vec<usize>,
    /// Metadata opcional (model name, token range, etc.)
    pub metadata: Option<serde_json::Value>,
}

/// Respuesta con sparse features extraídas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorResponse {
    /// ID de la solicitud original
    pub request_id: String,
    /// ID de la capa SAE ejecutada
    pub layer_id: u32,
    /// Sparse features resultantes (activaciones no-cero)
    pub sparse_features: Vec<SparseFeature>,
    /// Score de confianza de la inferencia (0.0 - 1.0)
    pub confidence_score: f64,
    /// Error opcional (si la inferencia falló)
    pub error: Option<String>,
}

/// Feature sparse individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseFeature {
    /// Índice del neuron activado en el SAE
    pub neuron_index: u32,
    /// Valor de activación
    pub activation_value: f32,
    /// Importancia relativa (normalizada)
    pub importance: f32,
}

// ============================================================================
// Lease Management
// ============================================================================

/// Solicitud de lease para capas SAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRequest {
    /// Peer que solicita el lease
    pub requester_id: String,
    /// Capas SAE solicitadas
    pub layers: Vec<u32>,
    /// Duración solicitada en segundos
    pub duration_secs: u64,
    /// Recursos del nodo solicitante
    pub resources: NodeResources,
}

/// Respuesta de lease
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseResponse {
    /// Si el lease fue aprobado
    pub granted: bool,
    /// Timestamp de expiración (Unix epoch ms)
    pub expires_at: Option<u64>,
    /// Capas asignadas (puede ser subconjunto de las solicitadas)
    pub assigned_layers: Vec<u32>,
    /// Razón de denegación (si aplica)
    pub denial_reason: Option<String>,
}

// ============================================================================
// Steering Signals
// ============================================================================

/// Señal de steering para control asincrónico
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringSignal {
    /// Tipo de señal (attention, temperature, top_k, etc.)
    pub signal_type: String,
    /// Payload de la señal (JSON serializable)
    pub payload: String,
    /// Prioridad (0 = baja, 100 = crítica)
    pub priority: u8,
    /// Timestamp de creación (Unix epoch ms)
    pub timestamp: u64,
}

// ============================================================================
// Recursos del Nodo
// ============================================================================

/// Recursos computacionales del nodo (para sharding dinámico)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResources {
    /// Núcleos de CPU disponibles
    pub cpu_cores: usize,
    /// RAM disponible en GB
    pub available_ram_gb: f64,
    /// Ancho de banda en Mbps
    pub bandwidth_mbps: f64,
    /// Latencia promedio en ms
    pub avg_latency_ms: f64,
    /// Si tiene GPU disponible
    pub has_gpu: bool,
    /// Modelo de GPU (si aplica)
    pub gpu_model: Option<String>,
    /// VRAM disponible en GB (si aplica)
    pub vram_gb: Option<f64>,
}

impl Default for NodeResources {
    fn default() -> Self {
        Self {
            cpu_cores: num_cpus::get(),
            available_ram_gb: 8.0, // TODO: Phase 2 - Detectar RAM real
            bandwidth_mbps: 100.0, // TODO: Phase 2 - Medir bandwidth real
            avg_latency_ms: 10.0,  // TODO: Phase 2 - Medir latencia real
            has_gpu: false,        // TODO: Phase 2 - Detectar GPU
            gpu_model: None,
            vram_gb: None,
        }
    }
}

// ============================================================================
// Fase 2: Nuevos Mensajes de Protocolo
// ============================================================================

/// Batch de features para broadcast via gossipsub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBatch {
    /// ID único del batch
    pub batch_id: String,
    /// Peer ID del nodo origen
    pub origin_peer_id: String,
    /// Capa SAE de origen
    pub layer_id: u32,
    /// Features sparse del batch
    pub features: Vec<SparseFeature>,
    /// Raíz Merkle del batch (para validación de consenso)
    pub merkle_root: String,
    /// Time window (Unix epoch ms)
    pub time_window: u64,
    /// Timestamp (Unix epoch ms)
    pub timestamp: u64,
}

/// Voto de consenso para validación distribuida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    /// Peer ID del nodo votante
    pub voter_peer_id: String,
    /// Batch ID al que se vota
    pub batch_id: String,
    /// Raíz Merkle reportada
    pub merkle_root: String,
    /// Capa SAE
    pub layer_id: u32,
    /// Time window
    pub time_window: u64,
    /// Confianza del voto (0.0 - 1.0)
    pub confidence: f64,
    /// Timestamp (Unix epoch ms)
    pub timestamp: u64,
}

/// Payload de resultado de análisis para broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResultPayload {
    /// Peer ID del nodo analizador
    pub analyzer_peer_id: String,
    /// Capa SAE analizada
    pub layer_id: u32,
    /// Score de anomalía (0.0 - 1.0)
    pub anomaly_score: f32,
    /// Features flaggeadas
    pub flagged_features: Vec<usize>,
    /// Confianza del análisis (0.0 - 1.0)
    pub confidence: f32,
    /// Patrones detectados (como strings)
    pub detected_patterns: Vec<String>,
    /// Timestamp (Unix epoch ms)
    pub timestamp: u64,
}

// ============================================================================
// Codec para libp2p request-response
// ============================================================================

// MIGRATION: libp2p 0.53 - cbor::codec is private. Use cbor::Behaviour directly
// which handles serialization internally. The Ed2kBehaviour will use
// request_response::cbor::Behaviour instead of request_response::Behaviour<Codec>.
/// Helper para obtener el protocolo Ed2kIA
#[derive(Clone, Copy)]
pub struct Ed2kMessageCodec;

impl Ed2kMessageCodec {
    /// Nombre del protocolo
    pub fn protocol_name() -> libp2p::StreamProtocol {
        libp2p::StreamProtocol::new(ED2K_PROTOCOL_NAME)
    }

    /// Prioridad del protocolo
    pub fn priority() -> u32 {
        ED2K_PROTOCOL_PRIORITY
    }

    /// Protocols soportados (inbound + outbound)
    pub fn supported_protocols() -> Vec<libp2p::StreamProtocol> {
        vec![Self::protocol_name()]
    }
}


// ============================================================================
// Placeholders para Phase 3 - ZKP y WASM
// ============================================================================

/// Placeholder para Zero-Knowledge Proofs
///
/// En Phase 3, este módulo implementará:
/// - Generación de ZK proofs para validación de inferencia SAE
/// - Verificación de proofs sin revelar datos originales
/// - Integración con circuitos WASM para computación verificable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKPPlaceholder {
    pub proof_hash: String,
    pub circuit_id: String,
    pub verified: bool,
}

impl ZKPPlaceholder {
    /// Generar placeholder de ZKP (seguro pero no funcional)
    pub fn generate(_data: &[u8]) -> Self {
        Self {
            proof_hash: "0x00000000000000000000000000000000".to_string(),
            circuit_id: "placeholder_circuit_v1".to_string(),
            verified: false, // TODO: Phase 3 - Implementar verificación real
        }
    }
}

/// Placeholder para WASM Module Execution
///
/// En Phase 3, este módulo implementará:
/// - Ejecución de módulos WASM para inferencia SAE verificable
/// - Sandboxing de código no confiable
/// - Integración con ZKP para proofs de ejecución correcta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WASMPlaceholder {
    pub module_hash: String,
    pub execution_result: Option<serde_json::Value>,
    pub gas_used: u64,
}

impl WASMPlaceholder {
    /// Ejecutar placeholder de WASM (seguro pero no funcional)
    pub fn execute(_module: &[u8], _input: &[u8]) -> Self {
        Self {
            module_hash: "0x00000000000000000000000000000000".to_string(),
            execution_result: None, // TODO: Phase 3 - Implementar ejecución WASM real
            gas_used: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_request_serialization() {
        let req = TensorRequest {
            request_id: "test-123".to_string(),
            layer_id: 0,
            tensor_data: vec![1.0, 2.0, 3.0],
            tensor_shape: vec![1, 3],
            metadata: None,
        };
        let serialized = bincode::serialize(&req).unwrap();
        let deserialized: TensorRequest = bincode::deserialize(&serialized).unwrap();
        assert_eq!(req.request_id, deserialized.request_id);
        assert_eq!(req.layer_id, deserialized.layer_id);
    }

    #[test]
    fn test_tensor_response_serialization() {
        let resp = TensorResponse {
            request_id: "test-123".to_string(),
            layer_id: 0,
            sparse_features: vec![SparseFeature {
                neuron_index: 42,
                activation_value: 0.95,
                importance: 0.8,
            }],
            confidence_score: 0.95,
            error: None,
        };
        let serialized = bincode::serialize(&resp).unwrap();
        let deserialized: TensorResponse = bincode::deserialize(&serialized).unwrap();
        assert_eq!(resp.confidence_score, deserialized.confidence_score);
    }

    #[test]
    fn test_node_resources_default() {
        let resources = NodeResources::default();
        assert!(resources.cpu_cores > 0);
    }
}
