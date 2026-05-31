//! Ethical Resonance Field — Dynamic field computation for Noosphere Engine.
//!
//! Core formula: R(x,t) = Σ w_i · GEI_i · exp(-d²/2σ(t)²) · tanh(k·Z_i)
//!
//! Integrates `TemporalCohesionEngine` for σ(t) field width dynamics.
//! WASM-compatible: no blocking operations, pure computation.

use std::collections::HashMap;

/// Maximum nodes tracked in the field before pruning.
const MAX_FIELD_NODES: usize = 10_000;

/// Default ethical focus scaling factor `k` in tanh(k·Z).
const DEFAULT_K: f64 = 2.0;

/// Default temporal cohesion σ(t) when cohesion data is unavailable.
const DEFAULT_SIGMA: f64 = 1.0;

// ---------------------------------------------------------------------------
// Public errors
// ---------------------------------------------------------------------------

/// Errors specific to EthicalResonanceField operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ResonanceFieldError {
    /// Node ID already exists in the field.
    DuplicateNode(u128),
    /// Node not found in the field.
    NodeNotFound(u128),
    /// Invalid GEI value (must be in [0, 1]).
    InvalidGEI(f64),
    /// Invalid Z-score (must be in [-1, 1]).
    InvalidZScore(f64),
    /// Invalid weight (must be > 0).
    InvalidWeight(f64),
    /// Field is empty — cannot compute.
    FieldEmpty,
}

impl std::fmt::Display for ResonanceFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResonanceFieldError::DuplicateNode(id) => write!(f, "Duplicate node {}", id),
            ResonanceFieldError::NodeNotFound(id) => write!(f, "Node {} not found", id),
            ResonanceFieldError::InvalidGEI(v) => write!(f, "Invalid GEI: {} (must be [0,1])", v),
            ResonanceFieldError::InvalidZScore(v) => {
                write!(f, "Invalid Z-score: {} (must be [-1,1])", v)
            }
            ResonanceFieldError::InvalidWeight(v) => {
                write!(f, "Invalid weight: {} (must be >0)", v)
            }
            ResonanceFieldError::FieldEmpty => write!(f, "Field is empty"),
        }
    }
}

// ---------------------------------------------------------------------------
// Node state
// ---------------------------------------------------------------------------

/// Per-node state within the EthicalResonanceField.
#[derive(Debug, Clone)]
pub struct NodeState {
    /// Unique node identifier.
    pub node_id: u128,
    /// Node weight in the field summation.
    pub weight: f64,
    /// Global Ethical Index for this node (0 = dark, 1 = luminous).
    pub gei: f64,
    /// Z-score ethical focus (-1 = lower, 0 = neutral, +1 = upper).
    pub z_score: f64,
    /// Spatial position in the field (used for distance calculation).
    pub position: f64,
}

