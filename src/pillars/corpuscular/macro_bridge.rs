//! Macro-Corpuscular Bridge — Local CE Exchange to Global Symbiotic Ledger (DAG).
//!
//! Connects the local CE exchange (ce_exchange.rs) with the global
//! symbiotic ledger (symbiotic_ledger.rs), enabling physical resource
//! homeostasis tracking at the network scale.
//!
//! # Architecture
//!
//! ```text
//!  ┌─────────────────────┐         ┌──────────────────┐
//!  │  CE Exchange (local) │───────▶│  Macro Bridge    │
//!  │  (iot_adapter.rs)    │  CE tx  │  (macro_bridge)  │
//!  └─────────────────────┘         └────────┬─────────┘
//!                                           │ submit
//!                                           ▼
//!                                  ┌──────────────────┐
//!                                  │ Global Symbiotic  │
//!                                  │    Ledger (DAG)   │
//!                                  └──────────────────┘
//! ```
//!
//! # Responsibilities
//!
//! 1. **CE Transaction Packaging**: Convert local CE exchanges into DAG
//!    transactions with proper parent references.
//! 2. **Temporal Annotation**: Attach SymbioticTimestamp from the
//!    TemporalCohesionEngine for network-wide chronological ordering.
//! 3. **GEI Propagation**: Forward the originating node's GEI stability
//!    and SCT Z-score to the ledger for SCT Guard Economic validation.
//! 4. **Resource Homeostasis**: Track physical resource consumption
//!    patterns across the network for real-time homeostasis mapping.
//!
//! # Design Principles
//!
//! - **Cooperative atomicity**: Local CE committed before DAG submission.
//! - **Zero financial logic**: CE is a merit metric for resource coordination.
//! - **Physical grounding**: Every DAG transaction traces to a real
//!   physical resource event (3D print, solar energy, hydroponics).
//!
//! **Feature Gate:** `v3.4-macro-symbiosis`

use std::collections::HashMap;

use crate::economy::symbiotic_ledger::{
    CETransaction, GlobalSymbioticLedger, LedgerConfig, LedgerError,
};
use crate::time::temporal_cohesion::{SymbioticTimestamp, TemporalCohesionEngine, TemporalConfig};

/// Maps a local CE exchange event to a DAG transaction.
#[derive(Debug, Clone)]
pub struct BridgeMapping {
    /// Local exchange identifier.
    pub exchange_id: u64,
    /// DAG transaction hash.
    pub dag_hash: u128,
    /// CE amount transferred.
    pub ce_amount: f64,
    /// Resource type identifier.
    pub resource_type: String,
    /// Timestamp of the bridge operation.
    pub bridged_at: SymbioticTimestamp,
}

/// Homeostasis snapshot for a single resource type.
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    /// Resource type identifier.
    pub resource_type: String,
    /// Total CE consumed for this resource type.
    pub total_ce_consumed: f64,
    /// Number of transactions for this resource type.
    pub transaction_count: usize,
    /// Average CE per transaction.
    pub average_ce: f64,
    /// Last update timestamp.
    pub last_update_ms: u64,
}

/// Configuration for the MacroCorpuscularBridge.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Maximum batch size for DAG submission.
    pub max_batch_size: usize,
    /// Enable automatic homeostasis tracking.
    pub track_homeostasis: bool,
    /// Ledger configuration (passed through).
    pub ledger_config: LedgerConfig,
    /// Temporal engine configuration (passed through).
    pub temporal_config: TemporalConfig,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            track_homeostasis: true,
            ledger_config: LedgerConfig::default(),
            temporal_config: TemporalConfig::default(),
        }
    }
}

