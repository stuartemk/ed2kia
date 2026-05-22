//! DAO Operational Ledger v2 — Immutable ledger for DAO operational events.
//!
//! Tracks technical operations, resource allocations, and governance actions
//! as an append-only ledger with cryptographic hashing for integrity.
//! Analogous to Linux's auditd but for federated DAO operations.
//!
//! Zero financial logic: entries represent technical operations only.

use std::collections::HashMap;

/// Errors for DAO ledger operations.
#[derive(Debug)]
pub enum DaoLedgerError {
    EntryNotFound(String),
    DuplicateEntry(String),
    HashMismatch(String),
    ChainBroken(String),
    MaxEntriesExceeded(usize),
}

impl std::fmt::Display for DaoLedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaoLedgerError::EntryNotFound(id) => {
                write!(f, "Entry not found: {}", id)
            }
            DaoLedgerError::DuplicateEntry(id) => {
                write!(f, "Duplicate entry: {}", id)
            }
            DaoLedgerError::HashMismatch(id) => {
                write!(f, "Hash mismatch: {}", id)
            }
            DaoLedgerError::ChainBroken(id) => {
                write!(f, "Chain broken at: {}", id)
            }
            DaoLedgerError::MaxEntriesExceeded(max) => {
                write!(f, "Max entries exceeded: {}", max)
            }
        }
    }
}

/// Configuration for the DAO ledger.
#[derive(Debug, Clone)]
pub struct DaoLedgerConfig {
    /// Maximum number of entries before rotation.
    pub max_entries: usize,
    /// Enable automatic chain verification on each write.
    pub auto_verify: bool,
    /// Retain N previous hash roots for audit trail.
    pub retention_roots: usize,
}

impl Default for DaoLedgerConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            auto_verify: true,
            retention_roots: 100,
        }
    }
}

/// Type of operational event recorded in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DaoEventType {
    ResourceAllocated,
    ShardRegistered,
    ShardRemoved,
    PoolCreated,
    PoolDestroyed,
    GovernanceAction,
    ReputationUpdate,
    BridgeRelay,
    Custom(String),
}

impl std::fmt::Display for DaoEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaoEventType::ResourceAllocated => write!(f, "ResourceAllocated"),
            DaoEventType::ShardRegistered => write!(f, "ShardRegistered"),
            DaoEventType::ShardRemoved => write!(f, "ShardRemoved"),
            DaoEventType::PoolCreated => write!(f, "PoolCreated"),
            DaoEventType::PoolDestroyed => write!(f, "PoolDestroyed"),
            DaoEventType::GovernanceAction => write!(f, "GovernanceAction"),
            DaoEventType::ReputationUpdate => write!(f, "ReputationUpdate"),
            DaoEventType::BridgeRelay => write!(f, "BridgeRelay"),
            DaoEventType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// A single immutable ledger entry.
#[derive(Debug, Clone)]
pub struct DaoEntry {
    /// Unique entry identifier.
    pub entry_id: String,
    /// Sequence number in the chain.
    pub sequence: u64,
    /// Event type.
    pub event_type: DaoEventType,
    /// Acting node identifier.
    pub actor_id: String,
    /// Target resource identifier.
    pub target_id: String,
    /// Event payload data.
    pub payload: String,
    /// Cryptographic hash of this entry.
    pub hash: String,
    /// Hash of the previous entry (chain link).
    pub previous_hash: String,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

impl DaoEntry {
    /// Create a new ledger entry.
    pub fn new(
        entry_id: String,
        sequence: u64,
        event_type: DaoEventType,
        actor_id: String,
        target_id: String,
        payload: String,
        previous_hash: String,
    ) -> Self {
        let timestamp_ms = current_timestamp_ms();
        let hash = compute_hash(&entry_id, sequence, &payload, &previous_hash, timestamp_ms);
        Self {
            entry_id,
            sequence,
            event_type,
            actor_id,
            target_id,
            payload,
            hash,
            previous_hash,
            timestamp_ms,
        }
    }

    /// Verify this entry's hash integrity.
    pub fn verify_hash(&self) -> bool {
        let expected = compute_hash(
            &self.entry_id,
            self.sequence,
            &self.payload,
            &self.previous_hash,
            self.timestamp_ms,
        );
        self.hash == expected
    }

