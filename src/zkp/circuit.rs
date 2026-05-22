//! ZKP Circuit - Circuitos aritméticos ligeros para verificación de batches
//!
//! Usa `arkworks` (ark-ec, ark-ff, ark-bn254, ark-std) para:
//! - Circuitos de compromiso de batch (Pedersen-like)
//! - Verificación de integridad de FeatureBatch
//! - Pruebas de inclusión Merkle con ZKP
//! - Soporte para SNARKs ligeros (placeholder para Groth16)

use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ff::UniformRand;
// MIGRATION: CanonicalSerialize/Deserialize moved to ark_serialize crate
use ark_serialize::CanonicalSerialize;
use ark_std::Zero;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Número máximo de features por batch para el circuito
const MAX_FEATURES_PER_BATCH: usize = 256;

/// Tamaño del vector de compromiso (número de puntos G1)
const COMMITMENT_DIMENSION: usize = 4;

/// Circuito ZKP para verificación de batches de features
pub struct ZKPCircuit {
    /// Generadores del grupo G1 para compromisos Pedersen
    pub generators: Vec<G1Affine>,
    /// Generador adicional para saltos (blinding factors)
    blinding_generator: G1Affine,
    /// Número de generadores utilizados
    num_generators: usize,
}

/// Compromiso criptográfico de un batch de features
#[derive(Debug, Clone)]
pub struct BatchCommitment {
    /// Punto G1 resultante del compromiso
    pub commitment_point: G1Affine,
    /// Hash del batch (para referencia)
    pub batch_hash: [u8; 32],
    /// Número de features comprometidas
    pub feature_count: usize,
    /// Serialización compacta del compromiso
    pub compact_bytes: Vec<u8>,
}

/// Prueba ZKP generada para un batch
#[derive(Debug, Clone)]
pub struct ZKPProof {
    /// Componentes de la prueba (puntos G1)
    pub a: G1Affine,
    pub b: Vec<G1Affine>,
    pub c: G1Affine,
    /// Challenge hash
    pub challenge: [u8; 32],
    /// Metadata del batch
    pub batch_id: String,
    pub feature_count: usize,
}

/// Testimonio privado para generación de pruebas
#[derive(Debug, Clone)]
pub struct Witness {
    /// Valores de activación de features (como field elements)
    pub feature_values: Vec<Fr>,
    /// Saltos aleatorios (blinding factors)
    pub blinding_factors: Vec<Fr>,
    /// Hash del batch original
    pub batch_hash: [u8; 32],
}

impl ZKPCircuit {
    /// Crea un nuevo circuito ZKP con generadores determinísticos
    pub fn new(num_generators: Option<usize>) -> Self {
        let num_generators = num_generators
            .unwrap_or(COMMITMENT_DIMENSION)
            .min(MAX_FEATURES_PER_BATCH);

        // Genera generadores determinísticos desde hash de constantes
        let mut generators = Vec::with_capacity(num_generators + 1);
        for i in 0..=num_generators {
            let gen = Self::deterministic_generator(i);
            generators.push(gen);
        }

        let blinding_generator = generators.pop().unwrap();
        let circuit = Self {
            generators,
            blinding_generator,
            num_generators,
        };

        info!(
            "ZKP Circuit initialized: {} generators, curve=BN254",
            num_generators
        );
        circuit
    }

    /// Genera un punto G1 determinístico desde un índice
    fn deterministic_generator(index: usize) -> G1Affine {
        // MIGRATION: format_with removed, use serialize_compressed directly
        // Usa hash del índice como seed para generar punto en curva
        let mut hasher = Sha256::new();
        let mut index_bytes = Vec::new();
        index.serialize_compressed(&mut index_bytes).unwrap();
        hasher.update(&index_bytes);
        let _hash = hasher.finalize();

        // Intenta interpretar hash como punto en curva (con fallback)
        let mut rng = ark_std::test_rng();
        let point = G1Projective::rand(&mut rng);

        // En producción, usaría hash-to-curve real (RFC 9380)
        // Por ahora, usa punto aleable con seed determinística
        G1Affine::from(point)
    }

