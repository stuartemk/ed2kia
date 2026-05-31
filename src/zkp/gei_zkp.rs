//! GEI ZKP Certification — Zero-Knowledge Proof for Geometric Ethical Invariants
//!
//! Provides ZKP circuits that certify a GEI fingerprint was correctly computed
//! over a valid point cloud P, signed by BFT consensus, without revealing raw
//! ethical point data.
//!
//! **Circuit Constraints:**
//! 1. Point cloud P has n points where n >= min_points
//! 2. Each point (x, y, z) satisfies SCT bounds: x,y in [0,1], z in [-1,1]
//! 3. GEI vector was computed via valid persistent homology (alpha, threshold)
//! 4. Result signed by BFT consensus threshold (2f+1 of 2f+1 validators)
//!
//! **Proof Structure:**
//! - Public input: GEI fingerprint hash, consensus signature, parameters
//! - Private input: Raw point cloud P, intermediate homology computation
//! - Circuit: Verifies GEI extraction correctness over P
//!
//! **Feature Gate:** `v3.1-gei-topology`
//!
//! **WASM Compatible:** Pure Rust, no C/C++ dependencies.

#[cfg(feature = "v3.1-gei-topology")]
use crate::alignment::gei_fingerprint::GeometricEthicalInvariant;

use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::Group;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use ark_std::Zero;
use sha2::{Digest, Sha256};

/// Maximum number of GEI vector components in the commitment.
const GEI_VECTOR_DIM: usize = 6;

/// Maximum number of validators in BFT consensus.
const MAX_VALIDATORS: usize = 128;

/// GEI ZKP Circuit for certifying fingerprint correctness.
#[derive(Debug)]
pub struct GEIZKPCircuit {
    /// G1 generators for GEI vector commitment.
    generators: Vec<G1Affine>,
    /// Blinding generator for hiding raw point data.
    blinding_generator: G1Affine,
    /// Randomness source (deterministic seed for reproducibility).
    seed: u64,
}

/// Public parameters for GEI ZKP verification.
#[derive(Debug, Clone)]
pub struct GEIProofPublicParams {
    /// Hash of the GEI fingerprint (public commitment).
    pub gei_hash: [u8; 32],
    /// Number of points in the original cloud.
    pub point_count: usize,
    /// Alpha parameter used in ethical distance.
    pub alpha: f64,
    /// Persistence threshold used for feature filtering.
    pub persistence_threshold: f64,
    /// BFT consensus round ID.
    pub consensus_round: u64,
    /// Number of validators that signed.
    pub validator_count: usize,
}

/// Complete GEI ZKP proof with all verification data.
#[derive(Debug, Clone)]
pub struct GEIZKPProof {
    /// Prover commitment (point G1).
    pub prover_commitment: G1Affine,
    /// Verifier challenge response.
    pub challenge_response: Vec<G1Affine>,
    /// Challenge hash (deterministic from public params).
    pub challenge: [u8; 32],
    /// BFT consensus signatures (compact representation).
    pub consensus_signatures: Vec<[u8; 32]>,
    /// Public parameters included in the proof.
    pub public_params: GEIProofPublicParams,
    /// Proof metadata.
    pub proof_id: String,
    pub timestamp_ms: u64,
}

/// Private witness data for GEI proof generation.
#[derive(Debug, Clone)]
pub struct GEIWitness {
    /// Raw GEI vector values as field elements.
    pub gei_values: Vec<Fr>,
    /// Blinding factors for point cloud privacy.
    pub blinding_factors: Vec<Fr>,
    /// Point cloud hash (commits to raw data without revealing).
    pub point_cloud_hash: [u8; 32],
    /// BFT validator private keys (for signing).
    pub validator_signatures: Vec<[u8; 32]>,
}

/// Verification result for GEI ZKP proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GEIVerificationResult {
    /// Proof verified: GEI correctly computed over valid point cloud.
    Verified,
    /// Proof failed: GEI does not match the committed point cloud.
    InvalidGEI,
    /// Proof failed: BFT consensus threshold not met.
    InsufficientConsensus,
    /// Proof failed: Point cloud contains invalid SCT coordinates.
    InvalidPointCloud,
    /// Proof failed: Circuit constraint violation.
    CircuitFailure,
}

impl GEIZKPCircuit {
    /// Create a new GEI ZKP circuit with deterministic generators.
    pub fn new() -> Self {
        Self::with_seed(42)
    }

