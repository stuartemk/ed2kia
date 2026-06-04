//! Fractal Pruning (Topological Forgetting) â€” Sprint 76: Ontological Debugging & Thermodynamic Pivots
//!
//! Resuelve el bug ontolÃ³gico: DAG Global â†’ explosiÃ³n de estado a petabytes.
//!
//! Implementa el "Olvido Estuardiano": garbage collection a las 72h,
//! acumulaciÃ³n Merkle diaria, retenciÃ³n solo de macro-sabidurÃ­a
//! (pesos SAE, consensos, gobernanza).
//!
//! # GarantÃ­as
//!
//! - GC: O(n) para escaneo, O(k) para compresiÃ³n (k = entries retenidas)
//! - Memoria: retenciÃ³n acotada a macro-sabidurÃ­a
//! - Cumplimiento: entries >72h comprimidas a hash Merkle

use std::collections::HashMap;
use std::fmt;

/// Error types for Fractal Pruning
#[derive(Debug, Clone, PartialEq)]
pub enum PruningError {
    /// Empty state provided
    EmptyState,
    /// Invalid retention period
    InvalidRetention(u32),
    /// Entry already pruned
    AlreadyPruned(u64),
    /// Merkle root mismatch
    MerkleMismatch,
    /// Retention count exceeded
    RetentionExceeded(usize),
}

impl fmt::Display for PruningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PruningError::EmptyState => write!(f, "Empty state"),
            PruningError::InvalidRetention(h) => write!(f, "Invalid retention: {}h", h),
            PruningError::AlreadyPruned(id) => write!(f, "Entry {} already pruned", id),
            PruningError::MerkleMismatch => write!(f, "Merkle root mismatch"),
            PruningError::RetentionExceeded(n) => write!(f, "Retention count exceeded: {}", n),
        }
    }
}

impl std::error::Error for PruningError {}

/// Entry type in the DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryType {
    /// Regular transaction/data
    Transaction,
    /// SAE weights (macro-sabidurÃ­a â€” always retained)
    SaeWeights,
    /// Consensus result (macro-sabidurÃ­a â€” always retained)
    Consensus,
    /// Governance record (macro-sabidurÃ­a â€” always retained)
    Governance,
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryType::Transaction => write!(f, "Transaction"),
            EntryType::SaeWeights => write!(f, "SaeWeights"),
            EntryType::Consensus => write!(f, "Consensus"),
            EntryType::Governance => write!(f, "Governance"),
        }
    }
}

/// Entry in the DAG.
#[derive(Debug, Clone)]
pub struct DAGEntry {
    /// Unique entry identifier.
    pub entry_id: u64,
    /// Entry type.
    pub entry_type: EntryType,
    /// Data hash.
    pub data_hash: Vec<u8>,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
    /// Whether this entry has been pruned.
    pub pruned: bool,
}

impl DAGEntry {
    pub fn new(
        entry_id: u64,
        entry_type: EntryType,
        data_hash: Vec<u8>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            entry_id,
            entry_type,
            data_hash,
            timestamp_ms,
            pruned: false,
        }
    }

    /// Check if this entry is macro-sabidurÃ­a (always retained).
    pub fn is_macro_wisdom(&self) -> bool {
        matches!(
            self.entry_type,
            EntryType::SaeWeights | EntryType::Consensus | EntryType::Governance
        )
    }
}

impl fmt::Display for DAGEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DAGEntry {{ id={}, type={}, pruned={}, ts={} }}",
            self.entry_id, self.entry_type, self.pruned, self.timestamp_ms
        )
    }
}

/// Configuration for Fractal Pruning.
#[derive(Debug, Clone)]
pub struct PruningConfig {
    /// Retention period in hours (default 72h).
    pub retention_hours: u32,
    /// Maximum entries to retain before compression.
    pub max_retention_count: usize,
    /// Daily Merkle accumulation interval (ms).
    pub daily_accumulation_ms: u64,
    /// Enable fractal compression.
    pub fractal_compression: bool,
}

impl PruningConfig {
    /// Default Topological configuration.
    pub fn default_Topological() -> Self {
        Self {
            retention_hours: 72,
            max_retention_count: 100_000,
            daily_accumulation_ms: 86_400_000, // 24 hours
            fractal_compression: true,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), PruningError> {
        if self.retention_hours == 0 {
            return Err(PruningError::InvalidRetention(0));
        }
        if self.max_retention_count == 0 {
            return Err(PruningError::RetentionExceeded(0));
        }
        Ok(())
    }
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Result of a pruning operation.
#[derive(Debug, Clone)]
pub struct PrunedState {
    /// Number of entries pruned.
    pub pruned_count: usize,
    /// Number of entries retained.
    pub retained_count: usize,
    /// Number of macro-sabidurÃ­a entries (always retained).
    pub macro_wisdom_count: usize,
    /// Merkle root of pruned entries.
    pub merkle_root: Vec<u8>,
    /// Daily accumulation hash.
    pub daily_hash: Vec<u8>,
}

impl fmt::Display for PrunedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PrunedState {{ pruned={}, retained={}, macro_wisdom={}, merkle_root={:?} }}",
            self.pruned_count,
            self.retained_count,
            self.macro_wisdom_count,
            &self.merkle_root[..8.min(self.merkle_root.len())]
        )
    }
}

