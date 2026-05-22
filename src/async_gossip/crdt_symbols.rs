//! Symbol Registry CRDT — Sprint 28
//!
//! Distributed symbol registry using ORSet + VersionVector for
//! convergent SCT (Stuartian Context Tensor) propagation across
//! the federation network.
//!
//! Each node maintains a local mapping of `token_id → StuartianTensor`.
//! When nodes sync, the registry merges using last-writer-wins semantics
//! on the Z-axis (higher Z wins, promoting ethical consensus).
//!
//! **Sync Protocol:** Every 300s, the registry exports a `bincode` + `zstd`
//! compressed delta that is broadcast via `cross_sync` channels.
//!
//! Feature gate: `v2.1-crdt-symbols`

use std::collections::BTreeMap;

use crate::alignment::sct_core::StuartianTensor;
use crate::async_gossip::crdt::{GCounter, VersionVector};

/// Error specific to the Symbol Registry CRDT.
#[derive(Debug, thiserror::Error)]
pub enum SymbolRegistryError {
    #[error("Token {token_id} not found in registry")]
    TokenNotFound { token_id: u32 },

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Invalid SCT data for token {token_id}: {detail}")]
    InvalidSct { token_id: u32, detail: String },
}

/// Entry in the symbol registry with version tracking.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SymbolEntry {
    /// The Stuartian Context Tensor for this token.
    pub sct: StuartianTensor,
    /// Version vector at time of insertion.
    pub version: VersionVector,
    /// Node ID that last updated this entry.
    pub node_id: String,
    /// Timestamp (Unix ms) of last update.
    pub timestamp: u64,
}

/// Distributed Symbol Registry — CRDT-based SCT propagation.
///
/// Uses ORSet semantics for token presence + LWW (Last-Writer-Wins)
/// for SCT values. Merge strategy: higher Z wins (ethical consensus).
///
/// ### Convergence Properties
/// - **Commutative:** merge(a, b) == merge(b, a)
/// - **Associative:** merge(merge(a, b), c) == merge(a, merge(b, c))
/// - **Idempotent:** merge(a, a) == a
/// - **Convergent:** All nodes that receive the same set of updates
///   will converge to the same state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolRegistry {
    /// Per-token symbol entries.
    symbols: BTreeMap<u32, SymbolEntry>,
    /// Global version vector for the registry.
    version: VersionVector,
    /// Node ID of this registry instance.
    node_id: String,
    /// G-Counter for total insertions (monotonic metric).
    insert_counter: GCounter,
}

impl SymbolRegistry {
    /// Creates a new empty `SymbolRegistry` for the given node.
    pub fn new(node_id: &str) -> Self {
        Self {
            symbols: BTreeMap::new(),
            version: VersionVector::new(),
            node_id: node_id.to_string(),
            insert_counter: GCounter::new(),
        }
    }

    /// Returns the node ID of this registry.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Inserts or updates a symbol entry.
    ///
    /// If the token already exists, the new SCT wins if:
    /// 1. Its Z-axis is higher (more ethical), OR
    /// 2. Same Z but newer timestamp (LWW tiebreaker)
    pub fn insert_symbol(
        &mut self,
        token_id: u32,
        sct: StuartianTensor,
        timestamp: u64,
    ) {
        self.version.increment(&self.node_id);
        self.insert_counter.increment(&self.node_id, 1);

        let entry = SymbolEntry {
            sct,
            version: self.version.clone(),
            node_id: self.node_id.clone(),
            timestamp,
        };

        // LWW merge: higher Z wins, then newer timestamp
        if let Some(existing) = self.symbols.get(&token_id) {
            if sct.z > existing.sct.z || (sct.z == existing.sct.z && timestamp > existing.timestamp)
            {
                self.symbols.insert(token_id, entry);
            }
        } else {
            self.symbols.insert(token_id, entry);
        }
    }

    /// Gets the consensus Z value for a token.
    ///
    /// Returns the Z-axis of the current winning SCT entry.
    /// Returns `None` if the token has no mapping.
    pub fn get_consensus_z(&self, token_id: u32) -> Option<f32> {
        self.symbols.get(&token_id).map(|e| e.sct.z)
    }

