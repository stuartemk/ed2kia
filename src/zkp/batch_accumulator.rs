//! Batch Accumulator — Acumulador de pruebas ZKP con agregación incremental
//!
//! Acumula compromisos de batches individuales y genera pruebas acumulativas
//! usando técnicas de agregación de compromisos Pedersen, reduciendo el costo
//! de verificación múltiple a una sola verificación agregada.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use crate::zkp::circuit::{BatchCommitment, ZKPProof, ZKPCircuit};
use ark_bn254::{G1Affine, G1Projective};
use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use ark_std::Zero;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errores del acumulador
#[derive(Debug, Error)]
pub enum AccumulatorError {
    #[error("Batch not found: {0}")]
    BatchNotFound(String),

    #[error("Accumulation failed: {0}")]
    AccumulationFailed(String),

    #[error("Capacity exceeded: {0}")]
    CapacityExceeded(usize),

    #[error("Invalid commitment: {0}")]
    InvalidCommitment(String),

    #[error("No batches to accumulate")]
    EmptyAccumulator,
}

/// Entrada de batch para acumulación
#[derive(Debug, Clone)]
pub struct BatchEntry {
    /// ID del batch
    pub batch_id: String,
    /// Compromiso individual
    pub commitment: BatchCommitment,
    /// Prueba ZKP individual
    pub proof: Option<ZKPProof>,
    /// Timestamp de inserción
    pub inserted_at: u128,
    /// Tamaño del batch (número de features)
    pub feature_count: usize,
}

/// Prueba acumulativa agregada
#[derive(Debug, Clone)]
pub struct AccumulatedProof {
    /// ID único del acumulador
    pub accumulator_id: String,
    /// Compromiso agregado (suma de compromisos individuales)
    pub aggregated_commitment: G1Affine,
    /// Raíz Merkle de todos los batch IDs
    pub merkle_root: [u8; 32],
    /// Número de batches acumulados
    pub batch_count: usize,
    /// Total de features acumuladas
    pub total_features: usize,
    /// Hash de integridad
    pub integrity_hash: [u8; 32],
    /// Timestamp de acumulación
    pub accumulated_at: u128,
    /// IDs de batches incluidos
    pub included_batches: Vec<String>,
}

/// Estadísticas del acumulador
#[derive(Debug, Clone)]
pub struct AccumulatorStats {
    /// Total de batches acumulados
    pub total_batches: u64,
    /// Total de acumulaciones realizadas
    pub total_accumulations: u64,
    /// Total de features procesadas
    pub total_features: u64,
    /// Capacidad actual
    pub current_capacity: usize,
    /// Capacidad máxima
    pub max_capacity: usize,
    /// Tiempo promedio de acumulación (ms)
    pub avg_accumulation_ms: f64,
}

/// Configuración del acumulador
#[derive(Debug, Clone)]
pub struct AccumulatorConfig {
    /// Capacidad máxima de batches (default: 64)
    pub max_capacity: usize,
    /// Umbral para acumulación automática (default: 16)
    pub auto_accumulate_threshold: usize,
    /// Timeout para batches pendientes (default: 30s)
    pub stale_batch_timeout: Duration,
}

impl Default for AccumulatorConfig {
    fn default() -> Self {
        Self {
            max_capacity: 64,
            auto_accumulate_threshold: 16,
            stale_batch_timeout: Duration::from_secs(30),
        }
    }
}

/// Acumulador de batches ZKP
pub struct BatchAccumulator {
    /// Configuración
    config: AccumulatorConfig,
    /// Batches pendientes
    batches: Mutex<HashMap<String, BatchEntry>>,
    /// Circuit ZKP
    circuit: Mutex<ZKPCircuit>,
    /// Historial de acumulaciones
    history: Mutex<Vec<AccumulatedProof>>,
    /// Estadísticas
    stats: Mutex<AccumulatorStats>,
}

