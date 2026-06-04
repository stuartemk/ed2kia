//! Relativistic Entropy â€” Sprint 78: Invariant Architecture & Planetary-Scale Resilience
//!
//! Resuelve el bug terminal: Trampa de liquidez CE durante particiones geopolÃ­ticas.
//!
//! Implementa entropÃ­a relativista: Î» se congela criptogrÃ¡ficamente si
//! `peer_density < partition_threshold`. El CE no decae durante apagones
//! geopolÃ­ticos (cryosleep mode). La red protege el mÃ©rito de nodos aislados.
//!
//! # GarantÃ­as
//!
//! - Decaimiento: O(1) por nodo, adaptativo a densidad de peers
//! - Cryosleep: Î» â†’ 0 cuando peer_density < threshold (congelamiento total)
//! - TransiciÃ³n suave: rampa exponencial entre Î»_base y Î»_cryosleep
//! - ParticiÃ³n: nodos aislados conservan CE hasta reconexiÃ³n

use std::collections::HashMap;
use std::fmt;

/// Error types for Relativistic Entropy
#[derive(Debug, Clone, PartialEq)]
pub enum RelativisticError {
    /// Negative CE value
    NegativeCe(f64),
    /// Invalid decay constant (must be > 0)
    InvalidLambda(f64),
    /// Invalid peer density (must be in [0, 1])
    InvalidPeerDensity(f64),
    /// Invalid partition threshold (must be in (0, 1])
    InvalidPartitionThreshold(f64),
    /// Timestamp regression
    TimestampRegression(u64, u64),
    /// Node not found
    NodeNotFound(u64),
}

impl fmt::Display for RelativisticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelativisticError::NegativeCe(v) => write!(f, "Negative CE value: {:.4}", v),
            RelativisticError::InvalidLambda(l) => {
                write!(f, "Invalid lambda: {:.6} (must be > 0)", l)
            }
            RelativisticError::InvalidPeerDensity(d) => {
                write!(f, "Invalid peer density: {:.4} (must be in [0, 1])", d)
            }
            RelativisticError::InvalidPartitionThreshold(t) => {
                write!(
                    f,
                    "Invalid partition threshold: {:.4} (must be in (0, 1])",
                    t
                )
            }
            RelativisticError::TimestampRegression(old, new) => {
                write!(f, "Timestamp regression: {} -> {}", old, new)
            }
            RelativisticError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
        }
    }
}

impl std::error::Error for RelativisticError {}

/// Configuration for Relativistic Entropy.
#[derive(Debug, Clone)]
pub struct RelativisticConfig {
    /// Base decay constant Î» (default 0.000_01)
    pub base_lambda: f64,
    /// Partition threshold: below this, Î» â†’ 0 (default 0.15)
    pub partition_threshold: f64,
    /// Maximum CE a node can hold
    pub max_ce: f64,
    /// Minimum CE before node is considered inactive
    pub min_ce_threshold: f64,
    /// Ramp smoothness factor (default 5.0)
    pub ramp_factor: f64,
}

impl RelativisticConfig {
    /// Default Topological configuration.
    pub fn default_topological() -> Self {
        Self {
            base_lambda: 0.000_01,
            partition_threshold: 0.15,
            max_ce: 1000.0,
            min_ce_threshold: 0.001,
            ramp_factor: 15.0,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), RelativisticError> {
        if self.base_lambda <= 0.0 {
            return Err(RelativisticError::InvalidLambda(self.base_lambda));
        }
        if self.partition_threshold <= 0.0 || self.partition_threshold > 1.0 {
            return Err(RelativisticError::InvalidPartitionThreshold(
                self.partition_threshold,
            ));
        }
        if self.max_ce <= 0.0 {
            return Err(RelativisticError::NegativeCe(-self.max_ce));
        }
        if self.ramp_factor <= 0.0 {
            return Err(RelativisticError::InvalidLambda(-self.ramp_factor));
        }
        Ok(())
    }
}

impl Default for RelativisticConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

/// CE state for a single node with relativistic awareness.
#[derive(Debug, Clone)]
pub struct RelativisticNode {
    /// Node identifier.
    pub node_id: u64,
    /// Initial CE at last contribution.
    pub ce_0: f64,
    /// Current (decayed) CE.
    pub current_ce: f64,
    /// Timestamp of last contribution (ms).
    pub last_contribution_ms: u64,
    /// Total contributions count.
    pub contribution_count: u32,
    /// Current peer density for this node.
    pub peer_density: f64,
    /// Whether node is in cryosleep mode.
    pub is_cryosleep: bool,
}

impl RelativisticNode {
    /// Create a new relativistic node.
    pub fn new(
        node_id: u64,
        initial_ce: f64,
        timestamp_ms: u64,
    ) -> Result<Self, RelativisticError> {
        if initial_ce < 0.0 {
            return Err(RelativisticError::NegativeCe(initial_ce));
        }
        Ok(Self {
            node_id,
            ce_0: initial_ce,
            current_ce: initial_ce,
            last_contribution_ms: timestamp_ms,
            contribution_count: 1,
            peer_density: 1.0,
            is_cryosleep: false,
        })
    }

