//! Proof of Symbiosis (PoSym) — Cryptographic Local Reputation.
//!
//! PoSym assigns each node a trust score based on its verified contributions
//! to the collective safety of the network. The score is computed as:
//!
//! ```text
//! Score = 0.6 * ln(steers + 1) + 0.3 * ln(1 + vfe_reduction) + 0.1 * ln(uptime + 1)
//! ```
//!
//! Where:
//! - `steers`: Number of certified steering corrections performed.
//! - `vfe_reduction`: Cumulative VFE (Variational Free Energy) reduction achieved.
//! - `uptime`: Node uptime in seconds (or blocks).
//!
//! Each contribution is cryptographically hashed and appended to a local
//! reputation chain, making it tamper-evident and auditable.
//!
//! # Sprint 122 — Game-Theoretic Extensions
//!
//! **Energy Impact Model:**
//! ```text
//! E = P_idle * (Δt / 3600) + FLOPs * energy_per_flop * hw_factor
//! hw_factor = 1.0 + 0.15 * ln(FLOPs / 1e12)  (heterogeneity adjustment)
//! ```
//!
//! **Game-Theoretic Trust Score:**
//! ```text
//! s_i(t+1) = (1-γ) * s_i(t) + α * (ΔVFE_i / cost_energy_i) + β * verify_proof(π_i)
//! ```
//! Where:
//! - `γ`: Trust decay rate (prevents stale reputation)
//! - `α`: Efficiency weight (VFE reduction per energy unit)
//! - `β`: Proof verification bonus
//! - Score clamped to [0.0, 1.0] for numerical stability
//!
//! **PAC-Bayesian Bound:**
//! ```text
//! bound = empirical_vfe + sqrt((KL + ln(1/δ)) / (2n))
//! ```
//! Provides probably-approximately-correct guarantees for collective trust.

use sha2::{Digest, Sha256};
use std::collections::VecDeque;

/// Cryptographic hash digest (SHA-256).
pub type Hash = [u8; 32];

/// A single certified steering contribution recorded in the PoSym chain.
#[derive(Debug, Clone)]
pub struct SteerContribution {
    /// Block or timestamp when the steer occurred.
    pub timestamp: u64,
    /// VFE before steering.
    pub vfe_before: f64,
    /// VFE after steering.
    pub vfe_after: f64,
    /// Cryptographic hash of the contribution.
    pub hash: Hash,
}

impl SteerContribution {
    /// Create a new steering contribution with cryptographic hash.
    pub fn new(timestamp: u64, vfe_before: f64, vfe_after: f64) -> Self {
        let hash = Self::compute_hash(timestamp, vfe_before, vfe_after);
        Self {
            timestamp,
            vfe_before,
            vfe_after,
            hash,
        }
    }

    /// Compute the VFE reduction from this contribution.
    pub fn vfe_reduction(&self) -> f64 {
        (self.vfe_before - self.vfe_after).max(0.0)
    }

    /// Generate SHA-256 hash for this contribution.
    fn compute_hash(timestamp: u64, vfe_before: f64, vfe_after: f64) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(timestamp.to_le_bytes());
        hasher.update(vfe_before.to_le_bytes());
        hasher.update(vfe_after.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify the hash matches the stored values.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(self.timestamp, self.vfe_before, self.vfe_after);
        self.hash == expected
    }
}

/// Proof of Symbiosis (PoSym) engine for a single node.
///
/// Tracks certified steering contributions and computes the trust score
/// based on the symbiotic formula.
#[derive(Debug, Clone)]
pub struct ProofOfSymbiosis {
    /// Node identifier.
    pub node_id: u64,
    /// Ordered chain of certified steering contributions.
    pub contributions: VecDeque<SteerContribution>,
    /// Maximum contributions to retain (prevents unbounded growth).
    pub max_contributions: usize,
    /// Node uptime in seconds (or blocks).
    pub uptime: u64,
    /// Weight for steering count component.
    pub weight_steers: f64,
    /// Weight for VFE reduction component.
    pub weight_vfe: f64,
    /// Weight for uptime component.
    pub weight_uptime: f64,
}

impl ProofOfSymbiosis {
    /// Create a new PoSym engine with default weights.
    ///
    /// Default formula: `Score = 0.6 * ln(steers+1) + 0.3 * ln(1+vfe_reduction) + 0.1 * ln(uptime+1)`
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            contributions: VecDeque::new(),
            max_contributions: 10_000,
            uptime: 0,
            weight_steers: 0.6,
            weight_vfe: 0.3,
            weight_uptime: 0.1,
        }
    }

    /// Create a new PoSym engine with custom weights.
    pub fn with_weights(
        node_id: u64,
        weight_steers: f64,
        weight_vfe: f64,
        weight_uptime: f64,
    ) -> Self {
        Self {
            node_id,
            contributions: VecDeque::new(),
            max_contributions: 10_000,
            uptime: 0,
            weight_steers,
            weight_vfe,
            weight_uptime,
        }
    }

    /// Record a certified steering contribution.
    ///
    /// Appends the contribution to the chain and enforces the maximum size limit.
    pub fn record_certified_steer(&mut self, timestamp: u64, vfe_before: f64, vfe_after: f64) {
        let contribution = SteerContribution::new(timestamp, vfe_before, vfe_after);
        self.contributions.push_back(contribution);
        if self.contributions.len() > self.max_contributions {
            self.contributions.pop_front();
        }
    }

    /// Increment node uptime by the given amount.
    pub fn add_uptime(&mut self, delta: u64) {
        self.uptime += delta;
    }

    /// Compute the current trust score using the symbiotic formula.
    ///
    /// ```text
    /// Score = w_steers * ln(steers + 1) + w_vfe * ln(1 + vfe_reduction) + w_uptime * ln(uptime + 1)
    /// ```
    pub fn compute_trust_score(&self) -> f64 {
        let steers = self.contributions.len() as f64;
        let total_vfe_reduction: f64 = self.contributions.iter().map(|c| c.vfe_reduction()).sum();
        let uptime = self.uptime as f64;

        let score_steers = self.weight_steers * (steers + 1.0).ln();
        let score_vfe = self.weight_vfe * (1.0 + total_vfe_reduction).ln();
        let score_uptime = self.weight_uptime * (uptime + 1.0).ln();

        score_steers + score_vfe + score_uptime
    }

    /// Generate a SHA-256 hash of the current PoSym state (for chain linking).
    pub fn generate_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.node_id.to_le_bytes());
        hasher.update(self.uptime.to_le_bytes());
        hasher.update((self.contributions.len() as u64).to_le_bytes());
        for contrib in &self.contributions {
            hasher.update(contrib.hash);
        }
        hasher.finalize().into()
    }

    /// Verify all contributions in the chain have valid hashes.
    pub fn verify_chain(&self) -> bool {
        self.contributions.iter().all(|c| c.verify())
    }

    /// Get the total VFE reduction achieved by this node.
    pub fn total_vfe_reduction(&self) -> f64 {
        self.contributions.iter().map(|c| c.vfe_reduction()).sum()
    }

    /// Get the number of certified steers.
    pub fn steer_count(&self) -> usize {
        self.contributions.len()
    }

    /// Get the most recent contribution (if any).
    pub fn latest_contribution(&self) -> Option<&SteerContribution> {
        self.contributions.back()
    }

    /// Check if this node's trust score exceeds a threshold.
    pub fn is_trusted(&self, threshold: f64) -> bool {
        self.compute_trust_score() >= threshold
    }
}

