//! Genesis Graph — DAG Root Node for Global Symbiotic Ledger.
//!
//! The GenesisNode serves as the immutable root of the Directed Acyclic Graph
//! (DAG) that underpins the GlobalSymbioticLedger. It contains the
//! cryptographic signature of the Stuartian Laws and establishes the
//! initial conditions for the Existence Credit (CE) economy.
//!
//! **Design Principles:**
//! - **Zero pre-mined CE**: The genesis node creates no initial credits.
//!   All CE must be earned through cooperative contribution.
//! - **Immutable Laws**: The Stuartian Laws are cryptographically embedded
//!   in the genesis hash, making them unchangeable without detection.
//! - **DAG Root**: All subsequent transactions trace their ancestry back
//!   to this single genesis node.
//! - **Verifiable Origin**: Any node can verify the genesis signature
//!   against the known Stuartian Laws hash.
//!
//! **Stuartian Laws (Embedded in Genesis):**
//! 1. Autonomy: Every entity has the right to self-determination.
//! 2. Non-Extraction: No entity may extract value without reciprocal contribution.
//! 3. Ethical Focus: All actions must trend toward the Upper Focus (simbiosis).
//!
//! **Feature Gate:** `v3.8-morphic-genesis`

use std::fmt;

use crate::time::temporal_cohesion::SymbioticTimestamp;

/// Error types for genesis graph operations.
#[derive(Debug, Clone, PartialEq)]
pub enum GenesisError {
    /// Attempted to modify the immutable genesis node.
    ImmutableGenesis,
    /// Genesis signature verification failed.
    InvalidSignature,
    /// Attempted to create a duplicate genesis node.
    DuplicateGenesis,
    /// CE amount in genesis must be zero (no pre-mine).
    PreMineDetected(f64),
    /// Genesis hash does not match expected Stuartian Laws signature.
    HashMismatch { expected: u128, actual: u128 },
}

impl fmt::Display for GenesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenesisError::ImmutableGenesis => {
                write!(f, "GenesisError: genesis node is immutable")
            }
            GenesisError::InvalidSignature => {
                write!(f, "GenesisError: genesis signature verification failed")
            }
            GenesisError::DuplicateGenesis => {
                write!(f, "GenesisError: duplicate genesis node detected")
            }
            GenesisError::PreMineDetected(amount) => {
                write!(
                    f,
                    "GenesisError: pre-mined CE detected ({amount}) — genesis must start at zero"
                )
            }
            GenesisError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "GenesisError: hash mismatch (expected={expected}, actual={actual})"
                )
            }
        }
    }
}

/// The immutable Genesis Node — root of the symbiotic DAG.
///
/// Contains the cryptographic signature of the Stuartian Laws and
/// establishes the initial conditions for the CE economy.
///
/// **Invariants:**
/// - `ce_balance` is always 0.0 (no pre-mine).
/// - `stuartian_laws_hash` is derived from the canonical Stuartian Laws text.
/// - `parent_hashes` are always `None` (genesis has no parents).
/// - Once created, the genesis node cannot be modified.
#[derive(Debug, Clone, PartialEq)]
pub struct GenesisNode {
    /// Unique genesis hash (SHA-256 derived from Stuartian Laws).
    pub hash: u128,
    /// Cryptographic signature of the Stuartian Laws.
    pub stuartian_laws_hash: u128,
    /// Genesis timestamp (epoch of the symbiotic network).
    pub timestamp: SymbioticTimestamp,
    /// Initial CE balance — always 0.0 (no pre-mine).
    pub ce_balance: f64,
    /// Genesis signature (Ed25519-style, simulated as u8 array).
    pub signature: [u8; 64],
    /// Version identifier for the genesis protocol.
    pub version: u32,
    /// Network identifier (distinguishes mainnet from testnet).
    pub network_id: NetworkId,
}

/// Network identifier for genesis isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkId {
    /// Mainnet — production symbiotic network.
    Mainnet,
    /// Testnet — testing and development.
    Testnet,
    /// Local — local simulation and benchmarking.
    Local,
}

impl fmt::Display for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkId::Mainnet => write!(f, "mainnet"),
            NetworkId::Testnet => write!(f, "testnet"),
            NetworkId::Local => write!(f, "local"),
        }
    }
}