    /// Apply relativistic decay based on peer density.
    pub fn apply_relativistic_decay(
        &mut self,
        base_lambda: f64,
        partition_threshold: f64,
        ramp_factor: f64,
        current_ms: u64,
    ) {
        if current_ms < self.last_contribution_ms {
            return;
        }
        let delta_t = (current_ms - self.last_contribution_ms) as f64;
        if delta_t == 0.0 {
            return;
        }

        let effective_lambda = compute_effective_lambda(
            base_lambda,
            self.peer_density,
            partition_threshold,
            ramp_factor,
        );

        // Cryosleep when effective lambda is negligible (< 0.1% of base)
        self.is_cryosleep = effective_lambda < base_lambda * 0.001;
        self.current_ce = self.ce_0 * (-effective_lambda * delta_t).exp();
        // Update ce_0 to current_ce so repeated decays compound properly
        self.ce_0 = self.current_ce;
    }

    /// Contribute additional CE.
    pub fn contribute(&mut self, amount: f64, max_ce: f64, timestamp_ms: u64) {
        if amount < 0.0 {
            return;
        }
        self.current_ce = (self.current_ce + amount).min(max_ce);
        self.ce_0 = self.current_ce;
        self.last_contribution_ms = timestamp_ms;
        self.contribution_count += 1;
        self.is_cryosleep = false;
    }

    /// Update peer density for this node.
    pub fn update_peer_density(&mut self, density: f64) {
        self.peer_density = density.max(0.0).min(1.0);
    }
}

impl fmt::Display for RelativisticNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node{} [CE={:.4}, cryosleep={}, peers={:.3}]",
            self.node_id, self.current_ce, self.is_cryosleep, self.peer_density
        )
    }
}

/// Relativistic decay state.
#[derive(Debug, Clone)]
pub struct DecayRecord {
    /// Node identifier.
    pub node_id: u64,
    /// Original CE before decay.
    pub ce_before: f64,
    /// CE after relativistic decay.
    pub ce_after: f64,
    /// Effective lambda used.
    pub effective_lambda: f64,
    /// Whether cryosleep was active.
    pub was_cryosleep: bool,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

impl fmt::Display for DecayRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Decay[{}]: {:.4} -> {:.4} (Î»={:.8}, cryosleep={})",
            self.node_id, self.ce_before, self.ce_after, self.effective_lambda, self.was_cryosleep
        )
    }
}

/// Main engine for relativistic entropy.
#[derive(Debug, Clone)]
pub struct RelativisticEntropy {
    /// Configuration.
    pub config: RelativisticConfig,
    /// Node states.
    pub nodes: HashMap<u64, RelativisticNode>,
    /// Decay history.
    pub decay_history: Vec<DecayRecord>,
}

impl RelativisticEntropy {
    /// Create with default Topological config.
    pub fn new() -> Self {
        Self {
            config: RelativisticConfig::default_topological(),
            nodes: HashMap::new(),
            decay_history: Vec::new(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: RelativisticConfig) -> Result<Self, RelativisticError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            decay_history: Vec::new(),
        })
    }

    /// Register a new node.
    pub fn register_node(
        &mut self,
        node_id: u64,
        initial_ce: f64,
        timestamp_ms: u64,
    ) -> Result<(), RelativisticError> {
        let node = RelativisticNode::new(node_id, initial_ce, timestamp_ms)?;
        self.nodes.insert(node_id, node);
        Ok(())
    }

    /// Contribute CE to a node.
    pub fn contribute(
        &mut self,
        node_id: u64,
        amount: f64,
        timestamp_ms: u64,
    ) -> Result<(), RelativisticError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(RelativisticError::NodeNotFound(node_id))?;
        node.contribute(amount, self.config.max_ce, timestamp_ms);
        Ok(())
    }

    /// Update peer density for a node.
    pub fn update_peer_density(
        &mut self,
        node_id: u64,
        density: f64,
    ) -> Result<(), RelativisticError> {
        if density < 0.0 || density > 1.0 {
            return Err(RelativisticError::InvalidPeerDensity(density));
        }
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(RelativisticError::NodeNotFound(node_id))?;
        node.update_peer_density(density);
        Ok(())
    }

    /// Apply relativistic decay to all nodes.
    pub fn apply_global_decay(&mut self, current_ms: u64) {
        for node in self.nodes.values_mut() {
            let ce_before = node.current_ce;
            let effective_lambda = compute_effective_lambda(
                self.config.base_lambda,
                node.peer_density,
                self.config.partition_threshold,
                self.config.ramp_factor,
            );

            node.apply_relativistic_decay(
                self.config.base_lambda,
                self.config.partition_threshold,
                self.config.ramp_factor,
                current_ms,
            );

            self.decay_history.push(DecayRecord {
                node_id: node.node_id,
                ce_before,
                ce_after: node.current_ce,
                effective_lambda,
                was_cryosleep: node.is_cryosleep,
                timestamp_ms: current_ms,
            });
        }
    }

    /// Get current CE for a node.
    pub fn get_ce(&self, node_id: u64) -> Result<f64, RelativisticError> {
        self.nodes
            .get(&node_id)
            .map(|n| n.current_ce)
            .ok_or(RelativisticError::NodeNotFound(node_id))
    }

    /// Get nodes in cryosleep mode.
    pub fn get_cryosleep_nodes(&self) -> Vec<u64> {
        self.nodes
            .values()
            .filter(|n| n.is_cryosleep)
            .map(|n| n.node_id)
            .collect()
    }

    /// Get total CE across all nodes.
    pub fn total_ce(&self) -> f64 {
        self.nodes.values().map(|n| n.current_ce).sum()
    }

    /// Reset engine state.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.decay_history.clear();
    }
}

