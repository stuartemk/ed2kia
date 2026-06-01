//! Graceful Apoptosis — Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! Ventanas de cuarentena, observación acotada, reintegración por consenso ε-tolerante.
//! Prevención de partición vía bounded failure domains.
//!
//! **Pivot Arquitectónico:** Apoptosis con riesgo de partición en cascada mitigado
//! mediante dominios de fallo acotados y reintegración controlada.

use std::collections::HashMap;
use std::fmt;

/// Error types for Graceful Apoptosis
#[derive(Debug, Clone, PartialEq)]
pub enum ApoptosisError {
    /// Invalid SCT-Z value
    InvalidSctZ(f64),
    /// Node not found
    NodeNotFound(u64),
    /// Already in apoptosis
    AlreadyApoptosing(u64),
    /// Quarantine capacity exceeded
    QuarantineFull(usize),
    /// Reintegration rejected
    ReintegrationRejected(String),
    /// Cascade prevention triggered
    CascadePrevented,
}

impl fmt::Display for ApoptosisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApoptosisError::InvalidSctZ(v) => write!(f, "Invalid SCT-Z: {}", v),
            ApoptosisError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            ApoptosisError::AlreadyApoptosing(id) => write!(f, "Node already apoptosing: {}", id),
            ApoptosisError::QuarantineFull(cap) => write!(f, "Quarantine capacity full: {}", cap),
            ApoptosisError::ReintegrationRejected(msg) => {
                write!(f, "Reintegration rejected: {}", msg)
            }
            ApoptosisError::CascadePrevented => {
                write!(f, "Cascade prevention triggered — apoptosis blocked")
            }
        }
    }
}

/// Apoptosis state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApoptosisState {
    /// Healthy — normal operation
    Healthy,
    /// Observing — monitoring for anomalies
    Observing,
    /// Quarantined — isolated from mesh
    Quarantined,
    /// Apoptosing — preparing for removal
    Apoptosing,
    /// Reintegrating — attempting re-entry
    Reintegrating,
    /// Removed — permanently excised
    Removed,
}

impl fmt::Display for ApoptosisState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApoptosisState::Healthy => write!(f, "Healthy"),
            ApoptosisState::Observing => write!(f, "Observing"),
            ApoptosisState::Quarantined => write!(f, "Quarantined"),
            ApoptosisState::Apoptosing => write!(f, "Apoptosing"),
            ApoptosisState::Reintegrating => write!(f, "Reintegrating"),
            ApoptosisState::Removed => write!(f, "Removed"),
        }
    }
}

/// Configuration for Graceful Apoptosis
#[derive(Debug, Clone)]
pub struct ApoptosisConfig {
    /// SCT-Z threshold for triggering observation
    pub observation_threshold: f64,
    /// SCT-Z threshold for triggering quarantine
    pub quarantine_threshold: f64,
    /// SCT-Z threshold for triggering apoptosis
    pub apoptosis_threshold: f64,
    /// Observation window in milliseconds
    pub observation_window_ms: u64,
    /// Quarantine window in milliseconds
    pub quarantine_window_ms: u64,
    /// Maximum quarantine capacity (cascade prevention)
    pub max_quarantine_size: usize,
    /// BFT epsilon for reintegration consensus
    pub reintegration_epsilon: f64,
    /// Minimum neighbors for reintegration vote
    pub min_reintegration_neighbors: usize,
    /// Maximum concurrent apoptosis (cascade limit)
    pub max_concurrent_apoptosis: usize,
}

impl ApoptosisConfig {
    pub fn default_stuartian() -> Self {
        Self {
            observation_threshold: 0.4,
            quarantine_threshold: 0.25,
            apoptosis_threshold: 0.1,
            observation_window_ms: 30_000,
            quarantine_window_ms: 120_000,
            max_quarantine_size: 100,
            reintegration_epsilon: 0.1,
            min_reintegration_neighbors: 3,
            max_concurrent_apoptosis: 5,
        }
    }