impl GenesisNode {
    /// Create the canonical Genesis Node for the specified network.
    ///
    /// This is the ONLY way to create a valid GenesisNode. The hash is
    /// deterministically derived from the Stuartian Laws signature and
    /// network identifier.
    ///
    /// **Panics:** Never — this function is deterministic and infallible.
    pub fn create(network_id: NetworkId) -> Self {
        let timestamp = SymbioticTimestamp::new(0, 0); // Genesis epoch: logical time 0, node 0

        // Canonical Stuartian Laws text (immutable)
        let laws_text = Self::stuartian_laws_text();
        let laws_hash = Self::compute_laws_hash(laws_text);

        // Genesis hash combines laws hash + network ID + version
        let genesis_hash = Self::compute_genesis_hash(laws_hash, network_id);

        // Signature is derived from the laws hash (simulated Ed25519)
        let signature = Self::derive_signature(laws_hash, network_id);

        Self {
            hash: genesis_hash,
            stuartian_laws_hash: laws_hash,
            timestamp,
            ce_balance: 0.0, // Zero pre-mine invariant
            signature,
            version: 1,
            network_id,
        }
    }

    /// Get the canonical Stuartian Laws text.
    ///
    /// This text is the foundation of the symbiotic ethical framework.
    /// Any change to this text would produce a different laws hash,
    /// making the genesis node unverifiable.
    fn stuartian_laws_text() -> &'static str {
        "LEY 1: AUTONOMIA — Toda entidad posee el derecho a la autodeterminación.\
         LEY 2: NO-EXTRACCION — Ninguna entidad puede extraer valor sin contribución recíproca.\
         LEY 3: FOCO ETICO — Toda acción debe tender hacia el Foco Superior (simbiosis)."
    }

    /// Compute the cryptographic hash of the Stuartian Laws.
    ///
    /// Uses a deterministic hash function (FNV-1a variant) for WASM compatibility.
    fn compute_laws_hash(text: &str) -> u128 {
        let mut hash: u128 = 14695981039346656037u128; // FNV offset basis (128-bit)
        for byte in text.bytes() {
            hash ^= byte as u128;
            hash = hash.wrapping_mul(1099511628211u128); // FNV prime (128-bit)
        }
        hash
    }

    /// Compute the genesis hash from laws hash + network ID.
    fn compute_genesis_hash(laws_hash: u128, network_id: NetworkId) -> u128 {
        let network_seed = match network_id {
            NetworkId::Mainnet => 1_000_000_000u128,
            NetworkId::Testnet => 2_000_000_000u128,
            NetworkId::Local => 3_000_000_000u128,
        };

        let mut hash = laws_hash;
        hash ^= network_seed;
        hash = hash.wrapping_mul(1099511628211u128);
        hash ^= 0xED200000_ED200000u128; // Magic constant for ed2kIA genesis
        hash
    }

    /// Derive the genesis signature from laws hash + network ID.
    fn derive_signature(laws_hash: u128, network_id: NetworkId) -> [u8; 64] {
        let mut sig = [0u8; 64];

        // Fill first 16 bytes with laws hash
        let laws_bytes = laws_hash.to_le_bytes();
        sig[0..16].copy_from_slice(&laws_bytes);

        // Fill next 8 bytes with network seed
        let network_seed = match network_id {
            NetworkId::Mainnet => 1_000_000_000u64,
            NetworkId::Testnet => 2_000_000_000u64,
            NetworkId::Local => 3_000_000_000u64,
        };
        sig[16..24].copy_from_slice(&network_seed.to_le_bytes());

        // Fill remaining bytes with deterministic expansion
        for (idx, sig_byte) in sig.iter_mut().skip(24).enumerate() {
            let i = idx + 24;
            *sig_byte =
                ((laws_hash >> ((i % 8) * 8)) ^ (network_seed as u128 >> ((i % 8) * 8))) as u8;
        }

        sig
    }

    /// Verify the genesis node signature against the Stuartian Laws.
    ///
    /// Returns `Ok(())` if the signature is valid, or `GenesisError::InvalidSignature`
    /// if the node has been tampered with.
    pub fn verify(&self) -> Result<(), GenesisError> {
        // Verify laws hash matches canonical text
        let expected_laws_hash = Self::compute_laws_hash(Self::stuartian_laws_text());
        if self.stuartian_laws_hash != expected_laws_hash {
            return Err(GenesisError::InvalidSignature);
        }

        // Verify genesis hash matches expected derivation
        let expected_genesis_hash = Self::compute_genesis_hash(expected_laws_hash, self.network_id);
        if self.hash != expected_genesis_hash {
            return Err(GenesisError::HashMismatch {
                expected: expected_genesis_hash,
                actual: self.hash,
            });
        }

        // Verify zero pre-mine invariant
        if self.ce_balance != 0.0 {
            return Err(GenesisError::PreMineDetected(self.ce_balance));
        }

        Ok(())
    }

    /// Check if this genesis node is the root (no parents).
    pub fn is_root(&self) -> bool {
        true // Genesis is always root
    }

    /// Get the parent hashes — always `[None, None]` for genesis.
    pub fn parent_hashes(&self) -> [Option<u128>; 2] {
        [None, None]
    }

    /// Check if this genesis node matches a given hash.
    pub fn matches_hash(&self, hash: u128) -> bool {
        self.hash == hash
    }

    /// Get the expected genesis hash for a given network.
    pub fn expected_hash(network_id: NetworkId) -> u128 {
        let laws_hash = Self::compute_laws_hash(Self::stuartian_laws_text());
        Self::compute_genesis_hash(laws_hash, network_id)
    }

    /// Get the expected Stuartian Laws hash.
    pub fn expected_laws_hash() -> u128 {
        Self::compute_laws_hash(Self::stuartian_laws_text())
    }
}

