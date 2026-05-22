//! Trust Sync — Sincronización de confianza ponderada por reputación criptográfica
//!
//! LP-32: Cross-Model Federation Scaling
//! Integración con ZKP bridge validation y resistencia Sybil.
//!
//! Características:
//! - Actualizaciones de confianza ponderadas por reputación + cumplimiento SLO
//! - Decaimiento temporal para nodos inactivos
//! - Propagación de confianza entre redes federadas
//! - Detección Sybil basada en firmas criptográficas duplicadas
//! - Validación de pruebas ZKP para boosts de confianza
//!
//! Todos los módulos de Sprint 4 están protegidos con `#[cfg(feature = "v1.1-sprint4")]`.

#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::info;

#[cfg(all(feature = "v1.1-sprint4", doc))]
use ed25519_dalek::{SigningKey, VerifyingKey};

/// Errores del módulo Trust Sync.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Error, Debug)]
pub enum TrustSyncError {
    #[error("Nodo no registrado: {0}")]
    NodeNotRegistered(String),

    #[error("Firma criptográfica inválida para nodo: {0}")]
    InvalidCryptoSignature(String),

    #[error("Confianza fuera de rango: {0}")]
    TrustOutOfRange(f32),

    #[error("Clúster Sybil detectado: {0}")]
    SybilClusterDetected(String),

    #[error("ZKP proof inválida: {0}")]
    InvalidZKPProof(String),

    #[error("Propagación bloqueada por gobernanza: {0}")]
    GovernanceBlocked(String),

    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

/// Registro de confianza por nodo con reputación criptográfica.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustNodeRecord {
    /// Identificador único del nodo.
    pub node_id: String,
    /// Firma criptográfica ed25519 del nodo.
    pub crypto_signature: String,
    /// Clave de verificación pública (hex).
    pub public_key_hex: String,
    /// Puntuación de confianza actual [0.0, 1.0].
    pub trust_score: f32,
    /// Reputación criptográfica acumulada [0.0, 1.0].
    pub crypto_reputation: f32,
    /// Historial de cumplimiento SLO (éxitos / totales).
    pub slo_compliance_rate: f32,
    /// Número de verificaciones ZKP exitosas.
    pub zkp_verifications_passed: u64,
    /// Número total de verificaciones ZKP intentadas.
    pub zkp_verifications_total: u64,
    /// Timestamp de última actividad (ms).
    pub last_activity_ms: u64,
    /// Timestamp de registro (ms).
    pub registered_at_ms: u64,
    /// Red federada a la que pertenece.
    pub network: String,
    /// Estado actual del nodo.
    pub status: NodeStatus,
}

#[cfg(feature = "v1.1-sprint4")]
impl TrustNodeRecord {
    /// Crear nuevo registro de nodo.
    pub fn new(
        node_id: String,
        crypto_signature: String,
        public_key_hex: String,
        network: String,
    ) -> Self {
        let now = current_timestamp_ms();
        Self {
            node_id,
            crypto_signature,
            public_key_hex,
            trust_score: 0.5,
            crypto_reputation: 0.5,
            slo_compliance_rate: 1.0,
            zkp_verifications_passed: 0,
            zkp_verifications_total: 0,
            last_activity_ms: now,
            registered_at_ms: now,
            network,
            status: NodeStatus::Active,
        }
    }

    /// Actualizar puntuación de confianza con ponderación por reputación + SLO.
    pub fn update_trust(&mut self, slo_success: bool, decay_factor: f32) {
        let now = current_timestamp_ms();
        self.last_activity_ms = now;

        // Actualizar cumplimiento SLO
        if slo_success {
            self.slo_compliance_rate = (self.slo_compliance_rate * 0.9) + 0.1; // Smooth hacia 1.0
        } else {
            self.slo_compliance_rate *= 0.9; // Smooth hacia 0.0
        }

        // Calcular confianza ponderada
        let weighted_trust = self.compute_weighted_trust();

        // Aplicar decaimiento temporal
        let days_inactive = (now.saturating_sub(self.last_activity_ms)) as f64 / 86_400_000.0;
        let decay = (decay_factor * days_inactive as f32).min(1.0);
        self.trust_score = (weighted_trust * (1.0 - decay)).max(0.0);

        // Actualizar estado
        self.update_status();
    }