impl Default for ProofOfSymbiosis {
    fn default() -> Self {
        Self::new(0)
    }
}

// ---------------------------------------------------------------------------
// Sprint 122 — Game-Theoretic Extensions
// ---------------------------------------------------------------------------

/// Calculate energy impact of a compute operation.
///
/// ```text
/// E = P_idle * (Δt / 3600) + FLOPs * energy_per_flop * hw_factor
/// hw_factor = 1.0 + 0.15 * ln(FLOPs / 1e12)  (heterogeneity adjustment)
/// ```
///
/// # Arguments
/// * `idle_power_w` - Idle power consumption in Watts
/// * `delta_t_sec` - Duration of operation in seconds
/// * `total_flops` - Total floating-point operations performed
/// * `energy_per_flop` - Energy cost per FLOP (device-specific)
///
/// # Returns
/// Energy consumed in Watt-hours (Wh)
pub fn calculate_energy_impact(
    idle_power_w: f64,
    delta_t_sec: f64,
    total_flops: f64,
    energy_per_flop: f64,
) -> f64 {
    let idle_energy = idle_power_w * (delta_t_sec / 3600.0);
    let compute_energy = total_flops * energy_per_flop;
    // Hardware heterogeneity factor: accounts for edge device variance
    // Uses ln(FLOPs / 1e12) normalized to 1 TFLOP baseline
    let hw_factor = if total_flops > 0.0 {
        1.0 + 0.15 * (total_flops / 1e12).ln().max(0.0)
    } else {
        1.0
    };
    idle_energy + compute_energy * hw_factor
}

/// Game-theoretic trust score update.
///
/// ```text
/// s_i(t+1) = (1-γ) * s_i(t) + α * (ΔVFE_i / cost_energy_i) + β * verify_proof(π_i)
/// ```
///
/// # Arguments
/// * `current_score` - Current trust score s_i(t) ∈ [0, 1]
/// * `gamma` - Trust decay rate γ ∈ [0, 1] (prevents stale reputation)
/// * `alpha` - Efficiency weight α > 0 (VFE reduction per energy unit)
/// * `delta_vfe` - VFE reduction achieved by this contribution
/// * `cost_energy` - Energy cost of this contribution (Wh)
/// * `beta` - Proof verification bonus β ∈ [0, 1]
/// * `proof_valid` - Whether the cryptographic proof verified successfully
///
/// # Returns
/// Updated trust score clamped to [0.0, 1.0]
pub fn update_trust_score(
    current_score: f64,
    gamma: f64,
    alpha: f64,
    delta_vfe: f64,
    cost_energy: f64,
    beta: f64,
    proof_valid: bool,
) -> f64 {
    // Decay term: prevents stale reputation
    let decay_term = (1.0 - gamma) * current_score;

    // Efficiency term: VFE reduction per energy unit
    // Protects against division by zero and extreme values
    let efficiency = if cost_energy > 1e-12 {
        delta_vfe / cost_energy
    } else {
        delta_vfe
    };
    let efficiency_term = alpha * efficiency;

    // Proof verification bonus
    let proof_term = if proof_valid { beta } else { 0.0 };

    // Combine and clamp for numerical stability
    let new_score = decay_term + efficiency_term + proof_term;
    new_score.clamp(0.0, 1.0)
}

/// PAC-Bayesian generalization bound for collective trust.
///
/// ```text
/// bound = empirical_vfe + sqrt((KL + ln(1/δ)) / (2n))
/// ```
///
/// Provides probably-approximately-correct guarantees that the empirical
/// VFE reduction generalizes to unseen data with probability ≥ 1-δ.
///
/// # Arguments
/// * `empirical_vfe` - Empirical VFE reduction observed
/// * `kl_divergence` - KL divergence between prior and posterior
/// * `n` - Number of samples (contributions)
/// * `delta` - Failure probability δ ∈ (0, 1)
///
/// # Returns
/// Upper bound on generalization error
///
/// # Panics
/// Panics if `n == 0` or `delta <= 0.0` or `delta >= 1.0`
pub fn pac_bayes_bound(empirical_vfe: f64, kl_divergence: f64, n: usize, delta: f64) -> f64 {
    assert!(n > 0, "Sample count must be positive");
    assert!(delta > 0.0 && delta < 1.0, "Delta must be in (0, 1)");

    let log_term = (1.0 / delta).ln();
    let numerator = kl_divergence + log_term;
    let denominator = 2.0 * (n as f64);

    empirical_vfe + (numerator / denominator).sqrt()
}

/// Federated median aggregation with trust-weighted contributions.
///
/// Trims the top and bottom 1/3 of values (Byzantine-resilient),
/// then computes the trust-weighted median.
///
/// # Arguments
/// * `values` - List of (value, trust_score) pairs
///
/// # Returns
/// Byzantine-resilient weighted median, or 0.0 if empty
pub fn byzantine_weighted_median(values: &[(f64, f64)]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    // Sort by value for trimming
    let mut sorted: Vec<(f64, f64)> = values.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Trim top and bottom 1/3 (Byzantine resilience)
    let trim_count = sorted.len() / 3;
    let trimmed: Vec<(f64, f64)> = sorted[trim_count..sorted.len() - trim_count].to_vec();

    if trimmed.is_empty() {
        return sorted[0].0;
    }

    // Compute trust-weighted average of trimmed values
    let total_trust: f64 = trimmed.iter().map(|(_, t)| t).sum();
    if total_trust < 1e-12 {
        trimmed.iter().map(|(v, _)| v).sum::<f64>() / (trimmed.len() as f64)
    } else {
        trimmed.iter().map(|(v, t)| v * t).sum::<f64>() / total_trust
    }
}

// ============================================================================
// Sprint 123 — Verifiable PoSym + zk-Ready Stubs
// ============================================================================

