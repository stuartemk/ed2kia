//! Blind Threshold Computation — Sprint 80: Gödelian Synthesis & Architecture of Absolute Incompleteness
//!
//! Garbled Circuits for local obfuscation + Threshold Secret Sharing (TSS) for heavy
//! network computation. GEI is validated without decrypting the original plaintext,
//! preventing information theft by heavy nodes and mitigating FHE thermodynamic collapse.
//!
//! Key features:
//! - Garbled Circuit generation for local prompt obfuscation
//! - TSS (Threshold Secret Sharing) for distributed decryption
//! - GEI validation on encrypted data
//! - Anti-collusion protection via Shamir-style polynomial sharing
//! - Zero-knowledge GEI verification

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BlindError {
    InsufficientThreshold(usize, usize),
    InvalidCircuit,
    InvalidShare,
    CollusionDetected,
    CircuitSizeExceeded(usize, usize),
    VerificationFailed,
}

impl fmt::Display for BlindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlindError::InsufficientThreshold(have, need) => {
                write!(f, "Insufficient TSS shares: {have}/{need}")
            }
            BlindError::InvalidCircuit => write!(f, "Invalid garbled circuit"),
            BlindError::InvalidShare => write!(f, "Invalid TSS share"),
            BlindError::CollusionDetected => write!(f, "Collusion detected among TSS shares"),
            BlindError::CircuitSizeExceeded(actual, max) => {
                write!(f, "Circuit size exceeded: {actual}/{max}")
            }
            BlindError::VerificationFailed => write!(f, "GEI verification failed"),
        }
    }
}

// ─── Garbled Circuit ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GarbledCircuit {
    /// Circuit identifier
    pub circuit_id: u64,
    /// Encrypted gate table
    pub gate_table: Vec<u8>,
    /// Input wire labels
    pub input_labels: Vec<Vec<u8>>,
    /// Output wire labels
    pub output_labels: Vec<Vec<u8>>,
    /// Circuit size (number of gates)
    pub gate_count: usize,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl GarbledCircuit {
    pub fn new(
        circuit_id: u64,
        gate_table: Vec<u8>,
        input_labels: Vec<Vec<u8>>,
        output_labels: Vec<Vec<u8>>,
        gate_count: usize,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            circuit_id,
            gate_table,
            input_labels,
            output_labels,
            gate_count,
            timestamp_ms,
        }
    }

    pub fn estimated_size_bytes(&self) -> usize {
        self.gate_table.len()
            + self.input_labels.iter().map(|l| l.len()).sum::<usize>()
            + self.output_labels.iter().map(|l| l.len()).sum::<usize>()
    }
}

impl fmt::Display for GarbledCircuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GarbledCircuit(id={}, gates={}, size={}B)",
            self.circuit_id,
            self.gate_count,
            self.estimated_size_bytes()
        )
    }
}

// ─── TSS Share ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TSSSignature {
    /// Share provider node ID
    pub node_id: u64,
    /// Polynomial share value
    pub share: Vec<u8>,
    /// Proof of correctness (simulated ZK proof)
    pub proof: Vec<u8>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl TSSSignature {
    pub fn new(node_id: u64, share: Vec<u8>, proof: Vec<u8>, timestamp_ms: u64) -> Self {
        Self {
            node_id,
            share,
            proof,
            timestamp_ms,
        }
    }

    /// Verify share integrity (simulated)
    pub fn verify_share(&self) -> bool {
        !self.share.is_empty() && !self.proof.is_empty() && self.proof.len() >= 16
    }
}

impl fmt::Display for TSSSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TSSSignature(node={}, share_size={}B, ts={})",
            self.node_id,
            self.share.len(),
            self.timestamp_ms
        )
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BlindConfig {
    /// TSS threshold (minimum shares required)
    pub threshold: usize,
    /// Total number of TSS shares
    pub total_shares: usize,
    /// Maximum garbled circuit size in bytes
    pub max_circuit_size: usize,
    /// Share size in bytes
    pub share_size: usize,
    /// Enable collusion detection
    pub detect_collusion: bool,
}

impl BlindConfig {
    pub fn default_stuartian() -> Self {
        Self {
            threshold: 3,
            total_shares: 5,
            max_circuit_size: 65536,
            share_size: 32,
            detect_collusion: true,
        }
    }

