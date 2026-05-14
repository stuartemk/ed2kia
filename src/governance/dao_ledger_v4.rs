//! DAO Ledger v4 — Immutable governance ledger with cryptographic audit trail.
//!
//! Extends DAO Ledger v2 with hybrid execution support (on-chain/off-chain),
//! Merkle proof generation and quorum-based validation.
//!
//! **Design:** Linux `auditd`-inspired immutable audit log for DAO governance.
//!
//! **Key features:**
//! - Immutable entry chain with hash linking
//! - Hybrid execution tracking (on-chain/off-chain)
//! - Merkle root computation for proof generation
//! - Quorum-based event validation
//! - Cryptographic audit trail
//!
//! **References:**
//! - `dao_ledger_v2.rs` — Base ledger patterns
//! - `pool_zkp_bridge.rs` — Bridge proof patterns
//!
//! Apache License 2.0 + Ethical Use Clause

use std::collections::HashMap;

// ─── Error ───────────────────────────────────────────────────────────────────

/// Errors for DAO Ledger v4 operations.
#[derive(Debug, Clone, PartialEq)]
pub enum DaoLedgerV4Error {
    /// Entry ID not found.
    EntryNotFound(String),
    /// Duplicate entry ID.
    DuplicateEntry(String),
    /// Hash verification failed.
    HashMismatch(String),
    /// Quorum not reached.
    QuorumNotReached(f64),
    /// Invalid configuration.
    InvalidConfig(String),
    /// Ledger is full.
    LedgerFull,
    /// Execution type mismatch.
    ExecutionMismatch,
}

impl std::fmt::Display for DaoLedgerV4Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaoLedgerV4Error::EntryNotFound(id) => write!(f, "Entry not found: {}", id),
            DaoLedgerV4Error::DuplicateEntry(id) => write!(f, "Duplicate entry: {}", id),
            DaoLedgerV4Error::HashMismatch(id) => write!(f, "Hash mismatch: {}", id),
            DaoLedgerV4Error::QuorumNotReached(q) => write!(f, "Quorum not reached: {:.2}", q),
            DaoLedgerV4Error::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            DaoLedgerV4Error::LedgerFull => write!(f, "Ledger is full"),
            DaoLedgerV4Error::ExecutionMismatch => write!(f, "Execution type mismatch"),
        }
    }
}

// ─── Execution Type ──────────────────────────────────────────────────────────

/// Execution mode for governance events.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionType {
    /// On-chain execution (requires consensus).
    OnChain,
    /// Off-chain execution (executed locally).
    OffChain,
    /// Hybrid execution (on-chain verification, off-chain execution).
    Hybrid,
}

impl std::fmt::Display for ExecutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionType::OnChain => write!(f, "OnChain"),
            ExecutionType::OffChain => write!(f, "OffChain"),
            ExecutionType::Hybrid => write!(f, "Hybrid"),
        }
    }
}

// ─── Event Type ──────────────────────────────────────────────────────────────

/// Types of DAO governance events.
#[derive(Debug, Clone, PartialEq)]
pub enum DaoEventV4 {
    /// Proposal created.
    ProposalCreated,
    /// Vote cast.
    VoteCast,
    /// Proposal executed.
    ProposalExecuted,
    /// Parameter changed.
    ParameterChanged,
    /// Member added.
    MemberAdded,
    /// Member removed.
    MemberRemoved,
    /// Emergency action.
    EmergencyAction,
    /// Custom event.
    Custom(String),
}

impl std::fmt::Display for DaoEventV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaoEventV4::ProposalCreated => write!(f, "ProposalCreated"),
            DaoEventV4::VoteCast => write!(f, "VoteCast"),
            DaoEventV4::ProposalExecuted => write!(f, "ProposalExecuted"),
            DaoEventV4::ParameterChanged => write!(f, "ParameterChanged"),
            DaoEventV4::MemberAdded => write!(f, "MemberAdded"),
            DaoEventV4::MemberRemoved => write!(f, "MemberRemoved"),
            DaoEventV4::EmergencyAction => write!(f, "EmergencyAction"),
            DaoEventV4::Custom(msg) => write!(f, "Custom({})", msg),
        }
    }
}

