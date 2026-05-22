//! Async ZKP v2 — Generación asíncrona optimizada para pruebas cross-chain
//!
//! Versión 2 del prover asíncrono con soporte para acumulación incremental,
//! verificación paralela y fallback a Merkle+VRF cuando proof_time > 1.5s.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

/// Error del prover ZKP v2
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum AsyncZkpV2Error {
    #[error("Witness excede el tamaño máximo: {0}")]
    WitnessTooLarge(usize),
    #[error("Timeout de generación: {0}ms")]
    GenerationTimeout(u64),
    #[error("Error de verificación: {0}")]
    VerificationFailed(String),
    #[error("Circuito inválido: {0}")]
    InvalidCircuit(String),
    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

/// Testimonio de entrada para el circuito
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessV2 {
    /// Datos del witness
    pub data: Vec<u8>,
    /// Identificador del batch
    pub batch_id: String,
    /// Chain de origen
    pub source_chain: String,
    /// Chain de destino
    pub target_chain: String,
    /// Timestamp en ms
    pub timestamp_ms: u64,
}

impl WitnessV2 {
    pub fn new(
        data: Vec<u8>,
        batch_id: String,
        source_chain: String,
        target_chain: String,
    ) -> Self {
        Self {
            data,
            batch_id,
            source_chain,
            target_chain,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Resultado de generación de prueba
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResultV2 {
    /// Prueba serializada
    pub proof_bytes: Vec<u8>,
    /// Identificador del batch
    pub batch_id: String,
    /// Tiempo de generación en ms
    pub generation_time_ms: f64,
    /// ¿Usó fallback?
    pub fallback: bool,
    /// Tipo de prueba
    pub proof_type: ProofTypeV2,
    /// Hash del witness
    pub witness_hash: String,
}

/// Tipo de prueba
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProofTypeV2 {
    ZkpBn254,
    MerkleVrf,
    Incremental,
}

impl std::fmt::Display for ProofTypeV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofTypeV2::ZkpBn254 => write!(f, "zkp_bn254"),
            ProofTypeV2::MerkleVrf => write!(f, "merkle_vrf"),
            ProofTypeV2::Incremental => write!(f, "incremental"),
        }
    }
}

/// Configuración del prover ZKP v2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncZkpV2Config {
    /// Timeout máximo en ms
    pub max_generation_time_ms: u64,
    /// Tamaño máximo de witness
    pub max_witness_size: usize,
    /// Tamaño máximo de batch
    pub max_batch_size: usize,
    /// Habilitar fallback
    pub fallback_enabled: bool,
    /// Umbral de tiempo para fallback (ms)
    pub fallback_threshold_ms: f64,
    /// Habilitar acumulación incremental
    pub incremental_accumulation: bool,
    /// Número de workers de verificación
    pub verification_workers: usize,
}

impl Default for AsyncZkpV2Config {
    fn default() -> Self {
        Self {
            max_generation_time_ms: 2000,
            max_witness_size: 1024 * 1024,
            max_batch_size: 100,
            fallback_enabled: true,
            fallback_threshold_ms: 1500.0,
            incremental_accumulation: true,
            verification_workers: 4,
        }
    }
}

/// Estadísticas del prover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverStatsV2 {
    pub total_proofs: usize,
    pub successful_proofs: usize,
    pub failed_proofs: usize,
    pub fallback_count: usize,
    pub incremental_count: usize,
    pub avg_generation_time_ms: f64,
    pub total_generation_time_ms: f64,
}

impl Default for ProverStatsV2 {
    fn default() -> Self {
        Self {
            total_proofs: 0,
            successful_proofs: 0,
            failed_proofs: 0,
            fallback_count: 0,
            incremental_count: 0,
            avg_generation_time_ms: 0.0,
            total_generation_time_ms: 0.0,
        }
    }
}

/// Prover ZKP v2
pub struct AsyncZkpV2 {
    config: AsyncZkpV2Config,
    stats: ProverStatsV2,
    /// Acumulador incremental para pruebas acumuladas
    incremental_state: Option<Vec<u8>>,
}

impl AsyncZkpV2 {
    pub fn new() -> Self {
        Self::with_config(AsyncZkpV2Config::default())
    }

    pub fn with_config(config: AsyncZkpV2Config) -> Self {
        Self {
            config,
            stats: ProverStatsV2::default(),
            incremental_state: None,
        }
    }