    /// Registrar verificación ZKP exitosa.
    pub fn record_zkp_success(&mut self) {
        self.zkp_verifications_passed += 1;
        self.zkp_verifications_total += 1;
        self.crypto_reputation = (self.crypto_reputation * 0.95) + 0.05;
        self.update_status();
    }

    /// Registrar verificación ZKP fallida.
    pub fn record_zkp_failure(&mut self) {
        self.zkp_verifications_total += 1;
        self.crypto_reputation = (self.crypto_reputation * 0.95) - 0.05;
        self.crypto_reputation = self.crypto_reputation.max(0.0);
        self.update_status();
    }

    /// Calcular confianza ponderada por reputación criptográfica + SLO.
    fn compute_weighted_trust(&self) -> f32 {
        let crypto_weight = 0.4;
        let slo_weight = 0.4;
        let base_weight = 0.2;

        let base_trust = self.trust_score;
        let weighted = base_trust * base_weight
            + self.crypto_reputation * crypto_weight
            + self.slo_compliance_rate * slo_weight;

        weighted.clamp(0.0, 1.0)
    }

    /// Aplicar decaimiento por inactividad.
    pub fn apply_decay(&mut self, days_elapsed: f64, decay_rate: f32) {
        let decay = (decay_rate * days_elapsed as f32).min(1.0);
        self.trust_score *= 1.0 - decay;
        self.crypto_reputation *= 1.0 - decay * 0.5; // Decaimiento más lento para reputación
        self.trust_score = self.trust_score.max(0.0);
        self.crypto_reputation = self.crypto_reputation.max(0.0);
        self.update_status();
    }

    /// Actualizar estado basado en puntuación de confianza.
    fn update_status(&mut self) {
        self.status = match self.trust_score {
            s if s >= 0.8 => NodeStatus::Trusted,
            s if s >= 0.5 => NodeStatus::Active,
            s if s >= 0.3 => NodeStatus::Suspicious,
            _ => NodeStatus::Banned,
        };
    }

    /// Verificar si la firma criptográfica es válida (simulado).
    pub fn verify_crypto_signature(&self) -> bool {
        // En producción, se verificaría contra ed25519-dalek
        !self.crypto_signature.is_empty() && !self.public_key_hex.is_empty()
    }
}

/// Estado del nodo en la federación.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Trusted,
    Active,
    Suspicious,
    Banned,
}

#[cfg(feature = "v1.1-sprint4")]
impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Trusted => write!(f, "trusted"),
            NodeStatus::Active => write!(f, "active"),
            NodeStatus::Suspicious => write!(f, "suspicious"),
            NodeStatus::Banned => write!(f, "banned"),
        }
    }
}

/// Resultado de sincronización de confianza.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSyncResult {
    pub node_id: String,
    pub trust_score: f32,
    pub crypto_reputation: f32,
    pub slo_compliance: f32,
    pub status: NodeStatus,
    pub synced_at_ms: u64,
    pub propagation_count: usize,
}

#[cfg(feature = "v1.1-sprint4")]
impl TrustSyncResult {
    pub fn new(node_id: String, record: &TrustNodeRecord, propagation_count: usize) -> Self {
        Self {
            node_id,
            trust_score: record.trust_score,
            crypto_reputation: record.crypto_reputation,
            slo_compliance: record.slo_compliance_rate,
            status: record.status.clone(),
            synced_at_ms: current_timestamp_ms(),
            propagation_count,
        }
    }
}

/// Clúster Sybil detectado.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilCluster {
    pub cluster_id: String,
    pub node_ids: Vec<String>,
    pub detection_reason: String,
    pub detected_at_ms: u64,
    pub r#type: String, // "signature", "ip", "asn"
}

