//! Proof of Computation - Prueba ligera de cómputo para staking
//!
//! Genera y verifica pruebas de cómputo válido para la red ed2kIA,
//! permitiendo a los nodos demostrar trabajo real sin revelar datos.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info};

/// Prueba de cómputo (Proof of Computation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingProof {
    /// ID del nodo que genera la prueba
    pub node_id: String,
    /// Nonce único para esta prueba
    pub nonce: u64,
    /// Commitment hash (hash de los datos de cómputo)
    pub commitment_hash: String,
    /// Timestamp de generación (Unix ms)
    pub timestamp: u64,
    /// Métricas de cómputo reportadas
    pub compute_metrics: ComputeMetrics,
    /// Firma (placeholder para integración con Ed25519)
    pub signature: Option<Vec<u8>>,
}

/// Métricas de cómputo del nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeMetrics {
    /// Samples procesados
    pub samples_processed: usize,
    /// Tiempo de cómputo en ms
    pub compute_time_ms: u64,
    /// Uso de memoria en MB
    pub memory_usage_mb: f64,
    /// Uso de CPU (%)
    pub cpu_usage_percent: f64,
    /// Hash de verificación de integridad
    pub integrity_hash: String,
}

impl ComputeMetrics {
    pub fn new(
        samples_processed: usize,
        compute_time_ms: u64,
        memory_usage_mb: f64,
        cpu_usage_percent: f64,
    ) -> Self {
        let integrity_hash = Self::compute_integrity_hash(
            samples_processed,
            compute_time_ms,
            memory_usage_mb,
            cpu_usage_percent,
        );

        Self {
            samples_processed,
            compute_time_ms,
            memory_usage_mb,
            cpu_usage_percent,
            integrity_hash,
        }
    }

    fn compute_integrity_hash(
        samples: usize,
        time_ms: u64,
        memory_mb: f64,
        cpu_pct: f64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(samples.to_le_bytes());
        hasher.update(time_ms.to_le_bytes());
        hasher.update(memory_mb.to_le_bytes());
        hasher.update(cpu_pct.to_le_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Validar que las métricas están dentro de rangos razonables
    pub fn is_valid(&self) -> bool {
        // Samples > 0
        if self.samples_processed == 0 {
            return false;
        }
        // Tiempo > 0
        if self.compute_time_ms == 0 {
            return false;
        }
        // CPU entre 0-100%
        if self.cpu_usage_percent < 0.0 || self.cpu_usage_percent > 100.0 {
            return false;
        }
        // Memoria > 0 y < 1TB
        if self.memory_usage_mb <= 0.0 || self.memory_usage_mb > 1_048_576.0 {
            return false;
        }
        true
    }
}

/// Generador de pruebas de staking
pub struct ProofGenerator {
    node_id: String,
    nonce_counter: u64,
}

impl ProofGenerator {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            nonce_counter: 0,
        }
    }

    /// Generar nueva prueba de cómputo
    pub fn generate_proof(&mut self, metrics: ComputeMetrics) -> Result<StakingProof> {
        self.nonce_counter += 1;

        let commitment_hash = Self::compute_commitment(
            &self.node_id,
            self.nonce_counter,
            &metrics.integrity_hash,
        );

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let proof = StakingProof {
            node_id: self.node_id.clone(),
            nonce: self.nonce_counter,
            commitment_hash,
            timestamp,
            compute_metrics: metrics,
            signature: None,
        };

        debug!(
            "Prueba generada: nonce={}, samples={}, time={}ms",
            proof.nonce,
            proof.compute_metrics.samples_processed,
            proof.compute_metrics.compute_time_ms
        );

        Ok(proof)
    }