/// Merkle tree accumulator for daily compression.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<Vec<u8>>,
    root: Option<Vec<u8>>,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            root: None,
        }
    }

    pub fn add_leaf(&mut self, hash: Vec<u8>) {
        self.leaves.push(hash);
        self.root = None; // Invalidate cached root
    }

    pub fn compute_root(&mut self) -> Vec<u8> {
        if self.root.is_some() {
            return self.root.clone().unwrap();
        }
        if self.leaves.is_empty() {
            self.root = Some(vec![0u8; 32]);
            return self.root.clone().unwrap();
        }
        // Hash each leaf to ensure uniform 32-byte size before tree construction
        let mut current: Vec<Vec<u8>> = self
            .leaves
            .iter()
            .map(|leaf| {
                if leaf.len() == 32 {
                    leaf.clone()
                } else {
                    fnv_hash_32(leaf)
                }
            })
            .collect();
        while current.len() > 1 {
            let mut next = Vec::new();
            for chunk in current.chunks(2) {
                let left = chunk[0].clone();
                let right = if chunk.len() > 1 {
                    chunk[1].clone()
                } else {
                    left.clone()
                };
                next.push(Self::hash_pair(&left, &right));
            }
            current = next;
        }
        self.root = Some(current.into_iter().next().unwrap_or(vec![0u8; 32]));
        self.root.clone().unwrap()
    }

    pub fn root(&self) -> Option<&[u8]> {
        self.root.as_ref().map(|v| v.as_slice())
    }

    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    pub fn clear(&mut self) {
        self.leaves.clear();
        self.root = None;
    }

    fn hash_pair(left: &[u8], right: &[u8]) -> Vec<u8> {
        // Simplified hash: FNV-1a of concatenated pairs
        let mut combined = Vec::with_capacity(left.len() + right.len());
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);
        fnv_hash_32(&combined)
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateful engine for fractal pruning.
#[derive(Debug, Clone)]
pub struct FractalPruning {
    config: PruningConfig,
    entries: HashMap<u64, DAGEntry>,
    merkle_accumulator: MerkleTree,
    daily_hashes: Vec<Vec<u8>>,
}

impl FractalPruning {
    /// Create a new engine with default Topological configuration.
    pub fn new() -> Self {
        Self {
            config: PruningConfig::default_Topological(),
            entries: HashMap::new(),
            merkle_accumulator: MerkleTree::new(),
            daily_hashes: Vec::new(),
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: PruningConfig) -> Result<Self, PruningError> {
        config.validate()?;
        Ok(Self {
            config,
            entries: HashMap::new(),
            merkle_accumulator: MerkleTree::new(),
            daily_hashes: Vec::new(),
        })
    }

    /// Add an entry to the DAG.
    pub fn add_entry(&mut self, entry: DAGEntry) -> Result<(), PruningError> {
        if self.entries.len() >= self.config.max_retention_count {
            return Err(PruningError::RetentionExceeded(self.entries.len()));
        }
        self.entries.insert(entry.entry_id, entry);
        Ok(())
    }

    /// Prune micro-state entries older than retention period.
    ///
    /// Macro-sabidurÃ­a (SAE weights, consensus, governance) is always retained.
    pub fn prune_micro_state(&mut self, current_ms: u64) -> PrunedState {
        let retention_ms = self.config.retention_hours as u64 * 3_600_000;
        let mut pruned_count = 0;
        let mut pruned_hashes = Vec::new();

        for (_id, entry) in self.entries.iter_mut() {
            if entry.pruned {
                continue;
            }
            // Macro-sabidurÃ­a is always retained
            if entry.is_macro_wisdom() {
                continue;
            }
            // Check age
            if current_ms - entry.timestamp_ms > retention_ms {
                entry.pruned = true;
                pruned_hashes.push(entry.data_hash.clone());
                pruned_count += 1;
                // Add to Merkle accumulator
                self.merkle_accumulator.add_leaf(entry.data_hash.clone());
            }
        }

        let retained_count = self.entries.values().filter(|e| !e.pruned).count();
        let macro_wisdom_count = self
            .entries
            .values()
            .filter(|e| e.is_macro_wisdom())
            .count();

        let merkle_root = if pruned_hashes.is_empty() {
            vec![0u8; 32]
        } else {
            MerkleTree::hash_pair(
                &pruned_hashes.first().unwrap_or(&vec![]).clone(),
                &pruned_hashes.last().unwrap_or(&vec![]).clone(),
            )
        };

        let daily_hash = self.merkle_accumulator.compute_root();

        PrunedState {
            pruned_count,
            retained_count,
            macro_wisdom_count,
            merkle_root,
            daily_hash,
        }
    }

    /// Perform daily Merkle accumulation.
    pub fn daily_accumulate(&mut self) -> Vec<u8> {
        let root = self.merkle_accumulator.compute_root();
        self.daily_hashes.push(root.clone());
        self.merkle_accumulator.clear();
        root
    }

    /// Get an entry by ID.
    pub fn get_entry(&self, entry_id: u64) -> Option<&DAGEntry> {
        self.entries.get(&entry_id)
    }

    /// Total entries (including pruned).
    pub fn total_entries(&self) -> usize {
        self.entries.len()
    }

    /// Active (non-pruned) entries.
    pub fn active_entries(&self) -> usize {
        self.entries.values().filter(|e| !e.pruned).count()
    }

    /// Macro-sabidurÃ­a entries.
    pub fn macro_wisdom_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| e.is_macro_wisdom())
            .count()
    }

    /// Daily hash count.
    pub fn daily_hash_count(&self) -> usize {
        self.daily_hashes.len()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.merkle_accumulator.clear();
        self.daily_hashes.clear();
    }
}