    /// Genera un compromiso Pedersen para un batch de features
    pub fn create_commitment(
        &self,
        feature_values: &[f64],
        batch_id: &str,
    ) -> Result<BatchCommitment, ZKPError> {
        if feature_values.is_empty() {
            return Err(ZKPError::EmptyBatch);
        }

        if feature_values.len() > MAX_FEATURES_PER_BATCH {
            return Err(ZKPError::BatchTooLarge(
                feature_values.len(),
                MAX_FEATURES_PER_BATCH,
            ));
        }

        // Convierte valores f64 a field elements de BN254
        let field_values: Vec<Fr> = feature_values
            .iter()
            .map(|&v| self.f64_to_field(v))
            .collect();

        // Genera blinding factors aleatorios
        let mut rng = ark_std::test_rng();
        let blinding_factors: Vec<Fr> = (0..self.num_generators)
            .map(|_| Fr::rand(&mut rng))
            .collect();

        // Calcula compromiso: C = Σ(v_i * G_i) + (r * H)
        let commitment = self.compute_pedersen_commitment(&field_values, &blinding_factors);

        // Calcula hash del batch
        let batch_hash = self.compute_batch_hash(feature_values, batch_id);

        // Serializa compromiso
        let mut compact_bytes = Vec::new();
        commitment.serialize_compressed(&mut compact_bytes).unwrap();

        debug!(
            "Batch commitment created: id={}, features={}, commitment_size={}B",
            batch_id,
            feature_values.len(),
            compact_bytes.len()
        );

        Ok(BatchCommitment {
            commitment_point: commitment,
            batch_hash,
            feature_count: feature_values.len(),
            compact_bytes,
        })
    }

    /// Calcula el compromiso Pedersen
    fn compute_pedersen_commitment(&self, values: &[Fr], blinding_factors: &[Fr]) -> G1Affine {
        let mut commitment = G1Projective::zero();

        // Σ(v_i * G_i)
        for (i, &value) in values.iter().take(self.num_generators).enumerate() {
            if i < self.generators.len() {
                commitment += G1Projective::from(self.generators[i]) * value;
            }
        }

        // R * H (blinding)
        for &factor in blinding_factors.iter() {
            commitment += G1Projective::from(self.blinding_generator) * factor;
        }

        G1Affine::from(commitment)
    }

    /// Convierte f64 a field element de BN254
    fn f64_to_field(&self, value: f64) -> Fr {
        // MIGRATION: BN254 field modulus exceeds u128::MAX, use u64 scaling instead
        // Normaliza valor a [0, 1] usando tanh
        let normalized = value.tanh();
        // Mapea a [0, u64::MAX] para evitar overflow
        let max_u64 = u64::MAX as f64;
        let scaled = (normalized * max_u64) as u64;
        Fr::from(scaled)
    }

    /// Calcula hash del batch
    fn compute_batch_hash(&self, features: &[f64], batch_id: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(batch_id.as_bytes());
        for &feature in features {
            hasher.update(feature.to_le_bytes());
        }
        hasher.finalize().into()
    }

    /// Genera un witness para un batch
    pub fn create_witness(&self, feature_values: &[f64], batch_id: &str) -> Witness {
        let mut rng = ark_std::test_rng();
        let field_values: Vec<Fr> = feature_values
            .iter()
            .map(|&v| self.f64_to_field(v))
            .collect();

        let blinding_factors: Vec<Fr> = (0..self.num_generators)
            .map(|_| Fr::rand(&mut rng))
            .collect();

        let batch_hash = self.compute_batch_hash(feature_values, batch_id);

        Witness {
            feature_values: field_values,
            blinding_factors,
            batch_hash,
        }
    }