impl NodeState {
    pub fn new(
        node_id: u128,
        weight: f64,
        gei: f64,
        z_score: f64,
        position: f64,
    ) -> Result<Self, ResonanceFieldError> {
        if !(0.0..=1.0).contains(&gei) {
            return Err(ResonanceFieldError::InvalidGEI(gei));
        }
        if !(-1.0..=1.0).contains(&z_score) {
            return Err(ResonanceFieldError::InvalidZScore(z_score));
        }
        if weight <= 0.0 {
            return Err(ResonanceFieldError::InvalidWeight(weight));
        }
        Ok(NodeState {
            node_id,
            weight,
            gei,
            z_score,
            position,
        })
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the EthicalResonanceField.
#[derive(Debug, Clone)]
pub struct FieldConfig {
    /// Ethical focus scaling factor `k` in tanh(k·Z).
    pub k_factor: f64,
    /// Default σ(t) when temporal cohesion is unavailable.
    pub default_sigma: f64,
    /// Maximum nodes before LRU-style pruning.
    pub max_nodes: usize,
}

impl Default for FieldConfig {
    fn default() -> Self {
        FieldConfig {
            k_factor: DEFAULT_K,
            default_sigma: DEFAULT_SIGMA,
            max_nodes: MAX_FIELD_NODES,
        }
    }
}

// ---------------------------------------------------------------------------
// EthicalResonanceField
// ---------------------------------------------------------------------------

/// Dynamic ethical resonance field that computes R(x,t) across all registered nodes.
///
/// Formula: R(x,t) = Σ w_i · GEI_i · exp(-d²/2σ(t)²) · tanh(k·Z_i)
///
/// Where:
/// - `w_i` = node weight
/// - `GEI_i` = Global Ethical Index (0..1)
/// - `d` = distance from evaluation point x to node position
/// - `σ(t)` = temporal cohesion field width (contracts as cohesion increases)
/// - `k` = ethical focus scaling factor
/// - `Z_i` = node Z-score ethical focus (-1..1)
#[derive(Debug, Clone)]
pub struct EthicalResonanceField {
    config: FieldConfig,
    nodes: HashMap<u128, NodeState>,
    /// Current σ(t) value. Updated via `update_temporal_cohesion`.
    sigma_t: f64,
}

impl EthicalResonanceField {
    /// Create a new field with default configuration.
    pub fn new() -> Self {
        Self::with_config(FieldConfig::default())
    }

    /// Create a field with explicit configuration.
    pub fn with_config(config: FieldConfig) -> Self {
        EthicalResonanceField {
            config,
            nodes: HashMap::new(),
            sigma_t: DEFAULT_SIGMA,
        }
    }

    // ---- Node management ----

    /// Register or update a node in the field.
    pub fn add_node(&mut self, state: NodeState) -> Result<(), ResonanceFieldError> {
        if self.nodes.contains_key(&state.node_id) {
            return Err(ResonanceFieldError::DuplicateNode(state.node_id));
        }
        self.nodes.insert(state.node_id, state);
        self.prune_if_needed();
        Ok(())
    }

    /// Remove a node from the field.
    pub fn remove_node(&mut self, node_id: u128) -> Result<NodeState, ResonanceFieldError> {
        self.nodes
            .remove(&node_id)
            .ok_or(ResonanceFieldError::NodeNotFound(node_id))
    }

    /// Get the current state of a node.
    pub fn get_node(&self, node_id: u128) -> Option<&NodeState> {
        self.nodes.get(&node_id)
    }

    /// Update an existing node's state (GEI, Z-score, etc.).
    pub fn update_node(
        &mut self,
        node_id: u128,
        gei: Option<f64>,
        z_score: Option<f64>,
        weight: Option<f64>,
    ) -> Result<(), ResonanceFieldError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(ResonanceFieldError::NodeNotFound(node_id))?;

        if let Some(g) = gei {
            if !(0.0..=1.0).contains(&g) {
                return Err(ResonanceFieldError::InvalidGEI(g));
            }
            node.gei = g;
        }
        if let Some(z) = z_score {
            if !(-1.0..=1.0).contains(&z) {
                return Err(ResonanceFieldError::InvalidZScore(z));
            }
            node.z_score = z;
        }
        if let Some(w) = weight {
            if w <= 0.0 {
                return Err(ResonanceFieldError::InvalidWeight(w));
            }
            node.weight = w;
        }
        Ok(())
    }

    // ---- Temporal cohesion integration ----

    /// Update σ(t) from TemporalCohesionEngine variance.
    ///
    /// As temporal cohesion increases (variance decreases), σ(t) contracts,
    /// making the field more localized around each node.
    pub fn update_temporal_cohesion(&mut self, timestamp_variance: f64) {
        // Map variance [0, ∞) → sigma [0.1, 2.0]
        // Low variance (high cohesion) → narrow sigma
        // High variance (low cohesion) → wide sigma
        self.sigma_t = 0.1 + 1.9 * (timestamp_variance * 5.0).tanh().max(0.0);
    }

    /// Get the current σ(t) value.
    pub fn sigma_t(&self) -> f64 {
        self.sigma_t
    }

    // ---- Field computation ----

    /// Compute the ethical resonance field value at position `x`.
    ///
    /// R(x,t) = Σ w_i · GEI_i · exp(-d²/2σ(t)²) · tanh(k·Z_i)
    pub fn compute_at(&self, x: f64) -> Result<f64, ResonanceFieldError> {
        if self.nodes.is_empty() {
            return Err(ResonanceFieldError::FieldEmpty);
        }

        let sigma = self.sigma_t;
        let sigma_sq = sigma * sigma;
        let k = self.config.k_factor;

        let mut sum = 0.0;
        for (_, node) in self.nodes.iter() {
            let d = x - node.position;
            let distance_factor = (-d * d / (2.0 * sigma_sq)).exp();
            let ethical_focus = (k * node.z_score).tanh();
            sum += node.weight * node.gei * distance_factor * ethical_focus;
        }
        Ok(sum)
    }

    /// Compute the global field integral (sum over all node positions).
    ///
    /// This gives the total ethical resonance of the network.
    pub fn compute_global(&self) -> Result<f64, ResonanceFieldError> {
        if self.nodes.is_empty() {
            return Err(ResonanceFieldError::FieldEmpty);
        }

        let mut total = 0.0;
        for (_, node) in self.nodes.iter() {
            let val = self.compute_at(node.position)?;
            total += val;
        }
        Ok(total)
    }

    /// Compute the field gradient at position `x` using finite differences.
    pub fn compute_gradient_at(&self, x: f64, epsilon: f64) -> Result<f64, ResonanceFieldError> {
        let f_plus = self.compute_at(x + epsilon)?;
        let f_minus = self.compute_at(x - epsilon)?;
        Ok((f_plus - f_minus) / (2.0 * epsilon))
    }

    // ---- Metadata ----

    /// Number of nodes in the field.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if a node exists.
    pub fn contains(&self, node_id: u128) -> bool {
        self.nodes.contains_key(&node_id)
    }

    /// Iterate over all nodes.
    pub fn iter(&self) -> impl Iterator<Item = (&u128, &NodeState)> {
        self.nodes.iter()
    }

    /// Reset the field to empty state.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.sigma_t = self.config.default_sigma;
    }