impl BatchAccumulator {
    /// Crea un nuevo acumulador con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(AccumulatorConfig::default())
    }

    /// Crea un nuevo acumulador con configuración personalizada
    pub fn with_config(config: AccumulatorConfig) -> Self {
        info!(
            capacity = config.max_capacity,
            threshold = config.auto_accumulate_threshold,
            "BatchAccumulator initialized"
        );
        Self {
            config,
            batches: Mutex::new(HashMap::new()),
            circuit: Mutex::new(ZKPCircuit::new(None)),
            history: Mutex::new(Vec::new()),
            stats: Mutex::new(AccumulatorStats::default()),
        }
    }

    /// Añade un batch al acumulador
    pub fn add_batch(
        &self,
        batch_id: String,
        commitment: BatchCommitment,
    ) -> Result<(), AccumulatorError> {
        let mut batches = self.batches.lock();

        if batches.len() >= self.config.max_capacity {
            return Err(AccumulatorError::CapacityExceeded(self.config.max_capacity));
        }

        if batches.contains_key(&batch_id) {
            debug!(batch = %batch_id, "Batch already exists, skipping");
            return Ok(());
        }

        let entry = BatchEntry {
            batch_id: batch_id.clone(),
            commitment: commitment.clone(),
            proof: None,
            inserted_at: current_timestamp(),
            feature_count: commitment.feature_count,
        };

        batches.insert(batch_id.clone(), entry);

        let mut stats = self.stats.lock();
        stats.current_capacity = batches.len();
        stats.total_features += commitment.feature_count as u64;

        debug!(
            batch = %batch_id,
            capacity = batches.len(),
            "Batch added to accumulator"
        );

        Ok(())
    }

    /// Añade un batch con prueba ZKP
    pub fn add_batch_with_proof(
        &self,
        batch_id: String,
        commitment: BatchCommitment,
        proof: ZKPProof,
    ) -> Result<(), AccumulatorError> {
        let mut batches = self.batches.lock();

        if batches.len() >= self.config.max_capacity {
            return Err(AccumulatorError::CapacityExceeded(self.config.max_capacity));
        }

        let entry = BatchEntry {
            batch_id: batch_id.clone(),
            commitment: commitment.clone(),
            proof: Some(proof),
            inserted_at: current_timestamp(),
            feature_count: commitment.feature_count,
        };

        batches.insert(batch_id.clone(), entry);

        let mut stats = self.stats.lock();
        stats.current_capacity = batches.len();
        stats.total_features += commitment.feature_count as u64;

        debug!(
            batch = %batch_id,
            capacity = batches.len(),
            "Batch with proof added to accumulator"
        );

        Ok(())
    }

    /// Genera prueba acumulativa de todos los batches pendientes
    pub fn accumulate(&self) -> Result<AccumulatedProof, AccumulatorError> {
        let batches = self.batches.lock();

        if batches.is_empty() {
            return Err(AccumulatorError::EmptyAccumulator);
        }

        let start = Instant::now();

        // Sumar todos los compromisos (Pedersen homomorphism)
        let mut aggregated = G1Projective::zero();
        let mut total_features = 0usize;
        let mut batch_ids = Vec::new();

        for (_, entry) in batches.iter() {
            aggregated += G1Projective::from(entry.commitment.commitment_point);
            total_features += entry.feature_count;
            batch_ids.push(entry.batch_id.clone());
        }

        let aggregated_commitment = aggregated.into_affine();

        // Calcular Merkle root de batch IDs
        let merkle_root = compute_merkle_root(&batch_ids);

        // Generar integrity hash
        let mut hash_input = Vec::new();
        aggregated_commitment
            .serialize_compressed(&mut hash_input)
            .expect("serialize");
        hash_input.extend_from_slice(&merkle_root);
        let integrity_hash: [u8; 32] = Sha256::digest(&hash_input).into();

        // Generar accumulator ID
        let accumulator_id = format!("acc-{}", hex::encode(&integrity_hash[..8]));

        let accumulated = AccumulatedProof {
            accumulator_id,
            aggregated_commitment,
            merkle_root,
            batch_count: batches.len(),
            total_features,
            integrity_hash,
            accumulated_at: current_timestamp(),
            included_batches: batch_ids,
        };

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Guardar en historial
        self.history.lock().push(accumulated.clone());

        // Actualizar stats
        let mut stats = self.stats.lock();
        stats.total_accumulations += 1;
        stats.total_batches += batches.len() as u64;
        stats.avg_accumulation_ms =
            stats.avg_accumulation_ms * 0.9 + elapsed_ms * 0.1; // EMA

        info!(
            batches = accumulated.batch_count,
            features = accumulated.total_features,
            time_ms = elapsed_ms,
            "Accumulation complete"
        );

        Ok(accumulated)
    }

    /// Limpia batches stale (más antiguos que timeout)
    pub fn cleanup_stale(&self) -> usize {
        let mut batches = self.batches.lock();
        let now = current_timestamp();
        let timeout_ns = self.config.stale_batch_timeout.as_nanos();

        let before = batches.len();
        batches.retain(|_, entry| now - entry.inserted_at < timeout_ns);
        let removed = before - batches.len();

        let mut stats = self.stats.lock();
        stats.current_capacity = batches.len();

        if removed > 0 {
            warn!(removed, "Stale batches cleaned up");
        }

        removed
    }

    /// Obtiene un batch específico
    pub fn get_batch(&self, batch_id: &str) -> Option<BatchEntry> {
        self.batches.lock().get(batch_id).cloned()
    }

    /// Obtiene el historial de acumulaciones
    pub fn get_history(&self) -> Vec<AccumulatedProof> {
        self.history.lock().clone()
    }

    /// Obtiene la última acumulación
    pub fn get_last_accumulation(&self) -> Option<AccumulatedProof> {
        self.history.lock().last().cloned()
    }

    /// Obtiene estadísticas
    pub fn get_stats(&self) -> AccumulatorStats {
        self.stats.lock().clone()
    }

    /// Obtiene la configuración
    pub fn config(&self) -> &AccumulatorConfig {
        &self.config
    }

    /// Limpia todos los batches pendientes
    pub fn clear(&self) {
        let mut batches = self.batches.lock();
        batches.clear();
        self.stats.lock().current_capacity = 0;
    }

    /// Limpia el historial
    pub fn clear_history(&self) {
        self.history.lock().clear();
    }

    /// Resetea estadísticas
    pub fn reset_stats(&self) {
        *self.stats.lock() = AccumulatorStats::default();
    }

    /// Verifica si un batch está incluido en una acumulación
    pub fn is_batch_in_accumulation(
        &self,
        batch_id: &str,
        accumulation: &AccumulatedProof,
    ) -> bool {
        accumulation.included_batches.contains(&batch_id.to_string())
    }

    /// Verifica la integridad de una prueba acumulativa
    pub fn verify_accumulation(proof: &AccumulatedProof) -> bool {
        // Recalcular Merkle root
        let expected_root = compute_merkle_root(&proof.included_batches);
        if expected_root != proof.merkle_root {
            return false;
        }

        // Verificar integrity hash
        let mut hash_input = Vec::new();
        proof.aggregated_commitment
            .serialize_compressed(&mut hash_input)
            .expect("serialize");
        hash_input.extend_from_slice(&proof.merkle_root);
        let expected_hash: [u8; 32] = Sha256::digest(&hash_input).into();

        expected_hash == proof.integrity_hash
    }
}

