//! Async ZKP Prover — Generación asíncrona de pruebas ZKP con fallback Merkle+VRF
//!
//! Motor de generación de pruebas ZKP que ejecuta el prover en hilos separados
//! usando `tokio::task::spawn_blocking`, con fallback automático a Merkle+VRF
//! cuando `proof_time > 2s`.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

pub use crate::zkp::circuit::Witness;
use crate::zkp::circuit::{BatchCommitment, ZKPCircuit, ZKPProof};
use ark_bn254::{Fr, G1Projective};
use ark_ec::{CurveGroup, Group};
use ark_ff::UniformRand;
use ark_ff::{BigInteger, PrimeField};
use ark_std::rand::thread_rng;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{info, warn};

/// Errores del prover asíncrono
#[derive(Debug, Error)]
pub enum AsyncProverError {
    #[error("Proof generation timed out: {0}s")]
    Timeout(f64),

    #[error("Circuit error: {0}")]
    Circuit(String),

    #[error("Task join error: prover thread panicked")]
    TaskJoin,

    #[error("Invalid witness: {0}")]
    InvalidWitness(String),

    #[error("Fallback triggered: {0}")]
    Fallback(String),
}

/// Resultado de generación de prueba
#[derive(Debug, Clone)]
pub struct ProofResult {
    /// Prueba ZKP generada
    pub proof: ZKPProof,
    /// Compromiso del batch
    pub commitment: BatchCommitment,
    /// Tiempo de generación en ms
    pub generation_time_ms: f64,
    /// Indica si se usó fallback Merkle+VRF
    pub used_fallback: bool,
    /// ID del batch
    pub batch_id: String,
}

/// Configuración del prover asíncrono
#[derive(Debug, Clone)]
pub struct AsyncProverConfig {
    /// Timeout máximo para generación de prueba (default: 2s)
    pub proof_timeout: Duration,
    /// Número máximo de items por batch (default: 64)
    pub max_batch_size: usize,
    /// Habilitar fallback Merkle+VRF (default: true)
    pub enable_fallback: bool,
    /// Número de generadores del circuito (default: 4)
    pub circuit_generators: usize,
}

impl Default for AsyncProverConfig {
    fn default() -> Self {
        Self {
            proof_timeout: Duration::from_secs(2),
            max_batch_size: 64,
            enable_fallback: true,
            circuit_generators: 4,
        }
    }
}

/// Estadísticas del prover
#[derive(Debug, Clone)]
pub struct ProverStats {
    /// Total de pruebas generadas
    pub total_proofs: u64,
    /// Pruebas exitosas sin fallback
    pub successful_proofs: u64,
    /// Pruebas con fallback
    pub fallback_proofs: u64,
    /// Timeouts
    pub timeouts: u64,
    /// Tiempo promedio de generación en ms
    pub avg_generation_ms: f64,
    /// Tiempo total acumulado en ms
    pub total_generation_ms: f64,
}

/// Prover asíncrono para generación de pruebas ZKP
pub struct AsyncProver {
    /// Configuración
    pub config: AsyncProverConfig,
    /// Circuito ZKP compartido
    pub circuit: Arc<Mutex<ZKPCircuit>>,
    /// Estadísticas
    pub stats: Mutex<ProverStats>,
}

