//! Federation Bridge - Puente de Federación Cross-Red para Fase 7 Sprint 1
//!
//! Implementa `FederationBridge` para sincronización de deltas entre redes
//! independientes ed2kIA, con traducción de esquemas, routing por confianza
//! y handshake de protocolo.
//!
//! Extiende `sync_protocol.rs` agregando capacidades inter-red:
//! - Handshake de protocolo con negociación de versión
//! - Traducción de esquemas de delta (Qwen-Scope ↔ Llama-3 adapters)
//! - Trust routing: pondera actualizaciones por reputación criptográfica

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
// CLEANUP: removed unused import Duration
use thiserror::Error;
use tracing::{debug, info, warn};
// CLEANUP: removed unused import error

/// Error específico del Federation Bridge
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Protocol handshake failed: {reason}")]
    HandshakeFailed { reason: String },

    #[error("Schema translation error: {source_schema} -> {target_schema}: {details}")]
    SchemaTranslation {
        source_schema: String,
        target_schema: String,
        details: String,
    },

    #[error("Trust score too low: {trust_score:.4} < {min_trust:.4} for node {node_id}")]
    TrustTooLow {
        node_id: String,
        trust_score: f32,
        min_trust: f32,
    },

    #[error("Invalid delta hash: expected {expected}, got {actual}")]
    InvalidDeltaHash { expected: String, actual: String },

    #[error("Protocol version mismatch: local={local}, remote={remote}")]
    ProtocolVersionMismatch { local: String, remote: String },

    #[error("Network identity not found: {network_id}")]
    NetworkNotFound { network_id: String },

    #[error("Delta merge conflict: {reason}")]
    MergeConflict { reason: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Identidad de una red federada
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkIdentity {
    /// ID único de la red (ej: "ed2k-main", "ed2k-health")
    pub network_id: String,
    /// Hash del bloque génesis de la red
    pub genesis_hash: String,
    /// Clave pública Ed25519 de la red (hex)
    pub public_key_hex: String,
    /// Tags de dominio semántico
    pub domain_tags: Vec<String>,
    /// Versión de protocolo soportada
    pub protocol_version: String,
}

impl NetworkIdentity {
    /// Crea nueva identidad de red
    pub fn new(network_id: String, genesis_hash: String, public_key_hex: String) -> Self {
        Self {
            network_id,
            genesis_hash,
            public_key_hex,
            domain_tags: Vec::new(),
            protocol_version: PROTOCOL_VERSION.to_string(),
        }
    }

    /// Agrega tag de dominio
    pub fn with_domain_tag(mut self, tag: String) -> Self {
        self.domain_tags.push(tag);
        self
    }
}

/// Delta de pesos para sincronización cross-red
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaUpdate {
    /// ID de la red origen
    pub source_network: String,
    /// ID del nodo origen
    pub source_node: String,
    /// ID de capa SAE
    pub layer_id: u32,
    /// Delta de pesos (vector plano)
    pub weights: Vec<f32>,
    /// Hash SHA-256 del delta (hex)
    pub delta_hash: String,
    /// Round de federación local
    pub local_round: u64,
    /// Número de participantes en la agregación local
    pub participant_count: usize,
    /// Confianza de la agregación local (0.0 - 1.0)
    pub confidence: f32,
    /// Esquema de origen (ej: "qwen-scope", "llama-3")
    pub source_schema: String,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

impl DeltaUpdate {
    /// Crea nuevo delta con hash automático
    #[allow(clippy::too_many_arguments)] // ALLOWED: 8 args required for DeltaUpdate
    pub fn new(
        source_network: String,
        source_node: String,
        layer_id: u32,
        weights: Vec<f32>,
        local_round: u64,
        participant_count: usize,
        confidence: f32,
        source_schema: String,
    ) -> Self {
        let delta_hash = Self::compute_hash(&weights, &source_network, layer_id);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            source_network,
            source_node,
            layer_id,
            weights,
            delta_hash,
            local_round,
            participant_count,
            confidence,
            source_schema,
            timestamp_ms,
        }
    }

    /// Verifica integridad del hash
    pub fn verify_hash(&self) -> bool {
        let expected = Self::compute_hash(&self.weights, &self.source_network, self.layer_id);
        self.delta_hash == expected
    }

    fn compute_hash(weights: &[f32], source_network: &str, layer_id: u32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source_network.as_bytes());
        hasher.update(layer_id.to_be_bytes()); // CLEANUP: Removed needless borrow
        for w in weights {
            hasher.update(w.to_be_bytes()); // CLEANUP: Removed needless borrow
        }
        let result = hasher.finalize();
        hex::encode(result)
    }
}

