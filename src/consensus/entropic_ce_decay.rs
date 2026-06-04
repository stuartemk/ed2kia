//! Entropic CE Decay â€” Sprint 77: Physics of Consciousness & Thermodynamic Finality
//!
//! Resuelve el bug ontolÃ³gico: CE oligÃ¡rquico (Gini â†’ 1.0).
//!
//! Implementa decaimiento entrÃ³pico radioactivo: CE(t) = CE_0Â·e^(-Î»t).
//! El mÃ©rito pasado se evapora. Influencia solo por contribuciÃ³n continua.
//! Evita acumulaciÃ³n perpetua de poder (Gini controlado).
//!
//! # GarantÃ­as
//!
//! - Decaimiento: O(1) por nodo, O(n) para red completa
//! - Gini: acotado por Î» (mayor Î» â†’ menor concentraciÃ³n)
//! - Cumplimiento: crÃ©ditos sin decaimiento = 0 influencia

use std::collections::HashMap;
use std::fmt;

/// Error types for Entropic CE Decay
#[derive(Debug, Clone, PartialEq)]
pub enum DecayError {
    /// Negative CE value
    NegativeCe(f64),
    /// Invalid decay constant (must be > 0)
    InvalidLambda(f64),
    /// Timestamp regression
    TimestampRegression(u64, u64),
    /// Node not found
    NodeNotFound(u64),
}

impl fmt::Display for DecayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecayError::NegativeCe(v) => write!(f, "Negative CE value: {:.4}", v),
            DecayError::InvalidLambda(l) => write!(f, "Invalid lambda: {:.6} (must be > 0)", l),
            DecayError::TimestampRegression(old, new) => {
                write!(f, "Timestamp regression: {} â†’ {}", old, new)
            }
            DecayError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
        }
    }
}

impl std::error::Error for DecayError {}

/// Configuration for Entropic CE Decay.
#[derive(Debug, Clone)]
pub struct DecayConfig {
    /// Decay constant Î» (default 0.00001 â‰ˆ half-life ~69.3k ms)
    pub lambda: f64,
    /// Maximum CE a node can hold
    pub max_ce: f64,
    /// Minimum CE before node is considered inactive
    pub min_ce_threshold: f64,
    /// Gini warning threshold
    pub gini_warning_threshold: f64,
}

impl DecayConfig {
    /// Default Topological configuration.
    pub fn default_Topological() -> Self {
        Self {
            lambda: 0.000_01,
            max_ce: 1000.0,
            min_ce_threshold: 0.001,
            gini_warning_threshold: 0.6,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), DecayError> {
        if self.lambda <= 0.0 {
            return Err(DecayError::InvalidLambda(self.lambda));
        }
        if self.max_ce <= 0.0 {
            return Err(DecayError::NegativeCe(-self.max_ce));
        }
        Ok(())
    }
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// CE state for a single node.
#[derive(Debug, Clone)]
pub struct CeNodeState {
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
}

impl CeNodeState {
    pub fn new(node_id: u64, initial_ce: f64, timestamp_ms: u64) -> Result<Self, DecayError> {
        if initial_ce < 0.0 {
            return Err(DecayError::NegativeCe(initial_ce));
        }
        Ok(Self {
            node_id,
            ce_0: initial_ce,
            current_ce: initial_ce,
            last_contribution_ms: timestamp_ms,
            contribution_count: 1,
        })
    }

    /// Apply entropic decay to current CE.
    pub fn apply_decay(&mut self, lambda: f64, current_ms: u64) {
        let delta_t = current_ms.saturating_sub(self.last_contribution_ms) as f64;
        self.current_ce = self.ce_0 * (-lambda * delta_t).exp();
    }

    /// Record new contribution, boosting CE.
    pub fn contribute(&mut self, amount: f64, max_ce: f64, timestamp_ms: u64) {
        self.ce_0 = (self.ce_0 + amount).min(max_ce);
        self.current_ce = self.ce_0;
        self.last_contribution_ms = timestamp_ms;
        self.contribution_count += 1;
    }
}

impl fmt::Display for CeNodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CeNode {{ id={}, ce_0={:.4}, current={:.4}, decay={:.2}%, contributions={} }}",
            self.node_id,
            self.ce_0,
            self.current_ce,
            if self.ce_0 > 0.0 {
                (1.0 - self.current_ce / self.ce_0) * 100.0
            } else {
                0.0
            },
            self.contribution_count
        )
    }
}