/// Configuración de Trust Sync.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSyncConfig {
    /// Tasa de decaimiento diario (0.01 = 1% por día).
    pub decay_rate: f32,
    /// Umbral mínimo de confianza para participar.
    pub min_trust_threshold: f32,
    /// Umbral de reputación criptográfica para boosts ZKP.
    pub crypto_reputation_threshold: f32,
    /// Máximo de propagaciones por ciclo.
    pub max_propagation_per_cycle: usize,
    /// Ventana de detección Sybil (ms).
    pub sybil_detection_window_ms: u64,
    /// Habilitar validación ZKP para boosts.
    pub enable_zkp_boost: bool,
    /// Factor de boost ZKP (0.05 = +5% por verificación exitosa).
    pub zkp_boost_factor: f32,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for TrustSyncConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.01,
            min_trust_threshold: 0.3,
            crypto_reputation_threshold: 0.7,
            max_propagation_per_cycle: 100,
            sybil_detection_window_ms: 3_600_000, // 1 hora
            enable_zkp_boost: true,
            zkp_boost_factor: 0.05,
        }
    }
}

/// Estadísticas de Trust Sync.
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSyncStats {
    pub total_nodes: usize,
    pub trusted_nodes: usize,
    pub active_nodes: usize,
    pub suspicious_nodes: usize,
    pub banned_nodes: usize,
    pub avg_trust_score: f32,
    pub avg_crypto_reputation: f32,
    pub avg_slo_compliance: f32,
    pub sybil_clusters_detected: usize,
    pub zkp_boosts_applied: u64,
    pub propagations_this_cycle: usize,
    pub last_sync_ms: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for TrustSyncStats {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            trusted_nodes: 0,
            active_nodes: 0,
            suspicious_nodes: 0,
            banned_nodes: 0,
            avg_trust_score: 0.0,
            avg_crypto_reputation: 0.0,
            avg_slo_compliance: 0.0,
            sybil_clusters_detected: 0,
            zkp_boosts_applied: 0,
            propagations_this_cycle: 0,
            last_sync_ms: 0,
        }
    }
}

/// Motor de sincronización de confianza.
#[cfg(feature = "v1.1-sprint4")]
pub struct TrustSyncEngine {
    pub config: TrustSyncConfig,
    pub records: std::collections::HashMap<String, TrustNodeRecord>,
    pub sybil_clusters: Vec<SybilCluster>,
    pub stats: TrustSyncStats,
    pub propagation_log: Vec<TrustSyncResult>,
}

#[cfg(feature = "v1.1-sprint4")]
impl TrustSyncEngine {
    /// Crear motor con configuración por defecto.
    pub fn new() -> Self {
        Self {
            config: TrustSyncConfig::default(),
            records: std::collections::HashMap::new(),
            sybil_clusters: Vec::new(),
            stats: TrustSyncStats::default(),
            propagation_log: Vec::new(),
        }
    }

    /// Crear motor con configuración personalizada.
    pub fn with_config(config: TrustSyncConfig) -> Self {
        Self {
            config,
            records: std::collections::HashMap::new(),
            sybil_clusters: Vec::new(),
            stats: TrustSyncStats::default(),
            propagation_log: Vec::new(),
        }
    }

    /// Registrar nodo en la federación.
    pub fn register_node(
        &mut self,
        record: TrustNodeRecord,
    ) -> Result<TrustSyncResult, TrustSyncError> {
        let node_id = record.node_id.clone();

        // Verificar firma criptográfica
        if !record.verify_crypto_signature() {
            return Err(TrustSyncError::InvalidCryptoSignature(node_id.clone()));
        }

        // Verificar que no sea duplicado de firma (Sybil)
        if self.signature_exists(&record.crypto_signature) {
            return Err(TrustSyncError::SybilClusterDetected(format!(
                "Firma duplicada para nodo {}",
                node_id
            )));
        }

        self.records.insert(node_id.clone(), record);
        self.update_stats();

        info!("Nodo registrado: {}", node_id);
        Ok(TrustSyncResult::new(
            node_id.clone(),
            self.records.get(&node_id).unwrap(),
            0,
        ))
    }

    /// Actualizar confianza de nodo con resultado SLO.
    pub fn update_trust(
        &mut self,
        node_id: &str,
        slo_success: bool,
    ) -> Result<TrustSyncResult, TrustSyncError> {
        let decay_rate = self.config.decay_rate;
        if let Some(record) = self.records.get_mut(node_id) {
            record.update_trust(slo_success, decay_rate);
        } else {
            return Err(TrustSyncError::NodeNotRegistered(node_id.to_string()));
        }
        self.update_stats();

        let record = self.records.get(node_id).unwrap();
        let result = TrustSyncResult::new(node_id.to_string(), record, 0);
        Ok(result)
    }

