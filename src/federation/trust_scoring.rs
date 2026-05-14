//! Dynamic Trust Scoring - Scoring dinámico de confianza con resistencia Sybil
//!
//! Implementa `DynamicTrustScorer` para gestión avanzada de confianza en federación:
//! 1. Actualización dinámica de scores con decaimiento temporal
//! 2. Propagación cross-network de reputación
//! 3. Detección de clusters Sybil por ASN/IP + firma criptográfica
//! 4. Decaimiento exponencial configurable
//!
//! Fórmula: `trust = base × (1 - decay_factor^days) × consensus_weight × zkp_multiplier`
//!
//! **Feature:** `phase7-sprint2`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
// CLEANUP: removed unused import Duration
use thiserror::Error;
use tracing::{debug, warn};
// CLEANUP: removed unused import info

// ============================================================================
// Errors
// ============================================================================

/// Error específico del Dynamic Trust Scorer
#[derive(Debug, Error)]
pub enum TrustScoringError {
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: String },

    #[error("Invalid trust score: {score} (must be 0.0 - 1.0)")]
    InvalidScore { score: f32 },

    #[error("Sybil cluster detected: {cluster_id} with {count} suspicious nodes")]
    SybilDetected { cluster_id: String, count: usize },

    #[error("Node banned: {node_id} (trust {score:.4} < threshold {threshold:.4})")]
    NodeBanned { node_id: String, score: f32, threshold: f32 },

    #[error("Propagation failed: no path to {target_network}")]
    PropagationFailed { target_network: String },
}

// ============================================================================
// Node Trust Record
// ============================================================================

/// Registro de confianza por nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTrustRecord {
    /// ID único del nodo
    pub node_id: String,
    /// Score de confianza actual (0.0 - 1.0)
    pub trust_score: f32,
    /// Score base (antes de decaimiento)
    pub base_score: f32,
    /// Estado del nodo
    pub status: NodeStatus,
    /// ASN del nodo (para detección Sybil)
    pub asn: Option<String>,
    /// IP hash del nodo (para detección Sybil)
    pub ip_hash: Option<String>,
    /// Firma criptográfica del nodo (hex)
    pub crypto_signature: String,
    /// Número de sincronizaciones exitosas
    pub successful_syncs: u64,
    /// Número de sincronizaciones fallidas
    pub failed_syncs: u64,
    /// Último timestamp de actividad (epoch ms)
    pub last_activity_ms: u64,
    /// Factor de peso de consenso (0.0 - 1.0)
    pub consensus_weight: f32,
    /// Multiplicador ZKP (0.0 - 1.0)
    pub zkp_multiplier: f32,
    /// Radio de propagación cross-network
    pub propagation_radius: usize,
    /// Redes que han validado este nodo
    pub validating_networks: Vec<String>,
}

impl NodeTrustRecord {
    /// Crea nuevo registro de confianza
    pub fn new(node_id: String, crypto_signature: String) -> Self {
        Self {
            node_id,
            trust_score: 0.5, // Neutral inicial
            base_score: 0.5,
            status: NodeStatus::Active,
            asn: None,
            ip_hash: None,
            crypto_signature,
            successful_syncs: 0,
            failed_syncs: 0,
            last_activity_ms: current_timestamp_ms(),
            consensus_weight: 1.0,
            zkp_multiplier: 1.0,
            propagation_radius: 1,
            validating_networks: Vec::new(),
        }
    }

    /// Actualiza score con fórmula completa
    // FIX: E0308 - powf expects f32 argument when called on f32, cast days_since_activity to f32
    pub fn update_score(&mut self, days_since_activity: f64, decay_factor: f32) {
        let decay_component = 1.0 - (decay_factor.powf(days_since_activity as f32)); // CLEANUP: Removed redundant as f32 cast
        let new_score = self.base_score
            * (1.0 - decay_component)
            * self.consensus_weight
            * self.zkp_multiplier;

        self.trust_score = new_score.clamp(0.0, 1.0);
        self.update_status();
    }

    /// Actualiza estado basado en score
    fn update_status(&mut self) {
        self.status = match self.trust_score {
            s if s < 0.3 => NodeStatus::Banned,
            s if s < 0.5 => NodeStatus::Degraded,
            _ => NodeStatus::Active,
        };
    }

