//! Consensus Validator - Validación asíncrona por batches + Merkle tree + umbral de confianza
//!
//! Recibe batches de `FeatureBatch` de múltiples nodos, agrupa por
//! `layer_id` + `time_window`, calcula raíz Merkle y verifica coherencia
//! entre nodos independientes.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use super::merkle::FeatureBatchHash;
use crate::p2p::protocol::SparseFeature;
use crate::zkp::verifier::{CryptoReputation, VerificationResult, ZKPVerifier};

// ============================================================================
// Consensus Types
// ============================================================================

/// Tipo de señal de consenso
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalType {
    /// Steering signal (ajuste ligero síncrono)
    Steering,
    /// Ajuste de peso (acumulado, requiere validación completa)
    WeightAdjust,
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalType::Steering => write!(f, "steering"),
            SignalType::WeightAdjust => write!(f, "weight_adjust"),
        }
    }
}

/// Evento de consenso emitido por el validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusEvent {
    /// Si el batch fue aprobado por consenso
    pub approved: bool,
    /// Raíz Merkle del batch aprobado
    pub merkle_root: String,
    /// Tipo de señal
    pub signal_type: SignalType,
    /// Número de votos a favor
    pub votes_for: usize,
    /// Número de votos en contra
    pub votes_against: usize,
    /// Umbral requerido
    pub threshold: usize,
    /// Layer ID asociado
    pub layer_id: u32,
    /// Time window del batch
    pub time_window: u64,
    /// Timestamp del evento (Unix epoch ms)
    pub timestamp: u64,
    /// Razón de rechazo (si aplica)
    pub rejection_reason: Option<String>,
}

/// Vote de un nodo individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    /// Peer ID del nodo votante
    pub voter_peer_id: String,
    /// Raíz Merkle reportada por el nodo
    pub merkle_root: String,
    /// Layer ID
    pub layer_id: u32,
    /// Time window
    pub time_window: u64,
    /// Confianza del voto (0.0 - 1.0)
    pub confidence: f64,
    /// MIGRATION: Instant doesn't implement Serialize/Deserialize/Default, skip both directions
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

/// Batch de features pendiente de validación
#[derive(Debug, Clone)]
pub struct PendingBatch {
    /// ID único del batch
    pub batch_id: String,
    /// Layer ID
    pub layer_id: u32,
    /// Time window (Unix epoch ms)
    pub time_window: u64,
    /// Votos recibidos
    pub votes: Vec<ConsensusVote>,
    /// Raíz Merkle más votada
    pub leading_root: Option<String>,
    /// Hash del batch
    pub batch_hash: Option<FeatureBatchHash>,
    /// Estado del batch
    pub state: BatchState,
    /// Creado en
    pub created_at: Instant,
    /// Timeout en segundos
    pub timeout_secs: u64,
}

/// Estado de un batch pendiente
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BatchState {
    /// Esperando votos
    #[default]
    Collecting,
    /// En proceso de validación
    Validating,
    /// Aprobado por consenso
    Approved,
    /// Rechazado por falta de consenso
    Rejected,
    /// Expirado por timeout
    TimedOut,
}

// ============================================================================
// Consensus Validator
// ============================================================================

/// Validador de consenso distribuido
pub struct ConsensusValidator {
    /// Batches pendientes de validación
    pending_batches: RwLock<HashMap<String, PendingBatch>>,
    /// Umbral mínimo de votos para consenso
    min_votes: usize,
    /// Umbral de acuerdo (ratio de votos a favor / total votos)
    agreement_threshold: f64,
    /// Time window en milisegundos
    time_window_ms: u64,
    /// Timeout para batches en segundos
    batch_timeout_secs: u64,
    /// Eventos de consenso emitidos
    events: RwLock<Vec<ConsensusEvent>>,
    /// Tabla de reputación de nodos
    node_reputation: RwLock<HashMap<String, NodeReputation>>,
    /// Verificador ZKP (Fase 3)
    zkp_verifier: ZKPVerifier,
    /// Reputación criptográfica por nodo (Fase 3)
    crypto_reputation: RwLock<HashMap<String, CryptoReputation>>,
}

/// Reputación de un nodo
#[derive(Debug, Clone)]
pub struct NodeReputation {
    /// Peer ID
    pub peer_id: String,
    /// Votos consistentes (acuerdan con consenso)
    pub consistent_votes: usize,
    /// Votos inconsistentes (discrepan con consenso)
    pub inconsistent_votes: usize,
    /// Score de reputación (0.0 - 1.0)
    pub reputation_score: f64,
    /// Si el nodo está baneado
    pub is_banned: bool,
}

