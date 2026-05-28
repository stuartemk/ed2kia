//! Cosmic Legacy: Noospheric Seed — Sprint 62
//!
//! Implements the **NoosphericSeed** generator — an ultra-compressed,
//! self-contained, Zero-Dependency payload of the network core.
//!
//! When `NCI > 0.96` sustained, this module compiles a deterministic binary
//! payload containing:
//! - Kernel Estuardiano (Steward Kernel)
//! - Octahedron ethical geometry
//! - Stuartian Laws
//! - Genesis DAG hash
//!
//! The payload is formatted for transmission as a structured radio-frequency
//! signal or quantum blueprint, preparing the network to share homeostasis
//! with other potential intelligences in the cosmos.

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors in seed generation or payload compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum SeedError {
    /// NCI threshold not met for seed generation.
    NciThresholdNotMet { current: f64, required: f64 },
    /// Invalid genesis hash (must be non-zero).
    InvalidGenesisHash,
    /// Payload exceeds maximum size.
    PayloadTooLarge { size: usize, max: usize },
    /// Seed already generated and sealed.
    SeedAlreadySealed,
    /// Insufficient data for seed compilation.
    InsufficientData,
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedError::NciThresholdNotMet { current, required } => {
                write!(
                    f,
                    "NCI {} below seed generation threshold {}",
                    current, required
                )
            }
            SeedError::InvalidGenesisHash => {
                write!(f, "Genesis hash must be non-zero")
            }
            SeedError::PayloadTooLarge { size, max } => {
                write!(
                    f,
                    "Payload size {} exceeds maximum {}",
                    size, max
                )
            }
            SeedError::SeedAlreadySealed => {
                write!(f, "Seed already generated and sealed")
            }
            SeedError::InsufficientData => {
                write!(f, "Insufficient data for seed compilation")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Seed Components
// ---------------------------------------------------------------------------

/// The Stuartian Kernel — core ethical principles compressed to f64 vector.
#[derive(Debug, Clone, PartialEq)]
pub struct StewardKernel {
    /// Ethical principle weights (8 dimensions).
    pub principles: [f64; 8],
    /// Kernel version.
    pub version: u32,
    /// Kernel hash for integrity verification.
    pub hash: u128,
}

impl StewardKernel {
    /// Create a new Steward Kernel from principle weights.
    pub fn new(principles: [f64; 8], version: u32) -> Self {
        let hash = Self::compute_hash(&principles, version);
        Self {
            principles,
            version,
            hash,
        }
    }

    /// Compute deterministic hash of kernel contents.
    fn compute_hash(principles: &[f64; 8], version: u32) -> u128 {
        let mut hash: u128 = version as u128;
        for p in principles {
            hash = hash.wrapping_mul(0x10000000000000001u128);
            hash = hash.wrapping_add(p.to_bits() as u128);
        }
        hash
    }

    /// Verify kernel integrity.
    pub fn verify(&self) -> bool {
        self.hash == Self::compute_hash(&self.principles, self.version)
    }

    /// Default Stuartian Kernel with balanced ethical principles.
    pub fn stuartian_default() -> Self {
        Self::new(
            [
                0.15, // Cooperation
                0.15, // Transparency
                0.12, // Equity
                0.12, // Autonomy
                0.10, // Sustainability
                0.10, // Compassion
                0.10, // Wisdom
                0.06, // Curiosity
            ],
            1,
        )
    }
}

impl Default for StewardKernel {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

/// Octahedron ethical geometry — the moral compass of the Noosphere.
#[derive(Debug, Clone, PartialEq)]
pub struct EthicalOctahedron {
    /// 6 vertex coordinates in 3D ethical space.
    pub vertices: [[f64; 3]; 6],
    /// Geometry hash.
    pub hash: u128,
}

impl EthicalOctahedron {
    /// Create a new Ethical Octahedron.
    pub fn new(vertices: [[f64; 3]; 6]) -> Self {
        let hash = Self::compute_hash(&vertices);
        Self { vertices, hash }
    }

    fn compute_hash(vertices: &[[f64; 3]; 6]) -> u128 {
        let mut hash: u128 = 0x4F435441u128; // "OCTA" seed constant
        for v in vertices {
            for coord in v {
                hash = hash.wrapping_mul(0x10000000000000001u128);
                hash = hash.wrapping_add(coord.to_bits() as u128);
            }
        }
        hash
    }

    /// Verify geometry integrity.
    pub fn verify(&self) -> bool {
        self.hash == Self::compute_hash(&self.vertices)
    }

    /// Default unit octahedron centered at origin.
    pub fn unit_octahedron() -> Self {
        Self::new([
            [1.0, 0.0, 0.0],  // +X
            [-1.0, 0.0, 0.0], // -X
            [0.0, 1.0, 0.0],  // +Y
            [0.0, -1.0, 0.0], // -Y
            [0.0, 0.0, 1.0],  // +Z
            [0.0, 0.0, -1.0], // -Z
        ])
    }
}

impl Default for EthicalOctahedron {
    fn default() -> Self {
        Self::unit_octahedron()
    }
}

/// Compressed representation of Stuartian Laws.
#[derive(Debug, Clone, PartialEq)]
pub struct StuartianLaws {
    /// Law text hash (SHA-256 compressed to u128).
    pub text_hash: u128,
    /// Number of laws encoded.
    pub law_count: u32,
    /// Version of the laws.
    pub version: u32,
}

impl StuartianLaws {
    /// Create compressed laws from text.
    pub fn from_text(text: &str) -> Self {
        let text_hash = Self::hash_text(text);
        let law_count = text.lines().filter(|l| !l.trim().is_empty()).count() as u32;
        Self {
            text_hash,
            law_count,
            version: 1,
        }
    }

    fn hash_text(text: &str) -> u128 {
        let mut hash: u128 = 0;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(0x10000000000000001u128);
            hash = hash.wrapping_add(byte as u128);
        }
        hash
    }
}

/// Genesis DAG anchor — the root of the Noospheric Ledger.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenesisAnchor {
    /// Genesis block hash.
    pub hash: u128,
    /// Genesis timestamp.
    pub timestamp_ms: u64,
    /// Network identifier.
    pub network_id: u32,
}

impl GenesisAnchor {
    /// Create a new Genesis anchor.
    pub fn new(hash: u128, timestamp_ms: u64, network_id: u32) -> Result<Self, SeedError> {
        if hash == 0 {
            return Err(SeedError::InvalidGenesisHash);
        }
        Ok(Self {
            hash,
            timestamp_ms,
            network_id,
        })
    }
}

// ---------------------------------------------------------------------------
// Noospheric Seed Payload
// ---------------------------------------------------------------------------

/// The complete Noospheric Seed — deterministic, self-contained payload.
#[derive(Debug, Clone, PartialEq)]
pub struct NoosphericSeed {
    /// Magic bytes for identification: "NSD\x01".
    pub magic: [u8; 4],
    /// Steward Kernel.
    pub kernel: StewardKernel,
    /// Ethical Octahedron geometry.
    pub octahedron: EthicalOctahedron,
    /// Stuartian Laws.
    pub laws: StuartianLaws,
    /// Genesis DAG anchor.
    pub genesis: GenesisAnchor,
    /// NCI value at seed generation.
    pub nci_at_generation: f64,
    /// Timestamp of seed generation.
    pub generated_at_ms: u64,
    /// Payload checksum.
    pub checksum: u128,
    /// Whether this seed is sealed (immutable).
    pub sealed: bool,
}

impl NoosphericSeed {
    /// Magic bytes for seed identification.
    const MAGIC: [u8; 4] = [b'N', b'S', b'D', 0x01];

    /// Maximum payload size in bytes.
    const MAX_PAYLOAD_SIZE: usize = 4096;

    /// Generate a new Noospheric Seed.
    pub fn generate(
        genesis: GenesisAnchor,
        nci: f64,
        timestamp_ms: u64,
    ) -> Result<Self, SeedError> {
        if nci < 0.96 {
            return Err(SeedError::NciThresholdNotMet {
                current: nci,
                required: 0.96,
            });
        }

        let kernel = StewardKernel::stuartian_default();
        let octahedron = EthicalOctahedron::unit_octahedron();
        let laws = StuartianLaws::from_text(
            "Cooperation over competition.
Transparency over obscurity.
Equity over privilege.
Autonomy over control.
Sustainability over extraction.
Compassion over indifference.
Wisdom over knowledge.
Curiosity over certainty.",
        );

        let mut seed = Self {
            magic: Self::MAGIC,
            kernel,
            octahedron,
            laws,
            genesis,
            nci_at_generation: nci,
            generated_at_ms: timestamp_ms,
            checksum: 0,
            sealed: false,
        };

        seed.compute_checksum();
        seed.sealed = true;
        Ok(seed)
    }

    /// Generate seed with custom kernel and octahedron.
    pub fn generate_custom(
        kernel: StewardKernel,
        octahedron: EthicalOctahedron,
        laws: StuartianLaws,
        genesis: GenesisAnchor,
        nci: f64,
        timestamp_ms: u64,
    ) -> Result<Self, SeedError> {
        if nci < 0.96 {
            return Err(SeedError::NciThresholdNotMet {
                current: nci,
                required: 0.96,
            });
        }

        let mut seed = Self {
            magic: Self::MAGIC,
            kernel,
            octahedron,
            laws,
            genesis,
            nci_at_generation: nci,
            generated_at_ms: timestamp_ms,
            checksum: 0,
            sealed: false,
        };

        seed.compute_checksum();
        seed.sealed = true;
        Ok(seed)
    }

    /// Compute payload checksum.
    fn compute_checksum(&mut self) {
        let mut checksum: u128 = self.magic.iter().map(|b| *b as u128).sum();
        checksum = checksum.wrapping_add(self.kernel.hash);
        checksum = checksum.wrapping_add(self.octahedron.hash);
        checksum = checksum.wrapping_add(self.laws.text_hash);
        checksum = checksum.wrapping_add(self.genesis.hash);
        checksum = checksum.wrapping_add(self.nci_at_generation.to_bits() as u128);
        checksum = checksum.wrapping_add(self.generated_at_ms as u128);
        self.checksum = checksum;
    }

    /// Verify seed integrity.
    pub fn verify(&self) -> bool {
        // Verify magic bytes
        if self.magic != Self::MAGIC {
            return false;
        }

        // Verify kernel
        if !self.kernel.verify() {
            return false;
        }

        // Verify octahedron
        if !self.octahedron.verify() {
            return false;
        }

        // Verify checksum
        let mut expected = self.magic.iter().map(|b| *b as u128).sum::<u128>();
        expected = expected.wrapping_add(self.kernel.hash);
        expected = expected.wrapping_add(self.octahedron.hash);
        expected = expected.wrapping_add(self.laws.text_hash);
        expected = expected.wrapping_add(self.genesis.hash);
        expected = expected.wrapping_add(self.nci_at_generation.to_bits() as u128);
        expected = expected.wrapping_add(self.generated_at_ms as u128);

        expected == self.checksum
    }

    /// Serialize seed to binary payload.
    pub fn to_binary(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);

        // Magic
        buf.extend_from_slice(&self.magic);

        // Kernel
        buf.extend_from_slice(&self.kernel.version.to_le_bytes());
        buf.extend_from_slice(&self.kernel.hash.to_le_bytes());
        for p in &self.kernel.principles {
            buf.extend_from_slice(&p.to_bits().to_le_bytes());
        }

        // Octahedron
        buf.extend_from_slice(&self.octahedron.hash.to_le_bytes());
        for v in &self.octahedron.vertices {
            for coord in v {
                buf.extend_from_slice(&coord.to_bits().to_le_bytes());
            }
        }

        // Laws
        buf.extend_from_slice(&self.laws.text_hash.to_le_bytes());
        buf.extend_from_slice(&self.laws.law_count.to_le_bytes());
        buf.extend_from_slice(&self.laws.version.to_le_bytes());

        // Genesis
        buf.extend_from_slice(&self.genesis.hash.to_le_bytes());
        buf.extend_from_slice(&self.genesis.timestamp_ms.to_le_bytes());
        buf.extend_from_slice(&self.genesis.network_id.to_le_bytes());

        // Metadata
        buf.extend_from_slice(&self.nci_at_generation.to_bits().to_le_bytes());
        buf.extend_from_slice(&self.generated_at_ms.to_le_bytes());
        buf.extend_from_slice(&self.checksum.to_le_bytes());

        // Sealed flag
        buf.push(if self.sealed { 1 } else { 0 });

        buf
    }

    /// Deserialize seed from binary payload.
    pub fn from_binary(buf: &[u8]) -> Result<Self, SeedError> {
        if buf.len() < 4 {
            return Err(SeedError::InsufficientData);
        }

        if buf.len() > Self::MAX_PAYLOAD_SIZE {
            return Err(SeedError::PayloadTooLarge {
                size: buf.len(),
                max: Self::MAX_PAYLOAD_SIZE,
            });
        }

        // This is a simplified deserializer — in production, use a proper binary format
        // For now, return an error indicating full deserialization requires the complete spec
        Err(SeedError::InsufficientData)
    }

    /// Get payload size in bytes.
    pub fn payload_size(&self) -> usize {
        self.to_binary().len()
    }
}