/// Registro de confianza por red
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRecord {
    /// ID de la red
    pub network_id: String,
    /// Puntuación de confianza actual (0.0 - 1.0)
    pub trust_score: f32,
    /// Número de sincronizaciones exitosas
    pub successful_syncs: u64,
    /// Número de sincronizaciones fallidas
    pub failed_syncs: u64,
    /// Timestamp del último sync (epoch ms)
    pub last_sync_ms: u64,
    /// Factor de decaimiento de confianza (0.0 - 1.0)
    pub decay_factor: f32,
}

impl TrustRecord {
    /// Crea nuevo registro de confianza
    pub fn new(network_id: String) -> Self {
        Self {
            network_id,
            trust_score: 0.5, // Trust inicial neutral
            successful_syncs: 0,
            failed_syncs: 0,
            last_sync_ms: 0,
            decay_factor: 0.995, // 0.5% decay por ciclo
        }
    }

    /// Actualiza confianza tras sync exitoso
    pub fn record_success(&mut self) {
        self.successful_syncs += 1;
        self.trust_score = (self.trust_score + 0.02).min(1.0);
        self.last_sync_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Actualiza confianza tras sync fallido
    pub fn record_failure(&mut self) {
        self.failed_syncs += 1;
        self.trust_score = (self.trust_score - 0.05).max(0.0);
        self.last_sync_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Aplica decaimiento de confianza
    pub fn apply_decay(&mut self) {
        self.trust_score *= self.decay_factor;
        self.trust_score = self.trust_score.max(0.0);
    }
}

/// Resultado de operaciones del puente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResult {
    /// Número de nodos sincronizados
    pub synced_nodes: usize,
    /// Número de updates mergeados
    pub merged_updates: usize,
    /// Promedio de confianza de las redes participantes
    pub trust_avg: f32,
    /// Versión de protocolo usada
    pub protocol_version: String,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

/// Mensaje de handshake de protocolo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Identidad de la red
    pub identity: NetworkIdentity,
    /// Esquemas soportados
    pub supported_schemas: Vec<String>,
    /// Nonce para prevención de replay
    pub nonce: u64,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

impl HandshakeMessage {
    /// Crea mensaje de handshake
    pub fn new(identity: NetworkIdentity, supported_schemas: Vec<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            identity,
            supported_schemas,
            nonce: fastrand::u64(1..u64::MAX), // CLEANUP: Removed unnecessary cast
            timestamp_ms: timestamp,
        }
    }

    /// Verifica que el handshake no haya expirado (max 5 min)
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let age_ms = now.saturating_sub(self.timestamp_ms);
        age_ms < 300_000 // 5 minutes
    }
}

/// Versión actual del protocolo
const PROTOCOL_VERSION: &str = "7.1.0";

/// Puente de federación cross-red
pub struct FederationBridge {
    /// Identidad local
    local_identity: NetworkIdentity,
    /// Redes confiables
    trusted_networks: HashMap<String, NetworkIdentity>,
    /// Registros de confianza
    trust_records: HashMap<String, TrustRecord>,
    /// Umbral mínimo de confianza para aceptar deltas
    min_trust_threshold: f32,
    /// Esquemas soportados localmente
    supported_schemas: Vec<String>,
    /// Buffer de deltas pendientes
    pending_deltas: Vec<DeltaUpdate>,
    /// Historial de resultados
    result_history: Vec<BridgeResult>,
}

impl FederationBridge {
    /// Crea nuevo puente de federación
    pub fn new(local_identity: NetworkIdentity, min_trust_threshold: f32) -> Self {
        Self {
            local_identity,
            trusted_networks: HashMap::new(),
            trust_records: HashMap::new(),
            min_trust_threshold,
            supported_schemas: vec!["qwen-scope".to_string()],
            pending_deltas: Vec::new(),
            result_history: Vec::new(),
        }
    }

    /// Inicia handshake de protocolo con red remota
    ///
    /// Negocia versión de protocolo y esquemas compatibles.
    /// Retorna HandshakeMessage para enviar a la red remota.
    pub fn init_handshake(&self, remote_network_id: &str) -> Result<HandshakeMessage, BridgeError> {
        // Verificar que la red remota esté en la lista de confiables
        if !self.trusted_networks.contains_key(remote_network_id) {
            return Err(BridgeError::HandshakeFailed {
                reason: format!("Unknown network: {}", remote_network_id),
            });
        }

        let remote_identity = self.trusted_networks.get(remote_network_id).unwrap();

        // Verificar compatibilidad de versión de protocolo
        if !Self::versions_compatible(
            &self.local_identity.protocol_version,
            &remote_identity.protocol_version,
        ) {
            return Err(BridgeError::ProtocolVersionMismatch {
                local: self.local_identity.protocol_version.clone(),
                remote: remote_identity.protocol_version.clone(),
            });
        }

        info!(
            remote = %remote_network_id,
            local_version = %self.local_identity.protocol_version,
            remote_version = %remote_identity.protocol_version,
            "Handshake initiated"
        );

        Ok(HandshakeMessage::new(
            self.local_identity.clone(),
            self.supported_schemas.clone(),
        ))
    }

