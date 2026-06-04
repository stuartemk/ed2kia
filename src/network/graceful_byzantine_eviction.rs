//! Graceful Byzantine_Eviction â€” Sprint 73: Pragmatic Pivot & Asymptotic Hardening
//!
//! Ventanas de cuarentena, observaciÃ³n acotada, reintegraciÃ³n por consenso Îµ-tolerante.
//! PrevenciÃ³n de particiÃ³n vÃ­a bounded failure domains.
//!
//! **Pivot ArquitectÃ³nico:** Byzantine_Eviction con riesgo de particiÃ³n en cascada mitigado
//! mediante dominios de fallo acotados y reintegraciÃ³n controlada.

use std::collections::HashMap;
use std::fmt;

/// Error types for Graceful Byzantine_Eviction
#[derive(Debug, Clone, PartialEq)]
pub enum Byzantine_EvictionError {
    /// Invalid SCT-Z value
    InvalidSctZ(f64),
    /// Node not found
    NodeNotFound(u64),
    /// Already in Byzantine_Eviction
    AlreadyApoptosing(u64),
    /// Quarantine capacity exceeded
    QuarantineFull(usize),
    /// Reintegration rejected
    ReintegrationRejected(String),
    /// Cascade prevention triggered
    CascadePrevented,
}

impl fmt::Display for Byzantine_EvictionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Byzantine_EvictionError::InvalidSctZ(v) => write!(f, "Invalid SCT-Z: {}", v),
            Byzantine_EvictionError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Byzantine_EvictionError::AlreadyApoptosing(id) => {
                write!(f, "Node already apoptosing: {}", id)
            }
            Byzantine_EvictionError::QuarantineFull(cap) => {
                write!(f, "Quarantine capacity full: {}", cap)
            }
            Byzantine_EvictionError::ReintegrationRejected(msg) => {
                write!(f, "Reintegration rejected: {}", msg)
            }
            Byzantine_EvictionError::CascadePrevented => {
                write!(
                    f,
                    "Cascade prevention triggered â€” Byzantine_Eviction blocked"
                )
            }
        }
    }
}

/// Byzantine_Eviction state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Byzantine_EvictionState {
    /// Healthy â€” normal operation
    Healthy,
    /// Observing â€” monitoring for anomalies
    Observing,
    /// Quarantined â€” isolated from mesh
    Quarantined,
    /// Apoptosing â€” preparing for removal
    Apoptosing,
    /// Reintegrating â€” attempting re-entry
    Reintegrating,
    /// Removed â€” permanently excised
    Removed,
}

impl fmt::Display for Byzantine_EvictionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Byzantine_EvictionState::Healthy => write!(f, "Healthy"),
            Byzantine_EvictionState::Observing => write!(f, "Observing"),
            Byzantine_EvictionState::Quarantined => write!(f, "Quarantined"),
            Byzantine_EvictionState::Apoptosing => write!(f, "Apoptosing"),
            Byzantine_EvictionState::Reintegrating => write!(f, "Reintegrating"),
            Byzantine_EvictionState::Removed => write!(f, "Removed"),
        }
    }
}

/// Configuration for Graceful Byzantine_Eviction
#[derive(Debug, Clone)]
pub struct Byzantine_EvictionConfig {
    /// SCT-Z threshold for triggering observation
    pub observation_threshold: f64,
    /// SCT-Z threshold for triggering quarantine
    pub quarantine_threshold: f64,
    /// SCT-Z threshold for triggering Byzantine_Eviction
    pub Byzantine_Eviction_threshold: f64,
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
    /// Maximum concurrent Byzantine_Eviction (cascade limit)
    pub max_concurrent_Byzantine_Eviction: usize,
}

impl Byzantine_EvictionConfig {
    pub fn default_Topological() -> Self {
        Self {
            observation_threshold: 0.4,
            quarantine_threshold: 0.25,
            Byzantine_Eviction_threshold: 0.1,
            observation_window_ms: 30_000,
            quarantine_window_ms: 120_000,
            max_quarantine_size: 100,
            reintegration_epsilon: 0.1,
            min_reintegration_neighbors: 3,
            max_concurrent_Byzantine_Eviction: 5,
        }
    }