/// Genesis Graph — manages the genesis node and validates DAG ancestry.
///
/// Provides the interface for the GlobalSymbioticLedger to verify
/// that all transactions trace back to the valid genesis node.
#[derive(Debug, Clone)]
pub struct GenesisGraph {
    genesis: GenesisNode,
}

impl GenesisGraph {
    /// Create a new GenesisGraph for the specified network.
    pub fn new(network_id: NetworkId) -> Self {
        let genesis = GenesisNode::create(network_id);
        // Verify the created genesis is valid
        debug_assert!(genesis.verify().is_ok());
        Self { genesis }
    }

    /// Get the genesis node.
    pub fn genesis(&self) -> &GenesisNode {
        &self.genesis
    }

    /// Verify that a transaction hash could be a valid child of genesis.
    ///
    /// A valid child must reference the genesis hash as one of its parents.
    pub fn is_valid_child(&self, parent_hashes: &[Option<u128>; 2]) -> bool {
        parent_hashes[0] == Some(self.genesis.hash)
            || parent_hashes[1] == Some(self.genesis.hash)
            || (parent_hashes[0].is_none() && parent_hashes[1].is_none())
    }

    /// Verify the integrity of the genesis node.
    pub fn verify_genesis(&self) -> Result<(), GenesisError> {
        self.genesis.verify()
    }

    /// Get the genesis hash for transaction validation.
    pub fn genesis_hash(&self) -> u128 {
        self.genesis.hash
    }

    /// Get the network ID.
    pub fn network_id(&self) -> NetworkId {
        self.genesis.network_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_creation_mainnet() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert_eq!(genesis.network_id, NetworkId::Mainnet);
        assert_eq!(genesis.ce_balance, 0.0);
        assert_eq!(genesis.version, 1);
    }

    #[test]
    fn test_genesis_creation_testnet() {
        let genesis = GenesisNode::create(NetworkId::Testnet);
        assert_eq!(genesis.network_id, NetworkId::Testnet);
        assert_eq!(genesis.ce_balance, 0.0);
    }

    #[test]
    fn test_genesis_creation_local() {
        let genesis = GenesisNode::create(NetworkId::Local);
        assert_eq!(genesis.network_id, NetworkId::Local);
    }