    fn compute_commitment(node_id: &str, nonce: u64, integrity_hash: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(node_id.as_bytes());
        hasher.update(nonce.to_le_bytes());
        hasher.update(integrity_hash.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Obtener nonce actual
    pub fn current_nonce(&self) -> u64 {
        self.nonce_counter
    }
}

/// Verificador de pruebas de staking
pub struct ProofVerifier {
    /// Pruebas ya verificadas (para prevenir replay)
    verified_nonces: std::collections::HashSet<u64>,
    /// Edad máxima de prueba en segundos
    max_proof_age_seconds: u64,
}

impl ProofVerifier {
    pub fn new(max_proof_age_seconds: u64) -> Self {
        Self {
            verified_nonces: std::collections::HashSet::new(),
            max_proof_age_seconds,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(300) // 5 minutos
    }

    /// Verificar prueba de staking
    pub fn verify(&mut self, proof: &StakingProof) -> Result<VerificationResult> {
        // 1. Verificar métricas válidas
        if !proof.compute_metrics.is_valid() {
            return Ok(VerificationResult {
                valid: false,
                reason: "Invalid compute metrics".to_string(),
            });
        }

        // 2. Verificar nonce no repetido (anti-replay)
        if self.verified_nonces.contains(&proof.nonce) {
            return Ok(VerificationResult {
                valid: false,
                reason: format!("Replay detected: nonce {} already verified", proof.nonce),
            });
        }

        // 3. Verificar edad de la prueba
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let age_seconds = (now - proof.timestamp) / 1000;
        if age_seconds > self.max_proof_age_seconds {
            return Ok(VerificationResult {
                valid: false,
                reason: format!("Proof expired: {}s > {}s", age_seconds, self.max_proof_age_seconds),
            });
        }

        // 4. Verificar commitment hash
        let expected_commitment = ProofGenerator::compute_commitment(
            &proof.node_id,
            proof.nonce,
            &proof.compute_metrics.integrity_hash,
        );

        if proof.commitment_hash != expected_commitment {
            return Ok(VerificationResult {
                valid: false,
                reason: "Commitment hash mismatch".to_string(),
            });
        }

        // 5. Marcar nonce como verificado
        self.verified_nonces.insert(proof.nonce);

        info!(
            "Prueba verificada: node={}, nonce={}, samples={}",
            proof.node_id,
            proof.nonce,
            proof.compute_metrics.samples_processed
        );

        Ok(VerificationResult {
            valid: true,
            reason: "Proof verified successfully".to_string(),
        })
    }

    /// Obtener conteo de pruebas verificadas
    pub fn verified_count(&self) -> usize {
        self.verified_nonces.len()
    }

    /// Limpiar nonces antiguos (opcional)
    pub fn cleanup_old_nonces(&mut self, _max_age_seconds: u64) {
        // En implementación real, usaría un mapa timestamp→nonce
        // Por ahora, solo logueamos
        debug!("Cleanup de nonces antiguos ({} verificados)", self.verified_nonces.len());
    }
}

/// Resultado de verificación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub reason: String,
}

impl Default for ProofVerifier {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_generation() {
        let mut generator = ProofGenerator::new("test_node".to_string());
        let metrics = ComputeMetrics::new(1000, 500, 256.0, 45.0);
        let proof = generator.generate_proof(metrics).unwrap();

        assert_eq!(proof.node_id, "test_node");
        assert_eq!(proof.nonce, 1);
        assert!(proof.commitment_hash.len() > 0);
        assert_eq!(proof.compute_metrics.samples_processed, 1000);
    }

    #[test]
    fn test_proof_verification() {
        let mut generator = ProofGenerator::new("test_node".to_string());
        let mut verifier = ProofVerifier::with_defaults();

        let metrics = ComputeMetrics::new(500, 200, 128.0, 30.0);
        let proof = generator.generate_proof(metrics).unwrap();

        let result = verifier.verify(&proof).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_replay_prevention() {
        let mut generator = ProofGenerator::new("test_node".to_string());
        let mut verifier = ProofVerifier::with_defaults();

        let metrics = ComputeMetrics::new(100, 50, 64.0, 20.0);
        let proof = generator.generate_proof(metrics).unwrap();

        let result1 = verifier.verify(&proof).unwrap();
        assert!(result1.valid);

        let result2 = verifier.verify(&proof).unwrap();
        assert!(!result2.valid);
        assert!(result2.reason.contains("Replay"));
    }

    #[test]
    fn test_invalid_metrics() {
        let mut verifier = ProofVerifier::with_defaults();
        let proof = StakingProof {
            node_id: "bad".to_string(),
            nonce: 1,
            commitment_hash: "abc".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            compute_metrics: ComputeMetrics::new(0, 0, 0.0, 0.0), // Invalid
            signature: None,
        };

        let result = verifier.verify(&proof).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_compute_metrics_validation() {
        let valid = ComputeMetrics::new(100, 100, 64.0, 50.0);
        assert!(valid.is_valid());

        let zero_samples = ComputeMetrics::new(0, 100, 64.0, 50.0);
        assert!(!zero_samples.is_valid());

        let bad_cpu = ComputeMetrics::new(100, 100, 64.0, 150.0);
        assert!(!bad_cpu.is_valid());
    }

    #[test]
    fn test_nonce_increment() {
        let mut generator = ProofGenerator::new("node".to_string());
        let metrics = ComputeMetrics::new(10, 10, 10.0, 10.0);

        generator.generate_proof(metrics.clone()).unwrap();
        generator.generate_proof(metrics.clone()).unwrap();
        generator.generate_proof(metrics).unwrap();

        assert_eq!(generator.current_nonce(), 3);
    }
}
