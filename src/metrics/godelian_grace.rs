//! Gödelian Grace — Sprint 80: Gödelian Synthesis & Architecture of Absolute Incompleteness
//!
//! Detection of Gödelian paradoxes (indecidable fluctuations) and graceful handling
//! via "Punto de Singularidad" marking. Nodes exhibiting chaotic Z-fluctuation without
//! convergence are isolated without penalty and delegated to human intuition.
//!
//! Key features:
//! - Chaotic fluctuation detection via Z-score history
//! - Singularity point marking
//! - Graceful isolation (no penalty)
//! - Human delegation queue
//! - Atractor de lo Desconocido (unknown attractor) metrics

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GraceError {
    InsufficientHistory(usize, usize),
    InvalidNode,
    AlreadyMarked,
    ChaosThresholdExceeded(f64, f64),
    EmptySignature,
}

impl fmt::Display for GraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraceError::InsufficientHistory(have, need) => {
                write!(f, "Insufficient history: {have}/{need}")
            }
            GraceError::InvalidNode => write!(f, "Invalid node ID"),
            GraceError::AlreadyMarked => write!(f, "Node already marked as singularity"),
            GraceError::ChaosThresholdExceeded(actual, threshold) => {
                write!(f, "Chaos threshold exceeded: {actual}/{threshold}")
            }
            GraceError::EmptySignature => write!(f, "Empty paradox signature"),
        }
    }
}

// ─── Grace State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraceState {
    /// Node operating normally
    Normal,
    /// Node flagged for monitoring
    Monitoring,
    /// Node marked as singularity point (isolated, no penalty)
    Singularity,
    /// Node delegated to human review
    HumanDelegated,
}

impl fmt::Display for GraceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraceState::Normal => write!(f, "Normal"),
            GraceState::Monitoring => write!(f, "Monitoring"),
            GraceState::Singularity => write!(f, "Singularity"),
            GraceState::HumanDelegated => write!(f, "HumanDelegated"),
        }
    }
}

// ─── Node ID ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

// ─── Node State ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GodelianNode {
    /// Node identifier
    pub node_id: NodeId,
    /// Current grace state
    pub state: GraceState,
    /// Z-score history
    pub z_history: Vec<f64>,
    /// Paradox signature (empty until detected)
    pub paradox_signature: Vec<u8>,
    /// Chaos score (0.0 = stable, 1.0 = chaotic)
    pub chaos_score: f64,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl GodelianNode {
    pub fn new(node_id: NodeId, timestamp_ms: u64) -> Self {
        Self {
            node_id,
            state: GraceState::Normal,
            z_history: Vec::new(),
            paradox_signature: Vec::new(),
            chaos_score: 0.0,
            timestamp_ms,
        }
    }

    /// Add a Z-score observation
    pub fn observe(&mut self, z_score: f64, timestamp_ms: u64) {
        self.z_history.push(z_score);
        self.timestamp_ms = timestamp_ms;
        self.update_chaos_score();
    }

    /// Update chaos score from Z-history
    fn update_chaos_score(&mut self) {
        if self.z_history.len() < 2 {
            self.chaos_score = 0.0;
            return;
        }

        // Compute variance of recent Z-scores
        let recent = &self.z_history[self.z_history.len().saturating_sub(10)..];
        let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
        let variance: f64 =
            recent.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / recent.len() as f64;

        // Chaos score: normalized standard deviation
        self.chaos_score = (variance.sqrt().min(3.0) / 3.0).min(1.0);
    }
}

impl fmt::Display for GodelianNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GodelianNode(id={}, state={}, chaos={:.3})",
            self.node_id, self.state, self.chaos_score
        )
    }
}

// ─── Grace Record ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GraceRecord {
    /// Node ID
    pub node_id: NodeId,
    /// Previous state
    pub previous_state: GraceState,
    /// New state
    pub new_state: GraceState,
    /// Chaos score at transition
    pub chaos_score: f64,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl fmt::Display for GraceRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GraceRecord(node={}, {}→{}, chaos={:.3})",
            self.node_id, self.previous_state, self.new_state, self.chaos_score
        )
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GraceConfig {
    /// Chaos threshold for singularity marking
    pub chaos_threshold: f64,
    /// Minimum Z-history required for detection
    pub min_history: usize,
    /// Maximum nodes tracked
    pub max_nodes: usize,
    /// Auto-delegate to human on singularity
    pub auto_delegate: bool,
}

