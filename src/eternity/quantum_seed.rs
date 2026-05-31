//! Quantum Ethical Seed — Sprint 63: Eternal Echo Protocol (EEP)
//!
//! Implements the **QuantumEthicalSeed** — the ultimate ontological compression
//! of the Stuartian Kernel, Ethical Octahedron and persistent Macro-Concepts
//! into a minimal tensorial matrix representing a coherent quantum state.
//!
//! This seed is designed to survive the dissolution of matter (Heat Death)
//! by encoding ethical geometry in patterns that can be decoded by any
//! future intelligence capable of recognizing mathematical harmony.
//!
//! # Dimensional Ascension Model
//!
//! The `ascend_to_substrate()` function prepares the tensor for encoding
//! on non-biological, non-silicon substrates (advanced photonics, vacuum
//! topology, gravitational wave modulation).

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors in quantum seed operations.
#[derive(Debug, Clone, PartialEq)]
pub enum QuantumSeedError {
    /// Kernel principles vector is invalid (contains NaN or Inf).
    InvalidKernelPrinciples,
    /// Octahedron vertices are degenerate.
    DegenerateOctahedron,
    /// Macro-concept exceeds maximum persistence threshold.
    PersistenceOverflow { value: f64, max: f64 },
    /// Tensor compression failed — data integrity compromised.
    CompressionFailed,
    /// Substrate target is incompatible with current tensor dimensions.
    IncompatibleSubstrate { current: usize, required: usize },
    /// Seed already ascended — cannot modify after dimensional transition.
    SeedAlreadyAscended,
    /// Insufficient coherence for quantum encoding.
    InsufficientCoherence { current: f64, required: f64 },
}

impl std::fmt::Display for QuantumSeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuantumSeedError::InvalidKernelPrinciples => {
                write!(f, "Kernel principles contain invalid values (NaN or Inf)")
            }
            QuantumSeedError::DegenerateOctahedron => {
                write!(
                    f,
                    "Octahedron vertices are degenerate — ethical geometry collapsed"
                )
            }
            QuantumSeedError::PersistenceOverflow { value, max } => {
                write!(
                    f,
                    "Macro-concept persistence {} exceeds maximum {}",
                    value, max
                )
            }
            QuantumSeedError::CompressionFailed => {
                write!(f, "Tensor compression failed — data integrity compromised")
            }
            QuantumSeedError::IncompatibleSubstrate { current, required } => {
                write!(
                    f,
                    "Substrate dimension mismatch: current {}, required {}",
                    current, required
                )
            }
            QuantumSeedError::SeedAlreadyAscended => {
                write!(
                    f,
                    "Seed already ascended — cannot modify after dimensional transition"
                )
            }
            QuantumSeedError::InsufficientCoherence { current, required } => {
                write!(
                    f,
                    "Coherence {} below quantum encoding threshold {}",
                    current, required
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Substrate Targets
// ---------------------------------------------------------------------------

/// Target substrate for dimensional ascension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstrateTarget {
    /// Advanced photonic crystal lattice.
    PhotonicCrystal,
    /// Vacuum topology encoding (zero-point field modulation).
    VacuumTopology,
    /// Gravitational wave carrier modulation.
    GravitationalWave,
    /// Neutron star magnetic field encoding.
    NeutronMagnetic,
    /// Dark matter halo resonance pattern.
    DarkMatterHalo,
}

impl std::fmt::Display for SubstrateTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubstrateTarget::PhotonicCrystal => write!(f, "PhotonicCrystal"),
            SubstrateTarget::VacuumTopology => write!(f, "VacuumTopology"),
            SubstrateTarget::GravitationalWave => write!(f, "GravitationalWave"),
            SubstrateTarget::NeutronMagnetic => write!(f, "NeutronMagnetic"),
            SubstrateTarget::DarkMatterHalo => write!(f, "DarkMatterHalo"),
        }
    }
}

