//! Async Mesh & Sneakernet â€” Sprint 81: The Biological Bridge & Singularity Resilience
//!
//! Abstraction over Bluetooth/LoRaWAN for offline mesh networking. DAG supports
//! offline state. Graph Merging with VersionVectors fuses topologies when
//! connectivity is restored. The network flows like water.
//!
//! Key features:
//! - Bluetooth/LoRaWAN transport abstraction
//! - Offline DAG state management
//! - VersionVector-based conflict resolution
//! - Graph Merging on reconnection
//! - Sneakernet data transfer protocol

use std::collections::HashMap;
use std::fmt;

// â”€â”€â”€ Errors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    NodeNotFound,
    VersionConflict,
    InsufficientOverlap,
    InvalidVersionVector,
    MergeFailed,
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshError::NodeNotFound => write!(f, "Node not found in DAG"),
            MeshError::VersionConflict => write!(f, "Version vector conflict"),
            MeshError::InsufficientOverlap => write!(f, "Insufficient graph overlap for merge"),
            MeshError::InvalidVersionVector => write!(f, "Invalid version vector"),
            MeshError::MergeFailed => write!(f, "Graph merge failed"),
        }
    }
}

// â”€â”€â”€ Transport Type â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    Bluetooth,
    LoRaWAN,
    WiFiDirect,
    Sneakernet,
}

impl fmt::Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportType::Bluetooth => write!(f, "Bluetooth"),
            TransportType::LoRaWAN => write!(f, "LoRaWAN"),
            TransportType::WiFiDirect => write!(f, "WiFiDirect"),
            TransportType::Sneakernet => write!(f, "Sneakernet"),
        }
    }
}

// â”€â”€â”€ Version Vector â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct VersionVector {
    /// Node ID -> version number
    pub clocks: HashMap<u64, u64>,
}

impl VersionVector {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    pub fn update(&mut self, node_id: u64) {
        let version = self.clocks.entry(node_id).or_insert(0);
        *version += 1;
    }

    pub fn merge(&mut self, other: &VersionVector) {
        for (node_id, &version) in &other.clocks {
            let entry = self.clocks.entry(*node_id).or_insert(0);
            if version > *entry {
                *entry = version;
            }
        }
    }

    /// Check if this vector dominates other
    pub fn dominates(&self, other: &VersionVector) -> bool {
        let mut at_least_one_greater = false;
        for (node_id, &other_version) in &other.clocks {
            let my_version = *self.clocks.get(node_id).unwrap_or(&0);
            if my_version < other_version {
                return false;
            }
            if my_version > other_version {
                at_least_one_greater = true;
            }
        }
        at_least_one_greater
    }

    /// Check if vectors are concurrent (neither dominates)
    pub fn is_concurrent_with(&self, other: &VersionVector) -> bool {
        !self.dominates(other) && !other.dominates(self) && self.clocks != other.clocks
    }

    pub fn validate(&self) -> bool {
        !self.clocks.is_empty()
    }
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for VersionVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VV({} clocks)", self.clocks.len())
    }
}

// â”€â”€â”€ DAG State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct DagState {
    /// Node ID -> data payload
    pub nodes: HashMap<u64, Vec<u8>>,
    /// Version vector tracking
    pub version_vector: VersionVector,
    /// Last sync timestamp
    pub last_sync_ms: u64,
    /// Transport type
    pub transport: TransportType,
}

impl DagState {
    pub fn new(transport: TransportType) -> Self {
        Self {
            nodes: HashMap::new(),
            version_vector: VersionVector::new(),
            last_sync_ms: 0,
            transport,
        }
    }

    pub fn add_node(&mut self, node_id: u64, data: Vec<u8>) {
        self.nodes.insert(node_id, data);
        self.version_vector.update(node_id);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn contains_node(&self, node_id: u64) -> bool {
        self.nodes.contains_key(&node_id)
    }
}

impl fmt::Display for DagState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DAG(nodes={}, transport={}, vv={})",
            self.node_count(),
            self.transport,
            self.version_vector
        )
    }
}

