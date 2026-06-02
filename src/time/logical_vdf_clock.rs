//! Logical VDF Clock — Sprint 77: Physics of Consciousness & Thermodynamic Finality
//!
//! Resuelve el bug ontológico: dependencia de NTP/PTP vulnerable a Time-Spoofing.
//!
//! Implementa Relojes Lógicos + Verifiable Delay Functions (VDFs).
//! Tiempo relativo al cómputo, no a servidores externos. Inmune a spoofing.
//!
//! # Garantías
//!
//! - VDF Generation: O(n) secuencial (no paralelizable)
//! - VDF Verification: O(log n) con shortcut (simulado aquí)
//! - Inmunidad: sin dependencia de NTP/PTP

use std::collections::HashMap;
use std::fmt;

/// Error types for Logical VDF Clock
#[derive(Debug, Clone, PartialEq)]
pub enum VdfError {
    /// Invalid iteration count
    InvalidIterations(u64),
    /// Empty seed
    EmptySeed,
    /// Proof verification failed
    VerificationFailed,
    /// Clock regression detected
    ClockRegression(u64, u64),
    /// Invalid public parameters
    InvalidParams,
}

impl fmt::Display for VdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VdfError::InvalidIterations(n) => write!(f, "Invalid iterations: {}", n),
            VdfError::EmptySeed => write!(f, "Empty seed"),
            VdfError::VerificationFailed => write!(f, "VDF verification failed"),
            VdfError::ClockRegression(old, new) => {
                write!(f, "Clock regression: {} → {}", old, new)
            }
            VdfError::InvalidParams => write!(f, "Invalid public parameters"),
        }
    }
}

impl std::error::Error for VdfError {}

/// VDF Configuration.
#[derive(Debug, Clone)]
pub struct VdfConfig {
    /// Default iteration count for VDF generation
    pub default_iterations: u64,
    /// Minimum iterations allowed
    pub min_iterations: u64,
    /// Maximum iterations allowed
    pub max_iterations: u64,
    /// Tolerance for clock synchronization (ms)
    pub sync_tolerance_ms: u64,
}

impl VdfConfig {
    pub fn default_stuartian() -> Self {
        Self {
            default_iterations: 1000,
            min_iterations: 100,
            max_iterations: 100_000,
            sync_tolerance_ms: 1000,
        }
    }

    pub fn validate(&self) -> Result<(), VdfError> {
        if self.min_iterations == 0 {
            return Err(VdfError::InvalidIterations(0));
        }
        if self.max_iterations < self.min_iterations {
            return Err(VdfError::InvalidIterations(self.max_iterations));
        }
        Ok(())
    }
}

impl Default for VdfConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// VDF Proof structure.
#[derive(Debug, Clone)]
pub struct VDFProof {
    /// Input seed hash
    pub seed_hash: u64,
    /// Number of iterations performed
    pub iterations: u64,
    /// Final output value
    pub output: u64,
    /// Intermediate checkpoint (for verification)
    pub checkpoint: u64,
    /// Logical timestamp
    pub logical_timestamp: u64,
}

impl VDFProof {
    pub fn new(
        seed_hash: u64,
        iterations: u64,
        output: u64,
        checkpoint: u64,
        logical_ts: u64,
    ) -> Self {
        Self {
            seed_hash,
            iterations,
            output,
            checkpoint,
            logical_timestamp: logical_ts,
        }
    }
}

impl fmt::Display for VDFProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VDFProof {{ seed={:#x}, iters={}, output={:#x}, ts={} }}",
            self.seed_hash, self.iterations, self.output, self.logical_timestamp
        )
    }
}

/// Logical clock state for a node.
#[derive(Debug, Clone)]
pub struct LogicalClockState {
    /// Node identifier
    pub node_id: u64,
    /// Current logical timestamp
    pub logical_timestamp: u64,
    /// Last VDF proof
    pub last_proof: Option<VDFProof>,
    /// Total VDFs generated
    pub vdf_count: u32,
}

impl LogicalClockState {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            logical_timestamp: 0,
            last_proof: None,
            vdf_count: 0,
        }
    }

    pub fn advance(&mut self, proof: VDFProof) {
        self.logical_timestamp = proof.logical_timestamp;
        self.last_proof = Some(proof);
        self.vdf_count += 1;
    }
}

impl fmt::Display for LogicalClockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LogicalClock {{ node={}, ts={}, vdfs={} }}",
            self.node_id, self.logical_timestamp, self.vdf_count
        )
    }
}

/// Stateful engine for Logical VDF Clock.
#[derive(Debug, Clone)]
pub struct LogicalVdfClock {
    config: VdfConfig,
    clocks: HashMap<u64, LogicalClockState>,
    public_params: u64,
}

impl LogicalVdfClock {
    pub fn new() -> Self {
        Self {
            config: VdfConfig::default_stuartian(),
            clocks: HashMap::new(),
            public_params: 0xDEAD_BEEF_CAFE_BABE,
        }
    }