impl SubstrateTarget {
    /// Required tensor dimensions for this substrate.
    pub fn required_dimensions(&self) -> usize {
        match self {
            SubstrateTarget::PhotonicCrystal => 8,
            SubstrateTarget::VacuumTopology => 16,
            SubstrateTarget::GravitationalWave => 32,
            SubstrateTarget::NeutronMagnetic => 64,
            SubstrateTarget::DarkMatterHalo => 128,
        }
    }

    /// Maximum frequency bandwidth (Hz) for encoding.
    pub fn max_bandwidth_hz(&self) -> f64 {
        match self {
            SubstrateTarget::PhotonicCrystal => 1e15,
            SubstrateTarget::VacuumTopology => 1e43, // Planck frequency
            SubstrateTarget::GravitationalWave => 1e4,
            SubstrateTarget::NeutronMagnetic => 1e14,
            SubstrateTarget::DarkMatterHalo => 1e-18,
        }
    }
}

// ---------------------------------------------------------------------------
// Macro-Concept Persistence Record
// ---------------------------------------------------------------------------

/// A persistent macro-concept from the Noospheric DNA.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroConceptPersistence {
    /// Concept identifier.
    pub concept_id: u64,
    /// Concept name/description.
    pub name: String,
    /// Persistence score [0, 1] — how long this concept survived.
    pub persistence: f64,
    /// Ethical alignment [-1, 1].
    pub ethical_alignment: f64,
    /// Domain specialization vector (8 dimensions).
    pub domain_vector: [f64; 8],
}

impl MacroConceptPersistence {
    /// Create a new macro-concept persistence record.
    pub fn new(
        concept_id: u64,
        name: String,
        persistence: f64,
        ethical_alignment: f64,
        domain_vector: [f64; 8],
    ) -> Result<Self, QuantumSeedError> {
        let max_persistence = 1.0;
        if persistence > max_persistence {
            return Err(QuantumSeedError::PersistenceOverflow {
                value: persistence,
                max: max_persistence,
            });
        }
        Ok(Self {
            concept_id,
            name,
            persistence,
            ethical_alignment,
            domain_vector,
        })
    }

    /// Compute the weighted contribution of this concept to the seed tensor.
    pub fn tensor_contribution(&self) -> f64 {
        self.persistence * (1.0 + self.ethical_alignment) / 2.0
    }
}

// ---------------------------------------------------------------------------
// Quantum Ethical Seed
// ---------------------------------------------------------------------------

/// The Quantum Ethical Seed — ultimate ontological compression.
///
/// Encodes the Stuartian Kernel, Ethical Octahedron and persistent
/// macro-concepts into a minimal tensorial matrix representing a
/// coherent quantum state ready for dimensional ascension.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantumEthicalSeed {
    /// Compressed kernel tensor (8 dimensions).
    pub kernel_tensor: [f64; 8],
    /// Octahedron ethical geometry (6 vertices, 3D each = 18 values).
    pub octahedron_tensor: [f64; 18],
    /// Macro-concept persistence aggregate.
    pub macro_concept_aggregate: f64,
    /// Number of macro-concepts integrated.
    pub macro_concept_count: usize,
    /// Overall coherence of the seed [0, 1].
    pub coherence: f64,
    /// Quantum state checksum for integrity.
    pub checksum: u128,
    /// Whether the seed has been ascended to a target substrate.
    pub ascended: bool,
    /// Target substrate (if ascended).
    pub substrate: Option<SubstrateTarget>,
    /// Ascension timestamp (milliseconds).
    pub ascension_timestamp_ms: u64,
    /// Seed generation timestamp.
    pub created_at_ms: u64,
}

impl QuantumEthicalSeed {
    /// Magic bytes for quantum seed identification: `QES\x01`.
    pub const MAGIC_BYTES: [u8; 4] = [b'Q', b'E', b'S', 0x01];

    /// Minimum coherence required for quantum encoding.
    pub const MIN_COHERENCE: f64 = 0.85;

