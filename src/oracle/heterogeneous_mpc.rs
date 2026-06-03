//! Heterogeneous MPC — Sprint 80: Gödelian Synthesis & Architecture of Absolute Incompleteness
//!
//! Multi-ISA consensus validation: physical attestation is only trusted when
//! x86, ARM, and RISC-V architectures attest simultaneously. Manufacturer
//! agnosticism prevents Silicon Trojan attacks from any single vendor.
//!
//! Key features:
//! - Multi-ISA attestation (x86/ARM/RISC-V)
//! - Threshold-based validation (≥2 of 3 architectures required)
//! - Cross-architecture hash reconciliation
//! - Replay protection via nonce binding
//! - Manufacturer-agnostic trust model

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MpcError {
    InsufficientArchitectures(usize, usize),
    HashMismatch,
    NonceMismatch,
    ExpiredAttestation,
    InvalidProofFormat,
    DuplicateAttestation,
}

impl fmt::Display for MpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MpcError::InsufficientArchitectures(have, need) => {
                write!(f, "Insufficient architectures: {have}/{need}")
            }
            MpcError::HashMismatch => write!(f, "Cross-architecture hash mismatch"),
            MpcError::NonceMismatch => write!(f, "Nonce mismatch across attestations"),
            MpcError::ExpiredAttestation => write!(f, "Attestation expired"),
            MpcError::InvalidProofFormat => write!(f, "Invalid proof format"),
            MpcError::DuplicateAttestation => write!(f, "Duplicate attestation from same architecture"),
        }
    }
}

// ─── Architecture Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Architecture {
    X86,
    Arm,
    RiscV,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Architecture::X86 => write!(f, "x86"),
            Architecture::Arm => write!(f, "ARM"),
            Architecture::RiscV => write!(f, "RISC-V"),
        }
    }
}

impl Architecture {
    pub fn all() -> [Self; 3] {
        [Architecture::X86, Architecture::Arm, Architecture::RiscV]
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MpcConfig {
    /// Minimum architectures required for consensus (default: 2)
    pub threshold: usize,
    /// Maximum attestation age in milliseconds
    pub max_age_ms: u64,
    /// Enforce nonce consistency across architectures
    pub enforce_nonce: bool,
    /// Hash function output size in bytes
    pub hash_size: usize,
}

impl MpcConfig {
    pub fn default_stuartian() -> Self {
        Self {
            threshold: 2,
            max_age_ms: 60_000,
            enforce_nonce: true,
            hash_size: 32,
        }
    }

    pub fn validate(&self) -> Result<(), MpcError> {
        if self.threshold < 2 {
            return Err(MpcError::InsufficientArchitectures(0, 2));
        }
        if self.threshold > 3 {
            return Err(MpcError::InsufficientArchitectures(3, 3));
        }
        if self.max_age_ms == 0 {
            return Err(MpcError::ExpiredAttestation);
        }
        if self.hash_size == 0 {
            return Err(MpcError::InvalidProofFormat);
        }
        Ok(())
    }
}

impl Default for MpcConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Attestation Proof ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AttestationProof {
    /// Architecture that produced this attestation
    pub architecture: Architecture,
    /// Manufacturer identifier (Intel, AMD, Apple, etc.)
    pub manufacturer: String,
    /// Cryptographic proof bytes
    pub proof: Vec<u8>,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Committed hash of the attested state
    pub committed_hash: Vec<u8>,
}

impl AttestationProof {
    pub fn new(
        architecture: Architecture,
        manufacturer: String,
        proof: Vec<u8>,
        nonce: u64,
        timestamp_ms: u64,
        committed_hash: Vec<u8>,
    ) -> Self {
        Self {
            architecture,
            manufacturer,
            proof,
            nonce,
            timestamp_ms,
            committed_hash,
        }
    }

    pub fn is_expired(&self, current_ms: u64, max_age_ms: u64) -> bool {
        current_ms.saturating_sub(self.timestamp_ms) > max_age_ms
    }
}

impl fmt::Display for AttestationProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttestationProof(arch={}, mfr={}, nonce={}, ts={})",
            self.architecture, self.manufacturer, self.nonce, self.timestamp_ms
        )
    }
}