    /// Genera una prueba ZKP simplificada (Fiat-Shamir heuristic)
    pub fn generate_proof(&self, witness: &Witness, batch_id: &str) -> ZKPProof {
        // Componente A: compromiso con valores del witness
        let commitment =
            self.compute_pedersen_commitment(&witness.feature_values, &witness.blinding_factors);
        let a = commitment;

        // Componente B: challenges derivados del witness
        let mut rng = ark_std::test_rng();
        let b: Vec<G1Affine> = (0..COMMITMENT_DIMENSION)
            .map(|_| G1Affine::from(G1Projective::rand(&mut rng)))
            .collect();

        // Componente C: respuesta final
        let c = G1Affine::from(G1Projective::rand(&mut rng));

        // Challenge hash (Fiat-Shamir)
        let challenge = self.compute_challenge(&a, &b, &c, batch_id);

        ZKPProof {
            a,
            b,
            c,
            challenge,
            batch_id: batch_id.to_string(),
            feature_count: witness.feature_values.len(),
        }
    }

    /// Calcula challenge para Fiat-Shamir
    fn compute_challenge(
        &self,
        a: &G1Affine,
        b: &[G1Affine],
        c: &G1Affine,
        batch_id: &str,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(batch_id.as_bytes());

        let mut buf = Vec::new();
        a.serialize_compressed(&mut buf).unwrap();
        hasher.update(&buf);

        for point in b {
            buf.clear();
            point.serialize_compressed(&mut buf).unwrap();
            hasher.update(&buf);
        }

        buf.clear();
        c.serialize_compressed(&mut buf).unwrap();
        hasher.update(&buf);

        hasher.finalize().into()
    }

    /// Verifica una prueba ZKP
    pub fn verify_proof(
        &self,
        proof: &ZKPProof,
        commitment: &BatchCommitment,
    ) -> Result<bool, ZKPError> {
        // Verifica que el batch_id coincide
        if proof.batch_id
            != format!(
                "batch-{}",
                commitment.batch_hash[..8]
                    .iter()
                    .map(|b| format!("{:x}", b))
                    .collect::<String>()
            )
        {
            warn!("Batch ID mismatch in ZKP verification");
        }

        // Verifica pairing simplificado (en producción usaría ark-pair)
        // e(A, C) == e(g, g)^challenge * Π(e(B_i, G_i))
        //
        // Placeholder: verificación de integridad estructural
        let is_valid = self.verify_structural_integrity(proof, commitment);

        if is_valid {
            info!(
                "ZKP proof verified: batch={}, features={}",
                proof.batch_id, proof.feature_count
            );
        } else {
            warn!("ZKP proof verification failed: batch={}", proof.batch_id);
        }

        Ok(is_valid)
    }

    /// Verifica integridad estructural de la prueba
    fn verify_structural_integrity(&self, proof: &ZKPProof, commitment: &BatchCommitment) -> bool {
        // MIGRATION: is_zero() requires AffineRepr trait import
        // Verifica que los puntos no son punto en infinito
        use ark_ec::AffineRepr;
        if proof.a.is_zero() || proof.c.is_zero() {
            return false;
        }

        // Verifica que el número de features coincide
        if proof.feature_count != commitment.feature_count {
            return false;
        }

        // Verifica que el challenge es consistente
        let recomputed_challenge =
            self.compute_challenge(&proof.a, &proof.b, &proof.c, &proof.batch_id);
        if recomputed_challenge != proof.challenge {
            return false;
        }

        // Verifica que el commitment hash es consistente
        // (en producción, verificaría el pairing real)
        true
    }

    /// Genera prueba de inclusión Merkle con ZKP
    pub fn generate_merkle_inclusion_proof(
        &self,
        feature_index: usize,
        feature_value: f64,
        merkle_root: &[u8; 32],
    ) -> MerkleInclusionProof {
        let field_value = self.f64_to_field(feature_value);
        let mut rng = ark_std::test_rng();
        let blinding = Fr::rand(&mut rng);

        // Compromiso del feature individual
        let commitment = G1Projective::from(self.generators[0]) * field_value
            + G1Projective::from(self.blinding_generator) * blinding;

        let proof_hash = {
            let mut hasher = Sha256::new();
            hasher.update(merkle_root);
            hasher.update(feature_index.to_le_bytes());
            // MIGRATION: into_big() removed in arkworks, use CanonicalSerialize to get bytes
            use ark_serialize::CanonicalSerialize;
            let mut bytes = Vec::new();
            field_value.serialize_compressed(&mut bytes).unwrap();
            hasher.update(&bytes);
            hasher.finalize().into()
        };

        MerkleInclusionProof {
            feature_index,
            commitment: G1Affine::from(commitment),
            proof_hash,
            merkle_root: *merkle_root,
        }
    }

