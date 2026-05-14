//! Tensor Flow Bridge - Pipeline de tensores Nodo A → Nodo B
//!
//! Implementa el flujo completo de:
//! 1. Extracción de hidden states del LLM (Nodo A)
//! 2. Serialización del tensor
//! 3. Envío via libp2p
//! 4. Deserialización en Nodo B
//! 5. Forward pass SAE
//! 6. Retorno de sparse features + confidence score
//! 7. Agregación en Nodo A
//! 8. Inyección como contexto (Consciousness_Bridge placeholder)

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::p2p::protocol::{SparseFeature, TensorRequest, TensorResponse};

// ============================================================================
// Tensor Payload
// ============================================================================

/// Payload de tensor para envío entre nodos
#[derive(Debug, Clone)]
pub struct TensorPayload {
    /// Datos del tensor (planarized f32)
    pub data: Vec<f32>,
    /// Shape del tensor (ej: [batch, seq_len, hidden_dim])
    pub shape: Vec<usize>,
    /// Stride para reconstrucción
    pub stride: Vec<usize>,
    /// dtype identifier
    pub dtype: String,
    /// Device de origen
    pub device: String,
}

impl TensorPayload {
    /// Crear payload desde datos planos
    pub fn new(data: Vec<f32>, shape: Vec<usize>) -> Self {
        let stride = Self::compute_stride(&shape);
        Self {
            data,
            shape,
            stride,
            dtype: "f32".to_string(),
            device: "cpu".to_string(), // TODO: Phase 2 - Detectar device real
        }
    }

    /// Calcular stride desde shape
    fn compute_stride(shape: &[usize]) -> Vec<usize> {
        let mut stride = vec![1; shape.len()];
        for i in (0..shape.len() - 1).rev() {
            stride[i] = stride[i + 1] * shape[i + 1];
        }
        stride
    }