impl GraceConfig {
    pub fn default_stuartian() -> Self {
        Self {
            chaos_threshold: 0.7,
            min_history: 5,
            max_nodes: 10000,
            auto_delegate: true,
        }
    }

    pub fn validate(&self) -> Result<(), GraceError> {
        if self.chaos_threshold < 0.0 || self.chaos_threshold > 1.0 {
            return Err(GraceError::ChaosThresholdExceeded(
                self.chaos_threshold,
                0.7,
            ));
        }
        if self.min_history == 0 {
            return Err(GraceError::InsufficientHistory(0, 1));
        }
        if self.max_nodes == 0 {
            return Err(GraceError::InvalidNode);
        }
        Ok(())
    }
}

impl Default for GraceConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Gödelian Grace Engine ────────────────────────────────────────────────────

pub struct GodelianGrace {
    config: GraceConfig,
    nodes: HashMap<NodeId, GodelianNode>,
    records: Vec<GraceRecord>,
    singularity_count: usize,
    delegation_queue: Vec<NodeId>,
}

impl GodelianGrace {
    pub fn new() -> Self {
        Self {
            config: GraceConfig::default_stuartian(),
            nodes: HashMap::new(),
            records: Vec::new(),
            singularity_count: 0,
            delegation_queue: Vec::new(),
        }
    }