    pub fn validate(&self) -> Result<(), Byzantine_EvictionError> {
        if self.observation_threshold < 0.0 || self.observation_threshold > 1.0 {
            return Err(Byzantine_EvictionError::InvalidSctZ(
                self.observation_threshold,
            ));
        }
        if self.quarantine_threshold >= self.observation_threshold {
            return Err(Byzantine_EvictionError::InvalidSctZ(
                self.quarantine_threshold,
            ));
        }
        if self.Byzantine_Eviction_threshold >= self.quarantine_threshold {
            return Err(Byzantine_EvictionError::InvalidSctZ(
                self.Byzantine_Eviction_threshold,
            ));
        }
        if self.max_quarantine_size == 0 {
            return Err(Byzantine_EvictionError::QuarantineFull(0));
        }
        Ok(())
    }
}

impl Default for Byzantine_EvictionConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// Node state in Byzantine_Eviction system
#[derive(Debug, Clone)]
pub struct Byzantine_EvictionNodeState {
    pub node_id: u64,
    pub state: Byzantine_EvictionState,
    pub sct_z: f64,
    pub state_entered_ms: u64,
    pub neighbor_count: usize,
    pub reintegration_votes: usize,
}

impl Byzantine_EvictionNodeState {
    pub fn new(node_id: u64, neighbor_count: usize) -> Self {
        Self {
            node_id,
            state: Byzantine_EvictionState::Healthy,
            sct_z: 1.0,
            state_entered_ms: 0,
            neighbor_count,
            reintegration_votes: 0,
        }
    }
}

impl fmt::Display for Byzantine_EvictionNodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Byzantine_EvictionNodeState {{ id: {}, state: {}, sct_z: {:.4}, neighbors: {} }}",
            self.node_id, self.state, self.sct_z, self.neighbor_count
        )
    }
}

/// Record of Byzantine_Eviction event
#[derive(Debug, Clone)]
pub struct Byzantine_EvictionRecord {
    pub node_id: u64,
    pub from_state: Byzantine_EvictionState,
    pub to_state: Byzantine_EvictionState,
    pub sct_z: f64,
    pub timestamp_ms: u64,
    pub reason: String,
}

impl fmt::Display for Byzantine_EvictionRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Byzantine_EvictionRecord {{ node: {}, {} â†’ {}, sct_z: {:.4}, reason: {}, ts: {} }}",
            self.node_id,
            self.from_state,
            self.to_state,
            self.sct_z,
            self.reason,
            self.timestamp_ms
        )
    }
}

/// Graceful Byzantine_Eviction Engine
pub struct GracefulByzantine_Eviction {
    config: Byzantine_EvictionConfig,
    nodes: HashMap<u64, Byzantine_EvictionNodeState>,
    history: Vec<Byzantine_EvictionRecord>,
}

impl GracefulByzantine_Eviction {
    pub fn new() -> Self {
        Self {
            config: Byzantine_EvictionConfig::default_Topological(),
            nodes: HashMap::new(),
            history: Vec::new(),
        }
    }