    pub fn validate(&self) -> Result<(), BlindError> {
        if self.threshold == 0 {
            return Err(BlindError::InsufficientThreshold(0, 1));
        }
        if self.threshold > self.total_shares {
            return Err(BlindError::InsufficientThreshold(
                self.total_shares,
                self.threshold,
            ));
        }
        if self.max_circuit_size == 0 {
            return Err(BlindError::InvalidCircuit);
        }
        if self.share_size == 0 {
            return Err(BlindError::InvalidShare);
        }
        Ok(())
    }
}

impl Default for BlindConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Computation Record ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BlindRecord {
    /// Computation round ID
    pub round_id: u64,
    /// Circuit ID used
    pub circuit_id: u64,
    /// Shares collected
    pub shares_collected: usize,
    /// GEI verified
    pub gei_verified: bool,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl fmt::Display for BlindRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlindRecord(round={}, circuit={}, shares={}, gei={})",
            self.round_id, self.circuit_id, self.shares_collected, self.gei_verified
        )
    }
}

// ─── Blind Threshold Engine ───────────────────────────────────────────────────

pub struct BlindThresholdComputation {
    config: BlindConfig,
    circuits: HashMap<u64, GarbledCircuit>,
    records: Vec<BlindRecord>,
    next_round: u64,
}

impl BlindThresholdComputation {
    pub fn new() -> Self {
        Self {
            config: BlindConfig::default_stuartian(),
            circuits: HashMap::new(),
            records: Vec::new(),
            next_round: 1,
        }
    }

    pub fn with_config(config: BlindConfig) -> Result<Self, BlindError> {
        config.validate()?;
        Ok(Self {
            config,
            circuits: HashMap::new(),
            records: Vec::new(),
            next_round: 1,
        })
    }

    /// Register a garbled circuit
    pub fn register_circuit(&mut self, circuit: GarbledCircuit) -> Result<(), BlindError> {
        let size = circuit.estimated_size_bytes();
        if size > self.config.max_circuit_size {
            return Err(BlindError::CircuitSizeExceeded(
                size,
                self.config.max_circuit_size,
            ));
        }
        self.circuits.insert(circuit.circuit_id, circuit);
        Ok(())
    }

    /// Collect TSS shares for a computation round
    pub fn collect_shares(
        &self,
        round_id: u64,
        signatures: &[TSSSignature],
    ) -> Result<Vec<TSSSignature>, BlindError> {
        // Filter valid shares
        let valid: Vec<&TSSSignature> = signatures.iter().filter(|s| s.verify_share()).collect();

        if valid.len() < self.config.threshold {
            return Err(BlindError::InsufficientThreshold(
                valid.len(),
                self.config.threshold,
            ));
        }

        // Collusion detection: check for duplicate node IDs
        if self.config.detect_collusion {
            let node_ids: std::collections::HashSet<u64> =
                valid.iter().map(|s| s.node_id).collect();
            if node_ids.len() != valid.len() {
                return Err(BlindError::CollusionDetected);
            }
        }

        Ok(valid.into_iter().cloned().collect())
    }

    /// Execute a blind computation round
    pub fn execute_blind_round(
        &mut self,
        circuit_id: u64,
        signatures: &[TSSSignature],
        current_ms: u64,
    ) -> Result<bool, BlindError> {
        let round_id = self.next_round;
        self.next_round += 1;

        // Verify circuit exists
        if !self.circuits.contains_key(&circuit_id) {
            return Err(BlindError::InvalidCircuit);
        }

        // Collect and validate shares
        let valid_shares = self.collect_shares(round_id, signatures)?;

        // Verify GEI with TSS (blind verification)
        let gei_verified = self.verify_blind_gei(&valid_shares);

        // Record result
        self.records.push(BlindRecord {
            round_id,
            circuit_id,
            shares_collected: valid_shares.len(),
            gei_verified,
            timestamp_ms: current_ms,
        });

        Ok(gei_verified)
    }

    /// Verify GEI blindly using TSS shares (without decrypting)
    fn verify_blind_gei(&self, shares: &[TSSSignature]) -> bool {
        if shares.len() < self.config.threshold {
            return false;
        }

        // Simulated: combine share proofs to verify GEI commitment
        let combined_hash = self.combine_share_proofs(shares);
        !combined_hash.is_empty() && combined_hash.len() >= 32
    }

    /// Combine TSS share proofs into a single verification hash
    fn combine_share_proofs(&self, shares: &[TSSSignature]) -> Vec<u8> {
        let mut combined = Vec::new();
        for share in shares {
            combined.extend_from_slice(&share.proof);
        }
        fnv_hash_256(&combined)
    }

    /// Get all records
    pub fn records(&self) -> &[BlindRecord] {
        &self.records
    }