    pub fn with_config(config: VdfConfig) -> Result<Self, VdfError> {
        config.validate()?;
        Ok(Self {
            config,
            clocks: HashMap::new(),
            public_params: 0xDEAD_BEEF_CAFE_BABE,
        })
    }

    /// Register a node with a logical clock.
    pub fn register_node(&mut self, node_id: u64) {
        self.clocks.insert(node_id, LogicalClockState::new(node_id));
    }

    /// Generate a VDF proof for a node.
    pub fn generate_vdf(
        &mut self,
        node_id: u64,
        seed: &[u8],
        iterations: Option<u64>,
    ) -> Result<VDFProof, VdfError> {
        if seed.is_empty() {
            return Err(VdfError::EmptySeed);
        }

        let iters = iterations.unwrap_or(self.config.default_iterations);
        if iters < self.config.min_iterations || iters > self.config.max_iterations {
            return Err(VdfError::InvalidIterations(iters));
        }

        let seed_hash = Self::hash_bytes(seed);
        let proof = Self::generate_vdf_proof(seed_hash, iters);

        let clock = self
            .clocks
            .get_mut(&node_id)
            .ok_or(VdfError::ClockRegression(0, 0))?;
        clock.advance(proof.clone());

        Ok(proof)
    }

    /// Verify a VDF proof.
    pub fn verify_vdf(&self, proof: &VDFProof) -> bool {
        // Verify iterations are in valid range
        if proof.iterations < self.config.min_iterations
            || proof.iterations > self.config.max_iterations
        {
            return false;
        }
        // Verify output matches expected computation (simplified)
        let expected = Self::verify_vdf_computation(proof.seed_hash, proof.iterations);
        proof.output == expected
    }

    /// Get logical timestamp for a node.
    pub fn get_logical_timestamp(&self, node_id: u64) -> Option<u64> {
        self.clocks.get(&node_id).map(|c| c.logical_timestamp)
    }

    /// Check if two clocks are synchronized (within tolerance).
    pub fn are_synchronized(&self, node_a: u64, node_b: u64) -> bool {
        match (
            self.get_logical_timestamp(node_a),
            self.get_logical_timestamp(node_b),
        ) {
            (Some(ts_a), Some(ts_b)) => {
                let diff = if ts_a > ts_b {
                    ts_a - ts_b
                } else {
                    ts_b - ts_a
                };
                diff <= self.config.sync_tolerance_ms
            }
            _ => false,
        }
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.clocks.len()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.clocks.clear();
    }

    // ─── Internal VDF Implementation ───

    /// Generate VDF proof (sequential computation).
    pub fn generate_vdf_proof(seed: u64, iterations: u64) -> VDFProof {
        let mut state = seed;
        let mut checkpoint = 0u64;

        for i in 1..=iterations {
            // Sequential reduction: state = fnv1a_hash(state)
            state ^= state.rotate_left(17);
            state = state.wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
            state ^= state.rotate_right(31);

            // Store checkpoint at midpoint
            if i == iterations / 2 {
                checkpoint = state;
            }
        }

        VDFProof::new(seed, iterations, state, checkpoint, iterations)
    }

    /// Verify VDF computation (simplified O(1) check).
    pub fn verify_vdf_computation(seed: u64, iterations: u64) -> u64 {
        // In production, this would use a shortcut (e.g., RSA-based VDF)
        // For simulation, we recompute
        let proof = Self::generate_vdf_proof(seed, iterations);
        proof.output
    }

    /// FNV-1a hash of byte slice to u64.
    pub fn hash_bytes(data: &[u8]) -> u64 {
        let mut hash: u64 = 146_959_810_393_466_564_3;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(109_951_163_424_807_173);
        }
        hash
    }
}

impl Default for LogicalVdfClock {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LogicalVdfClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LogicalVdfClock {{ nodes={}, params={:#x} }}",
            self.node_count(),
            self.public_params
        )
    }
}

// ─── Public Standalone Functions ─────────────────────────────────────────────

/// Generate a VDF proof (standalone).
pub fn generate_vdf_proof(seed: &[u8], iterations: u64) -> VDFProof {
    let seed_hash = LogicalVdfClock::hash_bytes(seed);
    LogicalVdfClock::generate_vdf_proof(seed_hash, iterations)
}

/// Verify a VDF proof (standalone).
pub fn verify_vdf_proof(proof: &VDFProof, _public_params: &[u8]) -> bool {
    let expected = LogicalVdfClock::verify_vdf_computation(proof.seed_hash, proof.iterations);
    proof.output == expected
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = VdfConfig::default_stuartian();
        assert!(config.validate().is_ok());
        assert_eq!(config.default_iterations, 1000);
    }