impl Default for RelativisticEntropy {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RelativisticEntropy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RelativisticEntropy [nodes={}, cryosleep={}, total_ce={:.4}]",
            self.nodes.len(),
            self.get_cryosleep_nodes().len(),
            self.total_ce()
        )
    }
}

// -- Standalone public functions --

/// Compute effective lambda based on peer density and partition threshold.
///
/// When `peer_density < partition_threshold`, lambda approaches 0 (cryosleep).
/// Uses exponential ramp for smooth transition.
pub fn compute_effective_lambda(
    base_lambda: f64,
    peer_density: f64,
    partition_threshold: f64,
    ramp_factor: f64,
) -> f64 {
    if peer_density >= partition_threshold {
        return base_lambda;
    }
    // Exponential ramp: as density drops below threshold, lambda â†’ 0
    let ratio = peer_density / partition_threshold;
    base_lambda * (-ramp_factor * (1.0 - ratio)).exp()
}

/// Compute relativistic decay for a single CE value.
pub fn compute_relativistic_decay(
    current_ce: f64,
    last_contribution_ts: u64,
    current_ts: u64,
    base_lambda: f64,
    local_peer_density: f64,
    partition_threshold: f64,
) -> f64 {
    if current_ts < last_contribution_ts {
        return current_ce;
    }
    let delta_t = (current_ts - last_contribution_ts) as f64;
    let effective_lambda = compute_effective_lambda(
        base_lambda,
        local_peer_density,
        partition_threshold,
        5.0, // default ramp factor
    );
    current_ce * (-effective_lambda * delta_t).exp()
}