    // ---- Internal helpers ----

    /// Prune oldest nodes when exceeding max capacity (keep half).
    fn prune_if_needed(&mut self) {
        if self.nodes.len() <= self.config.max_nodes {
            return;
        }
        // Simple pruning: remove nodes with lowest GEI first (least ethical contribution).
        let mut nodes: Vec<_> = self.nodes.drain().collect();
        nodes.sort_by(|a, b| {
            a.1.gei
                .partial_cmp(&b.1.gei)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let keep = self.config.max_nodes / 2;
        if nodes.len() > keep {
            self.nodes = nodes.split_off(keep).into_iter().collect();
        } else {
            self.nodes = nodes.into_iter().collect();
        }
    }
}

impl Default for EthicalResonanceField {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u128, pos: f64) -> NodeState {
        NodeState::new(id, 1.0, 0.8, 0.5, pos).unwrap()
    }

    #[test]
    fn test_field_creation() {
        let field = EthicalResonanceField::new();
        assert_eq!(field.node_count(), 0);
        assert_eq!(field.sigma_t(), DEFAULT_SIGMA);
    }

    #[test]
    fn test_field_custom_config() {
        let config = FieldConfig {
            k_factor: 3.0,
            default_sigma: 0.5,
            max_nodes: 100,
        };
        let field = EthicalResonanceField::with_config(config);
        assert_eq!(field.sigma_t(), 0.5);
    }

    #[test]
    fn test_add_node() {
        let mut field = EthicalResonanceField::new();
        assert!(field.add_node(make_node(1, 0.0)).is_ok());
        assert_eq!(field.node_count(), 1);
        assert!(field.contains(1));
    }

    #[test]
    fn test_duplicate_node() {
        let mut field = EthicalResonanceField::new();
        assert!(field.add_node(make_node(1, 0.0)).is_ok());
        assert!(field.add_node(make_node(1, 0.0)).is_err());
    }

    #[test]
    fn test_remove_node() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        let removed = field.remove_node(1).unwrap();
        assert_eq!(removed.node_id, 1);
        assert_eq!(field.node_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut field = EthicalResonanceField::new();
        assert!(field.remove_node(999).is_err());
    }

    #[test]
    fn test_update_node_gei() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        field.update_node(1, Some(0.95), None, None).unwrap();
        assert_eq!(field.get_node(1).unwrap().gei, 0.95);
    }