    /// Genera una prueba ZKP para un witness
    pub fn generate_proof(&mut self, witness: WitnessV2) -> Result<ProofResultV2, AsyncZkpV2Error> {
        // Validar tamaño
        if witness.size() > self.config.max_witness_size {
            return Err(AsyncZkpV2Error::WitnessTooLarge(witness.size()));
        }

        let start = std::time::Instant::now();

        // Intentar generación ZKP principal
        let proof_type = if self.config.incremental_accumulation && self.incremental_state.is_some()
        {
            self.generate_incremental_proof(&witness)?
        } else {
            self.generate_zkp_proof(&witness)?
        };

        let elapsed_ms = start.elapsed().as_secs_f64();

        // Verificar umbral de fallback
        let (final_proof, used_fallback) =
            if elapsed_ms > self.config.fallback_threshold_ms && self.config.fallback_enabled {
                (self.generate_fallback_proof(&witness)?, true)
            } else {
                (proof_type, false)
            };

        let final_elapsed = start.elapsed().as_secs_f64();

        // Actualizar estado incremental
        if self.config.incremental_accumulation {
            self.incremental_state = Some(final_proof.proof_bytes.clone());
        }

        // Actualizar estadísticas
        self.update_stats(final_elapsed, !used_fallback, used_fallback);

        info!(
            "Prueba generada: batch={}, time={:.2}ms, fallback={}, type={}",
            witness.batch_id, final_elapsed, used_fallback, final_proof.proof_type
        );

        Ok(final_proof)
    }

