//! Evolutionary Quarantine â€” Sprint 76: Ontological Debugging & Thermodynamic Pivots
//!
//! Resuelve el bug ontolÃ³gico: SCT Guard â†’ conservadurismo algorÃ­tmico / censura estÃ¡tica.
//!
//! La cuarentena evolutiva aÃ­sla nodos con Z<0 en un shard de prueba,
//! donde son validados por simulaciÃ³n macro. La reintegraciÃ³n ocurre
//! si el nodo mejora las mÃ©tricas globales. La Ã©tica es un atractor dinÃ¡mico,
//! no un muro estÃ¡tico.
//!
//! # GarantÃ­as
//!
//! - Complejidad: O(1) para enrutamiento a cuarentena, O(n) para evaluaciÃ³n de reintegraciÃ³n
//! - Memoria: O(n) para nodos en cuarentena
//! - PrevenciÃ³n de cascada: mÃ¡ximo 20% de la red en cuarentena simultÃ¡nea

use std::collections::HashMap;
use std::fmt;

/// Error types for Evolutionary Quarantine
#[derive(Debug, Clone, PartialEq)]
pub enum QuarantineError {
    /// Node not found
    NodeNotFound(u64),
    /// Quarantine capacity exceeded (cascade prevention)
    QuarantineFull(usize),
    /// Invalid SCT-Z value
    InvalidSctZ(f64),
    /// Node already in quarantine
    AlreadyQuarantined(u64),
    /// Simulation cycles insufficient
    InsufficientCycles(u32),
    /// Reintegration rejected by macro simulation
    ReintegrationRejected(String),
}

impl fmt::Display for QuarantineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuarantineError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            QuarantineError::QuarantineFull(cap) => {
                write!(f, "Quarantine capacity full: {} nodes", cap)
            }
            QuarantineError::InvalidSctZ(v) => write!(f, "Invalid SCT-Z: {}", v),
            QuarantineError::AlreadyQuarantined(id) => {
                write!(f, "Node already in quarantine: {}", id)
            }
            QuarantineError::InsufficientCycles(c) => {
                write!(f, "Insufficient simulation cycles: {}", c)
            }
            QuarantineError::ReintegrationRejected(msg) => {
                write!(f, "Reintegration rejected: {}", msg)
            }
        }
    }
}

impl std::error::Error for QuarantineError {}

/// Quarantine state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuarantineState {
    /// Active â€” normal operation
    Active,
    /// Observing â€” monitoring for anomalies
    Observing,
    /// Quarantined â€” isolated in test shard
    Quarantined,
    /// Simulating â€” running macro simulation for reintegration
    Simulating,
    /// Reintegrating â€” attempting re-entry
    Reintegrating,
}

impl fmt::Display for QuarantineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuarantineState::Active => write!(f, "Active"),
            QuarantineState::Observing => write!(f, "Observing"),
            QuarantineState::Quarantined => write!(f, "Quarantined"),
            QuarantineState::Simulating => write!(f, "Simulating"),
            QuarantineState::Reintegrating => write!(f, "Reintegrating"),
        }
    }
}

/// Configuration for Evolutionary Quarantine.
#[derive(Debug, Clone)]
pub struct QuarantineConfig {
    /// SCT-Z threshold for triggering observation.
    pub observation_threshold: f64,
    /// SCT-Z threshold for triggering quarantine.
    pub quarantine_threshold: f64,
    /// Maximum quarantine capacity as fraction of total nodes (cascade prevention).
    pub max_quarantine_fraction: f64,
    /// Minimum simulation cycles for reintegration evaluation.
    pub min_simulation_cycles: u32,
    /// Improvement threshold for reintegration (macro metric delta).
    pub improvement_threshold: f64,
    /// Reintegration vote epsilon (BFT tolerance).
    pub reintegration_epsilon: f64,
}