    /// Aplicar boost de confianza por verificación ZKP exitosa.
    pub fn apply_zkp_boost(&mut self, node_id: &str) -> Result<TrustSyncResult, TrustSyncError> {
        if !self.config.enable_zkp_boost {
            return Err(TrustSyncError::GovernanceBlocked(
                "ZKP boost deshabilitado".to_string(),
            ));
        }

        let crypto_threshold = self.config.crypto_reputation_threshold;
        let zkp_boost = self.config.zkp_boost_factor;
        if let Some(record) = self.records.get_mut(node_id) {
            record.record_zkp_success();

            // Aplicar boost adicional si reputación criptográfica es alta
            if record.crypto_reputation >= crypto_threshold {
                record.trust_score = (record.trust_score + zkp_boost).min(1.0);
                self.stats.zkp_boosts_applied += 1;
            }
        } else {
            return Err(TrustSyncError::NodeNotRegistered(node_id.to_string()));
        }

        self.update_stats();
        let record = self.records.get(node_id).unwrap();
        Ok(TrustSyncResult::new(node_id.to_string(), record, 0))
    }

    /// Registrar fallo de verificación ZKP.
    pub fn record_zkp_failure(&mut self, node_id: &str) -> Result<TrustSyncResult, TrustSyncError> {
        if let Some(record) = self.records.get_mut(node_id) {
            record.record_zkp_failure();
        } else {
            return Err(TrustSyncError::NodeNotRegistered(node_id.to_string()));
        }
        self.update_stats();

        let record = self.records.get(node_id).unwrap();
        Ok(TrustSyncResult::new(node_id.to_string(), record, 0))
    }

    /// Ejecutar ciclo de sincronización completo.
    pub fn sync_cycle(&mut self) -> Vec<TrustSyncResult> {
        let now = current_timestamp_ms();
        let mut results = Vec::new();

        // Aplicar decaimiento a todos los nodos
        for record in self.records.values_mut() {
            let days_inactive = (now.saturating_sub(record.last_activity_ms)) as f64 / 86_400_000.0;
            if days_inactive > 0.0 {
                record.apply_decay(days_inactive, self.config.decay_rate);
            }
        }

        // Detectar clústers Sybil
        self.detect_sybil_clusters();

        // Propagar confianza entre redes
        let propagated = self.propagate_trust();
        self.stats.propagations_this_cycle = propagated;

        // Actualizar estadísticas
        self.update_stats();
        self.stats.last_sync_ms = now;

        // Generar resultados
        for (node_id, record) in &self.records {
            results.push(TrustSyncResult::new(node_id.clone(), record, propagated));
        }

        info!(
            "Ciclo de sincronización completado: {} nodos, {} propagaciones",
            results.len(),
            propagated
        );

        results
    }

    /// Obtener estadísticas actuales.
    pub fn get_stats(&self) -> TrustSyncStats {
        self.stats.clone()
    }

    /// Obtener registro de nodo.
    pub fn get_node(&self, node_id: &str) -> Option<&TrustNodeRecord> {
        self.records.get(node_id)
    }

    /// Obtener nodos por estado.
    pub fn get_nodes_by_status(&self, status: &NodeStatus) -> Vec<&TrustNodeRecord> {
        self.records
            .values()
            .filter(|r| &r.status == status)
            .collect()
    }

    /// Obtener clústers Sybil detectados.
    pub fn get_sybil_clusters(&self) -> &[SybilCluster] {
        &self.sybil_clusters
    }

    /// Limpiar clústers Sybil antiguos.
    pub fn clear_old_sybil_clusters(&mut self) {
        let cutoff = current_timestamp_ms().saturating_sub(self.config.sybil_detection_window_ms);
        self.sybil_clusters.retain(|c| c.detected_at_ms > cutoff);
    }

    /// Verificar si una firma ya existe en los registros.
    fn signature_exists(&self, signature: &str) -> bool {
        self.records
            .values()
            .any(|r| r.crypto_signature == signature)
    }

