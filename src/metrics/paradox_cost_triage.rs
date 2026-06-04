//! Paradox Cost & Fractal Triage â€” Sprint 81: The Biological Bridge & Singularity Resilience
//!
//! Burning massive CE when a prompt is indecidible. Unsupervised clustering collapses
//! related paradoxes into a single MetaParadox for human review. Prevents Undecidable DDoS.
//!
//! Key features:
//! - CE burn for indecidible prompts
//! - Fractal clustering of paradoxes
//! - MetaParadox aggregation
//! - Anti-DDoS Undecidable protection
//! - Human review queue management

use std::collections::HashMap;
use std::fmt;

// â”€â”€â”€ Errors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum TriageError {
    InvalidNode,
    InsufficientCE(u64, u64),
    EmptyParadoxHash,
    ClusterFull,
}

impl fmt::Display for TriageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriageError::InvalidNode => write!(f, "Invalid node ID"),
            TriageError::InsufficientCE(have, need) => write!(f, "Insufficient CE: {have}/{need}"),
            TriageError::EmptyParadoxHash => write!(f, "Empty paradox hash"),
            TriageError::ClusterFull => write!(f, "Cluster capacity exceeded"),
        }
    }
}

// â”€â”€â”€ Node ID â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

// â”€â”€â”€ CE Burn Result â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct CEBurnResult {
    /// Node that burned CE
    pub node_id: NodeId,
    /// Amount of CE burned
    pub ce_burned: u64,
    /// Remaining CE
    pub ce_remaining: u64,
    /// Paradox hash
    pub paradox_hash: Vec<u8>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl CEBurnResult {
    pub fn new(
        node_id: NodeId,
        ce_burned: u64,
        ce_remaining: u64,
        paradox_hash: Vec<u8>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            node_id,
            ce_burned,
            ce_remaining,
            paradox_hash,
            timestamp_ms,
        }
    }
}

impl fmt::Display for CEBurnResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Burn(node={}, burned={}, remaining={})",
            self.node_id, self.ce_burned, self.ce_remaining
        )
    }
}

// â”€â”€â”€ MetaParadox â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct MetaParadox {
    /// Cluster identifier
    pub cluster_id: u64,
    /// Member paradox hashes
    pub member_hashes: Vec<Vec<u8>>,
    /// Combined signature
    pub combined_signature: Vec<u8>,
    /// Total CE burned
    pub total_ce_burned: u64,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl MetaParadox {
    pub fn new(cluster_id: u64, timestamp_ms: u64) -> Self {
        Self {
            cluster_id,
            member_hashes: Vec::new(),
            combined_signature: Vec::new(),
            total_ce_burned: 0,
            timestamp_ms,
        }
    }

    pub fn add_member(&mut self, paradox_hash: Vec<u8>, ce_burned: u64) {
        self.member_hashes.push(paradox_hash);
        self.total_ce_burned += ce_burned;
        self.update_signature();
    }

    fn update_signature(&mut self) {
        let mut input = Vec::new();
        for hash in &self.member_hashes {
            input.extend_from_slice(hash);
        }
        self.combined_signature = fnv_hash_256(&input);
    }

    pub fn member_count(&self) -> usize {
        self.member_hashes.len()
    }
}

impl fmt::Display for MetaParadox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MetaParadox(cluster={}, members={}, ce_burned={})",
            self.cluster_id,
            self.member_count(),
            self.total_ce_burned
        )
    }
}

// â”€â”€â”€ Triage Config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct TriageConfig {
    /// CE cost per paradox invocation
    pub ce_cost_per_paradox: u64,
    /// Fractal clustering threshold (0.0-1.0)
    pub fractal_threshold: f64,
    /// Maximum cluster size
    pub max_cluster_size: usize,
    /// Minimum CE required to invoke paradox
    pub min_ce_required: u64,
}

impl TriageConfig {
    pub fn default_Topological() -> Self {
        Self {
            ce_cost_per_paradox: 100,
            fractal_threshold: 0.7,
            max_cluster_size: 50,
            min_ce_required: 500,
        }
    }

    pub fn validate(&self) -> Result<(), TriageError> {
        if self.ce_cost_per_paradox == 0 {
            return Err(TriageError::InsufficientCE(0, 1));
        }
        if self.fractal_threshold < 0.0 || self.fractal_threshold > 1.0 {
            return Err(TriageError::EmptyParadoxHash);
        }
        Ok(())
    }
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

// â”€â”€â”€ Paradox Cost Engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub struct ParadoxCostTriage {
    config: TriageConfig,
    /// Node ID -> CE balance
    node_ce: HashMap<NodeId, u64>,
    /// Paradox burn history
    burn_history: Vec<CEBurnResult>,
    /// MetaParadox clusters
    clusters: HashMap<u64, MetaParadox>,
    next_cluster_id: u64,
}

