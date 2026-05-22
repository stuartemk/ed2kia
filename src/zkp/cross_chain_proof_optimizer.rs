//! Cross-Chain Proof Optimizer — Optimización de pruebas ZKP para verificación cross-chain
//!
//! Acumulación incremental, verificación paralela y caché de pruebas
//! para minimizar el tiempo de verificación cross-chain.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

/// Error del optimizador de pruebas cross-chain
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ProofOptimizerError {
    #[error("Prueba no encontrada: {0}")]
    ProofNotFound(String),
    #[error("Chain no soportada: {0}")]
    ChainNotSupported(String),
    #[error("Error de acumulación: {0}")]
    AccumulationError(String),
    #[error("Caché llena")]
    CacheFull,
    #[error("Error de verificación: {0}")]
    VerificationError(String),
}

/// Entrada de prueba cross-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainProof {
    /// Identificador único
    pub proof_id: String,
    /// Chain de origen
    pub source_chain: String,
    /// Chain de destino
    pub target_chain: String,
    /// Datos de la prueba
    pub proof_data: Vec<u8>,
    /// Timestamp en ms
    pub timestamp_ms: u64,
    /// ¿Verificada?
    pub verified: bool,
    /// Tiempo de verificación en ms
    pub verification_time_ms: f64,
}

impl CrossChainProof {
    pub fn new(
        proof_id: String,
        source_chain: String,
        target_chain: String,
        proof_data: Vec<u8>,
    ) -> Self {
        Self {
            proof_id,
            source_chain,
            target_chain,
            proof_data,
            timestamp_ms: current_timestamp_ms(),
            verified: false,
            verification_time_ms: 0.0,
        }
    }

    pub fn mark_verified(&mut self, time_ms: f64) {
        self.verified = true;
        self.verification_time_ms = time_ms;
    }
}

/// Entrada de caché
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub proof_id: String,
    pub chain_pair: String,
    pub proof_hash: String,
    pub verified: bool,
    pub timestamp_ms: u64,
    pub hit_count: usize,
}

impl CacheEntry {
    pub fn new(proof_id: String, chain_pair: String, proof_hash: String, verified: bool) -> Self {
        Self {
            proof_id,
            chain_pair,
            proof_hash,
            verified,
            timestamp_ms: current_timestamp_ms(),
            hit_count: 0,
        }
    }

    pub fn record_hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Estadísticas del optimizador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerStats {
    pub total_proofs: usize,
    pub verified_proofs: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub accumulated_proofs: usize,
    pub avg_verification_time_ms: f64,
    pub total_verification_time_ms: f64,
}

impl Default for OptimizerStats {
    fn default() -> Self {
        Self {
            total_proofs: 0,
            verified_proofs: 0,
            cache_hits: 0,
            cache_misses: 0,
            accumulated_proofs: 0,
            avg_verification_time_ms: 0.0,
            total_verification_time_ms: 0.0,
        }
    }
}

/// Configuración del optimizador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOptimizerConfig {
    /// Tamaño máximo de caché
    pub max_cache_size: usize,
    /// Edad máxima de entrada en caché (ms)
    pub max_cache_age_ms: u64,
    /// Tamaño máximo de batch para acumulación
    pub max_accumulation_batch: usize,
    /// Habilitar verificación paralela
    pub parallel_verification: bool,
    /// Número de workers
    pub verification_workers: usize,
    /// Chains soportadas
    pub supported_chains: Vec<String>,
}

impl Default for ProofOptimizerConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 500,
            max_cache_age_ms: 300_000,
            max_accumulation_batch: 50,
            parallel_verification: true,
            verification_workers: 4,
            supported_chains: vec![
                "ethereum".to_string(),
                "polygon".to_string(),
                "arbitrum".to_string(),
                "optimism".to_string(),
                "ed2k".to_string(),
            ],
        }
    }
}

/// Optimizador de pruebas cross-chain
pub struct CrossChainProofOptimizer {
    config: ProofOptimizerConfig,
    proofs: Vec<CrossChainProof>,
    cache: Vec<CacheEntry>,
    stats: OptimizerStats,
    /// Acumulador de pruebas pendientes
    accumulation_buffer: Vec<CrossChainProof>,
}

impl CrossChainProofOptimizer {
    pub fn new() -> Self {
        Self::with_config(ProofOptimizerConfig::default())
    }