    /// Detectar clústers Sybil basados en firmas duplicadas.
    fn detect_sybil_clusters(&mut self) {
        let mut signature_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for record in self.records.values() {
            signature_map
                .entry(record.crypto_signature.clone())
                .or_default()
                .push(record.node_id.clone());
        }

        for (signature, node_ids) in signature_map {
            if node_ids.len() > 1 {
                let cluster = SybilCluster {
                    cluster_id: generate_cluster_id(&signature, "signature"),
                    node_ids,
                    detection_reason: format!(
                        "Firma duplicada detectada en {} nodos",
                        signature.len()
                    ),
                    detected_at_ms: current_timestamp_ms(),
                    r#type: "signature".to_string(),
                };
                self.sybil_clusters.push(cluster);
                self.stats.sybil_clusters_detected += 1;
            }
        }
    }

    /// Propagar confianza entre redes federadas.
    fn propagate_trust(&mut self) -> usize {
        let mut count = 0;
        let max_prop = self.config.max_propagation_per_cycle;

        // Agrupar nodos por red
        let mut network_nodes: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (node_id, record) in &self.records {
            network_nodes
                .entry(record.network.clone())
                .or_default()
                .push(node_id.clone());
        }

        // Propagar confianza promedio entre redes
        let network_avg_trust: std::collections::HashMap<String, f32> = network_nodes
            .iter()
            .filter_map(|(network, node_ids)| {
                let records: Vec<&TrustNodeRecord> = node_ids
                    .iter()
                    .filter_map(|id| self.records.get(id))
                    .collect();
                if records.is_empty() {
                    return None;
                }
                let avg: f32 =
                    records.iter().map(|r| r.trust_score).sum::<f32>() / records.len() as f32;
                Some((network.clone(), avg))
            })
            .collect();

        // Aplicar influencia cruzada (suave)
        for record in self.records.values_mut() {
            if count >= max_prop {
                break;
            }
            if let Some(&avg_trust) = network_avg_trust.get(&record.network) {
                // Influencia suave de la red (10% del promedio de la red)
                record.trust_score = record.trust_score * 0.9 + avg_trust * 0.1;
                count += 1;
            }
        }

        count
    }

    /// Actualizar estadísticas.
    fn update_stats(&mut self) {
        let records: Vec<&TrustNodeRecord> = self.records.values().collect();
        let total = records.len();

        if total == 0 {
            self.stats = TrustSyncStats::default();
            return;
        }

        self.stats.total_nodes = total;
        self.stats.trusted_nodes = records
            .iter()
            .filter(|r| r.status == NodeStatus::Trusted)
            .count();
        self.stats.active_nodes = records
            .iter()
            .filter(|r| r.status == NodeStatus::Active)
            .count();
        self.stats.suspicious_nodes = records
            .iter()
            .filter(|r| r.status == NodeStatus::Suspicious)
            .count();
        self.stats.banned_nodes = records
            .iter()
            .filter(|r| r.status == NodeStatus::Banned)
            .count();

        let sum_trust: f32 = records.iter().map(|r| r.trust_score).sum();
        let sum_crypto: f32 = records.iter().map(|r| r.crypto_reputation).sum();
        let sum_slo: f32 = records.iter().map(|r| r.slo_compliance_rate).sum();

        self.stats.avg_trust_score = sum_trust / total as f32;
        self.stats.avg_crypto_reputation = sum_crypto / total as f32;
        self.stats.avg_slo_compliance = sum_slo / total as f32;
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for TrustSyncEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Generar ID de clúster Sybil.
#[cfg(feature = "v1.1-sprint4")]
fn generate_cluster_id(signature: &str, r#type: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", signature, r#type));
    let result = hasher.finalize();
    format!("sybil_{}", hex::encode(&result[..8]))
}

/// Obtener timestamp actual en milisegundos.
#[cfg(feature = "v1.1-sprint4")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "v1.1-sprint4"))]
mod tests {
    use super::*;

    fn make_signature(node_id: &str) -> String {
        format!("sig_{}_{}", node_id, node_id.len())
    }

    fn make_public_key(node_id: &str) -> String {
        format!("pk_{}_{}", node_id, node_id.len())
    }

    fn make_record(node_id: &str, network: &str) -> TrustNodeRecord {
        TrustNodeRecord::new(
            node_id.to_string(),
            make_signature(node_id),
            make_public_key(node_id),
            network.to_string(),
        )
    }

    #[test]
    fn test_engine_creation() {
        let engine = TrustSyncEngine::new();
        assert_eq!(engine.stats.total_nodes, 0);
        assert_eq!(engine.config.decay_rate, 0.01);
    }

    #[test]
    fn test_engine_with_config() {
        let config = TrustSyncConfig {
            decay_rate: 0.05,
            enable_zkp_boost: false,
            ..TrustSyncConfig::default()
        };
        let engine = TrustSyncEngine::with_config(config);
        assert_eq!(engine.config.decay_rate, 0.05);
        assert!(!engine.config.enable_zkp_boost);
    }

    #[test]
    fn test_register_node() {
        let mut engine = TrustSyncEngine::new();
        let record = make_record("node1", "net_a");

        let result = engine.register_node(record).unwrap();
        assert_eq!(result.node_id, "node1");
        assert_eq!(engine.stats.total_nodes, 1);
    }

    #[test]
    fn test_register_duplicate_signature() {
        let mut engine = TrustSyncEngine::new();
        let mut record1 = make_record("node1", "net_a");
        let mut record2 = make_record("node2", "net_a");

        // Ambas con la misma firma
        record2.crypto_signature = record1.crypto_signature.clone();

        engine.register_node(record1).unwrap();
        let result = engine.register_node(record2);

        assert!(result.is_err());
        match result.unwrap_err() {
            TrustSyncError::SybilClusterDetected(_) => {}
            other => panic!("Expected SybilClusterDetected, got: {}", other),
        }
    }

    #[test]
    fn test_update_trust_success() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();

        let result = engine.update_trust("node1", true).unwrap();
        assert!(result.trust_score > 0.5);
        assert!(result.slo_compliance > 0.9);
    }

    #[test]
    fn test_update_trust_failure() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();

        let result = engine.update_trust("node1", false).unwrap();
        assert!(result.slo_compliance < 1.0);
    }