/// Verifiable steering proof — zk-STARK/SNARK placeholder.
///
/// Produces a deterministic proof blob from steering parameters that can be
/// verified without re-executing the full steering computation. In production,
/// this will be replaced by Halo2 / zk-STARK circuits.
///
/// # Proof Format (v1 placeholder)
/// ```text
/// [tag(1)] [node_id_len(4)] [node_id(N)] [vfe(8)] [energy(8)] [taylor_flag(1)] [timestamp(8)] [sha256(32)]
/// ```
///
/// # Arguments
/// * `node_id` — Node identifier string
/// * `vfe_reduction` — VFE reduction achieved by steering
/// * `energy_cost` — Energy cost in Wh
/// * `taylor_containment` — Whether Taylor zonotope containment was verified
///
/// # Returns
/// Serialized proof bytes (54 bytes for 8-char node_id)
#[derive(Debug, Clone)]
pub struct SteeringProof {
    /// Raw proof bytes
    pub data: Vec<u8>,
    /// Node that generated this proof
    pub node_id: String,
    /// VFE reduction claimed
    pub vfe_reduction: f64,
    /// Energy cost claimed
    pub energy_cost: f64,
    /// Taylor zonotope containment flag
    pub taylor_containment: bool,
    /// Generation timestamp
    pub timestamp: u64,
}

impl SteeringProof {
    /// Create a new steering proof.
    pub fn new(
        node_id: String,
        vfe_reduction: f64,
        energy_cost: f64,
        taylor_containment: bool,
        timestamp: u64,
    ) -> Self {
        let data = Self::compute_proof_bytes(
            &node_id,
            vfe_reduction,
            energy_cost,
            taylor_containment,
            timestamp,
        );
        Self {
            data,
            node_id,
            vfe_reduction,
            energy_cost,
            taylor_containment,
            timestamp,
        }
    }

    /// Compute deterministic proof bytes from parameters.
    ///
    /// Format: SHA-256 of concatenated parameters + Merkle leaf structure.
    fn compute_proof_bytes(
        node_id: &str,
        vfe_reduction: f64,
        energy_cost: f64,
        taylor_containment: bool,
        timestamp: u64,
    ) -> Vec<u8> {
        let mut hasher = Sha256::new();
        // Tag byte: 0x01 = SteeringProof v1
        hasher.update([0x01]);
        // Node ID
        hasher.update(node_id.as_bytes());
        // Numeric parameters
        hasher.update(vfe_reduction.to_le_bytes());
        hasher.update(energy_cost.to_le_bytes());
        // Taylor containment flag
        hasher.update([taylor_containment as u8]);
        // Timestamp
        hasher.update(timestamp.to_le_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify the proof bytes match the claimed parameters.
    ///
    /// Returns `true` if the proof is structurally valid (parameters match hash).
    /// In production, this will also verify zk circuit satisfaction.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_proof_bytes(
            &self.node_id,
            self.vfe_reduction,
            self.energy_cost,
            self.taylor_containment,
            self.timestamp,
        );
        self.data == expected
    }

    /// Compute the Merkle leaf hash for this proof (for batch inclusion proofs).
    pub fn merkle_leaf(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update([0x02]); // Merkle leaf tag
        hasher.update(&self.data);
        hasher.finalize().into()
    }
}

/// Generate a steering proof (convenience function matching the directive signature).
///
/// # Arguments
/// * `node_id` — Node identifier string
/// * `vfe_reduction` — VFE reduction achieved by steering
/// * `energy_cost` — Energy cost in Wh
/// * `taylor_containment` — Whether Taylor zonotope containment was verified
///
/// # Returns
/// Serialized proof bytes
pub fn generate_steering_proof(
    node_id: &str,
    vfe_reduction: f64,
    energy_cost: f64,
    taylor_containment: bool,
) -> Vec<u8> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let proof = SteeringProof::new(
        node_id.to_string(),
        vfe_reduction,
        energy_cost,
        taylor_containment,
        timestamp,
    );
    proof.data
}

/// Verify a steering proof from raw bytes.
///
/// # Arguments
/// * `proof_bytes` — Raw proof bytes from `generate_steering_proof`
///
/// # Returns
/// `true` if the proof is a valid SHA-256 digest (32 bytes)
pub fn verify_steering_proof(proof_bytes: &[u8]) -> bool {
    // Placeholder: validate proof structure
    // In production: verify zk circuit + Merkle inclusion
    proof_bytes.len() == 32
}

/// Verify proof and update trust score with verifiable bonus.
///
/// Combines proof verification with the game-theoretic trust update:
/// ```text
/// s_i(t+1) = (1-γ) * s_i(t) + α * (ΔVFE_i / cost_energy_i) + β * verify_proof(π_i)
/// ```
///
/// # Arguments
/// * `current_score` — Current trust score
/// * `proof_bytes` — Raw proof bytes to verify
/// * `delta_vfe` — VFE reduction achieved
/// * `energy` — Energy cost in Wh
///
/// # Returns
/// Updated trust score clamped to [0.0, 1.0]
pub fn verify_and_update_trust(
    current_score: f64,
    proof_bytes: &[u8],
    delta_vfe: f64,
    energy: f64,
) -> f64 {
    let valid = verify_steering_proof(proof_bytes);
    update_trust_score(current_score, 0.05, 0.4, delta_vfe, energy, 0.3, valid)
}

/// Merkle batch proof for multiple steering proofs.
///
/// Enables efficient batch verification of multiple proofs with a single
/// cryptographic commitment.
#[derive(Debug, Clone)]
pub struct MerkleBatchProof {
    /// Root hash of the Merkle tree
    pub root: Hash,
    /// Individual leaf hashes
    pub leaves: Vec<Hash>,
    /// Number of proofs in batch
    pub count: usize,
}

impl MerkleBatchProof {
    /// Create a Merkle batch proof from a list of steering proofs.
    pub fn from_proofs(proofs: &[SteeringProof]) -> Self {
        let leaves: Vec<Hash> = proofs.iter().map(|p| p.merkle_leaf()).collect();
        let root = if leaves.is_empty() {
            [0u8; 32]
        } else {
            Self::compute_root(&leaves)
        };
        Self {
            root,
            leaves,
            count: proofs.len(),
        }
    }