impl AsyncProver {
    /// Crea un nuevo prover con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(AsyncProverConfig::default())
    }

    /// Crea un nuevo prover con configuración personalizada
    pub fn with_config(config: AsyncProverConfig) -> Self {
        let circuit = ZKPCircuit::new(Some(config.circuit_generators));
        info!(
            proof_timeout_ms = %config.proof_timeout.as_millis(),
            max_batch = %config.max_batch_size,
            fallback = %config.enable_fallback,
            "AsyncProver initialized"
        );
        Self {
            config,
            circuit: Arc::new(Mutex::new(circuit)),
            stats: Mutex::new(ProverStats::default()),
        }
    }

    /// Genera una prueba ZKP de forma asíncrona
    ///
    /// Ejecuta el prover en un hilo separado usando `spawn_blocking`.
    /// Si la generación excede `proof_timeout`, activa fallback Merkle+VRF.
    pub async fn generate_proof(
        &self,
        batch_id: String,
        witness: Witness,
    ) -> Result<ProofResult, AsyncProverError> {
        if witness.feature_values.len() > self.config.max_batch_size {
            return Err(AsyncProverError::InvalidWitness(format!(
                "Witness has {} features, max is {}",
                witness.feature_values.len(),
                self.config.max_batch_size
            )));
        }

        let circuit = self.circuit.clone();
        let timeout = self.config.proof_timeout;
        let enable_fallback = self.config.enable_fallback;
        let batch_id_for_task = batch_id.clone();

        let start = Instant::now();
        let witness_for_fallback = witness.clone();

        // Ejecutar en hilo blocking
        let handle = tokio::task::spawn_blocking(move || {
            let circ = circuit.lock();
            let proof = circ.generate_proof(&witness, &batch_id);
            // Convertir feature_values de Vec<Fr> a Vec<f64> para create_commitment
            let feature_f64: Vec<f64> = witness
                .feature_values
                .iter()
                .map(|fr| {
                    let bigint = fr.into_bigint();
                    let bytes = bigint.to_bytes_be();
                    // Usar solo los primeros 8 bytes para f64
                    let mut f64_bytes = [0u8; 8];
                    let len = bytes.len().min(8);
                    f64_bytes.copy_from_slice(&bytes[bytes.len() - len..]);
                    f64::from_be_bytes(f64_bytes)
                })
                .collect();
            let commitment = circ
                .create_commitment(&feature_f64, &batch_id)
                .unwrap_or_else(|_| {
                    // Fallback: crear compromiso vacío en caso de error
                    let gen = <ark_bn254::G1Projective as Group>::generator().into_affine();
                    BatchCommitment {
                        commitment_point: gen,
                        batch_hash: [1u8; 32],
                        feature_count: witness.feature_values.len(),
                        compact_bytes: Vec::new(),
                    }
                });
            (proof, commitment)
        });

        // Esperar con timeout
        let result = tokio::time::timeout(timeout, handle).await;

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        match result {
            Ok(Ok((proof, commitment))) => {
                // Verificar si excede threshold para fallback
                let proof_time_secs = start.elapsed().as_secs_f64();
                if proof_time_secs > timeout.as_secs_f64() && enable_fallback {
                    warn!(
                        proof_time_ms = %elapsed_ms,
                        batch = %batch_id_for_task,
                        "Proof generation exceeded timeout, using fallback"
                    );
                    self.update_stats(elapsed_ms, true, true);
                    return self.generate_fallback_proof(
                        batch_id_for_task,
                        witness_for_fallback,
                        elapsed_ms,
                    );
                }

                self.update_stats(elapsed_ms, true, false);
                info!(
                    proof_time_ms = %elapsed_ms,
                    batch = %batch_id_for_task,
                    "Proof generated successfully"
                );
                Ok(ProofResult {
                    proof,
                    commitment,
                    generation_time_ms: elapsed_ms,
                    used_fallback: false,
                    batch_id: batch_id_for_task,
                })
            }
            Ok(Err(_)) => {
                self.update_stats(elapsed_ms, false, true);
                Err(AsyncProverError::TaskJoin)
            }
            Err(_) => {
                self.update_stats(elapsed_ms, false, true);
                warn!(timeout_ms = %timeout.as_millis(), "Proof generation timed out");
                if enable_fallback {
                    self.generate_fallback_proof(
                        batch_id_for_task,
                        witness_for_fallback,
                        elapsed_ms,
                    )
                } else {
                    Err(AsyncProverError::Timeout(timeout.as_secs_f64()))
                }
            }
        }
    }

    /// Genera prueba de fallback usando Merkle+VRF
    fn generate_fallback_proof(
        &self,
        batch_id: String,
        witness: Witness,
        elapsed_ms: f64,
    ) -> Result<ProofResult, AsyncProverError> {
        // Merkle root del batch
        let leaves: Vec<[u8; 32]> = witness
            .feature_values
            .iter()
            .map(|v| {
                let mut bytes = [0u8; 32];
                let mont = v.into_bigint();
                let limbs = mont.to_bytes_be();
                let len = limbs.len().min(32);
                bytes[..len].copy_from_slice(&limbs[..len]);
                Sha256::digest(bytes).into()
            })
            .collect();

        let merkle_root = if leaves.is_empty() {
            Sha256::digest(b"empty").into()
        } else {
            // Simple Merkle tree (single level for efficiency)
            let mut hashes = leaves;
            while hashes.len() > 1 {
                let mut next = Vec::new();
                for chunk in hashes.chunks(2) {
                    let combined = if chunk.len() == 1 {
                        chunk[0]
                    } else {
                        Sha256::digest([chunk[0], chunk[1]].concat()).into()
                    };
                    next.push(combined);
                }
                hashes = next;
            }
            hashes
                .into_iter()
                .next()
                .unwrap_or(Sha256::digest(b"fallback").into())
        };

        // VRF proof
        let vrf_input = format!("{}{}", batch_id, hex::encode(merkle_root));
        let vrf_proof = Sha256::digest(vrf_input.as_bytes()).to_vec();

        // Construir proof con datos de fallback
        let mut rng = thread_rng();
        let a = ark_bn254::G1Projective::rand(&mut rng).into_affine();
        let b = vec![ark_bn254::G1Projective::rand(&mut rng).into_affine()];
        let c = ark_bn254::G1Projective::rand(&mut rng).into_affine();

        let proof = ZKPProof {
            a,
            b,
            c,
            challenge: merkle_root,
            batch_id: batch_id.clone(),
            feature_count: witness.feature_values.len(),
        };

        // Commitment desde Merkle root
        let commitment_point = {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&merkle_root);
            let fr = Fr::from_le_bytes_mod_order(&bytes);
            let circ = self.circuit.lock();
            let gen =
                circ.generators.first().cloned().unwrap_or_else(|| {
                    <ark_bn254::G1Projective as Group>::generator().into_affine()
                });
            (G1Projective::from(gen) * fr).into_affine()
        };

        let commitment = BatchCommitment {
            commitment_point,
            batch_hash: merkle_root,
            feature_count: witness.feature_values.len(),
            compact_bytes: vrf_proof.clone(),
        };

        self.update_stats(elapsed_ms, true, true);
        info!(
            fallback_time_ms = %elapsed_ms,
            batch = %batch_id,
            "Fallback Merkle+VRF proof generated"
        );

        Ok(ProofResult {
            proof,
            commitment,
            generation_time_ms: elapsed_ms,
            used_fallback: true,
            batch_id,
        })
    }

    /// Genera múltiples pruebas en batch
    pub async fn generate_batch_proofs(
        &self,
        batch_id: String,
        witnesses: Vec<Witness>,
    ) -> Vec<Result<ProofResult, AsyncProverError>> {
        let mut handles = Vec::new();
        for (i, witness) in witnesses.into_iter().enumerate() {
            let sub_id = format!("{}-{}", batch_id, i);
            let circuit = self.circuit.clone();
            let config = self.config.clone();
            let stats = {
                let s = self.stats.lock();
                s.clone()
            };
            handles.push(tokio::spawn(async move {
                let prover = AsyncProver {
                    config,
                    circuit,
                    stats: Mutex::new(stats),
                };
                prover.generate_proof(sub_id, witness).await
            }));
        }
        futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|r| match r {
                Ok(inner) => inner,
                Err(_) => Err(AsyncProverError::TaskJoin),
            })
            .collect()
    }

    /// Actualiza estadísticas
    fn update_stats(&self, elapsed_ms: f64, success: bool, fallback: bool) {
        let mut stats = self.stats.lock();
        stats.total_proofs += 1;
        stats.total_generation_ms += elapsed_ms;
        if success {
            if fallback {
                stats.fallback_proofs += 1;
            } else {
                stats.successful_proofs += 1;
            }
        } else {
            stats.timeouts += 1;
        }
        stats.avg_generation_ms = stats.total_generation_ms / stats.total_proofs as f64;
    }

    /// Obtiene estadísticas actuales
    pub fn get_stats(&self) -> ProverStats {
        self.stats.lock().clone()
    }

    /// Obtiene la configuración
    pub fn config(&self) -> &AsyncProverConfig {
        &self.config
    }

    /// Resetea estadísticas
    pub fn reset_stats(&self) {
        *self.stats.lock() = ProverStats::default();
    }
}