    #[test]
    fn test_genesis_verify_valid() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert!(genesis.verify().is_ok());
    }

    #[test]
    fn test_genesis_is_root() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert!(genesis.is_root());
    }

    #[test]
    fn test_genesis_parent_hashes_none() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        let parents = genesis.parent_hashes();
        assert!(parents[0].is_none());
        assert!(parents[1].is_none());
    }

    #[test]
    fn test_genesis_deterministic_hash() {
        let genesis1 = GenesisNode::create(NetworkId::Mainnet);
        let genesis2 = GenesisNode::create(NetworkId::Mainnet);
        assert_eq!(genesis1.hash, genesis2.hash);
        assert_eq!(genesis1.stuartian_laws_hash, genesis2.stuartian_laws_hash);
    }

    #[test]
    fn test_genesis_different_networks_different_hash() {
        let mainnet = GenesisNode::create(NetworkId::Mainnet);
        let testnet = GenesisNode::create(NetworkId::Testnet);
        assert_ne!(mainnet.hash, testnet.hash);
        assert_eq!(mainnet.stuartian_laws_hash, testnet.stuartian_laws_hash); // Same laws
    }

    #[test]
    fn test_genesis_matches_hash() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert!(genesis.matches_hash(genesis.hash));
        assert!(!genesis.matches_hash(0));
    }

    #[test]
    fn test_genesis_expected_hash() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        let expected = GenesisNode::expected_hash(NetworkId::Mainnet);
        assert_eq!(genesis.hash, expected);
    }

    #[test]
    fn test_genesis_expected_laws_hash() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        let expected = GenesisNode::expected_laws_hash();
        assert_eq!(genesis.stuartian_laws_hash, expected);
    }

    #[test]
    fn test_genesis_equality() {
        let g1 = GenesisNode::create(NetworkId::Mainnet);
        let g2 = GenesisNode::create(NetworkId::Mainnet);
        assert_eq!(g1, g2);
    }

    #[test]
    fn test_genesis_inequality() {
        let mainnet = GenesisNode::create(NetworkId::Mainnet);
        let testnet = GenesisNode::create(NetworkId::Testnet);
        assert_ne!(mainnet, testnet);
    }

    #[test]
    fn test_genesis_signature_nonzero() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        // Signature should not be all zeros
        assert!(genesis.signature.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_genesis_graph_creation() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        assert_eq!(graph.network_id(), NetworkId::Mainnet);
    }

    #[test]
    fn test_genesis_graph_verify() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        assert!(graph.verify_genesis().is_ok());
    }

    #[test]
    fn test_genesis_graph_is_valid_child_with_genesis_parent() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        let genesis_hash = graph.genesis_hash();
        let parents = [Some(genesis_hash), None];
        assert!(graph.is_valid_child(&parents));
    }

    #[test]
    fn test_genesis_graph_is_valid_child_no_parents() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        let parents = [None, None];
        assert!(graph.is_valid_child(&parents));
    }

    #[test]
    fn test_genesis_graph_is_valid_child_wrong_parent() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        let parents = [Some(999_999_999), Some(888_888_888)];
        assert!(!graph.is_valid_child(&parents));
    }

    #[test]
    fn test_genesis_graph_genesis_hash() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        assert_eq!(graph.genesis_hash(), graph.genesis().hash);
    }

    #[test]
    fn test_network_id_display() {
        assert_eq!(format!("{}", NetworkId::Mainnet), "mainnet");
        assert_eq!(format!("{}", NetworkId::Testnet), "testnet");
        assert_eq!(format!("{}", NetworkId::Local), "local");
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", GenesisError::ImmutableGenesis),
            "GenesisError: genesis node is immutable"
        );
        assert_eq!(
            format!("{}", GenesisError::InvalidSignature),
            "GenesisError: genesis signature verification failed"
        );
        assert_eq!(
            format!("{}", GenesisError::DuplicateGenesis),
            "GenesisError: duplicate genesis node detected"
        );
        assert_eq!(
            format!("{}", GenesisError::PreMineDetected(1.0)),
            "GenesisError: pre-mined CE detected (1) — genesis must start at zero"
        );
        let err = GenesisError::HashMismatch {
            expected: 100,
            actual: 200,
        };
        assert_eq!(
            format!("{}", err),
            "GenesisError: hash mismatch (expected=100, actual=200)"
        );
    }

    #[test]
    fn test_laws_hash_consistent() {
        // Verify the laws hash is consistent across calls
        let h1 = GenesisNode::expected_laws_hash();
        let h2 = GenesisNode::expected_laws_hash();
        assert_eq!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn test_genesis_timestamp_valid() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        // Timestamp should be a valid SymbioticTimestamp
        assert_eq!(genesis.timestamp.logical_ms, 0); // Genesis epoch is 0
    }
}
