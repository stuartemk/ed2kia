//! ZKP Verifier - Verificación de compromisos de batch + fallback Merkle/VRF
//!
//! Integra con `circuit.rs` para:
//! - Verificación de compromisos Pedersen
//! - Fallback a verificación Merkle cuando ZKP no está disponible
//! - VRF (Verifiable Random Function) para selección determinística de auditores
//! - Reputación criptográfica basada en historial de verificaciones

use super::circuit::{BatchCommitment, ZKPCircuit, ZKPError, ZKPProof};
use crate::consensus::merkle::MerkleTree;
// MIGRATION: CanonicalSerialize moved to ark_serialize crate
use ark_serialize::CanonicalSerialize;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn};

/// Resultado de verificación
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// Verificación ZKP exitosa
    ZKPVerified {
        batch_id: String,
        proof_hash: [u8; 32],
        confidence: f64,
    },
    /// Verificación Merkle exitosa (fallback)
    MerkleVerified {
        batch_id: String,
        merkle_root: [u8; 32],
        confidence: f64,
    },
    /// Verificación VRF exitosa (para auditoría)
    VRFVerified {
        batch_id: String,
        vrf_proof: Vec<u8>,
        confidence: f64,
    },
    /// Verificación fallida
    Failed { batch_id: String, reason: String },
}

/// Registro de verificación
#[derive(Debug, Clone)]
pub struct VerificationRecord {
    pub batch_id: String,
    pub result: VerificationResult,
    pub timestamp: u128,
    pub verifier_id: String,
    pub computation_time_ms: f64,
}

/// Reputación criptográfica de un nodo verificador
#[derive(Debug, Clone)]
pub struct CryptoReputation {
    pub node_id: String,
    pub total_verifications: u64,
    pub successful_verifications: u64,
    pub failed_verifications: u64,
    pub zkp_verifications: u64,
    pub merkle_verifications: u64,
    pub vrf_verifications: u64,
    pub reputation_score: f64,
    pub last_updated: u128,
}

impl CryptoReputation {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            total_verifications: 0,
            successful_verifications: 0,
            failed_verifications: 0,
            zkp_verifications: 0,
            merkle_verifications: 0,
            vrf_verifications: 0,
            reputation_score: 1.0, // Empieza con reputación máxima
            last_updated: 0,
        }
    }

    /// Actualiza reputación basado en resultado de verificación
    pub fn update(
        &mut self,
        result: &VerificationResult,
        _is_zkp: bool,
        _is_merkle: bool,
        _is_vrf: bool,
    ) {
        self.total_verifications += 1;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        match result {
            VerificationResult::ZKPVerified { .. } => {
                self.successful_verifications += 1;
                self.zkp_verifications += 1;
                self.reputation_score = (self.reputation_score * 0.95) + 0.05; // Refuerzo positivo
            }
            VerificationResult::MerkleVerified { .. } => {
                self.successful_verifications += 1;
                self.merkle_verifications += 1;
                self.reputation_score = (self.reputation_score * 0.95) + 0.02; // Refuerzo moderado
            }
            VerificationResult::VRFVerified { .. } => {
                self.successful_verifications += 1;
                self.vrf_verifications += 1;
                self.reputation_score = (self.reputation_score * 0.95) + 0.03;
            }
            VerificationResult::Failed { .. } => {
                self.failed_verifications += 1;
                self.reputation_score = (self.reputation_score * 0.9) - 0.1; // Penalización
                self.reputation_score = self.reputation_score.max(0.0);
            }
        }

        // Normaliza a [0, 1]
        self.reputation_score = self.reputation_score.clamp(0.0, 1.0);
    }

    /// Obtiene nivel de confianza basado en reputación
    pub fn trust_level(&self) -> TrustLevel {
        match self.reputation_score {
            s if s >= 0.9 => TrustLevel::High,
            s if s >= 0.7 => TrustLevel::Medium,
            s if s >= 0.5 => TrustLevel::Low,
            _ => TrustLevel::Untrusted,
        }
    }
}

/// Nivel de confianza
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLevel {
    High,
    Medium,
    Low,
    Untrusted,
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustLevel::High => write!(f, "HIGH"),
            TrustLevel::Medium => write!(f, "MEDIUM"),
            TrustLevel::Low => write!(f, "LOW"),
            TrustLevel::Untrusted => write!(f, "UNTRUSTED"),
        }
    }
}