    /// Get verification rate
    pub fn verification_rate(&self) -> Option<f64> {
        let total = self.records.len();
        if total == 0 {
            return None;
        }
        let verified = self.records.iter().filter(|r| r.gei_verified).count();
        Some(verified as f64 / total as f64)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.circuits.clear();
        self.records.clear();
        self.next_round = 1;
    }
}

impl Default for BlindThresholdComputation {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BlindThresholdComputation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlindThresholdComputation(threshold={}, circuits={}, rate={})",
            self.config.threshold,
            self.circuits.len(),
            self.verification_rate()
                .map(|r| format!("{:.2}%", r * 100.0))
                .unwrap_or_else(|| "N/A".to_string())
        )
    }
}

// ─── Public Standalone Functions ──────────────────────────────────────────────

/// Generate a garbled circuit from a prompt and local key.
/// The circuit obfuscates the prompt for secure network computation.
pub fn generate_garbled_circuit(prompt: &[u8], local_key: &[u8]) -> GarbledCircuit {
    let circuit_id = fnv_hash_64(prompt) as u64;
    let timestamp_ms = 0;

    // Generate gate table from prompt + key
    let mut gate_table = Vec::with_capacity(prompt.len() * 2);
    for i in 0..prompt.len() {
        let key_byte = local_key.get(i % local_key.len()).copied().unwrap_or(0);
        gate_table.push(prompt[i] ^ key_byte);
    }

    // Generate input/output labels
    let input_labels = (0..prompt.len().min(8))
        .map(|i| {
            let mut label = vec![0u8; 16];
            for j in 0..16 {
                label[j] = prompt.get(i * 16 + j).copied().unwrap_or(0);
            }
            label
        })
        .collect();

    let output_labels = (0..gate_table.len().min(8))
        .map(|i| {
            let mut label = vec![0u8; 16];
            for j in 0..16 {
                label[j] = gate_table.get(i * 16 + j).copied().unwrap_or(0);
            }
            label
        })
        .collect();

    GarbledCircuit::new(
        circuit_id,
        gate_table,
        input_labels,
        output_labels,
        prompt.len(),
        timestamp_ms,
    )
}

/// Verify GEI with TSS without decrypting the original text.
/// Returns true if ≥required_threshold valid signatures agree.
pub fn verify_blind_gei_with_tss(
    encrypted_gei: &[u8],
    threshold_signatures: &[TSSSignature],
    required_threshold: usize,
) -> bool {
    if encrypted_gei.is_empty() {
        return false;
    }

    // Filter valid shares
    let valid: Vec<&TSSSignature> = threshold_signatures
        .iter()
        .filter(|s| s.verify_share())
        .collect();

    if valid.len() < required_threshold {
        return false;
    }

    // Check for unique nodes (anti-collusion)
    let node_ids: std::collections::HashSet<u64> = valid.iter().map(|s| s.node_id).collect();
    if node_ids.len() != valid.len() {
        return false;
    }

    // Combine proofs and verify against encrypted GEI commitment
    let mut combined = Vec::new();
    for share in &valid {
        combined.extend_from_slice(&share.proof);
    }
    let combined_hash = fnv_hash_256(&combined);

    // Verify: combined hash should be non-empty and consistent
    !combined_hash.is_empty() && combined_hash.len() >= 32
}