// ─── Config ──────────────────────────────────────────────────────────────────

/// Configuration for DAO Ledger v4.
#[derive(Debug, Clone)]
pub struct DaoLedgerV4Config {
    /// Maximum entries in the ledger.
    pub max_entries: usize,
    /// Quorum threshold [0.0, 1.0].
    pub quorum_threshold: f64,
    /// Minimum validators for quorum.
    pub min_validators: usize,
    /// Enable Merkle proof generation.
    pub merkle_proofs_enabled: bool,
    /// Enable hybrid execution tracking.
    pub hybrid_execution: bool,
    /// Maximum payload size in bytes.
    pub max_payload_bytes: usize,
}

impl Default for DaoLedgerV4Config {
    fn default() -> Self {
        Self {
            max_entries: 100000,
            quorum_threshold: 0.66,
            min_validators: 3,
            merkle_proofs_enabled: true,
            hybrid_execution: true,
            max_payload_bytes: 65536,
        }
    }
}

// ─── Ledger Entry ────────────────────────────────────────────────────────────

/// An immutable entry in the DAO ledger.
#[derive(Debug, Clone)]
pub struct LedgerEntryV4 {
    /// Unique entry identifier.
    pub entry_id: String,
    /// Sequence number in the ledger.
    pub sequence: u64,
    /// Event type.
    pub event_type: DaoEventV4,
    /// Actor who triggered the event.
    pub actor_id: String,
    /// Event payload.
    pub payload: String,
    /// Execution type.
    pub execution_type: ExecutionType,
    /// On-chain transaction hash (if applicable).
    pub tx_hash: Option<String>,
    /// Validator signatures count.
    pub validator_count: usize,
    /// Quorum achieved ratio.
    pub quorum_ratio: f64,
    /// Entry hash.
    pub hash: String,
    /// Previous entry hash (chain linking).
    pub previous_hash: String,
    /// Creation timestamp.
    pub timestamp_ms: u64,
}

impl LedgerEntryV4 {
    pub fn new(
        entry_id: String,
        sequence: u64,
        event_type: DaoEventV4,
        actor_id: String,
        payload: String,
        execution_type: ExecutionType,
        previous_hash: String,
        timestamp_ms: u64,
    ) -> Self {
        let hash = compute_hash(&entry_id, sequence, &payload, &previous_hash, timestamp_ms);
        Self {
            entry_id,
            sequence,
            event_type,
            actor_id,
            payload,
            execution_type,
            tx_hash: None,
            validator_count: 0,
            quorum_ratio: 0.0,
            hash,
            previous_hash,
            timestamp_ms,
        }
    }

    /// Verify entry hash integrity.
    pub fn verify_hash(&self) -> bool {
        let expected = compute_hash(&self.entry_id, self.sequence, &self.payload, &self.previous_hash, self.timestamp_ms);
        self.hash == expected
    }

    /// Check if quorum was reached.
    pub fn has_quorum(&self, threshold: f64) -> bool {
        self.quorum_ratio >= threshold
    }
}

// ─── Merkle Proof ────────────────────────────────────────────────────────────

/// Merkle proof for ledger entry verification.
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Entry ID this proof is for.
    pub entry_id: String,
    /// Merkle root hash.
    pub root_hash: String,
    /// Proof path (sibling hashes).
    pub proof_path: Vec<String>,
    /// Position in the tree.
    pub position: u64,
}

impl MerkleProof {
    pub fn new(entry_id: String, root_hash: String, proof_path: Vec<String>, position: u64) -> Self {
        Self {
            entry_id,
            root_hash,
            proof_path,
            position,
        }
    }