    pub fn with_config(config: ProofOptimizerConfig) -> Self {
        Self {
            config,
            proofs: Vec::new(),
            cache: Vec::new(),
            stats: OptimizerStats::default(),
            accumulation_buffer: Vec::new(),
        }
    }

    /// Registra una prueba cross-chain
    pub fn register_proof(&mut self, proof: CrossChainProof) -> Result<(), ProofOptimizerError> {
        // Validar chains
        if !self.is_chain_supported(&proof.source_chain) {
            return Err(ProofOptimizerError::ChainNotSupported(
                proof.source_chain.clone(),
            ));
        }
        if !self.is_chain_supported(&proof.target_chain) {
            return Err(ProofOptimizerError::ChainNotSupported(
                proof.target_chain.clone(),
            ));
        }

        // Verificar caché
        let proof_hash = compute_proof_hash(&proof.proof_data);
        if let Some(cache_entry) = self.check_cache(&proof_hash) {
            self.stats.cache_hits += 1;
            debug!(
                "Cache hit para prueba {} (hits: {})",
                proof.proof_id, cache_entry.hit_count
            );
            return Ok(());
        }
        self.stats.cache_misses += 1;

        let proof_id = proof.proof_id.clone();
        self.proofs.push(proof);
        self.stats.total_proofs += 1;
        info!("Prueba cross-chain registrada: {}", proof_id);
        Ok(())
    }

    /// Verifica una prueba por ID
    pub fn verify_proof(&mut self, proof_id: &str) -> Result<bool, ProofOptimizerError> {
        let proof = self
            .proofs
            .iter_mut()
            .find(|p| p.proof_id == proof_id)
            .ok_or(ProofOptimizerError::ProofNotFound(proof_id.to_string()))?;

        if proof.verified {
            return Ok(true);
        }

        let start = std::time::Instant::now();

        // Simulación de verificación
        let valid = !proof.proof_data.is_empty();

        let elapsed_ms = start.elapsed().as_secs_f64();
        proof.mark_verified(elapsed_ms);

        self.stats.verified_proofs += 1;
        self.stats.total_verification_time_ms += elapsed_ms;
        if self.stats.verified_proofs > 0 {
            self.stats.avg_verification_time_ms =
                self.stats.total_verification_time_ms / self.stats.verified_proofs as f64;
        }

        // Actualizar caché
        let chain_pair = format!("{}/{}", proof.source_chain, proof.target_chain);
        let proof_hash = compute_proof_hash(&proof.proof_data);
        let proof_id_for_cache = proof.proof_id.clone();
        self.update_cache(proof_id_for_cache, chain_pair, proof_hash, valid);

        if !valid {
            return Err(ProofOptimizerError::VerificationError(
                "Verificación fallida".to_string(),
            ));
        }

        info!("Prueba verificada: {} en {:.2}ms", proof_id, elapsed_ms);
        Ok(true)
    }

    /// Verifica un batch de pruebas
    pub fn verify_batch(
        &mut self,
        proof_ids: &[String],
    ) -> Result<Vec<(String, bool)>, ProofOptimizerError> {
        let mut results = Vec::with_capacity(proof_ids.len());
        for proof_id in proof_ids {
            let verified = self.verify_proof(proof_id);
            results.push((proof_id.clone(), verified.is_ok()));
        }
        Ok(results)
    }

    /// Agrega una prueba al buffer de acumulación
    pub fn accumulate_proof(&mut self, proof: CrossChainProof) -> Result<(), ProofOptimizerError> {
        if self.accumulation_buffer.len() >= self.config.max_accumulation_batch {
            self.flush_accumulation()?;
        }
        self.accumulation_buffer.push(proof);
        Ok(())
    }

    /// Procesa el buffer de acumulación
    pub fn flush_accumulation(&mut self) -> Result<usize, ProofOptimizerError> {
        if self.accumulation_buffer.is_empty() {
            return Ok(0);
        }

        let count = self.accumulation_buffer.len();
        info!("Procesando acumulación de {} pruebas", count);

        for proof in self.accumulation_buffer.drain(..) {
            self.proofs.push(proof);
            self.stats.total_proofs += 1;
            self.stats.accumulated_proofs += 1;
        }

        Ok(count)
    }