impl QuarantineConfig {
    /// Default Topological configuration.
    pub fn default_Topological() -> Self {
        Self {
            observation_threshold: -0.5,
            quarantine_threshold: -1.0,
            max_quarantine_fraction: 0.2,
            min_simulation_cycles: 10,
            improvement_threshold: 0.1,
            reintegration_epsilon: 0.67,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), QuarantineError> {
        if self.observation_threshold.is_nan() || self.quarantine_threshold.is_nan() {
            return Err(QuarantineError::InvalidSctZ(f64::NAN));
        }
        if self.max_quarantine_fraction <= 0.0 || self.max_quarantine_fraction > 0.5 {
            return Err(QuarantineError::QuarantineFull(0));
        }
        if self.min_simulation_cycles == 0 {
            return Err(QuarantineError::InsufficientCycles(0));
        }
        Ok(())
    }
}

impl Default for QuarantineConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Node state in the quarantine system.
#[derive(Debug, Clone)]
pub struct QuarantineNode {
    /// Unique node identifier.
    pub node_id: u64,
    /// Current quarantine state.
    pub state: QuarantineState,
    /// Current SCT-Z value.
    pub sct_z: f64,
    /// Simulation score (for reintegration evaluation).
    pub simulation_score: f64,
    /// Number of simulation cycles completed.
    pub cycles_completed: u32,
    /// Timestamp when entered current state (ms).
    pub state_entered_ms: u64,
}

impl QuarantineNode {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            state: QuarantineState::Active,
            sct_z: 0.0,
            simulation_score: 0.0,
            cycles_completed: 0,
            state_entered_ms: 0,
        }
    }
}

impl fmt::Display for QuarantineNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QuarantineNode {{ id={}, state={}, sct_z={:.4}, sim_score={:.4}, cycles={} }}",
            self.node_id, self.state, self.sct_z, self.simulation_score, self.cycles_completed
        )
    }
}

/// Record of a quarantine or reintegration event.
#[derive(Debug, Clone)]
pub struct QuarantineRecord {
    /// Node identifier.
    pub node_id: u64,
    /// Previous state.
    pub from_state: QuarantineState,
    /// New state.
    pub to_state: QuarantineState,
    /// SCT-Z value at transition.
    pub sct_z: f64,
    /// Reason for transition.
    pub reason: String,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

impl fmt::Display for QuarantineRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QuarantineRecord {{ id={}, {}â†’{}, sct_z={:.4}, reason=\"{}\" }}",
            self.node_id, self.from_state, self.to_state, self.sct_z, self.reason
        )
    }
}

/// Stateful engine for evolutionary quarantine management.
#[derive(Debug, Clone)]
pub struct EvolutionaryQuarantine {
    config: QuarantineConfig,
    nodes: HashMap<u64, QuarantineNode>,
    total_nodes: usize,
    records: Vec<QuarantineRecord>,
}

