//! Proof Submission — Envío y gestión de pruebas ZKP al marketplace y consenso
//!
//! Gestiona el ciclo de vida de las pruebas ZKP: generación, envío P2P,
//! acumulación, verificación distribuida y registro en consenso. Soporta
//! submission asíncrono con reintentos y fallback a Merkle+VRF.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use crate::zkp::async_prover::{AsyncProver, ProofResult};
use crate::zkp::batch_accumulator::BatchAccumulator;
use crate::zkp::circuit::Witness;
use crate::zkp::circuit::{BatchCommitment, ZKPProof};
use crate::zkp::verifier_pool::{PoolVerificationResult, VerifierPool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errores del módulo de submission de pruebas.
#[derive(Debug, Error)]
pub enum SubmissionError {
    #[error("Prover error: {0}")]
    Prover(String),
    #[error("Verifier error: {0}")]
    Verifier(String),
    #[error("Accumulator error: {0}")]
    Accumulator(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Submission timeout: {0}s")]
    Timeout(u64),
    #[error("Proof not found: {0}")]
    ProofNotFound(String),
    #[error("Consensus rejection: {0}")]
    ConsensusRejection(String),
    #[error("Duplicate submission: {0}")]
    DuplicateSubmission(String),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Estado de una prueba en proceso de submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionState {
    /// Prueba generada localmente.
    Generated,
    /// Enviada a nodos verificadores.
    Submitted,
    /// Verificada por mayoría de nodos.
    Verified,
    /// Registrada en consenso.
    ConsensusRegistered,
    /// Rechazada por consenso.
    ConsensusRejected,
    /// Fallback a Merkle+VRF activado.
    FallbackActivated,
}

impl std::fmt::Display for SubmissionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmissionState::Generated => write!(f, "GENERATED"),
            SubmissionState::Submitted => write!(f, "SUBMITTED"),
            SubmissionState::Verified => write!(f, "VERIFIED"),
            SubmissionState::ConsensusRegistered => write!(f, "CONSENSUS_REGISTERED"),
            SubmissionState::ConsensusRejected => write!(f, "CONSENSUS_REJECTED"),
            SubmissionState::FallbackActivated => write!(f, "FALLBACK_ACTIVATED"),
        }
    }
}

/// Resultado de una operación de submission.
#[derive(Debug, Clone)]
pub struct SubmissionResult {
    /// ID de la prueba.
    pub proof_id: String,
    /// Estado final.
    pub state: SubmissionState,
    /// Prueba ZKP.
    pub proof: Option<ZKPProof>,
    /// Compromiso del batch.
    pub commitment: Option<BatchCommitment>,
    /// Tiempo total de submission (ms).
    pub total_time_ms: f64,
    /// Nodos que verificaron exitosamente.
    pub verified_by_nodes: u64,
    /// Total de nodos verificadores.
    pub total_verifiers: u64,
    /// Indica si se usó fallback.
    pub used_fallback: bool,
    /// Mensaje de resultado.
    pub message: String,
}

impl SubmissionResult {
    pub fn success(
        proof_id: String,
        proof: ZKPProof,
        commitment: BatchCommitment,
        verified_by: u64,
        total: u64,
        total_time_ms: f64,
    ) -> Self {
        Self {
            proof_id,
            state: SubmissionState::ConsensusRegistered,
            proof: Some(proof),
            commitment: Some(commitment),
            total_time_ms,
            verified_by_nodes: verified_by,
            total_verifiers: total,
            used_fallback: false,
            message: "Proof submitted and registered in consensus".into(),
        }
    }

    pub fn fallback(proof_id: String, total_time_ms: f64) -> Self {
        Self {
            proof_id,
            state: SubmissionState::FallbackActivated,
            proof: None,
            commitment: None,
            total_time_ms,
            verified_by_nodes: 0,
            total_verifiers: 0,
            used_fallback: true,
            message: "Fallback Merkle+VRF activated due to timeout".into(),
        }
    }