    /// Registra sincronización exitosa
    pub fn record_success(&mut self) {
        self.successful_syncs += 1;
        self.base_score = (self.base_score + 0.02).min(1.0);
        self.trust_score = self.base_score.min(1.0);
        self.last_activity_ms = current_timestamp_ms();
        self.update_status();
    }

    /// Registra sincronización fallida
    pub fn record_failure(&mut self) {
        self.failed_syncs += 1;
        self.base_score = (self.base_score - 0.05).max(0.0);
        self.trust_score = self.base_score.max(0.0);
        self.last_activity_ms = current_timestamp_ms();
        self.update_status();
    }
}

// ============================================================================
// Node Status
// ============================================================================

/// Estado del nodo en la federación
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Nodo activo y confiable
    Active,
    /// Nodo con confianza degradada (monitoreo)
    Degraded,
    /// Nodo baneado (excluido de federación)
    Banned,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Active => write!(f, "active"),
            NodeStatus::Degraded => write!(f, "degraded"),
            NodeStatus::Banned => write!(f, "banned"),
        }
    }
}

// ============================================================================
// Trust Result
// ============================================================================

/// Resultado de evaluación de confianza
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustResult {
    /// ID del nodo
    pub node_id: String,
    /// Score de confianza (0.0 - 1.0)
    pub score: f32,
    /// Estado del nodo
    pub status: NodeStatus,
    /// Radio de propagación cross-network
    pub propagation_radius: usize,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

// ============================================================================
// Sybil Cluster
// ============================================================================

/// Cluster detectado de nodos Sybil
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilCluster {
    /// ID único del cluster
    pub cluster_id: String,
    /// Nodos sospechosos en el cluster
    pub suspicious_nodes: Vec<String>,
    /// ASN compartido (si aplica)
    pub shared_asn: Option<String>,
    /// IP hash compartido (si aplica)
    pub shared_ip_hash: Option<String>,
    /// Score de sospecha (0.0 - 1.0)
    pub suspicion_score: f32,
    /// Timestamp de detección (epoch ms)
    pub detected_at_ms: u64,
}

// ============================================================================
// Trust Config
// ============================================================================

/// Configuración del Dynamic Trust Scorer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustConfig {
    /// Factor de decaimiento diario (0.0 - 1.0)
    pub decay_factor: f32,
    /// Umbral de exclusión (ban)
    pub ban_threshold: f32,
    /// Umbral de degradación
    pub degraded_threshold: f32,
    /// Peso máximo de consenso
    pub max_consensus_weight: f32,
    /// Multiplicador ZKP máximo
    pub max_zkp_multiplier: f32,
    /// Radio máximo de propagación
    pub max_propagation_radius: usize,
    /// Umbral Sybil (nodos por ASN/IP)
    pub sybil_threshold: usize,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            decay_factor: 0.995, // 0.5% decaimiento por ciclo
            ban_threshold: 0.3,
            degraded_threshold: 0.6,
            max_consensus_weight: 1.5,
            max_zkp_multiplier: 1.3,
            max_propagation_radius: 5,
            sybil_threshold: 3, // >3 nodos por ASN/IP = sospechoso
        }
    }
}

// ============================================================================
// DynamicTrustScorer
// ============================================================================

/// Scorer dinámico de confianza con resistencia Sybil
pub struct DynamicTrustScorer {
    /// Configuración
    config: TrustConfig,
    /// Registros de confianza por nodo
    records: HashMap<String, NodeTrustRecord>,
    /// Clusters Sybil detectados
    sybil_clusters: Vec<SybilCluster>,
    /// Mapa de red → nodos (para propagación)
    network_map: HashMap<String, Vec<String>>,
}

// CLEANUP: Added Default impl for DynamicTrustScorer (clippy::new_without_default)
impl Default for DynamicTrustScorer {
    fn default() -> Self {
        Self::with_config(TrustConfig::default())
    }
}

impl DynamicTrustScorer {
    /// Crea nuevo DynamicTrustScorer con configuración por defecto
    pub fn new() -> Self {
        Self::default()
    }