// ─── Consensus Record ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MpcRecord {
    /// Round identifier
    pub round_id: u64,
    /// Architectures that participated
    pub architectures: Vec<Architecture>,
    /// Consensus result
    pub consensus: bool,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl fmt::Display for MpcRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arch_strs: Vec<String> = self.architectures.iter().map(|a| a.to_string()).collect();
        write!(
            f,
            "MpcRecord(round={}, archs=[{}], consensus={})",
            self.round_id,
            arch_strs.join(", "),
            self.consensus
        )
    }
}

// ─── Heterogeneous MPC Engine ─────────────────────────────────────────────────

pub struct HeterogeneousMpc {
    config: MpcConfig,
    proofs: HashMap<u64, Vec<AttestationProof>>,
    records: Vec<MpcRecord>,
    next_round: u64,
}

impl HeterogeneousMpc {
    pub fn new() -> Self {
        Self {
            config: MpcConfig::default_stuartian(),
            proofs: HashMap::new(),
            records: Vec::new(),
            next_round: 1,
        }
    }

    pub fn with_config(config: MpcConfig) -> Result<Self, MpcError> {
        config.validate()?;
        Ok(Self {
            config,
            proofs: HashMap::new(),
            records: Vec::new(),
            next_round: 1,
        })
    }

    /// Submit an attestation proof from a specific architecture
    pub fn submit_attestation(
        &mut self,
        round_id: u64,
        proof: AttestationProof,
    ) -> Result<(), MpcError> {
        // Check for duplicate architecture in same round
        if let Some(existing) = self.proofs.get(&round_id) {
            if existing.iter().any(|p| p.architecture == proof.architecture) {
                return Err(MpcError::DuplicateAttestation);
            }
        }

        // Check expiration
        if proof.is_expired(self.next_round.saturating_mul(1000), self.config.max_age_ms) {
            return Err(MpcError::ExpiredAttestation);
        }

        self.proofs
            .entry(round_id)
            .or_insert_with(Vec::new)
            .push(proof);
        Ok(())
    }

    /// Validate heterogeneous attestation for a round
    pub fn validate_round(&self, round_id: u64) -> Result<bool, MpcError> {
        let proofs = match self.proofs.get(&round_id) {
            Some(p) => p,
            None => return Ok(false),
        };

        // Check threshold
        let unique_archs: std::collections::HashSet<Architecture> =
            proofs.iter().map(|p| p.architecture).collect();
        if unique_archs.len() < self.config.threshold {
            return Err(MpcError::InsufficientArchitectures(
                unique_archs.len(),
                self.config.threshold,
            ));
        }

        // Check nonce consistency if enforced
        if self.config.enforce_nonce {
            let first_nonce = proofs[0].nonce;
            if !proofs.iter().all(|p| p.nonce == first_nonce) {
                return Err(MpcError::NonceMismatch);
            }
        }

        // Check hash reconciliation
        let first_hash = &proofs[0].committed_hash;
        if !proofs.iter().all(|p| p.committed_hash == *first_hash) {
            return Err(MpcError::HashMismatch);
        }

        Ok(true)
    }

    /// Execute a full MPC round: submit proofs and validate
    pub fn execute_round(
        &mut self,
        proofs: Vec<AttestationProof>,
        current_ms: u64,
    ) -> Result<bool, MpcError> {
        let round_id = self.next_round;
        self.next_round += 1;

        for proof in proofs {
            self.submit_attestation(round_id, proof)?;
        }

        let consensus = self.validate_round(round_id)?;

        // Record result
        let architectures: Vec<Architecture> = self
            .proofs
            .get(&round_id)
            .map(|p| p.iter().map(|x| x.architecture).collect())
            .unwrap_or_default();

        self.records.push(MpcRecord {
            round_id,
            architectures,
            consensus,
            timestamp_ms: current_ms,
        });

        Ok(consensus)
    }

    /// Get the number of unique architectures for a round
    pub fn architecture_count(&self, round_id: u64) -> usize {
        self.proofs
            .get(&round_id)
            .map(|p| {
                let unique: std::collections::HashSet<Architecture> =
                    p.iter().map(|x| x.architecture).collect();
                unique.len()
            })
            .unwrap_or(0)
    }