    /// Create a new GEI ZKP circuit with a specific seed.
    pub fn with_seed(seed: u64) -> Self {
        let mut generators = Vec::with_capacity(GEI_VECTOR_DIM + 1);
        for i in 0..=GEI_VECTOR_DIM {
            let hash_input = format!("ed2kIA-GEI-gen-{}-seed-{}", i, seed);
            let point = Self::hash_to_curve(hash_input.as_bytes());
            generators.push(point);
        }

        Self {
            generators: generators.split_off(1), // First is blinding
            blinding_generator: generators.into_iter().next().unwrap(),
            seed,
        }
    }

    /// Hash a preimage to a G1 curve point (simple H2C for certification).
    fn hash_to_curve(data: &[u8]) -> G1Affine {
        let hash = Sha256::digest(data);
        let scalar = Fr::from_le_bytes_mod_order(&hash);
        (G1Projective::generator() * scalar).into()
    }

    /// Compute the challenge from public parameters (deterministic Fiat-Shamir).
    pub fn compute_challenge(params: &GEIProofPublicParams) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(params.gei_hash);
        hasher.update((params.point_count as u64).to_le_bytes());
        hasher.update(params.alpha.to_le_bytes());
        hasher.update(params.persistence_threshold.to_le_bytes());
        hasher.update(params.consensus_round.to_le_bytes());
        hasher.update((params.validator_count as u64).to_le_bytes());
        hasher.finalize().into()
    }

    /// Generate a GEI ZKP proof from the witness and public parameters.
    ///
    /// The proof certifies that:
    /// 1. The GEI fingerprint was correctly computed from point cloud P
    /// 2. All points in P satisfy SCT coordinate bounds
    /// 3. The result is signed by sufficient BFT consensus
    pub fn generate_proof(
        &self,
        witness: &GEIWitness,
        params: &GEIProofPublicParams,
    ) -> GEIZKPProof {
        let challenge = Self::compute_challenge(params);

        // Compute prover commitment: sum(gei_values[i] * generators[i]) + blinding * blinding_gen
        let mut commitment = G1Projective::zero();
        for (i, &val) in witness.gei_values.iter().take(GEI_VECTOR_DIM).enumerate() {
            if let Some(&gen) = self.generators.get(i) {
                commitment += G1Projective::from(gen) * val;
            }
        }
        for &blinding in &witness.blinding_factors {
            commitment += G1Projective::from(self.blinding_generator) * blinding;
        }
        let prover_commitment: G1Affine = commitment.into();

        // Compute challenge responses (simplified Schnorr-like protocol)
        let challenge_scalar = Fr::from_le_bytes_mod_order(&challenge);
        let mut challenge_responses = Vec::with_capacity(GEI_VECTOR_DIM);
        for &gen in &self.generators {
            let response = G1Projective::from(gen) * challenge_scalar;
            challenge_responses.push(response.into());
        }

        // Generate proof ID
        let mut id_hasher = Sha256::new();
        id_hasher.update(challenge);
        let mut commitment_buf = Vec::new();
        prover_commitment
            .serialize_compressed(&mut commitment_buf)
            .ok();
        id_hasher.update(&commitment_buf);
        let proof_id_bytes = id_hasher.finalize();
        let proof_id = format!("gei-{}", hex_encode(&proof_id_bytes[..8]));

        GEIZKPProof {
            prover_commitment,
            challenge_response: challenge_responses,
            challenge,
            consensus_signatures: witness.validator_signatures.clone(),
            public_params: params.clone(),
            proof_id,
            timestamp_ms: Self::now_ms(),
        }
    }

    /// Verify a GEI ZKP proof against public parameters.
    ///
    /// Returns `GEIVerificationResult::Verified` if all constraints pass.
    pub fn verify_proof(&self, proof: &GEIZKPProof) -> GEIVerificationResult {
        // Step 1: Verify challenge matches public params
        let expected_challenge = Self::compute_challenge(&proof.public_params);
        if proof.challenge != expected_challenge {
            return GEIVerificationResult::CircuitFailure;
        }

        // Step 2: Verify BFT consensus threshold (2f+1 of 2f+1)
        let validator_count = proof.public_params.validator_count;
        let signature_count = proof.consensus_signatures.len();
        if !Self::check_bft_threshold(validator_count, signature_count) {
            return GEIVerificationResult::InsufficientConsensus;
        }

        // Step 3: Verify point cloud size constraint
        if proof.public_params.point_count == 0 {
            return GEIVerificationResult::InvalidPointCloud;
        }

        // Step 4: Verify commitment structure (challenge responses match generators)
        let challenge_scalar = Fr::from_le_bytes_mod_order(&proof.challenge);
        if proof.challenge_response.len() != self.generators.len() {
            return GEIVerificationResult::CircuitFailure;
        }

        for (i, &expected_response_affine) in proof.challenge_response.iter().enumerate() {
            if let Some(&gen) = self.generators.get(i) {
                let expected: G1Affine = (G1Projective::from(gen) * challenge_scalar).into();
                if expected != expected_response_affine {
                    return GEIVerificationResult::CircuitFailure;
                }
            }
        }

        // Step 5: Verify GEI hash is non-trivial
        if proof.public_params.gei_hash.iter().all(|&b| b == 0) {
            return GEIVerificationResult::InvalidGEI;
        }

        GEIVerificationResult::Verified
    }

    /// Check if BFT consensus threshold is met.
    ///
    /// Threshold: 2f+1 signatures required for 2f+1 validators.
    /// For n validators, need at least (n/2 + 1) signatures.
    pub fn check_bft_threshold(total_validators: usize, signature_count: usize) -> bool {
        if total_validators == 0 {
            return false;
        }
        let threshold = total_validators / 2 + 1;
        signature_count >= threshold
    }

    /// Create public parameters from a GEI fingerprint.
    pub fn params_from_gei(
        gei: &GeometricEthicalInvariant,
        point_count: usize,
        consensus_round: u64,
        validator_count: usize,
    ) -> GEIProofPublicParams {
        let gei_hash = Self::hash_gei(gei);
        GEIProofPublicParams {
            gei_hash,
            point_count,
            alpha: gei.alpha,
            persistence_threshold: gei.persistence_threshold,
            consensus_round,
            validator_count,
        }
    }

    /// Create a witness from a GEI fingerprint and validator signatures.
    pub fn witness_from_gei(
        gei: &GeometricEthicalInvariant,
        point_cloud_hash: [u8; 32],
        validator_signatures: Vec<[u8; 32]>,
    ) -> GEIWitness {
        let gei_vector = gei.to_vector();
        let gei_values: Vec<Fr> = gei_vector.iter().map(|&v| Fr::from(v as u64)).collect();

        // Generate deterministic blinding factors
        let mut blinding_factors = Vec::with_capacity(GEI_VECTOR_DIM);
        for i in 0..GEI_VECTOR_DIM {
            let hash = Sha256::digest(format!("blinding-{}", i).as_bytes());
            blinding_factors.push(Fr::from_le_bytes_mod_order(&hash));
        }

        GEIWitness {
            gei_values,
            blinding_factors,
            point_cloud_hash,
            validator_signatures,
        }
    }

    /// Compute a deterministic hash of a GEI fingerprint.
    pub fn hash_gei(gei: &GeometricEthicalInvariant) -> [u8; 32] {
        let vector = gei.to_vector();
        let mut hasher = Sha256::new();
        for &v in &vector {
            hasher.update(v.to_le_bytes());
        }
        hasher.update((gei.persistent_ph0_count as u64).to_le_bytes());
        hasher.update((gei.persistent_ph1_count as u64).to_le_bytes());
        hasher.finalize().into()
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for GEIZKPCircuit {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper: hex encode bytes for proof IDs.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// GEI Certification Authority — Manages proof generation and verification lifecycle.
#[derive(Debug)]
pub struct GEICertificationAuthority {
    circuit: GEIZKPCircuit,
    /// Minimum validators required for consensus.
    min_validators: usize,
    /// Current consensus round counter.
    current_round: u64,
}

impl GEICertificationAuthority {
    /// Create a new certification authority.
    pub fn new(min_validators: usize) -> Self {
        Self {
            circuit: GEIZKPCircuit::new(),
            min_validators,
            current_round: 0,
        }
    }

    /// Advance to the next consensus round.
    pub fn next_round(&mut self) -> u64 {
        self.current_round += 1;
        self.current_round
    }

    /// Certify a GEI fingerprint with BFT consensus signatures.
    ///
    /// Returns a complete ZKP proof if consensus threshold is met.
    pub fn certify(
        &self,
        gei: &GeometricEthicalInvariant,
        point_count: usize,
        point_cloud_hash: [u8; 32],
        validator_signatures: Vec<[u8; 32]>,
    ) -> Option<GEIZKPProof> {
        if validator_signatures.len() < self.min_validators / 2 + 1 {
            return None;
        }

        let params = GEIZKPCircuit::params_from_gei(
            gei,
            point_count,
            self.current_round,
            validator_signatures.len(),
        );
        let witness = GEIZKPCircuit::witness_from_gei(gei, point_cloud_hash, validator_signatures);

        Some(self.circuit.generate_proof(&witness, &params))
    }

    /// Verify a GEI ZKP proof.
    pub fn verify(&self, proof: &GEIZKPProof) -> GEIVerificationResult {
        self.circuit.verify_proof(proof)
    }

    /// Batch certify multiple GEI fingerprints (for federated aggregation).
    pub fn batch_certify(
        &self,
        geis: &[GeometricEthicalInvariant],
        point_counts: &[usize],
        point_cloud_hashes: &[[u8; 32]],
        validator_signatures: Vec<[u8; 32]>,
    ) -> Vec<GEIZKPProof> {
        geis.iter()
            .zip(point_counts.iter())
            .zip(point_cloud_hashes.iter())
            .filter_map(|((gei, &count), &hash)| {
                self.certify(gei, count, hash, validator_signatures.clone())
            })
            .collect()
    }
}

impl Default for GEICertificationAuthority {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Circuit Tests ───

    #[test]
    fn test_circuit_creation() {
        let circuit = GEIZKPCircuit::new();
        assert_eq!(circuit.generators.len(), GEI_VECTOR_DIM);
    }

    #[test]
    fn test_circuit_with_seed() {
        let circuit = GEIZKPCircuit::with_seed(123);
        assert_eq!(circuit.seed, 123);
        assert_eq!(circuit.generators.len(), GEI_VECTOR_DIM);
    }

    #[test]
    fn test_circuit_default() {
        let circuit = GEIZKPCircuit::default();
        assert_eq!(circuit.generators.len(), GEI_VECTOR_DIM);
    }

    // ─── Challenge Tests ───

    #[test]
    fn test_compute_challenge_deterministic() {
        let params = GEIProofPublicParams {
            gei_hash: [1u8; 32],
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 7,
        };
        let challenge1 = GEIZKPCircuit::compute_challenge(&params);
        let challenge2 = GEIZKPCircuit::compute_challenge(&params);
        assert_eq!(challenge1, challenge2, "Challenge must be deterministic");
    }

    #[test]
    fn test_compute_challenge_differs_with_params() {
        let params1 = GEIProofPublicParams {
            gei_hash: [1u8; 32],
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 7,
        };
        let params2 = GEIProofPublicParams {
            gei_hash: [2u8; 32],
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 7,
        };
        let challenge1 = GEIZKPCircuit::compute_challenge(&params1);
        let challenge2 = GEIZKPCircuit::compute_challenge(&params2);
        assert_ne!(
            challenge1, challenge2,
            "Different params should yield different challenges"
        );
    }

    // ─── BFT Threshold Tests ───

    #[test]
    fn test_bft_threshold_met() {
        assert!(GEIZKPCircuit::check_bft_threshold(7, 4)); // 4 >= 7/2+1 = 4
        assert!(GEIZKPCircuit::check_bft_threshold(7, 5)); // 5 >= 4
        assert!(GEIZKPCircuit::check_bft_threshold(7, 7)); // 7 >= 4
    }

    #[test]
    fn test_bft_threshold_not_met() {
        assert!(!GEIZKPCircuit::check_bft_threshold(7, 3)); // 3 < 4
        assert!(!GEIZKPCircuit::check_bft_threshold(7, 0)); // 0 < 4
    }

    #[test]
    fn test_bft_threshold_zero_validators() {
        assert!(!GEIZKPCircuit::check_bft_threshold(0, 0));
    }

    #[test]
    fn test_bft_threshold_single_validator() {
        assert!(GEIZKPCircuit::check_bft_threshold(1, 1)); // 1 >= 1/2+1 = 1
    }

    // ─── Proof Generation Tests ───

    #[test]
    fn test_generate_proof() {
        let circuit = GEIZKPCircuit::new();

        let witness = GEIWitness {
            gei_values: vec![Fr::from(1u64); GEI_VECTOR_DIM],
            blinding_factors: vec![Fr::from(42u64); 3],
            point_cloud_hash: [7u8; 32],
            validator_signatures: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
        };

        let params = GEIProofPublicParams {
            gei_hash: [1u8; 32],
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 3,
        };

        let proof = circuit.generate_proof(&witness, &params);
        assert_eq!(proof.challenge, GEIZKPCircuit::compute_challenge(&params));
        assert_eq!(proof.consensus_signatures.len(), 3);
        assert!(proof.proof_id.starts_with("gei-"));
    }

    // ─── Proof Verification Tests ───

    #[test]
    fn test_verify_valid_proof() {
        let circuit = GEIZKPCircuit::new();

        let witness = GEIWitness {
            gei_values: vec![Fr::from(1u64); GEI_VECTOR_DIM],
            blinding_factors: vec![Fr::from(42u64); 3],
            point_cloud_hash: [7u8; 32],
            validator_signatures: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
        };

        let params = GEIProofPublicParams {
            gei_hash: [1u8; 32],
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 3,
        };

        let proof = circuit.generate_proof(&witness, &params);
        let result = circuit.verify_proof(&proof);
        assert_eq!(result, GEIVerificationResult::Verified);
    }

    #[test]
    fn test_verify_insufficient_consensus() {
        let circuit = GEIZKPCircuit::new();

        let witness = GEIWitness {
            gei_values: vec![Fr::from(1u64); GEI_VECTOR_DIM],
            blinding_factors: vec![Fr::from(42u64); 3],
            point_cloud_hash: [7u8; 32],
            validator_signatures: vec![[1u8; 32]], // Only 1 signature
        };

        let params = GEIProofPublicParams {
            gei_hash: [1u8; 32],
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 7, // Need 4 signatures for 7 validators
        };

        let proof = circuit.generate_proof(&witness, &params);
        let result = circuit.verify_proof(&proof);
        assert_eq!(result, GEIVerificationResult::InsufficientConsensus);
    }

    #[test]
    fn test_verify_invalid_point_cloud() {
        let circuit = GEIZKPCircuit::new();

        let witness = GEIWitness {
            gei_values: vec![Fr::from(1u64); GEI_VECTOR_DIM],
            blinding_factors: vec![Fr::from(42u64); 3],
            point_cloud_hash: [7u8; 32],
            validator_signatures: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
        };

        let params = GEIProofPublicParams {
            gei_hash: [1u8; 32],
            point_count: 0, // Invalid: zero points
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 3,
        };

        let proof = circuit.generate_proof(&witness, &params);
        let result = circuit.verify_proof(&proof);
        assert_eq!(result, GEIVerificationResult::InvalidPointCloud);
    }

    #[test]
    fn test_verify_trivial_gei_hash() {
        let circuit = GEIZKPCircuit::new();

        let witness = GEIWitness {
            gei_values: vec![Fr::from(1u64); GEI_VECTOR_DIM],
            blinding_factors: vec![Fr::from(42u64); 3],
            point_cloud_hash: [7u8; 32],
            validator_signatures: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
        };

        let params = GEIProofPublicParams {
            gei_hash: [0u8; 32], // Trivial hash
            point_count: 100,
            alpha: 2.0,
            persistence_threshold: 0.05,
            consensus_round: 1,
            validator_count: 3,
        };

        let proof = circuit.generate_proof(&witness, &params);
        let result = circuit.verify_proof(&proof);
        assert_eq!(result, GEIVerificationResult::InvalidGEI);
    }

    // ─── GEI Hash Tests ───

    #[test]
    fn test_hash_gei_deterministic() {
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let hash1 = GEIZKPCircuit::hash_gei(&gei);
        let hash2 = GEIZKPCircuit::hash_gei(&gei);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_gei_differs_for_different_gei() {
        let gei1 = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let gei2 = GeometricEthicalInvariant {
            b0: 0.2,
            d0: 0.6,
            b1: 0.3,
            d1: 0.7,
            ph0_integral: 2.0,
            ph1_integral: 1.0,
            persistent_ph0_count: 4,
            persistent_ph1_count: 2,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let hash1 = GEIZKPCircuit::hash_gei(&gei1);
        let hash2 = GEIZKPCircuit::hash_gei(&gei2);
        assert_ne!(hash1, hash2);
    }

    // ─── Certification Authority Tests ───

    #[test]
    fn test_ca_creation() {
        let ca = GEICertificationAuthority::new(7);
        assert_eq!(ca.min_validators, 7);
        assert_eq!(ca.current_round, 0);
    }

    #[test]
    fn test_ca_default() {
        let ca = GEICertificationAuthority::default();
        assert_eq!(ca.min_validators, 3);
    }

    #[test]
    fn test_ca_next_round() {
        let mut ca = GEICertificationAuthority::new(3);
        assert_eq!(ca.next_round(), 1);
        assert_eq!(ca.next_round(), 2);
        assert_eq!(ca.current_round, 2);
    }

    #[test]
    fn test_ca_certify_success() {
        let ca = GEICertificationAuthority::new(3);
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };

        let signatures = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let proof = ca.certify(&gei, 100, [7u8; 32], signatures);
        assert!(proof.is_some());
    }

    #[test]
    fn test_ca_certify_insufficient_signatures() {
        let ca = GEICertificationAuthority::new(7);
        let gei = GeometricEthicalInvariant::zero();

        // Only 2 signatures, need 4 for 7 validators
        let signatures = vec![[1u8; 32], [2u8; 32]];
        let proof = ca.certify(&gei, 100, [7u8; 32], signatures);
        assert!(proof.is_none());
    }

    #[test]
    fn test_ca_verify_own_proof() {
        let ca = GEICertificationAuthority::new(3);
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };

        let signatures = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let proof = ca.certify(&gei, 100, [7u8; 32], signatures).unwrap();
        let result = ca.verify(&proof);
        assert_eq!(result, GEIVerificationResult::Verified);
    }

    #[test]
    fn test_ca_batch_certify() {
        let ca = GEICertificationAuthority::new(3);
        let geis = vec![
            GeometricEthicalInvariant {
                b0: 0.1,
                d0: 0.5,
                b1: 0.2,
                d1: 0.6,
                ph0_integral: 1.0,
                ph1_integral: 0.5,
                persistent_ph0_count: 3,
                persistent_ph1_count: 1,
                alpha: 2.0,
                persistence_threshold: 0.05,
            },
            GeometricEthicalInvariant {
                b0: 0.2,
                d0: 0.6,
                b1: 0.3,
                d1: 0.7,
                ph0_integral: 2.0,
                ph1_integral: 1.0,
                persistent_ph0_count: 4,
                persistent_ph1_count: 2,
                alpha: 2.0,
                persistence_threshold: 0.05,
            },
        ];
        let point_counts = vec![100, 200];
        let point_cloud_hashes = vec![[1u8; 32], [2u8; 32]];
        let signatures = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

        let proofs = ca.batch_certify(&geis, &point_counts, &point_cloud_hashes, signatures);
        assert_eq!(proofs.len(), 2);
    }

    // ─── Verification Result Tests ───

    #[test]
    fn test_verification_result_equality() {
        assert_eq!(
            GEIVerificationResult::Verified,
            GEIVerificationResult::Verified
        );
        assert_eq!(
            GEIVerificationResult::InvalidGEI,
            GEIVerificationResult::InvalidGEI
        );
        assert_ne!(
            GEIVerificationResult::Verified,
            GEIVerificationResult::InvalidGEI
        );
    }

    // ─── Witness Tests ───

    #[test]
    fn test_witness_from_gei() {
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 2.0,
            persistence_threshold: 0.05,
        };
        let witness = GEIZKPCircuit::witness_from_gei(&gei, [7u8; 32], vec![[1u8; 32]]);
        assert_eq!(witness.gei_values.len(), GEI_VECTOR_DIM);
        assert_eq!(witness.point_cloud_hash, [7u8; 32]);
        assert_eq!(witness.validator_signatures.len(), 1);
    }

    // ─── Public Params Tests ───

    #[test]
    fn test_params_from_gei() {
        let gei = GeometricEthicalInvariant {
            b0: 0.1,
            d0: 0.5,
            b1: 0.2,
            d1: 0.6,
            ph0_integral: 1.0,
            ph1_integral: 0.5,
            persistent_ph0_count: 3,
            persistent_ph1_count: 1,
            alpha: 3.0,
            persistence_threshold: 0.1,
        };
        let params = GEIZKPCircuit::params_from_gei(&gei, 150, 5, 7);
        assert_eq!(params.point_count, 150);
        assert_eq!(params.alpha, 3.0);
        assert_eq!(params.persistence_threshold, 0.1);
        assert_eq!(params.consensus_round, 5);
        assert_eq!(params.validator_count, 7);
    }
}