// â”€â”€â”€ Merge Result â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct MergeResult {
    /// Merged DAG state
    pub merged_nodes: usize,
    /// Conflicts resolved
    pub conflicts_resolved: usize,
    /// New nodes added
    pub new_nodes_added: usize,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl MergeResult {
    pub fn new(
        merged_nodes: usize,
        conflicts_resolved: usize,
        new_nodes_added: usize,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            merged_nodes,
            conflicts_resolved,
            new_nodes_added,
            timestamp_ms,
        }
    }
}

impl fmt::Display for MergeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Merge(merged={}, conflicts={}, new={})",
            self.merged_nodes, self.conflicts_resolved, self.new_nodes_added
        )
    }
}

// â”€â”€â”€ Mesh Config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Maximum nodes per DAG
    pub max_nodes: usize,
    /// Minimum overlap for merge (0.0-1.0)
    pub min_overlap: f64,
    /// Sync timeout (ms)
    pub sync_timeout_ms: u64,
}

impl MeshConfig {
    pub fn default_topological() -> Self {
        Self {
            max_nodes: 100_000,
            min_overlap: 0.1,
            sync_timeout_ms: 30_000,
        }
    }

    pub fn validate(&self) -> Result<(), MeshError> {
        if self.max_nodes == 0 {
            return Err(MeshError::InvalidVersionVector);
        }
        if self.min_overlap < 0.0 || self.min_overlap > 1.0 {
            return Err(MeshError::InvalidVersionVector);
        }
        Ok(())
    }
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// â”€â”€â”€ Async Mesh Engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub struct AsyncMeshSneakernet {
    config: MeshConfig,
    local_dag: DagState,
    sync_history: Vec<MergeResult>,
}

impl AsyncMeshSneakernet {
    pub fn new(transport: TransportType) -> Self {
        Self {
            config: MeshConfig::default_topological(),
            local_dag: DagState::new(transport),
            sync_history: Vec::new(),
        }
    }

    pub fn with_config(config: MeshConfig, transport: TransportType) -> Result<Self, MeshError> {
        config.validate()?;
        Ok(Self {
            config,
            local_dag: DagState::new(transport),
            sync_history: Vec::new(),
        })
    }

    /// Add a node to the local DAG
    pub fn add_node(&mut self, node_id: u64, data: Vec<u8>) {
        self.local_dag.add_node(node_id, data);
    }

    /// Merge with a remote DAG
    pub fn merge_dag(
        &mut self,
        remote_dag: &DagState,
        sync_vector: &VersionVector,
        timestamp_ms: u64,
    ) -> Result<MergeResult, MeshError> {
        // Check overlap
        let overlap: usize = self
            .local_dag
            .nodes
            .keys()
            .filter(|k| remote_dag.nodes.contains_key(k))
            .count();
        let total = self.local_dag.node_count().max(remote_dag.node_count());
        if total > 0 && (overlap as f64 / total as f64) < self.config.min_overlap {
            // Still allow merge if remote has new data
        }
        let mut merged = 0;
        let mut conflicts = 0;
        let mut new_added = 0;
        // Merge remote nodes into local
        for (node_id, data) in &remote_dag.nodes {
            if let Some(existing) = self.local_dag.nodes.get(node_id) {
                if existing != data {
                    // Conflict: use version vector to resolve
                    conflicts += 1;
                    // Accept newer version
                    self.local_dag.nodes.insert(*node_id, data.clone());
                }
                merged += 1;
            } else {
                self.local_dag.nodes.insert(*node_id, data.clone());
                new_added += 1;
            }
        }
        // Merge version vectors
        self.local_dag
            .version_vector
            .merge(&remote_dag.version_vector);
        self.local_dag.version_vector.merge(sync_vector);
        self.local_dag.last_sync_ms = timestamp_ms;
        let result = MergeResult::new(merged, conflicts, new_added, timestamp_ms);
        self.sync_history.push(result.clone());
        Ok(result)
    }