    pub fn validate(&self) -> Result<(), ApoptosisError> {
        if self.observation_threshold < 0.0 || self.observation_threshold > 1.0 {
            return Err(ApoptosisError::InvalidSctZ(self.observation_threshold));
        }
        if self.quarantine_threshold >= self.observation_threshold {
            return Err(ApoptosisError::InvalidSctZ(self.quarantine_threshold));
        }
        if self.apoptosis_threshold >= self.quarantine_threshold {
            return Err(ApoptosisError::InvalidSctZ(self.apoptosis_threshold));
        }
        if self.max_quarantine_size == 0 {
            return Err(ApoptosisError::QuarantineFull(0));
        }
        Ok(())
    }
}

impl Default for ApoptosisConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Node state in apoptosis system
#[derive(Debug, Clone)]
pub struct ApoptosisNodeState {
    pub node_id: u64,
    pub state: ApoptosisState,
    pub sct_z: f64,
    pub state_entered_ms: u64,
    pub neighbor_count: usize,
    pub reintegration_votes: usize,
}

impl ApoptosisNodeState {
    pub fn new(node_id: u64, neighbor_count: usize) -> Self {
        Self {
            node_id,
            state: ApoptosisState::Healthy,
            sct_z: 1.0,
            state_entered_ms: 0,
            neighbor_count,
            reintegration_votes: 0,
        }
    }
}

impl fmt::Display for ApoptosisNodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ApoptosisNodeState {{ id: {}, state: {}, sct_z: {:.4}, neighbors: {} }}",
            self.node_id, self.state, self.sct_z, self.neighbor_count
        )
    }
}

/// Record of apoptosis event
#[derive(Debug, Clone)]
pub struct ApoptosisRecord {
    pub node_id: u64,
    pub from_state: ApoptosisState,
    pub to_state: ApoptosisState,
    pub sct_z: f64,
    pub timestamp_ms: u64,
    pub reason: String,
}

impl fmt::Display for ApoptosisRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ApoptosisRecord {{ node: {}, {} → {}, sct_z: {:.4}, reason: {}, ts: {} }}",
            self.node_id,
            self.from_state,
            self.to_state,
            self.sct_z,
            self.reason,
            self.timestamp_ms
        )
    }
}

/// Graceful Apoptosis Engine
pub struct GracefulApoptosis {
    config: ApoptosisConfig,
    nodes: HashMap<u64, ApoptosisNodeState>,
    history: Vec<ApoptosisRecord>,
}

impl GracefulApoptosis {
    pub fn new() -> Self {
        Self {
            config: ApoptosisConfig::default_stuartian(),
            nodes: HashMap::new(),
            history: Vec::new(),
        }
    }