    /// Crea nuevo DynamicTrustScorer con configuración personalizada
    pub fn with_config(config: TrustConfig) -> Self {
        Self {
            config,
            records: HashMap::new(),
            sybil_clusters: Vec::new(),
            network_map: HashMap::new(),
        }
    }

    /// Actualiza score de confianza para un nodo
    ///
    /// Aplica la fórmula: `trust = base × (1 - decay_factor^days) × consensus_weight × zkp_multiplier`
    ///
    /// # Arguments
    ///
    /// * `node_id` - ID del nodo
    /// * `days_since_activity` - Días desde última actividad
    ///
    /// # Returns
    ///
    /// `Ok(TrustResult)` con el score actualizado
    pub fn update_score(&mut self, node_id: &str, days_since_activity: f64) -> Result<TrustResult, TrustScoringError> {
        let record = self.records.entry(node_id.to_string()).or_insert_with(|| {
            NodeTrustRecord::new(node_id.to_string(), Self::generate_default_signature(node_id))
        });

        record.update_score(days_since_activity, self.config.decay_factor);

        let result = TrustResult {
            node_id: record.node_id.clone(),
            score: record.trust_score,
            status: record.status.clone(),
            propagation_radius: record.propagation_radius,
            timestamp_ms: current_timestamp_ms(),
        };

        debug!(node = %node_id, score = result.score, status = %result.status, "Trust score updated");
        Ok(result)
    }

    /// Registra sincronización exitosa para un nodo
    pub fn record_success(&mut self, node_id: &str) -> Result<TrustResult, TrustScoringError> {
        let record = self.records.entry(node_id.to_string()).or_insert_with(|| {
            NodeTrustRecord::new(node_id.to_string(), Self::generate_default_signature(node_id))
        });

        record.record_success();

        Ok(TrustResult {
            node_id: record.node_id.clone(),
            score: record.trust_score,
            status: record.status.clone(),
            propagation_radius: record.propagation_radius,
            timestamp_ms: current_timestamp_ms(),
        })
    }

    /// Registra sincronización fallida para un nodo
    pub fn record_failure(&mut self, node_id: &str) -> Result<TrustResult, TrustScoringError> {
        let record = self.records.entry(node_id.to_string()).or_insert_with(|| {
            NodeTrustRecord::new(node_id.to_string(), Self::generate_default_signature(node_id))
        });

        record.record_failure();

        // Verificar si debe ser baneado
        if record.trust_score < self.config.ban_threshold {
            warn!(node = %node_id, score = record.trust_score, "Node banned for low trust");
        }

        Ok(TrustResult {
            node_id: record.node_id.clone(),
            score: record.trust_score,
            status: record.status.clone(),
            propagation_radius: record.propagation_radius,
            timestamp_ms: current_timestamp_ms(),
        })
    }

    /// Propaga reputación cross-network
    ///
    /// Difunde el score de confianza a redes vecinas dentro del radio de propagación.
    ///
    /// # Arguments
    ///
    /// * `node_id` - ID del nodo origen
    /// * `source_network` - Red origen
    /// * `radius` - Radio de propagación
    ///
    /// # Returns
    ///
    /// `Ok(propagated_count)` con el número de redes que recibieron la propagación
    pub fn propagate_cross_net(
        &mut self,
        node_id: &str,
        source_network: &str,
        radius: usize,
    ) -> Result<usize, TrustScoringError> {
        let record = self.records.get(node_id)
            .ok_or_else(|| TrustScoringError::NodeNotFound { node_id: node_id.to_string() })?;

        if record.status == NodeStatus::Banned {
            return Ok(0); // No propagar nodos baneados
        }

        let effective_radius = radius.min(self.config.max_propagation_radius);
        let mut propagated = 0;

        // Simular propagación a redes vecinas
        for network in &record.validating_networks {
            if network != source_network {
                propagated += 1;
                debug!(node = %node_id, network = %network, "Trust propagated to network");
            }
        }

        // Actualizar radio de propagación
        if let Some(rec) = self.records.get_mut(node_id) {
            rec.propagation_radius = effective_radius;
        }

        Ok(propagated)
    }