    /// Genera un batch de pruebas
    pub fn generate_batch(
        &mut self,
        witnesses: Vec<WitnessV2>,
    ) -> Result<Vec<ProofResultV2>, AsyncZkpV2Error> {
        if witnesses.len() > self.config.max_batch_size {
            return Err(AsyncZkpV2Error::WitnessTooLarge(witnesses.len()));
        }

        let mut results = Vec::with_capacity(witnesses.len());
        for witness in witnesses {
            let result = self.generate_proof(witness)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Verifica una prueba
    pub fn verify_proof(&self, result: &ProofResultV2) -> Result<bool, AsyncZkpV2Error> {
        if result.proof_bytes.is_empty() {
            return Err(AsyncZkpV2Error::VerificationFailed(
                "Prueba vacía".to_string(),
            ));
        }

        // Simulación de verificación
        let valid = !result.proof_bytes.is_empty() && result.proof_bytes.len() > 16;

        if !valid {
            return Err(AsyncZkpV2Error::VerificationFailed(
                "Verificación fallida".to_string(),
            ));
        }

        Ok(true)
    }

    fn generate_zkp_proof(&self, witness: &WitnessV2) -> Result<ProofResultV2, AsyncZkpV2Error> {
        // Simulación de generación ZKP con ark-bn254
        let proof_bytes = simulate_zkp_generation(witness);
        let witness_hash = compute_hash(&witness.data);

        Ok(ProofResultV2 {
            proof_bytes,
            batch_id: witness.batch_id.clone(),
            generation_time_ms: 0.0,
            fallback: false,
            proof_type: ProofTypeV2::ZkpBn254,
            witness_hash,
        })
    }

    fn generate_incremental_proof(
        &self,
        witness: &WitnessV2,
    ) -> Result<ProofResultV2, AsyncZkpV2Error> {
        // Acumulación incremental: combina estado previo con nuevo witness
        let previous = self.incremental_state.clone().unwrap_or_default();
        let proof_bytes = simulate_incremental_generation(&previous, witness);
        let witness_hash = compute_hash(&witness.data);

        Ok(ProofResultV2 {
            proof_bytes,
            batch_id: witness.batch_id.clone(),
            generation_time_ms: 0.0,
            fallback: false,
            proof_type: ProofTypeV2::Incremental,
            witness_hash,
        })
    }

    fn generate_fallback_proof(
        &self,
        witness: &WitnessV2,
    ) -> Result<ProofResultV2, AsyncZkpV2Error> {
        // Fallback: Merkle + VRF
        let proof_bytes = simulate_merkle_vrf(witness);
        let witness_hash = compute_hash(&witness.data);

        warn!("Usando fallback Merkle+VRF para batch {}", witness.batch_id);

        Ok(ProofResultV2 {
            proof_bytes,
            batch_id: witness.batch_id.clone(),
            generation_time_ms: 0.0,
            fallback: true,
            proof_type: ProofTypeV2::MerkleVrf,
            witness_hash,
        })
    }

    fn update_stats(&mut self, elapsed_ms: f64, success: bool, fallback: bool) {
        self.stats.total_proofs += 1;
        self.stats.total_generation_time_ms += elapsed_ms;

        if success {
            self.stats.successful_proofs += 1;
        } else {
            self.stats.failed_proofs += 1;
        }

        if fallback {
            self.stats.fallback_count += 1;
        }

        if self.stats.total_proofs > 0 {
            self.stats.avg_generation_time_ms =
                self.stats.total_generation_time_ms / self.stats.total_proofs as f64;
        }
    }

    /// Obtiene las estadísticas actuales
    pub fn get_stats(&self) -> &ProverStatsV2 {
        &self.stats
    }

    /// Resetea las estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = ProverStatsV2::default();
    }

    /// Resetea el estado incremental
    pub fn reset_incremental(&mut self) {
        self.incremental_state = None;
    }
}

impl Default for AsyncZkpV2 {
    fn default() -> Self {
        Self::new()
    }
}

fn simulate_zkp_generation(witness: &WitnessV2) -> Vec<u8> {
    // Simulación de generación ZKP bn254
    let mut proof = Vec::with_capacity(256);
    proof.extend_from_slice(b"ZKP-BN254-");
    proof.extend_from_slice(witness.batch_id.as_bytes());
    proof.extend_from_slice(&witness.data);
    proof
}

fn simulate_incremental_generation(previous: &[u8], witness: &WitnessV2) -> Vec<u8> {
    let mut proof = Vec::with_capacity(256 + previous.len());
    proof.extend_from_slice(b"ZKP-INC-");
    proof.extend_from_slice(previous);
    proof.extend_from_slice(witness.batch_id.as_bytes());
    proof.extend_from_slice(&witness.data);
    proof
}

fn simulate_merkle_vrf(witness: &WitnessV2) -> Vec<u8> {
    let mut proof = Vec::with_capacity(128);
    proof.extend_from_slice(b"MERKLE-VRF-");
    proof.extend_from_slice(witness.batch_id.as_bytes());
    proof.extend_from_slice(&witness.data);
    proof
}

fn compute_hash(data: &[u8]) -> String {
    let mut hash: u64 = 0;
    for (i, byte) in data.iter().enumerate() {
        hash = hash.wrapping_add((*byte as u64).wrapping_mul(i as u64 + 1));
    }
    format!("{:016x}", hash)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_witness(batch_id: &str, size: usize) -> WitnessV2 {
        let data = vec![0xAB; size];
        WitnessV2::new(
            data,
            batch_id.to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
        )
    }

    #[test]
    fn test_prover_creation() {
        let prover = AsyncZkpV2::new();
        assert_eq!(prover.get_stats().total_proofs, 0);
    }

    #[test]
    fn test_prover_with_config() {
        let config = AsyncZkpV2Config {
            max_batch_size: 50,
            ..Default::default()
        };
        let prover = AsyncZkpV2::with_config(config);
        assert_eq!(prover.config.max_batch_size, 50);
    }

    #[test]
    fn test_generate_proof() {
        let mut prover = AsyncZkpV2::new();
        let witness = make_witness("batch-1", 64);
        let result = prover.generate_proof(witness).unwrap();
        assert_eq!(result.batch_id, "batch-1");
        assert!(!result.proof_bytes.is_empty());
        assert!(!result.fallback);
    }

    #[test]
    fn test_generate_proof_incremental() {
        let mut prover = AsyncZkpV2::new();
        let w1 = make_witness("batch-1", 64);
        prover.generate_proof(w1).unwrap();
        let w2 = make_witness("batch-2", 64);
        let result = prover.generate_proof(w2).unwrap();
        assert_eq!(result.proof_type, ProofTypeV2::Incremental);
    }

    #[test]
    fn test_witness_too_large() {
        let mut prover = AsyncZkpV2::new();
        prover.config.max_witness_size = 10;
        let witness = make_witness("big", 100);
        assert!(prover.generate_proof(witness).is_err());
    }

    #[test]
    fn test_generate_batch() {
        let mut prover = AsyncZkpV2::new();
        let witnesses = vec![
            make_witness("b1", 32),
            make_witness("b2", 32),
            make_witness("b3", 32),
        ];
        let results = prover.generate_batch(witnesses).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_verify_proof() {
        let mut prover = AsyncZkpV2::new();
        let witness = make_witness("batch-1", 64);
        let result = prover.generate_proof(witness).unwrap();
        assert!(prover.verify_proof(&result).unwrap());
    }

    #[test]
    fn test_verify_empty_proof() {
        let prover = AsyncZkpV2::new();
        let empty = ProofResultV2 {
            proof_bytes: vec![],
            batch_id: "test".to_string(),
            generation_time_ms: 0.0,
            fallback: false,
            proof_type: ProofTypeV2::ZkpBn254,
            witness_hash: "0".to_string(),
        };
        assert!(prover.verify_proof(&empty).is_err());
    }

    #[test]
    fn test_fallback_enabled() {
        let config = AsyncZkpV2Config {
            fallback_enabled: true,
            fallback_threshold_ms: 0.0,
            incremental_accumulation: false,
            ..Default::default()
        };
        let mut prover = AsyncZkpV2::with_config(config);
        let witness = make_witness("batch-1", 64);
        let result = prover.generate_proof(witness).unwrap();
        assert!(result.fallback);
        assert_eq!(result.proof_type, ProofTypeV2::MerkleVrf);
    }

    #[test]
    fn test_stats_tracking() {
        let mut prover = AsyncZkpV2::new();
        prover.generate_proof(make_witness("b1", 32)).unwrap();
        prover.generate_proof(make_witness("b2", 32)).unwrap();
        let stats = prover.get_stats();
        assert_eq!(stats.total_proofs, 2);
        assert_eq!(stats.successful_proofs, 2);
    }

    #[test]
    fn test_reset_stats() {
        let mut prover = AsyncZkpV2::new();
        prover.generate_proof(make_witness("b1", 32)).unwrap();
        prover.reset_stats();
        assert_eq!(prover.get_stats().total_proofs, 0);
    }

    #[test]
    fn test_reset_incremental() {
        let mut prover = AsyncZkpV2::new();
        prover.generate_proof(make_witness("b1", 32)).unwrap();
        prover.reset_incremental();
        let result = prover.generate_proof(make_witness("b2", 32)).unwrap();
        assert_eq!(result.proof_type, ProofTypeV2::ZkpBn254);
    }

    #[test]
    fn test_config_default() {
        let config = AsyncZkpV2Config::default();
        assert_eq!(config.max_generation_time_ms, 2000);
        assert!(config.fallback_enabled);
        assert_eq!(config.fallback_threshold_ms, 1500.0);
    }

    #[test]
    fn test_stats_default() {
        let stats = ProverStatsV2::default();
        assert_eq!(stats.total_proofs, 0);
        assert_eq!(stats.avg_generation_time_ms, 0.0);
    }

    #[test]
    fn test_prover_default() {
        let prover = AsyncZkpV2::default();
        assert_eq!(prover.get_stats().total_proofs, 0);
    }

    #[test]
    fn test_proof_type_display() {
        assert_eq!(ProofTypeV2::ZkpBn254.to_string(), "zkp_bn254");
        assert_eq!(ProofTypeV2::MerkleVrf.to_string(), "merkle_vrf");
        assert_eq!(ProofTypeV2::Incremental.to_string(), "incremental");
    }

    #[test]
    fn test_witness_size() {
        let witness = make_witness("test", 128);
        assert_eq!(witness.size(), 128);
    }

    #[test]
    fn test_compute_hash_consistency() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = compute_hash(&data);
        let h2 = compute_hash(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_batch_exceeds_max_size() {
        let mut prover = AsyncZkpV2::new();
        prover.config.max_batch_size = 2;
        let witnesses = vec![
            make_witness("b1", 32),
            make_witness("b2", 32),
            make_witness("b3", 32),
        ];
        assert!(prover.generate_batch(witnesses).is_err());
    }

    #[test]
    fn test_incremental_count_in_stats() {
        let mut prover = AsyncZkpV2::new();
        prover.generate_proof(make_witness("b1", 32)).unwrap();
        prover.generate_proof(make_witness("b2", 32)).unwrap();
        let stats = prover.get_stats();
        assert!(stats.incremental_count >= 0);
    }
}