    #[test]
    fn test_update_node_invalid_gei() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        assert!(field.update_node(1, Some(1.5), None, None).is_err());
    }

    #[test]
    fn test_compute_at_single_node() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        // At the node position, distance = 0, so exp(0) = 1.
        let val = field.compute_at(0.0).unwrap();
        // R = 1.0 * 0.8 * 1.0 * tanh(2.0 * 0.5) = 0.8 * tanh(1.0)
        let expected = 0.8 * 1.0_f64.tanh();
        assert!((val - expected).abs() < 0.001);
    }

    #[test]
    fn test_compute_at_distance_decay() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        let val_near = field.compute_at(0.1).unwrap();
        let val_far = field.compute_at(5.0).unwrap();
        assert!(val_near > val_far, "Distance should decay resonance");
    }

    #[test]
    fn test_compute_global() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        field.add_node(make_node(2, 1.0)).unwrap();
        let global = field.compute_global().unwrap();
        assert!(
            global > 0.0,
            "Global resonance should be positive for ethical nodes"
        );
    }

    #[test]
    fn test_compute_empty_field() {
        let field = EthicalResonanceField::new();
        assert!(field.compute_at(0.0).is_err());
    }

    #[test]
    fn test_temporal_cohesion_contraction() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        let sigma_before = field.sigma_t();
        // Low variance → high cohesion → narrow sigma
        field.update_temporal_cohesion(0.001);
        assert!(
            field.sigma_t() < sigma_before,
            "Sigma should contract with high cohesion"
        );
    }

    #[test]
    fn test_temporal_cohesion_expansion() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        let sigma_before = field.sigma_t();
        // High variance → low cohesion → wide sigma
        field.update_temporal_cohesion(100.0);
        assert!(
            field.sigma_t() > sigma_before,
            "Sigma should expand with low cohesion"
        );
    }

    #[test]
    fn test_gradient_computation() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        let grad = field.compute_gradient_at(0.5, 0.001).unwrap();
        // Gradient should be negative when moving away from a positive node at 0.0
        assert!(grad < 0.0, "Gradient should point toward the node");
    }

    #[test]
    fn test_node_state_invalid_gei() {
        assert!(NodeState::new(1, 1.0, 1.5, 0.5, 0.0).is_err());
    }

    #[test]
    fn test_node_state_invalid_z() {
        assert!(NodeState::new(1, 1.0, 0.5, 1.5, 0.0).is_err());
    }

    #[test]
    fn test_node_state_invalid_weight() {
        assert!(NodeState::new(1, 0.0, 0.5, 0.5, 0.0).is_err());
    }

    #[test]
    fn test_reset() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        field.reset();
        assert_eq!(field.node_count(), 0);
        assert_eq!(field.sigma_t(), DEFAULT_SIGMA);
    }

    #[test]
    fn test_field_default() {
        let field = EthicalResonanceField::default();
        assert_eq!(field.node_count(), 0);
    }

    #[test]
    fn test_negative_z_score_produces_negative_resonance() {
        let mut field = EthicalResonanceField::new();
        let node = NodeState::new(1, 1.0, 0.8, -0.7, 0.0).unwrap();
        field.add_node(node).unwrap();
        let val = field.compute_at(0.0).unwrap();
        assert!(val < 0.0, "Negative Z should produce negative resonance");
    }

    #[test]
    fn test_multiple_nodes_summation() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, -1.0)).unwrap();
        field.add_node(make_node(2, 1.0)).unwrap();
        let center = field.compute_at(0.0).unwrap();
        let left = field.compute_at(-1.0).unwrap();
        let right = field.compute_at(1.0).unwrap();
        // Center should be between left and right due to symmetry
        assert!(
            center < left || center < right,
            "Center should be influenced by both nodes"
        );
    }

    #[test]
    fn test_error_display() {
        let err = ResonanceFieldError::FieldEmpty;
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_iter() {
        let mut field = EthicalResonanceField::new();
        field.add_node(make_node(1, 0.0)).unwrap();
        field.add_node(make_node(2, 1.0)).unwrap();
        let count = field.iter().count();
        assert_eq!(count, 2);
    }
}