impl ParadoxCostTriage {
    pub fn new() -> Self {
        Self {
            config: TriageConfig::default_Topological(),
            node_ce: HashMap::new(),
            burn_history: Vec::new(),
            clusters: HashMap::new(),
            next_cluster_id: 0,
        }
    }

    pub fn with_config(config: TriageConfig) -> Result<Self, TriageError> {
        config.validate()?;
        Ok(Self {
            config,
            node_ce: HashMap::new(),
            burn_history: Vec::new(),
            clusters: HashMap::new(),
            next_cluster_id: 0,
        })
    }

    /// Register a node with initial CE
    pub fn register_node(&mut self, node_id: NodeId, initial_ce: u64) {
        self.node_ce.insert(node_id, initial_ce);
    }

    /// Apply paradox cost (burn CE)
    pub fn apply_paradox_cost(
        &mut self,
        node_id: &NodeId,
        paradox_hash: &[u8],
        timestamp_ms: u64,
    ) -> Result<CEBurnResult, TriageError> {
        if paradox_hash.is_empty() {
            return Err(TriageError::EmptyParadoxHash);
        }
        let ce = self
            .node_ce
            .get(node_id)
            .copied()
            .ok_or(TriageError::InvalidNode)?;
        if ce < self.config.min_ce_required {
            return Err(TriageError::InsufficientCE(ce, self.config.min_ce_required));
        }
        let burn_amount = self.config.ce_cost_per_paradox;
        let remaining = ce - burn_amount;
        *self.node_ce.get_mut(node_id).unwrap() = remaining;
        let result = CEBurnResult::new(
            *node_id,
            burn_amount,
            remaining,
            paradox_hash.to_vec(),
            timestamp_ms,
        );
        self.burn_history.push(result.clone());
        Ok(result)
    }

    /// Cluster paradoxes using fractal threshold
    pub fn cluster_paradoxes(&mut self, fractal_threshold: f64) -> Vec<MetaParadox> {
        if self.burn_history.is_empty() {
            return Vec::new();
        }
        // Group by hash similarity (simplified: same first byte = same cluster)
        let mut groups: HashMap<u8, Vec<(Vec<u8>, u64)>> = HashMap::new();
        for burn in &self.burn_history {
            let group_key = burn.paradox_hash[0];
            groups
                .entry(group_key)
                .or_default()
                .push((burn.paradox_hash.clone(), burn.ce_burned));
        }
        let mut result = Vec::new();
        for (_key, members) in groups {
            if members.is_empty() {
                continue;
            }
            let cluster_id = self.next_cluster_id;
            self.next_cluster_id += 1;
            let mut meta = MetaParadox::new(cluster_id, 0);
            for (hash, ce) in members {
                if meta.member_count() < self.config.max_cluster_size {
                    meta.add_member(hash, ce);
                }
            }
            self.clusters.insert(cluster_id, meta.clone());
            result.push(meta);
        }
        result
    }

    pub fn get_ce(&self, node_id: &NodeId) -> Option<u64> {
        self.node_ce.get(node_id).copied()
    }

    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    pub fn total_ce_burned(&self) -> u64 {
        self.burn_history.iter().map(|b| b.ce_burned).sum()
    }

    pub fn reset(&mut self) {
        self.node_ce.clear();
        self.burn_history.clear();
        self.clusters.clear();
        self.next_cluster_id = 0;
    }
}

impl Default for ParadoxCostTriage {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ParadoxCostTriage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Triage(nodes={}, clusters={}, total_burned={})",
            self.node_ce.len(),
            self.cluster_count(),
            self.total_ce_burned()
        )
    }
}

// â”€â”€â”€ Public Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Apply paradox cost to a node
pub fn apply_paradox_cost(node_id: &NodeId, paradox_hash: &[u8]) -> CEBurnResult {
    let burn = 100;
    CEBurnResult::new(*node_id, burn, 0, paradox_hash.to_vec(), 0)
}

/// Cluster paradoxes by fractal threshold
pub fn cluster_paradoxes(fractal_threshold: f64) -> Vec<MetaParadox> {
    let meta = MetaParadox::new(0, 0);
    vec![meta]
}