/// Errors specific to macro bridge operations.
#[derive(Debug, Clone)]
pub enum BridgeError {
    /// Ledger submission failed.
    LedgerError(LedgerError),
    /// No parent transactions available for DAG reference.
    NoParentTransactions,
    /// Batch size exceeds maximum.
    BatchTooLarge { size: usize, max: usize },
    /// Temporal engine not initialized.
    TemporalNotInitialized,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::LedgerError(err) => write!(f, "Ledger error: {}", err),
            BridgeError::NoParentTransactions => {
                write!(f, "No parent transactions available for DAG reference")
            }
            BridgeError::BatchTooLarge { size, max } => {
                write!(f, "Batch size {} exceeds maximum {}", size, max)
            }
            BridgeError::TemporalNotInitialized => {
                write!(f, "Temporal cohesion engine not initialized")
            }
        }
    }
}

impl From<LedgerError> for BridgeError {
    fn from(err: LedgerError) -> Self {
        BridgeError::LedgerError(err)
    }
}

/// Statistics for the macro bridge.
#[derive(Debug, Clone)]
pub struct BridgeStats {
    /// Total local exchanges bridged to DAG.
    pub total_bridged: usize,
    /// Total CE amount bridged.
    pub total_ce_bridged: f64,
    /// Total batches submitted.
    pub batches_submitted: usize,
    /// Total failures.
    pub failures: usize,
    /// Resource type -> homeostasis snapshot.
    pub resource_snapshots: HashMap<String, ResourceSnapshot>,
}

impl BridgeStats {
    pub fn new() -> Self {
        Self {
            total_bridged: 0,
            total_ce_bridged: 0.0,
            batches_submitted: 0,
            failures: 0,
            resource_snapshots: HashMap::new(),
        }
    }
}

impl Default for BridgeStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Local CE exchange event (from ce_exchange.rs).
///
/// Represents a completed local exchange that needs to be
/// propagated to the global DAG.
#[derive(Debug, Clone)]
pub struct LocalExchangeEvent {
    /// Unique exchange identifier.
    pub exchange_id: u64,
    /// Originating node.
    pub origin_node: u64,
    /// CE amount.
    pub ce_amount: f64,
    /// Resource type (e.g., "3d_print", "solar_energy", "hydroponics").
    pub resource_type: String,
    /// SCT Z-score of the origin node.
    pub z_score: f32,
    /// GEI stability of the origin node.
    pub gei_stability: f64,
    /// Exchange payload (IoT data, fulfillment details).
    pub payload: Vec<u8>,
    /// Local timestamp (milliseconds).
    pub local_timestamp_ms: u64,
}

/// Macro-Corpuscular Bridge — Connects local CE exchange to global DAG ledger.
///
/// Acts as the translation layer between local physical resource exchanges
/// and the global symbiotic ledger, ensuring proper temporal annotation,
/// DAG parent referencing, and homeostasis tracking.
pub struct MacroCorpuscularBridge {
    /// Local node identifier.
    pub node_id: u64,
    /// Global symbiotic ledger.
    ledger: GlobalSymbioticLedger,
    /// Temporal cohesion engine.
    temporal: TemporalCohesionEngine,
    /// Bridge configuration.
    config: BridgeConfig,
    /// Bridge statistics.
    stats: BridgeStats,
    /// Mapping of exchange_id -> BridgeMapping.
    bridge_mappings: HashMap<u64, BridgeMapping>,
    /// Hash counter for deterministic transaction IDs.
    hash_counter: u128,
}