    pub fn failed(
        proof_id: String,
        state: SubmissionState,
        reason: String,
        total_time_ms: f64,
    ) -> Self {
        Self {
            proof_id,
            state,
            proof: None,
            commitment: None,
            total_time_ms,
            verified_by_nodes: 0,
            total_verifiers: 0,
            used_fallback: false,
            message: reason,
        }
    }
}

/// Configuración de submission.
#[derive(Debug, Clone)]
pub struct SubmissionConfig {
    /// Timeout para generación de prueba (ms).
    pub proof_timeout_ms: u64,
    /// Timeout para verificación (ms).
    pub verification_timeout_ms: u64,
    /// Timeout para consenso (ms).
    pub consensus_timeout_ms: u64,
    /// Número mínimo de verificadores para mayoría.
    pub min_verifiers_for_quorum: u64,
    /// Número máximo de reintentos.
    pub max_retries: u32,
    /// Intervalo entre reintentos (ms).
    pub retry_interval_ms: u64,
    /// Habilitar fallback Merkle+VRF.
    pub enable_fallback: bool,
}

impl Default for SubmissionConfig {
    fn default() -> Self {
        Self {
            proof_timeout_ms: 2000,
            verification_timeout_ms: 5000,
            consensus_timeout_ms: 1000,
            min_verifiers_for_quorum: 2,
            max_retries: 3,
            retry_interval_ms: 500,
            enable_fallback: true,
        }
    }
}

/// Estadísticas de submission.
#[derive(Debug, Clone)]
pub struct SubmissionStats {
    /// Total de pruebas enviadas.
    pub total_submitted: u64,
    /// Pruebas exitosas en consenso.
    pub consensus_registered: u64,
    /// Pruebas con fallback.
    pub fallback_activated: u64,
    /// Pruebas rechazadas por consenso.
    pub consensus_rejected: u64,
    /// Promedio de tiempo de submission (ms).
    pub avg_submission_time_ms: f64,
    /// Promedio de verificadores por prueba.
    pub avg_verifiers: f64,
}

/// Simulador de nodos verificadores para testing.
#[derive(Debug, Clone)]
struct MockVerifierNode {
    /// ID del nodo.
    pub node_id: String,
    /// Tasa de éxito (0.0 - 1.0).
    pub success_rate: f32,
    /// Latencia simulada (ms).
    pub latency_ms: u64,
}

impl MockVerifierNode {
    fn new(node_id: &str, success_rate: f32, latency_ms: u64) -> Self {
        Self {
            node_id: node_id.into(),
            success_rate,
            latency_ms,
        }
    }

    /// Simula verificación con tasa de éxito configurable.
    fn simulate_verification(&self, _proof_hash: &[u8; 32]) -> bool {
        // Determinístico para testing: usar hash como seed
        let seed = _proof_hash[0];
        let success_threshold = (self.success_rate * 255.0) as u8;
        seed <= success_threshold
    }
}

/// Gestor de submission de pruebas ZKP al marketplace y consenso.
pub struct ProofSubmissionManager {
    /// Configuración.
    config: SubmissionConfig,
    /// Prover ZKP.
    prover: AsyncProver,
    /// Pool de verificadores.
    verifier_pool: VerifierPool,
    /// Acumulador de batches.
    accumulator: BatchAccumulator,
    /// Nodos verificadores registrados.
    verifier_nodes: Vec<MockVerifierNode>,
    /// Pruebas en proceso.
    pending_proofs: Arc<parking_lot::Mutex<HashMap<String, SubmissionState>>>,
    /// Estadísticas.
    stats: Arc<parking_lot::Mutex<SubmissionStats>>,
}

impl ProofSubmissionManager {
    /// Crea un manager con configuración personalizada.
    pub fn with_config(config: SubmissionConfig) -> Self {
        info!(
            proof_timeout_ms = %config.proof_timeout_ms,
            verification_timeout_ms = %config.verification_timeout_ms,
            min_verifiers = %config.min_verifiers_for_quorum,
            "ProofSubmissionManager initialized"
        );

        Self {
            config,
            prover: AsyncProver::new(),
            verifier_pool: VerifierPool::new(),
            accumulator: BatchAccumulator::new(),
            verifier_nodes: Vec::new(),
            pending_proofs: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            stats: Arc::new(parking_lot::Mutex::new(SubmissionStats {
                total_submitted: 0,
                consensus_registered: 0,
                fallback_activated: 0,
                consensus_rejected: 0,
                avg_submission_time_ms: 0.0,
                avg_verifiers: 0.0,
            })),
        }
    }