/// Result of a Gini coefficient calculation.
#[derive(Debug, Clone)]
pub struct GiniResult {
    /// Gini coefficient (0.0 = perfect equality, 1.0 = maximal inequality)
    pub gini: f64,
    /// Total CE in the system
    pub total_ce: f64,
    /// Number of active nodes
    pub active_nodes: usize,
    /// Whether Gini exceeds warning threshold
    pub warning: bool,
}

impl fmt::Display for GiniResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Gini {{ coeff={:.4}, total_ce={:.2}, active={}, warning={} }}",
            self.gini, self.total_ce, self.active_nodes, self.warning
        )
    }
}

/// Stateful engine for Entropic CE Decay.
#[derive(Debug, Clone)]
pub struct EntropicCeDecay {
    config: DecayConfig,
    nodes: HashMap<u64, CeNodeState>,
}

impl EntropicCeDecay {
    /// Create a new engine with default Topological configuration.
    pub fn new() -> Self {
        Self {
            config: DecayConfig::default_Topological(),
            nodes: HashMap::new(),
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: DecayConfig) -> Result<Self, DecayError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
        })
    }

    /// Register a new node with initial CE.
    pub fn register_node(
        &mut self,
        node_id: u64,
        initial_ce: f64,
        timestamp_ms: u64,
    ) -> Result<(), DecayError> {
        let state = CeNodeState::new(node_id, initial_ce, timestamp_ms)?;
        self.nodes.insert(node_id, state);
        Ok(())
    }

    /// Record a contribution from a node.
    pub fn contribute(
        &mut self,
        node_id: u64,
        amount: f64,
        timestamp_ms: u64,
    ) -> Result<(), DecayError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(DecayError::NodeNotFound(node_id))?;
        node.contribute(amount, self.config.max_ce, timestamp_ms);
        Ok(())
    }

    /// Apply decay to all nodes based on current timestamp.
    pub fn apply_global_decay(&mut self, current_ms: u64) {
        for node in self.nodes.values_mut() {
            node.apply_decay(self.config.lambda, current_ms);
        }
    }

    /// Get current CE for a node (after decay).
    pub fn get_ce(&self, node_id: u64) -> Result<f64, DecayError> {
        self.nodes
            .get(&node_id)
            .map(|n| n.current_ce)
            .ok_or(DecayError::NodeNotFound(node_id))
    }

    /// Compute Gini coefficient of CE distribution.
    pub fn compute_gini(&self) -> GiniResult {
        let ces: Vec<f64> = self.nodes.values().map(|n| n.current_ce).collect();
        let total_ce: f64 = ces.iter().sum();
        let active_nodes = ces.len();

        if active_nodes == 0 || total_ce == 0.0 {
            return GiniResult {
                gini: 0.0,
                total_ce,
                active_nodes,
                warning: false,
            };
        }

        let mut sorted = ces;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted.len() as f64;
        let numerator: f64 = sorted
            .iter()
            .enumerate()
            .map(|(i, &val)| (i as f64 + 1.0) * val)
            .sum::<f64>();
        let denominator = n * total_ce;

        let gini = if denominator > 0.0 {
            2.0 * numerator / denominator - (n + 1.0) / n
        } else {
            0.0
        };

        GiniResult {
            gini: gini.max(0.0).min(1.0),
            total_ce,
            active_nodes,
            warning: gini > self.config.gini_warning_threshold,
        }
    }

    /// Get inactive nodes (CE below threshold).
    pub fn get_inactive_nodes(&self) -> Vec<u64> {
        self.nodes
            .values()
            .filter(|n| n.current_ce < self.config.min_ce_threshold)
            .map(|n| n.node_id)
            .collect()
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total CE in the system.
    pub fn total_ce(&self) -> f64 {
        self.nodes.values().map(|n| n.current_ce).sum()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.nodes.clear();
    }
}