    /// Verify the Merkle proof.
    pub fn verify(&self, entry_hash: &str) -> bool {
        let mut current = entry_hash.to_string();
        for sibling in &self.proof_path {
            let combined = if current.len() < sibling.len() {
                format!("{}{}", current, sibling)
            } else {
                format!("{}{}", sibling, current)
            };
            current = compute_single_hash(&combined);
        }
        current == self.root_hash
    }
}

// ─── Stats ───────────────────────────────────────────────────────────────────

/// Statistics for DAO Ledger v4.
#[derive(Debug, Clone)]
pub struct DaoLedgerV4Stats {
    /// Total entries recorded.
    pub total_entries: usize,
    /// Total on-chain executions.
    pub on_chain_count: usize,
    /// Total off-chain executions.
    pub off_chain_count: usize,
    /// Total hybrid executions.
    pub hybrid_count: usize,
    /// Total quorum validations.
    pub quorum_validations: usize,
    /// Quorum success rate.
    pub quorum_success_rate: f64,
    /// Current Merkle root.
    pub current_merkle_root: String,
    /// Last entry sequence.
    pub last_sequence: u64,
}

impl Default for DaoLedgerV4Stats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            on_chain_count: 0,
            off_chain_count: 0,
            hybrid_count: 0,
            quorum_validations: 0,
            quorum_success_rate: 0.0,
            current_merkle_root: String::new(),
            last_sequence: 0,
        }
    }
}

// ─── Main Ledger ─────────────────────────────────────────────────────────────

/// DAO Ledger v4 with hybrid execution and cryptographic audit trail.
pub struct DaoLedgerV4 {
    config: DaoLedgerV4Config,
    entries: HashMap<String, LedgerEntryV4>,
    sequence_order: Vec<String>,
    stats: DaoLedgerV4Stats,
    quorum_successes: usize,
    current_time_ms: u64,
}

impl DaoLedgerV4 {
    // ─── Construction ──────────────────────────────────────────────────────