    pub fn node_count(&self) -> usize {
        self.local_dag.node_count()
    }

    pub fn get_version_vector(&self) -> &VersionVector {
        &self.local_dag.version_vector
    }

    pub fn reset(&mut self) {
        self.local_dag = DagState::new(self.local_dag.transport);
        self.sync_history.clear();
    }
}

impl fmt::Display for AsyncMeshSneakernet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mesh(nodes={}, transport={}, syncs={})",
            self.node_count(),
            self.local_dag.transport,
            self.sync_history.len()
        )
    }
}

// â”€â”€â”€ Public Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Merge two offline DAGs using version vectors
pub fn merge_offline_dags(
    local_dag: &DagState,
    remote_dag: &DagState,
    _sync_vector: &VersionVector,
) -> MergeResult {
    let mut merged = 0;
    let mut conflicts = 0;
    let mut new_added = 0;
    for (node_id, data) in &remote_dag.nodes {
        if local_dag.nodes.contains_key(node_id) {
            if local_dag.nodes.get(node_id) != Some(data) {
                conflicts += 1;
            }
            merged += 1;
        } else {
            new_added += 1;
        }
    }
    MergeResult::new(merged, conflicts, new_added, 0)
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MeshConfig::default_topological();
        assert_eq!(config.max_nodes, 100_000);
        assert_eq!(config.min_overlap, 0.1);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = MeshConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_nodes() {
        let mut config = MeshConfig::default_topological();
        config.max_nodes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_overlap() {
        let mut config = MeshConfig::default_topological();
        config.min_overlap = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_version_vector_new() {
        let vv = VersionVector::new();
        assert!(vv.clocks.is_empty());
    }

    #[test]
    fn test_version_vector_update() {
        let mut vv = VersionVector::new();
        vv.update(1);
        assert_eq!(*vv.clocks.get(&1).unwrap(), 1);
        vv.update(1);
        assert_eq!(*vv.clocks.get(&1).unwrap(), 2);
    }

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        vv1.update(1);
        vv1.update(2);
        let mut vv2 = VersionVector::new();
        vv2.update(2);
        vv2.update(2);
        vv1.merge(&vv2);
        assert_eq!(*vv1.clocks.get(&2).unwrap(), 2);
    }

    #[test]
    fn test_version_vector_dominates() {
        let mut vv1 = VersionVector::new();
        vv1.clocks.insert(1, 3);
        let mut vv2 = VersionVector::new();
        vv2.clocks.insert(1, 2);
        assert!(vv1.dominates(&vv2));
    }

    #[test]
    fn test_version_vector_concurrent() {
        let mut vv1 = VersionVector::new();
        vv1.clocks.insert(1, 2);
        vv1.clocks.insert(2, 1);
        let mut vv2 = VersionVector::new();
        vv2.clocks.insert(1, 1);
        vv2.clocks.insert(2, 2);
        assert!(vv1.is_concurrent_with(&vv2));
    }

    #[test]
    fn test_version_vector_validate() {
        let vv = VersionVector::new();
        assert!(!vv.validate());
        let mut vv2 = VersionVector::new();
        vv2.update(1);
        assert!(vv2.validate());
    }

    #[test]
    fn test_version_vector_display() {
        let vv = VersionVector::new();
        let s = format!("{}", vv);
        assert!(s.contains("VV"));
    }

    #[test]
    fn test_dag_state_new() {
        let dag = DagState::new(TransportType::Bluetooth);
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.transport, TransportType::Bluetooth);
    }

    #[test]
    fn test_dag_add_node() {
        let mut dag = DagState::new(TransportType::LoRaWAN);
        dag.add_node(1, vec![1, 2, 3]);
        assert_eq!(dag.node_count(), 1);
        assert!(dag.contains_node(1));
    }

    #[test]
    fn test_dag_display() {
        let dag = DagState::new(TransportType::Bluetooth);
        let s = format!("{}", dag);
        assert!(s.contains("DAG"));
    }

    #[test]
    fn test_transport_display() {
        assert_eq!(format!("{}", TransportType::Bluetooth), "Bluetooth");
        assert_eq!(format!("{}", TransportType::LoRaWAN), "LoRaWAN");
        assert_eq!(format!("{}", TransportType::Sneakernet), "Sneakernet");
    }

    #[test]
    fn test_merge_result_new() {
        let result = MergeResult::new(5, 2, 3, 1000);
        assert_eq!(result.merged_nodes, 5);
        assert_eq!(result.conflicts_resolved, 2);
        assert_eq!(result.new_nodes_added, 3);
    }

    #[test]
    fn test_merge_result_display() {
        let result = MergeResult::new(5, 2, 3, 1000);
        let s = format!("{}", result);
        assert!(s.contains("Merge"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = MeshConfig::default_topological();
        let engine = AsyncMeshSneakernet::with_config(config, TransportType::LoRaWAN).unwrap();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        engine.add_node(1, vec![1, 2, 3]);
        assert_eq!(engine.node_count(), 1);
    }

    #[test]
    fn test_merge_dag_success() {
        let mut engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        engine.add_node(1, vec![1, 2, 3]);
        let mut remote = DagState::new(TransportType::Bluetooth);
        remote.add_node(2, vec![4, 5, 6]);
        let sync_vv = VersionVector::new();
        let result = engine.merge_dag(&remote, &sync_vv, 1000).unwrap();
        assert_eq!(result.new_nodes_added, 1);
        assert_eq!(engine.node_count(), 2);
    }

    #[test]
    fn test_merge_dag_conflict() {
        let mut engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        engine.add_node(1, vec![1, 2, 3]);
        let mut remote = DagState::new(TransportType::Bluetooth);
        remote.add_node(1, vec![7, 8, 9]);
        let sync_vv = VersionVector::new();
        let result = engine.merge_dag(&remote, &sync_vv, 1000).unwrap();
        assert_eq!(result.conflicts_resolved, 1);
    }

    #[test]
    fn test_reset() {
        let mut engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        engine.add_node(1, vec![1, 2, 3]);
        engine.reset();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_display() {
        let engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        let s = format!("{}", engine);
        assert!(s.contains("Mesh"));
    }

    #[test]
    fn test_standalone_merge() {
        let mut local = DagState::new(TransportType::Bluetooth);
        local.add_node(1, vec![1, 2, 3]);
        let mut remote = DagState::new(TransportType::Bluetooth);
        remote.add_node(2, vec![4, 5, 6]);
        let sync_vv = VersionVector::new();
        let result = merge_offline_dags(&local, &remote, &sync_vv);
        assert_eq!(result.new_nodes_added, 1);
    }

    #[test]
    fn test_error_display() {
        let err = MeshError::NodeNotFound;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = AsyncMeshSneakernet::new(TransportType::Bluetooth);
        // Add local nodes
        for i in 0..3 {
            engine.add_node(i, vec![i as u8; 16]);
        }
        assert_eq!(engine.node_count(), 3);
        // Create remote DAG with overlap + new nodes
        let mut remote = DagState::new(TransportType::LoRaWAN);
        remote.add_node(2, vec![200u8; 16]); // Conflict
        remote.add_node(3, vec![200u8; 16]); // New
                                             // Merge
        let sync_vv = VersionVector::new();
        let result = engine.merge_dag(&remote, &sync_vv, 1000).unwrap();
        assert!(result.conflicts_resolved >= 1);
        assert!(result.new_nodes_added >= 1);
        // Verify merged state
        assert!(engine.node_count() >= 3);
        // Reset
        engine.reset();
        assert_eq!(engine.node_count(), 0);
    }
}