    /// Detecta clusters Sybil por ASN/IP + firma criptográfica
    ///
    /// Agrupa nodos por ASN e IP hash, marcando como sospechosos
    /// los grupos que exceden el umbral configurado.
    ///
    /// # Returns
    ///
    /// `Vec<SybilCluster>` con los clusters detectados
    pub fn detect_sybil(&mut self) -> Vec<SybilCluster> {
        let mut clusters = Vec::new();

        // Agrupar por ASN
        let mut asn_groups: HashMap<String, Vec<String>> = HashMap::new();
        for record in self.records.values() {
            if let Some(ref asn) = record.asn {
                asn_groups.entry(asn.clone()).or_default().push(record.node_id.clone());
            }
        }

        // Detectar clusters sospechosos por ASN
        for (asn, nodes) in asn_groups {
            if nodes.len() > self.config.sybil_threshold {
                let cluster_id = Self::generate_cluster_id(&asn, "asn");
                let suspicion_score = (nodes.len() as f32 / self.config.sybil_threshold as f32).min(1.0);

                clusters.push(SybilCluster {
                    cluster_id,
                    suspicious_nodes: nodes.clone(),
                    shared_asn: Some(asn),
                    shared_ip_hash: None,
                    suspicion_score,
                    detected_at_ms: current_timestamp_ms(),
                });

                // Degradar nodos sospechosos
                for node_id in &nodes {
                    if let Some(record) = self.records.get_mut(node_id) {
                        record.base_score = (record.base_score * 0.7).max(0.0);
                        record.update_status();
                        warn!(node = %node_id, "Node degraded due to Sybil suspicion");
                    }
                }
            }
        }

        // Agrupar por IP hash
        let mut ip_groups: HashMap<String, Vec<String>> = HashMap::new();
        for record in self.records.values() {
            if let Some(ref ip_hash) = record.ip_hash {
                ip_groups.entry(ip_hash.clone()).or_default().push(record.node_id.clone());
            }
        }

        // Detectar clusters sospechosos por IP
        for (ip_hash, nodes) in ip_groups {
            if nodes.len() > self.config.sybil_threshold {
                let cluster_id = Self::generate_cluster_id(&ip_hash, "ip");
                let suspicion_score = (nodes.len() as f32 / self.config.sybil_threshold as f32).min(1.0);

                clusters.push(SybilCluster {
                    cluster_id,
                    suspicious_nodes: nodes.clone(),
                    shared_asn: None,
                    shared_ip_hash: Some(ip_hash),
                    suspicion_score,
                    detected_at_ms: current_timestamp_ms(),
                });
            }
        }

        self.sybil_clusters = clusters.clone();
        clusters
    }

    /// Aplica decaimiento a todos los nodos
    ///
    /// Recorre todos los registros y aplica decaimiento basado en
    /// el tiempo transcurrido desde la última actividad.
    ///
    /// # Arguments
    ///
    /// * `days_elapsed` - Días transcurridos desde última actualización
    pub fn decay(&mut self, days_elapsed: f64) {
        let count = self.records.len();
        for record in self.records.values_mut() {
            record.update_score(days_elapsed, self.config.decay_factor);
        }
        debug!(count, days = days_elapsed, "Trust decay applied to all nodes");
    }

    /// Registra un nodo en una red específica
    pub fn register_node_in_network(&mut self, node_id: &str, network: &str) {
        self.network_map.entry(network.to_string()).or_default().push(node_id.to_string());

        if let Some(record) = self.records.get_mut(node_id) {
            if !record.validating_networks.contains(&network.to_string()) {
                record.validating_networks.push(network.to_string());
            }
        }
    }

    /// Obtiene el registro de confianza de un nodo
    pub fn get_record(&self, node_id: &str) -> Option<&NodeTrustRecord> {
        self.records.get(node_id)
    }

    /// Obtiene todos los nodos con un estado específico
    pub fn get_nodes_by_status(&self, status: &NodeStatus) -> Vec<&NodeTrustRecord> {
        self.records.values().filter(|r| &r.status == status).collect()
    }

    /// Obtiene los clusters Sybil detectados
    pub fn get_sybil_clusters(&self) -> &[SybilCluster] {
        &self.sybil_clusters
    }