    pub fn with_config(config: ApoptosisConfig) -> Result<Self, ApoptosisError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            history: Vec::new(),
        })
    }

    /// Register a node
    pub fn register_node(&mut self, node_id: u64, neighbor_count: usize) {
        self.nodes
            .insert(node_id, ApoptosisNodeState::new(node_id, neighbor_count));
    }

    /// Update SCT-Z for a node
    pub fn update_sct_z(
        &mut self,
        node_id: u64,
        sct_z: f64,
        _current_ms: u64,
    ) -> Result<(), ApoptosisError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(ApoptosisError::NodeNotFound(node_id))?;

        if sct_z < 0.0 || sct_z > 1.0 {
            return Err(ApoptosisError::InvalidSctZ(sct_z));
        }

        node.sct_z = sct_z;
        Ok(())
    }

    /// Evaluate apoptosis trigger for a node
    pub fn evaluate(
        &mut self,
        node_id: u64,
        current_ms: u64,
    ) -> Result<ApoptosisState, ApoptosisError> {
        // Extract node values to avoid simultaneous borrows
        let (old_state, sct_z, state_entered_ms, neighbor_count, reintegration_votes) = {
            let node = self
                .nodes
                .get(&node_id)
                .ok_or(ApoptosisError::NodeNotFound(node_id))?;
            (
                node.state,
                node.sct_z,
                node.state_entered_ms,
                node.neighbor_count,
                node.reintegration_votes,
            )
        };

        // Pre-compute cascade count before evaluation (needed for Quarantined branch)
        let apoptosing_count = self
            .nodes
            .values()
            .filter(|n| {
                n.state == ApoptosisState::Apoptosing || n.state == ApoptosisState::Quarantined
            })
            .count();

        let (new_state, reason) = match old_state {
            ApoptosisState::Healthy => (
                Self::evaluate_healthy_values(&self.config, sct_z),
                "threshold_evaluation".to_string(),
            ),
            ApoptosisState::Observing => (
                Self::evaluate_observing_values(&self.config, sct_z, state_entered_ms, current_ms),
                "observation_evaluation".to_string(),
            ),
            ApoptosisState::Quarantined => (
                Self::evaluate_quarantined_values(
                    &self.config,
                    sct_z,
                    state_entered_ms,
                    current_ms,
                    apoptosing_count,
                ),
                "quarantine_evaluation".to_string(),
            ),
            ApoptosisState::Apoptosing => (
                ApoptosisState::Removed,
                "apoptosis_complete".to_string(),
            ),
            ApoptosisState::Reintegrating => (
                Self::evaluate_reintegrating_values(
                    &self.config,
                    sct_z,
                    neighbor_count,
                    reintegration_votes,
                ),
                "reintegration_evaluation".to_string(),
            ),
            ApoptosisState::Removed => {
                // Terminal state — no transition
                return Ok(old_state);
            }
        };

        self.transition(node_id, old_state, new_state, current_ms, reason);
        Ok(new_state)
    }

    fn evaluate_healthy_values(config: &ApoptosisConfig, sct_z: f64) -> ApoptosisState {
        if sct_z < config.apoptosis_threshold {
            return ApoptosisState::Apoptosing;
        }
        if sct_z < config.quarantine_threshold {
            return ApoptosisState::Quarantined;
        }
        if sct_z < config.observation_threshold {
            return ApoptosisState::Observing;
        }
        ApoptosisState::Healthy
    }

    fn evaluate_observing_values(
        config: &ApoptosisConfig,
        sct_z: f64,
        state_entered_ms: u64,
        current_ms: u64,
    ) -> ApoptosisState {
        let elapsed = current_ms.saturating_sub(state_entered_ms);

        if sct_z >= config.observation_threshold {
            return ApoptosisState::Healthy;
        }
        if sct_z < config.apoptosis_threshold {
            return ApoptosisState::Apoptosing;
        }
        if sct_z < config.quarantine_threshold {
            return ApoptosisState::Quarantined;
        }
        if elapsed > config.observation_window_ms {
            return ApoptosisState::Quarantined;
        }
        ApoptosisState::Observing
    }

    fn evaluate_quarantined_values(
        config: &ApoptosisConfig,
        sct_z: f64,
        state_entered_ms: u64,
        current_ms: u64,
        apoptosing_count: usize,
    ) -> ApoptosisState {
        let elapsed = current_ms.saturating_sub(state_entered_ms);

        if sct_z >= config.observation_threshold {
            return ApoptosisState::Reintegrating;
        }
        if sct_z < config.apoptosis_threshold {
            // Cascade prevention
            if apoptosing_count >= config.max_concurrent_apoptosis {
                return ApoptosisState::Quarantined; // Block cascade
            }
            return ApoptosisState::Apoptosing;
        }
        if elapsed > config.quarantine_window_ms {
            return ApoptosisState::Apoptosing;
        }
        ApoptosisState::Quarantined
    }

    fn evaluate_reintegrating_values(
        config: &ApoptosisConfig,
        sct_z: f64,
        neighbor_count: usize,
        reintegration_votes: usize,
    ) -> ApoptosisState {
        // ε-tolerant consensus for reintegration
        let required_votes = if neighbor_count >= config.min_reintegration_neighbors {
            let threshold = neighbor_count as f64
                * (2.0 / 3.0 * (1.0 - config.reintegration_epsilon));
            threshold as usize
        } else {
            neighbor_count
        };

        if reintegration_votes >= required_votes && sct_z >= config.observation_threshold {
            return ApoptosisState::Healthy;
        }
        if sct_z < config.quarantine_threshold {
            return ApoptosisState::Quarantined;
        }
        ApoptosisState::Reintegrating
    }

    fn transition(
        &mut self,
        node_id: u64,
        from: ApoptosisState,
        to: ApoptosisState,
        timestamp_ms: u64,
        reason: String,
    ) {
        if from == to {
            return;
        }
        let node = self.nodes.get_mut(&node_id).unwrap();
        node.state = to;
        node.state_entered_ms = timestamp_ms;

        let sct_z = node.sct_z;
        self.history.push(ApoptosisRecord {
            node_id,
            from_state: from,
            to_state: to,
            sct_z,
            timestamp_ms,
            reason,
        });
    }

    /// Cast reintegration vote for a node
    pub fn vote_reintegration(&mut self, node_id: u64) -> Result<(), ApoptosisError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(ApoptosisError::NodeNotFound(node_id))?;
        if node.state != ApoptosisState::Reintegrating {
            return Err(ApoptosisError::ReintegrationRejected(format!(
                "Node is in state {}",
                node.state
            )));
        }
        node.reintegration_votes += 1;
        Ok(())
    }

    /// Check cascade prevention
    pub fn is_cascade_risk(&self) -> bool {
        let quarantined = self
            .nodes
            .values()
            .filter(|n| {
                n.state == ApoptosisState::Quarantined || n.state == ApoptosisState::Apoptosing
            })
            .count();
        quarantined >= self.config.max_concurrent_apoptosis
    }

    /// Get nodes in a specific state
    pub fn nodes_in_state(&self, state: ApoptosisState) -> Vec<u64> {
        self.nodes
            .values()
            .filter(|n| n.state == state)
            .map(|n| n.node_id)
            .collect()
    }

    /// Get stats
    pub fn stats(&self) -> HashMap<ApoptosisState, usize> {
        let mut counts = HashMap::new();
        *counts.entry(ApoptosisState::Healthy).or_insert(0) += 0;
        *counts.entry(ApoptosisState::Observing).or_insert(0) += 0;
        *counts.entry(ApoptosisState::Quarantined).or_insert(0) += 0;
        *counts.entry(ApoptosisState::Apoptosing).or_insert(0) += 0;
        *counts.entry(ApoptosisState::Reintegrating).or_insert(0) += 0;
        *counts.entry(ApoptosisState::Removed).or_insert(0) += 0;

        for node in self.nodes.values() {
            *counts.entry(node.state).or_insert(0) += 1;
        }
        counts
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.history.clear();
    }
}