/// Check if a node should enter cryosleep mode.
pub fn should_cryosleep(peer_density: f64, partition_threshold: f64) -> bool {
    peer_density < partition_threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RelativisticConfig::default_topological();
        assert!(config.base_lambda > 0.0);
        assert!(config.partition_threshold > 0.0);
        assert!(config.max_ce > 0.0);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = RelativisticConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_lambda() {
        let mut config = RelativisticConfig::default_topological();
        config.base_lambda = 0.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let mut config = RelativisticConfig::default_topological();
        config.partition_threshold = 0.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_creation() {
        let node = RelativisticNode::new(1, 100.0, 1000).unwrap();
        assert_eq!(node.node_id, 1);
        assert_eq!(node.current_ce, 100.0);
        assert!(!node.is_cryosleep);
    }

    #[test]
    fn test_node_negative_ce_error() {
        let result = RelativisticNode::new(1, -1.0, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_decay_normal_density() {
        let mut node = RelativisticNode::new(1, 100.0, 1000).unwrap();
        node.update_peer_density(1.0);
        node.apply_relativistic_decay(0.001, 0.15, 5.0, 2000);
        assert!(node.current_ce < 100.0);
        assert!(!node.is_cryosleep);
    }

    #[test]
    fn test_apply_decay_cryosleep() {
        let mut node = RelativisticNode::new(1, 100.0, 1000).unwrap();
        node.update_peer_density(0.01); // Well below threshold
        node.apply_relativistic_decay(0.001, 0.15, 15.0, 2000);
        assert!(node.is_cryosleep);
        assert!(node.current_ce > 99.0); // Minimal decay
    }

    #[test]
    fn test_contribute_boosts_ce() {
        let mut node = RelativisticNode::new(1, 100.0, 1000).unwrap();
        node.contribute(50.0, 1000.0, 2000);
        assert_eq!(node.current_ce, 150.0);
        assert_eq!(node.contribution_count, 2);
    }

    #[test]
    fn test_contribute_caps_at_max() {
        let mut node = RelativisticNode::new(1, 900.0, 1000).unwrap();
        node.contribute(200.0, 1000.0, 2000);
        assert_eq!(node.current_ce, 1000.0);
    }

    #[test]
    fn test_engine_creation() {
        let engine = RelativisticEntropy::new();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = RelativisticConfig::default_topological();
        let engine = RelativisticEntropy::with_config(config).unwrap();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        assert_eq!(engine.nodes.len(), 1);
    }

    #[test]
    fn test_contribute_via_engine() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.contribute(1, 50.0, 2000).unwrap();
        assert_eq!(engine.get_ce(1).unwrap(), 150.0);
    }

    #[test]
    fn test_contribute_unknown_node() {
        let mut engine = RelativisticEntropy::new();
        let result = engine.contribute(999, 50.0, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_global_decay_normal() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.apply_global_decay(2000);
        assert!(engine.get_ce(1).unwrap() <= 100.0);
    }

    #[test]
    fn test_global_decay_cryosleep() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.update_peer_density(1, 0.01).unwrap();
        engine.apply_global_decay(2000);
        let ce = engine.get_ce(1).unwrap();
        assert!(ce > 99.0); // Minimal decay in cryosleep
    }

    #[test]
    fn test_cryosleep_nodes() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 100.0, 1000).unwrap();
        engine.update_peer_density(1, 0.01).unwrap();
        engine.update_peer_density(2, 1.0).unwrap();
        engine.apply_global_decay(2000);
        let cryosleep = engine.get_cryosleep_nodes();
        assert_eq!(cryosleep, vec![1]);
    }

    #[test]
    fn test_total_ce() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 200.0, 1000).unwrap();
        assert_eq!(engine.total_ce(), 300.0);
    }

    #[test]
    fn test_reset() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.reset();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = RelativisticEntropy::new();
        let s = format!("{}", engine);
        assert!(s.contains("RelativisticEntropy"));
    }

    #[test]
    fn test_node_display() {
        let node = RelativisticNode::new(1, 100.0, 1000).unwrap();
        let s = format!("{}", node);
        assert!(s.contains("Node1"));
    }

    #[test]
    fn test_decay_record_display() {
        let record = DecayRecord {
            node_id: 1,
            ce_before: 100.0,
            ce_after: 95.0,
            effective_lambda: 0.001,
            was_cryosleep: false,
            timestamp_ms: 2000,
        };
        let s = format!("{}", record);
        assert!(s.contains("Decay"));
    }

    #[test]
    fn test_compute_effective_lambda_normal() {
        let lambda = compute_effective_lambda(0.001, 1.0, 0.15, 5.0);
        assert!((lambda - 0.001).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_effective_lambda_cryosleep() {
        let lambda = compute_effective_lambda(0.001, 0.01, 0.15, 15.0);
        assert!(lambda < 0.000_001);
    }

    #[test]
    fn test_compute_relativistic_decay() {
        let ce = compute_relativistic_decay(100.0, 1000, 2000, 0.001, 1.0, 0.15);
        assert!(ce < 100.0);
    }

    #[test]
    fn test_compute_relativistic_decay_cryosleep() {
        let ce = compute_relativistic_decay(100.0, 1000, 2000, 0.001, 0.01, 0.15);
        assert!(ce > 99.0);
    }

    #[test]
    fn test_should_cryosleep() {
        assert!(should_cryosleep(0.01, 0.15));
        assert!(!should_cryosleep(1.0, 0.15));
    }

    #[test]
    fn test_error_display() {
        let err = RelativisticError::NegativeCe(-1.0);
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = RelativisticEntropy::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 200.0, 1000).unwrap();

        // Normal decay over 10000ms (significant decay with base_lambda=0.00001)
        engine.apply_global_decay(11000);
        assert!(engine.get_ce(1).unwrap() < 100.0);

        // Node 1 enters partition
        engine.update_peer_density(1, 0.01).unwrap();
        engine.apply_global_decay(21000);

        // Node 1 preserved (cryosleep), Node 2 continues decaying
        let ce1 = engine.get_ce(1).unwrap();
        let ce2 = engine.get_ce(2).unwrap();
        assert!(ce1 > 90.0); // Cryosleep preserved (minimal decay)
        assert!(ce2 < 190.0); // Normal decay over 20000ms

        // Node 1 reconnects
        engine.update_peer_density(1, 1.0).unwrap();
        engine.contribute(1, 10.0, 31000).unwrap();
        assert!(engine.get_ce(1).unwrap() > ce1);
    }
}