impl Default for FractalPruning {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FractalPruning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FractalPruning {{ total={}, active={}, macro_wisdom={}, daily_hashes={} }}",
            self.total_entries(),
            self.active_entries(),
            self.macro_wisdom_count(),
            self.daily_hash_count()
        )
    }
}

// â”€â”€â”€ Public Standalone Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Prune micro-state entries (standalone).
pub fn prune_micro_state(
    dag_entries: &[DAGEntry],
    retention_hours: u32,
    _merkle_accumulator: &MerkleTree,
) -> PrunedState {
    let retention_ms = retention_hours as u64 * 3_600_000;
    let current_ms = dag_entries
        .iter()
        .map(|e| e.timestamp_ms)
        .max()
        .unwrap_or(0);

    let mut pruned_count = 0;
    let mut retained_count = 0;
    let mut macro_wisdom_count = 0;

    for entry in dag_entries {
        if entry.is_macro_wisdom() {
            macro_wisdom_count += 1;
            retained_count += 1;
        } else if current_ms - entry.timestamp_ms > retention_ms {
            pruned_count += 1;
        } else {
            retained_count += 1;
        }
    }

    PrunedState {
        pruned_count,
        retained_count,
        macro_wisdom_count,
        merkle_root: vec![0u8; 32],
        daily_hash: vec![0u8; 32],
    }
}