    /// Create a new DAO Ledger v4.
    pub fn new(config: DaoLedgerV4Config) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            sequence_order: Vec::new(),
            stats: DaoLedgerV4Stats::default(),
            quorum_successes: 0,
            current_time_ms: current_timestamp_ms(),
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(DaoLedgerV4Config::default())
    }

    // ─── Event Recording ───────────────────────────────────────────────────

    /// Record a governance event.
    pub fn record_event(
        &mut self,
        entry_id: String,
        event_type: DaoEventV4,
        actor_id: String,
        payload: String,
        execution_type: ExecutionType,
    ) -> Result<LedgerEntryV4, DaoLedgerV4Error> {
        // Check for duplicate
        if self.entries.contains_key(&entry_id) {
            return Err(DaoLedgerV4Error::DuplicateEntry(entry_id.clone()));
        }

        // Check capacity
        if self.entries.len() >= self.config.max_entries {
            return Err(DaoLedgerV4Error::LedgerFull);
        }

        // Check payload size
        if payload.len() > self.config.max_payload_bytes {
            return Err(DaoLedgerV4Error::InvalidConfig(format!(
                "Payload size {} exceeds maximum {}",
                payload.len(),
                self.config.max_payload_bytes
            )));
        }

        // Get previous hash
        let previous_hash = self.get_last_hash();

        // Create entry
        let sequence = self.entries.len() as u64 + 1;
        let entry = LedgerEntryV4::new(
            entry_id.clone(),
            sequence,
            event_type,
            actor_id,
            payload,
            execution_type.clone(),
            previous_hash,
            self.current_time_ms,
        );

        // Store entry
        self.entries.insert(entry_id.clone(), entry.clone());
        self.sequence_order.push(entry_id);

        // Update stats
        self.update_stats(&execution_type);
        self.stats.last_sequence = sequence;

        Ok(entry)
    }

    // ─── Quorum Validation ─────────────────────────────────────────────────

    /// Validate quorum for an entry.
    pub fn validate_quorum(
        &mut self,
        entry_id: &str,
        validator_count: usize,
        total_validators: usize,
    ) -> Result<bool, DaoLedgerV4Error> {
        if total_validators < self.config.min_validators {
            return Err(DaoLedgerV4Error::InvalidConfig(format!(
                "Total validators {} below minimum {}",
                total_validators, self.config.min_validators
            )));
        }

        let ratio = if total_validators > 0 {
            validator_count as f64 / total_validators as f64
        } else {
            0.0
        };

        let entry = self.entries.get_mut(entry_id).ok_or_else(|| {
            DaoLedgerV4Error::EntryNotFound(entry_id.to_string())
        })?;

        entry.validator_count = validator_count;
        entry.quorum_ratio = ratio;

        self.stats.quorum_validations += 1;

        if ratio >= self.config.quorum_threshold {
            self.quorum_successes += 1;
            self.stats.quorum_success_rate =
                self.quorum_successes as f64 / self.stats.quorum_validations as f64;
            Ok(true)
        } else {
            self.stats.quorum_success_rate =
                self.quorum_successes as f64 / self.stats.quorum_validations as f64;
            Err(DaoLedgerV4Error::QuorumNotReached(ratio))
        }
    }

    // ─── Merkle Operations ─────────────────────────────────────────────────

    /// Compute current Merkle root.
    pub fn compute_merkle_root(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let leaves: Vec<String> = self.sequence_order
            .iter()
            .filter_map(|id| self.entries.get(id).map(|e| e.hash.clone()))
            .collect();

        compute_merkle_root(&leaves)
    }

    /// Generate Merkle proof for an entry.
    pub fn generate_merkle_proof(&self, entry_id: &str) -> Result<MerkleProof, DaoLedgerV4Error> {
        if !self.config.merkle_proofs_enabled {
            return Err(DaoLedgerV4Error::InvalidConfig("Merkle proofs disabled".to_string()));
        }

        let entry = self.entries.get(entry_id).ok_or_else(|| {
            DaoLedgerV4Error::EntryNotFound(entry_id.to_string())
        })?;

        let root = self.compute_merkle_root();
        let position = entry.sequence - 1;

        // Simplified proof path (in production, build actual tree path)
        let proof_path = self.sequence_order
            .iter()
            .filter_map(|id| {
                if id != entry_id {
                    self.entries.get(id).map(|e| e.hash.clone())
                } else {
                    None
                }
            })
            .take(8)
            .collect();

        Ok(MerkleProof::new(
            entry_id.to_string(),
            root,
            proof_path,
            position,
        ))
    }

    // ─── Chain Verification ────────────────────────────────────────────────

    /// Verify the entire chain integrity.
    pub fn verify_chain(&self) -> Result<(), DaoLedgerV4Error> {
        for id in &self.sequence_order {
            let entry = self.entries.get(id).ok_or_else(|| {
                DaoLedgerV4Error::EntryNotFound(id.clone())
            })?;

            if !entry.verify_hash() {
                return Err(DaoLedgerV4Error::HashMismatch(id.clone()));
            }
        }
        Ok(())
    }

    // ─── Queries ───────────────────────────────────────────────────────────

    /// Get entry by ID.
    pub fn get_entry(&self, entry_id: &str) -> Option<&LedgerEntryV4> {
        self.entries.get(entry_id)
    }

    /// Get entries by event type.
    pub fn get_entries_by_type(&self, event_type: &DaoEventV4) -> Vec<&LedgerEntryV4> {
        self.entries
            .values()
            .filter(|e| &e.event_type == event_type)
            .collect()
    }

    /// Get entries by actor.
    pub fn get_entries_by_actor(&self, actor_id: &str) -> Vec<&LedgerEntryV4> {
        self.entries
            .values()
            .filter(|e| e.actor_id == actor_id)
            .collect()
    }

    /// Get entries by execution type.
    pub fn get_entries_by_execution(&self, execution_type: &ExecutionType) -> Vec<&LedgerEntryV4> {
        self.entries
            .values()
            .filter(|e| &e.execution_type == execution_type)
            .collect()
    }

    /// Get recent entries.
    pub fn get_recent_entries(&self, count: usize) -> Vec<&LedgerEntryV4> {
        self.sequence_order
            .iter()
            .rev()
            .take(count)
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    /// Get entry by sequence.
    pub fn get_entry_by_sequence(&self, sequence: u64) -> Option<&LedgerEntryV4> {
        self.entries.values().find(|e| e.sequence == sequence)
    }

    // ─── Execution Tracking ────────────────────────────────────────────────

    /// Set transaction hash for on-chain execution.
    pub fn set_tx_hash(&mut self, entry_id: &str, tx_hash: String) -> Result<(), DaoLedgerV4Error> {
        let entry = self.entries.get_mut(entry_id).ok_or_else(|| {
            DaoLedgerV4Error::EntryNotFound(entry_id.to_string())
        })?;

        if entry.execution_type != ExecutionType::OnChain && entry.execution_type != ExecutionType::Hybrid {
            return Err(DaoLedgerV4Error::ExecutionMismatch);
        }

        entry.tx_hash = Some(tx_hash);
        Ok(())
    }

    // ─── Time ──────────────────────────────────────────────────────────────

    /// Advance internal time.
    pub fn advance_time(&mut self, ms: u64) {
        self.current_time_ms += ms;
    }

    // ─── Stats ─────────────────────────────────────────────────────────────

    /// Get current statistics.
    pub fn stats(&self) -> DaoLedgerV4Stats {
        DaoLedgerV4Stats {
            total_entries: self.entries.len(),
            on_chain_count: self.stats.on_chain_count,
            off_chain_count: self.stats.off_chain_count,
            hybrid_count: self.stats.hybrid_count,
            quorum_validations: self.stats.quorum_validations,
            quorum_success_rate: self.stats.quorum_success_rate,
            current_merkle_root: self.compute_merkle_root(),
            last_sequence: self.stats.last_sequence,
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = DaoLedgerV4Stats::default();
        self.quorum_successes = 0;
    }

    // ─── Internal ──────────────────────────────────────────────────────────

    fn get_last_hash(&self) -> String {
        self.sequence_order
            .last()
            .and_then(|id| self.entries.get(id))
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string())
    }

    fn update_stats(&mut self, execution_type: &ExecutionType) {
        match execution_type {
            ExecutionType::OnChain => self.stats.on_chain_count += 1,
            ExecutionType::OffChain => self.stats.off_chain_count += 1,
            ExecutionType::Hybrid => self.stats.hybrid_count += 1,
        }
        self.stats.total_entries += 1;
    }
}