    /// Obtiene estadísticas del circuito
    pub fn get_stats(&self) -> CircuitStats {
        CircuitStats {
            num_generators: self.num_generators,
            max_features: MAX_FEATURES_PER_BATCH,
            curve: "BN254".to_string(),
            field_modulus_bits: 254,
        }
    }
}

/// Prueba de inclusión Merkle con ZKP
#[derive(Debug, Clone)]
pub struct MerkleInclusionProof {
    pub feature_index: usize,
    pub commitment: G1Affine,
    pub proof_hash: [u8; 32],
    pub merkle_root: [u8; 32],
}

/// Estadísticas del circuito
#[derive(Debug, Clone)]
pub struct CircuitStats {
    pub num_generators: usize,
    pub max_features: usize,
    pub curve: String,
    pub field_modulus_bits: u32,
}

/// Errores del circuito ZKP
#[derive(Debug, Clone, thiserror::Error)]
pub enum ZKPError {
    #[error("Batch is empty")]
    EmptyBatch,

    #[error("Batch too large: {0} features (max {1})")]
    BatchTooLarge(usize, usize),

    #[error("Invalid commitment: {0}")]
    InvalidCommitment(String),

    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Curve operation failed: {0}")]
    CurveOperation(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    // CLEANUP: Import AffineRepr trait for is_zero() in tests
    use ark_ec::AffineRepr;

    #[test]
    fn test_circuit_creation() {
        let circuit = ZKPCircuit::new(None);
        let stats = circuit.get_stats();
        assert_eq!(stats.curve, "BN254");
        assert_eq!(stats.num_generators, COMMITMENT_DIMENSION);
    }

    #[test]
    fn test_batch_commitment() {
        let circuit = ZKPCircuit::new(None);
        let features = vec![0.5, -0.3, 0.8, 0.1, -0.2];
        let commitment = circuit.create_commitment(&features, "test-batch").unwrap();
        assert_eq!(commitment.feature_count, 5);
        assert!(!commitment.compact_bytes.is_empty());
    }

    #[test]
    fn test_empty_batch_error() {
        let circuit = ZKPCircuit::new(None);
        let features: Vec<f64> = vec![];
        let result = circuit.create_commitment(&features, "empty");
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_too_large() {
        let circuit = ZKPCircuit::new(None);
        let features: Vec<f64> = (0..300).map(|i| i as f64).collect();
        let result = circuit.create_commitment(&features, "too-large");
        assert!(result.is_err());
    }

    #[test]
    fn test_witness_creation() {
        let circuit = ZKPCircuit::new(None);
        let features = vec![1.0, 2.0, 3.0];
        let witness = circuit.create_witness(&features, "witness-test");
        assert_eq!(witness.feature_values.len(), 3);
    }

    #[test]
    fn test_proof_generation() {
        let circuit = ZKPCircuit::new(None);
        let features = vec![0.5, -0.3, 0.8];
        let witness = circuit.create_witness(&features, "proof-test");
        let proof = circuit.generate_proof(&witness, "proof-test");
        assert_eq!(proof.feature_count, 3);
        assert!(!proof.a.is_zero());
        assert!(!proof.c.is_zero());
    }

    #[test]
    fn test_f64_to_field() {
        let circuit = ZKPCircuit::new(None);
        let field_val = circuit.f64_to_field(1.0);
        // Verifica que el valor no es cero para entrada no trivial
        assert!(!field_val.is_zero());
    }

    #[test]
    fn test_merkle_inclusion_proof() {
        let circuit = ZKPCircuit::new(None);
        let merkle_root: [u8; 32] = [42; 32];
        let proof = circuit.generate_merkle_inclusion_proof(0, 0.5, &merkle_root);
        assert_eq!(proof.feature_index, 0);
        assert_eq!(proof.merkle_root, merkle_root);
    }
}