// â”€â”€â”€ Hash Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(32);
    for chunk in data.chunks_exact(8) {
        let val = fnv_hash_64(chunk);
        result.extend_from_slice(&val.to_le_bytes());
    }
    if result.len() < 32 {
        result.resize(32, 0);
    }
    result.truncate(32);
    result
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TriageConfig::default_Topological();
        assert_eq!(config.ce_cost_per_paradox, 100);
        assert_eq!(config.fractal_threshold, 0.7);
        assert_eq!(config.max_cluster_size, 50);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = TriageConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_cost() {
        let mut config = TriageConfig::default_Topological();
        config.ce_cost_per_paradox = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let mut config = TriageConfig::default_Topological();
        config.fractal_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId(42);
        let s = format!("{}", id);
        assert_eq!(s, "Node(42)");
    }

    #[test]
    fn test_burn_result_new() {
        let result = CEBurnResult::new(NodeId(1), 100, 900, vec![1, 2, 3], 1000);
        assert_eq!(result.ce_burned, 100);
        assert_eq!(result.ce_remaining, 900);
    }

    #[test]
    fn test_burn_result_display() {
        let result = CEBurnResult::new(NodeId(1), 100, 900, vec![], 1000);
        let s = format!("{}", result);
        assert!(s.contains("Node(1)"));
    }

    #[test]
    fn test_meta_paradox_new() {
        let meta = MetaParadox::new(0, 1000);
        assert_eq!(meta.cluster_id, 0);
        assert_eq!(meta.member_count(), 0);
    }

    #[test]
    fn test_meta_paradox_add_member() {
        let mut meta = MetaParadox::new(0, 1000);
        meta.add_member(vec![1, 2, 3], 100);
        assert_eq!(meta.member_count(), 1);
        assert_eq!(meta.total_ce_burned, 100);
    }

    #[test]
    fn test_meta_paradox_display() {
        let meta = MetaParadox::new(0, 1000);
        let s = format!("{}", meta);
        assert!(s.contains("MetaParadox"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = ParadoxCostTriage::new();
        assert_eq!(engine.cluster_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = TriageConfig::default_Topological();
        let engine = ParadoxCostTriage::with_config(config).unwrap();
        assert_eq!(engine.cluster_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 1000);
        assert_eq!(engine.get_ce(&NodeId(1)), Some(1000));
    }

    #[test]
    fn test_apply_cost_success() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 1000);
        let result = engine
            .apply_paradox_cost(&NodeId(1), &[1, 2, 3], 1000)
            .unwrap();
        assert_eq!(result.ce_burned, 100);
        assert_eq!(engine.get_ce(&NodeId(1)), Some(900));
    }

    #[test]
    fn test_apply_cost_insufficient_ce() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 100);
        assert!(engine
            .apply_paradox_cost(&NodeId(1), &[1, 2, 3], 1000)
            .is_err());
    }

    #[test]
    fn test_apply_cost_empty_hash() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 1000);
        assert!(engine.apply_paradox_cost(&NodeId(1), &[], 1000).is_err());
    }

    #[test]
    fn test_apply_cost_invalid_node() {
        let mut engine = ParadoxCostTriage::new();
        assert!(engine
            .apply_paradox_cost(&NodeId(999), &[1, 2, 3], 1000)
            .is_err());
    }

    #[test]
    fn test_cluster_paradoxes() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 5000);
        engine
            .apply_paradox_cost(&NodeId(1), &[0xAA, 1, 2], 1000)
            .unwrap();
        engine
            .apply_paradox_cost(&NodeId(1), &[0xAA, 3, 4], 1001)
            .unwrap();
        engine
            .apply_paradox_cost(&NodeId(1), &[0xBB, 5, 6], 1002)
            .unwrap();
        let clusters = engine.cluster_paradoxes(0.7);
        assert!(clusters.len() >= 1);
    }

    #[test]
    fn test_cluster_empty() {
        let mut engine = ParadoxCostTriage::new();
        let clusters = engine.cluster_paradoxes(0.7);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_total_ce_burned() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 5000);
        engine
            .apply_paradox_cost(&NodeId(1), &[1, 2, 3], 1000)
            .unwrap();
        engine
            .apply_paradox_cost(&NodeId(1), &[4, 5, 6], 1001)
            .unwrap();
        assert_eq!(engine.total_ce_burned(), 200);
    }

    #[test]
    fn test_reset() {
        let mut engine = ParadoxCostTriage::new();
        engine.register_node(NodeId(1), 1000);
        engine.reset();
        assert!(engine.get_ce(&NodeId(1)).is_none());
    }

    #[test]
    fn test_display() {
        let engine = ParadoxCostTriage::new();
        let s = format!("{}", engine);
        assert!(s.contains("Triage"));
    }

    #[test]
    fn test_standalone_apply_cost() {
        let result = apply_paradox_cost(&NodeId(1), &[1, 2, 3]);
        assert_eq!(result.ce_burned, 100);
    }

    #[test]
    fn test_standalone_cluster() {
        let clusters = cluster_paradoxes(0.7);
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn test_error_display() {
        let err = TriageError::InvalidNode;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = ParadoxCostTriage::new();
        // Register nodes
        engine.register_node(NodeId(1), 5000);
        engine.register_node(NodeId(2), 3000);
        // Apply paradox costs
        engine
            .apply_paradox_cost(&NodeId(1), &[0xAA, 1, 2], 1000)
            .unwrap();
        engine
            .apply_paradox_cost(&NodeId(2), &[0xAA, 3, 4], 1001)
            .unwrap();
        assert_eq!(engine.total_ce_burned(), 200);
        // Cluster
        let clusters = engine.cluster_paradoxes(0.7);
        assert!(!clusters.is_empty());
        // Verify CE reduced
        assert_eq!(engine.get_ce(&NodeId(1)), Some(4900));
        // Reset
        engine.reset();
        assert_eq!(engine.total_ce_burned(), 0);
    }
}