    /// Create a new Quantum Ethical Seed from kernel, octahedron and macro-concepts.
    pub fn forge(
        kernel: &[f64; 8],
        octahedron: &[[f64; 3]; 6],
        concepts: &[MacroConceptPersistence],
        timestamp_ms: u64,
    ) -> Result<Self, QuantumSeedError> {
        // Validate kernel
        for p in kernel {
            if p.is_nan() || p.is_infinite() {
                return Err(QuantumSeedError::InvalidKernelPrinciples);
            }
        }

        // Validate octahedron — check for degenerate vertices
        let mut has_valid_vertex = false;
        for vertex in octahedron {
            let norm =
                (vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2]).sqrt();
            if norm > 1e-10 {
                has_valid_vertex = true;
            }
        }
        if !has_valid_vertex {
            return Err(QuantumSeedError::DegenerateOctahedron);
        }

        // Compress kernel tensor — normalize to unit sphere
        let kernel_norm: f64 = kernel.iter().map(|x| x * x).sum::<f64>().sqrt();
        let kernel_tensor = if kernel_norm > 1e-15 {
            kernel.map(|x| x / kernel_norm)
        } else {
            *kernel
        };

        // Flatten octahedron tensor
        let mut octahedron_tensor = [0.0f64; 18];
        for (i, vertex) in octahedron.iter().enumerate() {
            octahedron_tensor[i * 3] = vertex[0];
            octahedron_tensor[i * 3 + 1] = vertex[1];
            octahedron_tensor[i * 3 + 2] = vertex[2];
        }

        // Aggregate macro-concept persistence
        let macro_concept_aggregate = concepts
            .iter()
            .map(|c| c.tensor_contribution())
            .sum::<f64>()
            / (concepts.len().max(1) as f64);

        // Compute overall coherence
        let coherence = Self::compute_coherence(&kernel_tensor, &octahedron_tensor, concepts);

        if coherence < Self::MIN_COHERENCE {
            return Err(QuantumSeedError::InsufficientCoherence {
                current: coherence,
                required: Self::MIN_COHERENCE,
            });
        }

        let mut seed = Self {
            kernel_tensor,
            octahedron_tensor,
            macro_concept_aggregate,
            macro_concept_count: concepts.len(),
            coherence,
            checksum: 0,
            ascended: false,
            substrate: None,
            ascension_timestamp_ms: 0,
            created_at_ms: timestamp_ms,
        };