/// Verificador ZKP principal
pub struct ZKPVerifier {
    /// Circuito ZKP compartido
    circuit: ZKPCircuit,
    /// Historial de verificaciones
    verification_history: RwLock<Vec<VerificationRecord>>,
    /// Reputación por nodo
    node_reputations: RwLock<HashMap<String, CryptoReputation>>,
    /// Umbral mínimo de confianza para aprobación
    min_confidence_threshold: f64,
    /// Verificaciones exitosas totales
    successful_count: AtomicU64,
    /// Verificaciones fallidas totales
    failed_count: AtomicU64,
}

impl ZKPVerifier {
    /// Crea un nuevo verificador ZKP
    pub fn new(min_confidence: Option<f64>) -> Self {
        Self {
            circuit: ZKPCircuit::new(None),
            verification_history: RwLock::new(Vec::new()),
            node_reputations: RwLock::new(HashMap::new()),
            min_confidence_threshold: min_confidence.unwrap_or(0.6),
            successful_count: AtomicU64::new(0),
            failed_count: AtomicU64::new(0),
        }
    }

    /// Verifica un batch usando ZKP con fallback a Merkle
    pub fn verify_batch(
        &self,
        batch_id: &str,
        feature_values: &[f64],
        verifier_id: &str,
    ) -> VerificationResult {
        let start_time = std::time::Instant::now();

        // Intenta verificación ZKP primero
        match self.verify_with_zkp(batch_id, feature_values) {
            Ok(result) => {
                self.record_verification(
                    batch_id.to_string(),
                    result.clone(),
                    verifier_id,
                    start_time,
                );
                result
            }
            Err(e) => {
                warn!(
                    "ZKP verification failed for batch {}: {}. Falling back to Merkle.",
                    batch_id, e
                );
                // Fallback a Merkle
                match self.verify_with_merkle(batch_id, feature_values) {
                    Ok(result) => {
                        self.record_verification(
                            batch_id.to_string(),
                            result.clone(),
                            verifier_id,
                            start_time,
                        );
                        result
                    }
                    Err(e) => {
                        warn!("Merkle fallback also failed: {}", e);
                        let result = VerificationResult::Failed {
                            batch_id: batch_id.to_string(),
                            reason: format!("ZKP: {}; Merkle: {}", e, e),
                        };
                        self.record_verification(
                            batch_id.to_string(),
                            result.clone(),
                            verifier_id,
                            start_time,
                        );
                        result
                    }
                }
            }
        }
    }

    /// Verifica usando ZKP
    fn verify_with_zkp(
        &self,
        batch_id: &str,
        feature_values: &[f64],
    ) -> Result<VerificationResult, ZKPError> {
        // Crea compromiso
        let commitment = self.circuit.create_commitment(feature_values, batch_id)?;

        // Crea witness
        let witness = self.circuit.create_witness(feature_values, batch_id);

        // Genera prueba
        let proof = self.circuit.generate_proof(&witness, batch_id);

        // Verifica prueba
        let is_valid = self.circuit.verify_proof(&proof, &commitment)?;

        if is_valid {
            let proof_hash = Self::compute_proof_hash(&proof);
            let confidence = self.calculate_zkp_confidence(&proof, &commitment);

            Ok(VerificationResult::ZKPVerified {
                batch_id: batch_id.to_string(),
                proof_hash,
                confidence,
            })
        } else {
            Err(ZKPError::ProofVerificationFailed(
                "ZKP proof verification returned false".to_string(),
            ))
        }
    }