    /// Verify chain link to previous entry.
    pub fn verify_chain_link(&self, previous_entry: &DaoEntry) -> bool {
        self.previous_hash == previous_entry.hash
    }
}

/// Ledger statistics.
#[derive(Debug, Clone, Default)]
pub struct DaoLedgerStats {
    /// Total entries recorded.
    pub total_entries: usize,
    /// Total verifications performed.
    pub total_verifications: usize,
    /// Total chain breaks detected.
    pub chain_breaks_detected: usize,
    /// Current chain root hash.
    pub current_root_hash: String,
    /// Last entry timestamp (ms).
    pub last_entry_ms: u64,
}

/// DAO Operational Ledger v2 engine.
pub struct DaoLedgerV2 {
    /// Ledger configuration.
    pub config: DaoLedgerConfig,
    /// Ordered entries by sequence.
    entries: Vec<DaoEntry>,
    /// Entry lookup by ID.
    entry_index: HashMap<String, usize>,
    /// Hash root history for audit trail.
    root_history: Vec<String>,
    /// Next sequence number.
    next_sequence: u64,
    /// Ledger statistics.
    stats: DaoLedgerStats,
}

impl DaoLedgerV2 {
    /// Create a new DAO ledger with config.
    pub fn new(config: DaoLedgerConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
            entry_index: HashMap::new(),
            root_history: Vec::new(),
            next_sequence: 1,
            stats: DaoLedgerStats::default(),
        }
    }

    /// Create ledger with default config.
    pub fn with_defaults() -> Self {
        Self::new(DaoLedgerConfig::default())
    }

    /// Record a new operational event.
    pub fn record_event(
        &mut self,
        entry_id: String,
        event_type: DaoEventType,
        actor_id: String,
        target_id: String,
        payload: String,
    ) -> Result<DaoEntry, DaoLedgerError> {
        // Check duplicate
        if self.entry_index.contains_key(&entry_id) {
            return Err(DaoLedgerError::DuplicateEntry(entry_id.clone()));
        }

        // Check max entries
        if self.entries.len() >= self.config.max_entries {
            return Err(DaoLedgerError::MaxEntriesExceeded(self.config.max_entries));
        }

        // Get previous hash
        let previous_hash = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());

        // Create entry
        let entry = DaoEntry::new(
            entry_id.clone(),
            self.next_sequence,
            event_type,
            actor_id,
            target_id,
            payload,
            previous_hash,
        );

        // Auto-verify if enabled
        if self.config.auto_verify && !entry.verify_hash() {
            return Err(DaoLedgerError::HashMismatch(entry_id.clone()));
        }

        // Store entry
        let pos = self.entries.len();
        self.entries.push(entry.clone());
        self.entry_index.insert(entry_id, pos);
        self.next_sequence += 1;

        // Update stats
        self.stats.total_entries += 1;
        self.stats.current_root_hash = entry.hash.clone();
        self.stats.last_entry_ms = entry.timestamp_ms;

        // Update root history
        self.root_history.push(entry.hash.clone());
        if self.root_history.len() > self.config.retention_roots {
            self.root_history.remove(0);
        }

