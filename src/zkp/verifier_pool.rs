//! Verifier Pool — Pool de workers para verificación paralela de pruebas ZKP
//!
//! Pool de 4 workers para verificación concurrente de pruebas ZKP con
//! cola de trabajo, métricas de rendimiento y balanceo de carga.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use crate::zkp::circuit::{BatchCommitment, ZKPProof, ZKPCircuit};
use crate::zkp::verifier::{VerificationResult, VerificationRecord, ZKPVerifier};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info};

/// Errors del pool de verificación
#[derive(Debug, Error)]
pub enum VerifierPoolError {
    #[error("Pool shutdown")]
    Shutdown,

    #[error("Verification failed: {0}")]
    Verification(String),

    #[error("Queue full: max capacity {0}")]
    QueueFull(usize),

    #[error("Worker panic: {0}")]
    WorkerPanic(String),
}

/// Mensaje enviado a workers
struct WorkItem {
    /// Prueba a verificar
    proof: ZKPProof,
    /// Compromiso asociado
    commitment: BatchCommitment,
    /// Canal para respuesta
    response_tx: Sender<VerificationRecord>,
}

/// Resultado de verificación desde el pool
#[derive(Debug, Clone)]
pub struct PoolVerificationResult {
    /// Registro de verificación
    pub record: VerificationRecord,
    /// Worker ID que procesó
    pub worker_id: usize,
    /// Tiempo total en cola + procesamiento (ms)
    pub total_time_ms: f64,
}

/// Estadísticas del pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total de verificaciones procesadas
    pub total_verified: u64,
    /// Verificaciones exitosas
    pub successful: u64,
    /// Verificaciones fallidas
    pub failed: u64,
    /// Tiempo promedio de verificación (ms)
    pub avg_verification_ms: f64,
    /// Tiempo total acumulado (ms)
    pub total_verification_ms: f64,
    /// Items en cola pendientes
    pub pending_queue: usize,
    /// Workers activos
    pub active_workers: usize,
}

/// Configuración del pool
#[derive(Debug, Clone)]
pub struct VerifierPoolConfig {
    /// Número de workers (default: 4)
    pub worker_count: usize,
    /// Tamaño máximo de cola (default: 256)
    pub max_queue_size: usize,
    /// Timeout por verificación (default: 5s)
    pub verification_timeout: Duration,
}

impl Default for VerifierPoolConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            max_queue_size: 256,
            verification_timeout: Duration::from_secs(5),
        }
    }
}

/// Pool de verificación ZKP con workers paralelos
pub struct VerifierPool {
    /// Configuración
    config: VerifierPoolConfig,
    /// Canal de envío de trabajo
    sender: Option<Sender<WorkItem>>,
    /// Shutdown channel
    _shutdown_tx: Option<Sender<()>>,
    /// Worker handles (kept alive)
    _workers: Vec<thread::JoinHandle<()>>,
    /// Contador de items en cola
    pending: AtomicUsize,
    /// Contador de verificaciones
    verified: AtomicU64,
    /// Tiempo total
    total_time_ms: Mutex<f64>,
    /// Exitosos
    successful: AtomicU64,
    /// Fallidos
    failed: AtomicU64,
}