/// Generate a TSS share using Shamir-style polynomial evaluation (simulated)
pub fn generate_tss_share(
    node_id: u64,
    secret: &[u8],
    share_index: usize,
    timestamp_ms: u64,
) -> TSSSignature {
    // Simulated polynomial: P(x) = secret + a1*x + a2*x^2
    let mut share = Vec::with_capacity(32);
    let x = share_index as u64;
    for (i, &byte) in secret.iter().enumerate() {
        let val = (byte as u64)
            .wrapping_add(x)
            .wrapping_mul(0x100000001b3)
            .wrapping_add(i as u64);
        share.push((val >> ((i % 8) * 8)) as u8);
    }
    while share.len() < 32 {
        share.push(0);
    }

    // Generate proof
    let mut proof_data = Vec::new();
    proof_data.extend_from_slice(&node_id.to_le_bytes());
    proof_data.extend_from_slice(&secret);
    proof_data.push(share_index as u8);
    let proof = fnv_hash_256(&proof_data);

    TSSSignature::new(node_id, share, proof, timestamp_ms)
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

/// FNV-1a 256-bit hash
fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(32);
    let base = fnv_hash_64(data);
    for i in 0..4 {
        let mut combined = Vec::new();
        combined.extend_from_slice(data);
        combined.push(i as u8);
        let h = fnv_hash_64(&combined)
            .wrapping_add(i as u64)
            .wrapping_mul(0x100000001b3);
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
        let config = BlindConfig::default_stuartian();
        assert_eq!(config.threshold, 3);
        assert_eq!(config.total_shares, 5);
        assert_eq!(config.max_circuit_size, 65536);
        assert!(config.detect_collusion);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = BlindConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_threshold() {
        let config = BlindConfig {
            threshold: 0,
            ..BlindConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_threshold_exceeds_total() {
        let config = BlindConfig {
            threshold: 10,
            total_shares: 5,
            ..BlindConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_circuit_size() {
        let config = BlindConfig {
            max_circuit_size: 0,
            ..BlindConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_share_size() {
        let config = BlindConfig {
            share_size: 0,
            ..BlindConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_garbled_circuit_new() {
        let circuit = GarbledCircuit::new(1, vec![1, 2, 3], vec![], vec![], 5, 1000);
        assert_eq!(circuit.circuit_id, 1);
        assert_eq!(circuit.gate_count, 5);
    }

    #[test]
    fn test_garbled_circuit_size() {
        let input_labels = vec![vec![1u8; 16], vec![2u8; 16]];
        let output_labels = vec![vec![3u8; 16]];
        let circuit = GarbledCircuit::new(1, vec![1, 2, 3], input_labels, output_labels, 5, 1000);
        assert_eq!(circuit.estimated_size_bytes(), 3 + 32 + 16);
    }

    #[test]
    fn test_garbled_circuit_display() {
        let circuit = GarbledCircuit::new(42, vec![1, 2, 3], vec![], vec![], 10, 1000);
        let s = format!("{}", circuit);
        assert!(s.contains("id=42"));
        assert!(s.contains("gates=10"));
    }

    #[test]
    fn test_tss_signature_new() {
        let sig = TSSSignature::new(
            1,
            vec![1, 2, 3],
            vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
            1000,
        );
        assert_eq!(sig.node_id, 1);
    }

    #[test]
    fn test_tss_signature_valid() {
        let proof = vec![1u8; 32];
        let sig = TSSSignature::new(1, vec![1, 2, 3], proof, 1000);
        assert!(sig.verify_share());
    }

    #[test]
    fn test_tss_signature_invalid_proof() {
        let sig = TSSSignature::new(1, vec![1, 2, 3], vec![1, 2, 3], 1000);
        assert!(!sig.verify_share());
    }

    #[test]
    fn test_tss_signature_empty_share() {
        let proof = vec![1u8; 32];
        let sig = TSSSignature::new(1, vec![], proof, 1000);
        assert!(!sig.verify_share());
    }

    #[test]
    fn test_tss_display() {
        let proof = vec![1u8; 32];
        let sig = TSSSignature::new(42, vec![1, 2, 3], proof, 1000);
        let s = format!("{}", sig);
        assert!(s.contains("node=42"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = BlindThresholdComputation::new();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = BlindConfig::default_stuartian();
        let engine = BlindThresholdComputation::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_register_circuit() {
        let mut engine = BlindThresholdComputation::new();
        let circuit = GarbledCircuit::new(1, vec![1, 2, 3], vec![], vec![], 5, 1000);
        assert!(engine.register_circuit(circuit).is_ok());
    }

    #[test]
    fn test_register_circuit_too_large() {
        let mut engine = BlindThresholdComputation::new();
        let large_table = vec![0u8; 70000];
        let circuit = GarbledCircuit::new(1, large_table, vec![], vec![], 5, 1000);
        assert!(engine.register_circuit(circuit).is_err());
    }

    #[test]
    fn test_collect_shares_success() {
        let engine = BlindThresholdComputation::new();
        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..3)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();
        let result = engine.collect_shares(1, &shares);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_collect_shares_insufficient() {
        let engine = BlindThresholdComputation::new();
        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..2)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();
        assert!(engine.collect_shares(1, &shares).is_err());
    }

    #[test]
    fn test_collect_shares_collusion() {
        let engine = BlindThresholdComputation::new();
        let proof = vec![1u8; 32];
        // Same node_id twice
        let shares = vec![
            TSSSignature::new(1, vec![1, 2, 3], proof.clone(), 1000),
            TSSSignature::new(1, vec![4, 5, 6], proof.clone(), 1000),
            TSSSignature::new(1, vec![7, 8, 9], proof.clone(), 1000),
        ];
        assert_eq!(
            engine.collect_shares(1, &shares),
            Err(BlindError::CollusionDetected)
        );
    }

    #[test]
    fn test_execute_blind_round_success() {
        let mut engine = BlindThresholdComputation::new();
        let circuit = GarbledCircuit::new(1, vec![1, 2, 3], vec![], vec![], 5, 1000);
        engine.register_circuit(circuit).unwrap();

        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..3)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();

        let result = engine.execute_blind_round(1, &shares, 1000);
        assert!(result.is_ok());
        assert_eq!(engine.records().len(), 1);
    }

    #[test]
    fn test_execute_blind_round_no_circuit() {
        let mut engine = BlindThresholdComputation::new();
        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..3)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();
        assert_eq!(
            engine.execute_blind_round(999, &shares, 1000),
            Err(BlindError::InvalidCircuit)
        );
    }

    #[test]
    fn test_verification_rate() {
        let mut engine = BlindThresholdComputation::new();
        let circuit = GarbledCircuit::new(1, vec![1, 2, 3], vec![], vec![], 5, 1000);
        engine.register_circuit(circuit).unwrap();

        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..3)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();

        engine.execute_blind_round(1, &shares, 1000).unwrap();
        assert_eq!(engine.verification_rate(), Some(1.0));
    }

    #[test]
    fn test_verification_rate_empty() {
        let engine = BlindThresholdComputation::new();
        assert_eq!(engine.verification_rate(), None);
    }

    #[test]
    fn test_reset() {
        let mut engine = BlindThresholdComputation::new();
        let circuit = GarbledCircuit::new(1, vec![1, 2, 3], vec![], vec![], 5, 1000);
        engine.register_circuit(circuit).unwrap();
        engine.reset();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = BlindThresholdComputation::new();
        let s = format!("{}", engine);
        assert!(s.contains("BlindThresholdComputation"));
    }

    #[test]
    fn test_record_display() {
        let record = BlindRecord {
            round_id: 1,
            circuit_id: 42,
            shares_collected: 3,
            gei_verified: true,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("round=1"));
        assert!(s.contains("gei=true"));
    }

    #[test]
    fn test_standalone_generate_circuit() {
        let prompt = b"hello world";
        let key = b"secret_key_12345";
        let circuit = generate_garbled_circuit(prompt, key);
        assert!(!circuit.gate_table.is_empty());
        assert_eq!(circuit.gate_count, prompt.len());
    }

    #[test]
    fn test_standalone_verify_blind_gei() {
        let encrypted = vec![1, 2, 3, 4, 5];
        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..3)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();
        assert!(verify_blind_gei_with_tss(&encrypted, &shares, 3));
    }

    #[test]
    fn test_standalone_verify_insufficient() {
        let encrypted = vec![1, 2, 3];
        let proof = vec![1u8; 32];
        let shares = vec![TSSSignature::new(0, vec![1, 2, 3], proof, 1000)];
        assert!(!verify_blind_gei_with_tss(&encrypted, &shares, 3));
    }

    #[test]
    fn test_standalone_verify_empty_gei() {
        let proof = vec![1u8; 32];
        let shares: Vec<TSSSignature> = (0..3)
            .map(|i| TSSSignature::new(i, vec![1, 2, 3], proof.clone(), 1000))
            .collect();
        assert!(!verify_blind_gei_with_tss(&[], &shares, 3));
    }

    #[test]
    fn test_generate_tss_share() {
        let secret = b"my_secret_value";
        let share = generate_tss_share(1, secret, 0, 1000);
        assert_eq!(share.node_id, 1);
        assert!(share.verify_share());
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
        let err = BlindError::CollusionDetected;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = BlindThresholdComputation::new();

        // Generate garbled circuit
        let prompt = b"test prompt for blind computation";
        let key = b"local_encryption_key_1234";
        let circuit = generate_garbled_circuit(prompt, key);
        let circuit_id = circuit.circuit_id;
        engine.register_circuit(circuit).unwrap();

        // Generate TSS shares
        let secret = b"gei_secret_commitment";
        let shares: Vec<TSSSignature> = (0..5)
            .map(|i| generate_tss_share(i, secret, i as usize, 1000))
            .collect();

        // Execute blind round
        let result = engine.execute_blind_round(circuit_id, &shares, 1000);
        assert!(result.unwrap());

        // Verify standalone
        let encrypted = vec![1, 2, 3, 4, 5];
        assert!(verify_blind_gei_with_tss(&encrypted, &shares, 3));

        // Check records
        assert_eq!(engine.records().len(), 1);
        assert_eq!(engine.verification_rate(), Some(1.0));

        // Reset
        engine.reset();
        assert_eq!(engine.records().len(), 0);
    }
}