        Ok(entry)
    }

    /// Get an entry by ID.
    pub fn get_entry(&self, entry_id: &str) -> Option<&DaoEntry> {
        let pos = self.entry_index.get(entry_id)?;
        self.entries.get(*pos)
    }

    /// Get entry by sequence number.
    pub fn get_entry_by_sequence(&self, sequence: u64) -> Option<&DaoEntry> {
        self.entries.iter().find(|e| e.sequence == sequence)
    }

    /// Get entries filtered by event type.
    pub fn get_entries_by_type(&self, event_type: &DaoEventType) -> Vec<&DaoEntry> {
        self.entries
            .iter()
            .filter(|e| &e.event_type == event_type)
            .collect()
    }

    /// Get entries filtered by actor.
    pub fn get_entries_by_actor(&self, actor_id: &str) -> Vec<&DaoEntry> {
        self.entries
            .iter()
            .filter(|e| e.actor_id == actor_id)
            .collect()
    }

    /// Verify the entire chain integrity.
    pub fn verify_chain(&mut self) -> Result<(), DaoLedgerError> {
        self.stats.total_verifications += 1;
        for window in self.entries.windows(2) {
            let prev = &window[0];
            let next = &window[1];
            if !next.verify_chain_link(prev) {
                self.stats.chain_breaks_detected += 1;
                return Err(DaoLedgerError::ChainBroken(next.entry_id.clone()));
            }
            if !next.verify_hash() {
                self.stats.chain_breaks_detected += 1;
                return Err(DaoLedgerError::HashMismatch(next.entry_id.clone()));
            }
        }
        Ok(())
    }

    /// Compute current Merkle root from all entry hashes.
    pub fn compute_merkle_root(&self) -> String {
        if self.entries.is_empty() {
            return "empty".to_string();
        }
        let hashes: Vec<String> = self.entries.iter().map(|e| e.hash.clone()).collect();
        compute_merkle_root(&hashes)
    }

    /// Get recent entries (last N).
    pub fn get_recent_entries(&self, count: usize) -> Vec<&DaoEntry> {
        if count >= self.entries.len() {
            return self.entries.iter().collect();
        }
        self.entries.iter().rev().take(count).rev().collect()
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> DaoLedgerStats {
        self.stats.clone()
    }

    /// Get root hash history.
    pub fn get_root_history(&self) -> Vec<String> {
        self.root_history.clone()
    }

    /// Get total entry count.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for DaoLedgerV2 {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Compute cryptographic hash for an entry.
fn compute_hash(
    entry_id: &str,
    sequence: u64,
    payload: &str,
    previous_hash: &str,
    timestamp_ms: u64,
) -> String {
    let data = format!(
        "{}:{}:{}:{}:{}",
        entry_id, sequence, payload, previous_hash, timestamp_ms
    );
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hasher.write(data.as_bytes());
    format!("{:x}", hasher.finish())
}

/// Compute Merkle root from a list of hashes.
fn compute_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return "empty".to_string();
    }
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for chunk in current.chunks(2) {
            let combined = match chunk.len() {
                2 => format!("{}{}", chunk[0], chunk[1]),
                _ => chunk[0].clone(),
            };
            use std::hash::Hasher;
            let mut hasher = std::collections::hash_map::DefaultHasher::default();
            hasher.write(combined.as_bytes());
            next.push(format!("{:x}", hasher.finish()));
        }
        current = next;
    }
    current
        .into_iter()
        .next()
        .unwrap_or_else(|| "empty".to_string())
}