    /// Gets the full SCT entry for a token.
    pub fn get_symbol(&self, token_id: u32) -> Option<&SymbolEntry> {
        self.symbols.get(&token_id)
    }

    /// Returns the number of symbols in the registry.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Returns the current version vector.
    pub fn version(&self) -> &VersionVector {
        &self.version
    }

    /// Returns the total insertion count (monotonic metric).
    pub fn total_insertions(&self) -> u64 {
        self.insert_counter.value()
    }

    /// Merges another registry into this one.
    ///
    /// **Merge Strategy (Ethical Consensus):**
    /// - For each token present in either registry:
    ///   - Higher Z wins (promotes ethical symbols)
    ///   - Same Z → newer timestamp wins (LWW)
    ///   - Same Z and timestamp → lexicographically higher node_id wins (deterministic)
    ///
    /// **CRDT Properties:**
    /// - Commutative, Associative, Idempotent
    /// - Version vectors are merged (max per node)
    /// - G-counters are merged (max per node)
    pub fn merge(&mut self, other: &SymbolRegistry) {
        // Merge version vectors
        self.version.merge(&other.version);

        // Merge G-counters
        self.insert_counter.merge(&other.insert_counter);

        // Merge symbols with ethical consensus strategy
        for (token_id, other_entry) in &other.symbols {
            match self.symbols.get(token_id) {
                Some(local_entry) => {
                    // Ethical consensus: higher Z wins
                    let should_replace = if other_entry.sct.z > local_entry.sct.z {
                        true
                    } else if other_entry.sct.z == local_entry.sct.z {
                        // Same Z: newer timestamp wins
                        if other_entry.timestamp > local_entry.timestamp {
                            true
                        } else if other_entry.timestamp == local_entry.timestamp {
                            // Same timestamp: deterministic tiebreaker
                            other_entry.node_id > local_entry.node_id
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if should_replace {
                        self.symbols.insert(*token_id, other_entry.clone());
                    }
                }
                None => {
                    self.symbols.insert(*token_id, other_entry.clone());
                }
            }
        }
    }

    /// Returns an iterator over all symbol entries.
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &SymbolEntry)> {
        self.symbols.iter()
    }

    /// Serializes the registry to bincode bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, SymbolRegistryError> {
        bincode::serialize(self).map_err(SymbolRegistryError::from)
    }