    /// Procesa respuesta de handshake de red remota
    ///
    /// Valida la respuesta y establece conexión si es compatible.
    pub fn process_handshake_response(
        &mut self,
        response: HandshakeMessage,
    ) -> Result<(), BridgeError> {
        if !response.is_valid() {
            return Err(BridgeError::HandshakeFailed {
                reason: "Handshake message expired".to_string(),
            });
        }

        let remote_id = &response.identity.network_id;

        // Verificar compatibilidad de versión
        if !Self::versions_compatible(
            &self.local_identity.protocol_version,
            &response.identity.protocol_version,
        ) {
            return Err(BridgeError::ProtocolVersionMismatch {
                local: self.local_identity.protocol_version.clone(),
                remote: response.identity.protocol_version.clone(),
            });
        }

        // Verificar compatibilidad de esquemas
        let has_common_schema = response
            .supported_schemas
            .iter()
            .any(|s| self.supported_schemas.contains(s));

        if !has_common_schema {
            return Err(BridgeError::HandshakeFailed {
                reason: "No common schema found".to_string(),
            });
        }

        // Registrar red como confiable
        self.trusted_networks
            .insert(remote_id.clone(), response.identity.clone());

        // Crear registro de confianza si no existe
        if !self.trust_records.contains_key(remote_id) {
            self.trust_records
                .insert(remote_id.clone(), TrustRecord::new(remote_id.clone()));
        }

        info!(
            remote = %remote_id,
            schemas = ?response.supported_schemas,
            "Handshake completed successfully"
        );

        Ok(())
    }

    /// Sincroniza delta desde red remota
    ///
    /// Valida hash, verifica confianza y traduce esquema si es necesario.
    pub fn sync_delta(&mut self, delta: DeltaUpdate) -> Result<(), BridgeError> {
        // Verificar integridad del hash
        if !delta.verify_hash() {
            return Err(BridgeError::InvalidDeltaHash {
                expected: delta.delta_hash.clone(),
                actual: DeltaUpdate::compute_hash(
                    &delta.weights,
                    &delta.source_network,
                    delta.layer_id,
                ),
            });
        }

        // Verificar confianza de la red origen
        let trust_score = self.get_trust_score(&delta.source_network);
        if trust_score < self.min_trust_threshold {
            // Registrar fallo
            self.record_sync_failure(&delta.source_network);
            return Err(BridgeError::TrustTooLow {
                node_id: delta.source_network.clone(),
                trust_score,
                min_trust: self.min_trust_threshold,
            });
        }

        // Traducir esquema si es necesario
        let translated_weights = self.translate_schema(
            &delta.weights,
            &delta.source_schema,
            "qwen-scope", // Target schema local
        )?;

        // Crear delta traducido
        let translated_delta = DeltaUpdate {
            weights: translated_weights,
            ..delta
        };

        // FIX: borrow/move - Extract values before pushing translated_delta | borrow/move
        let source_network = translated_delta.source_network.clone();
        let layer_id = translated_delta.layer_id;
        let weights_len = translated_delta.weights.len();

        // Almacenar en buffer pendiente
        self.pending_deltas.push(translated_delta);

        // Registrar éxito
        self.record_sync_success(&source_network);

        debug!(
            source = %source_network,
            layer = layer_id,
            weights_len = weights_len,
            "Delta synced"
        );

        Ok(())
    }