    pub fn with_config(config: Byzantine_EvictionConfig) -> Result<Self, Byzantine_EvictionError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            history: Vec::new(),
        })
    }

    /// Register a node
    pub fn register_node(&mut self, node_id: u64, neighbor_count: usize) {
        self.nodes.insert(
            node_id,
            Byzantine_EvictionNodeState::new(node_id, neighbor_count),
        );
    }

    /// Update SCT-Z for a node
    pub fn update_sct_z(
        &mut self,
        node_id: u64,
        sct_z: f64,
        _current_ms: u64,
    ) -> Result<(), Byzantine_EvictionError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(Byzantine_EvictionError::NodeNotFound(node_id))?;

        if sct_z < 0.0 || sct_z > 1.0 {
            return Err(Byzantine_EvictionError::InvalidSctZ(sct_z));
        }

        node.sct_z = sct_z;
        Ok(())
    }

    /// Evaluate Byzantine_Eviction trigger for a node
    pub fn evaluate(
        &mut self,
        node_id: u64,
        current_ms: u64,
    ) -> Result<Byzantine_EvictionState, Byzantine_EvictionError> {
        // Extract node values to avoid simultaneous borrows
        let (old_state, sct_z, state_entered_ms, neighbor_count, reintegration_votes) = {
            let node = self
                .nodes
                .get(&node_id)
                .ok_or(Byzantine_EvictionError::NodeNotFound(node_id))?;
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
                n.state == Byzantine_EvictionState::Apoptosing
                    || n.state == Byzantine_EvictionState::Quarantined
            })
            .count();

        let (new_state, reason) = match old_state {
            Byzantine_EvictionState::Healthy => (
                Self::evaluate_healthy_values(&self.config, sct_z),
                "threshold_evaluation".to_string(),
            ),
            Byzantine_EvictionState::Observing => (
                Self::evaluate_observing_values(&self.config, sct_z, state_entered_ms, current_ms),
                "observation_evaluation".to_string(),
            ),
            Byzantine_EvictionState::Quarantined => (
                Self::evaluate_quarantined_values(
                    &self.config,
                    sct_z,
                    state_entered_ms,
                    current_ms,
                    apoptosing_count,
                ),
                "quarantine_evaluation".to_string(),
            ),
            Byzantine_EvictionState::Apoptosing => (
                Byzantine_EvictionState::Removed,
                "Byzantine_Eviction_complete".to_string(),
            ),
            Byzantine_EvictionState::Reintegrating => (
                Self::evaluate_reintegrating_values(
                    &self.config,
                    sct_z,
                    neighbor_count,
                    reintegration_votes,
                ),
                "reintegration_evaluation".to_string(),
            ),
            Byzantine_EvictionState::Removed => {
                // Terminal state â€” no transition
                return Ok(old_state);
            }
        };

        self.transition(node_id, old_state, new_state, current_ms, reason);
        Ok(new_state)
    }

    fn evaluate_healthy_values(
        config: &Byzantine_EvictionConfig,
        sct_z: f64,
    ) -> Byzantine_EvictionState {
        if sct_z < config.Byzantine_Eviction_threshold {
            return Byzantine_EvictionState::Apoptosing;
        }
        if sct_z < config.quarantine_threshold {
            return Byzantine_EvictionState::Quarantined;
        }
        if sct_z < config.observation_threshold {
            return Byzantine_EvictionState::Observing;
        }
        Byzantine_EvictionState::Healthy
    }

    fn evaluate_observing_values(
        config: &Byzantine_EvictionConfig,
        sct_z: f64,
        state_entered_ms: u64,
        current_ms: u64,
    ) -> Byzantine_EvictionState {
        let elapsed = current_ms.saturating_sub(state_entered_ms);

        if sct_z >= config.observation_threshold {
            return Byzantine_EvictionState::Healthy;
        }
        if sct_z < config.Byzantine_Eviction_threshold {
            return Byzantine_EvictionState::Apoptosing;
        }
        if sct_z < config.quarantine_threshold {
            return Byzantine_EvictionState::Quarantined;
        }
        if elapsed > config.observation_window_ms {
            return Byzantine_EvictionState::Quarantined;
        }
        Byzantine_EvictionState::Observing
    }

    fn evaluate_quarantined_values(
        config: &Byzantine_EvictionConfig,
        sct_z: f64,
        state_entered_ms: u64,
        current_ms: u64,
        apoptosing_count: usize,
    ) -> Byzantine_EvictionState {
        let elapsed = current_ms.saturating_sub(state_entered_ms);

        if sct_z >= config.observation_threshold {
            return Byzantine_EvictionState::Reintegrating;
        }
        if sct_z < config.Byzantine_Eviction_threshold {
            // Cascade prevention
            if apoptosing_count >= config.max_concurrent_Byzantine_Eviction {
                return Byzantine_EvictionState::Quarantined; // Block cascade
            }
            return Byzantine_EvictionState::Apoptosing;
        }
        if elapsed > config.quarantine_window_ms {
            return Byzantine_EvictionState::Apoptosing;
        }
        Byzantine_EvictionState::Quarantined
    }

    fn evaluate_reintegrating_values(
        config: &Byzantine_EvictionConfig,
        sct_z: f64,
        neighbor_count: usize,
        reintegration_votes: usize,
    ) -> Byzantine_EvictionState {
        // Îµ-tolerant consensus for reintegration
        let required_votes = if neighbor_count >= config.min_reintegration_neighbors {
            let threshold =
                neighbor_count as f64 * (2.0 / 3.0 * (1.0 - config.reintegration_epsilon));
            threshold as usize
        } else {
            neighbor_count
        };

        if reintegration_votes >= required_votes && sct_z >= config.observation_threshold {
            return Byzantine_EvictionState::Healthy;
        }
        if sct_z < config.quarantine_threshold {
            return Byzantine_EvictionState::Quarantined;
        }
        Byzantine_EvictionState::Reintegrating
    }

    fn transition(
        &mut self,
        node_id: u64,
        from: Byzantine_EvictionState,
        to: Byzantine_EvictionState,
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
        self.history.push(Byzantine_EvictionRecord {
            node_id,
            from_state: from,
            to_state: to,
            sct_z,
            timestamp_ms,
            reason,
        });
    }

    /// Cast reintegration vote for a node
    pub fn vote_reintegration(&mut self, node_id: u64) -> Result<(), Byzantine_EvictionError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(Byzantine_EvictionError::NodeNotFound(node_id))?;
        if node.state != Byzantine_EvictionState::Reintegrating {
            return Err(Byzantine_EvictionError::ReintegrationRejected(format!(
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
                n.state == Byzantine_EvictionState::Quarantined
                    || n.state == Byzantine_EvictionState::Apoptosing
            })
            .count();
        quarantined >= self.config.max_concurrent_Byzantine_Eviction
    }

    /// Get nodes in a specific state
    pub fn nodes_in_state(&self, state: Byzantine_EvictionState) -> Vec<u64> {
        self.nodes
            .values()
            .filter(|n| n.state == state)
            .map(|n| n.node_id)
            .collect()
    }

    /// Get stats
    pub fn stats(&self) -> HashMap<Byzantine_EvictionState, usize> {
        let mut counts = HashMap::new();
        *counts.entry(Byzantine_EvictionState::Healthy).or_insert(0) += 0;
        *counts
            .entry(Byzantine_EvictionState::Observing)
            .or_insert(0) += 0;
        *counts
            .entry(Byzantine_EvictionState::Quarantined)
            .or_insert(0) += 0;
        *counts
            .entry(Byzantine_EvictionState::Apoptosing)
            .or_insert(0) += 0;
        *counts
            .entry(Byzantine_EvictionState::Reintegrating)
            .or_insert(0) += 0;
        *counts.entry(Byzantine_EvictionState::Removed).or_insert(0) += 0;

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

impl Default for GracefulByzantine_Eviction {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GracefulByzantine_Eviction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _stats = self.stats();
        write!(
            f,
            "GracefulByzantine_Eviction {{ nodes: {}, history: {}, cascade_risk: {} }}",
            self.nodes.len(),
            self.history.len(),
            self.is_cascade_risk()
        )
    }
}

/// Public function: Evaluate Byzantine_Eviction trigger
pub fn evaluate_Byzantine_Eviction_trigger(
    sct_z: f64,
    _observation_window_ms: u64,
    neighbor_consensus: f64,
) -> Byzantine_EvictionState {
    let config = Byzantine_EvictionConfig::default_Topological();

    if sct_z < config.Byzantine_Eviction_threshold {
        // Check neighbor consensus for cascade prevention
        if neighbor_consensus < 0.5 {
            return Byzantine_EvictionState::Quarantined; // Block cascade
        }
        return Byzantine_EvictionState::Apoptosing;
    }
    if sct_z < config.quarantine_threshold {
        return Byzantine_EvictionState::Quarantined;
    }
    if sct_z < config.observation_threshold {
        return Byzantine_EvictionState::Observing;
    }
    Byzantine_EvictionState::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Byzantine_EvictionConfig::default();
        assert!(config.observation_threshold > config.quarantine_threshold);
        assert!(config.quarantine_threshold > config.Byzantine_Eviction_threshold);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = Byzantine_EvictionConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let config = Byzantine_EvictionConfig {
            observation_threshold: 0.2,
            quarantine_threshold: 0.3,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_engine_creation() {
        let engine = GracefulByzantine_Eviction::new();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        assert_eq!(engine.nodes.len(), 1);
    }

    #[test]
    fn test_update_sct_z() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        assert!(engine.update_sct_z(1, 0.8, 1000).is_ok());
    }

    #[test]
    fn test_evaluate_healthy() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.9, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, Byzantine_EvictionState::Healthy);
    }

    #[test]
    fn test_evaluate_observing() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.35, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, Byzantine_EvictionState::Observing);
    }

    #[test]
    fn test_evaluate_quarantined() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.2, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, Byzantine_EvictionState::Quarantined);
    }

    #[test]
    fn test_evaluate_apoptosing() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        engine.update_sct_z(1, 0.05, 1000).unwrap();
        let state = engine.evaluate(1, 1000).unwrap();
        assert_eq!(state, Byzantine_EvictionState::Apoptosing);
    }

    #[test]
    fn test_cascade_prevention() {
        let mut engine = GracefulByzantine_Eviction::with_config(Byzantine_EvictionConfig {
            max_concurrent_Byzantine_Eviction: 2,
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
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        // Set to reintegrating manually for test
        engine.nodes.get_mut(&1).unwrap().state = Byzantine_EvictionState::Reintegrating;
        engine.nodes.get_mut(&1).unwrap().sct_z = 0.5;
        assert!(engine.vote_reintegration(1).is_ok());
    }

    #[test]
    fn test_nodes_in_state() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        engine.register_node(2, 5);
        let healthy = engine.nodes_in_state(Byzantine_EvictionState::Healthy);
        assert_eq!(healthy.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        let stats = engine.stats();
        assert!(stats.contains_key(&Byzantine_EvictionState::Healthy));
    }

    #[test]
    fn test_reset() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);
        engine.reset();
        assert_eq!(engine.nodes.len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = GracefulByzantine_Eviction::new();
        let s = format!("{}", engine);
        assert!(s.contains("GracefulByzantine_Eviction"));
    }

    #[test]
    fn test_evaluate_Byzantine_Eviction_trigger_healthy() {
        let state = evaluate_Byzantine_Eviction_trigger(0.9, 30000, 0.8);
        assert_eq!(state, Byzantine_EvictionState::Healthy);
    }

    #[test]
    fn test_evaluate_Byzantine_Eviction_trigger_quarantine() {
        let state = evaluate_Byzantine_Eviction_trigger(0.2, 30000, 0.8);
        assert_eq!(state, Byzantine_EvictionState::Quarantined);
    }

    #[test]
    fn test_evaluate_Byzantine_Eviction_trigger_cascade_block() {
        let state = evaluate_Byzantine_Eviction_trigger(0.05, 30000, 0.3);
        assert_eq!(state, Byzantine_EvictionState::Quarantined);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = GracefulByzantine_Eviction::new();
        engine.register_node(1, 5);

        // Start healthy
        engine.update_sct_z(1, 0.9, 1000).unwrap();
        assert_eq!(
            engine.evaluate(1, 1000).unwrap(),
            Byzantine_EvictionState::Healthy
        );

        // Degrade to observing
        engine.update_sct_z(1, 0.35, 2000).unwrap();
        assert_eq!(
            engine.evaluate(1, 2000).unwrap(),
            Byzantine_EvictionState::Observing
        );

        // Degrade to quarantined
        engine.update_sct_z(1, 0.2, 3000).unwrap();
        assert_eq!(
            engine.evaluate(1, 3000).unwrap(),
            Byzantine_EvictionState::Quarantined
        );

        assert!(engine.history.len() >= 2);
    }
}