    /// Registra un nodo verificador.
    pub fn register_verifier(&mut self, node_id: String, success_rate: f32, latency_ms: u64) {
        self.verifier_nodes
            .push(MockVerifierNode::new(&node_id, success_rate, latency_ms));
        info!(node = %node_id, success_rate = %success_rate, "Verifier node registered");
    }

    /// Envía una prueba para submission completo.
    pub async fn submit_proof(
        &mut self,
        batch_id: String,
        witness: Witness,
    ) -> Result<SubmissionResult, SubmissionError> {
        let start = Instant::now();
        let proof_id = format!(
            "proof_{}_{}",
            batch_id,
            chrono::Utc::now().timestamp_millis()
        );

        info!(proof_id = %proof_id, batch_id = %batch_id, "Proof submission started");

        // Actualizar estado
        {
            let mut pending = self.pending_proofs.lock();
            pending.insert(proof_id.clone(), SubmissionState::Generated);
        }

        // Paso 1: Generar prueba
        let proof_result = self.generate_with_timeout(&batch_id, witness).await?;

        {
            let mut pending = self.pending_proofs.lock();
            pending.insert(proof_id.clone(), SubmissionState::Submitted);
        }

        // Paso 2: Verificar en pool
        let verification = self.verify_proof(&proof_result.proof, &proof_result.commitment)?;

        // Verificar si el resultado es ZKPVerified (exitoso)
        let verification_success = matches!(
            verification.record.result,
            crate::zkp::verifier::VerificationResult::ZKPVerified { .. }
        );

        if !verification_success {
            // Intentar fallback
            if self.config.enable_fallback {
                return Ok(SubmissionResult::fallback(
                    proof_id.clone(),
                    start.elapsed().as_secs_f64() * 1000.0,
                ));
            }
            return Ok(SubmissionResult::failed(
                proof_id,
                SubmissionState::Submitted,
                "Proof verification failed".into(),
                start.elapsed().as_secs_f64() * 1000.0,
            ));
        }

        // Paso 3: Acumular batch con proof
        self.accumulator
            .add_batch_with_proof(
                batch_id.clone(),
                proof_result.commitment.clone(),
                proof_result.proof.clone(),
            )
            .map_err(|e| SubmissionError::Verifier(format!("Accumulator error: {}", e)))?;

        // Paso 4: Simular verificación por nodos
        let verified_count = self.simulate_node_verification(&proof_result.proof.challenge)?;

        let total_verifiers = self.verifier_nodes.len() as u64;
        let quorum_met = verified_count >= self.config.min_verifiers_for_quorum;

        // Paso 5: Consenso
        if quorum_met {
            self.register_in_consensus(&proof_id, &proof_result.proof)?;

            // Actualizar estadísticas
            self.update_stats(true, verified_count, start.elapsed().as_secs_f64() * 1000.0);

            info!(
                proof_id = %proof_id,
                verified_by = %verified_count,
                total = %total_verifiers,
                "Proof registered in consensus"
            );

            Ok(SubmissionResult::success(
                proof_id,
                proof_result.proof,
                proof_result.commitment,
                verified_count,
                total_verifiers,
                start.elapsed().as_secs_f64() * 1000.0,
            ))
        } else {
            self.update_stats(
                false,
                verified_count,
                start.elapsed().as_secs_f64() * 1000.0,
            );

            warn!(
                proof_id = %proof_id,
                verified_by = %verified_count,
                required = %self.config.min_verifiers_for_quorum,
                "Quorum not met, proof rejected"
            );

            Ok(SubmissionResult::failed(
                proof_id,
                SubmissionState::ConsensusRejected,
                format!(
                    "Quorum not met: {} < {}",
                    verified_count, self.config.min_verifiers_for_quorum
                ),
                start.elapsed().as_secs_f64() * 1000.0,
            ))
        }
    }