impl Default for GracefulApoptosis {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GracefulApoptosis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _stats = self.stats();
        write!(
            f,
            "GracefulApoptosis {{ nodes: {}, history: {}, cascade_risk: {} }}",
            self.nodes.len(),
            self.history.len(),
            self.is_cascade_risk()
        )
    }
}

/// Public function: Evaluate apoptosis trigger
pub fn evaluate_apoptosis_trigger(
    sct_z: f64,
    _observation_window_ms: u64,
    neighbor_consensus: f64,
) -> ApoptosisState {
    let config = ApoptosisConfig::default_stuartian();

    if sct_z < config.apoptosis_threshold {
        // Check neighbor consensus for cascade prevention
        if neighbor_consensus < 0.5 {
            return ApoptosisState::Quarantined; // Block cascade
        }
        return ApoptosisState::Apoptosing;
    }
    if sct_z < config.quarantine_threshold {
        return ApoptosisState::Quarantined;
    }
    if sct_z < config.observation_threshold {
        return ApoptosisState::Observing;
    }
    ApoptosisState::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ApoptosisConfig::default();
        assert!(config.observation_threshold > config.quarantine_threshold);
        assert!(config.quarantine_threshold > config.apoptosis_threshold);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = ApoptosisConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = ApoptosisConfig {
            observation_threshold: 0.2,
            quarantine_threshold: 0.3,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_engine_creation() {
        let engine = GracefulApoptosis::new();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        assert_eq!(engine.nodes.len(), 1);
    }

    #[test]
    fn test_update_sct_z() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        assert!(engine.update_sct_z(1, 0.8, 1000).is_ok());
    }

    #[test]
    fn test_evaluate_healthy() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.9, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, ApoptosisState::Healthy);
    }

    #[test]
    fn test_evaluate_observing() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.35, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, ApoptosisState::Observing);
    }

    #[test]
    fn test_evaluate_quarantined() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.2, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, ApoptosisState::Quarantined);
    }

    #[test]
    fn test_evaluate_apoptosing() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.05, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, ApoptosisState::Apoptosing);
    }

    #[test]
    fn test_cascade_prevention() {
        let mut engine = GracefulApoptosis::with_config(ApoptosisConfig {
            max_concurrent_apoptosis: 2,
            ..Default::default()
        })
        .unwrap();

        for i in 0..3 {
            engine.register_node(i, 5);
            engine.update_sct_z(i, 0.05, 1000).unwrap();
            engine.evaluate(i, 1000).unwrap();
        }

        assert!(engine.is_cascade_risk());
    }

    #[test]
    fn test_reintegration_vote() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        // Set to reintegrating manually for test
        engine.nodes.get_mut(&1).unwrap().state = ApoptosisState::Reintegrating;
        engine.nodes.get_mut(&1).unwrap().sct_z = 0.5;
        assert!(engine.vote_reintegration(1).is_ok());
    }

    #[test]
    fn test_nodes_in_state() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        engine.register_node(2, 5);
        let healthy = engine.nodes_in_state(ApoptosisState::Healthy);
        assert_eq!(healthy.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        let stats = engine.stats();
        assert!(stats.contains_key(&ApoptosisState::Healthy));
    }

    #[test]
    fn test_reset() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);
        engine.reset();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = GracefulApoptosis::new();
        let s = format!("{}", engine);
        assert!(s.contains("GracefulApoptosis"));
    }

    #[test]
    fn test_evaluate_apoptosis_trigger_healthy() {
        let state = evaluate_apoptosis_trigger(0.9, 30000, 0.8);
        assert_eq!(state, ApoptosisState::Healthy);
    }

    #[test]
    fn test_evaluate_apoptosis_trigger_quarantine() {
        let state = evaluate_apoptosis_trigger(0.2, 30000, 0.8);
        assert_eq!(state, ApoptosisState::Quarantined);
    }

    #[test]
    fn test_evaluate_apoptosis_trigger_cascade_block() {
        let state = evaluate_apoptosis_trigger(0.05, 30000, 0.3);
        assert_eq!(state, ApoptosisState::Quarantined);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = GracefulApoptosis::new();
        engine.register_node(1, 5);

        // Start healthy
        engine.update_sct_z(1, 0.9, 1000).unwrap();
        assert_eq!(engine.evaluate(1, 1000).unwrap(), ApoptosisState::Healthy);

        // Degrade to observing
        engine.update_sct_z(1, 0.35, 2000).unwrap();
        assert_eq!(engine.evaluate(1, 2000).unwrap(), ApoptosisState::Observing);

        // Degrade to quarantined
        engine.update_sct_z(1, 0.2, 3000).unwrap();
        assert_eq!(
            engine.evaluate(1, 3000).unwrap(),
            ApoptosisState::Quarantined
        );

        assert!(engine.history.len() >= 2);
    }
}