    /// Obtiene las pruebas no verificadas
    pub fn get_unverified_proofs(&self) -> Vec<&CrossChainProof> {
        self.proofs.iter().filter(|p| !p.verified).collect()
    }

    /// Limpia pruebas antiguas
    pub fn cleanup_old_proofs(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let before = self.proofs.len();
        self.proofs
            .retain(|p| now.saturating_sub(p.timestamp_ms) <= max_age_ms);
        before - self.proofs.len()
    }

    /// Obtiene las estadísticas
    pub fn get_stats(&self) -> &OptimizerStats {
        &self.stats
    }

    /// Obtiene las pruebas registradas
    pub fn get_proofs(&self) -> &[CrossChainProof] {
        &self.proofs
    }

    fn is_chain_supported(&self, chain: &str) -> bool {
        self.config.supported_chains.iter().any(|c| c == chain)
    }

    fn check_cache(&mut self, proof_hash: &str) -> Option<CacheEntry> {
        if let Some(entry) = self.cache.iter_mut().find(|e| e.proof_hash == proof_hash) {
            entry.record_hit();
            return Some(entry.clone());
        }
        None
    }

    fn update_cache(
        &mut self,
        proof_id: String,
        chain_pair: String,
        proof_hash: String,
        verified: bool,
    ) {
        if self.cache.len() >= self.config.max_cache_size {
            self.evict_old_cache_entries();
        }

        let entry = CacheEntry::new(proof_id, chain_pair, proof_hash, verified);
        self.cache.push(entry);
    }

    fn evict_old_cache_entries(&mut self) {
        let now = current_timestamp_ms();
        self.cache
            .retain(|e| now.saturating_sub(e.timestamp_ms) <= self.config.max_cache_age_ms);
        if self.cache.len() >= self.config.max_cache_size {
            let excess = self.cache.len() - self.config.max_cache_size / 2;
            self.cache.drain(..excess);
        }
    }
}