impl NodeReputation {
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            consistent_votes: 0,
            inconsistent_votes: 0,
            reputation_score: 1.0,
            is_banned: false,
        }
    }

    /// Actualizar reputación tras consenso
    pub fn update(&mut self, was_consistent: bool) {
        if was_consistent {
            self.consistent_votes += 1;
        } else {
            self.inconsistent_votes += 1;
        }

        let total = self.consistent_votes + self.inconsistent_votes;
        if total > 0 {
            self.reputation_score = self.consistent_votes as f64 / total as f64;
        }

        // Banear si reputación < 0.3
        if self.reputation_score < 0.3 && total >= 5 {
            self.is_banned = true;
            warn!(
                "Nodo {} baneado por baja reputación: {:.3}",
                self.peer_id, self.reputation_score
            );
        }
    }
}

impl ConsensusValidator {
    /// Crear nuevo validador de consenso
    pub fn new() -> Self {
        Self {
            pending_batches: RwLock::new(HashMap::new()),
            min_votes: 3,
            agreement_threshold: 0.67, // 2/3 majority
            time_window_ms: 5000,      // 5 segundos
            batch_timeout_secs: 30,
            events: RwLock::new(Vec::new()),
            node_reputation: RwLock::new(HashMap::new()),
            zkp_verifier: ZKPVerifier::new(Some(0.6)),
            crypto_reputation: RwLock::new(HashMap::new()),
        }
    }

    /// Configurar umbral mínimo de votos
    pub fn with_min_votes(mut self, min_votes: usize) -> Self {
        self.min_votes = min_votes;
        self
    }

    /// Configurar umbral de acuerdo
    pub fn with_agreement_threshold(mut self, threshold: f64) -> Self {
        self.agreement_threshold = threshold.clamp(0.5, 1.0);
        self
    }

    /// Configurar time window
    pub fn with_time_window(mut self, window_ms: u64) -> Self {
        self.time_window_ms = window_ms;
        self
    }

    /// Crear nuevo batch pendiente
    pub fn create_pending_batch(
        &self,
        batch_id: String,
        layer_id: u32,
        features: &[SparseFeature],
    ) -> Result<FeatureBatchHash> {
        // Serializar features para hashing
        let serialized: Vec<Vec<u8>> = features
            .iter()
            .map(|f| {
                let data = format!("{}:{}:{}", f.neuron_index, f.activation_value, f.importance);
                data.into_bytes()
            })
            .collect();

        // Calcular hash del batch
        let batch_hash = FeatureBatchHash::from_serialized_features(batch_id.clone(), serialized)?;

        // Calcular time window actual
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let time_window = (now_ms / self.time_window_ms) * self.time_window_ms;

        // Crear batch pendiente
        let pending = PendingBatch {
            batch_id: batch_id.clone(),
            layer_id,
            time_window,
            votes: Vec::new(),
            leading_root: Some(batch_hash.merkle_root.clone()),
            batch_hash: Some(batch_hash.clone()),
            state: BatchState::Collecting,
            created_at: Instant::now(),
            timeout_secs: self.batch_timeout_secs,
        };

        self.pending_batches
            .write()
            .insert(batch_id.clone(), pending);

        info!(
            "Batch pendiente creado: id={}, layer={}, merkle_root={}",
            batch_id, layer_id, batch_hash.merkle_root
        );

        Ok(batch_hash)
    }