    /// Obtiene la configuración actual
    pub fn config(&self) -> &TrustConfig {
        &self.config
    }

    /// Obtiene estadísticas generales
    pub fn stats(&self) -> TrustStats {
        let total = self.records.len();
        let active = self.get_nodes_by_status(&NodeStatus::Active).len();
        let degraded = self.get_nodes_by_status(&NodeStatus::Degraded).len();
        let banned = self.get_nodes_by_status(&NodeStatus::Banned).len();

        let avg_score = if total > 0 {
            self.records.values().map(|r| r.trust_score).sum::<f32>() / total as f32
        } else {
            0.0
        };

        TrustStats {
            total_nodes: total,
            active_nodes: active,
            degraded_nodes: degraded,
            banned_nodes: banned,
            avg_trust_score: avg_score,
            sybil_clusters: self.sybil_clusters.len(),
        }
    }

    /// Limpia clusters Sybil antiguos
    pub fn clear_sybil_clusters(&mut self) {
        self.sybil_clusters.clear();
    }

    // ------------------------------------------------------------------------
    // Private Helpers
    // ------------------------------------------------------------------------

    /// Genera firma por defecto para un nodo
    fn generate_default_signature(node_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(node_id.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Genera ID único para un cluster Sybil
    fn generate_cluster_id(identifier: &str, r#type: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", r#type, identifier));
        let result = hasher.finalize();
        format!("sybil_{}", hex::encode(&result[..8]))
    }
}

/// Obtiene timestamp actual en milisegundos epoch
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Trust Stats
// ============================================================================

/// Estadísticas de confianza
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStats {
    /// Total de nodos registrados
    pub total_nodes: usize,
    /// Nodos activos
    pub active_nodes: usize,
    /// Nodos degradados
    pub degraded_nodes: usize,
    /// Nodos baneados
    pub banned_nodes: usize,
    /// Score promedio de confianza
    pub avg_trust_score: f32,
    /// Clusters Sybil detectados
    pub sybil_clusters: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorer_creation() {
        let scorer = DynamicTrustScorer::new();
        assert_eq!(scorer.stats().total_nodes, 0);
    }

    #[test]
    fn test_record_creation() {
        let record = NodeTrustRecord::new("node_1".to_string(), "sig_abc".to_string());
        assert_eq!(record.trust_score, 0.5);
        assert_eq!(record.status, NodeStatus::Active);
    }

    #[test]
    fn test_update_score_no_decay() {
        let mut scorer = DynamicTrustScorer::new();
        let result = scorer.update_score("node_1", 0.0); // 0 days = no decay
        assert!(result.is_ok());
        assert_eq!(result.unwrap().score, 0.5);
    }

    #[test]
    fn test_update_score_with_decay() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.update_score("node_1", 10.0).unwrap(); // 10 days
        let result = scorer.get_record("node_1").unwrap();
        assert!(result.trust_score < 0.5); // Decay reduces score
    }

    #[test]
    fn test_record_success_increases_trust() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.record_success("node_1").unwrap();
        let record = scorer.get_record("node_1").unwrap();
        assert!(record.base_score > 0.5);
        assert_eq!(record.successful_syncs, 1);
    }