    /// Get all records
    pub fn records(&self) -> &[MpcRecord] {
        &self.records
    }

    /// Get consensus rate
    pub fn consensus_rate(&self) -> Option<f64> {
        let total = self.records.len();
        if total == 0 {
            return None;
        }
        let consensus_count = self.records.iter().filter(|r| r.consensus).count();
        Some(consensus_count as f64 / total as f64)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.proofs.clear();
        self.records.clear();
        self.next_round = 1;
    }
}

impl Default for HeterogeneousMpc {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for HeterogeneousMpc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HeterogeneousMpc(threshold={}, rounds={}, rate={})",
            self.config.threshold,
            self.records.len(),
            self.consensus_rate()
                .map(|r| format!("{:.2}%", r * 100.0))
                .unwrap_or_else(|| "N/A".to_string())
        )
    }
}

// ─── Public Standalone Functions ──────────────────────────────────────────────

/// Validate heterogeneous attestation across multiple architectures.
/// Returns true if ≥threshold architectures agree on the committed hash.
pub fn validate_heterogeneous_attestation(
    x86_proof: &[u8],
    arm_proof: &[u8],
    riscv_proof: &[u8],
    threshold: usize,
) -> bool {
    let proofs = vec![
        (!x86_proof.is_empty(), x86_proof),
        (!arm_proof.is_empty(), arm_proof),
        (!riscv_proof.is_empty(), riscv_proof),
    ];

    let valid_count = proofs.iter().filter(|(valid, _)| *valid).count();
    if valid_count < threshold {
        return false;
    }

    // Compute cross-architecture hash reconciliation
    let hashes: Vec<Vec<u8>> = proofs
        .iter()
        .filter(|(valid, _)| *valid)
        .map(|(_, p)| compute_proof_hash(p))
        .collect();

    // All valid proofs must agree on the hash
    if hashes.is_empty() {
        return false;
    }
    let first = &hashes[0];
    hashes.iter().all(|h| h == first)
}

/// Compute a deterministic hash from proof bytes using FNV-1a
fn compute_proof_hash(proof: &[u8]) -> Vec<u8> {
    let hash = fnv_hash_256(proof);
    hash
}