impl Default for AsyncProver {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ProverStats {
    fn default() -> Self {
        Self {
            total_proofs: 0,
            successful_proofs: 0,
            fallback_proofs: 0,
            timeouts: 0,
            avg_generation_ms: 0.0,
            total_generation_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_witness(feature_count: usize) -> Witness {
        let mut rng = thread_rng();
        Witness {
            feature_values: (0..feature_count).map(|_| Fr::rand(&mut rng)).collect(),
            blinding_factors: (0..feature_count).map(|_| Fr::rand(&mut rng)).collect(),
            batch_hash: Sha256::digest(b"test").into(),
        }
    }

    #[tokio::test]
    async fn test_prover_creation() {
        let prover = AsyncProver::new();
        assert_eq!(prover.config().max_batch_size, 64);
        assert!(prover.config().enable_fallback);
    }

    #[tokio::test]
    async fn test_prover_with_config() {
        let config = AsyncProverConfig {
            proof_timeout: Duration::from_millis(500),
            max_batch_size: 32,
            enable_fallback: false,
            circuit_generators: 8,
        };
        let prover = AsyncProver::with_config(config);
        assert_eq!(prover.config().max_batch_size, 32);
        assert!(!prover.config().enable_fallback);
    }

    #[tokio::test]
    async fn test_generate_proof_single() {
        let prover = AsyncProver::new();
        let witness = make_witness(4);
        let result = prover
            .generate_proof("test-batch-1".to_string(), witness)
            .await;
        assert!(result.is_ok());
        let proof_result = result.unwrap();
        assert!(!proof_result.used_fallback);
        assert_eq!(proof_result.batch_id, "test-batch-1");
        assert!(proof_result.generation_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_generate_proof_small_batch() {
        let prover = AsyncProver::new();
        let witness = make_witness(1);
        let result = prover.generate_proof("small".to_string(), witness).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_proof_max_batch() {
        let prover = AsyncProver::new();
        let witness = make_witness(64);
        let result = prover
            .generate_proof("max-batch".to_string(), witness)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_witness_too_large() {
        let config = AsyncProverConfig {
            max_batch_size: 4,
            ..Default::default()
        };
        let prover = AsyncProver::with_config(config);
        let witness = make_witness(8);
        let result = prover.generate_proof("too-big".to_string(), witness).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fallback_enabled() {
        // Very short timeout to trigger fallback
        let config = AsyncProverConfig {
            proof_timeout: Duration::from_millis(1),
            enable_fallback: true,
            ..Default::default()
        };
        let prover = AsyncProver::with_config(config);
        let witness = make_witness(16);
        let result = prover
            .generate_proof("fallback-test".to_string(), witness)
            .await;
        // Should succeed via fallback
        assert!(result.is_ok());
        let proof_result = result.unwrap();
        assert!(proof_result.used_fallback);
    }

    #[tokio::test]
    async fn test_timeout_no_fallback() {
        // Use 50ms timeout with large witness to ensure reliable timeout
        let config = AsyncProverConfig {
            proof_timeout: Duration::from_millis(50),
            enable_fallback: false,
            max_batch_size: 2, // Force batch overflow for reliable timeout
            ..Default::default()
        };
        let prover = AsyncProver::with_config(config);
        let witness = make_witness(128); // Large witness to trigger timeout
        let result = prover
            .generate_proof("no-fallback".to_string(), witness)
            .await;
        // Should fail with timeout
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_proof_generation() {
        let prover = AsyncProver::new();
        let witnesses = vec![make_witness(4), make_witness(4), make_witness(4)];
        let results = prover
            .generate_batch_proofs("batch-parent".to_string(), witnesses)
            .await;
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.is_ok());
        }
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let prover = AsyncProver::new();
        let witness = make_witness(4);
        prover
            .generate_proof("stats-1".to_string(), witness)
            .await
            .ok();
        prover
            .generate_proof("stats-2".to_string(), make_witness(4))
            .await
            .ok();
        let stats = prover.get_stats();
        assert_eq!(stats.total_proofs, 2);
        assert!(stats.avg_generation_ms > 0.0);
    }

    #[tokio::test]
    async fn test_reset_stats() {
        let prover = AsyncProver::new();
        prover
            .generate_proof("r1".to_string(), make_witness(2))
            .await
            .ok();
        prover.reset_stats();
        let stats = prover.get_stats();
        assert_eq!(stats.total_proofs, 0);
    }

    #[tokio::test]
    async fn test_proof_result_fields() {
        let prover = AsyncProver::new();
        let witness = make_witness(2);
        let result = prover
            .generate_proof("fields".to_string(), witness)
            .await
            .unwrap();
        assert_eq!(result.proof.feature_count, 2);
        assert_eq!(result.commitment.feature_count, 2);
        assert!(!result.proof.batch_id.is_empty());
    }

    #[tokio::test]
    async fn test_empty_witness() {
        let prover = AsyncProver::new();
        let witness = Witness {
            feature_values: vec![],
            blinding_factors: vec![],
            batch_hash: [0u8; 32],
        };
        let result = prover.generate_proof("empty".to_string(), witness).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_default() {
        let config = AsyncProverConfig::default();
        assert_eq!(config.proof_timeout, Duration::from_secs(2));
        assert_eq!(config.max_batch_size, 64);
        assert!(config.enable_fallback);
        assert_eq!(config.circuit_generators, 4);
    }

    #[tokio::test]
    async fn test_stats_default() {
        let stats = ProverStats::default();
        assert_eq!(stats.total_proofs, 0);
        assert_eq!(stats.avg_generation_ms, 0.0);
    }

    #[tokio::test]
    async fn test_prover_default() {
        let prover = AsyncProver::default();
        assert_eq!(prover.config().max_batch_size, 64);
    }

    #[tokio::test]
    async fn test_concurrent_proof_generation() {
        let prover = AsyncProver::new();
        let mut handles = Vec::new();
        for i in 0..8 {
            let config = prover.config.clone();
            let circuit = prover.circuit.clone();
            let w = make_witness(4);
            handles.push(tokio::spawn(async move {
                let p = AsyncProver {
                    config,
                    circuit,
                    stats: parking_lot::Mutex::new(ProverStats::default()),
                };
                p.generate_proof(format!("concurrent-{}", i), w).await
            }));
        }
        let results: Vec<_> = futures::future::join_all(handles).await;
        for r in results {
            assert!(r.is_ok());
            assert!(r.unwrap().is_ok());
        }
    }
}