fn current_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

/// Calcula raíz Merkle de una lista de strings
fn compute_merkle_root(items: &[String]) -> [u8; 32] {
    if items.is_empty() {
        return Sha256::digest(b"empty").into();
    }

    let mut hashes: Vec<[u8; 32]> = items
        .iter()
        .map(|s| Sha256::digest(s.as_bytes()).into())
        .collect();

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

    hashes.into_iter().next().unwrap_or(Sha256::digest(b"fallback").into())
}

impl Default for BatchAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AccumulatorStats {
    fn default() -> Self {
        Self {
            total_batches: 0,
            total_accumulations: 0,
            total_features: 0,
            current_capacity: 0,
            max_capacity: 64,
            avg_accumulation_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;
    use ark_std::rand::thread_rng;

    fn make_commitment(batch_id: &str, feature_count: usize) -> BatchCommitment {
        let circuit = ZKPCircuit::new(None);
        let feature_values: Vec<f64> = (0..feature_count).map(|i| i as f64 * 1.1).collect();
        circuit.create_commitment(&feature_values, batch_id).unwrap()
    }

    #[test]
    fn test_accumulator_creation() {
        let acc = BatchAccumulator::new();
        assert_eq!(acc.config().max_capacity, 64);
        assert_eq!(acc.config().auto_accumulate_threshold, 16);
    }

    #[test]
    fn test_accumulator_with_config() {
        let config = AccumulatorConfig {
            max_capacity: 32,
            auto_accumulate_threshold: 8,
            stale_batch_timeout: Duration::from_secs(60),
        };
        let acc = BatchAccumulator::with_config(config);
        assert_eq!(acc.config().max_capacity, 32);
    }

    #[test]
    fn test_add_batch() {
        let acc = BatchAccumulator::new();
        let commitment = make_commitment("batch-1", 4);
        acc.add_batch("batch-1".to_string(), commitment).ok();
        assert_eq!(acc.get_stats().current_capacity, 1);
    }

    #[test]
    fn test_add_duplicate_batch() {
        let acc = BatchAccumulator::new();
        let c1 = make_commitment("dup-1", 4);
        let c2 = make_commitment("dup-1", 4);
        acc.add_batch("dup-1".to_string(), c1).ok();
        acc.add_batch("dup-1".to_string(), c2).ok();
        assert_eq!(acc.get_stats().current_capacity, 1);
    }

    #[test]
    fn test_add_batch_with_proof() {
        let acc = BatchAccumulator::new();
        let commitment = make_commitment("batch-proof", 4);
        let mut rng = thread_rng();
        let proof = ZKPProof {
            a: G1Projective::rand(&mut rng).into_affine(),
            b: vec![G1Projective::rand(&mut rng).into_affine()],
            c: G1Projective::rand(&mut rng).into_affine(),
            challenge: [0u8; 32],
            batch_id: "batch-proof".to_string(),
            feature_count: 4,
        };
        acc.add_batch_with_proof("batch-proof".to_string(), commitment, proof)
            .ok();
        let entry = acc.get_batch("batch-proof").unwrap();
        assert!(entry.proof.is_some());
    }

    #[test]
    fn test_accumulate_single_batch() {
        let acc = BatchAccumulator::new();
        let commitment = make_commitment("single", 4);
        acc.add_batch("single".to_string(), commitment).ok();
        let result = acc.accumulate();
        assert!(result.is_ok());
        let proof = result.unwrap();
        assert_eq!(proof.batch_count, 1);
        assert_eq!(proof.total_features, 4);
    }

    #[test]
    fn test_accumulate_multiple_batches() {
        let acc = BatchAccumulator::new();
        for i in 0..8 {
            let c = make_commitment(&format!("multi-{}", i), 4);
            acc.add_batch(format!("multi-{}", i), c).ok();
        }
        let result = acc.accumulate();
        assert!(result.is_ok());
        let proof = result.unwrap();
        assert_eq!(proof.batch_count, 8);
        assert_eq!(proof.total_features, 32);
    }

    #[test]
    fn test_empty_accumulator() {
        let acc = BatchAccumulator::new();
        let result = acc.accumulate();
        assert!(result.is_err());
    }

    #[test]
    fn test_capacity_exceeded() {
        let config = AccumulatorConfig {
            max_capacity: 3,
            ..Default::default()
        };
        let acc = BatchAccumulator::with_config(config);
        for i in 0..3 {
            let c = make_commitment(&format!("cap-{}", i), 2);
            acc.add_batch(format!("cap-{}", i), c).ok();
        }
        let c = make_commitment("cap-overflow", 2);
        let result = acc.add_batch("cap-overflow".to_string(), c);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_batch() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("get-test", 4);
        acc.add_batch("get-test".to_string(), c).ok();
        let entry = acc.get_batch("get-test");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().batch_id, "get-test");
    }

    #[test]
    fn test_get_missing_batch() {
        let acc = BatchAccumulator::new();
        assert!(acc.get_batch("nonexistent").is_none());
    }

    #[test]
    fn test_history_tracking() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("hist-1", 4);
        acc.add_batch("hist-1".to_string(), c).ok();
        acc.accumulate().ok();
        let history = acc.get_history();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_get_last_accumulation() {
        let acc = BatchAccumulator::new();
        assert!(acc.get_last_accumulation().is_none());
        let c = make_commitment("last-1", 4);
        acc.add_batch("last-1".to_string(), c).ok();
        acc.accumulate().ok();
        assert!(acc.get_last_accumulation().is_some());
    }

    #[test]
    fn test_clear() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("clear-1", 4);
        acc.add_batch("clear-1".to_string(), c).ok();
        acc.clear();
        assert_eq!(acc.get_stats().current_capacity, 0);
    }