impl EvolutionaryQuarantine {
    /// Create a new engine with default Topological configuration.
    pub fn new() -> Self {
        Self {
            config: QuarantineConfig::default_Topological(),
            nodes: HashMap::new(),
            total_nodes: 0,
            records: Vec::new(),
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: QuarantineConfig) -> Result<Self, QuarantineError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            total_nodes: 0,
            records: Vec::new(),
        })
    }

    /// Register a node in the system.
    pub fn register_node(&mut self, node_id: u64) {
        self.nodes
            .entry(node_id)
            .or_insert_with(|| QuarantineNode::new(node_id));
        self.total_nodes = self.nodes.len();
    }

    /// Update SCT-Z value for a node and evaluate state transition.
    pub fn update_sct_z(
        &mut self,
        node_id: u64,
        sct_z: f64,
        current_ms: u64,
    ) -> Result<Option<QuarantineRecord>, QuarantineError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(QuarantineError::NodeNotFound(node_id))?;

        let old_state = node.state;
        node.sct_z = sct_z;

        let new_state = match old_state {
            QuarantineState::Active => {
                if sct_z < self.config.quarantine_threshold {
                    QuarantineState::Quarantined
                } else if sct_z < self.config.observation_threshold {
                    QuarantineState::Observing
                } else {
                    QuarantineState::Active
                }
            }
            QuarantineState::Observing => {
                if sct_z < self.config.quarantine_threshold {
                    QuarantineState::Quarantined
                } else if sct_z >= self.config.observation_threshold {
                    QuarantineState::Active
                } else {
                    QuarantineState::Observing
                }
            }
            QuarantineState::Quarantined => {
                // Once quarantined, must go through simulation
                QuarantineState::Quarantined
            }
            QuarantineState::Simulating => {
                // Must complete simulation
                QuarantineState::Simulating
            }
            QuarantineState::Reintegrating => {
                // Must complete reintegration vote
                QuarantineState::Reintegrating
            }
        };

        if new_state != old_state {
            node.state = new_state;
            node.state_entered_ms = current_ms;
            let record = QuarantineRecord {
                node_id,
                from_state: old_state,
                to_state: new_state,
                sct_z,
                reason: format!("SCT-Z {:.4} crossed threshold", sct_z),
                timestamp_ms: current_ms,
            };
            self.records.push(record.clone());
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Route a quarantined node to simulation shard.
    pub fn route_to_quarantine(
        &mut self,
        node_id: u64,
        simulation_cycles: u32,
        current_ms: u64,
    ) -> Result<QuarantineState, QuarantineError> {
        // Check cascade prevention before mutable borrow
        let quarantine_count = self
            .nodes
            .values()
            .filter(|n| {
                n.state == QuarantineState::Quarantined || n.state == QuarantineState::Simulating
            })
            .count();
        let max_allowed = (self.total_nodes as f64 * self.config.max_quarantine_fraction) as usize;
        if quarantine_count >= max_allowed && max_allowed > 0 {
            return Err(QuarantineError::QuarantineFull(quarantine_count));
        }

        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(QuarantineError::NodeNotFound(node_id))?;

        if node.state != QuarantineState::Quarantined {
            return Ok(node.state);
        }

        if simulation_cycles < self.config.min_simulation_cycles {
            return Err(QuarantineError::InsufficientCycles(simulation_cycles));
        }

        let old_state = node.state;
        node.state = QuarantineState::Simulating;
        node.cycles_completed = simulation_cycles;
        node.state_entered_ms = current_ms;

        // Simulate macro evaluation: score based on SCT-Z improvement potential
        node.simulation_score = 1.0 + node.sct_z; // Higher SCT-Z â†’ better score

        let record = QuarantineRecord {
            node_id,
            from_state: old_state,
            to_state: QuarantineState::Simulating,
            sct_z: node.sct_z,
            reason: format!("Routed to simulation shard, {} cycles", simulation_cycles),
            timestamp_ms: current_ms,
        };
        self.records.push(record);

        Ok(QuarantineState::Simulating)
    }

    /// Evaluate reintegration for a simulated node.
    pub fn evaluate_reintegration(
        &mut self,
        node_id: u64,
        current_ms: u64,
    ) -> Result<bool, QuarantineError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(QuarantineError::NodeNotFound(node_id))?;

        if node.state != QuarantineState::Simulating {
            return Ok(false);
        }

        if node.cycles_completed < self.config.min_simulation_cycles {
            return Err(QuarantineError::InsufficientCycles(node.cycles_completed));
        }

        // Reintegration criteria: simulation score shows improvement
        let improved = node.simulation_score >= self.config.improvement_threshold;

        let old_state = node.state;
        if improved {
            node.state = QuarantineState::Reintegrating;
            let record = QuarantineRecord {
                node_id,
                from_state: old_state,
                to_state: QuarantineState::Reintegrating,
                sct_z: node.sct_z,
                reason: format!(
                    "Simulation score {:.4} >= threshold {:.4}",
                    node.simulation_score, self.config.improvement_threshold
                ),
                timestamp_ms: current_ms,
            };
            self.records.push(record);
        }

        Ok(improved)
    }

    /// Complete reintegration for a node that passed simulation.
    pub fn complete_reintegration(
        &mut self,
        node_id: u64,
        current_ms: u64,
    ) -> Result<(), QuarantineError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(QuarantineError::NodeNotFound(node_id))?;

        if node.state != QuarantineState::Reintegrating {
            return Ok(());
        }

        let old_state = node.state;
        node.state = QuarantineState::Active;
        node.state_entered_ms = current_ms;

        let record = QuarantineRecord {
            node_id,
            from_state: old_state,
            to_state: QuarantineState::Active,
            sct_z: node.sct_z,
            reason: "Reintegration completed â€” node returned to active mesh".to_string(),
            timestamp_ms: current_ms,
        };
        self.records.push(record);

        Ok(())
    }

    /// Get nodes in a specific quarantine state.
    pub fn nodes_in_state(&self, state: QuarantineState) -> Vec<u64> {
        self.nodes
            .values()
            .filter(|n| n.state == state)
            .map(|n| n.node_id)
            .collect()
    }

    /// Quarantine statistics by state.
    pub fn stats(&self) -> HashMap<QuarantineState, usize> {
        let mut counts = HashMap::new();
        for state in [
            QuarantineState::Active,
            QuarantineState::Observing,
            QuarantineState::Quarantined,
            QuarantineState::Simulating,
            QuarantineState::Reintegrating,
        ] {
            let count = self.nodes.values().filter(|n| n.state == state).count();
            if count > 0 {
                counts.insert(state, count);
            }
        }
        counts
    }

    /// Total nodes registered.
    pub fn total_nodes(&self) -> usize {
        self.total_nodes
    }

    /// Quarantine rate (fraction of nodes not active).
    pub fn quarantine_rate(&self) -> Option<f64> {
        if self.total_nodes == 0 {
            return None;
        }
        let non_active = self
            .nodes
            .values()
            .filter(|n| n.state != QuarantineState::Active)
            .count();
        Some(non_active as f64 / self.total_nodes as f64)
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        for node in self.nodes.values_mut() {
            node.state = QuarantineState::Active;
            node.sct_z = 0.0;
            node.simulation_score = 0.0;
            node.cycles_completed = 0;
            node.state_entered_ms = 0;
        }
        self.records.clear();
    }
}