    #[test]
    fn test_config_invalid_min() {
        let config = VdfConfig {
            min_iterations: 0,
            ..VdfConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_max_less_than_min() {
        let config = VdfConfig {
            max_iterations: 50,
            min_iterations: 100,
            ..VdfConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_engine_creation() {
        let engine = LogicalVdfClock::new();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        assert_eq!(engine.node_count(), 1);
    }

    #[test]
    fn test_generate_vdf() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        let proof = engine.generate_vdf(1, &[1, 2, 3], Some(100)).unwrap();
        assert_eq!(proof.iterations, 100);
        assert!(proof.output > 0);
    }

    #[test]
    fn test_generate_vdf_empty_seed() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        assert!(engine.generate_vdf(1, &[], Some(100)).is_err());
    }

    #[test]
    fn test_generate_vdf_invalid_iterations() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        assert!(engine.generate_vdf(1, &[1], Some(10)).is_err()); // below min
    }

    #[test]
    fn test_generate_vdf_unknown_node() {
        let mut engine = LogicalVdfClock::new();
        assert!(engine.generate_vdf(999, &[1], Some(100)).is_err());
    }

    #[test]
    fn test_verify_vdf_valid() {
        let engine = LogicalVdfClock::new();
        let proof = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        assert!(engine.verify_vdf(&proof));
    }

    #[test]
    fn test_verify_vdf_invalid_iterations() {
        let engine = LogicalVdfClock::new();
        let mut proof = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        proof.iterations = 5; // below min
        assert!(!engine.verify_vdf(&proof));
    }

    #[test]
    fn test_verify_vdf_tampered_output() {
        let engine = LogicalVdfClock::new();
        let mut proof = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        proof.output = 0xDEAD; // tampered
        assert!(!engine.verify_vdf(&proof));
    }

    #[test]
    fn test_logical_timestamp_advances() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        assert_eq!(engine.get_logical_timestamp(1), Some(0));
        engine.generate_vdf(1, &[1], Some(100)).unwrap();
        assert!(engine.get_logical_timestamp(1).unwrap() > 0);
    }

    #[test]
    fn test_synchronization_same_timestamp() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.generate_vdf(1, &[1], Some(100)).unwrap();
        engine.generate_vdf(2, &[1], Some(100)).unwrap();
        assert!(engine.are_synchronized(1, 2));
    }

    #[test]
    fn test_synchronization_different_timestamp() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.generate_vdf(1, &[1], Some(100)).unwrap();
        engine.generate_vdf(2, &[1], Some(5000)).unwrap();
        assert!(!engine.are_synchronized(1, 2));
    }

    #[test]
    fn test_synchronization_unregistered_node() {
        let engine = LogicalVdfClock::new();
        assert!(!engine.are_synchronized(1, 2));
    }

    #[test]
    fn test_hash_bytes_deterministic() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = LogicalVdfClock::hash_bytes(&data);
        let h2 = LogicalVdfClock::hash_bytes(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_bytes_different_input() {
        let h1 = LogicalVdfClock::hash_bytes(&[1, 2, 3]);
        let h2 = LogicalVdfClock::hash_bytes(&[1, 2, 4]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_vdf_proof_deterministic() {
        let p1 = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        let p2 = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        assert_eq!(p1.output, p2.output);
    }

    #[test]
    fn test_vdf_checkpoint() {
        let proof = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        assert!(proof.checkpoint > 0);
    }

    #[test]
    fn test_reset() {
        let mut engine = LogicalVdfClock::new();
        engine.register_node(1);
        engine.reset();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_display() {
        let engine = LogicalVdfClock::new();
        let s = format!("{}", engine);
        assert!(s.contains("LogicalVdfClock"));
    }

    #[test]
    fn test_proof_display() {
        let proof = LogicalVdfClock::generate_vdf_proof(0xABC, 100);
        let s = format!("{}", proof);
        assert!(s.contains("VDFProof"));
    }

    #[test]
    fn test_clock_display() {
        let clock = LogicalClockState::new(1);
        let s = format!("{}", clock);
        assert!(s.contains("LogicalClock"));
    }

    #[test]
    fn test_standalone_generate() {
        let proof = generate_vdf_proof(&[1, 2, 3], 100);
        assert!(proof.output > 0);
        assert_eq!(proof.iterations, 100);
    }

    #[test]
    fn test_standalone_verify() {
        let proof = generate_vdf_proof(&[1, 2, 3], 100);
        assert!(verify_vdf_proof(&proof, &[0xAB]));
    }

    #[test]
    fn test_standalone_verify_tampered() {
        let mut proof = generate_vdf_proof(&[1, 2, 3], 100);
        proof.output = 0;
        assert!(!verify_vdf_proof(&proof, &[0xAB]));
    }

    #[test]
    fn test_error_display() {
        let err = VdfError::EmptySeed;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = LogicalVdfClock::new();

        // Register nodes
        engine.register_node(1);
        engine.register_node(2);

        // Generate VDFs
        let p1 = engine.generate_vdf(1, &[1, 2, 3], Some(100)).unwrap();
        let p2 = engine.generate_vdf(2, &[4, 5, 6], Some(100)).unwrap();

        // Verify proofs
        assert!(engine.verify_vdf(&p1));
        assert!(engine.verify_vdf(&p2));

        // Check synchronization
        assert!(engine.are_synchronized(1, 2));

        // Different iterations → different timestamps
        engine.generate_vdf(2, &[7, 8, 9], Some(5000)).unwrap();
        assert!(!engine.are_synchronized(1, 2));
    }
}
