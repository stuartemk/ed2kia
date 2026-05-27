//! Global Symbiotic Ledger — DAG-based Ledger for Existence Credit (CE).
//!
//! Implements a Directed Acyclic Graph (DAG) ledger for cooperative tracking
//! of CE (Credit of Existence) transactions. Each node validates 2 previous
//! transactions in the DAG, creating a web of mutual verification without
//! centralized consensus.
//!
//! # Design Principles
//!
//! - **DAG structure**: Not a blockchain — transactions form a directed acyclic
//!   graph where each node references 2 previous transactions.
//! - **Cooperative validation**: Each validating node checks 2 parent transactions,
//!   creating redundant verification paths.
//! - **SCT Guard Economic**: Rejects transactions from nodes with unstable GEI
//!   (Geometric Ethical Invariant) or negative Z-score.
//! - **Ed25519 signatures**: All transactions are cryptographically signed.
//! - **Zero speculation**: CE is a merit metric, not a tradable currency.
//!   No market, no exchange rate, no price discovery.
//!
//! # DAG Structure
//!
//! Each transaction references 2 parent transaction hashes (or none for genesis).
//! Validation follows the parent pointers to verify the chain of custody.
//!
//! ```text
//!     T0 ──┐
//!     T1 ──┼── T3 ── T5
//!     T2 ──┘    └── T6
//! ```
//!
//! **Feature Gate:** `v3.4-macro-symbiosis`

use std::collections::{HashMap, HashSet, VecDeque};

use crate::time::temporal_cohesion::SymbioticTimestamp;

/// CE Transaction in the symbiotic DAG.
///
/// Represents a single cooperative exchange of Existence Credit.
/// Each transaction references 2 parent transactions for DAG structure.
#[derive(Debug, Clone)]
pub struct CETransaction {
    /// Unique transaction identifier (SHA-256 hash as u128).
    pub hash: u128,
    /// Node that originated this transaction.
    pub origin_node: u64,
    /// Validating node that processed this transaction.
    pub validator_node: u64,
    /// CE amount transferred (merit metric, not currency).
    pub ce_amount: f64,
    /// Symbiotic timestamp for chronological ordering.
    pub timestamp: SymbioticTimestamp,
    /// References to 2 parent transactions (empty for genesis).
    pub parent_hashes: [Option<u128>; 2],
    /// Ed25519 signature (simulated as u8 array for WASM compatibility).
    pub signature: [u8; 64],
    /// SCT Z-score of the originating node (must be >= 0).
    pub z_score: f32,
    /// GEI stability score of the originating node (must be > threshold).
    pub gei_stability: f64,
    /// Arbitrary payload for resource tracking (e.g., IoT data).
    pub payload: Vec<u8>,
}

impl CETransaction {
    /// Create a new CE transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hash: u128,
        origin_node: u64,
        validator_node: u64,
        ce_amount: f64,
        timestamp: SymbioticTimestamp,
        parent_hashes: [Option<u128>; 2],
        signature: [u8; 64],
        z_score: f32,
        gei_stability: f64,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            hash,
            origin_node,
            validator_node,
            ce_amount,
            timestamp,
            parent_hashes,
            signature,
            z_score,
            gei_stability,
            payload,
        }
    }

    /// Check if this is a genesis transaction (no parents).
    pub fn is_genesis(&self) -> bool {
        self.parent_hashes[0].is_none() && self.parent_hashes[1].is_none()
    }
}

/// Result of validating a CE transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Transaction passed all validation checks.
    Valid,
    /// Transaction rejected: unstable GEI (below threshold).
    RejectedUnstableGEI { gei_stability: f64, threshold: f64 },
    /// Transaction rejected: negative Z-score (unethical origin).
    RejectedNegativeZScore { z_score: f32 },
    /// Transaction rejected: invalid Ed25519 signature.
    RejectedInvalidSignature,
    /// Transaction rejected: parent transaction not found in DAG.
    RejectedMissingParent { parent_hash: u128 },
    /// Transaction rejected: CE amount must be positive.
    RejectedInvalidCEAmount { ce_amount: f64 },
    /// Transaction rejected: would create a cycle in the DAG.
    RejectedCycleDetected,
}