    #[test]
    fn test_update_trust_unknown_node() {
        let mut engine = TrustSyncEngine::new();
        let result = engine.update_trust("unknown", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_zkp_boost() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();

        let result = engine.apply_zkp_boost("node1").unwrap();
        assert_eq!(result.crypto_reputation, 0.5 * 0.95 + 0.05);
        assert!(engine.get_node("node1").unwrap().zkp_verifications_passed == 1);
    }

    #[test]
    fn test_zkp_boost_disabled() {
        let config = TrustSyncConfig {
            enable_zkp_boost: false,
            ..TrustSyncConfig::default()
        };
        let mut engine = TrustSyncEngine::with_config(config);
        engine.register_node(make_record("node1", "net_a")).unwrap();

        let result = engine.apply_zkp_boost("node1");
        assert!(result.is_err());
    }

    #[test]
    fn test_zkp_failure() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();

        let result = engine.record_zkp_failure("node1").unwrap();
        assert!(result.crypto_reputation < 0.5);
        assert!(engine.get_node("node1").unwrap().zkp_verifications_total == 1);
    }

    #[test]
    fn test_sync_cycle() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();
        engine.register_node(make_record("node2", "net_a")).unwrap();
        engine.register_node(make_record("node3", "net_b")).unwrap();

        let results = engine.sync_cycle();
        assert_eq!(results.len(), 3);
        assert!(engine.stats.propagations_this_cycle > 0);
    }

    #[test]
    fn test_get_nodes_by_status() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();
        engine.register_node(make_record("node2", "net_a")).unwrap();

        let active = engine.get_nodes_by_status(&NodeStatus::Active);
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_decay_application() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();

        // Simular 10 días de inactividad
        let record = engine.records.get_mut("node1").unwrap();
        record.last_activity_ms = current_timestamp_ms().saturating_sub(864_000_000 * 10);

        engine.sync_cycle();
        let record = engine.get_node("node1").unwrap();
        assert!(record.trust_score < 0.5);
    }

    #[test]
    fn test_config_default() {
        let config = TrustSyncConfig::default();
        assert_eq!(config.decay_rate, 0.01);
        assert_eq!(config.min_trust_threshold, 0.3);
        assert!(config.enable_zkp_boost);
    }

    #[test]
    fn test_stats_default() {
        let stats = TrustSyncStats::default();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.avg_trust_score, 0.0);
    }

    #[test]
    fn test_engine_default() {
        let engine = TrustSyncEngine::default();
        assert_eq!(engine.stats.total_nodes, 0);
    }

    #[test]
    fn test_node_status_display() {
        assert_eq!(NodeStatus::Trusted.to_string(), "trusted");
        assert_eq!(NodeStatus::Active.to_string(), "active");
        assert_eq!(NodeStatus::Suspicious.to_string(), "suspicious");
        assert_eq!(NodeStatus::Banned.to_string(), "banned");
    }

    #[test]
    fn test_trust_sync_result() {
        let record = make_record("node1", "net_a");
        let result = TrustSyncResult::new("node1".to_string(), &record, 5);

        assert_eq!(result.node_id, "node1");
        assert_eq!(result.propagation_count, 5);
        assert_eq!(result.status, NodeStatus::Active);
    }

    #[test]
    fn test_multiple_sync_cycles() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();

        engine.sync_cycle();
        let stats1 = engine.get_stats();
        engine.sync_cycle();
        let stats2 = engine.get_stats();

        assert!(stats2.last_sync_ms >= stats1.last_sync_ms);
    }

    #[test]
    fn test_propagation_between_networks() {
        let mut engine = TrustSyncEngine::new();
        engine.register_node(make_record("node1", "net_a")).unwrap();
        engine.register_node(make_record("node2", "net_b")).unwrap();

        // Dar alta confianza a node1
        engine.update_trust("node1", true).unwrap();

        let results = engine.sync_cycle();
        let net_a_results: Vec<_> = results.iter().filter(|r| r.node_id == "node1").collect();
        let net_b_results: Vec<_> = results.iter().filter(|r| r.node_id == "node2").collect();

        assert_eq!(net_a_results.len(), 1);
        assert_eq!(net_b_results.len(), 1);
    }

    #[test]
    fn test_clear_old_sybil_clusters() {
        let mut engine = TrustSyncEngine::new();
        engine.sybil_clusters.push(SybilCluster {
            cluster_id: "old".to_string(),
            node_ids: vec!["n1".to_string()],
            detection_reason: "test".to_string(),
            detected_at_ms: current_timestamp_ms().saturating_sub(7_200_000), // 2 horas
            r#type: "signature".to_string(),
        });

        engine.clear_old_sybil_clusters();
        assert_eq!(engine.sybil_clusters.len(), 0); // Expirado
    }

    #[test]
    fn test_record_verify_crypto_signature() {
        let record = make_record("node1", "net_a");
        assert!(record.verify_crypto_signature());

        let mut bad_record = record.clone();
        bad_record.crypto_signature = "".to_string();
        assert!(!bad_record.verify_crypto_signature());
    }

    #[test]
    fn test_compute_weighted_trust() {
        let mut record = make_record("node1", "net_a");
        record.trust_score = 0.8;
        record.crypto_reputation = 0.9;
        record.slo_compliance_rate = 0.7;

        // weighted = 0.8*0.2 + 0.9*0.4 + 0.7*0.4 = 0.16 + 0.36 + 0.28 = 0.8
        let weighted = record.compute_weighted_trust();
        assert!((weighted - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_node_record_apply_decay() {
        let mut record = make_record("node1", "net_a");
        record.trust_score = 0.8;
        record.crypto_reputation = 0.9;

        record.apply_decay(5.0, 0.02); // 5 días, 2% por día = 10% decay

        assert!(record.trust_score < 0.8);
        assert!(record.crypto_reputation < 0.9);
    }

    #[test]
    fn test_status_transitions() {
        let mut record = make_record("node1", "net_a");

        record.trust_score = 0.9;
        record.update_status();
        assert_eq!(record.status, NodeStatus::Trusted);

        record.trust_score = 0.6;
        record.update_status();
        assert_eq!(record.status, NodeStatus::Active);

        record.trust_score = 0.4;
        record.update_status();
        assert_eq!(record.status, NodeStatus::Suspicious);

        record.trust_score = 0.2;
        record.update_status();
        assert_eq!(record.status, NodeStatus::Banned);
    }
}