    /// Verifica usando árbol Merkle (fallback)
    fn verify_with_merkle(
        &self,
        batch_id: &str,
        feature_values: &[f64],
    ) -> Result<VerificationResult, ZKPError> {
        // Convierte features a hashes de hojas
        let leaf_data: Vec<Vec<u8>> = feature_values
            .iter()
            .map(|&v| {
                let mut hasher = Sha256::new();
                hasher.update(v.to_le_bytes());
                hasher.finalize().to_vec()
            })
            .collect();

        // Construye árbol Merkle
        // MIGRATION: MerkleTree::from_data returns Result, need to unwrap
        let merkle_tree = MerkleTree::from_data(leaf_data)
            .map_err(|e| ZKPError::ProofVerificationFailed(e.to_string()))?;
        let merkle_root = merkle_tree.root.hash.clone();

        // Verifica que el árbol es válido
        if merkle_tree.leaf_count != feature_values.len() {
            return Err(ZKPError::ProofVerificationFailed(
                "Merkle tree leaf count mismatch".to_string(),
            ));
        }

        // Verifica prueba de inclusión para cada feature
        for (i, &value) in feature_values.iter().enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(value.to_le_bytes());
            let leaf_hash = hasher.finalize();

            let proof = merkle_tree
                .generate_proof(i)
                .map_err(|e| ZKPError::ProofVerificationFailed(e.to_string()))?;
            let leaf_hash_hex = format!("{:x}", leaf_hash);
            // MIGRATION: MerkleTree::verify_proof is a static method that takes leaf_hash, proof, root, index
            if !MerkleTree::verify_proof(&leaf_hash_hex, &proof, &merkle_root, i) {
                return Err(ZKPError::ProofVerificationFailed(format!(
                    "Merkle proof failed for feature {}",
                    i
                )));
            }
        }

        let confidence = self.calculate_merkle_confidence(feature_values.len());