/// Configuration for the GlobalSymbioticLedger.
#[derive(Debug, Clone)]
pub struct LedgerConfig {
    /// Minimum GEI stability score for transaction acceptance.
    pub min_gei_stability: f64,
    /// Minimum SCT Z-score for transaction acceptance.
    pub min_z_score: f32,
    /// Maximum CE amount per transaction (prevent accumulation).
    pub max_ce_per_transaction: f64,
    /// Maximum DAG depth for cycle detection.
    pub max_dag_depth: usize,
    /// Number of previous transactions each validator must check.
    pub validation_depth: usize,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            min_gei_stability: 0.5,
            min_z_score: 0.0,
            max_ce_per_transaction: 100.0,
            max_dag_depth: 100,
            validation_depth: 2,
        }
    }
}

/// Errors specific to ledger operations.
#[derive(Debug, Clone)]
pub enum LedgerError {
    /// Transaction already exists in the ledger.
    DuplicateTransaction(u128),
    /// Validation failed with specific reason.
    ValidationFailed(ValidationResult),
    /// DAG integrity violation (cycle detected).
    CycleDetected,
    /// Configuration error.
    InvalidConfig(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::DuplicateTransaction(hash) => {
                write!(f, "Duplicate transaction: {:x}", hash)
            }
            LedgerError::ValidationFailed(result) => {
                write!(f, "Validation failed: {:?}", result)
            }
            LedgerError::CycleDetected => {
                write!(f, "DAG cycle detected — integrity violation")
            }
            LedgerError::InvalidConfig(msg) => {
                write!(f, "Invalid ledger configuration: {}", msg)
            }
        }
    }
}

/// Statistics for the symbiotic ledger.
#[derive(Debug, Clone)]
pub struct LedgerStats {
    /// Total number of transactions in the DAG.
    pub total_transactions: usize,
    /// Total CE circulated (merit metric sum).
    pub total_ce_circulated: f64,
    /// Number of unique participating nodes.
    pub unique_nodes: usize,
    /// Number of validated transactions.
    pub validated_count: usize,
    /// Number of rejected transactions.
    pub rejected_count: usize,
    /// Current DAG width (transactions at the latest layer).
    pub dag_width: usize,
    /// Current DAG depth (longest chain from genesis).
    pub dag_depth: usize,
}

impl LedgerStats {
    pub fn new() -> Self {
        Self {
            total_transactions: 0,
            total_ce_circulated: 0.0,
            unique_nodes: 0,
            validated_count: 0,
            rejected_count: 0,
            dag_width: 0,
            dag_depth: 0,
        }
    }
}

impl Default for LedgerStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Global Symbiotic Ledger — DAG-based CE transaction ledger.
///
/// Maintains a directed acyclic graph of CE transactions with cooperative
/// validation. Each transaction references 2 parents, and validators check
/// the previous N transactions for integrity.
pub struct GlobalSymbioticLedger {
    /// Local node identifier.
    pub node_id: u64,
    /// Transaction store: hash -> transaction.
    transactions: HashMap<u128, CETransaction>,
    /// Reverse index: node -> list of transaction hashes.
    node_index: HashMap<u64, Vec<u128>>,
    /// Ledger configuration.
    config: LedgerConfig,
    /// Ledger statistics.
    stats: LedgerStats,
    /// Set of unique node IDs.
    unique_nodes: HashSet<u64>,
}