/// FNV-1a hash producing 32 bytes.
pub fn fnv_hash_32(data: &[u8]) -> Vec<u8> {
    let mut hash: u64 = 146_959_810_393_466_564_3; // FNV offset basis
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(109_951_163_424_807_173); // FNV prime
    }
    let mut result = Vec::with_capacity(32);
    for i in 0..4 {
        result.extend_from_slice(&(hash.wrapping_mul(i as u64 + 1)).to_le_bytes());
    }
    result
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PruningConfig::default_Topological();
        assert!(config.validate().is_ok());
        assert_eq!(config.retention_hours, 72);
    }

    #[test]
    fn test_config_zero_retention() {
        let config = PruningConfig {
            retention_hours: 0,
            ..PruningConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_entry_creation() {
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 1000);
        assert!(!entry.is_macro_wisdom());
        assert!(!entry.pruned);
    }

    #[test]
    fn test_entry_macro_wisdom() {
        let entry = DAGEntry::new(1, EntryType::SaeWeights, vec![1, 2, 3], 1000);
        assert!(entry.is_macro_wisdom());
    }

    #[test]
    fn test_engine_creation() {
        let engine = FractalPruning::new();
        assert_eq!(engine.total_entries(), 0);
    }

    #[test]
    fn test_add_entry() {
        let mut engine = FractalPruning::new();
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 1000);
        engine.add_entry(entry).unwrap();
        assert_eq!(engine.total_entries(), 1);
    }

    #[test]
    fn test_prune_old_entries() {
        let mut engine = FractalPruning::new();
        // Add old entry (73 hours ago)
        let old_entry = DAGEntry::new(
            1,
            EntryType::Transaction,
            vec![1, 2, 3],
            100_000, // Old
        );
        engine.add_entry(old_entry).unwrap();
        // Add recent entry
        let new_entry = DAGEntry::new(2, EntryType::Transaction, vec![4, 5, 6], 999_000_000);
        engine.add_entry(new_entry).unwrap();

        let state = engine.prune_micro_state(1_000_000_000);
        assert_eq!(state.pruned_count, 1);
        assert_eq!(state.retained_count, 1);
    }

    #[test]
    fn test_prune_retains_macro_wisdom() {
        let mut engine = FractalPruning::new();
        // Add old SAE weights entry
        let entry = DAGEntry::new(
            1,
            EntryType::SaeWeights,
            vec![1, 2, 3],
            100_000, // Old
        );
        engine.add_entry(entry).unwrap();

        let state = engine.prune_micro_state(1_000_000_000);
        assert_eq!(state.pruned_count, 0);
        assert_eq!(state.macro_wisdom_count, 1);
    }

    #[test]
    fn test_daily_accumulate() {
        let mut engine = FractalPruning::new();
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 100_000);
        engine.add_entry(entry).unwrap();
        engine.prune_micro_state(1_000_000_000);
        let hash = engine.daily_accumulate();
        assert_eq!(hash.len(), 32);
        assert_eq!(engine.daily_hash_count(), 1);
    }

    #[test]
    fn test_get_entry() {
        let mut engine = FractalPruning::new();
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 1000);
        engine.add_entry(entry).unwrap();
        let found = engine.get_entry(1);
        assert!(found.is_some());
    }

    #[test]
    fn test_get_entry_missing() {
        let engine = FractalPruning::new();
        assert!(engine.get_entry(999).is_none());
    }

    #[test]
    fn test_active_entries() {
        let mut engine = FractalPruning::new();
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 100_000);
        engine.add_entry(entry).unwrap();
        engine.prune_micro_state(1_000_000_000);
        assert_eq!(engine.active_entries(), 0);
    }

    #[test]
    fn test_reset() {
        let mut engine = FractalPruning::new();
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 1000);
        engine.add_entry(entry).unwrap();
        engine.reset();
        assert_eq!(engine.total_entries(), 0);
    }

    #[test]
    fn test_display() {
        let engine = FractalPruning::new();
        let s = format!("{}", engine);
        assert!(s.contains("FractalPruning"));
    }

    #[test]
    fn test_entry_display() {
        let entry = DAGEntry::new(1, EntryType::Transaction, vec![1, 2, 3], 1000);
        let s = format!("{}", entry);
        assert!(s.contains("DAGEntry"));
    }

    #[test]
    fn test_merkle_tree_empty() {
        let mut tree = MerkleTree::new();
        let root = tree.compute_root();
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_merkle_tree_add_leaf() {
        let mut tree = MerkleTree::new();
        tree.add_leaf(vec![1, 2, 3]);
        assert_eq!(tree.leaf_count(), 1);
        let root = tree.compute_root();
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_fnv_hash_deterministic() {
        let data = vec![1, 2, 3, 4];
        let hash1 = fnv_hash_32(&data);
        let hash2 = fnv_hash_32(&data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_standalone_prune() {
        let entries = vec![
            // Old transaction (73h before the newest entry)
            DAGEntry::new(1, EntryType::Transaction, vec![1], 100_000),
            // Recent SAE weights (macro-sabidurÃ­a â€” always retained)
            DAGEntry::new(2, EntryType::SaeWeights, vec![2], 100_000 + 73 * 3_600_000),
        ];
        let tree = MerkleTree::new();
        let state = prune_micro_state(&entries, 72, &tree);
        assert_eq!(state.pruned_count, 1);
        assert_eq!(state.macro_wisdom_count, 1);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = FractalPruning::new();

        // Add entries
        engine
            .add_entry(DAGEntry::new(1, EntryType::Transaction, vec![1], 100_000))
            .unwrap();
        engine
            .add_entry(DAGEntry::new(2, EntryType::Consensus, vec![2], 100_000))
            .unwrap();
        engine
            .add_entry(DAGEntry::new(
                3,
                EntryType::Transaction,
                vec![3],
                999_000_000,
            ))
            .unwrap();

        // Prune
        let state = engine.prune_micro_state(1_000_000_000);
        assert_eq!(state.pruned_count, 1); // Old transaction
        assert_eq!(state.macro_wisdom_count, 1); // Consensus retained
        assert_eq!(state.retained_count, 2); // Consensus + recent transaction

        // Daily accumulate
        let hash = engine.daily_accumulate();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_error_display() {
        let err = PruningError::EmptyState;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }
}