/// Helper to get current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_creation() {
        let ledger = DaoLedgerV2::with_defaults();
        assert!(ledger.is_empty());
        assert_eq!(ledger.entry_count(), 0);
    }

    #[test]
    fn test_record_event() {
        let mut ledger = DaoLedgerV2::with_defaults();
        let entry = ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "shard1".to_string(),
                "registered".to_string(),
            )
            .unwrap();
        assert_eq!(entry.entry_id, "e1");
        assert_eq!(entry.sequence, 1);
        assert_eq!(ledger.entry_count(), 1);
    }

    #[test]
    fn test_duplicate_entry() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "shard1".to_string(),
                "data".to_string(),
            )
            .unwrap();
        match ledger.record_event(
            "e1".to_string(),
            DaoEventType::ShardRegistered,
            "node1".to_string(),
            "shard1".to_string(),
            "data".to_string(),
        ) {
            Err(DaoLedgerError::DuplicateEntry(id)) => assert_eq!(id, "e1"),
            _ => panic!("Expected DuplicateEntry"),
        }
    }

    #[test]
    fn test_chain_verification() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "shard1".to_string(),
                "data1".to_string(),
            )
            .unwrap();
        ledger
            .record_event(
                "e2".to_string(),
                DaoEventType::ResourceAllocated,
                "node2".to_string(),
                "pool1".to_string(),
                "data2".to_string(),
            )
            .unwrap();
        assert!(ledger.verify_chain().is_ok());
    }

    #[test]
    fn test_chain_linking() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "d1".to_string(),
            )
            .unwrap();
        ledger
            .record_event(
                "e2".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s2".to_string(),
                "d2".to_string(),
            )
            .unwrap();
        let e1 = ledger.get_entry("e1").unwrap();
        let e2 = ledger.get_entry("e2").unwrap();
        assert_eq!(e2.previous_hash, e1.hash);
    }

    #[test]
    fn test_hash_verification() {
        let mut ledger = DaoLedgerV2::with_defaults();
        let entry = ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "data".to_string(),
            )
            .unwrap();
        assert!(entry.verify_hash());
    }

    #[test]
    fn test_get_entry_by_type() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "d1".to_string(),
            )
            .unwrap();
        ledger
            .record_event(
                "e2".to_string(),
                DaoEventType::ResourceAllocated,
                "node1".to_string(),
                "p1".to_string(),
                "d2".to_string(),
            )
            .unwrap();
        let shards = ledger.get_entries_by_type(&DaoEventType::ShardRegistered);
        assert_eq!(shards.len(), 1);
        let allocs = ledger.get_entries_by_type(&DaoEventType::ResourceAllocated);
        assert_eq!(allocs.len(), 1);
    }

    #[test]
    fn test_get_entries_by_actor() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "d1".to_string(),
            )
            .unwrap();
        ledger
            .record_event(
                "e2".to_string(),
                DaoEventType::ShardRegistered,
                "node2".to_string(),
                "s2".to_string(),
                "d2".to_string(),
            )
            .unwrap();
        let node1_entries = ledger.get_entries_by_actor("node1");
        assert_eq!(node1_entries.len(), 1);
    }

    #[test]
    fn test_merkle_root() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "data".to_string(),
            )
            .unwrap();
        let root = ledger.compute_merkle_root();
        assert!(!root.is_empty());
        assert_ne!(root, "empty");
    }

    #[test]
    fn test_stats_tracking() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "data".to_string(),
            )
            .unwrap();
        ledger.verify_chain().unwrap();
        let stats = ledger.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_verifications, 1);
    }

    #[test]
    fn test_root_history() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "d1".to_string(),
            )
            .unwrap();
        ledger
            .record_event(
                "e2".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s2".to_string(),
                "d2".to_string(),
            )
            .unwrap();
        let history = ledger.get_root_history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_get_recent_entries() {
        let mut ledger = DaoLedgerV2::with_defaults();
        for i in 0..5 {
            ledger
                .record_event(
                    format!("e{}", i),
                    DaoEventType::ShardRegistered,
                    "node1".to_string(),
                    format!("s{}", i),
                    "data".to_string(),
                )
                .unwrap();
        }
        let recent = ledger.get_recent_entries(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].entry_id, "e2");
    }

    #[test]
    fn test_get_entry_by_sequence() {
        let mut ledger = DaoLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "data".to_string(),
            )
            .unwrap();
        let entry = ledger.get_entry_by_sequence(1).unwrap();
        assert_eq!(entry.entry_id, "e1");
        assert!(ledger.get_entry_by_sequence(2).is_none());
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(DaoEventType::ShardRegistered.to_string(), "ShardRegistered");
        assert_eq!(
            DaoEventType::ResourceAllocated.to_string(),
            "ResourceAllocated"
        );
        assert_eq!(
            DaoEventType::Custom("test".to_string()).to_string(),
            "Custom(test)"
        );
    }

    #[test]
    fn test_config_default() {
        let config = DaoLedgerConfig::default();
        assert_eq!(config.max_entries, 10_000);
        assert!(config.auto_verify);
        assert_eq!(config.retention_roots, 100);
    }

    #[test]
    fn test_stats_default() {
        let stats = DaoLedgerStats::default();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_verifications, 0);
        assert_eq!(stats.chain_breaks_detected, 0);
    }

    #[test]
    fn test_ledger_default() {
        let ledger = DaoLedgerV2::default();
        assert!(ledger.is_empty());
    }

    #[test]
    fn test_error_display() {
        match DaoLedgerError::EntryNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_empty_merkle_root() {
        let ledger = DaoLedgerV2::with_defaults();
        assert_eq!(ledger.compute_merkle_root(), "empty");
    }

    #[test]
    fn test_genesis_previous_hash() {
        let mut ledger = DaoLedgerV2::with_defaults();
        let entry = ledger
            .record_event(
                "e1".to_string(),
                DaoEventType::ShardRegistered,
                "node1".to_string(),
                "s1".to_string(),
                "data".to_string(),
            )
            .unwrap();
        assert_eq!(entry.previous_hash, "genesis");
    }
}