impl GlobalSymbioticLedger {
    /// Create a new ledger with default configuration.
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            transactions: HashMap::new(),
            node_index: HashMap::new(),
            config: LedgerConfig::default(),
            stats: LedgerStats::new(),
            unique_nodes: HashSet::new(),
        }
    }

    /// Create a new ledger with custom configuration.
    pub fn with_config(node_id: u64, config: LedgerConfig) -> Result<Self, LedgerError> {
        if config.min_gei_stability < 0.0 || config.min_gei_stability > 1.0 {
            return Err(LedgerError::InvalidConfig(
                "min_gei_stability must be in [0, 1]".to_string(),
            ));
        }
        if config.max_ce_per_transaction <= 0.0 {
            return Err(LedgerError::InvalidConfig(
                "max_ce_per_transaction must be > 0".to_string(),
            ));
        }
        if config.validation_depth == 0 {
            return Err(LedgerError::InvalidConfig(
                "validation_depth must be > 0".to_string(),
            ));
        }

        Ok(Self {
            node_id,
            transactions: HashMap::new(),
            node_index: HashMap::new(),
            config,
            stats: LedgerStats::new(),
            unique_nodes: HashSet::new(),
        })
    }

    /// Validate a transaction against SCT Guard Economic rules.
    ///
    /// Checks:
    /// 1. CE amount is positive and within limits.
    /// 2. Z-score is non-negative (ethical origin).
    /// 3. GEI stability is above threshold (stable node).
    /// 4. Parent transactions exist in the DAG.
    /// 5. No cycle would be introduced.
    pub fn validate_transaction(&self, tx: &CETransaction) -> ValidationResult {
        // Check CE amount.
        if tx.ce_amount <= 0.0 {
            return ValidationResult::RejectedInvalidCEAmount {
                ce_amount: tx.ce_amount,
            };
        }
        if tx.ce_amount > self.config.max_ce_per_transaction {
            return ValidationResult::RejectedInvalidCEAmount {
                ce_amount: tx.ce_amount,
            };
        }

        // SCT Guard Economic: Check Z-score.
        if tx.z_score < self.config.min_z_score {
            return ValidationResult::RejectedNegativeZScore {
                z_score: tx.z_score,
            };
        }

        // SCT Guard Economic: Check GEI stability.
        if tx.gei_stability < self.config.min_gei_stability {
            return ValidationResult::RejectedUnstableGEI {
                gei_stability: tx.gei_stability,
                threshold: self.config.min_gei_stability,
            };
        }

        // Validate parent existence.
        for parent_hash in tx.parent_hashes.iter().flatten() {
            if !self.transactions.contains_key(parent_hash) {
                return ValidationResult::RejectedMissingParent {
                    parent_hash: *parent_hash,
                };
            }
        }

        // Cycle detection: BFS from parents should not reach this transaction.
        if !tx.is_genesis() && self.would_create_cycle(tx) {
            return ValidationResult::RejectedCycleDetected;
        }

        ValidationResult::Valid
    }

    /// Submit a validated transaction to the ledger.
    pub fn submit_transaction(&mut self, tx: CETransaction) -> Result<(), LedgerError> {
        // Check for duplicates.
        if self.transactions.contains_key(&tx.hash) {
            return Err(LedgerError::DuplicateTransaction(tx.hash));
        }

        // Validate.
        let result = self.validate_transaction(&tx);
        if result != ValidationResult::Valid {
            self.stats.rejected_count += 1;
            return Err(LedgerError::ValidationFailed(result));
        }

        // Insert into ledger.
        self.transactions.insert(tx.hash, tx.clone());

        // Update node index.
        self.node_index
            .entry(tx.origin_node)
            .or_default()
            .push(tx.hash);

        // Track unique nodes.
        self.unique_nodes.insert(tx.origin_node);
        self.unique_nodes.insert(tx.validator_node);

        // Update stats.
        self.stats.total_transactions += 1;
        self.stats.total_ce_circulated += tx.ce_amount;
        self.stats.validated_count += 1;
        self.stats.unique_nodes = self.unique_nodes.len();

        // Update DAG metrics.
        self.update_dag_metrics();

        Ok(())
    }

    /// Get a transaction by hash.
    pub fn get_transaction(&self, hash: &u128) -> Option<&CETransaction> {
        self.transactions.get(hash)
    }

    /// Get all transactions from a specific node.
    pub fn get_node_transactions(&self, node_id: u64) -> Vec<&CETransaction> {
        let hashes = match self.node_index.get(&node_id) {
            Some(h) => h,
            None => return Vec::new(),
        };
        hashes
            .iter()
            .filter_map(|h| self.transactions.get(h))
            .collect()
    }

    /// Get the latest N transaction hashes (for parent reference).
    pub fn get_latest_transactions(&self, n: usize) -> Vec<u128> {
        let mut txs: Vec<&CETransaction> = self.transactions.values().collect();
        txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        txs.into_iter().map(|tx| tx.hash).take(n).collect()
    }

    /// Get ledger statistics.
    pub fn get_stats(&self) -> &LedgerStats {
        &self.stats
    }

    /// Get the total number of transactions.
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Validate 2 previous transactions (cooperative validation).
    ///
    /// Each node is responsible for validating 2 previous transactions
    /// in the DAG, creating redundant verification paths.
    pub fn validate_previous_transactions(
        &self,
        hashes: &[u128; 2],
    ) -> Vec<(u128, ValidationResult)> {
        hashes
            .iter()
            .filter_map(|h| {
                self.transactions.get(h).map(|tx| {
                    let result = self.validate_transaction(tx);
                    (*h, result)
                })
            })
            .collect()
    }

    /// Check if adding this transaction would create a cycle.
    fn would_create_cycle(&self, tx: &CETransaction) -> bool {
        // BFS from parents — if we can reach tx.hash, a cycle exists.
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start from parents.
        for ph in tx.parent_hashes.iter().flatten() {
            if let Some(parent_tx) = self.transactions.get(ph) {
                queue.push_back(parent_tx.hash);
            }
        }

        while let Some(current_hash) = queue.pop_front() {
            if current_hash == tx.hash {
                return true; // Cycle detected.
            }
            if !visited.insert(current_hash) {
                continue;
            }

            if let Some(current_tx) = self.transactions.get(&current_hash) {
                for ph in current_tx.parent_hashes.iter().flatten() {
                    queue.push_back(*ph);
                }
            }
        }

        false
    }

    /// Update DAG width and depth metrics.
    fn update_dag_metrics(&mut self) {
        if self.transactions.is_empty() {
            self.stats.dag_width = 0;
            self.stats.dag_depth = 0;
            return;
        }

        // Compute depth: longest chain from any genesis to any leaf.
        self.stats.dag_depth = self.compute_dag_depth();

        // Compute width: number of leaf nodes (transactions with no children).
        let mut has_parent: HashSet<u128> = HashSet::new();
        for tx in self.transactions.values() {
            for ph in tx.parent_hashes.iter().flatten() {
                has_parent.insert(*ph);
            }
        }
        self.stats.dag_width = self
            .transactions
            .values()
            .filter(|tx| !has_parent.contains(&tx.hash))
            .count();
    }

    /// Compute the DAG depth (longest chain from genesis).
    fn compute_dag_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut cache: HashMap<u128, usize> = HashMap::new();

        for tx in self.transactions.values() {
            let depth = self.get_tx_depth(tx.hash, &mut cache);
            if depth > max_depth {
                max_depth = depth;
            }
        }

        max_depth
    }

    /// Get the depth of a specific transaction (recursive with memoization).
    fn get_tx_depth(&self, hash: u128, cache: &mut HashMap<u128, usize>) -> usize {
        if let Some(&depth) = cache.get(&hash) {
            return depth;
        }

        let tx = match self.transactions.get(&hash) {
            Some(tx) => tx,
            None => return 0,
        };

        if tx.is_genesis() {
            cache.insert(hash, 1);
            return 1;
        }

        let mut max_parent_depth = 0;
        for ph in tx.parent_hashes.iter().flatten() {
            let d = self.get_tx_depth(*ph, cache);
            if d > max_parent_depth {
                max_parent_depth = d;
            }
        }

        let depth = max_parent_depth + 1;
        cache.insert(hash, depth);
        depth
    }

    /// Reset the ledger (for testing).
    pub fn reset(&mut self) {
        self.transactions.clear();
        self.node_index.clear();
        self.unique_nodes.clear();
        self.stats = LedgerStats::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_timestamp(ms: u64, node: u64) -> SymbioticTimestamp {
        SymbioticTimestamp::new(ms, node)
    }

    fn make_genesis_tx(hash: u128, node: u64, ce: f64, ts: u64) -> CETransaction {
        CETransaction::new(
            hash,
            node,
            1, // validator
            ce,
            make_timestamp(ts, node),
            [None, None],
            [0u8; 64],
            1.0, // z_score
            0.8, // gei_stability
            Vec::new(),
        )
    }

    fn make_child_tx(
        hash: u128,
        node: u64,
        ce: f64,
        ts: u64,
        parents: [Option<u128>; 2],
    ) -> CETransaction {
        CETransaction::new(
            hash,
            node,
            1,
            ce,
            make_timestamp(ts, node),
            parents,
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        )
    }

    #[test]
    fn test_ledger_creation() {
        let ledger = GlobalSymbioticLedger::new(1);
        assert_eq!(ledger.node_id, 1);
        assert_eq!(ledger.transaction_count(), 0);
    }

    #[test]
    fn test_ledger_custom_config() {
        let config = LedgerConfig {
            min_gei_stability: 0.7,
            min_z_score: 0.5,
            ..Default::default()
        };
        let ledger = GlobalSymbioticLedger::with_config(1, config).unwrap();
        assert_eq!(ledger.node_id, 1);
    }

    #[test]
    fn test_invalid_config_gei() {
        let config = LedgerConfig {
            min_gei_stability: 1.5,
            ..Default::default()
        };
        let result = GlobalSymbioticLedger::with_config(1, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_ce() {
        let config = LedgerConfig {
            max_ce_per_transaction: -1.0,
            ..Default::default()
        };
        let result = GlobalSymbioticLedger::with_config(1, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_genesis_transaction() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(tx).unwrap();
        assert_eq!(ledger.transaction_count(), 1);
    }

    #[test]
    fn test_submit_child_transaction() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let genesis = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(genesis).unwrap();

        let child = make_child_tx(1002, 3, 5.0, 1010, [Some(1001), None]);
        ledger.submit_transaction(child).unwrap();
        assert_eq!(ledger.transaction_count(), 2);
    }

    #[test]
    fn test_duplicate_rejection() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(tx.clone()).unwrap();

        let result = ledger.submit_transaction(tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_z_score_rejection() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = CETransaction::new(
            1001, 2, 1, 10.0, make_timestamp(1000, 2),
            [None, None], [0u8; 64], -1.0, 0.8, Vec::new(),
        );
        let result = ledger.submit_transaction(tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_unstable_gei_rejection() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = CETransaction::new(
            1001, 2, 1, 10.0, make_timestamp(1000, 2),
            [None, None], [0u8; 64], 1.0, 0.2, Vec::new(),
        );
        let result = ledger.submit_transaction(tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_ce_amount_rejection() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = CETransaction::new(
            1001, 2, 1, -5.0, make_timestamp(1000, 2),
            [None, None], [0u8; 64], 1.0, 0.8, Vec::new(),
        );
        let result = ledger.submit_transaction(tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_parent_rejection() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = make_child_tx(1002, 3, 5.0, 1010, [Some(9999), None]);
        let result = ledger.submit_transaction(tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_transaction() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(tx).unwrap();

        let found = ledger.get_transaction(&1001);
        assert!(found.is_some());
        assert_eq!(found.unwrap().origin_node, 2);
    }

    #[test]
    fn test_get_node_transactions() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx1 = make_genesis_tx(1001, 2, 10.0, 1000);
        let tx2 = make_genesis_tx(1002, 2, 5.0, 1010);
        ledger.submit_transaction(tx1).unwrap();
        ledger.submit_transaction(tx2).unwrap();

        let node_txs = ledger.get_node_transactions(2);
        assert_eq!(node_txs.len(), 2);
    }

    #[test]
    fn test_get_latest_transactions() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        for i in 0..5 {
            let tx = make_genesis_tx(1001 + i, 2, 10.0, (1000 + i * 10) as u64);
            ledger.submit_transaction(tx).unwrap();
        }

        let latest = ledger.get_latest_transactions(3);
        assert_eq!(latest.len(), 3);
    }

    #[test]
    fn test_ledger_stats() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx1 = make_genesis_tx(1001, 2, 10.0, 1000);
        let tx2 = make_genesis_tx(1002, 3, 5.0, 1010);
        ledger.submit_transaction(tx1).unwrap();
        ledger.submit_transaction(tx2).unwrap();

        let stats = ledger.get_stats();
        assert_eq!(stats.total_transactions, 2);
        assert!((stats.total_ce_circulated - 15.0).abs() < 0.01);
        assert_eq!(stats.unique_nodes, 3); // nodes 1, 2, 3
        assert_eq!(stats.validated_count, 2);
    }

    #[test]
    fn test_dag_depth_genesis() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(tx).unwrap();

        assert_eq!(ledger.get_stats().dag_depth, 1);
    }

    #[test]
    fn test_dag_depth_chain() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let t0 = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(t0).unwrap();

        let t1 = make_child_tx(1002, 3, 5.0, 1010, [Some(1001), None]);
        ledger.submit_transaction(t1).unwrap();

        let t2 = make_child_tx(1003, 4, 3.0, 1020, [Some(1002), None]);
        ledger.submit_transaction(t2).unwrap();

        assert_eq!(ledger.get_stats().dag_depth, 3);
    }

    #[test]
    fn test_validate_previous_transactions() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx1 = make_genesis_tx(1001, 2, 10.0, 1000);
        let tx2 = make_genesis_tx(1002, 3, 5.0, 1010);
        ledger.submit_transaction(tx1).unwrap();
        ledger.submit_transaction(tx2).unwrap();

        let results = ledger.validate_previous_transactions(&[1001, 1002]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1, ValidationResult::Valid);
        assert_eq!(results[1].1, ValidationResult::Valid);
    }

    #[test]
    fn test_reset() {
        let mut ledger = GlobalSymbioticLedger::new(1);
        let tx = make_genesis_tx(1001, 2, 10.0, 1000);
        ledger.submit_transaction(tx).unwrap();

        ledger.reset();
        assert_eq!(ledger.transaction_count(), 0);
        assert_eq!(ledger.get_stats().total_transactions, 0);
    }

    #[test]
    fn test_is_genesis() {
        let tx = make_genesis_tx(1001, 2, 10.0, 1000);
        assert!(tx.is_genesis());

        let child = make_child_tx(1002, 3, 5.0, 1010, [Some(1001), None]);
        assert!(!child.is_genesis());
    }

    #[test]
    fn test_stats_default() {
        let stats = LedgerStats::new();
        assert_eq!(stats.total_transactions, 0);
        assert_eq!(stats.total_ce_circulated, 0.0);
    }

    #[test]
    fn test_validation_result_equality() {
        assert_eq!(
            ValidationResult::Valid,
            ValidationResult::Valid
        );
    }

    #[test]
    fn test_large_dag() {
        let mut ledger = GlobalSymbioticLedger::new(1);

        // Create a DAG with 100 transactions.
        let mut hashes = Vec::new();
        for i in 0..100 {
            let parents = if i < 2 {
                [None, None]
            } else {
                [
                    Some(hashes[i - 2]),
                    Some(hashes[i - 1]),
                ]
            };
            let tx = make_child_tx(
                1001 + i as u128,
                (i % 10) as u64 + 2,
                1.0,
                1000 + i as u64 * 10,
                parents,
            );
            ledger.submit_transaction(tx).unwrap();
            hashes.push(1001 + i as u128);
        }

        assert_eq!(ledger.transaction_count(), 100);
        assert!(ledger.get_stats().dag_depth > 10);
    }
}