    /// Compute Merkle root from leaf hashes.
    fn compute_root(leaves: &[Hash]) -> Hash {
        if leaves.len() == 1 {
            return leaves[0];
        }
        // Build tree bottom-up
        let mut current: Vec<Hash> = leaves.to_vec();
        while current.len() > 1 {
            let mut next = Vec::new();
            for chunk in current.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() > 1 { chunk[1] } else { left };
                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                next.push(hasher.finalize().into());
            }
            current = next;
        }
        current.into_iter().next().unwrap_or([0u8; 32])
    }

    /// Verify that a leaf belongs to this Merkle batch.
    pub fn contains_leaf(&self, leaf: Hash) -> bool {
        self.leaves.contains(&leaf)
    }
}

/// zk-proof type enumeration for future circuit integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZkProofType {
    /// zk-STARK placeholder — transparent setup, post-quantum secure
    ZkStark,
    /// zk-SNARK placeholder — compact proofs, trusted setup required
    ZkSnark,
    /// Halo2 placeholder — production-ready Rust zk-SNARK
    Halo2,
    /// No proof (fallback for testing)
    None,
}

impl Default for ZkProofType {
    fn default() -> Self {
        ZkProofType::None
    }
}

/// Configuration for zk-proof generation parameters.
#[derive(Debug, Clone)]
pub struct ZkProofConfig {
    /// Type of zk-proof system to use
    pub proof_type: ZkProofType,
    /// Security level in bits (128 = post-quantum, 64 = classical)
    pub security_bits: u32,
    /// Maximum proof size in bytes
    pub max_proof_size: usize,
    /// Proof generation timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for ZkProofConfig {
    fn default() -> Self {
        Self {
            proof_type: ZkProofType::None,
            security_bits: 128,
            max_proof_size: 4096,
            timeout_ms: 5000,
        }
    }
}

impl ZkProofConfig {
    /// Create config for zk-STARK proofs.
    pub fn stark(security_bits: u32) -> Self {
        Self {
            proof_type: ZkProofType::ZkStark,
            security_bits,
            max_proof_size: 8192,
            timeout_ms: 10000,
        }
    }

    /// Create config for Halo2 proofs.
    pub fn halo2(max_proof_size: usize) -> Self {
        Self {
            proof_type: ZkProofType::Halo2,
            security_bits: 128,
            max_proof_size,
            timeout_ms: 15000,
        }
    }
}

/// Succinct proof result from `generate_succinct_proof`.
#[derive(Debug, Clone)]
pub struct SuccinctProof {
    /// Proof bytes (stub — hash-based commitment)
    pub proof_bytes: Vec<u8>,
    /// Public inputs commitment (SHA-256)
    pub public_input_hash: [u8; 32],
    /// Proof system type used
    pub proof_type: ZkProofType,
    /// Proof size in bytes
    pub proof_size: usize,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
}

impl SuccinctProof {
    /// Verify the succinct proof structure.
    pub fn verify(&self) -> bool {
        !self.proof_bytes.is_empty()
            && self.proof_size == self.proof_bytes.len()
            && self.proof_size <= 4096
    }

    /// Estimate verification cost (lightweight hash check).
    pub fn verification_cost_estimate(&self) -> u64 {
        // Stub: proportional to proof size (ceiling division, min 1 chunk)
        (self.proof_size as u64 + 63) / 64
    }
}

/// Generate a succinct proof (SNARK/Groth16/Halo2 stub).
///
/// Produces a compact zero-knowledge proof commitment for verified steering.
/// Currently implements a hash-based stub that can be replaced with real
/// zk-proof backends (snarkvm, halo2, arkworks) when available.
///
/// # Arguments
/// * `proof` — Steering proof to generate succinct proof for
/// * `config` — zk-proof configuration
///
/// # Returns
/// `SuccinctProof` with proof bytes and metadata
pub fn generate_succinct_proof(
    proof: &SteeringProof,
    config: &ZkProofConfig,
) -> SuccinctProof {
    let start = std::time::Instant::now();

    // Build public input from steering proof data
    let public_input_hash: [u8; 32] = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(proof.node_id.as_bytes());
        hasher.update(proof.timestamp.to_le_bytes());
        hasher.update(proof.vfe_reduction.to_bits().to_be_bytes());
        hasher.update(proof.energy_cost.to_bits().to_be_bytes());
        hasher.update(&(proof.taylor_containment as u8).to_le_bytes());
        hasher.update(&proof.data);
        hasher.finalize().into()
    };

    // Generate proof bytes based on proof type (stub implementation)
    let proof_bytes = match config.proof_type {
        ZkProofType::ZkStark => {
            // zk-STARK stub: hash-based commitment with transparency
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(b"STARK");
            data.extend_from_slice(&public_input_hash);
            data.extend_from_slice(&config.security_bits.to_le_bytes());
            let mut hasher = sha2::Sha256::new();
            hasher.update(&data);
            hasher.finalize().to_vec()
        }
        ZkProofType::ZkSnark => {
            // zk-SNARK stub: compact proof with trusted setup commitment
            let mut data = Vec::with_capacity(128);
            data.extend_from_slice(b"SNARK");
            data.extend_from_slice(&public_input_hash);
            // Simulate Groth16-style compact proof
            let mut hasher = sha2::Sha256::new();
            hasher.update(&data);
            let h1 = hasher.finalize();
            let mut hasher2 = sha2::Sha256::new();
            hasher2.update(h1);
            hasher2.update(b"groth16_commitment");
            hasher2.finalize().to_vec()
        }
        ZkProofType::Halo2 => {
            // Halo2 stub: production-ready placeholder
            let mut data = Vec::with_capacity(512);
            data.extend_from_slice(b"HALO2");
            data.extend_from_slice(&public_input_hash);
            data.extend_from_slice(&(config.max_proof_size as u64).to_le_bytes());
            let mut hasher = sha2::Sha256::new();
            hasher.update(&data);
            hasher.finalize().to_vec()
        }
        ZkProofType::None => {
            // No proof — just hash commitment
            let mut hasher = sha2::Sha256::new();
            hasher.update(public_input_hash);
            hasher.finalize().to_vec()
        }
    };

    // Enforce max proof size
    let proof_bytes = proof_bytes.into_iter()
        .take(config.max_proof_size)
        .collect::<Vec<u8>>();

    let elapsed = start.elapsed();
    let generation_time_ms = elapsed.as_millis() as u64;

    let proof_size = proof_bytes.len();
    SuccinctProof {
        proof_bytes,
        public_input_hash,
        proof_type: config.proof_type.clone(),
        proof_size,
        generation_time_ms,
    }
}