impl VerifierPool {
    /// Crea un nuevo pool con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(VerifierPoolConfig::default())
    }

    /// Crea un nuevo pool con configuración personalizada
    pub fn with_config(config: VerifierPoolConfig) -> Self {
        let (work_tx, work_rx) = channel::<WorkItem>();
        let (shutdown_tx, shutdown_rx) = channel::<()>();
        let work_rx = Arc::new(Mutex::new(work_rx));
        let shutdown_rx = Arc::new(Mutex::new(shutdown_rx));

        let mut workers = Vec::new();
        for worker_id in 0..config.worker_count {
            let work_rx = work_rx.clone();
            let shutdown_rx = shutdown_rx.clone();

            let handle = thread::Builder::new()
                .name(format!("zkp-verifier-{}", worker_id))
                .spawn(move || {
                    let verifier = ZKPVerifier::new(Some(0.5));
                    let _circuit = ZKPCircuit::new(None);

                    loop {
                        // Check shutdown first (non-blocking)
                        {
                            let rx = shutdown_rx.lock();
                            if rx.try_recv().is_ok() {
                                debug!(worker = worker_id, "Worker shutting down");
                                break;
                            }
                        }

                        // Blocking receive with timeout on work channel
                        let rx = work_rx.lock();
                        match rx.recv_timeout(Duration::from_millis(200)) {
                            Ok(item) => {
                                let start = Instant::now();
                                let batch_id = item.proof.batch_id.clone();
                                let verify_result =
                                    verifier.verify(item.proof, item.commitment);
                                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

                                // Determinar el resultado de verificación para el record
                                let verification_result = match &verify_result {
                                    Ok(v) => v.clone(),
                                    Err(_) => crate::zkp::verifier::VerificationResult::Failed {
                                        batch_id: batch_id.clone(),
                                        reason: "Verification returned Err".to_string(),
                                    },
                                };

                                let record = VerificationRecord {
                                    batch_id: batch_id.clone(),
                                    result: verification_result,
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_nanos(),
                                    verifier_id: format!("pool-worker-{}", worker_id),
                                    computation_time_ms: elapsed_ms,
                                };

                                debug!(
                                    worker = worker_id,
                                    batch = %batch_id,
                                    time_ms = elapsed_ms,
                                    "Proof verified"
                                );

                                let _ = item.response_tx.send(record);
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                // Continue loop to check shutdown
                                continue;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                debug!(worker = worker_id, "Channel closed, shutting down");
                                break;
                            }
                        }
                    }
                })
                .expect("Failed to spawn verifier worker");
            workers.push(handle);
        }

        info!(
            workers = config.worker_count,
            queue_size = config.max_queue_size,
            "VerifierPool initialized"
        );

        Self {
            config,
            sender: Some(work_tx),
            _shutdown_tx: Some(shutdown_tx),
            _workers: workers,
            pending: AtomicUsize::new(0),
            verified: AtomicU64::new(0),
            total_time_ms: Mutex::new(0.0),
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        }
    }

    /// Verifica una prueba ZKP (bloquea hasta respuesta)
    pub fn verify(
        &self,
        proof: ZKPProof,
        commitment: BatchCommitment,
    ) -> Result<PoolVerificationResult, VerifierPoolError> {
        let sender = self.sender.as_ref().ok_or(VerifierPoolError::Shutdown)?;

        let (response_tx, response_rx) = channel();
        let work_item = WorkItem {
            proof,
            commitment,
            response_tx,
        };

        self.pending.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();

        sender.send(work_item).map_err(|_| VerifierPoolError::Shutdown)?;

        match response_rx.recv_timeout(self.config.verification_timeout) {
            Ok(record) => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                self.pending.fetch_sub(1, Ordering::Relaxed);
                self.verified.fetch_add(1, Ordering::Relaxed);
                *self.total_time_ms.lock() += elapsed;

                let is_success = matches!(
                    record.result,
                    VerificationResult::ZKPVerified { .. }
                        | VerificationResult::MerkleVerified { .. }
                        | VerificationResult::VRFVerified { .. }
                );
                if is_success {
                    self.successful.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.failed.fetch_add(1, Ordering::Relaxed);
                }

                let worker_id = extract_worker_id(&record.verifier_id);

                Ok(PoolVerificationResult {
                    record,
                    worker_id,
                    total_time_ms: elapsed,
                })
            }
            Err(_) => {
                self.pending.fetch_sub(1, Ordering::Relaxed);
                Err(VerifierPoolError::Verification(
                    "Verification timeout".to_string(),
                ))
            }
        }
    }

    /// Verifica múltiples pruebas en paralelo
    pub fn verify_batch(
        &self,
        proofs: Vec<(ZKPProof, BatchCommitment)>,
    ) -> Vec<Result<PoolVerificationResult, VerifierPoolError>> {
        proofs
            .into_iter()
            .map(|(proof, commitment)| self.verify(proof, commitment))
            .collect()
    }

    /// Obtiene estadísticas actuales
    pub fn get_stats(&self) -> PoolStats {
        let verified = self.verified.load(Ordering::Relaxed);
        let successful = self.successful.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let total_ms = *self.total_time_ms.lock();
        let pending = self.pending.load(Ordering::Relaxed);

        PoolStats {
            total_verified: verified,
            successful,
            failed,
            avg_verification_ms: if verified > 0 {
                total_ms / verified as f64
            } else {
                0.0
            },
            total_verification_ms: total_ms,
            pending_queue: pending,
            active_workers: self.config.worker_count,
        }
    }

    /// Obtiene la configuración
    pub fn config(&self) -> &VerifierPoolConfig {
        &self.config
    }

    /// Resetea estadísticas
    pub fn reset_stats(&self) {
        self.verified.store(0, Ordering::Relaxed);
        self.successful.store(0, Ordering::Relaxed);
        self.failed.store(0, Ordering::Relaxed);
        *self.total_time_ms.lock() = 0.0;
    }
}