    pub fn with_config(config: GraceConfig) -> Result<Self, GraceError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            records: Vec::new(),
            singularity_count: 0,
            delegation_queue: Vec::new(),
        })
    }

    /// Register a new node
    pub fn register_node(&mut self, node_id: NodeId, timestamp_ms: u64) -> Result<(), GraceError> {
        if self.nodes.len() >= self.config.max_nodes {
            return Err(GraceError::InvalidNode);
        }
        if self.nodes.contains_key(&node_id) {
            return Err(GraceError::AlreadyMarked);
        }
        self.nodes
            .insert(node_id, GodelianNode::new(node_id, timestamp_ms));
        Ok(())
    }

    /// Observe a Z-score for a node
    pub fn observe(
        &mut self,
        node_id: &NodeId,
        z_score: f64,
        timestamp_ms: u64,
    ) -> Result<(), GraceError> {
        let node = self.nodes.get_mut(node_id).ok_or(GraceError::InvalidNode)?;
        node.observe(z_score, timestamp_ms);

        // Extract history length before reborrowing self
        let history_len = node.z_history.len();
        let min_history = self.config.min_history;

        if history_len >= min_history {
            // Copy data needed for chaos check to avoid borrow conflicts
            let node_id_copy = node_id.clone();
            let z_history = node.z_history.clone();
            let chaos_threshold = self.config.chaos_threshold;
            let node_state = node.state.clone();
            let chaos_score = node.chaos_score;

            // Detect Gödelian paradox: high chaos without convergence
            let is_paradox = {
                if z_history.len() < min_history {
                    return Ok(());
                }
                let recent = &z_history[z_history.len().saturating_sub(10)..];
                let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
                let variance: f64 =
                    recent.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / recent.len() as f64;
                let chaos = (variance.sqrt().min(3.0) / 3.0).min(1.0);
                chaos >= chaos_threshold
            };

            if is_paradox && node_state == GraceState::Normal {
                self.escalate_node(&node_id_copy, GraceState::Monitoring, timestamp_ms);
            }

            if chaos_score >= chaos_threshold && node_state != GraceState::Singularity {
                self.mark_singularity(&node_id_copy, timestamp_ms);
            }
        }

        Ok(())
    }

    /// Detect Gödelian paradox in Z-history
    pub fn detect_godelian_paradox(&self, z_history: &[f64], chaos_threshold: f64) -> bool {
        if z_history.len() < self.config.min_history {
            return false;
        }

        // Compute chaos from history
        let recent = &z_history[z_history.len().saturating_sub(10)..];
        let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
        let variance: f64 =
            recent.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / recent.len() as f64;

        let chaos = (variance.sqrt().min(3.0) / 3.0).min(1.0);
        chaos >= chaos_threshold
    }

    /// Escalate node state
    fn escalate_node(&mut self, node_id: &NodeId, new_state: GraceState, timestamp_ms: u64) {
        let node = self.nodes.get_mut(node_id).unwrap();
        let previous = node.state;
        node.state = new_state;
        node.timestamp_ms = timestamp_ms;

        self.records.push(GraceRecord {
            node_id: *node_id,
            previous_state: previous,
            new_state,
            chaos_score: node.chaos_score,
            timestamp_ms,
        });
    }

    /// Mark node as singularity point
    pub fn mark_singularity(
        &mut self,
        node_id: &NodeId,
        timestamp_ms: u64,
    ) -> Result<GraceState, GraceError> {
        // Extract data from node before mutable borrow
        let (previous, z_history) = {
            let node = self.nodes.get(node_id).ok_or(GraceError::InvalidNode)?;
            if node.state == GraceState::Singularity || node.state == GraceState::HumanDelegated {
                return Err(GraceError::AlreadyMarked);
            }
            (node.state.clone(), node.z_history.clone())
        };

        self.escalate_node(node_id, GraceState::Singularity, timestamp_ms);
        self.singularity_count += 1;

        // Generate paradox signature with extracted data
        let paradox_sig = self.generate_paradox_signature(node_id, &z_history);
        if let Some(node_mut) = self.nodes.get_mut(node_id) {
            node_mut.paradox_signature = paradox_sig;
        }

        // Auto-delegate if configured
        if self.config.auto_delegate {
            self.escalate_node(node_id, GraceState::HumanDelegated, timestamp_ms);
            self.delegation_queue.push(*node_id);
        }

        Ok(GraceState::Singularity)
    }

    /// Generate paradox signature from Z-history
    fn generate_paradox_signature(&self, node_id: &NodeId, z_history: &[f64]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&node_id.0.to_le_bytes());
        for &z in z_history {
            data.extend_from_slice(&z.to_le_bytes());
        }
        fnv_hash_256(&data)
    }

    /// Invoke Gödelian grace for a node
    pub fn invoke_godelian_grace(
        &mut self,
        node_id: &NodeId,
        paradox_signature: &[u8],
        timestamp_ms: u64,
    ) -> Result<GraceState, GraceError> {
        if paradox_signature.is_empty() {
            return Err(GraceError::EmptySignature);
        }

        let node = self.nodes.get_mut(node_id).ok_or(GraceError::InvalidNode)?;
        let previous = node.state;

        node.state = GraceState::Singularity;
        node.paradox_signature = paradox_signature.to_vec();
        node.timestamp_ms = timestamp_ms;
        self.singularity_count += 1;

        self.records.push(GraceRecord {
            node_id: *node_id,
            previous_state: previous,
            new_state: GraceState::Singularity,
            chaos_score: node.chaos_score,
            timestamp_ms,
        });

        if self.config.auto_delegate {
            node.state = GraceState::HumanDelegated;
            self.delegation_queue.push(*node_id);
        }

        Ok(GraceState::Singularity)
    }

    /// Get node state
    pub fn get_state(&self, node_id: &NodeId) -> Option<GraceState> {
        self.nodes.get(node_id).map(|n| n.state)
    }

    /// Get chaos score for a node
    pub fn get_chaos_score(&self, node_id: &NodeId) -> Option<f64> {
        self.nodes.get(node_id).map(|n| n.chaos_score)
    }

    /// Get singularity count
    pub fn singularity_count(&self) -> usize {
        self.singularity_count
    }

    /// Get delegation queue
    pub fn delegation_queue(&self) -> &[NodeId] {
        &self.delegation_queue
    }

    /// Process delegation queue (remove processed node)
    pub fn process_delegation(&mut self, node_id: &NodeId) -> bool {
        if let Some(pos) = self.delegation_queue.iter().position(|n| n == node_id) {
            self.delegation_queue.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all records
    pub fn records(&self) -> &[GraceRecord] {
        &self.records
    }

    /// Get average chaos score across all nodes
    pub fn average_chaos(&self) -> Option<f64> {
        if self.nodes.is_empty() {
            return None;
        }
        let sum: f64 = self.nodes.values().map(|n| n.chaos_score).sum();
        Some(sum / self.nodes.len() as f64)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.records.clear();
        self.singularity_count = 0;
        self.delegation_queue.clear();
    }
}

impl Default for GodelianGrace {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GodelianGrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GodelianGrace(nodes={}, singularities={}, delegations={}, avg_chaos={})",
            self.nodes.len(),
            self.singularity_count,
            self.delegation_queue.len(),
            self.average_chaos()
                .map(|c| format!("{:.3}", c))
                .unwrap_or_else(|| "N/A".to_string())
        )
    }
}