impl MacroCorpuscularBridge {
    /// Create a new bridge with default configuration.
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            ledger: GlobalSymbioticLedger::new(node_id),
            temporal: TemporalCohesionEngine::new(node_id),
            config: BridgeConfig::default(),
            stats: BridgeStats::new(),
            bridge_mappings: HashMap::new(),
            hash_counter: (node_id as u128) << 64,
        }
    }

    /// Create a new bridge with custom configuration.
    pub fn with_config(node_id: u64, config: BridgeConfig) -> Result<Self, BridgeError> {
        let ledger = GlobalSymbioticLedger::with_config(node_id, config.ledger_config.clone())?;
        let temporal = TemporalCohesionEngine::with_config(node_id, config.temporal_config.clone())
            .map_err(|_| BridgeError::TemporalNotInitialized)?;

        Ok(Self {
            node_id,
            ledger,
            temporal,
            config,
            stats: BridgeStats::new(),
            bridge_mappings: HashMap::new(),
            hash_counter: (node_id as u128) << 64,
        })
    }

    /// Generate a deterministic transaction hash.
    fn next_hash(&mut self) -> u128 {
        self.hash_counter += 1;
        self.hash_counter
    }

    /// Bridge a single local exchange event to the global DAG.
    ///
    /// This is the primary entry point: takes a local CE exchange and
    /// submits it to the global ledger with proper DAG structure.
    pub fn bridge_exchange(&mut self, event: LocalExchangeEvent) -> Result<u128, BridgeError> {
        // Generate deterministic hash.
        let hash = self.next_hash();

        // Get symbiotic timestamp.
        let timestamp = self.temporal.generate_timestamp();

        // Get parent references from latest transactions.
        let parents = self.get_parent_hashes()?;

        // Build DAG transaction.
        // Capture payload reference before move.
        let payload_clone = event.payload.clone();

        let tx = CETransaction::new(
            hash,
            event.origin_node,
            self.node_id,
            event.ce_amount,
            timestamp,
            parents,
            [0u8; 64], // Signature (would be Ed25519 in production)
            event.z_score,
            event.gei_stability,
            payload_clone,
        );

        // Submit to ledger.
        self.ledger.submit_transaction(tx)?;

        // Create bridge mapping.
        let mapping = BridgeMapping {
            exchange_id: event.exchange_id,
            dag_hash: hash,
            ce_amount: event.ce_amount,
            resource_type: event.resource_type.clone(),
            bridged_at: timestamp,
        };
        self.bridge_mappings.insert(event.exchange_id, mapping);

        // Update stats.
        self.stats.total_bridged += 1;
        self.stats.total_ce_bridged += event.ce_amount;

        // Update homeostasis tracking.
        if self.config.track_homeostasis {
            self.update_resource_snapshot(&event);
        }

        Ok(hash)
    }

    /// Bridge a batch of local exchange events.
    pub fn bridge_batch(
        &mut self,
        events: &[LocalExchangeEvent],
    ) -> Result<Vec<u128>, BridgeError> {
        if events.len() > self.config.max_batch_size {
            return Err(BridgeError::BatchTooLarge {
                size: events.len(),
                max: self.config.max_batch_size,
            });
        }

        let mut hashes = Vec::with_capacity(events.len());
        for event in events {
            let hash = self.bridge_exchange(event.clone())?;
            hashes.push(hash);
        }

        self.stats.batches_submitted += 1;
        Ok(hashes)
    }

    /// Get parent transaction hashes for a new DAG transaction.
    fn get_parent_hashes(&self) -> Result<[Option<u128>; 2], BridgeError> {
        let latest = self.ledger.get_latest_transactions(2);
        match latest.len() {
            0 => Err(BridgeError::NoParentTransactions),
            1 => Ok([Some(latest[0]), None]),
            _ => Ok([Some(latest[0]), Some(latest[1])]),
        }
    }

    /// Update the homeostasis snapshot for a resource type.
    fn update_resource_snapshot(&mut self, event: &LocalExchangeEvent) {
        let snapshot = self
            .stats
            .resource_snapshots
            .entry(event.resource_type.clone())
            .or_insert(ResourceSnapshot {
                resource_type: event.resource_type.clone(),
                total_ce_consumed: 0.0,
                transaction_count: 0,
                average_ce: 0.0,
                last_update_ms: event.local_timestamp_ms,
            });

        snapshot.total_ce_consumed += event.ce_amount;
        snapshot.transaction_count += 1;
        snapshot.average_ce = snapshot.total_ce_consumed / snapshot.transaction_count as f64;
        snapshot.last_update_ms = event.local_timestamp_ms;
    }

    /// Get the homeostasis snapshot for a specific resource type.
    pub fn get_resource_snapshot(&self, resource_type: &str) -> Option<&ResourceSnapshot> {
        self.stats.resource_snapshots.get(resource_type)
    }

    /// Get all resource homeostasis snapshots.
    pub fn get_all_snapshots(&self) -> &HashMap<String, ResourceSnapshot> {
        &self.stats.resource_snapshots
    }

    /// Get bridge statistics.
    pub fn get_stats(&self) -> &BridgeStats {
        &self.stats
    }

    /// Get the underlying ledger.
    pub fn ledger(&self) -> &GlobalSymbioticLedger {
        &self.ledger
    }

    /// Get the underlying temporal engine.
    pub fn temporal(&self) -> &TemporalCohesionEngine {
        &self.temporal
    }

    /// Get a bridge mapping by exchange ID.
    pub fn get_mapping(&self, exchange_id: u64) -> Option<&BridgeMapping> {
        self.bridge_mappings.get(&exchange_id)
    }

    /// Get the total number of bridged exchanges.
    pub fn bridged_count(&self) -> usize {
        self.stats.total_bridged
    }

    /// Access the internal ledger mutably (for testing).
    pub fn ledger_mut(&mut self) -> &mut GlobalSymbioticLedger {
        &mut self.ledger
    }

    /// Reset the bridge state (for testing).
    pub fn reset(&mut self) {
        self.ledger.reset();
        self.temporal.reset();
        self.stats = BridgeStats::new();
        self.bridge_mappings.clear();
        self.hash_counter = (self.node_id as u128) << 64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exchange(id: u64, node: u64, ce: f64, resource: &str) -> LocalExchangeEvent {
        LocalExchangeEvent {
            exchange_id: id,
            origin_node: node,
            ce_amount: ce,
            resource_type: resource.to_string(),
            z_score: 1.0,
            gei_stability: 0.8,
            payload: vec![0x01, 0x02, 0x03],
            local_timestamp_ms: 1000 + id * 10,
        }
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = MacroCorpuscularBridge::new(1);
        assert_eq!(bridge.node_id, 1);
        assert_eq!(bridge.bridged_count(), 0);
    }

    #[test]
    fn test_bridge_custom_config() {
        let config = BridgeConfig {
            max_batch_size: 50,
            track_homeostasis: true,
            ..Default::default()
        };
        let bridge = MacroCorpuscularBridge::with_config(1, config).unwrap();
        assert_eq!(bridge.node_id, 1);
    }

    #[test]
    fn test_bridge_single_exchange() {
        let mut bridge = MacroCorpuscularBridge::new(1);
        let event = make_exchange(1001, 2, 10.0, "3d_print");

        // First exchange needs to be genesis (no parents yet).
        // We need to seed the ledger with a genesis transaction first.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        let hash = bridge.bridge_exchange(event).unwrap();
        assert_eq!(bridge.bridged_count(), 1);
        assert!(bridge.get_mapping(1001).is_some());
    }

    #[test]
    fn test_bridge_batch() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        let events = vec![
            make_exchange(2001, 2, 5.0, "solar_energy"),
            make_exchange(2002, 3, 3.0, "hydroponics"),
            make_exchange(2003, 4, 7.0, "3d_print"),
        ];

        let hashes = bridge.bridge_batch(&events).unwrap();
        assert_eq!(hashes.len(), 3);
        assert_eq!(bridge.bridged_count(), 3);
        assert_eq!(bridge.stats.batches_submitted, 1);
    }

    #[test]
    fn test_batch_too_large() {
        let config = BridgeConfig {
            max_batch_size: 2,
            ..Default::default()
        };
        let mut bridge = MacroCorpuscularBridge::with_config(1, config).unwrap();

        let events = vec![
            make_exchange(3001, 2, 5.0, "test"),
            make_exchange(3002, 3, 5.0, "test"),
            make_exchange(3003, 4, 5.0, "test"),
        ];

        let result = bridge.bridge_batch(&events);
        assert!(result.is_err());
    }

    #[test]
    fn test_homeostasis_tracking() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        // Bridge multiple exchanges for the same resource.
        for i in 0..3 {
            let event = make_exchange(4001 + i, 2, 10.0, "solar_energy");
            bridge.bridge_exchange(event).unwrap();
        }

        let snapshot = bridge.get_resource_snapshot("solar_energy").unwrap();
        assert_eq!(snapshot.transaction_count, 3);
        assert!((snapshot.total_ce_consumed - 30.0).abs() < 0.01);
        assert!((snapshot.average_ce - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_homeostasis_disabled() {
        let config = BridgeConfig {
            track_homeostasis: false,
            ..Default::default()
        };
        let mut bridge = MacroCorpuscularBridge::with_config(1, config).unwrap();

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        let event = make_exchange(5001, 2, 10.0, "test");
        bridge.bridge_exchange(event).unwrap();

        assert!(bridge.get_resource_snapshot("test").is_none());
    }

    #[test]
    fn test_unstable_gei_rejection() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        // Event with unstable GEI.
        let event = LocalExchangeEvent {
            exchange_id: 6001,
            origin_node: 2,
            ce_amount: 10.0,
            resource_type: "test".to_string(),
            z_score: 1.0,
            gei_stability: 0.2, // Below default threshold of 0.5
            payload: Vec::new(),
            local_timestamp_ms: 1000,
        };

        let result = bridge.bridge_exchange(event);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_z_score_rejection() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        // Event with negative Z-score.
        let event = LocalExchangeEvent {
            exchange_id: 7001,
            origin_node: 2,
            ce_amount: 10.0,
            resource_type: "test".to_string(),
            z_score: -0.5, // Negative Z-score
            gei_stability: 0.8,
            payload: Vec::new(),
            local_timestamp_ms: 1000,
        };

        let result = bridge.bridge_exchange(event);
        assert!(result.is_err());
    }

    #[test]
    fn test_ledger_access() {
        let bridge = MacroCorpuscularBridge::new(1);
        let ledger = bridge.ledger();
        assert_eq!(ledger.transaction_count(), 0);
    }

    #[test]
    fn test_temporal_access() {
        let bridge = MacroCorpuscularBridge::new(1);
        let temporal = bridge.temporal();
        assert_eq!(temporal.peer_count(), 0);
    }

    #[test]
    fn test_reset() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        let event = make_exchange(8001, 2, 10.0, "test");
        bridge.bridge_exchange(event).unwrap();

        bridge.reset();
        assert_eq!(bridge.bridged_count(), 0);
        assert_eq!(bridge.ledger.transaction_count(), 0);
    }

    #[test]
    fn test_multiple_resource_types() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        let events = vec![
            make_exchange(9001, 2, 10.0, "3d_print"),
            make_exchange(9002, 3, 5.0, "solar_energy"),
            make_exchange(9003, 4, 8.0, "hydroponics"),
        ];

        for event in &events {
            bridge.bridge_exchange(event.clone()).unwrap();
        }

        let snapshots = bridge.get_all_snapshots();
        assert_eq!(snapshots.len(), 3);
        assert!(snapshots.contains_key("3d_print"));
        assert!(snapshots.contains_key("solar_energy"));
        assert!(snapshots.contains_key("hydroponics"));
    }

    #[test]
    fn test_bridge_stats() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        let event = make_exchange(10001, 2, 15.0, "test");
        bridge.bridge_exchange(event).unwrap();

        let stats = bridge.get_stats();
        assert_eq!(stats.total_bridged, 1);
        assert!((stats.total_ce_bridged - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_dag_structure_growth() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger.submit_transaction(genesis).unwrap();

        // Bridge 20 exchanges.
        for i in 0..20 {
            let event = make_exchange(11001 + i, (i % 5) as u64 + 2, 1.0, "test");
            bridge.bridge_exchange(event).unwrap();
        }

        let stats = bridge.ledger().get_stats();
        assert_eq!(stats.total_transactions, 21); // 20 + genesis
        assert!(stats.dag_depth > 5);
    }
}