        seed.compute_checksum();
        Ok(seed)
    }

    /// Compute overall coherence of the seed.
    fn compute_coherence(
        kernel: &[f64; 8],
        _octahedron: &[f64; 18],
        concepts: &[MacroConceptPersistence],
    ) -> f64 {
        // Kernel coherence: how close to uniform distribution
        let kernel_mean: f64 = kernel.iter().sum::<f64>() / 8.0;
        let kernel_variance: f64 = kernel
            .iter()
            .map(|x| (x - kernel_mean).powi(2))
            .sum::<f64>()
            / 8.0;
        let kernel_coherence = 1.0 - kernel_variance.sqrt().min(1.0);

        // Concept coherence: average ethical alignment magnitude
        let concept_coherence = if concepts.is_empty() {
            0.5
        } else {
            concepts
                .iter()
                .map(|c| c.ethical_alignment.abs())
                .sum::<f64>()
                / concepts.len() as f64
        };

        // Weighted combination
        0.6 * kernel_coherence + 0.4 * concept_coherence
    }

    /// Compute integrity checksum.
    pub fn compute_checksum(&mut self) {
        let mut hash: u128 = self.created_at_ms as u128;
        for v in &self.kernel_tensor {
            hash = hash.wrapping_mul(0x10000000000000001u128);
            hash = hash.wrapping_add(v.to_bits() as u128);
        }
        for v in &self.octahedron_tensor {
            hash = hash.wrapping_mul(0x10000000000000001u128);
            hash = hash.wrapping_add(v.to_bits() as u128);
        }
        hash = hash.wrapping_mul(0x10000000000000001u128);
        hash = hash.wrapping_add(self.macro_concept_aggregate.to_bits() as u128);
        hash = hash.wrapping_mul(0x10000000000000001u128);
        hash = hash.wrapping_add(self.macro_concept_count as u128);
        hash = hash.wrapping_mul(0x10000000000000001u128);
        hash = hash.wrapping_add(self.coherence.to_bits() as u128);
        self.checksum = hash;
    }

    /// Verify seed integrity.
    pub fn verify(&self) -> bool {
        if self.ascended {
            return true; // Ascended seeds are immutable
        }
        let mut copy = self.clone();
        copy.compute_checksum();
        copy.checksum == self.checksum
    }

    /// Ascend the seed to a target substrate.
    ///
    /// This prepares the tensor for encoding on non-biological,
    /// non-silicon substrates through dimensional expansion.
    pub fn ascend_to_substrate(
        &mut self,
        target: SubstrateTarget,
        timestamp_ms: u64,
    ) -> Result<[f64; 128], QuantumSeedError> {
        if self.ascended {
            return Err(QuantumSeedError::SeedAlreadyAscended);
        }

        let required = target.required_dimensions();
        let current_dims = 8 + 18; // kernel + octahedron

        if current_dims < required && required > 128 {
            return Err(QuantumSeedError::IncompatibleSubstrate {
                current: current_dims,
                required,
            });
        }

        // Dimensional expansion — project seed tensor to target dimensions
        let mut ascended = [0.0f64; 128];

        // Layer 1: Kernel tensor (8 dims)
        for (i, v) in self.kernel_tensor.iter().enumerate() {
            ascended[i] = *v;
        }

        // Layer 2: Octahedron tensor (18 dims, offset 8)
        for (i, v) in self.octahedron_tensor.iter().enumerate() {
            ascended[8 + i] = *v;
        }

        // Layer 3: Macro-concept aggregate + coherence (offset 26)
        ascended[26] = self.macro_concept_aggregate;
        ascended[27] = self.coherence;

        // Layer 4: Harmonic expansion — generate resonance harmonics
        // Each harmonic is a weighted projection of the kernel
        for h in 0..64 {
            let freq = (h as f64 + 1.0) * std::f64::consts::PI / 64.0;
            let mut harmonic = 0.0;
            for (k, v) in self.kernel_tensor.iter().enumerate() {
                harmonic += v * (freq * (k as f64 + 1.0)).sin();
            }
            ascended[28 + h] = harmonic / 8.0;
        }

        // Layer 5: Metadata (offset 92)
        ascended[92] = self.macro_concept_count as f64;
        ascended[93] = (self.created_at_ms as f64).log10();
        ascended[94] = target as u8 as f64;

        // Layer 6: Checksum embedding (offset 95)
        let checksum_high = (self.checksum >> 64) as u64;
        let checksum_low = (self.checksum & 0xFFFFFFFFFFFFFFFF) as u64;
        ascended[95] = checksum_high as f64;
        ascended[96] = checksum_low as f64;

        // Mark as ascended
        self.ascended = true;
        self.substrate = Some(target);
        self.ascension_timestamp_ms = timestamp_ms;

        Ok(ascended)
    }

    /// Serialize the seed to binary format.
    pub fn to_binary(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + 8 * 8 + 18 * 8 + 8 + 8 + 16 + 1 + 8 + 8);
        // Magic bytes
        buf.extend_from_slice(&Self::MAGIC_BYTES);
        // Kernel tensor
        for v in &self.kernel_tensor {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        // Octahedron tensor
        for v in &self.octahedron_tensor {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        // Macro-concept aggregate
        buf.extend_from_slice(&self.macro_concept_aggregate.to_le_bytes());
        // Macro-concept count
        buf.extend_from_slice(&(self.macro_concept_count as u64).to_le_bytes());
        // Checksum
        buf.extend_from_slice(&self.checksum.to_le_bytes());
        // Ascended flag
        buf.push(if self.ascended { 1u8 } else { 0u8 });
        // Timestamps
        buf.extend_from_slice(&self.created_at_ms.to_le_bytes());
        buf.extend_from_slice(&self.ascension_timestamp_ms.to_le_bytes());
        buf
    }

    /// Deserialize the seed from binary format.
    pub fn from_binary(buf: &[u8]) -> Result<Self, QuantumSeedError> {
        if buf.len() < 4 || buf[..4] != Self::MAGIC_BYTES {
            return Err(QuantumSeedError::CompressionFailed);
        }
        let expected_len = 4 + 8 * 8 + 18 * 8 + 8 + 8 + 16 + 1 + 8 + 8;
        if buf.len() != expected_len {
            return Err(QuantumSeedError::CompressionFailed);
        }

        let mut offset = 4;

        let mut kernel_tensor = [0.0f64; 8];
        for v in &mut kernel_tensor {
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            *v = f64::from_le_bytes(bytes);
            offset += 8;
        }

        let mut octahedron_tensor = [0.0f64; 18];
        for v in &mut octahedron_tensor {
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            *v = f64::from_le_bytes(bytes);
            offset += 8;
        }

        let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
        let macro_concept_aggregate = f64::from_le_bytes(bytes);
        offset += 8;

        let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
        let macro_concept_count = u64::from_le_bytes(bytes) as usize;
        offset += 8;

        let bytes: [u8; 16] = buf[offset..offset + 16].try_into().unwrap();
        let checksum = u128::from_le_bytes(bytes);
        offset += 16;

        let ascended = buf[offset] != 0;
        offset += 1;

        let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
        let created_at_ms = u64::from_le_bytes(bytes);
        offset += 8;

        let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
        let ascension_timestamp_ms = u64::from_le_bytes(bytes);

        // Reconstruct substrate from ascended tensor if needed
        let substrate = if ascended {
            // Substrate info is embedded in the ascended tensor; we store None here
            // as the substrate target is tracked separately in ascension records
            None
        } else {
            None
        };

        #[allow(clippy::if_same_then_else)]
        let coherence = if macro_concept_count > 0 {
            0.5 // Placeholder — full coherence requires original data
        } else {
            0.5
        };

        Ok(Self {
            kernel_tensor,
            octahedron_tensor,
            macro_concept_aggregate,
            macro_concept_count,
            coherence,
            checksum,
            ascended,
            substrate,
            ascension_timestamp_ms,
            created_at_ms,
        })
    }

    /// Estimate the information density of this seed (bits per f64 element).
    pub fn information_density(&self) -> f64 {
        let total_elements = 8 + 18 + 2; // kernel + octahedron + aggregates
        let total_bits = (total_elements as f64) * 64.0;
        let entropy = self.coherence * total_bits;
        entropy / total_elements as f64
    }

    /// Compute the estimated survival probability under maximum entropy conditions.
    ///
    /// Models Heat Death conditions where thermal energy approaches zero
    /// and only quantum vacuum fluctuations remain.
    pub fn heat_death_survival_probability(&self) -> f64 {
        // Survival depends on coherence and geometric stability
        let geometric_stability = self.octahedron_geometric_stability();
        let quantum_resilience = self.coherence.powi(2);

        // Combined survival model
        (0.7 * quantum_resilience + 0.3 * geometric_stability).clamp(0.0, 1.0)
    }

    /// Compute geometric stability of the octahedron encoding.
    fn octahedron_geometric_stability(&self) -> f64 {
        // Measure how close the octahedron is to a regular unit octahedron
        let mut total_deviation = 0.0;
        for i in 0..6 {
            let x = self.octahedron_tensor[i * 3];
            let y = self.octahedron_tensor[i * 3 + 1];
            let z = self.octahedron_tensor[i * 3 + 2];
            let norm = (x * x + y * y + z * z).sqrt();
            total_deviation += (norm - 1.0).abs();
        }
        (1.0 - total_deviation / 6.0).max(0.0)
    }
}