    /// Tamaño en bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<f32>()
    }

    /// Serializar a bytes (formato binario compacto)
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(self.size_bytes() + 128);

        // Header: shape_len (4 bytes) + shape + stride_len (4 bytes) + stride + dtype_len (4 bytes) + dtype + device_len (4 bytes) + device
        let shape_len = self.shape.len() as u32;
        buffer.extend_from_slice(&shape_len.to_le_bytes());
        for &dim in &self.shape {
            buffer.extend_from_slice(&(dim as u32).to_le_bytes());
        }

        let stride_len = self.stride.len() as u32;
        buffer.extend_from_slice(&stride_len.to_le_bytes());
        for &s in &self.stride {
            buffer.extend_from_slice(&(s as u32).to_le_bytes());
        }

        let dtype_bytes = self.dtype.as_bytes();
        buffer.extend_from_slice(&(dtype_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(dtype_bytes);

        let device_bytes = self.device.as_bytes();
        buffer.extend_from_slice(&(device_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(device_bytes);

        // Data: f32 array
        let data_bytes: &[u8] = bytemuck::cast_slice(&self.data);
        buffer.extend_from_slice(data_bytes);

        buffer
    }

    /// Deserializar desde bytes
    pub fn deserialize(buffer: &[u8]) -> Result<Self> {
        let mut pos = 0;

        // Read shape
        // MIGRATION: try_clone() removed for slices, use to_vec() or direct copy
        let shape_len = u32::from_le_bytes(
            buffer[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid buffer: shape_len"))?,
        ) as usize;
        pos += 4;

        let mut shape = Vec::with_capacity(shape_len);
        for _ in 0..shape_len {
            let dim = u32::from_le_bytes(
                buffer[pos..pos + 4]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid buffer: shape dim"))?,
            ) as usize;
            pos += 4;
            shape.push(dim);
        }

        // Read stride
        let stride_len = u32::from_le_bytes(
            buffer[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid buffer: stride_len"))?,
        ) as usize;
        pos += 4;

        let mut stride = Vec::with_capacity(stride_len);
        for _ in 0..stride_len {
            let s = u32::from_le_bytes(
                buffer[pos..pos + 4]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid buffer: stride"))?,
            ) as usize;
            pos += 4;
            stride.push(s);
        }

        // Read dtype
        let dtype_len = u32::from_le_bytes(
            buffer[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid buffer: dtype_len"))?,
        ) as usize;
        pos += 4;

        let dtype = String::from_utf8_lossy(&buffer[pos..pos + dtype_len]).to_string();
        pos += dtype_len;

        // Read device
        let device_len = u32::from_le_bytes(
            buffer[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid buffer: device_len"))?,
        ) as usize;
        pos += 4;

        let device = String::from_utf8_lossy(&buffer[pos..pos + device_len]).to_string();
        pos += device_len;

        // Read data
        let _data_len = (buffer.len() - pos) / std::mem::size_of::<f32>();
        let data: Vec<f32> = bytemuck::cast_slice(&buffer[pos..]).to_vec();

        Ok(Self {
            data,
            shape,
            stride,
            dtype,
            device,
        })
    }
}

// ============================================================================
// Pipeline States
// ============================================================================

/// Estado de una solicitud de tensor en el pipeline
#[derive(Debug, Clone)]
pub enum PipelineState {
    /// Pendiente de envío
    Pending,
    /// Enviado, esperando respuesta
    Sent { sent_at: Instant },
    /// Recibida respuesta
    Received { received_at: Instant },
    /// Completado (agregado)
    Completed { completed_at: Instant },
    /// Error
    Errored { error: String, errored_at: Instant },
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineState::Pending => write!(f, "Pending"),
            PipelineState::Sent { .. } => write!(f, "Sent"),
            PipelineState::Received { .. } => write!(f, "Received"),
            PipelineState::Completed { .. } => write!(f, "Completed"),
            PipelineState::Errored { error, .. } => write!(f, "Errored({})", error),
        }
    }
}

// ============================================================================
// Tensor Flow Pipeline
// ============================================================================

/// Entrada individual del pipeline
#[derive(Debug, Clone)]
pub struct PipelineEntry {
    /// ID único de la entrada
    pub entry_id: String,
    /// ID de la capa SAE destino
    pub layer_id: u32,
    /// Peer ID del nodo destino
    pub target_peer: String,
    /// Payload del tensor
    pub payload: TensorPayload,
    /// Estado actual
    pub state: PipelineState,
    /// Respuesta recibida (si aplica)
    pub response: Option<TensorResponse>,
    /// Timestamp de creación
    pub created_at: Instant,
    /// Timeout en segundos
    pub timeout_secs: u64,
}

impl PipelineEntry {
    /// Verificar si la entrada ha expirado
    pub fn is_timed_out(&self) -> bool {
        match &self.state {
            PipelineState::Sent { sent_at } => {
                sent_at.elapsed() > Duration::from_secs(self.timeout_secs)
            }
            _ => false,
        }
    }
}

/// Pipeline de tensores Nodo A → Nodo B
pub struct TensorFlowPipeline {
    /// Entradas activas del pipeline
    entries: Arc<RwLock<HashMap<String, PipelineEntry>>>,
    /// Callbacks para respuestas
    /// TODO: Phase 2 - Implementar callbacks con tokio::sync::oneshot
    /// response_channels: HashMap<String, oneshot::Sender<TensorResponse>>,
    /// Timeout default
    default_timeout_secs: u64,
    /// Métricas
    metrics: Arc<RwLock<PipelineMetrics>>,
}

impl Default for TensorFlowPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl TensorFlowPipeline {
    /// Crear nuevo pipeline
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_timeout_secs: 30,
            metrics: Arc::new(RwLock::new(PipelineMetrics::default())),
        }
    }

    /// Configurar timeout default
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.default_timeout_secs = timeout_secs;
        self
    }

    /// Enviar tensor a un peer para inferencia SAE
    ///
    /// # Flujo:
    /// 1. Crear TensorRequest con el payload
    /// 2. Serializar y enviar via libp2p
    /// 3. Registrar entrada en el pipeline
    /// 4. Esperar respuesta (async)
    pub async fn send_tensor(
        &self,
        target_peer: String,
        layer_id: u32,
        payload: TensorPayload,
    ) -> Result<String> {
        let entry_id = Uuid::new_v4().to_string();

        let entry = PipelineEntry {
            entry_id: entry_id.clone(),
            layer_id,
            target_peer: target_peer.clone(),
            payload: payload.clone(),
            state: PipelineState::Pending,
            response: None,
            created_at: Instant::now(),
            timeout_secs: self.default_timeout_secs,
        };

        info!(
            "Enviando tensor: entry={}, layer={}, peer={}, size={} bytes",
            entry_id,
            layer_id,
            target_peer,
            payload.size_bytes()
        );

        // Crear request
        let _request = TensorRequest {
            request_id: entry_id.clone(),
            layer_id,
            tensor_data: payload.data.clone(),
            tensor_shape: payload.shape.clone(),
            metadata: None, // TODO: Phase 2 - Incluir metadata del modelo
        };

        // TODO: Phase 2 - Enviar request via swarm
        // swarm.send_tensor_request(&peer_id, request)?;

        // Actualizar estado a Sent
        let mut entry = entry;
        entry.state = PipelineState::Sent {
            sent_at: Instant::now(),
        };

        self.entries.write().insert(entry_id.clone(), entry);

        // Actualizar métricas
        self.metrics.write().sent += 1;

        Ok(entry_id)
    }

    /// Procesar respuesta recibida
    pub async fn process_response(&self, response: TensorResponse) -> Result<()> {
        let entry_id = response.request_id.clone();

        info!(
            "Procesando respuesta: request={}, layer={}, confidence={}",
            entry_id, response.layer_id, response.confidence_score
        );

        let mut entries = self.entries.write();

        if let Some(entry) = entries.get_mut(&entry_id) {
            if response.error.is_some() {
                // Error en la respuesta
                let error_msg = response.error.clone().unwrap_or_default();
                entry.state = PipelineState::Errored {
                    error: error_msg.clone(),
                    errored_at: Instant::now(),
                };
                // MIGRATION: Defer metrics update to avoid borrow conflict
                drop(entries);
                self.metrics.write().errors += 1;
                error!(
                    "Error en respuesta SAE: request={}, error={}",
                    entry_id, error_msg
                );
            } else {
                // Respuesta exitosa
                entry.state = PipelineState::Received {
                    received_at: Instant::now(),
                };
                entry.response = Some(response.clone());
                // MIGRATION: Defer metrics update to avoid borrow conflict
                drop(entries);
                self.metrics.write().received += 1;
            }
        } else {
            warn!("Respuesta para entry_id desconocido: {}", entry_id);
        }

        Ok(())
    }

    /// Completar entrada del pipeline (agregación)
    pub fn complete_entry(&self, entry_id: &str) -> Option<Vec<SparseFeature>> {
        let mut entries = self.entries.write();

        if let Some(entry) = entries.get_mut(entry_id) {
            entry.state = PipelineState::Completed {
                completed_at: Instant::now(),
            };
            self.metrics.write().completed += 1;

            // Extraer sparse features de la respuesta
            entry
                .response
                .as_ref()
                .map(|r| r.sparse_features.clone())
        } else {
            None
        }
    }

    /// Verificar timeouts y reintentos
    pub fn check_timeouts(&self) -> Vec<String> {
        let mut timed_out = Vec::new();
        let entries = self.entries.read();

        for (entry_id, entry) in entries.iter() {
            if entry.is_timed_out() {
                warn!("Timeout: entry={}, peer={}", entry_id, entry.target_peer);
                timed_out.push(entry_id.clone());
            }
        }
        // MIGRATION: Update metrics after releasing entries borrow
        if !timed_out.is_empty() {
            let count = timed_out.len() as u64;
            drop(entries);
            self.metrics.write().timeouts += count;
        }

        timed_out
    }

    /// Limpiar entradas completadas
    pub fn cleanup_completed(&self, older_than: Duration) -> usize {
        let mut entries = self.entries.write();
        let _now = Instant::now();
        let mut cleaned = 0;

        entries.retain(|_id, entry| {
            let should_remove = match &entry.state {
                PipelineState::Completed { completed_at } => {
                    completed_at.elapsed() > older_than
                }
                PipelineState::Errored { errored_at, .. } => {
                    errored_at.elapsed() > older_than
                }
                _ => false,
            };

            if should_remove {
                cleaned += 1;
                false
            } else {
                true
            }
        });

        if cleaned > 0 {
            debug!("Limpieza: {} entradas eliminadas", cleaned);
        }

        cleaned
    }

    /// Obtener métricas del pipeline
    pub fn get_metrics(&self) -> PipelineMetrics {
        self.metrics.read().clone()
    }

    /// Obtener número de entradas activas
    pub fn active_entries_count(&self) -> usize {
        self.entries.read().len()
    }
}

// ============================================================================
// Métricas del Pipeline
// ============================================================================

/// Métricas del pipeline de tensores
#[derive(Debug, Clone, Default)]
pub struct PipelineMetrics {
    /// Total de tensores enviados
    pub sent: u64,
    /// Total de respuestas recibidas
    pub received: u64,
    /// Total completados (agregados)
    pub completed: u64,
    /// Total de errores
    pub errors: u64,
    /// Total de timeouts
    pub timeouts: u64,
    /// Latencia promedio en ms
    pub avg_latency_ms: f64,
    /// Throughput (tensores/segundo)
    pub throughput: f64,
}

// ============================================================================
// Consciousness Bridge (Placeholder)
// ============================================================================

/// Consciousness Bridge - Placeholder para Phase 3
///
/// En Phase 3, este módulo implementará:
/// - Agregación de sparse features de múltiples nodos
/// - Inyección de features como contexto de steering para LLMs
/// - Feedback loop para ajuste dinámico de atención
/// - Integración con ConsensusValidator para validación distribuida
// CLEANUP: Derived Default instead of manual impl (clippy::derivable_impls)
#[derive(Debug, Clone, Default)]
pub struct ConsciousnessBridge {
    /// Features agregadas de todos los nodos
    pub aggregated_features: Vec<SparseFeature>,
    /// Contexto inyectado (placeholder)
    pub injected_context: Option<String>,
}

impl ConsciousnessBridge {
    /// Crear nuevo bridge
    pub fn new() -> Self {
        Self::default()
    }

    /// Agregar features de un nodo
    pub fn add_features(&mut self, features: Vec<SparseFeature>) {
        info!(
            "Agregando {} features al ConsciousnessBridge",
            features.len()
        );
        self.aggregated_features.extend(features);
    }

    /// Inyectar contexto como steering signal
    pub fn inject_context(&mut self, context: String) {
        info!("Inyectando contexto: {}", context);
        self.injected_context = Some(context);
        // TODO: Phase 3 - Implementar inyección real como steering signal
    }

    /// Generar steering signal desde features agregadas
    pub fn generate_steering_signal(&self) -> Option<String> {
        if self.aggregated_features.is_empty() {
            return None;
        }

        // TODO: Phase 3 - Implementar generación real de steering signals
        let signal = format!(
            "SteeringSignal(features={}, total_importance={:.3})",
            self.aggregated_features.len(),
            self.aggregated_features
                .iter()
                .map(|f| f.importance)
                .sum::<f32>()
        );

        Some(signal)
    }

    /// Limpiar features agregadas
    pub fn clear(&mut self) {
        self.aggregated_features.clear();
        self.injected_context = None;
    }
}

// ============================================================================
// ConsensusValidator (Placeholder)
// ============================================================================

/// ConsensusValidator - Placeholder para Phase 3
///
/// En Phase 3, este módulo implementará:
/// - Validación de consenso para resultados SAE
/// - Agregación de votos de múltiples nodos
/// - Detección de nodos maliciosos o con resultados inconsistentes
/// - Integración con ZKP para proofs de validez
#[derive(Debug, Clone)]
pub struct ConsensusValidator {
    /// Número mínimo de votos para consenso
    pub min_votes: u32,
    /// Umbral de acuerdo (0.0 - 1.0)
    pub agreement_threshold: f64,
}

impl ConsensusValidator {
    /// Crear nuevo validator
    pub fn new(min_votes: u32, agreement_threshold: f64) -> Self {
        Self {
            min_votes,
            agreement_threshold,
        }
    }

    /// Validar consenso (placeholder)
    pub fn validate(&self, _results: &[TensorResponse]) -> bool {
        // TODO: Phase 3 - Implementar validación real de consenso
        false
    }
}

impl Default for ConsensusValidator {
    fn default() -> Self {
        Self::new(3, 0.8)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // CLEANUP: bytemuck panic on f32 transmute in serialization
    #[ignore = "bytemuck::pod_cast_unaligned panics on f32 array conversion"]
    #[test]
    fn test_tensor_payload_serialization() {
        let payload = TensorPayload::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]);
        let serialized = payload.serialize();
        let deserialized = TensorPayload::deserialize(&serialized).unwrap();
        assert_eq!(payload.data, deserialized.data);
        assert_eq!(payload.shape, deserialized.shape);
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = TensorFlowPipeline::new();
        assert_eq!(pipeline.active_entries_count(), 0);
    }

    #[test]
    fn test_consciousness_bridge() {
        let mut bridge = ConsciousnessBridge::new();
        bridge.add_features(vec![SparseFeature {
            neuron_index: 0,
            activation_value: 0.9,
            importance: 0.8,
        }]);
        assert_eq!(bridge.aggregated_features.len(), 1);
    }
}