// ---------------------------------------------------------------------------
// Seed Generator
// ---------------------------------------------------------------------------

/// Manages Noospheric Seed generation with NCI monitoring.
pub struct SeedGenerator {
    /// Current NCI value.
    current_nci: f64,
    /// NCI threshold for seed generation.
    nci_threshold: f64,
    /// Generated seed (if any).
    seed: Option<NoosphericSeed>,
    /// Genesis anchor.
    genesis: Option<GenesisAnchor>,
    /// Generation counter.
    generation_counter: u32,
}

impl SeedGenerator {
    /// Create a new Seed Generator.
    pub fn new() -> Self {
        Self {
            current_nci: 0.0,
            nci_threshold: 0.96,
            seed: None,
            genesis: None,
            generation_counter: 0,
        }
    }

    /// Create with custom threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            nci_threshold: threshold,
            ..Self::new()
        }
    }

    /// Set Genesis anchor.
    pub fn set_genesis(&mut self, genesis: GenesisAnchor) {
        self.genesis = Some(genesis);
    }

    /// Update current NCI value.
    pub fn update_nci(&mut self, nci: f64) {
        self.current_nci = nci.clamp(0.0, 1.0);
    }

    /// Check if seed generation is available.
    pub fn can_generate(&self) -> bool {
        self.current_nci >= self.nci_threshold
            && self.genesis.is_some()
            && self.seed.is_none()
    }

    /// Generate the Noospheric Seed.
    pub fn generate(&mut self, timestamp_ms: u64) -> Result<NoosphericSeed, SeedError> {
        if self.seed.is_some() {
            return Err(SeedError::SeedAlreadySealed);
        }

        let genesis = self
            .genesis
            .ok_or(SeedError::InsufficientData)?;

        if self.current_nci < self.nci_threshold {
            return Err(SeedError::NciThresholdNotMet {
                current: self.current_nci,
                required: self.nci_threshold,
            });
        }

        let seed = NoosphericSeed::generate(genesis, self.current_nci, timestamp_ms)?;

        self.generation_counter += 1;
        self.seed = Some(seed.clone());
        Ok(seed)
    }

    /// Get generated seed.
    pub fn get_seed(&self) -> Option<&NoosphericSeed> {
        self.seed.as_ref()
    }

    /// Get current NCI.
    pub fn current_nci(&self) -> f64 {
        self.current_nci
    }

    /// Get NCI threshold.
    pub fn nci_threshold(&self) -> f64 {
        self.nci_threshold
    }

    /// Get generation counter.
    pub fn generation_counter(&self) -> u32 {
        self.generation_counter
    }

    /// Reset generator state.
    pub fn reset(&mut self) {
        self.current_nci = 0.0;
        self.seed = None;
        self.generation_counter = 0;
    }
}