    #[test]
    fn test_clear_history() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("ch-1", 4);
        acc.add_batch("ch-1".to_string(), c).ok();
        acc.accumulate().ok();
        acc.clear_history();
        assert!(acc.get_history().is_empty());
    }

    #[test]
    fn test_reset_stats() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("rs-1", 4);
        acc.add_batch("rs-1".to_string(), c).ok();
        acc.accumulate().ok();
        acc.reset_stats();
        let stats = acc.get_stats();
        assert_eq!(stats.total_accumulations, 0);
    }

    #[test]
    fn test_verify_accumulation_valid() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("verify-1", 4);
        acc.add_batch("verify-1".to_string(), c).ok();
        let proof = acc.accumulate().unwrap();
        assert!(BatchAccumulator::verify_accumulation(&proof));
    }

    #[test]
    fn test_is_batch_in_accumulation() {
        let acc = BatchAccumulator::new();
        let c = make_commitment("in-acc-1", 4);
        acc.add_batch("in-acc-1".to_string(), c).ok();
        let proof = acc.accumulate().unwrap();
        assert!(acc.is_batch_in_accumulation("in-acc-1", &proof));
        assert!(!acc.is_batch_in_accumulation("not-included", &proof));
    }

    #[test]
    fn test_config_default() {
        let config = AccumulatorConfig::default();
        assert_eq!(config.max_capacity, 64);
        assert_eq!(config.auto_accumulate_threshold, 16);
        assert_eq!(config.stale_batch_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_stats_default() {
        let stats = AccumulatorStats::default();
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.current_capacity, 0);
    }

    #[test]
    fn test_accumulator_default() {
        let acc = BatchAccumulator::default();
        assert_eq!(acc.config().max_capacity, 64);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let r1 = compute_merkle_root(&items);
        let r2 = compute_merkle_root(&items);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = compute_merkle_root(&[]);
        let expected: [u8; 32] = Sha256::digest(b"empty").into();
        assert_eq!(root, expected);
    }
}