impl Default for EntropicCeDecay {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntropicCeDecay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let gini = self.compute_gini();
        write!(
            f,
            "EntropicCeDecay {{ nodes={}, total_ce={:.2}, gini={:.4}, lambda={:.6} }}",
            self.node_count(),
            self.total_ce(),
            gini.gini,
            self.config.lambda
        )
    }
}

// â”€â”€â”€ Public Standalone Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Apply entropic decay to a single CE value.
///
/// CE(t) = CE_0 Â· e^(-Î» Â· Î”t)
pub fn apply_entropic_decay(
    initial_ce: f64,
    last_contribution_ts: u64,
    current_ts: u64,
    lambda: f64,
) -> f64 {
    let delta_t = current_ts.saturating_sub(last_contribution_ts) as f64;
    initial_ce * (-lambda * delta_t).exp()
}

/// Compute half-life from decay constant.
pub fn half_life(lambda: f64) -> f64 {
    if lambda <= 0.0 {
        return f64::INFINITY;
    }
    std::f64::consts::LN_2 / lambda
}

/// Compute Gini coefficient from a slice of values.
pub fn compute_gini_coefficient(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let total: f64 = values.iter().sum();
    if total == 0.0 {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len() as f64;
    let numerator: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, &val)| (i as f64 + 1.0) * val)
        .sum();
    let denominator = n * total;
    if denominator == 0.0 {
        return 0.0;
    }
    (2.0 * numerator / denominator - (n + 1.0) / n)
        .max(0.0)
        .min(1.0)
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DecayConfig::default_Topological();
        assert!(config.validate().is_ok());
        assert!(config.lambda > 0.0);
    }

    #[test]
    fn test_config_invalid_lambda() {
        let config = DecayConfig {
            lambda: 0.0,
            ..DecayConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_creation() {
        let node = CeNodeState::new(1, 100.0, 1000).unwrap();
        assert_eq!(node.ce_0, 100.0);
        assert_eq!(node.current_ce, 100.0);
        assert_eq!(node.contribution_count, 1);
    }

    #[test]
    fn test_node_negative_ce_error() {
        assert!(CeNodeState::new(1, -1.0, 1000).is_err());
    }

    #[test]
    fn test_apply_decay_no_time_passage() {
        let mut node = CeNodeState::new(1, 100.0, 1000).unwrap();
        node.apply_decay(0.001, 1000);
        assert!((node.current_ce - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_decay_with_time_passage() {
        let mut node = CeNodeState::new(1, 100.0, 1000).unwrap();
        node.apply_decay(0.001, 2000);
        assert!(node.current_ce < 100.0);
        assert!(node.current_ce > 0.0);
    }

    #[test]
    fn test_contribute_boosts_ce() {
        let mut node = CeNodeState::new(1, 100.0, 1000).unwrap();
        node.contribute(50.0, 1000.0, 2000);
        assert_eq!(node.ce_0, 150.0);
        assert_eq!(node.current_ce, 150.0);
        assert_eq!(node.contribution_count, 2);
    }

    #[test]
    fn test_contribute_caps_at_max() {
        let mut node = CeNodeState::new(1, 900.0, 1000).unwrap();
        node.contribute(200.0, 1000.0, 2000);
        assert_eq!(node.ce_0, 1000.0);
    }

    #[test]
    fn test_engine_creation() {
        let engine = EntropicCeDecay::new();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        assert_eq!(engine.node_count(), 1);
    }

    #[test]
    fn test_contribute_via_engine() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.contribute(1, 50.0, 2000).unwrap();
        assert_eq!(engine.get_ce(1).unwrap(), 150.0);
    }

    #[test]
    fn test_contribute_unknown_node() {
        let mut engine = EntropicCeDecay::new();
        assert!(engine.contribute(999, 50.0, 2000).is_err());
    }

    #[test]
    fn test_global_decay() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.apply_global_decay(10_000);
        assert!(engine.get_ce(1).unwrap() < 100.0);
    }

    #[test]
    fn test_gini_perfect_equality() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 100.0, 1000).unwrap();
        engine.register_node(3, 100.0, 1000).unwrap();
        let gini = engine.compute_gini();
        assert!((gini.gini - 0.0).abs() < 1e-10);
        assert!(!gini.warning);
    }

    #[test]
    fn test_gini_inequality() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 1000.0, 1000).unwrap();
        engine.register_node(2, 1.0, 1000).unwrap();
        engine.register_node(3, 1.0, 1000).unwrap();
        let gini = engine.compute_gini();
        assert!(gini.gini > 0.5);
    }

    #[test]
    fn test_gini_warning() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 1000.0, 1000).unwrap();
        engine.register_node(2, 0.001, 1000).unwrap();
        engine.register_node(3, 0.001, 1000).unwrap();
        let gini = engine.compute_gini();
        assert!(gini.warning);
    }

    #[test]
    fn test_inactive_nodes() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 0.0001, 1000).unwrap();
        let inactive = engine.get_inactive_nodes();
        assert!(inactive.contains(&2));
        assert!(!inactive.contains(&1));
    }

    #[test]
    fn test_total_ce() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 200.0, 1000).unwrap();
        assert!((engine.total_ce() - 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_reset() {
        let mut engine = EntropicCeDecay::new();
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.reset();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_display() {
        let engine = EntropicCeDecay::new();
        let s = format!("{}", engine);
        assert!(s.contains("EntropicCeDecay"));
    }

    #[test]
    fn test_node_display() {
        let node = CeNodeState::new(1, 100.0, 1000).unwrap();
        let s = format!("{}", node);
        assert!(s.contains("CeNode"));
    }

    #[test]
    fn test_standalone_decay() {
        let ce = apply_entropic_decay(100.0, 1000, 2000, 0.001);
        assert!(ce < 100.0);
        assert!(ce > 0.0);
    }

    #[test]
    fn test_standalone_decay_zero_time() {
        let ce = apply_entropic_decay(100.0, 1000, 1000, 0.001);
        assert!((ce - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_half_life() {
        let hl = half_life(0.000_01);
        assert!(hl > 0.0);
        // ln(2) / 0.00001 â‰ˆ 69314.7
        assert!((hl - 69314.7).abs() < 1.0);
    }

    #[test]
    fn test_half_life_zero_lambda() {
        let hl = half_life(0.0);
        assert!(hl.is_infinite());
    }

    #[test]
    fn test_compute_gini_coefficient_empty() {
        assert!((compute_gini_coefficient(&[]) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_gini_coefficient_equal() {
        let vals = vec![10.0, 10.0, 10.0];
        assert!((compute_gini_coefficient(&vals) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_gini_coefficient_unequal() {
        let vals = vec![100.0, 1.0, 1.0];
        let g = compute_gini_coefficient(&vals);
        assert!(g > 0.5);
        assert!(g <= 1.0);
    }

    #[test]
    fn test_error_display() {
        let err = DecayError::NegativeCe(-5.0);
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = EntropicCeDecay::new();

        // Register nodes
        engine.register_node(1, 100.0, 1000).unwrap();
        engine.register_node(2, 100.0, 1000).unwrap();

        // Node 1 contributes continuously
        engine.contribute(1, 50.0, 5000).unwrap();
        engine.contribute(1, 50.0, 10_000).unwrap();

        // Apply decay
        engine.apply_global_decay(20_000);

        // Node 1 should have higher CE (continuous contribution)
        let ce1 = engine.get_ce(1).unwrap();
        let ce2 = engine.get_ce(2).unwrap();
        assert!(ce1 > ce2);

        // Gini should reflect inequality
        let gini = engine.compute_gini();
        assert!(gini.gini > 0.0);
    }
}