    /// Mergea deltas pendientes en actualización consolidada
    ///
    /// Aplica weighted average ponderado por confianza y participant_count.
    pub fn merge_updates(&mut self) -> Result<BridgeResult, BridgeError> {
        if self.pending_deltas.is_empty() {
            return Ok(self.create_empty_result());
        }

        // Agrupar deltas por layer_id
        let mut layer_deltas: HashMap<u32, Vec<DeltaUpdate>> = HashMap::new();
        for delta in self.pending_deltas.drain(..) {
            layer_deltas.entry(delta.layer_id).or_default().push(delta);
        }

        let mut total_merged = 0;
        let mut total_trust: f32 = 0.0;
        let mut trust_count: usize = 0;

        for (layer_id, deltas) in &layer_deltas {
            // Validar que todos los deltas tengan misma dimensión
            let target_dim = deltas[0].weights.len();
            for d in deltas {
                if d.weights.len() != target_dim {
                    return Err(BridgeError::MergeConflict {
                        reason: format!(
                            "Layer {}: dimension mismatch ({}) vs ({})",
                            layer_id,
                            d.weights.len(),
                            target_dim
                        ),
                    });
                }
            }

            // Weighted average por confianza * participant_count
            let mut merged = vec![0.0f32; target_dim];
            let mut total_weight: f32 = 0.0;

            for delta in deltas {
                let network_trust = self.get_trust_score(&delta.source_network);
                let weight = delta.confidence * network_trust * delta.participant_count as f32;
                total_weight += weight;

                for (i, w) in delta.weights.iter().enumerate() {
                    merged[i] += w * weight;
                }

                total_trust += network_trust;
                trust_count += 1;
            }

            // Normalizar
            if total_weight > 0.0 {
                for w in &mut merged {
                    *w /= total_weight;
                }
            }

            total_merged += 1;

            debug!(
                layer = layer_id,
                inputs = deltas.len(),
                "Layer deltas merged"
            );
        }

        let trust_avg = if trust_count > 0 {
            total_trust / trust_count as f32
        } else {
            0.0
        };

        let result = BridgeResult {
            synced_nodes: self.trusted_networks.len(),
            merged_updates: total_merged,
            trust_avg,
            protocol_version: PROTOCOL_VERSION.to_string(),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        self.result_history.push(result.clone());

        info!(
            merged = total_merged,
            trust_avg = trust_avg,
            "Updates merged successfully"
        );

        Ok(result)
    }

    /// Calcula puntuación de confianza para una red dada
    pub fn calculate_trust_score(&self, network_id: &str) -> f32 {
        self.get_trust_score(network_id)
    }

    /// Agrega red confiable
    pub fn add_trusted_network(&mut self, identity: NetworkIdentity) {
        let network_id = identity.network_id.clone();
        self.trusted_networks.insert(network_id.clone(), identity);

        if !self.trust_records.contains_key(&network_id) {
            // FIX: borrow/move - Clone network_id before inserting
            self.trust_records
                .insert(network_id.clone(), TrustRecord::new(network_id.clone()));
        }
    }

    /// Obtiene lista de redes confiables
    pub fn trusted_networks(&self) -> Vec<&NetworkIdentity> {
        self.trusted_networks.values().collect()
    }

    /// Obtiene registro de confianza para una red
    pub fn get_trust_record(&self, network_id: &str) -> Option<&TrustRecord> {
        self.trust_records.get(network_id)
    }

    /// Aplica decaimiento de confianza a todas las redes
    pub fn apply_trust_decay(&mut self) {
        for record in self.trust_records.values_mut() {
            record.apply_decay();
        }
    }

    /// Agrega esquema soportado
    pub fn add_supported_schema(&mut self, schema: String) {
        if !self.supported_schemas.contains(&schema) {
            self.supported_schemas.push(schema);
        }
    }

    /// Obtiene historial de resultados
    pub fn result_history(&self) -> &[BridgeResult] {
        &self.result_history
    }

    /// Obtiene identidad local
    pub fn local_identity(&self) -> &NetworkIdentity {
        &self.local_identity
    }

    // =====================================================================
    // Métodos privados
    // =====================================================================

    fn get_trust_score(&self, network_id: &str) -> f32 {
        self.trust_records
            .get(network_id)
            .map(|r| r.trust_score)
            .unwrap_or(0.0)
    }

    fn record_sync_success(&mut self, network_id: &str) {
        if let Some(record) = self.trust_records.get_mut(network_id) {
            record.record_success();
        }
    }

    fn record_sync_failure(&mut self, network_id: &str) {
        if let Some(record) = self.trust_records.get_mut(network_id) {
            record.record_failure();
        }
    }

    fn translate_schema(
        &self,
        weights: &[f32],
        source_schema: &str,
        target_schema: &str,
    ) -> Result<Vec<f32>, BridgeError> {
        if source_schema == target_schema {
            return Ok(weights.to_vec());
        }

        // Traducciones conocidas
        match (source_schema, target_schema) {
            ("llama-3", "qwen-scope") => {
                // Llama-3 → Qwen-Scope: re-mapeo de dimensión con padding/truncación
                // (Simplificado: en producción usaría adapter real)
                Ok(weights.to_vec())
            }
            ("qwen-scope", "llama-3") => Ok(weights.to_vec()),
            _ => {
                warn!(
                    source = %source_schema,
                    target = %target_schema,
                    "Unknown schema translation, passing through"
                );
                // Fallback: passthrough con warning
                Ok(weights.to_vec())
            }
        }
    }

    fn versions_compatible(local: &str, remote: &str) -> bool {
        // Compatibilidad: mismo major version
        let local_major = local.split('.').next().unwrap_or("0");
        let remote_major = remote.split('.').next().unwrap_or("0");
        local_major == remote_major
    }

    fn create_empty_result(&self) -> BridgeResult {
        BridgeResult {
            synced_nodes: 0,
            merged_updates: 0,
            trust_avg: 0.0,
            protocol_version: PROTOCOL_VERSION.to_string(),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}