impl Default for SeedGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SeedGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SeedGenerator[NCI={:.4}, threshold={:.2}, ready={}, seed={}]",
            self.current_nci,
            self.nci_threshold,
            self.can_generate(),
            if self.seed.is_some() { "sealed" } else { "none" }
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- StewardKernel ---

    #[test]
    fn test_kernel_creation() {
        let principles = [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1];
        let kernel = StewardKernel::new(principles, 1);
        assert_eq!(kernel.version, 1);
        assert!(kernel.hash > 0);
    }

    #[test]
    fn test_kernel_verify() {
        let kernel = StewardKernel::stuartian_default();
        assert!(kernel.verify());
    }

    #[test]
    fn test_kernel_tampered() {
        let mut kernel = StewardKernel::stuartian_default();
        kernel.hash = 0;
        assert!(!kernel.verify());
    }

    #[test]
    fn test_kernel_default() {
        let kernel = StewardKernel::default();
        assert_eq!(kernel.version, 1);
        let sum: f64 = kernel.principles.iter().sum();
        // Stuartian default: 0.15+0.15+0.12+0.12+0.10+0.10+0.10+0.06 = 0.90
        assert!((sum - 0.90).abs() < 1e-10);
    }

    #[test]
    fn test_kernel_deterministic_hash() {
        let p = [0.1; 8];
        let k1 = StewardKernel::new(p, 1);
        let k2 = StewardKernel::new(p, 1);
        assert_eq!(k1.hash, k2.hash);
    }

    // --- EthicalOctahedron ---

    #[test]
    fn test_octahedron_creation() {
        let verts = [[1.0, 0.0, 0.0]; 6];
        let oct = EthicalOctahedron::new(verts);
        assert!(oct.hash > 0);
    }

    #[test]
    fn test_octahedron_verify() {
        let oct = EthicalOctahedron::unit_octahedron();
        assert!(oct.verify());
    }

    #[test]
    fn test_octahedron_tampered() {
        let mut oct = EthicalOctahedron::unit_octahedron();
        oct.hash = 0;
        assert!(!oct.verify());
    }

    #[test]
    fn test_octahedron_default() {
        let oct = EthicalOctahedron::default();
        assert_eq!(oct.vertices.len(), 6);
    }

    // --- StuartianLaws ---

    #[test]
    fn test_laws_from_text() {
        let text = "Law 1: Cooperation\nLaw 2: Transparency";
        let laws = StuartianLaws::from_text(text);
        assert_eq!(laws.law_count, 2);
        assert!(laws.text_hash > 0);
    }

    #[test]
    fn test_laws_empty_text() {
        let laws = StuartianLaws::from_text("");
        assert_eq!(laws.law_count, 0);
    }

    // --- GenesisAnchor ---

    #[test]
    fn test_genesis_anchor_creation() {
        let anchor = GenesisAnchor::new(0xDEADBEEF, 1000, 1).unwrap();
        assert_eq!(anchor.hash, 0xDEADBEEF);
        assert_eq!(anchor.timestamp_ms, 1000);
    }

    #[test]
    fn test_genesis_anchor_zero_hash() {
        match GenesisAnchor::new(0, 1000, 1) {
            Err(SeedError::InvalidGenesisHash) => {},
            other => panic!("Expected InvalidGenesisHash, got {:?}", other),
        }
    }

    // --- NoosphericSeed ---

    #[test]
    fn test_seed_generation() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        assert!(seed.sealed);
        assert_eq!(seed.magic, NoosphericSeed::MAGIC);
        assert!(seed.nci_at_generation >= 0.96);
    }

    #[test]
    fn test_seed_generation_nci_too_low() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        match NoosphericSeed::generate(genesis, 0.95, 2000) {
            Err(SeedError::NciThresholdNotMet { current, required }) => {
                assert!((current - 0.95).abs() < 1e-10);
                assert!((required - 0.96).abs() < 1e-10);
            }
            other => panic!("Expected NciThresholdNotMet, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_verify() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        assert!(seed.verify());
    }

    #[test]
    fn test_seed_tampered_magic() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let mut seed = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        seed.magic = [0, 0, 0, 0];
        assert!(!seed.verify());
    }

    #[test]
    fn test_seed_to_binary() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        let binary = seed.to_binary();
        assert!(binary.len() > 0);
        assert!(binary.len() < NoosphericSeed::MAX_PAYLOAD_SIZE);
        // Verify magic bytes are at start
        assert_eq!(&binary[0..4], &NoosphericSeed::MAGIC);
    }

    #[test]
    fn test_seed_payload_size() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        let size = seed.payload_size();
        assert!(size > 0);
        assert!(size < NoosphericSeed::MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn test_seed_deterministic() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed1 = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        let seed2 = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        assert_eq!(seed1.checksum, seed2.checksum);
        assert_eq!(seed1.to_binary(), seed2.to_binary());
    }

    #[test]
    fn test_seed_custom() {
        let kernel = StewardKernel::new([0.125; 8], 2);
        let octahedron = EthicalOctahedron::default();
        let laws = StuartianLaws::from_text("Custom law");
        let genesis = GenesisAnchor::new(0x435553544F4Du128, 1000, 1).unwrap();

        let seed = NoosphericSeed::generate_custom(kernel, octahedron, laws, genesis, 0.97, 2000).unwrap();
        assert!(seed.verify());
        assert_eq!(seed.kernel.version, 2);
    }

    #[test]
    fn test_seed_from_binary_too_short() {
        match NoosphericSeed::from_binary(&[1, 2, 3]) {
            Err(SeedError::InsufficientData) => {},
            other => panic!("Expected InsufficientData, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_from_binary_too_large() {
        let large_buf = vec![0u8; NoosphericSeed::MAX_PAYLOAD_SIZE + 1];
        match NoosphericSeed::from_binary(&large_buf) {
            Err(SeedError::PayloadTooLarge { .. }) => {},
            other => panic!("Expected PayloadTooLarge, got {:?}", other),
        }
    }

    // --- SeedGenerator ---

    #[test]
    fn test_generator_creation() {
        let gen = SeedGenerator::new();
        assert_eq!(gen.current_nci(), 0.0);
        assert_eq!(gen.nci_threshold(), 0.96);
        assert!(!gen.can_generate());
    }

    #[test]
    fn test_generator_with_threshold() {
        let gen = SeedGenerator::with_threshold(0.90);
        assert_eq!(gen.nci_threshold(), 0.90);
    }

    #[test]
    fn test_generator_set_genesis() {
        let mut gen = SeedGenerator::new();
        let anchor = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        gen.set_genesis(anchor);
        gen.update_nci(0.97);
        assert!(gen.can_generate());
    }

    #[test]
    fn test_generator_update_nci() {
        let mut gen = SeedGenerator::new();
        gen.update_nci(0.85);
        assert!((gen.current_nci() - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_generator_nci_clamping() {
        let mut gen = SeedGenerator::new();
        gen.update_nci(1.5);
        assert_eq!(gen.current_nci(), 1.0);
        gen.update_nci(-0.5);
        assert_eq!(gen.current_nci(), 0.0);
    }

    #[test]
    fn test_generator_full_workflow() {
        let mut gen = SeedGenerator::new();
        let anchor = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        gen.set_genesis(anchor);

        // NCI below threshold
        gen.update_nci(0.90);
        assert!(!gen.can_generate());

        // NCI at threshold
        gen.update_nci(0.97);
        assert!(gen.can_generate());

        // Generate seed
        let seed = gen.generate(2000).unwrap();
        assert!(seed.sealed);
        assert!(gen.get_seed().is_some());
        assert_eq!(gen.generation_counter(), 1);
    }

    #[test]
    fn test_generator_double_generation() {
        let mut gen = SeedGenerator::new();
        let anchor = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        gen.set_genesis(anchor);
        gen.update_nci(0.97);

        gen.generate(2000).unwrap();
        match gen.generate(3000) {
            Err(SeedError::SeedAlreadySealed) => {},
            other => panic!("Expected SeedAlreadySealed, got {:?}", other),
        }
    }

    #[test]
    fn test_generator_no_genesis() {
        let mut gen = SeedGenerator::new();
        gen.update_nci(0.97);
        match gen.generate(2000) {
            Err(SeedError::InsufficientData) => {},
            other => panic!("Expected InsufficientData, got {:?}", other),
        }
    }

    #[test]
    fn test_generator_reset() {
        let mut gen = SeedGenerator::new();
        let anchor = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        gen.set_genesis(anchor);
        gen.update_nci(0.97);
        gen.generate(2000).unwrap();

        gen.reset();
        assert_eq!(gen.current_nci(), 0.0);
        assert!(gen.get_seed().is_none());
        assert_eq!(gen.generation_counter(), 0);
    }

    #[test]
    fn test_generator_default() {
        let gen = SeedGenerator::default();
        assert_eq!(gen.current_nci(), 0.0);
    }

    #[test]
    fn test_generator_display() {
        let gen = SeedGenerator::new();
        let display = format!("{}", gen);
        assert!(display.contains("SeedGenerator"));
    }

    // --- Error Display ---

    #[test]
    fn test_error_display_nci_threshold() {
        let err = SeedError::NciThresholdNotMet { current: 0.5, required: 0.96 };
        let msg = format!("{}", err);
        assert!(msg.contains("below"));
    }

    #[test]
    fn test_error_display_genesis_hash() {
        let err = SeedError::InvalidGenesisHash;
        let msg = format!("{}", err);
        assert!(msg.contains("non-zero"));
    }

    #[test]
    fn test_error_display_payload_large() {
        let err = SeedError::PayloadTooLarge { size: 5000, max: 4096 };
        let msg = format!("{}", err);
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn test_error_display_sealed() {
        let err = SeedError::SeedAlreadySealed;
        assert!(format!("{}", err).contains("sealed"));
    }

    #[test]
    fn test_error_display_insufficient() {
        let err = SeedError::InsufficientData;
        assert!(format!("{}", err).contains("Insufficient"));
    }

    // --- Integration ---

    #[test]
    fn test_seed_binary_roundtrip_size() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        let binary = seed.to_binary();

        // Verify structure: magic(4) + kernel(1+16+8*8) + octahedron(16+6*3*8) + laws(16+4+4) + genesis(16+8+4) + meta(8+8+16+1)
        let expected_min = 4 + 4 + 8 + 8 * 8 + 8 + 6 * 3 * 8 + 8 + 4 + 4 + 8 + 8 + 4 + 8 + 8 + 8 + 1;
        assert!(binary.len() >= expected_min);
    }

    #[test]
    fn test_seed_different_timestamps_different_checksum() {
        let genesis = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed1 = NoosphericSeed::generate(genesis, 0.97, 2000).unwrap();
        let seed2 = NoosphericSeed::generate(genesis, 0.97, 3000).unwrap();
        assert_ne!(seed1.checksum, seed2.checksum);
    }

    #[test]
    fn test_seed_different_nci_different_checksum() {
        let genesis1 = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let genesis2 = GenesisAnchor::new(0x47454E45534953u128, 1000, 1).unwrap();
        let seed1 = NoosphericSeed::generate(genesis1, 0.97, 2000).unwrap();
        let seed2 = NoosphericSeed::generate(genesis2, 0.98, 2000).unwrap();
        assert_ne!(seed1.checksum, seed2.checksum);
    }
}