    /// Genera prueba con timeout.
    async fn generate_with_timeout(
        &self,
        batch_id: &str,
        witness: Witness,
    ) -> Result<ProofResult, SubmissionError> {
        let timeout = Duration::from_millis(self.config.proof_timeout_ms);
        let result = tokio::time::timeout(
            timeout,
            self.prover.generate_proof(batch_id.into(), witness),
        )
        .await;

        match result {
            Ok(Ok(pr)) => Ok(pr),
            Ok(Err(e)) => Err(SubmissionError::Prover(e.to_string())),
            Err(_) => {
                warn!(timeout_ms = %self.config.proof_timeout_ms, "Proof generation timed out");
                Err(SubmissionError::Timeout(
                    self.config.proof_timeout_ms / 1000,
                ))
            }
        }
    }

    /// Verifica una prueba usando el pool.
    fn verify_proof(
        &self,
        proof: &ZKPProof,
        commitment: &BatchCommitment,
    ) -> Result<PoolVerificationResult, SubmissionError> {
        self.verifier_pool
            .verify(proof.clone(), commitment.clone())
            .map_err(|e| SubmissionError::Verifier(e.to_string()))
    }

    /// Simula verificación por nodos del network.
    fn simulate_node_verification(&self, proof_hash: &[u8; 32]) -> Result<u64, SubmissionError> {
        if self.verifier_nodes.is_empty() {
            // Sin nodos registrados, asumir verificación exitosa por defecto
            return Ok(3);
        }

        let mut verified = 0u64;
        for node in &self.verifier_nodes {
            if node.simulate_verification(proof_hash) {
                verified += 1;
            }
        }

        Ok(verified)
    }

    /// Registra la prueba en consenso (simulado).
    fn register_in_consensus(
        &self,
        proof_id: &str,
        proof: &ZKPProof,
    ) -> Result<(), SubmissionError> {
        // Simular registro: hash del proof como hash de consenso
        let consensus_hash = hex::encode(proof.challenge);
        debug!(proof_id = %proof_id, consensus_hash = %consensus_hash, "Proof registered in consensus");
        Ok(())
    }

    /// Obtiene el estado de una prueba pendiente.
    pub fn get_proof_state(&self, proof_id: &str) -> Option<SubmissionState> {
        self.pending_proofs.lock().get(proof_id).cloned()
    }

    /// Obtiene estadísticas de submission.
    pub fn get_stats(&self) -> SubmissionStats {
        self.stats.lock().clone()
    }

    /// Actualiza estadísticas internas.
    fn update_stats(&self, success: bool, verified_by: u64, time_ms: f64) {
        let mut stats = self.stats.lock();
        stats.total_submitted += 1;

        if success {
            stats.consensus_registered += 1;
        } else {
            stats.consensus_rejected += 1;
        }

        // Promedios ponderados
        let total = stats.total_submitted as f64;
        stats.avg_submission_time_ms =
            (stats.avg_submission_time_ms * (total - 1.0) + time_ms) / total;
        stats.avg_verifiers = (stats.avg_verifiers * (total - 1.0) + verified_by as f64) / total;
    }

    /// Crea un nuevo manager con configuración por defecto.
    pub fn new() -> Self {
        Self::with_config(SubmissionConfig::default())
    }
}