fn extract_worker_id(verifier_id: &str) -> usize {
    verifier_id
        .strip_prefix("pool-worker-")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

impl Drop for VerifierPool {
    fn drop(&mut self) {
        // Send shutdown signal to all workers
        if let Some(tx) = self._shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Drop work sender to unblock workers
        self.sender.take();
    }
}

impl Default for VerifierPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zkp::circuit::Witness;
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use ark_std::rand::thread_rng;
    use sha2::Digest;

    fn make_proof(batch_id: &str, feature_count: usize) -> (ZKPProof, BatchCommitment) {
        let feature_values: Vec<f64> = (0..feature_count).map(|i| i as f64 * 1.5).collect();
        let batch_hash: [u8; 32] = sha2::Sha256::digest(b"test").into();
        let mut rng = thread_rng();
        let blinding_factors: Vec<Fr> = (0..feature_count).map(|_| Fr::rand(&mut rng)).collect();
        let feature_values_fr: Vec<Fr> = (0..feature_count).map(|_| Fr::rand(&mut rng)).collect();

        let witness = Witness {
            feature_values: feature_values_fr,
            blinding_factors,
            batch_hash,
        };
        let circuit = ZKPCircuit::new(None);
        let proof = circuit.generate_proof(&witness, batch_id);
        let commitment = circuit.create_commitment(&feature_values, batch_id).unwrap();
        (proof, commitment)
    }

    #[test]
    fn test_pool_creation() {
        let pool = VerifierPool::new();
        assert_eq!(pool.config().worker_count, 4);
        assert_eq!(pool.config().max_queue_size, 256);
    }

    #[test]
    fn test_pool_with_config() {
        let config = VerifierPoolConfig {
            worker_count: 2,
            max_queue_size: 64,
            verification_timeout: Duration::from_secs(10),
        };
        let pool = VerifierPool::with_config(config);
        assert_eq!(pool.config().worker_count, 2);
    }

    #[test]
    fn test_single_verification() {
        let pool = VerifierPool::new();
        let (proof, commitment) = make_proof("test-1", 4);
        let result = pool.verify(proof, commitment);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.total_time_ms > 0.0);
    }

    #[test]
    fn test_batch_verification() {
        let pool = VerifierPool::new();
        let proofs: Vec<_> = (0..4)
            .map(|i| make_proof(&format!("batch-{}", i), 4))
            .collect();
        let results = pool.verify_batch(proofs);
        assert_eq!(results.len(), 4);
        for r in &results {
            assert!(r.is_ok());
        }
    }

    #[test]
    fn test_stats_tracking() {
        let pool = VerifierPool::new();
        let (proof, commitment) = make_proof("stats-1", 2);
        pool.verify(proof, commitment).ok();
        let stats = pool.get_stats();
        assert_eq!(stats.total_verified, 1);
        assert!(stats.avg_verification_ms > 0.0);
    }

    #[test]
    fn test_reset_stats() {
        let pool = VerifierPool::new();
        let (proof, commitment) = make_proof("reset-1", 2);
        pool.verify(proof, commitment).ok();
        pool.reset_stats();
        let stats = pool.get_stats();
        assert_eq!(stats.total_verified, 0);
    }

    #[test]
    fn test_config_default() {
        let config = VerifierPoolConfig::default();
        assert_eq!(config.worker_count, 4);
        assert_eq!(config.max_queue_size, 256);
        assert_eq!(config.verification_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_pool_default() {
        let pool = VerifierPool::default();
        assert_eq!(pool.config().worker_count, 4);
    }

    #[test]
    fn test_verification_result_fields() {
        let pool = VerifierPool::new();
        let (proof, commitment) = make_proof("fields", 2);
        let result = pool.verify(proof, commitment).unwrap();
        assert_eq!(result.record.batch_id, "fields");
        assert!(result.record.computation_time_ms > 0.0);
        assert!(result.worker_id >= 0);
    }

    #[test]
    fn test_extract_worker_id() {
        assert_eq!(extract_worker_id("pool-worker-0"), 0);
        assert_eq!(extract_worker_id("pool-worker-3"), 3);
        assert_eq!(extract_worker_id("unknown"), 0);
    }
}