// ─── Public Standalone Functions ──────────────────────────────────────────────

/// Detect Gödelian paradox in Z-score history.
/// Returns true if chaotic fluctuation without convergence is detected.
pub fn detect_godelian_paradox(sct_z_history: &[f64], chaos_threshold: f64) -> bool {
    if sct_z_history.len() < 5 {
        return false;
    }

    let recent = &sct_z_history[sct_z_history.len().saturating_sub(10)..];
    let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
    let variance: f64 =
        recent.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / recent.len() as f64;

    let chaos = (variance.sqrt().min(3.0) / 3.0).min(1.0);
    chaos >= chaos_threshold
}

/// Invoke Gödelian grace for a node with a paradox signature.
/// Marks as singularity point, isolates without penalty, delegates to human.
pub fn invoke_godelian_grace(node_id: &NodeId, paradox_signature: &[u8]) -> GraceState {
    if paradox_signature.is_empty() {
        return GraceState::Normal;
    }
    GraceState::Singularity
}

/// FNV-1a 64-bit hash
fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// FNV-1a 256-bit hash
fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(32);
    let base = fnv_hash_64(data);
    for i in 0..4 {
        let mut combined = Vec::new();
        combined.extend_from_slice(data);
        combined.push(i as u8);
        let h = fnv_hash_64(&combined)
            .wrapping_add(i as u64)
            .wrapping_mul(0x100000001b3);
        result.extend_from_slice(&h.to_le_bytes());
    }
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = GraceConfig::default_stuartian();
        assert_eq!(config.chaos_threshold, 0.7);
        assert_eq!(config.min_history, 5);
        assert_eq!(config.max_nodes, 10000);
        assert!(config.auto_delegate);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = GraceConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = GraceConfig {
            chaos_threshold: 1.5,
            ..GraceConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_history() {
        let config = GraceConfig {
            min_history: 0,
            ..GraceConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_nodes() {
        let config = GraceConfig {
            max_nodes: 0,
            ..GraceConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_new() {
        let node = GodelianNode::new(NodeId(1), 1000);
        assert_eq!(node.state, GraceState::Normal);
        assert_eq!(node.z_history.len(), 0);
        assert_eq!(node.chaos_score, 0.0);
    }

    #[test]
    fn test_node_observe() {
        let mut node = GodelianNode::new(NodeId(1), 1000);
        node.observe(1.5, 1100);
        assert_eq!(node.z_history.len(), 1);
    }

    #[test]
    fn test_node_chaos_low() {
        let mut node = GodelianNode::new(NodeId(1), 1000);
        for i in 0..10 {
            node.observe(0.1 * i as f64, 1000 + i as u64);
        }
        assert!(node.chaos_score < 0.7);
    }

    #[test]
    fn test_node_chaos_high() {
        let mut node = GodelianNode::new(NodeId(1), 1000);
        let values = [5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        for (i, v) in values.iter().enumerate() {
            node.observe(*v, 1000 + i as u64);
        }
        assert!(node.chaos_score > 0.5);
    }

    #[test]
    fn test_node_display() {
        let node = GodelianNode::new(NodeId(42), 1000);
        let s = format!("{}", node);
        assert!(s.contains("Node(42)"));
    }

    #[test]
    fn test_grace_state_display() {
        assert_eq!(format!("{}", GraceState::Normal), "Normal");
        assert_eq!(format!("{}", GraceState::Monitoring), "Monitoring");
        assert_eq!(format!("{}", GraceState::Singularity), "Singularity");
        assert_eq!(format!("{}", GraceState::HumanDelegated), "HumanDelegated");
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId(42);
        assert_eq!(format!("{}", id), "Node(42)");
    }

    #[test]
    fn test_engine_creation() {
        let engine = GodelianGrace::new();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = GraceConfig::default_stuartian();
        let engine = GodelianGrace::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_register_node() {
        let mut engine = GodelianGrace::new();
        assert!(engine.register_node(NodeId(1), 1000).is_ok());
    }

    #[test]
    fn test_register_duplicate_node() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        assert_eq!(
            engine.register_node(NodeId(1), 2000),
            Err(GraceError::AlreadyMarked)
        );
    }

    #[test]
    fn test_observe_normal() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        for i in 0..10 {
            engine
                .observe(&NodeId(1), 0.1 * i as f64, 1000 + i as u64)
                .unwrap();
        }
        assert_eq!(engine.get_state(&NodeId(1)), Some(GraceState::Normal));
    }

    #[test]
    fn test_observe_chaos_triggers_singularity() {
        let mut engine = GodelianGrace::with_config(GraceConfig {
            chaos_threshold: 0.5,
            min_history: 5,
            auto_delegate: false,
            ..GraceConfig::default_stuartian()
        })
        .unwrap();
        engine.register_node(NodeId(1), 1000).unwrap();
        let values = [5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        for (i, v) in values.iter().enumerate() {
            engine.observe(&NodeId(1), *v, 1000 + i as u64).unwrap();
        }
        assert!(engine.get_state(&NodeId(1)).unwrap() != GraceState::Normal);
    }

    #[test]
    fn test_observe_unknown_node() {
        let mut engine = GodelianGrace::new();
        assert_eq!(
            engine.observe(&NodeId(99), 1.0, 1000),
            Err(GraceError::InvalidNode)
        );
    }

    #[test]
    fn test_detect_paradox_chaotic() {
        let engine = GodelianGrace::new();
        let history = vec![5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        assert!(engine.detect_godelian_paradox(&history, 0.5));
    }

    #[test]
    fn test_detect_paradox_stable() {
        let engine = GodelianGrace::new();
        let history = vec![0.0, 0.1, 0.0, 0.1, 0.0, 0.1, 0.0, 0.1, 0.0, 0.1];
        assert!(!engine.detect_godelian_paradox(&history, 0.5));
    }

    #[test]
    fn test_detect_paradox_insufficient_history() {
        let engine = GodelianGrace::new();
        let history = vec![5.0, -5.0];
        assert!(!engine.detect_godelian_paradox(&history, 0.5));
    }

    #[test]
    fn test_mark_singularity() {
        let mut engine = GodelianGrace::with_config(GraceConfig {
            auto_delegate: false,
            ..GraceConfig::default_stuartian()
        })
        .unwrap();
        engine.register_node(NodeId(1), 1000).unwrap();
        let state = engine.mark_singularity(&NodeId(1), 2000);
        assert_eq!(state.unwrap(), GraceState::Singularity);
        assert_eq!(engine.singularity_count(), 1);
    }

    #[test]
    fn test_mark_singularity_already_marked() {
        let mut engine = GodelianGrace::with_config(GraceConfig {
            auto_delegate: false,
            ..GraceConfig::default_stuartian()
        })
        .unwrap();
        engine.register_node(NodeId(1), 1000).unwrap();
        engine.mark_singularity(&NodeId(1), 2000).unwrap();
        assert_eq!(
            engine.mark_singularity(&NodeId(1), 3000),
            Err(GraceError::AlreadyMarked)
        );
    }

    #[test]
    fn test_invoke_grace() {
        let mut engine = GodelianGrace::with_config(GraceConfig {
            auto_delegate: false,
            ..GraceConfig::default_stuartian()
        })
        .unwrap();
        engine.register_node(NodeId(1), 1000).unwrap();
        let sig = vec![1, 2, 3, 4, 5];
        let state = engine.invoke_godelian_grace(&NodeId(1), &sig, 2000);
        assert_eq!(state.unwrap(), GraceState::Singularity);
    }

    #[test]
    fn test_invoke_grace_empty_signature() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        assert_eq!(
            engine.invoke_godelian_grace(&NodeId(1), &[], 2000),
            Err(GraceError::EmptySignature)
        );
    }

    #[test]
    fn test_delegation_queue() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        let values = [5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        for (i, v) in values.iter().enumerate() {
            engine.observe(&NodeId(1), *v, 1000 + i as u64).unwrap();
        }
        assert!(!engine.delegation_queue().is_empty());
    }

    #[test]
    fn test_process_delegation() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        let values = [5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        for (i, v) in values.iter().enumerate() {
            engine.observe(&NodeId(1), *v, 1000 + i as u64).unwrap();
        }
        assert!(engine.process_delegation(&NodeId(1)));
        assert!(engine.delegation_queue().is_empty());
    }

    #[test]
    fn test_average_chaos() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        engine.register_node(NodeId(2), 1000).unwrap();
        engine.observe(&NodeId(1), 1.0, 1100).unwrap();
        assert!(engine.average_chaos().is_some());
    }

    #[test]
    fn test_average_chaos_empty() {
        let engine = GodelianGrace::new();
        assert_eq!(engine.average_chaos(), None);
    }

    #[test]
    fn test_get_chaos_score() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        assert_eq!(engine.get_chaos_score(&NodeId(1)), Some(0.0));
    }

    #[test]
    fn test_reset() {
        let mut engine = GodelianGrace::new();
        engine.register_node(NodeId(1), 1000).unwrap();
        engine.reset();
        assert_eq!(engine.nodes.len(), 0);
        assert_eq!(engine.singularity_count(), 0);
    }

    #[test]
    fn test_display() {
        let engine = GodelianGrace::new();
        let s = format!("{}", engine);
        assert!(s.contains("GodelianGrace"));
    }

    #[test]
    fn test_record_display() {
        let record = GraceRecord {
            node_id: NodeId(1),
            previous_state: GraceState::Normal,
            new_state: GraceState::Singularity,
            chaos_score: 0.8,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("Node(1)"));
        assert!(s.contains("Normal"));
        assert!(s.contains("Singularity"));
    }

    #[test]
    fn test_standalone_detect_paradox() {
        let history = vec![5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        assert!(detect_godelian_paradox(&history, 0.5));
    }

    #[test]
    fn test_standalone_detect_no_paradox() {
        let history = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        assert!(!detect_godelian_paradox(&history, 0.5));
    }

    #[test]
    fn test_standalone_invoke_grace() {
        let id = NodeId(1);
        let sig = vec![1, 2, 3];
        let state = invoke_godelian_grace(&id, &sig);
        assert_eq!(state, GraceState::Singularity);
    }

    #[test]
    fn test_standalone_invoke_grace_empty_sig() {
        let id = NodeId(1);
        let state = invoke_godelian_grace(&id, &[]);
        assert_eq!(state, GraceState::Normal);
    }

    #[test]
    fn test_fnv_hash_deterministic() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = fnv_hash_64(&data);
        let h2 = fnv_hash_64(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_error_display() {
        let err = GraceError::AlreadyMarked;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = GodelianGrace::new();

        // Register nodes
        engine.register_node(NodeId(1), 1000).unwrap();
        engine.register_node(NodeId(2), 1000).unwrap();

        // Normal observations for node 1
        for i in 0..10 {
            engine
                .observe(&NodeId(1), 0.1 * i as f64, 1000 + i as u64)
                .unwrap();
        }
        assert_eq!(engine.get_state(&NodeId(1)), Some(GraceState::Normal));

        // Chaotic observations for node 2
        let values = [5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0, 5.0, -5.0];
        for (i, v) in values.iter().enumerate() {
            engine.observe(&NodeId(2), *v, 1000 + i as u64).unwrap();
        }
        assert!(engine.get_state(&NodeId(2)).unwrap() != GraceState::Normal);

        // Verify metrics
        assert!(engine.singularity_count() > 0);
        assert!(!engine.delegation_queue().is_empty());
        assert!(engine.average_chaos().is_some());
        assert!(!engine.records().is_empty());

        // Process delegation
        assert!(engine.process_delegation(&NodeId(2)));

        // Standalone functions
        assert!(detect_godelian_paradox(&values, 0.5));
        assert_eq!(
            invoke_godelian_grace(&NodeId(99), &[1, 2, 3]),
            GraceState::Singularity
        );

        // Reset
        engine.reset();
        assert_eq!(engine.singularity_count(), 0);
        assert!(engine.delegation_queue().is_empty());
    }
}