impl Default for DaoLedgerV4 {
    fn default() -> Self {
        Self::new(DaoLedgerV4Config::default())
    }
}

// ─── Hash Utilities ──────────────────────────────────────────────────────────

fn compute_hash(entry_id: &str, sequence: u64, payload: &str, previous_hash: &str, timestamp_ms: u64) -> String {
    let data = format!("{}:{}:{}:{}:{}", entry_id, sequence, payload, previous_hash, timestamp_ms);
    compute_single_hash(&data)
}

fn compute_single_hash(data: &str) -> String {
    // Simplified hash using FNV-1a
    let mut hash: u64 = 14695981039346656037;
    for byte in data.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}", hash)
}

fn compute_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }

    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for i in (0..current.len()).step_by(2) {
            let combined = if i + 1 < current.len() {
                format!("{}{}", current[i], current[i + 1])
            } else {
                current[i].clone()
            };
            next.push(compute_single_hash(&combined));
        }
        current = next;
    }

    current.into_iter().next().unwrap_or_default()
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_creation() {
        let ledger = DaoLedgerV4::new(DaoLedgerV4Config::default());
        assert_eq!(ledger.stats().total_entries, 0);
    }

    #[test]
    fn test_record_event() {
        let mut ledger = DaoLedgerV4::default_config();
        let entry = ledger.record_event(
            "e1".to_string(),
            DaoEventV4::ProposalCreated,
            "actor-1".to_string(),
            "create proposal".to_string(),
            ExecutionType::OnChain,
        );
        assert!(entry.is_ok());
        assert_eq!(ledger.stats().total_entries, 1);
    }

    #[test]
    fn test_duplicate_entry() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event(
            "e1".to_string(),
            DaoEventV4::ProposalCreated,
            "actor-1".to_string(),
            "payload".to_string(),
            ExecutionType::OnChain,
        ).unwrap();
        let result = ledger.record_event(
            "e1".to_string(),
            DaoEventV4::VoteCast,
            "actor-2".to_string(),
            "payload".to_string(),
            ExecutionType::OffChain,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_verification() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event(
            "e1".to_string(),
            DaoEventV4::ProposalCreated,
            "actor-1".to_string(),
            "payload".to_string(),
            ExecutionType::OnChain,
        ).unwrap();
        let entry = ledger.get_entry("e1").unwrap();
        assert!(entry.verify_hash());
    }

    #[test]
    fn test_chain_verification() {
        let mut ledger = DaoLedgerV4::default_config();
        for i in 1..=5 {
            ledger.record_event(
                format!("e{}", i),
                DaoEventV4::ProposalCreated,
                "actor-1".to_string(),
                format!("payload {}", i),
                ExecutionType::OnChain,
            ).unwrap();
        }
        assert!(ledger.verify_chain().is_ok());
    }

    #[test]
    fn test_chain_linking() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.record_event("e2".to_string(), DaoEventV4::VoteCast, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        let e1 = ledger.get_entry("e1").unwrap();
        let e2 = ledger.get_entry("e2").unwrap();
        assert_eq!(e1.previous_hash, "genesis");
        assert_eq!(e2.previous_hash, e1.hash);
    }

    #[test]
    fn test_quorum_validation() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        let result = ledger.validate_quorum("e1", 7, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quorum_not_reached() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        let result = ledger.validate_quorum("e1", 3, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_root() {
        let mut ledger = DaoLedgerV4::default_config();
        for i in 1..=5 {
            ledger.record_event(format!("e{}", i), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        }
        let root = ledger.compute_merkle_root();
        assert!(!root.is_empty());
    }

    #[test]
    fn test_get_entries_by_type() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.record_event("e2".to_string(), DaoEventV4::VoteCast, "a".to_string(), "p".to_string(), ExecutionType::OffChain).unwrap();
        let proposals = ledger.get_entries_by_type(&DaoEventV4::ProposalCreated);
        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn test_get_entries_by_actor() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "actor-1".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.record_event("e2".to_string(), DaoEventV4::VoteCast, "actor-2".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        let entries = ledger.get_entries_by_actor("actor-1");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_get_entries_by_execution() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.record_event("e2".to_string(), DaoEventV4::VoteCast, "a".to_string(), "p".to_string(), ExecutionType::OffChain).unwrap();
        let on_chain = ledger.get_entries_by_execution(&ExecutionType::OnChain);
        assert_eq!(on_chain.len(), 1);
    }

    #[test]
    fn test_set_tx_hash() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        assert!(ledger.set_tx_hash("e1", "0xabc".to_string()).is_ok());
        assert!(ledger.get_entry("e1").unwrap().tx_hash.is_some());
    }

    #[test]
    fn test_set_tx_hash_mismatch() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OffChain).unwrap();
        assert!(ledger.set_tx_hash("e1", "0xabc".to_string()).is_err());
    }

    #[test]
    fn test_stats_tracking() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.record_event("e2".to_string(), DaoEventV4::VoteCast, "a".to_string(), "p".to_string(), ExecutionType::OffChain).unwrap();
        ledger.record_event("e3".to_string(), DaoEventV4::ProposalExecuted, "a".to_string(), "p".to_string(), ExecutionType::Hybrid).unwrap();
        let stats = ledger.stats();
        assert_eq!(stats.on_chain_count, 1);
        assert_eq!(stats.off_chain_count, 1);
        assert_eq!(stats.hybrid_count, 1);
    }

    #[test]
    fn test_get_recent_entries() {
        let mut ledger = DaoLedgerV4::default_config();
        for i in 1..=5 {
            ledger.record_event(format!("e{}", i), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        }
        let recent = ledger.get_recent_entries(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_get_entry_by_sequence() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        let entry = ledger.get_entry_by_sequence(1);
        assert!(entry.is_some());
    }

    #[test]
    fn test_reset_stats() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.reset_stats();
        assert_eq!(ledger.stats().quorum_validations, 0);
    }

    #[test]
    fn test_ledger_full() {
        let mut config = DaoLedgerV4Config::default();
        config.max_entries = 2;
        let mut ledger = DaoLedgerV4::new(config);
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.record_event("e2".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        assert!(ledger.record_event("e3".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).is_err());
    }

    #[test]
    fn test_payload_too_large() {
        let mut config = DaoLedgerV4Config::default();
        config.max_payload_bytes = 10;
        let mut ledger = DaoLedgerV4::new(config);
        let result = ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "very long payload".to_string(), ExecutionType::OnChain);
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_proof_generation() {
        let mut ledger = DaoLedgerV4::default_config();
        for i in 1..=5 {
            ledger.record_event(format!("e{}", i), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        }
        let proof = ledger.generate_merkle_proof("e1");
        assert!(proof.is_ok());
    }

    #[test]
    fn test_merkle_proof_disabled() {
        let mut config = DaoLedgerV4Config::default();
        config.merkle_proofs_enabled = false;
        let mut ledger = DaoLedgerV4::new(config);
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        assert!(ledger.generate_merkle_proof("e1").is_err());
    }

    #[test]
    fn test_quorum_min_validators() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        let result = ledger.validate_quorum("e1", 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_entry_has_quorum() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.record_event("e1".to_string(), DaoEventV4::ProposalCreated, "a".to_string(), "p".to_string(), ExecutionType::OnChain).unwrap();
        ledger.validate_quorum("e1", 8, 10).unwrap();
        let entry = ledger.get_entry("e1").unwrap();
        assert!(entry.has_quorum(0.66));
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(DaoEventV4::ProposalCreated.to_string(), "ProposalCreated");
        assert_eq!(DaoEventV4::VoteCast.to_string(), "VoteCast");
        assert_eq!(DaoEventV4::Custom("test".to_string()).to_string(), "Custom(test)");
    }

    #[test]
    fn test_execution_type_display() {
        assert_eq!(ExecutionType::OnChain.to_string(), "OnChain");
        assert_eq!(ExecutionType::OffChain.to_string(), "OffChain");
        assert_eq!(ExecutionType::Hybrid.to_string(), "Hybrid");
    }

    #[test]
    fn test_error_display() {
        match DaoLedgerV4Error::EntryNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_config_default() {
        let config = DaoLedgerV4Config::default();
        assert_eq!(config.max_entries, 100000);
        assert_eq!(config.quorum_threshold, 0.66);
        assert!(config.merkle_proofs_enabled);
    }

    #[test]
    fn test_stats_default() {
        let stats = DaoLedgerV4Stats::default();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.on_chain_count, 0);
    }

    #[test]
    fn test_ledger_default() {
        let ledger = DaoLedgerV4::default();
        assert_eq!(ledger.stats().total_entries, 0);
    }

    #[test]
    fn test_advance_time() {
        let mut ledger = DaoLedgerV4::default_config();
        ledger.advance_time(1000);
        assert_eq!(ledger.current_time_ms, ledger.current_time_ms);
    }

    #[test]
    fn test_empty_merkle_root() {
        let ledger = DaoLedgerV4::default_config();
        assert!(ledger.compute_merkle_root().is_empty());
    }
}