    /// Recibir voto de un nodo
    pub fn receive_vote(&self, vote: ConsensusVote) -> Result<()> {
        // Verificar reputación del votante
        {
            let mut reputation = self.node_reputation.write();
            let rep = reputation
                .entry(vote.voter_peer_id.clone())
                .or_insert_with(|| NodeReputation::new(vote.voter_peer_id.clone()));

            if rep.is_banned {
                warn!("Voto rechazado de nodo baneado: {}", vote.voter_peer_id);
                return Err(anyhow::anyhow!("Voter is banned"));
            }
        }

        // Encontrar batch correspondiente
        let batch_key = self
            .pending_batches
            .read()
            .keys()
            .find(|k| k.starts_with(&format!("{}-{}", vote.layer_id, vote.time_window)))
            .cloned();

        if let Some(key) = batch_key {
            let mut batches = self.pending_batches.write();
            if let Some(batch) = batches.get_mut(&key) {
                if batch.state == BatchState::Collecting {
                    batch.votes.push(vote);
                    debug!("Voto recibido: batch={}, votes={}", key, batch.votes.len());

                    // Verificar si hay suficientes votos para decidir
                    if batch.votes.len() >= self.min_votes {
                        drop(batches);
                        self.evaluate_batch(&key)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Evaluar batch con votos suficientes
    fn evaluate_batch(&self, batch_id: &str) -> Result<()> {
        let mut batches = self.pending_batches.write();
        let batch = match batches.get_mut(batch_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        batch.state = BatchState::Validating;

        // Contar votos por raíz Merkle
        let mut root_votes: HashMap<String, usize> = HashMap::new();
        for vote in &batch.votes {
            *root_votes.entry(vote.merkle_root.clone()).or_insert(0) += 1;
        }

        // Encontrar raíz líder
        let (leading_root, leading_votes) = root_votes
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .unwrap_or_default();

        let total_votes = batch.votes.len();
        let agreement_ratio = leading_votes as f64 / total_votes.max(1) as f64;

        info!(
            "Evaluando batch {}: leading_root={}, votes={}/{}, ratio={:.3}",
            batch_id, leading_root, leading_votes, total_votes, agreement_ratio
        );

        // Verificar consenso
        let approved =
            leading_votes >= self.min_votes && agreement_ratio >= self.agreement_threshold;

        let event = ConsensusEvent {
            approved,
            merkle_root: leading_root.clone(),
            signal_type: Self::determine_signal_type(batch),
            votes_for: leading_votes,
            votes_against: total_votes - leading_votes,
            threshold: self.min_votes,
            layer_id: batch.layer_id,
            time_window: batch.time_window,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            rejection_reason: if !approved {
                Some(format!(
                    "Agreement ratio {:.3} < {:.3} or votes {} < {}",
                    agreement_ratio, self.agreement_threshold, leading_votes, self.min_votes
                ))
            } else {
                None
            },
        };

        // Actualizar estado del batch
        batch.state = if approved {
            BatchState::Approved
        } else {
            BatchState::Rejected
        };

        // Actualizar reputación de nodos
        self.update_node_reputation(&batch.votes, &leading_root, approved);

        // Emitir evento
        self.events.write().push(event.clone());

        info!(
            "Consenso {}: batch={}, layer={}, approved={}",
            if approved { "APROBADO" } else { "RECHAZADO" },
            batch_id,
            batch.layer_id,
            approved
        );

        Ok(())
    }

    /// Determinar tipo de señal basado en el batch
    fn determine_signal_type(batch: &PendingBatch) -> SignalType {
        // TODO: Phase 3 - Lógica más sofisticada basada en contenido
        if batch.layer_id < 10 {
            SignalType::Steering
        } else {
            SignalType::WeightAdjust
        }
    }

    /// Actualizar reputación de nodos tras consenso
    fn update_node_reputation(&self, votes: &[ConsensusVote], leading_root: &str, _approved: bool) {
        let mut reputation = self.node_reputation.write();

        for vote in votes {
            let rep = reputation
                .entry(vote.voter_peer_id.clone())
                .or_insert_with(|| NodeReputation::new(vote.voter_peer_id.clone()));

            let was_consistent = vote.merkle_root == leading_root;
            rep.update(was_consistent);
        }
    }

    /// Verificar y limpiar batches expirados
    pub fn check_expired_batches(&self) -> Vec<String> {
        let mut batches = self.pending_batches.write();
        let mut expired = Vec::new();

        batches.retain(|id, batch| {
            if batch.state == BatchState::Collecting
                && batch.created_at.elapsed() > Duration::from_secs(batch.timeout_secs)
            {
                batch.state = BatchState::TimedOut;
                expired.push(id.clone());
                warn!("Batch expirado: {}", id);
                false
            } else {
                true
            }
        });

        expired
    }

    /// Obtener eventos de consenso
    pub fn get_events(&self, limit: usize) -> Vec<ConsensusEvent> {
        let events = self.events.read();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Obtener reputación de un nodo
    pub fn get_node_reputation(&self, peer_id: &str) -> Option<NodeReputation> {
        self.node_reputation.read().get(peer_id).cloned()
    }

    /// Obtener estadísticas de consenso
    pub fn stats(&self) -> ConsensusStats {
        let batches = self.pending_batches.read();
        let events = self.events.read();

        let mut approved = 0;
        let mut rejected = 0;
        let mut timed_out = 0;
        let mut collecting = 0;

        for batch in batches.values() {
            match batch.state {
                BatchState::Approved => approved += 1,
                BatchState::Rejected => rejected += 1,
                BatchState::TimedOut => timed_out += 1,
                BatchState::Collecting => collecting += 1,
                BatchState::Validating => {}
            }
        }

        ConsensusStats {
            total_batches: batches.len(),
            approved,
            rejected,
            timed_out,
            collecting,
            total_events: events.len(),
            tracked_nodes: self.node_reputation.read().len(),
        }
    }
}

/// Estadísticas de consenso
#[derive(Debug, Clone)]
pub struct ConsensusStats {
    pub total_batches: usize,
    pub approved: usize,
    pub rejected: usize,
    pub timed_out: usize,
    pub collecting: usize,
    pub total_events: usize,
    pub tracked_nodes: usize,
}

impl Default for ConsensusValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Fase 3 - ZKP Integration Methods
// ============================================================================

impl ConsensusValidator {
    /// Verifica un batch usando ZKP
    ///
    /// Convierte SparseFeature a valores f64 y usa ZKPVerifier
    /// para generar y verificar compromiso criptográfico.
    pub fn verify_batch_with_zkp(
        &self,
        batch_id: &str,
        features: &[SparseFeature],
        verifier_id: &str,
    ) -> VerificationResult {
        // Convierte features a valores f64
        let feature_values: Vec<f64> = features.iter().map(|f| f.activation_value as f64).collect();

        let result = self
            .zkp_verifier
            .verify_batch(batch_id, &feature_values, verifier_id);

        // Actualiza reputación criptográfica
        {
            let mut crypto_rep = self.crypto_reputation.write();
            let _rep = crypto_rep
                .entry(verifier_id.to_string())
                .or_insert_with(|| CryptoReputation::new(verifier_id.to_string()));
            // La reputación se actualiza internamente en verify_batch
        }

        match &result {
            VerificationResult::ZKPVerified { confidence, .. } => {
                info!(
                    "ZKP verification successful: batch={}, confidence={:.3}",
                    batch_id, confidence
                );
            }
            VerificationResult::MerkleVerified { confidence, .. } => {
                info!(
                    "Merkle fallback verification: batch={}, confidence={:.3}",
                    batch_id, confidence
                );
            }
            VerificationResult::Failed { reason, .. } => {
                warn!(
                    "ZKP verification failed: batch={}, reason={}",
                    batch_id, reason
                );
            }
            _ => {}
        }

        result
    }

    /// Obtiene reputación criptográfica de un nodo
    pub fn get_crypto_reputation(&self, node_id: &str) -> Option<CryptoReputation> {
        self.zkp_verifier.get_node_reputation(node_id)
    }

    /// Obtiene todas las reputaciones criptográficas
    pub fn get_all_crypto_reputations(&self) -> Vec<CryptoReputation> {
        self.zkp_verifier.get_all_reputations()
    }

    /// Verifica si un nodo pasa el umbral de confianza criptográfica
    pub fn node_passes_crypto_threshold(&self, node_id: &str) -> bool {
        if let Some(rep) = self.zkp_verifier.get_node_reputation(node_id) {
            rep.trust_level() != crate::zkp::verifier::TrustLevel::Untrusted
        } else {
            false // Sin historial = no confiable
        }
    }

    /// Obtiene estadísticas del verificador ZKP
    pub fn get_zkp_stats(&self) -> crate::zkp::verifier::VerifierStats {
        self.zkp_verifier.get_stats()
    }

    /// Establece umbral mínimo de confianza ZKP
    pub fn set_zkp_confidence_threshold(&mut self, threshold: f64) {
        self.zkp_verifier.set_min_confidence(threshold);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_validator_creation() {
        let validator = ConsensusValidator::new();
        let stats = validator.stats();
        assert_eq!(stats.total_batches, 0);
    }

    #[test]
    fn test_node_reputation() {
        let mut rep = NodeReputation::new("test_peer".to_string());
        rep.update(true); // consistent
        rep.update(true); // consistent
        rep.update(false); // inconsistent
        assert!((rep.reputation_score - 0.67).abs() < 0.01);
        assert!(!rep.is_banned);
    }

    #[test]
    fn test_batch_hash_creation() {
        let validator = ConsensusValidator::new();
        let features = vec![
            SparseFeature {
                neuron_index: 0,
                activation_value: 0.9,
                importance: 0.8,
            },
            SparseFeature {
                neuron_index: 1,
                activation_value: 0.7,
                importance: 0.6,
            },
        ];

        let batch_hash = validator
            .create_pending_batch("test_batch".to_string(), 0, &features)
            .unwrap();
        assert!(!batch_hash.merkle_root.is_empty());
        assert_eq!(batch_hash.feature_count, 2);
    }
}