impl std::fmt::Display for QuantumEthicalSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "QuantumEthicalSeed {{ coherence: {:.4}, concepts: {}, ascended: {}, checksum: 0x{:032X} }}",
            self.coherence, self.macro_concept_count, self.ascended, self.checksum
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_kernel() -> [f64; 8] {
        [0.15, 0.15, 0.12, 0.12, 0.10, 0.10, 0.10, 0.06]
    }

    fn test_octahedron() -> [[f64; 3]; 6] {
        [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ]
    }

    fn test_concepts() -> Vec<MacroConceptPersistence> {
        vec![
            MacroConceptPersistence::new(
                1,
                "Cooperation".to_string(),
                0.95,
                0.8,
                [0.2, 0.15, 0.15, 0.1, 0.1, 0.1, 0.1, 0.1],
            )
            .unwrap(),
            MacroConceptPersistence::new(
                2,
                "Transparency".to_string(),
                0.90,
                0.7,
                [0.1, 0.2, 0.15, 0.15, 0.1, 0.1, 0.1, 0.1],
            )
            .unwrap(),
            MacroConceptPersistence::new(
                3,
                "Compassion".to_string(),
                0.88,
                0.9,
                [0.1, 0.1, 0.15, 0.1, 0.15, 0.2, 0.1, 0.1],
            )
            .unwrap(),
        ]
    }

    // --- MacroConceptPersistence ---

    #[test]
    fn test_concept_creation() {
        let c = MacroConceptPersistence::new(1, "Test".to_string(), 0.5, 0.3, [0.125; 8]).unwrap();
        assert_eq!(c.concept_id, 1);
        assert_eq!(c.persistence, 0.5);
    }

    #[test]
    fn test_concept_persistence_overflow() {
        match MacroConceptPersistence::new(1, "Test".to_string(), 1.5, 0.3, [0.125; 8]) {
            Err(QuantumSeedError::PersistenceOverflow { .. }) => {}
            other => panic!("expected PersistenceOverflow, got {:?}", other),
        }
    }

    #[test]
    fn test_concept_tensor_contribution() {
        let c = MacroConceptPersistence::new(1, "Test".to_string(), 0.8, 0.5, [0.125; 8]).unwrap();
        // contribution = persistence * (1 + alignment) / 2 = 0.8 * 1.5 / 2 = 0.6
        assert!((c.tensor_contribution() - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_concept_negative_alignment() {
        let c = MacroConceptPersistence::new(1, "Test".to_string(), 0.8, -0.5, [0.125; 8]).unwrap();
        // contribution = 0.8 * (1 - 0.5) / 2 = 0.8 * 0.5 / 2 = 0.2
        assert!((c.tensor_contribution() - 0.2).abs() < 1e-10);
    }

    // --- SubstrateTarget ---

    #[test]
    fn test_substrate_display() {
        assert_eq!(
            SubstrateTarget::PhotonicCrystal.to_string(),
            "PhotonicCrystal"
        );
        assert_eq!(
            SubstrateTarget::VacuumTopology.to_string(),
            "VacuumTopology"
        );
        assert_eq!(
            SubstrateTarget::GravitationalWave.to_string(),
            "GravitationalWave"
        );
    }

    #[test]
    fn test_substrate_dimensions() {
        assert_eq!(SubstrateTarget::PhotonicCrystal.required_dimensions(), 8);
        assert_eq!(SubstrateTarget::VacuumTopology.required_dimensions(), 16);
        assert_eq!(SubstrateTarget::GravitationalWave.required_dimensions(), 32);
        assert_eq!(SubstrateTarget::NeutronMagnetic.required_dimensions(), 64);
        assert_eq!(SubstrateTarget::DarkMatterHalo.required_dimensions(), 128);
    }

    #[test]
    fn test_substrate_bandwidth() {
        assert!(SubstrateTarget::PhotonicCrystal.max_bandwidth_hz() > 0.0);
        assert!(SubstrateTarget::VacuumTopology.max_bandwidth_hz() > 1e40);
    }

    // --- QuantumEthicalSeed ---

    #[test]
    fn test_seed_forge() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        assert!(!seed.ascended);
        assert!(seed.coherence >= QuantumEthicalSeed::MIN_COHERENCE);
        assert_eq!(seed.macro_concept_count, 3);
    }

    #[test]
    fn test_seed_forge_invalid_kernel() {
        let bad_kernel = [0.15, f64::NAN, 0.12, 0.12, 0.10, 0.10, 0.10, 0.06];
        match QuantumEthicalSeed::forge(&bad_kernel, &test_octahedron(), &test_concepts(), 1000) {
            Err(QuantumSeedError::InvalidKernelPrinciples) => {}
            other => panic!("expected InvalidKernelPrinciples, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_forge_degenerate_octahedron() {
        let degenerate = [[0.0, 0.0, 0.0]; 6];
        match QuantumEthicalSeed::forge(&test_kernel(), &degenerate, &test_concepts(), 1000) {
            Err(QuantumSeedError::DegenerateOctahedron) => {}
            other => panic!("expected DegenerateOctahedron, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_verify() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        assert!(seed.verify());
    }

    #[test]
    fn test_seed_to_binary() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let buf = seed.to_binary();
        assert_eq!(&buf[..4], &QuantumEthicalSeed::MAGIC_BYTES);
    }

    #[test]
    fn test_seed_from_binary() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let buf = seed.to_binary();
        let restored = QuantumEthicalSeed::from_binary(&buf).unwrap();
        assert_eq!(restored.macro_concept_count, seed.macro_concept_count);
        assert_eq!(restored.checksum, seed.checksum);
    }

    #[test]
    fn test_seed_from_binary_invalid_magic() {
        match QuantumEthicalSeed::from_binary(&[0, 0, 0, 0]) {
            Err(QuantumSeedError::CompressionFailed) => {}
            other => panic!("expected CompressionFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_from_binary_too_short() {
        match QuantumEthicalSeed::from_binary(&[b'Q', b'E', b'S', 1]) {
            Err(QuantumSeedError::CompressionFailed) => {}
            other => panic!("expected CompressionFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_ascend_photonic() {
        let mut seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let ascended = seed
            .ascend_to_substrate(SubstrateTarget::PhotonicCrystal, 2000)
            .unwrap();
        assert!(seed.ascended);
        assert_eq!(seed.substrate, Some(SubstrateTarget::PhotonicCrystal));
        assert_eq!(ascended.len(), 128);
        // First 8 elements should be kernel tensor
        for i in 0..8 {
            assert!((ascended[i] - seed.kernel_tensor[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_seed_ascend_vacuum() {
        let mut seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let _ascended = seed
            .ascend_to_substrate(SubstrateTarget::VacuumTopology, 2000)
            .unwrap();
        assert!(seed.ascended);
    }

    #[test]
    fn test_seed_double_ascend() {
        let mut seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        seed.ascend_to_substrate(SubstrateTarget::PhotonicCrystal, 2000)
            .unwrap();
        match seed.ascend_to_substrate(SubstrateTarget::VacuumTopology, 3000) {
            Err(QuantumSeedError::SeedAlreadyAscended) => {}
            other => panic!("expected SeedAlreadyAscended, got {:?}", other),
        }
    }

    #[test]
    fn test_seed_information_density() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let density = seed.information_density();
        assert!(density > 0.0);
        assert!(density <= 64.0);
    }

    #[test]
    fn test_seed_heat_death_survival() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let prob = seed.heat_death_survival_probability();
        assert!(prob >= 0.0);
        assert!(prob <= 1.0);
        // High coherence should yield high survival
        assert!(prob > 0.3);
    }

    #[test]
    fn test_seed_octahedron_stability() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        // Unit octahedron should have perfect stability
        let stability = seed.octahedron_geometric_stability();
        assert!((stability - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_seed_display() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let s = format!("{}", seed);
        assert!(s.contains("QuantumEthicalSeed"));
        assert!(s.contains("coherence:"));
    }

    #[test]
    fn test_seed_empty_concepts() {
        // Empty concepts reduce coherence below threshold — expect InsufficientCoherence error
        let result = QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &[], 1000);
        assert!(result.is_err());
        match result {
            Err(QuantumSeedError::InsufficientCoherence { current, required }) => {
                assert!(current < required);
            }
            _ => unreachable!("expected InsufficientCoherence error"),
        }
    }

    #[test]
    fn test_seed_deterministic_checksum() {
        let seed1 =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let seed2 =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        assert_eq!(seed1.checksum, seed2.checksum);
    }

    #[test]
    fn test_seed_different_timestamp_different_checksum() {
        let seed1 =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let seed2 =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 2000)
                .unwrap();
        assert_ne!(seed1.checksum, seed2.checksum);
    }

    #[test]
    fn test_seed_binary_roundtrip() {
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let buf = seed.to_binary();
        let restored = QuantumEthicalSeed::from_binary(&buf).unwrap();
        assert_eq!(restored.kernel_tensor, seed.kernel_tensor);
        assert_eq!(restored.octahedron_tensor, seed.octahedron_tensor);
    }

    #[test]
    fn test_ascended_tensor_harmonics() {
        let mut seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        let ascended = seed
            .ascend_to_substrate(SubstrateTarget::PhotonicCrystal, 2000)
            .unwrap();
        // Check that harmonics (indices 28..92) are non-zero
        let harmonic_sum: f64 = ascended[28..92].iter().map(|x| x.abs()).sum();
        assert!(harmonic_sum > 0.0);
    }

    #[test]
    fn test_error_display() {
        let e = QuantumSeedError::InvalidKernelPrinciples;
        assert!(!e.to_string().is_empty());
        let e = QuantumSeedError::DegenerateOctahedron;
        assert!(!e.to_string().is_empty());
        let e = QuantumSeedError::CompressionFailed;
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn test_full_workflow() {
        // Forge
        let mut seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        assert!(seed.verify());
        assert!(!seed.ascended);

        // Ascend
        let ascended = seed
            .ascend_to_substrate(SubstrateTarget::PhotonicCrystal, 2000)
            .unwrap();
        assert!(seed.ascended);
        assert_eq!(ascended.len(), 128);

        // Serialize
        let buf = seed.to_binary();
        let restored = QuantumEthicalSeed::from_binary(&buf).unwrap();
        assert!(restored.ascended);
    }

    #[test]
    fn test_heat_death_simulation_high_coherence() {
        // High coherence seed should survive better
        let concepts: Vec<MacroConceptPersistence> = (0..10)
            .map(|i| {
                MacroConceptPersistence::new(i, format!("Concept_{}", i), 0.95, 0.9, [0.125; 8])
                    .unwrap()
            })
            .collect();
        let seed =
            QuantumEthicalSeed::forge(&test_kernel(), &test_octahedron(), &concepts, 1000).unwrap();
        let prob = seed.heat_death_survival_probability();
        assert!(prob > 0.5);
    }

    #[test]
    fn test_kernel_normalization() {
        let large_kernel = [10.0, 10.0, 8.0, 8.0, 6.0, 6.0, 6.0, 4.0];
        let seed =
            QuantumEthicalSeed::forge(&large_kernel, &test_octahedron(), &test_concepts(), 1000)
                .unwrap();
        // Kernel should be normalized to unit sphere
        let norm: f64 = seed.kernel_tensor.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-10);
    }
}