    #[test]
    fn test_record_failure_decreases_trust() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.record_failure("node_1").unwrap();
        let record = scorer.get_record("node_1").unwrap();
        assert!(record.base_score < 0.5);
        assert_eq!(record.failed_syncs, 1);
    }

    #[test]
    fn test_node_banned_when_trust_low() {
        let config = TrustConfig {
            ban_threshold: 0.4,
            ..Default::default()
        };
        let mut scorer = DynamicTrustScorer::with_config(config);

        // Multiple failures to drop trust below threshold
        for _ in 0..10 {
            scorer.record_failure("node_1").unwrap();
        }

        let record = scorer.get_record("node_1").unwrap();
        assert_eq!(record.status, NodeStatus::Banned);
    }

    #[test]
    fn test_sybil_detection_by_asn() {
        let mut scorer = DynamicTrustScorer::new();

        // Register 5 nodes with same ASN (threshold = 3)
        for i in 0..5 {
            let mut record = NodeTrustRecord::new(format!("node_{}", i), format!("sig_{}", i));
            record.asn = Some("ASN_12345".to_string());
            scorer.records.insert(format!("node_{}", i), record);
        }

        let clusters = scorer.detect_sybil();
        assert!(!clusters.is_empty());
        assert_eq!(clusters[0].suspicious_nodes.len(), 5);
    }

    #[test]
    fn test_sybil_detection_by_ip() {
        let mut scorer = DynamicTrustScorer::new();

        // Register 4 nodes with same IP hash (threshold = 3)
        for i in 0..4 {
            let mut record = NodeTrustRecord::new(format!("node_{}", i), format!("sig_{}", i));
            record.ip_hash = Some("ip_hash_abc".to_string());
            scorer.records.insert(format!("node_{}", i), record);
        }

        let clusters = scorer.detect_sybil();
        assert!(!clusters.is_empty());
    }

    #[test]
    fn test_decay_all_nodes() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.record_success("node_1").unwrap();
        scorer.record_success("node_2").unwrap();

        scorer.decay(1.0); // 1 day decay

        let stats = scorer.stats();
        assert_eq!(stats.total_nodes, 2);
    }

    #[test]
    fn test_propagate_cross_net() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.record_success("node_1").unwrap();
        scorer.register_node_in_network("node_1", "network_a");
        scorer.register_node_in_network("node_1", "network_b");

        let propagated = scorer.propagate_cross_net("node_1", "network_a", 2).unwrap();
        assert_eq!(propagated, 1); // Propagates to network_b only
    }

    #[test]
    fn test_propagate_banned_node_returns_zero() {
        let config = TrustConfig {
            ban_threshold: 1.0, // Ban immediately
            ..Default::default()
        };
        let mut scorer = DynamicTrustScorer::with_config(config);

        let mut record = NodeTrustRecord::new("node_1".to_string(), "sig".to_string());
        record.status = NodeStatus::Banned;
        scorer.records.insert("node_1".to_string(), record);

        let propagated = scorer.propagate_cross_net("node_1", "network_a", 2).unwrap();
        assert_eq!(propagated, 0);
    }

    #[test]
    fn test_stats_calculation() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.record_success("node_1").unwrap();
        scorer.record_success("node_2").unwrap();
        scorer.record_success("node_3").unwrap();

        let stats = scorer.stats();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.active_nodes, 3);
        assert!(stats.avg_trust_score > 0.5);
    }

    #[test]
    fn test_get_nodes_by_status() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.record_success("node_1").unwrap();
        scorer.record_success("node_2").unwrap();

        let active = scorer.get_nodes_by_status(&NodeStatus::Active);
        assert_eq!(active.len(), 2);

        let banned = scorer.get_nodes_by_status(&NodeStatus::Banned);
        assert_eq!(banned.len(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = TrustConfig::default();
        assert_eq!(config.decay_factor, 0.995);
        assert_eq!(config.ban_threshold, 0.3);
        assert_eq!(config.sybil_threshold, 3);
    }

    #[test]
    fn test_trust_result_structure() {
        let result = TrustResult {
            node_id: "node_1".to_string(),
            score: 0.75,
            status: NodeStatus::Active,
            propagation_radius: 2,
            timestamp_ms: 1000,
        };
        assert_eq!(result.score, 0.75);
        assert_eq!(result.status, NodeStatus::Active);
    }

    #[test]
    fn test_node_status_display() {
        assert_eq!(format!("{}", NodeStatus::Active), "active");
        assert_eq!(format!("{}", NodeStatus::Degraded), "degraded");
        assert_eq!(format!("{}", NodeStatus::Banned), "banned");
    }

    #[test]
    fn test_clear_sybil_clusters() {
        let mut scorer = DynamicTrustScorer::new();
        scorer.sybil_clusters.push(SybilCluster {
            cluster_id: "test".to_string(),
            suspicious_nodes: vec!["node_1".to_string()],
            shared_asn: None,
            shared_ip_hash: None,
            suspicion_score: 0.8,
            detected_at_ms: 1000,
        });

        scorer.clear_sybil_clusters();
        assert!(scorer.get_sybil_clusters().is_empty());
    }
}