impl Default for CrossChainProofOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_proof_hash(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
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

    fn make_proof(id: &str, source: &str, target: &str) -> CrossChainProof {
        CrossChainProof::new(
            id.to_string(),
            source.to_string(),
            target.to_string(),
            vec![0xAB; 64],
        )
    }

    #[test]
    fn test_optimizer_creation() {
        let optimizer = CrossChainProofOptimizer::new();
        assert_eq!(optimizer.get_proofs().len(), 0);
        assert_eq!(optimizer.get_stats().total_proofs, 0);
    }

    #[test]
    fn test_register_proof() {
        let mut optimizer = CrossChainProofOptimizer::new();
        let proof = make_proof("p1", "ethereum", "polygon");
        optimizer.register_proof(proof).unwrap();
        assert_eq!(optimizer.get_proofs().len(), 1);
    }

    #[test]
    fn test_register_proof_unsupported_chain() {
        let mut optimizer = CrossChainProofOptimizer::new();
        let proof = make_proof("p1", "unknown-chain", "polygon");
        assert!(optimizer.register_proof(proof).is_err());
    }

    #[test]
    fn test_verify_proof() {
        let mut optimizer = CrossChainProofOptimizer::new();
        optimizer
            .register_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        assert!(optimizer.verify_proof("p1").unwrap());
    }

    #[test]
    fn test_verify_nonexistent_proof() {
        let mut optimizer = CrossChainProofOptimizer::new();
        assert!(optimizer.verify_proof("nonexistent").is_err());
    }

    #[test]
    fn test_verify_batch() {
        let mut optimizer = CrossChainProofOptimizer::new();
        optimizer
            .register_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        optimizer
            .register_proof(make_proof("p2", "arbitrum", "ed2k"))
            .unwrap();
        let results = optimizer
            .verify_batch(&["p1".to_string(), "p2".to_string()])
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(_, v)| *v));
    }

    #[test]
    fn test_accumulate_proof() {
        let mut optimizer = CrossChainProofOptimizer::new();
        optimizer
            .accumulate_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        let flushed = optimizer.flush_accumulation().unwrap();
        assert_eq!(flushed, 1);
        assert_eq!(optimizer.get_stats().accumulated_proofs, 1);
    }

    #[test]
    fn test_flush_empty_accumulation() {
        let mut optimizer = CrossChainProofOptimizer::new();
        let flushed = optimizer.flush_accumulation().unwrap();
        assert_eq!(flushed, 0);
    }

    #[test]
    fn test_get_unverified_proofs() {
        let mut optimizer = CrossChainProofOptimizer::new();
        optimizer
            .register_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        let unverified = optimizer.get_unverified_proofs();
        assert_eq!(unverified.len(), 1);
    }

    #[test]
    fn test_cleanup_old_proofs() {
        let mut optimizer = CrossChainProofOptimizer::new();
        optimizer
            .register_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let removed = optimizer.cleanup_old_proofs(0);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_cache_hit() {
        let mut optimizer = CrossChainProofOptimizer::new();
        let data = vec![0xAB; 64];
        let proof1 = CrossChainProof::new(
            "p1".to_string(),
            "ethereum".to_string(),
            "polygon".to_string(),
            data.clone(),
        );
        // Register first proof: populates cache after verification
        optimizer.register_proof(proof1).unwrap();
        optimizer.verify_proof("p1").unwrap();
        // Register second proof with same data: should hit cache
        let proof2 = CrossChainProof::new(
            "p2".to_string(),
            "ethereum".to_string(),
            "polygon".to_string(),
            data,
        );
        optimizer.register_proof(proof2).unwrap();
        assert_eq!(optimizer.get_stats().cache_hits, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut optimizer = CrossChainProofOptimizer::new();
        optimizer
            .register_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        optimizer.verify_proof("p1").unwrap();
        let stats = optimizer.get_stats();
        assert_eq!(stats.total_proofs, 1);
        assert_eq!(stats.verified_proofs, 1);
    }

    #[test]
    fn test_config_default() {
        let config = ProofOptimizerConfig::default();
        assert_eq!(config.max_cache_size, 500);
        assert!(config.parallel_verification);
        assert!(config.supported_chains.contains(&"ethereum".to_string()));
    }

    #[test]
    fn test_stats_default() {
        let stats = OptimizerStats::default();
        assert_eq!(stats.total_proofs, 0);
        assert_eq!(stats.cache_hits, 0);
    }

    #[test]
    fn test_optimizer_default() {
        let optimizer = CrossChainProofOptimizer::default();
        assert_eq!(optimizer.get_proofs().len(), 0);
    }

    #[test]
    fn test_proof_mark_verified() {
        let mut proof = make_proof("p1", "ethereum", "polygon");
        assert!(!proof.verified);
        proof.mark_verified(10.5);
        assert!(proof.verified);
        assert_eq!(proof.verification_time_ms, 10.5);
    }

    #[test]
    fn test_cache_entry_hit() {
        let mut entry = CacheEntry::new(
            "p1".to_string(),
            "eth/poly".to_string(),
            "abc123".to_string(),
            true,
        );
        assert_eq!(entry.hit_count, 0);
        entry.record_hit();
        assert_eq!(entry.hit_count, 1);
    }

    #[test]
    fn test_proof_hash_consistency() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = compute_proof_hash(&data);
        let h2 = compute_proof_hash(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_accumulation_auto_flush() {
        let mut optimizer = CrossChainProofOptimizer::with_config(ProofOptimizerConfig {
            max_accumulation_batch: 3,
            ..Default::default()
        });
        // Llenar el buffer (3 items)
        optimizer
            .accumulate_proof(make_proof("p1", "ethereum", "polygon"))
            .unwrap();
        optimizer
            .accumulate_proof(make_proof("p2", "ethereum", "polygon"))
            .unwrap();
        optimizer
            .accumulate_proof(make_proof("p3", "ethereum", "polygon"))
            .unwrap();
        // Buffer lleno pero sin flush automatico aun
        assert_eq!(optimizer.get_stats().accumulated_proofs, 0);
        // El 4to trigger auto-flush de los 3 acumulados
        optimizer
            .accumulate_proof(make_proof("p4", "ethereum", "polygon"))
            .unwrap();
        // Ahora 3 fueron procesados, 1 queda en buffer
        assert_eq!(optimizer.get_stats().accumulated_proofs, 3);
    }

    #[test]
    fn test_multiple_chains_supported() {
        let optimizer = CrossChainProofOptimizer::new();
        assert!(optimizer.is_chain_supported("ethereum"));
        assert!(optimizer.is_chain_supported("polygon"));
        assert!(optimizer.is_chain_supported("ed2k"));
        assert!(!optimizer.is_chain_supported("unknown"));
    }
}