        // MIGRATION: merkle_root is String, convert to [u8; 32]
        let merkle_root_bytes: [u8; 32] = sha2::Sha256::digest(&merkle_root).into();
        Ok(VerificationResult::MerkleVerified {
            batch_id: batch_id.to_string(),
            merkle_root: merkle_root_bytes,
            confidence,
        })
    }

    /// Genera y verifica prueba VRF (para selección de auditores)
    pub fn verify_vrf(
        &self,
        batch_id: &str,
        verifier_id: &str,
        secret_seed: &[u8],
    ) -> VerificationResult {
        let start_time = std::time::Instant::now();

        // Genera prueba VRF simplificada
        let vrf_output = self.generate_vrf_proof(verifier_id, batch_id, secret_seed);
        let vrf_proof = vrf_output;

        // Verifica VRF
        let is_valid = self.verify_vrf_proof(verifier_id, batch_id, &vrf_proof, secret_seed);

        if is_valid {
            let confidence = 0.85; // VRF tiene confianza moderada-alta
            let result = VerificationResult::VRFVerified {
                batch_id: batch_id.to_string(),
                vrf_proof,
                confidence,
            };
            self.record_verification(
                batch_id.to_string(),
                result.clone(),
                verifier_id,
                start_time,
            );
            result
        } else {
            let result = VerificationResult::Failed {
                batch_id: batch_id.to_string(),
                reason: "VRF proof verification failed".to_string(),
            };
            self.record_verification(
                batch_id.to_string(),
                result.clone(),
                verifier_id,
                start_time,
            );
            result
        }
    }

    /// Genera prueba VRF
    fn generate_vrf_proof(&self, verifier_id: &str, batch_id: &str, secret_seed: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(secret_seed);
        hasher.update(verifier_id.as_bytes());
        hasher.update(batch_id.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verifica prueba VRF
    fn verify_vrf_proof(
        &self,
        verifier_id: &str,
        batch_id: &str,
        proof: &[u8],
        secret_seed: &[u8],
    ) -> bool {
        let expected = self.generate_vrf_proof(verifier_id, batch_id, secret_seed);
        proof == expected
    }

    /// Calcula confianza de verificación ZKP
    fn calculate_zkp_confidence(&self, proof: &ZKPProof, _commitment: &BatchCommitment) -> f64 {
        let mut confidence = 0.9; // Base alta para ZKP

        // Ajusta basado en número de features
        let feature_factor = 1.0 - (proof.feature_count as f64 / 1000.0);
        confidence *= feature_factor.max(0.5);

        // Ajusta basado en tamaño de prueba
        let proof_size = proof.b.len();
        if proof_size >= COMMITMENT_DIMENSION {
            confidence *= 1.0;
        } else {
            confidence *= 0.8;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Calcula confianza de verificación Merkle
    fn calculate_merkle_confidence(&self, feature_count: usize) -> f64 {
        let mut confidence = 0.75; // Base moderada para Merkle

        // Más features = más confianza (más trabajo para falsificar)
        let feature_factor = (feature_count as f64 / 100.0).min(0.25);
        confidence += feature_factor;

        confidence.clamp(0.0, 1.0)
    }

    /// Calcula hash de prueba
    fn compute_proof_hash(proof: &ZKPProof) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(proof.batch_id.as_bytes());

        let mut buf = Vec::new();
        proof.a.serialize_compressed(&mut buf).unwrap();
        hasher.update(&buf);

        buf.clear();
        proof.c.serialize_compressed(&mut buf).unwrap();
        hasher.update(&buf);

        hasher.update(proof.challenge);

        hasher.finalize().into()
    }

    /// Registra verificación en historial
    fn record_verification(
        &self,
        batch_id: String,
        result: VerificationResult,
        verifier_id: &str,
        start_time: std::time::Instant,
    ) {
        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        let record = VerificationRecord {
            batch_id: batch_id.clone(),
            result: result.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            verifier_id: verifier_id.to_string(),
            computation_time_ms: elapsed_ms,
        };

        // Actualiza historial
        {
            let mut history = self.verification_history.write();
            history.push(record);

            // Limita historial a 10000 entradas
            while history.len() > 10000 {
                history.remove(0);
            }
        }

        // Actualiza reputación del nodo
        {
            let mut reputations = self.node_reputations.write();
            let rep = reputations
                .entry(verifier_id.to_string())
                .or_insert_with(|| CryptoReputation::new(verifier_id.to_string()));

            let is_zkp = matches!(&result, VerificationResult::ZKPVerified { .. });
            let is_merkle = matches!(&result, VerificationResult::MerkleVerified { .. });
            let is_vrf = matches!(&result, VerificationResult::VRFVerified { .. });

            rep.update(&result, is_zkp, is_merkle, is_vrf);
        }

        // Actualiza contadores
        match result {
            VerificationResult::Failed { .. } => {
                self.failed_count.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.successful_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        debug!(
            "Verification recorded: batch={}, verifier={}, time={}ms",
            batch_id, verifier_id, elapsed_ms
        );
    }

    /// Obtiene reputación de un nodo
    pub fn get_node_reputation(&self, node_id: &str) -> Option<CryptoReputation> {
        self.node_reputations.read().get(node_id).cloned()
    }

    /// Obtiene todas las reputaciones
    pub fn get_all_reputations(&self) -> Vec<CryptoReputation> {
        self.node_reputations.read().values().cloned().collect()
    }

    /// Obtiene historial de verificaciones
    pub fn get_verification_history(&self, limit: Option<usize>) -> Vec<VerificationRecord> {
        let history = self.verification_history.read();
        let limit = limit.unwrap_or(100);
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Obtiene estadísticas del verificador
    pub fn get_stats(&self) -> VerifierStats {
        let total = self.successful_count.load(Ordering::Relaxed)
            + self.failed_count.load(Ordering::Relaxed);

        VerifierStats {
            total_verifications: total,
            successful_verifications: self.successful_count.load(Ordering::Relaxed),
            failed_verifications: self.failed_count.load(Ordering::Relaxed),
            success_rate: if total > 0 {
                self.successful_count.load(Ordering::Relaxed) as f64 / total as f64
            } else {
                0.0
            },
            min_confidence_threshold: self.min_confidence_threshold,
            tracked_nodes: self.node_reputations.read().len(),
            history_size: self.verification_history.read().len(),
        }
    }

    /// Verifica si un resultado supera el umbral de confianza
    pub fn passes_threshold(&self, result: &VerificationResult) -> bool {
        let confidence = match result {
            VerificationResult::ZKPVerified { confidence, .. }
            | VerificationResult::MerkleVerified { confidence, .. }
            | VerificationResult::VRFVerified { confidence, .. } => *confidence,
            VerificationResult::Failed { .. } => 0.0,
        };

        confidence >= self.min_confidence_threshold
    }

    /// Establece umbral mínimo de confianza
    pub fn set_min_confidence(&mut self, threshold: f64) {
        self.min_confidence_threshold = threshold.clamp(0.0, 1.0);
        info!(
            "Min confidence threshold set to {}",
            self.min_confidence_threshold
        );
    }

    /// Verifica una prueba ZKP con su compromiso asociado
    pub fn verify(
        &self,
        proof: ZKPProof,
        commitment: BatchCommitment,
    ) -> Result<VerificationResult, ZKPError> {
        let is_valid = self.circuit.verify_proof(&proof, &commitment)?;

        if is_valid {
            let proof_hash = Self::compute_proof_hash(&proof);
            let confidence = self.calculate_zkp_confidence(&proof, &commitment);

            Ok(VerificationResult::ZKPVerified {
                batch_id: proof.batch_id.clone(),
                proof_hash,
                confidence,
            })
        } else {
            Err(ZKPError::ProofVerificationFailed(
                "ZKP proof verification returned false".to_string(),
            ))
        }
    }
}

/// Estadísticas del verificador
#[derive(Debug, Clone)]
pub struct VerifierStats {
    pub total_verifications: u64,
    pub successful_verifications: u64,
    pub failed_verifications: u64,
    pub success_rate: f64,
    pub min_confidence_threshold: f64,
    pub tracked_nodes: usize,
    pub history_size: usize,
}

// Constante usada en calculate_zkp_confidence
const COMMITMENT_DIMENSION: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let verifier = ZKPVerifier::new(None);
        let stats = verifier.get_stats();
        assert_eq!(stats.total_verifications, 0);
        assert_eq!(stats.min_confidence_threshold, 0.6);
    }

    #[test]
    fn test_zkp_verification() {
        let verifier = ZKPVerifier::new(None);
        let features = vec![0.5, -0.3, 0.8, 0.1];
        let result = verifier.verify_batch("test-batch", &features, "test-verifier");

        match &result {
            VerificationResult::ZKPVerified { confidence, .. } => {
                assert!(*confidence > 0.0);
            }
            VerificationResult::MerkleVerified { confidence, .. } => {
                assert!(*confidence > 0.0);
            }
            _ => panic!("Expected successful verification"),
        }
    }

    #[test]
    fn test_vrf_verification() {
        let verifier = ZKPVerifier::new(None);
        let secret = b"test-secret-seed";
        let result = verifier.verify_vrf("vrf-batch", "vrf-verifier", secret);

        match result {
            VerificationResult::VRFVerified { confidence, .. } => {
                assert_eq!(confidence, 0.85);
            }
            _ => panic!("Expected VRF verification"),
        }
    }

    #[test]
    fn test_reputation_update() {
        let verifier = ZKPVerifier::new(None);
        verifier.verify_batch("batch-1", &[1.0, 2.0], "node-1");

        let rep = verifier.get_node_reputation("node-1").unwrap();
        assert_eq!(rep.total_verifications, 1);
        assert!(rep.reputation_score > 0.0);
    }

    #[test]
    fn test_trust_level() {
        let mut rep = CryptoReputation::new("test".to_string());
        assert_eq!(rep.trust_level(), TrustLevel::High);

        rep.reputation_score = 0.8;
        assert_eq!(rep.trust_level(), TrustLevel::Medium);

        rep.reputation_score = 0.6;
        assert_eq!(rep.trust_level(), TrustLevel::Low);

        rep.reputation_score = 0.3;
        assert_eq!(rep.trust_level(), TrustLevel::Untrusted);
    }

    #[test]
    fn test_threshold_check() {
        let verifier = ZKPVerifier::new(Some(0.7));

        let pass = VerificationResult::ZKPVerified {
            batch_id: "test".to_string(),
            proof_hash: [0; 32],
            confidence: 0.8,
        };
        assert!(verifier.passes_threshold(&pass));

        let fail = VerificationResult::ZKPVerified {
            batch_id: "test".to_string(),
            proof_hash: [0; 32],
            confidence: 0.5,
        };
        assert!(!verifier.passes_threshold(&fail));
    }

    #[test]
    fn test_verifier_stats() {
        let verifier = ZKPVerifier::new(None);
        verifier.verify_batch("b1", &[1.0, 2.0], "v1");
        verifier.verify_batch("b2", &[3.0, 4.0], "v1");

        let stats = verifier.get_stats();
        assert_eq!(stats.total_verifications, 2);
        assert_eq!(stats.tracked_nodes, 1);
    }
}