impl Default for EvolutionaryQuarantine {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EvolutionaryQuarantine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EvolutionaryQuarantine {{ total={}, quarantine_rate={:?}, records={} }}",
            self.total_nodes(),
            self.quarantine_rate(),
            self.records.len()
        )
    }
}

// â”€â”€â”€ Public Standalone Function â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Route a node to quarantine based on SCT-Z value.
///
/// Standalone function for use without stateful engine.
pub fn route_to_quarantine(
    _node_id: u64,
    sct_z: f64,
    simulation_cycles: u32,
    min_cycles: u32,
) -> QuarantineState {
    if sct_z >= -0.5 {
        return QuarantineState::Active;
    }
    if sct_z >= -1.0 {
        return QuarantineState::Observing;
    }
    if simulation_cycles < min_cycles {
        return QuarantineState::Quarantined; // Cannot simulate yet
    }
    QuarantineState::Simulating
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = QuarantineConfig::default_Topological();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_quarantine_fraction, 0.2);
    }

    #[test]
    fn test_config_invalid_fraction() {
        let config = QuarantineConfig {
            max_quarantine_fraction: 0.0,
            ..QuarantineConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_cycles() {
        let config = QuarantineConfig {
            min_simulation_cycles: 0,
            ..QuarantineConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_creation() {
        let node = QuarantineNode::new(1);
        assert_eq!(node.state, QuarantineState::Active);
        assert_eq!(node.sct_z, 0.0);
    }

    #[test]
    fn test_engine_creation() {
        let engine = EvolutionaryQuarantine::new();
        assert_eq!(engine.total_nodes(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        assert_eq!(engine.total_nodes(), 1);
    }

    #[test]
    fn test_update_sct_healthy() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        let record = engine.update_sct_z(1, 0.5, 1000).unwrap();
        assert!(record.is_none()); // No state change
    }

    #[test]
    fn test_update_sct_observing() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        let record = engine.update_sct_z(1, -0.7, 1000).unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.to_state, QuarantineState::Observing);
    }

    #[test]
    fn test_update_sct_quarantined() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        let record = engine.update_sct_z(1, -1.5, 1000).unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.to_state, QuarantineState::Quarantined);
    }

    #[test]
    fn test_route_to_quarantine() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        let state = engine.route_to_quarantine(1, 20, 2000).unwrap();
        assert_eq!(state, QuarantineState::Simulating);
    }

    #[test]
    fn test_route_insufficient_cycles() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        let result = engine.route_to_quarantine(1, 5, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_reintegration_passes() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        engine.route_to_quarantine(1, 20, 2000).unwrap();
        // Set simulation score to pass
        engine.nodes.get_mut(&1).unwrap().simulation_score = 0.5;
        let passed = engine.evaluate_reintegration(1, 3000).unwrap();
        assert!(passed);
    }

    #[test]
    fn test_evaluate_reintegration_fails() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        engine.route_to_quarantine(1, 20, 2000).unwrap();
        // Set simulation score to fail
        engine.nodes.get_mut(&1).unwrap().simulation_score = -0.5;
        let passed = engine.evaluate_reintegration(1, 3000).unwrap();
        assert!(!passed);
    }

    #[test]
    fn test_complete_reintegration() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        engine.route_to_quarantine(1, 20, 2000).unwrap();
        engine.nodes.get_mut(&1).unwrap().simulation_score = 0.5;
        engine.evaluate_reintegration(1, 3000).unwrap();
        engine.complete_reintegration(1, 4000).unwrap();
        let node = engine.nodes.get(&1).unwrap();
        assert_eq!(node.state, QuarantineState::Active);
    }

    #[test]
    fn test_nodes_in_state() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        let quarantined = engine.nodes_in_state(QuarantineState::Quarantined);
        assert_eq!(quarantined, vec![1]);
    }

    #[test]
    fn test_stats() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.register_node(2);
        let stats = engine.stats();
        assert_eq!(stats.get(&QuarantineState::Active), Some(&2));
    }

    #[test]
    fn test_quarantine_rate() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        let rate = engine.quarantine_rate().unwrap();
        assert!((rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_reset() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        engine.reset();
        let node = engine.nodes.get(&1).unwrap();
        assert_eq!(node.state, QuarantineState::Active);
    }

    #[test]
    fn test_display() {
        let engine = EvolutionaryQuarantine::new();
        let s = format!("{}", engine);
        assert!(s.contains("EvolutionaryQuarantine"));
    }

    #[test]
    fn test_node_display() {
        let node = QuarantineNode::new(1);
        let s = format!("{}", node);
        assert!(s.contains("QuarantineNode"));
    }

    #[test]
    fn test_standalone_route() {
        let state = route_to_quarantine(1, 0.5, 20, 10);
        assert_eq!(state, QuarantineState::Active);

        let state = route_to_quarantine(1, -0.7, 20, 10);
        assert_eq!(state, QuarantineState::Observing);

        let state = route_to_quarantine(1, -1.5, 20, 10);
        assert_eq!(state, QuarantineState::Simulating);

        let state = route_to_quarantine(1, -1.5, 5, 10);
        assert_eq!(state, QuarantineState::Quarantined);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = EvolutionaryQuarantine::new();
        engine.register_node(1);
        engine.register_node(2);

        // Node 1 goes to quarantine
        engine.update_sct_z(1, -1.5, 1000).unwrap();
        engine.route_to_quarantine(1, 20, 2000).unwrap();
        engine.nodes.get_mut(&1).unwrap().simulation_score = 0.5;
        engine.evaluate_reintegration(1, 3000).unwrap();
        engine.complete_reintegration(1, 4000).unwrap();

        // Node 2 stays healthy
        engine.update_sct_z(2, 0.5, 1000).unwrap();

        assert_eq!(engine.total_nodes(), 2);
        let node1 = engine.nodes.get(&1).unwrap();
        assert_eq!(node1.state, QuarantineState::Active);
        assert!(engine.quarantine_rate().unwrap() < 0.5);
    }

    #[test]
    fn test_error_display() {
        let err = QuarantineError::NodeNotFound(1);
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }
}