/// Verify a succinct proof against a steering proof.
///
/// # Arguments
/// * `succinct` — Succinct proof to verify
/// * `steering` — Original steering proof
///
/// # Returns
/// `true` if the proof is structurally valid
pub fn verify_succinct_proof(succinct: &SuccinctProof, steering: &SteeringProof) -> bool {
    if !succinct.verify() {
        return false;
    }

    // Verify public input hash matches
    let expected_hash: [u8; 32] = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(steering.node_id.as_bytes());
        hasher.update(steering.timestamp.to_le_bytes());
        hasher.update(steering.vfe_reduction.to_bits().to_be_bytes());
        hasher.update(steering.energy_cost.to_bits().to_be_bytes());
        hasher.update(&(steering.taylor_containment as u8).to_le_bytes());
        hasher.update(&steering.data);
        hasher.finalize().into()
    };

    succinct.public_input_hash == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posym_initial_state() {
        let posym = ProofOfSymbiosis::new(42);
        assert_eq!(posym.node_id, 42);
        assert_eq!(posym.steer_count(), 0);
        assert_eq!(posym.total_vfe_reduction(), 0.0);
        assert!(posym.verify_chain());
    }

    #[test]
    fn test_posym_record_steer() {
        let mut posym = ProofOfSymbiosis::new(1);
        posym.record_certified_steer(100, 1.0, 0.5);
        assert_eq!(posym.steer_count(), 1);
        assert!((posym.total_vfe_reduction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_posym_trust_score_increases() {
        let mut posym = ProofOfSymbiosis::new(1);
        let initial_score = posym.compute_trust_score();
        posym.record_certified_steer(100, 1.0, 0.3);
        posym.add_uptime(3600);
        let new_score = posym.compute_trust_score();
        assert!(new_score > initial_score);
    }

    #[test]
    fn test_posym_contribution_hash_verify() {
        let contrib = SteerContribution::new(50, 2.0, 1.0);
        assert!(contrib.verify());
        assert!((contrib.vfe_reduction() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_posym_chain_integrity() {
        let mut posym = ProofOfSymbiosis::new(7);
        for i in 0..10 {
            posym.record_certified_steer(i * 10, 1.0, 0.5);
        }
        assert!(posym.verify_chain());
        assert_eq!(posym.steer_count(), 10);
    }

    #[test]
    fn test_posym_generate_hash_deterministic() {
        let mut posym1 = ProofOfSymbiosis::new(99);
        let mut posym2 = ProofOfSymbiosis::new(99);
        posym1.record_certified_steer(10, 1.0, 0.5);
        posym2.record_certified_steer(10, 1.0, 0.5);
        posym1.add_uptime(100);
        posym2.add_uptime(100);
        assert_eq!(posym1.generate_hash(), posym2.generate_hash());
    }

    #[test]
    fn test_posym_max_contributions() {
        let mut posym = ProofOfSymbiosis::new(1);
        posym.max_contributions = 5;
        for i in 0..10 {
            posym.record_certified_steer(i, 1.0, 0.5);
        }
        assert_eq!(posym.steer_count(), 5);
        // Oldest contributions should be evicted
        assert_eq!(posym.contributions.front().unwrap().timestamp, 5);
    }

    #[test]
    fn test_posym_is_trusted() {
        let mut posym = ProofOfSymbiosis::new(1);
        assert!(!posym.is_trusted(10.0));
        for i in 0..100 {
            posym.record_certified_steer(i, 1.0, 0.1);
        }
        posym.add_uptime(86400);
        assert!(posym.is_trusted(1.0));
    }

    #[test]
    fn test_posym_custom_weights() {
        let posym = ProofOfSymbiosis::with_weights(1, 0.5, 0.3, 0.2);
        assert!((posym.weight_steers - 0.5).abs() < 1e-10);
        assert!((posym.weight_vfe - 0.3).abs() < 1e-10);
        assert!((posym.weight_uptime - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_posym_latest_contribution() {
        let mut posym = ProofOfSymbiosis::new(1);
        assert!(posym.latest_contribution().is_none());
        posym.record_certified_steer(100, 1.0, 0.5);
        assert!(posym.latest_contribution().is_some());
        assert_eq!(posym.latest_contribution().unwrap().timestamp, 100);
    }

    #[test]
    fn test_posym_default() {
        let posym = ProofOfSymbiosis::default();
        assert_eq!(posym.node_id, 0);
    }

    #[test]
    fn test_posym_vfe_reduction_clamped() {
        let contrib = SteerContribution::new(1, 0.5, 1.0);
        // vfe_after > vfe_before, so reduction should be 0 (clamped)
        assert_eq!(contrib.vfe_reduction(), 0.0);
    }

    // ─── Sprint 122 — Game-Theoretic PoSym Tests ──────────────

    // calculate_energy_impact tests

    #[test]
    fn test_energy_impact_basic() {
        // P_idle=10W, Δt=3600s (1h), FLOPs=1e12, energy_per_flop=1e-16
        let idle_power_w = 10.0;
        let delta_t_sec = 3600.0;
        let total_flops = 1e12;
        let energy_per_flop = 1e-16;
        let result =
            calculate_energy_impact(idle_power_w, delta_t_sec, total_flops, energy_per_flop);
        // idle_energy = 10 * 1 = 10 Wh
        // compute_energy = 1e12 * 1e-16 = 0.0001 Wh
        // hw_factor = 1.0 + 0.15 * ln(1e12/1e12) = 1.0 + 0 = 1.0
        // total = 10 + 0.0001 * 1.0 = 10.0001
        assert!((result - 10.0001).abs() < 1e-6);
    }

    #[test]
    fn test_energy_impact_zero_flops() {
        let result = calculate_energy_impact(5.0, 7200.0, 0.0, 1e-16);
        // idle_energy = 5 * 2 = 10 Wh, compute = 0, hw_factor = 1.0
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_energy_impact_hw_factor_scales() {
        // Higher FLOPs → larger hw_factor
        let e_low = calculate_energy_impact(0.0, 1.0, 1e12, 1e-16);
        let e_high = calculate_energy_impact(0.0, 1.0, 1e15, 1e-16);
        // hw_factor for 1e15: 1.0 + 0.15 * ln(1e15/1e12) = 1.0 + 0.15 * ln(1000) ≈ 1.0 + 1.017 = 2.017
        // e_low = 1e12 * 1e-16 * 1.0 = 1e-4
        // e_high = 1e15 * 1e-16 * 2.017 ≈ 2.017e-2
        assert!(e_high > e_low);
        // Ratio should reflect hw_factor scaling
        let ratio = e_high / e_low;
        assert!(ratio > 2.0);
    }

    #[test]
    fn test_energy_impact_large_flops() {
        let result = calculate_energy_impact(100.0, 3600.0, 1e15, 1e-16);
        // idle = 100 * 1 = 100 Wh
        // compute = 1e15 * 1e-16 = 0.1 Wh
        // hw_factor = 1.0 + 0.15 * ln(1e15/1e12) = 1.0 + 0.15 * ln(1000) ≈ 2.017
        // total ≈ 100 + 0.1 * 2.017 = 100.2017
        assert!((result - 100.2017).abs() < 0.01);
    }

    // update_trust_score tests

    #[test]
    fn test_trust_score_decay_only() {
        // gamma=0.1, alpha=0, beta=0 → pure decay
        let new_score = update_trust_score(0.8, 0.1, 0.0, 0.0, 1.0, 0.0, false);
        // (1-0.1)*0.8 + 0 + 0 = 0.72
        assert!((new_score - 0.72).abs() < 1e-10);
    }

    #[test]
    fn test_trust_score_efficiency_bonus() {
        // gamma=0.0, alpha=0.5, delta_vfe=1.0, cost_energy=1.0
        let new_score = update_trust_score(0.0, 0.0, 0.5, 1.0, 1.0, 0.0, false);
        // 0 + 0.5*(1.0/1.0) + 0 = 0.5
        assert!((new_score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_trust_score_proof_bonus() {
        // gamma=0.0, alpha=0, beta=0.3, proof_valid=true
        let new_score = update_trust_score(0.0, 0.0, 0.0, 0.0, 1.0, 0.3, true);
        // 0 + 0 + 0.3 = 0.3
        assert!((new_score - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_trust_score_proof_invalid() {
        let new_score = update_trust_score(0.0, 0.0, 0.0, 0.0, 1.0, 0.3, false);
        assert_eq!(new_score, 0.0);
    }

    #[test]
    fn test_trust_score_clamped_to_one() {
        // Large efficiency + proof should clamp to 1.0
        let new_score = update_trust_score(0.9, 0.0, 1.0, 100.0, 1.0, 0.5, true);
        // 0 + 1.0*100 + 0.5 = 100.5 → clamped to 1.0
        assert_eq!(new_score, 1.0);
    }

    #[test]
    fn test_trust_score_clamped_to_zero() {
        // All zeros → 0.0
        let new_score = update_trust_score(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, false);
        assert_eq!(new_score, 0.0);
    }

    #[test]
    fn test_trust_score_full_formula() {
        // gamma=0.1, alpha=0.5, beta=0.2
        // current=0.6, delta_vfe=2.0, cost=1.0, proof_valid=true
        let new_score = update_trust_score(0.6, 0.1, 0.5, 2.0, 1.0, 0.2, true);
        // decay = 0.9 * 0.6 = 0.54
        // efficiency = 0.5 * (2.0/1.0) = 1.0
        // proof = 0.2
        // total = 0.54 + 1.0 + 0.2 = 1.74 → clamped to 1.0
        assert_eq!(new_score, 1.0);
    }

    #[test]
    fn test_trust_score_small_update() {
        // gamma=0.05, alpha=0.1, beta=0.05
        let new_score = update_trust_score(0.5, 0.05, 0.1, 0.1, 1.0, 0.05, false);
        // decay = 0.95 * 0.5 = 0.475
        // efficiency = 0.1 * 0.1 = 0.01
        // proof = 0
        // total = 0.485
        assert!((new_score - 0.485).abs() < 1e-10);
    }

    #[test]
    fn test_trust_score_zero_energy_fallback() {
        // cost_energy < 1e-12 → fallback to delta_vfe directly
        let new_score = update_trust_score(0.0, 0.0, 0.5, 3.0, 1e-15, 0.0, false);
        // efficiency = 0.5 * 3.0 = 1.5 → clamped to 1.0
        assert_eq!(new_score, 1.0);
    }

    // pac_bayes_bound tests

    #[test]
    fn test_pac_bayes_basic() {
        // empirical=0.5, KL=0.1, n=100, delta=0.05
        let bound = pac_bayes_bound(0.5, 0.1, 100, 0.05);
        // ln(1/0.05) = ln(20) ≈ 2.9957
        // numerator = 0.1 + 2.9957 = 3.0957
        // denominator = 200
        // sqrt(3.0957/200) = sqrt(0.015478) ≈ 0.1244
        // bound = 0.5 + 0.1244 ≈ 0.6244
        assert!(bound > 0.5);
        assert!((bound - 0.6244).abs() < 0.01);
    }

    #[test]
    fn test_pac_bayes_bound_exceeds_empirical() {
        let bound = pac_bayes_bound(0.3, 0.05, 50, 0.1);
        assert!(bound > 0.3);
    }

    #[test]
    fn test_pac_bayes_tightens_with_samples() {
        let bound_10 = pac_bayes_bound(0.5, 0.1, 10, 0.05);
        let bound_1000 = pac_bayes_bound(0.5, 0.1, 1000, 0.05);
        assert!(bound_1000 < bound_10);
    }

    #[test]
    fn test_pac_bayes_tightens_with_lowern_kl() {
        let bound_high_kl = pac_bayes_bound(0.5, 1.0, 100, 0.05);
        let bound_low_kl = pac_bayes_bound(0.5, 0.01, 100, 0.05);
        assert!(bound_low_kl < bound_high_kl);
    }

    #[test]
    #[should_panic(expected = "Sample count must be positive")]
    fn test_pac_bayes_zero_samples_panics() {
        pac_bayes_bound(0.5, 0.1, 0, 0.05);
    }

    #[test]
    #[should_panic(expected = "Delta must be in (0, 1)")]
    fn test_pac_bayes_delta_zero_panics() {
        pac_bayes_bound(0.5, 0.1, 100, 0.0);
    }

    #[test]
    #[should_panic(expected = "Delta must be in (0, 1)")]
    fn test_pac_bayes_delta_one_panics() {
        pac_bayes_bound(0.5, 0.1, 100, 1.0);
    }

    #[test]
    fn test_pac_bayes_large_n_approaches_empirical() {
        let bound = pac_bayes_bound(0.5, 0.1, 1_000_000, 0.05);
        // With huge n, bound should be very close to empirical
        assert!((bound - 0.5).abs() < 0.01);
    }

    // byzantine_weighted_median tests

    #[test]
    fn test_byzantine_median_basic() {
        let values = vec![
            (10.0, 0.9),
            (11.0, 0.8),
            (10.5, 0.85),
            (12.0, 0.7),
            (10.0, 0.95),
        ];
        let result = byzantine_weighted_median(&values);
        // All honest — should be near 10.5
        assert!((result - 10.5).abs() < 1.0);
    }

    #[test]
    fn test_byzantine_median_trims_outliers() {
        let values = vec![
            (10.0, 0.9),
            (10.0, 0.9),
            (10.0, 0.9),
            (100.0, 0.1), // Byzantine outlier
            (-50.0, 0.1), // Byzantine outlier
        ];
        let result = byzantine_weighted_median(&values);
        // After trimming 1/3 top/bottom, only 10.0 values remain
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_byzantine_median_empty() {
        let values: [(f64, f64); 0] = [];
        let result = byzantine_weighted_median(&values);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_byzantine_median_single() {
        let values = vec![(42.0, 0.5)];
        let result = byzantine_weighted_median(&values);
        assert!((result - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_byzantine_median_two_values() {
        let values = vec![(10.0, 0.8), (20.0, 0.6)];
        let result = byzantine_weighted_median(&values);
        // Trim 1/3 of 2 = 0, so no trim. Weighted average.
        let expected = (10.0 * 0.8 + 20.0 * 0.6) / (0.8 + 0.6);
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_byzantine_median_all_identical() {
        let values = vec![(7.0, 0.5), (7.0, 0.5), (7.0, 0.5)];
        let result = byzantine_weighted_median(&values);
        assert!((result - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_byzantine_median_resists_one_third_byzantine() {
        // 9 honest + 3 byzantine = 12 total
        let mut values: Vec<(f64, f64)> = (0..9).map(|_| (50.0, 0.9)).collect();
        values.push((500.0, 0.1));
        values.push((-500.0, 0.1));
        values.push((999.0, 0.1));
        let result = byzantine_weighted_median(&values);
        // Trim 1/3 of 12 = 4 from each end → byzantine values removed
        assert!((result - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_byzantine_median_trust_weighting() {
        let values = vec![
            (10.0, 0.99), // High trust
            (20.0, 0.01), // Low trust
        ];
        let result = byzantine_weighted_median(&values);
        // Should be much closer to 10.0
        assert!(result < 15.0);
    }

    // ====================================================================
    // Sprint 123 — Verifiable PoSym Tests
    // ====================================================================

    #[test]
    fn test_steering_proof_creation() {
        let proof = SteeringProof::new("node-42".to_string(), 0.35, 12.5, true, 1000);
        assert_eq!(proof.node_id, "node-42");
        assert_eq!(proof.vfe_reduction, 0.35);
        assert_eq!(proof.energy_cost, 12.5);
        assert!(proof.taylor_containment);
        assert_eq!(proof.timestamp, 1000);
        assert_eq!(proof.data.len(), 32);
    }

    #[test]
    fn test_steering_proof_verify_valid() {
        let proof = SteeringProof::new("node-1".to_string(), 0.5, 10.0, false, 2000);
        assert!(proof.verify());
    }

    #[test]
    fn test_steering_proof_verify_tampered() {
        let mut proof = SteeringProof::new("node-2".to_string(), 0.5, 10.0, true, 3000);
        proof.vfe_reduction = 999.0; // Tamper with claimed value
        assert!(!proof.verify());
    }

    #[test]
    fn test_steering_proof_deterministic() {
        let p1 = SteeringProof::new("a".to_string(), 1.0, 2.0, true, 5);
        let p2 = SteeringProof::new("a".to_string(), 1.0, 2.0, true, 5);
        assert_eq!(p1.data, p2.data);
    }

    #[test]
    fn test_steering_proof_merkle_leaf() {
        let proof = SteeringProof::new("m".to_string(), 0.1, 0.5, false, 100);
        let leaf = proof.merkle_leaf();
        assert_eq!(leaf.len(), 32);
    }

    #[test]
    fn test_generate_steering_proof() {
        let data = generate_steering_proof("gen-node", 0.7, 20.0, true);
        assert_eq!(data.len(), 32);
    }

    #[test]
    fn test_verify_steering_proof_valid() {
        let data = generate_steering_proof("v-node", 0.2, 5.0, false);
        assert!(verify_steering_proof(&data));
    }

    #[test]
    fn test_verify_steering_proof_empty() {
        assert!(!verify_steering_proof(&[]));
    }

    #[test]
    fn test_verify_steering_proof_wrong_size() {
        assert!(!verify_steering_proof(&[1, 2, 3]));
    }

    #[test]
    fn test_verify_and_update_trust_valid_proof() {
        let data = generate_steering_proof("trust-node", 0.8, 15.0, true);
        let new_score = verify_and_update_trust(0.5, &data, 0.8, 15.0);
        // Proof valid → beta bonus applied
        assert!(new_score > 0.5);
        assert!(new_score <= 1.0);
    }

    #[test]
    fn test_verify_and_update_trust_invalid_proof() {
        let bad_data: Vec<u8> = vec![0; 10]; // Invalid size
        let new_score = verify_and_update_trust(0.5, &bad_data, 0.8, 15.0);
        // Proof invalid → no beta bonus
        assert!(new_score < 0.5 + 0.4 * 0.8 / 15.0 + 0.01);
    }

    #[test]
    fn test_verify_and_update_trust_clamped() {
        let data = generate_steering_proof("clamp", 100.0, 0.001, true);
        let new_score = verify_and_update_trust(1.0, &data, 100.0, 0.001);
        assert_eq!(new_score, 1.0);
    }

    #[test]
    fn test_merkle_batch_single_proof() {
        let proof = SteeringProof::new("single".to_string(), 0.5, 10.0, true, 1);
        let batch = MerkleBatchProof::from_proofs(&[proof]);
        assert_eq!(batch.count, 1);
        assert_eq!(batch.leaves.len(), 1);
        assert_eq!(batch.root, batch.leaves[0]);
    }

    #[test]
    fn test_merkle_batch_multiple_proofs() {
        let proofs = vec![
            SteeringProof::new("a".to_string(), 0.1, 1.0, true, 1),
            SteeringProof::new("b".to_string(), 0.2, 2.0, false, 2),
            SteeringProof::new("c".to_string(), 0.3, 3.0, true, 3),
        ];
        let batch = MerkleBatchProof::from_proofs(&proofs);
        assert_eq!(batch.count, 3);
        assert_eq!(batch.leaves.len(), 3);
        assert!(batch.contains_leaf(proofs[0].merkle_leaf()));
        assert!(batch.contains_leaf(proofs[1].merkle_leaf()));
    }

    #[test]
    fn test_merkle_batch_empty() {
        let batch = MerkleBatchProof::from_proofs(&[]);
        assert_eq!(batch.count, 0);
        assert_eq!(batch.root, [0u8; 32]);
    }

    #[test]
    fn test_merkle_batch_contains_leaf() {
        let proofs = vec![
            SteeringProof::new("x".to_string(), 0.5, 5.0, true, 10),
            SteeringProof::new("y".to_string(), 0.6, 6.0, false, 20),
        ];
        let batch = MerkleBatchProof::from_proofs(&proofs);
        let fake_leaf = SteeringProof::new("z".to_string(), 0.9, 9.0, true, 99).merkle_leaf();
        assert!(!batch.contains_leaf(fake_leaf));
    }

    #[test]
    fn test_merkle_batch_root_deterministic() {
        let proofs = vec![
            SteeringProof::new("d1".to_string(), 0.1, 1.0, true, 1),
            SteeringProof::new("d2".to_string(), 0.2, 2.0, false, 2),
        ];
        let b1 = MerkleBatchProof::from_proofs(&proofs);
        let b2 = MerkleBatchProof::from_proofs(&proofs);
        assert_eq!(b1.root, b2.root);
    }

    #[test]
    fn test_zk_proof_type_default() {
        let pt = ZkProofType::default();
        assert_eq!(pt, ZkProofType::None);
    }

    #[test]
    fn test_zk_proof_type_variants() {
        assert!(matches!(ZkProofType::ZkStark, ZkProofType::ZkStark));
        assert!(matches!(ZkProofType::ZkSnark, ZkProofType::ZkSnark));
        assert!(matches!(ZkProofType::Halo2, ZkProofType::Halo2));
        assert!(matches!(ZkProofType::None, ZkProofType::None));
    }

    #[test]
    fn test_zk_proof_config_default() {
        let cfg = ZkProofConfig::default();
        assert_eq!(cfg.proof_type, ZkProofType::None);
        assert_eq!(cfg.security_bits, 128);
        assert_eq!(cfg.max_proof_size, 4096);
        assert_eq!(cfg.timeout_ms, 5000);
    }

    #[test]
    fn test_zk_proof_config_stark() {
        let cfg = ZkProofConfig::stark(256);
        assert_eq!(cfg.proof_type, ZkProofType::ZkStark);
        assert_eq!(cfg.security_bits, 256);
    }

    #[test]
    fn test_zk_proof_config_halo2() {
        let cfg = ZkProofConfig::halo2(2048);
        assert_eq!(cfg.proof_type, ZkProofType::Halo2);
        assert_eq!(cfg.max_proof_size, 2048);
    }

    // PASO D: Succinct Proof tests

    #[test]
    fn test_generate_succinct_proof_none() {
        let proof = SteeringProof::new("test".to_string(), 0.5, 100.0, true, 1);
        let cfg = ZkProofConfig::default();
        let succinct = generate_succinct_proof(&proof, &cfg);
        assert!(!succinct.proof_bytes.is_empty());
        assert_eq!(succinct.proof_type, ZkProofType::None);
        assert!(succinct.proof_size <= cfg.max_proof_size);
        assert!(succinct.verify());
    }

    #[test]
    fn test_generate_succinct_proof_stark() {
        let proof = SteeringProof::new("stark".to_string(), 0.8, 200.0, true, 2);
        let cfg = ZkProofConfig::stark(128);
        let succinct = generate_succinct_proof(&proof, &cfg);
        assert_eq!(succinct.proof_type, ZkProofType::ZkStark);
        assert!(succinct.verify());
    }

    #[test]
    fn test_generate_succinct_proof_snark() {
        let proof = SteeringProof::new("snark".to_string(), 0.3, 50.0, false, 3);
        let cfg = ZkProofConfig::default();
        let mut cfg_snark = cfg.clone();
        cfg_snark.proof_type = ZkProofType::ZkSnark;
        let succinct = generate_succinct_proof(&proof, &cfg_snark);
        assert_eq!(succinct.proof_type, ZkProofType::ZkSnark);
        assert!(succinct.verify());
    }

    #[test]
    fn test_generate_succinct_proof_halo2() {
        let proof = SteeringProof::new("halo2".to_string(), 0.9, 300.0, true, 4);
        let cfg = ZkProofConfig::halo2(4096);
        let succinct = generate_succinct_proof(&proof, &cfg);
        assert_eq!(succinct.proof_type, ZkProofType::Halo2);
        assert!(succinct.verify());
        assert!(succinct.proof_size <= 4096);
    }

    #[test]
    fn test_verify_succinct_proof_valid() {
        let proof = SteeringProof::new("verify".to_string(), 0.7, 150.0, true, 5);
        let cfg = ZkProofConfig::default();
        let succinct = generate_succinct_proof(&proof, &cfg);
        assert!(verify_succinct_proof(&succinct, &proof));
    }

    #[test]
    fn test_verify_succinct_proof_wrong_steering() {
        let proof1 = SteeringProof::new("a".to_string(), 0.5, 100.0, true, 1);
        let proof2 = SteeringProof::new("b".to_string(), 0.6, 200.0, false, 2);
        let cfg = ZkProofConfig::default();
        let succinct = generate_succinct_proof(&proof1, &cfg);
        // Should fail — public input hash doesn't match proof2
        assert!(!verify_succinct_proof(&succinct, &proof2));
    }

    #[test]
    fn test_succinct_proof_verify_empty() {
        let empty = SuccinctProof {
            proof_bytes: vec![],
            public_input_hash: [0u8; 32],
            proof_type: ZkProofType::None,
            proof_size: 0,
            generation_time_ms: 0,
        };
        assert!(!empty.verify());
    }

    #[test]
    fn test_succinct_proof_verification_cost() {
        let proof = SteeringProof::new("cost".to_string(), 0.5, 100.0, true, 1);
        let cfg = ZkProofConfig::default();
        let succinct = generate_succinct_proof(&proof, &cfg);
        let cost = succinct.verification_cost_estimate();
        // Cost should be proportional to proof size (may be 0 for small proofs)
        assert_eq!(cost, (succinct.proof_size as u64 + 63) / 64);
    }

    #[test]
    fn test_succinct_proof_max_size_enforced() {
        let proof = SteeringProof::new("size".to_string(), 0.5, 100.0, true, 1);
        let cfg = ZkProofConfig {
            max_proof_size: 16,
            ..ZkProofConfig::default()
        };
        let succinct = generate_succinct_proof(&proof, &cfg);
        assert!(succinct.proof_size <= 16);
    }

    #[test]
    fn test_succinct_proof_deterministic() {
        let proof = SteeringProof::new("det".to_string(), 0.5, 100.0, true, 1);
        let cfg = ZkProofConfig::default();
        let s1 = generate_succinct_proof(&proof, &cfg);
        let s2 = generate_succinct_proof(&proof, &cfg);
        assert_eq!(s1.public_input_hash, s2.public_input_hash);
        assert_eq!(s1.proof_bytes, s2.proof_bytes);
    }
}