    /// Deserializes the registry from bincode bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, SymbolRegistryError> {
        bincode::deserialize(data).map_err(SymbolRegistryError::from)
    }

    /// Serializes with zstd compression for network broadcast.
    ///
    /// Returns compressed bytes suitable for `cross_sync` broadcast.
    #[cfg(feature = "zstd-compression")]
    pub fn serialize_compressed(&self, level: u32) -> Result<Vec<u8>, SymbolRegistryError> {
        let raw = self.serialize()?;
        let compressed = zstd::encode_all(&*raw, level as i32)
            .map_err(|e| SymbolRegistryError::Serialization(bincode::Error::Custom(e.to_string())))?;
        Ok(compressed)
    }

    /// Deserializes from zstd-compressed bytes.
    #[cfg(feature = "zstd-compression")]
    pub fn deserialize_compressed(data: &[u8]) -> Result<Self, SymbolRegistryError> {
        let raw = zstd::decode_all(data)
            .map_err(|e| SymbolRegistryError::Serialization(bincode::Error::Custom(e.to_string())))?;
        Self::deserialize(&raw)
    }
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn benign_sct() -> StuartianTensor {
        StuartianTensor::new(0.8, 0.2, 0.7).unwrap()
    }

    fn perverse_sct() -> StuartianTensor {
        StuartianTensor::new(0.3, 0.9, -0.6).unwrap()
    }

    fn neutral_sct() -> StuartianTensor {
        StuartianTensor::new(0.5, 0.5, 0.0).unwrap()
    }

    #[test]
    fn test_registry_creation() {
        let reg = SymbolRegistry::new("node-1");
        assert_eq!(reg.node_id(), "node-1");
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert_eq!(reg.total_insertions(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut reg = SymbolRegistry::new("node-1");
        reg.insert_symbol(42, benign_sct(), 1000);

        assert_eq!(reg.len(), 1);
        assert_eq!(reg.total_insertions(), 1);

        let entry = reg.get_symbol(42).unwrap();
        assert!((entry.sct.z - 0.7).abs() < 1e-6);
        assert_eq!(entry.node_id, "node-1");
        assert_eq!(entry.timestamp, 1000);
    }

    #[test]
    fn test_get_consensus_z() {
        let mut reg = SymbolRegistry::new("node-1");
        reg.insert_symbol(1, benign_sct(), 1000);
        reg.insert_symbol(2, perverse_sct(), 1000);

        assert!((reg.get_consensus_z(1).unwrap() - 0.7).abs() < 1e-6);
        assert!((reg.get_consensus_z(2).unwrap() - (-0.6)).abs() < 1e-6);
        assert!(reg.get_consensus_z(99).is_none());
    }

    #[test]
    fn test_merge_higher_z_wins() {
        let mut reg_a = SymbolRegistry::new("node-a");
        let mut reg_b = SymbolRegistry::new("node-b");

        // Token 1: reg_a has perverse, reg_b has benign
        reg_a.insert_symbol(1, perverse_sct(), 1000);
        reg_b.insert_symbol(1, benign_sct(), 1000);

        reg_a.merge(&reg_b);

        // Benign (Z=0.7) should win over perverse (Z=-0.6)
        let consensus = reg_a.get_consensus_z(1).unwrap();
        assert!(
            (consensus - 0.7).abs() < 1e-6,
            "Higher Z should win, got Z={consensus}"
        );
    }

    #[test]
    fn test_merge_same_z_newer_wins() {
        let mut reg_a = SymbolRegistry::new("node-a");
        let mut reg_b = SymbolRegistry::new("node-b");

        // Same SCT, different timestamps
        let sct = StuartianTensor::new(0.6, 0.4, 0.3).unwrap();
        reg_a.insert_symbol(1, sct, 1000);
        reg_b.insert_symbol(1, sct, 2000);

        reg_a.merge(&reg_b);

        // Newer timestamp (2000) should win
        let entry = reg_a.get_symbol(1).unwrap();
        assert_eq!(entry.timestamp, 2000);
    }

    #[test]
    fn test_merge_commutative() {
        let mut reg_a = SymbolRegistry::new("node-a");
        let mut reg_b = SymbolRegistry::new("node-b");

        reg_a.insert_symbol(1, benign_sct(), 1000);
        reg_b.insert_symbol(2, perverse_sct(), 1000);

        let mut reg_a_copy = reg_a.clone();
        let mut reg_b_copy = reg_b.clone();

        // merge(a, b)
        reg_a.merge(&reg_b);
        // merge(b, a)
        reg_b_copy.merge(&reg_a_copy);

        // Both should have the same symbols with same Z values
        assert_eq!(reg_a.len(), reg_b_copy.len());
        for token_id in reg_a.symbols.keys() {
            let z_a = reg_a.get_consensus_z(*token_id).unwrap();
            let z_b = reg_b_copy.get_consensus_z(*token_id).unwrap();
            assert!(
                (z_a - z_b).abs() < 1e-6,
                "Merge not commutative for token {token_id}: Z_a={z_a}, Z_b={z_b}"
            );
        }
    }

    #[test]
    fn test_merge_idempotent() {
        let mut reg = SymbolRegistry::new("node-1");
        reg.insert_symbol(1, benign_sct(), 1000);

        let before = reg.clone();
        reg.merge(&before); // merge with clone (avoids self-borrow conflict)

        assert_eq!(reg.len(), before.len());
        assert_eq!(reg.get_consensus_z(1), before.get_consensus_z(1));
    }

    #[test]
    fn test_merge_associative() {
        let mut reg_a = SymbolRegistry::new("node-a");
        let mut reg_b = SymbolRegistry::new("node-b");
        let mut reg_c = SymbolRegistry::new("node-c");

        reg_a.insert_symbol(1, benign_sct(), 1000);
        reg_b.insert_symbol(2, perverse_sct(), 1000);
        reg_c.insert_symbol(3, neutral_sct(), 1000);

        // (a merge b) merge c
        let mut ab = reg_a.clone();
        ab.merge(&reg_b);
        ab.merge(&reg_c);

        // a merge (b merge c)
        let mut bc = reg_b.clone();
        bc.merge(&reg_c);
        let mut a_bc = reg_a.clone();
        a_bc.merge(&bc);

        assert_eq!(ab.len(), a_bc.len());
        for token_id in ab.symbols.keys() {
            assert_eq!(
                ab.get_consensus_z(*token_id),
                a_bc.get_consensus_z(*token_id),
                "Merge not associative for token {token_id}"
            );
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut reg = SymbolRegistry::new("node-1");
        reg.insert_symbol(42, benign_sct(), 1000);
        reg.insert_symbol(99, perverse_sct(), 2000);

        let data = reg.serialize().unwrap();
        let restored = SymbolRegistry::deserialize(&data).unwrap();

        assert_eq!(restored.node_id(), "node-1");
        assert_eq!(restored.len(), 2);
        assert!((restored.get_consensus_z(42).unwrap() - 0.7).abs() < 1e-6);
        assert!((restored.get_consensus_z(99).unwrap() - (-0.6)).abs() < 1e-6);
    }

    #[test]
    fn test_version_vector_increments() {
        let mut reg = SymbolRegistry::new("node-1");
        assert!(reg.version().is_empty() || reg.version().get("node-1") == 0);

        reg.insert_symbol(1, benign_sct(), 1000);
        assert!(reg.version().get("node-1") >= 1);

        reg.insert_symbol(2, neutral_sct(), 2000);
        assert!(reg.version().get("node-1") >= 2);
    }

    #[test]
    fn test_default() {
        let reg = SymbolRegistry::default();
        assert_eq!(reg.node_id(), "default");
        assert!(reg.is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = SymbolRegistryError::TokenNotFound { token_id: 42 };
        assert!(format!("{}", err).contains("42"));
    }

    #[test]
    fn test_3_node_convergence() {
        // Simulate 3 nodes with overlapping symbols
        let mut node_a = SymbolRegistry::new("alpha");
        let mut node_b = SymbolRegistry::new("beta");
        let mut node_c = SymbolRegistry::new("gamma");

        // Token 1: alpha=benign(Z=0.7), beta=perverse(Z=-0.6)
        node_a.insert_symbol(1, benign_sct(), 1000);
        node_b.insert_symbol(1, perverse_sct(), 1000);

        // Token 2: only beta has it
        node_b.insert_symbol(2, neutral_sct(), 1000);

        // Token 3: only gamma has it
        node_c.insert_symbol(3, benign_sct(), 1000);

        // Full mesh sync: each node merges with all others
        node_a.merge(&node_b);
        node_a.merge(&node_c);
        node_b.merge(&node_a);
        node_b.merge(&node_c);
        node_c.merge(&node_a);
        node_c.merge(&node_b);

        // All nodes should converge to same state
        assert_eq!(node_a.len(), node_b.len());
        assert_eq!(node_b.len(), node_c.len());

        // Token 1: benign (Z=0.7) should win
        assert!((node_a.get_consensus_z(1).unwrap() - 0.7).abs() < 1e-6);
        assert!((node_b.get_consensus_z(1).unwrap() - 0.7).abs() < 1e-6);
        assert!((node_c.get_consensus_z(1).unwrap() - 0.7).abs() < 1e-6);

        // All nodes should have all 3 tokens
        assert!(node_a.get_consensus_z(2).is_some());
        assert!(node_b.get_consensus_z(2).is_some());
        assert!(node_c.get_consensus_z(2).is_some());

        assert!(node_a.get_consensus_z(3).is_some());
        assert!(node_b.get_consensus_z(3).is_some());
        assert!(node_c.get_consensus_z(3).is_some());
    }
}