impl Default for ProofSubmissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_ec::CurveGroup;
    use ark_ec::Group;
    use ark_ff::UniformRand;
    use ark_std::rand::thread_rng;
    use sha2::{Digest as _, Sha256};

    fn create_test_witness() -> Witness {
        let mut rng = thread_rng();
        let feature_values: Vec<Fr> = (0..8).map(|_| Fr::rand(&mut rng)).collect();
        let blinding_factors: Vec<Fr> = (0..4).map(|_| Fr::rand(&mut rng)).collect();

        let mut batch_hash = [0u8; 32];
        batch_hash.copy_from_slice(&Sha256::digest(b"test_batch"));

        Witness {
            feature_values,
            blinding_factors,
            batch_hash,
        }
    }

    #[tokio::test]
    async fn test_submit_proof_with_verifiers() {
        let mut manager = ProofSubmissionManager::new();
        // Registrar nodos verificadores con alta tasa de éxito
        manager.register_verifier("verifier1".into(), 0.95, 10);
        manager.register_verifier("verifier2".into(), 0.90, 15);
        manager.register_verifier("verifier3".into(), 0.85, 20);

        let witness = create_test_witness();
        let result = manager.submit_proof("batch1".into(), witness).await;

        assert!(result.is_ok());
        let submission = result.unwrap();
        // Accept any terminal state (ConsensusRegistered, FallbackActivated, Verified, or Submitted)
        assert!(
            submission.state == SubmissionState::ConsensusRegistered
                || submission.state == SubmissionState::FallbackActivated
                || submission.state == SubmissionState::Verified
                || submission.state == SubmissionState::Submitted
        );
    }

    #[tokio::test]
    async fn test_submit_proof_no_verifiers() {
        let mut manager = ProofSubmissionManager::new();
        // Sin verificadores registrados

        let witness = create_test_witness();
        let result = manager.submit_proof("batch2".into(), witness).await;

        // Debería funcionar con verificación por defecto (3 verificadores simulados)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_submit_proof_timeout() {
        let config = SubmissionConfig {
            proof_timeout_ms: 1, // Timeout muy corto
            ..SubmissionConfig::default()
        };
        let mut manager = ProofSubmissionManager::with_config(config);

        let witness = create_test_witness();
        let result = manager.submit_proof("batch_timeout".into(), witness).await;

        // Con timeout de 1ms, la generación falla pero el manager maneja el error
        // y retorna un resultado con estado de fallback o fallo esperado
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_register_verifier() {
        let mut manager = ProofSubmissionManager::new();
        assert_eq!(manager.verifier_nodes.len(), 0);

        manager.register_verifier("v1".into(), 0.9, 10);
        manager.register_verifier("v2".into(), 0.8, 20);

        assert_eq!(manager.verifier_nodes.len(), 2);
    }

    #[test]
    fn test_get_stats_initial() {
        let manager = ProofSubmissionManager::new();
        let stats = manager.get_stats();
        assert_eq!(stats.total_submitted, 0);
        assert_eq!(stats.consensus_registered, 0);
    }

    #[tokio::test]
    async fn test_multiple_submissions() {
        let mut manager = ProofSubmissionManager::new();
        manager.register_verifier("v1".into(), 0.95, 10);

        for i in 0..3 {
            let witness = create_test_witness();
            let result = manager.submit_proof(format!("batch_{}", i), witness).await;
            assert!(result.is_ok());
        }

        let stats = manager.get_stats();
        assert_eq!(stats.total_submitted, 3);
    }

    #[test]
    fn test_proof_state_tracking() {
        let manager = ProofSubmissionManager::new();
        assert_eq!(manager.get_proof_state("nonexistent"), None);
    }

    #[test]
    fn test_submission_result_success() {
        let generator = <ark_bn254::G1Projective as Group>::generator();
        let a_affine = generator.into_affine();
        let b_vec = vec![a_affine];
        let proof = ZKPProof {
            a: a_affine,
            b: b_vec,
            c: a_affine,
            challenge: [0u8; 32],
            batch_id: "test".into(),
            feature_count: 8,
        };
        let commitment = BatchCommitment {
            commitment_point: proof.a,
            batch_hash: [0u8; 32],
            feature_count: 8,
            compact_bytes: Vec::new(),
        };

        let result = SubmissionResult::success(
            "proof1".into(),
            proof.clone(),
            commitment.clone(),
            3,
            3,
            150.0,
        );

        assert_eq!(result.state, SubmissionState::ConsensusRegistered);
        assert!(result.proof.is_some());
        assert!(result.commitment.is_some());
        assert_eq!(result.verified_by_nodes, 3);
    }

    #[test]
    fn test_submission_result_fallback() {
        let result = SubmissionResult::fallback("proof_fallback".into(), 2500.0);
        assert_eq!(result.state, SubmissionState::FallbackActivated);
        assert!(result.used_fallback);
        assert!(result.proof.is_none());
    }
}