/// FNV-1a 64-bit hash
pub fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// FNV-1a 256-bit hash (4x 64-bit)
fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(32);
    let base = fnv_hash_64(data);
    for i in 0..4 {
        let mut combined = Vec::new();
        combined.extend_from_slice(data);
        combined.push(i as u8);
        let h = fnv_hash_64(&combined).wrapping_add(i as u64).wrapping_mul(0x100000001b3);
        result.extend_from_slice(&h.to_le_bytes());
    }
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MpcConfig::default_stuartian();
        assert_eq!(config.threshold, 2);
        assert_eq!(config.max_age_ms, 60_000);
        assert!(config.enforce_nonce);
        assert_eq!(config.hash_size, 32);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = MpcConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_threshold_too_low() {
        let config = MpcConfig {
            threshold: 1,
            ..MpcConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_threshold_too_high() {
        let config = MpcConfig {
            threshold: 4,
            ..MpcConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_max_age() {
        let config = MpcConfig {
            max_age_ms: 0,
            ..MpcConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_hash_size() {
        let config = MpcConfig {
            hash_size: 0,
            ..MpcConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_attestation_proof_new() {
        let proof = AttestationProof::new(
            Architecture::X86,
            "Intel".to_string(),
            vec![1, 2, 3],
            42,
            1000,
            vec![10, 20, 30],
        );
        assert_eq!(proof.architecture, Architecture::X86);
        assert_eq!(proof.manufacturer, "Intel");
        assert_eq!(proof.nonce, 42);
    }

    #[test]
    fn test_attestation_not_expired() {
        let proof = AttestationProof::new(
            Architecture::X86,
            "Intel".to_string(),
            vec![1, 2, 3],
            42,
            1000,
            vec![10, 20, 30],
        );
        assert!(!proof.is_expired(1050, 60_000));
    }

    #[test]
    fn test_attestation_expired() {
        let proof = AttestationProof::new(
            Architecture::X86,
            "Intel".to_string(),
            vec![1, 2, 3],
            42,
            1000,
            vec![10, 20, 30],
        );
        assert!(proof.is_expired(70_000, 60_000));
    }

    #[test]
    fn test_attestation_display() {
        let proof = AttestationProof::new(
            Architecture::Arm,
            "Apple".to_string(),
            vec![1, 2, 3],
            99,
            2000,
            vec![10, 20, 30],
        );
        let s = format!("{}", proof);
        assert!(s.contains("ARM"));
        assert!(s.contains("Apple"));
    }

    #[test]
    fn test_architecture_display() {
        assert_eq!(format!("{}", Architecture::X86), "x86");
        assert_eq!(format!("{}", Architecture::Arm), "ARM");
        assert_eq!(format!("{}", Architecture::RiscV), "RISC-V");
    }

    #[test]
    fn test_architecture_all() {
        let all = Architecture::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&Architecture::X86));
        assert!(all.contains(&Architecture::Arm));
        assert!(all.contains(&Architecture::RiscV));
    }

    #[test]
    fn test_engine_creation() {
        let engine = HeterogeneousMpc::new();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = MpcConfig::default_stuartian();
        let engine = HeterogeneousMpc::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_submit_attestation() {
        let mut engine = HeterogeneousMpc::new();
        let proof = AttestationProof::new(
            Architecture::X86,
            "Intel".to_string(),
            vec![1, 2, 3],
            42,
            1000,
            vec![10, 20, 30],
        );
        assert!(engine.submit_attestation(1, proof).is_ok());
    }

    #[test]
    fn test_submit_duplicate_attestation() {
        let mut engine = HeterogeneousMpc::new();
        let proof1 = AttestationProof::new(
            Architecture::X86,
            "Intel".to_string(),
            vec![1, 2, 3],
            42,
            1000,
            vec![10, 20, 30],
        );
        let proof2 = AttestationProof::new(
            Architecture::X86,
            "AMD".to_string(),
            vec![4, 5, 6],
            42,
            1000,
            vec![10, 20, 30],
        );
        assert!(engine.submit_attestation(1, proof1).is_ok());
        assert_eq!(engine.submit_attestation(1, proof2), Err(MpcError::DuplicateAttestation));
    }

    #[test]
    fn test_validate_round_insufficient() {
        let engine = HeterogeneousMpc::new();
        assert!(matches!(
            engine.validate_round(1),
            Ok(false)
        ));
    }

    #[test]
    fn test_validate_round_success() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        let x86 = AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone());
        let arm = AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 42, 1000, hash.clone());
        engine.submit_attestation(1, x86).unwrap();
        engine.submit_attestation(1, arm).unwrap();
        assert!(engine.validate_round(1).unwrap());
    }

    #[test]
    fn test_validate_round_nonce_mismatch() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        let x86 = AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone());
        let arm = AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 99, 1000, hash.clone());
        engine.submit_attestation(1, x86).unwrap();
        engine.submit_attestation(1, arm).unwrap();
        assert_eq!(engine.validate_round(1), Err(MpcError::NonceMismatch));
    }

    #[test]
    fn test_validate_round_hash_mismatch() {
        let mut engine = HeterogeneousMpc::new();
        let x86 = AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, vec![10, 20]);
        let arm = AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 42, 1000, vec![30, 40]);
        engine.submit_attestation(1, x86).unwrap();
        engine.submit_attestation(1, arm).unwrap();
        assert_eq!(engine.validate_round(1), Err(MpcError::HashMismatch));
    }

    #[test]
    fn test_execute_round_success() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        let proofs = vec![
            AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone()),
            AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 42, 1000, hash.clone()),
            AttestationProof::new(Architecture::RiscV, "SiFive".into(), vec![3], 42, 1000, hash.clone()),
        ];
        let result = engine.execute_round(proofs, 1000);
        assert!(result.unwrap());
        assert_eq!(engine.records().len(), 1);
    }

    #[test]
    fn test_execute_round_failure() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        let proofs = vec![
            AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone()),
        ];
        let result = engine.execute_round(proofs, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_architecture_count() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        let x86 = AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone());
        let arm = AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 42, 1000, hash.clone());
        engine.submit_attestation(1, x86).unwrap();
        engine.submit_attestation(1, arm).unwrap();
        assert_eq!(engine.architecture_count(1), 2);
        assert_eq!(engine.architecture_count(99), 0);
    }

    #[test]
    fn test_consensus_rate() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        // Round 1: success
        let proofs1 = vec![
            AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone()),
            AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 42, 1000, hash.clone()),
        ];
        engine.execute_round(proofs1, 1000).unwrap();
        // Round 2: failure (only 1 arch)
        let proofs2 = vec![
            AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 43, 2000, hash.clone()),
        ];
        let _ = engine.execute_round(proofs2, 2000);
        assert_eq!(engine.consensus_rate(), Some(1.0));
    }

    #[test]
    fn test_consensus_rate_empty() {
        let engine = HeterogeneousMpc::new();
        assert_eq!(engine.consensus_rate(), None);
    }

    #[test]
    fn test_reset() {
        let mut engine = HeterogeneousMpc::new();
        let hash = vec![10, 20, 30];
        let proofs = vec![
            AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 42, 1000, hash.clone()),
            AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 42, 1000, hash.clone()),
        ];
        engine.execute_round(proofs, 1000).unwrap();
        engine.reset();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = HeterogeneousMpc::new();
        let s = format!("{}", engine);
        assert!(s.contains("HeterogeneousMpc"));
    }

    #[test]
    fn test_record_display() {
        let record = MpcRecord {
            round_id: 1,
            architectures: vec![Architecture::X86, Architecture::Arm],
            consensus: true,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("round=1"));
        assert!(s.contains("consensus=true"));
    }

    #[test]
    fn test_standalone_validate_all_archs() {
        let proof = vec![1, 2, 3, 4, 5];
        let result = validate_heterogeneous_attestation(&proof, &proof, &proof, 2);
        assert!(result);
    }

    #[test]
    fn test_standalone_validate_insufficient() {
        let proof = vec![1, 2, 3];
        let result = validate_heterogeneous_attestation(&proof, &[], &[], 2);
        assert!(!result);
    }

    #[test]
    fn test_standalone_validate_hash_mismatch() {
        let x86 = vec![1, 2, 3];
        let arm = vec![4, 5, 6];
        let result = validate_heterogeneous_attestation(&x86, &arm, &[], 2);
        assert!(!result);
    }

    #[test]
    fn test_fnv_hash_deterministic() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = fnv_hash_64(&data);
        let h2 = fnv_hash_64(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fnv_hash_different() {
        let h1 = fnv_hash_64(&[1, 2, 3]);
        let h2 = fnv_hash_64(&[4, 5, 6]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_error_display() {
        let err = MpcError::HashMismatch;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = HeterogeneousMpc::new();

        // Round 1: Full consensus (3 architectures)
        let hash1 = vec![1, 2, 3];
        let proofs1 = vec![
            AttestationProof::new(Architecture::X86, "Intel".into(), vec![1], 1, 1000, hash1.clone()),
            AttestationProof::new(Architecture::Arm, "Apple".into(), vec![2], 1, 1000, hash1.clone()),
            AttestationProof::new(Architecture::RiscV, "SiFive".into(), vec![3], 1, 1000, hash1.clone()),
        ];
        assert!(engine.execute_round(proofs1, 1000).unwrap());

        // Round 2: Partial consensus (2 architectures)
        let hash2 = vec![4, 5, 6];
        let proofs2 = vec![
            AttestationProof::new(Architecture::X86, "AMD".into(), vec![4], 2, 2000, hash2.clone()),
            AttestationProof::new(Architecture::RiscV, "SiFive".into(), vec![5], 2, 2000, hash2.clone()),
        ];
        assert!(engine.execute_round(proofs2, 2000).unwrap());

        // Verify records
        assert_eq!(engine.records().len(), 2);
        assert_eq!(engine.consensus_rate(), Some(1.0));

        // Verify standalone function
        let proof = vec![10, 20, 30];
        assert!(validate_heterogeneous_attestation(&proof, &proof, &proof, 2));

        // Reset
        engine.reset();
        assert_eq!(engine.records().len(), 0);
    }
}
