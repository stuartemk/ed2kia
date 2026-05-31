//! Stuartian Noospheric Activation Protocol (SNAP) — Sprint 58
//!
//! Global activation engine that monitors network scale and ethical coherence
//! to trigger the GlobalIgnitionEvent, unlocking civilizatorio-level Macro-Concepts.
//!
//! **Core Logic:**
//! - Monitors concurrent node count against ignition threshold (default: 10,000)
//! - Tracks Ethical Resonance Field coherence stability over τ consecutive ticks
//! - Fires GlobalIgnitionEvent when both conditions are satisfied simultaneously
//!
//! **Feature Gate:** `v4.0-snap-activation`

use std::collections::VecDeque;

/// Errors specific to SNAP activation protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapError {
    /// Configuration validation failed.
    InvalidConfig(String),
    /// Network has not reached ignition threshold yet.
    ThresholdNotMet { current: usize, required: usize },
    /// Coherence stability not maintained for required ticks.
    CoherenceUnstable {
        current_ticks: u32,
        required_ticks: u32,
    },
    /// Activation already fired; cannot re-trigger without reset.
    AlreadyActivated,
    /// Reset attempted while active.
    CannotResetWhileActive,
}

impl std::fmt::Display for SnapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapError::InvalidConfig(msg) => write!(f, "SNAP config invalid: {}", msg),
            SnapError::ThresholdNotMet { current, required } => {
                write!(f, "Node threshold not met: {} < {}", current, required)
            }
            SnapError::CoherenceUnstable {
                current_ticks,
                required_ticks,
            } => {
                write!(
                    f,
                    "Coherence stability insufficient: {} < {} ticks",
                    current_ticks, required_ticks
                )
            }
            SnapError::AlreadyActivated => write!(f, "SNAP already activated"),
            SnapError::CannotResetWhileActive => write!(f, "Cannot reset while SNAP is active"),
        }
    }
}

/// Configuration for the SNAP Engine.
#[derive(Debug, Clone)]
pub struct SnapConfig {
    /// Minimum concurrent nodes required for ignition (default: 10,000).
    pub ignition_node_threshold: usize,
    /// Number of consecutive ticks coherence must remain stable (default: 100).
    pub coherence_stability_ticks: u32,
    /// Minimum coherence value for stability (default: 0.85).
    pub min_coherence_threshold: f64,
    /// Maximum coherence history to retain (default: 1000).
    pub max_coherence_history: usize,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            ignition_node_threshold: 10_000,
            coherence_stability_ticks: 100,
            min_coherence_threshold: 0.85,
            max_coherence_history: 1000,
        }
    }
}

impl SnapConfig {
    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), SnapError> {
        if self.ignition_node_threshold == 0 {
            return Err(SnapError::InvalidConfig(
                "ignition_node_threshold must be > 0".to_string(),
            ));
        }
        if self.coherence_stability_ticks == 0 {
            return Err(SnapError::InvalidConfig(
                "coherence_stability_ticks must be > 0".to_string(),
            ));
        }
        if self.min_coherence_threshold < 0.0 || self.min_coherence_threshold > 1.0 {
            return Err(SnapError::InvalidConfig(
                "min_coherence_threshold must be in [0, 1]".to_string(),
            ));
        }
        if self.max_coherence_history == 0 {
            return Err(SnapError::InvalidConfig(
                "max_coherence_history must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Result of a Global Ignition Event.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalIgnitionEvent {
    /// Tick number when ignition occurred.
    pub ignition_tick: u64,
    /// Number of concurrent nodes at ignition.
    pub node_count: usize,
    /// Final coherence value at ignition.
    pub coherence_value: f64,
    /// Number of consecutive stable ticks achieved.
    pub stable_ticks: u32,
}

impl std::fmt::Display for GlobalIgnitionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GlobalIgnitionEvent {{ tick={}, nodes={}, coherence={:.4}, stable_ticks={} }}",
            self.ignition_tick, self.node_count, self.coherence_value, self.stable_ticks
        )
    }
}

/// Activation state of the SNAP Engine.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivationState {
    /// Monitoring conditions; not yet activated.
    Monitoring,
    /// Global Ignition Event fired; network is activated.
    Activated(GlobalIgnitionEvent),
}

/// SNAP Engine — Monitors network for Global Ignition conditions.
#[derive(Debug)]
pub struct SnapEngine {
    config: SnapConfig,
    state: ActivationState,
    current_tick: u64,
    coherence_history: VecDeque<f64>,
    stable_tick_counter: u32,
}

impl SnapEngine {
    /// Create a new SnapEngine with default configuration.
    pub fn new() -> Self {
        Self {
            config: SnapConfig::default(),
            state: ActivationState::Monitoring,
            current_tick: 0,
            coherence_history: VecDeque::with_capacity(1000),
            stable_tick_counter: 0,
        }
    }

    /// Create a SnapEngine with custom configuration.
    pub fn with_config(config: SnapConfig) -> Result<Self, SnapError> {
        config.validate()?;
        let max_history = config.max_coherence_history;
        Ok(Self {
            config,
            state: ActivationState::Monitoring,
            current_tick: 0,
            coherence_history: VecDeque::with_capacity(max_history),
            stable_tick_counter: 0,
        })
    }

    /// Advance the engine by one tick, recording current network state.
    ///
    /// Returns `Some(GlobalIgnitionEvent)` if ignition conditions are met this tick.
    pub fn tick(
        &mut self,
        concurrent_nodes: usize,
        coherence: f64,
    ) -> Result<Option<GlobalIgnitionEvent>, SnapError> {
        if let ActivationState::Activated(_) = self.state {
            return Ok(None);
        }

        self.current_tick += 1;

        // Record coherence
        self.coherence_history.push_back(coherence);
        if self.coherence_history.len() > self.config.max_coherence_history {
            self.coherence_history.pop_front();
        }

        // Check ignition conditions
        let nodes_ok = concurrent_nodes >= self.config.ignition_node_threshold;
        let coherence_ok = coherence >= self.config.min_coherence_threshold;

        if nodes_ok && coherence_ok {
            self.stable_tick_counter += 1;
        } else {
            self.stable_tick_counter = 0;
        }

        // Check if both conditions sustained for required ticks
        if self.stable_tick_counter >= self.config.coherence_stability_ticks
            && nodes_ok
            && coherence_ok
        {
            let event = GlobalIgnitionEvent {
                ignition_tick: self.current_tick,
                node_count: concurrent_nodes,
                coherence_value: coherence,
                stable_ticks: self.stable_tick_counter,
            };
            self.state = ActivationState::Activated(event.clone());
            return Ok(Some(event));
        }

        Ok(None)
    }

    /// Get the current activation state.
    pub fn state(&self) -> &ActivationState {
        &self.state
    }

    /// Get the current tick number.
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the current stable tick counter.
    pub fn stable_ticks(&self) -> u32 {
        self.stable_tick_counter
    }

    /// Get the latest coherence value.
    pub fn latest_coherence(&self) -> Option<f64> {
        self.coherence_history.back().copied()
    }

    /// Get the average coherence over the last N ticks.
    pub fn average_coherence(&self, last_n: usize) -> Option<f64> {
        let len = self.coherence_history.len().min(last_n);
        if len == 0 {
            return None;
        }
        let sum: f64 = self.coherence_history.iter().rev().take(len).sum();
        Some(sum / len as f64)
    }

    /// Reset the engine to initial monitoring state.
    pub fn reset(&mut self) -> Result<(), SnapError> {
        if let ActivationState::Activated(_) = self.state {
            return Err(SnapError::CannotResetWhileActive);
        }
        self.current_tick = 0;
        self.coherence_history.clear();
        self.stable_tick_counter = 0;
        Ok(())
    }

    /// Force reset even if activated (emergency use only).
    pub fn force_reset(&mut self) {
        self.state = ActivationState::Monitoring;
        self.current_tick = 0;
        self.coherence_history.clear();
        self.stable_tick_counter = 0;
    }

    /// Update configuration (only when in Monitoring state).
    pub fn update_config(&mut self, config: SnapConfig) -> Result<(), SnapError> {
        if let ActivationState::Activated(_) = self.state {
            return Err(SnapError::CannotResetWhileActive);
        }
        config.validate()?;
        self.config = config;
        Ok(())
    }

    /// Check if node threshold is currently met.
    pub fn nodes_threshold_met(&self, concurrent_nodes: usize) -> bool {
        concurrent_nodes >= self.config.ignition_node_threshold
    }

    /// Check if coherence is currently stable.
    pub fn coherence_stable(&self, coherence: f64) -> bool {
        coherence >= self.config.min_coherence_threshold
    }
}

impl Default for SnapEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(threshold: usize, stability: u32, min_coherence: f64) -> SnapConfig {
        SnapConfig {
            ignition_node_threshold: threshold,
            coherence_stability_ticks: stability,
            min_coherence_threshold: min_coherence,
            max_coherence_history: 100,
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = SnapEngine::new();
        assert_eq!(engine.state(), &ActivationState::Monitoring);
        assert_eq!(engine.current_tick(), 0);
    }

    #[test]
    fn test_engine_custom_config() {
        let config = make_config(5000, 50, 0.9);
        let engine = SnapEngine::with_config(config).unwrap();
        assert_eq!(engine.state(), &ActivationState::Monitoring);
    }

    #[test]
    fn test_invalid_config_zero_threshold() {
        let config = SnapConfig {
            ignition_node_threshold: 0,
            coherence_stability_ticks: 100,
            min_coherence_threshold: 0.85,
            max_coherence_history: 1000,
        };
        assert!(SnapEngine::with_config(config).is_err());
    }

    #[test]
    fn test_invalid_config_zero_stability() {
        let config = SnapConfig {
            ignition_node_threshold: 10000,
            coherence_stability_ticks: 0,
            min_coherence_threshold: 0.85,
            max_coherence_history: 1000,
        };
        assert!(SnapEngine::with_config(config).is_err());
    }

    #[test]
    fn test_invalid_config_coherence_range() {
        let config = SnapConfig {
            ignition_node_threshold: 10000,
            coherence_stability_ticks: 100,
            min_coherence_threshold: 1.5,
            max_coherence_history: 1000,
        };
        assert!(SnapEngine::with_config(config).is_err());
    }

    #[test]
    fn test_no_ignition_below_threshold() {
        let config = make_config(100, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        for _ in 0..10 {
            let result = engine.tick(50, 0.95).unwrap();
            assert!(result.is_none());
        }
        assert_eq!(engine.state(), &ActivationState::Monitoring);
    }

    #[test]
    fn test_no_ignition_low_coherence() {
        let config = make_config(50, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        for _ in 0..10 {
            let result = engine.tick(200, 0.5).unwrap();
            assert!(result.is_none());
        }
        assert_eq!(engine.state(), &ActivationState::Monitoring);
    }

    #[test]
    fn test_ignition_fires() {
        let config = make_config(100, 5, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        // 4 ticks — not enough stability
        for _ in 0..4 {
            let result = engine.tick(200, 0.9).unwrap();
            assert!(result.is_none());
        }
        assert_eq!(engine.stable_ticks(), 4);

        // 5th tick — ignition!
        let result = engine.tick(200, 0.9).unwrap();
        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.ignition_tick, 5);
        assert_eq!(event.node_count, 200);
        assert!((event.coherence_value - 0.9).abs() < f64::EPSILON);
        assert_eq!(event.stable_ticks, 5);
    }

    #[test]
    fn test_ignition_interrupted() {
        let config = make_config(100, 5, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        // 3 good ticks
        for _ in 0..3 {
            engine.tick(200, 0.9).unwrap();
        }
        assert_eq!(engine.stable_ticks(), 3);

        // 1 bad tick resets counter
        engine.tick(200, 0.5).unwrap();
        assert_eq!(engine.stable_ticks(), 0);

        // Need 5 more good ticks
        for _ in 0..4 {
            engine.tick(200, 0.9).unwrap();
        }
        assert_eq!(engine.stable_ticks(), 4);
    }

    #[test]
    fn test_no_double_ignition() {
        let config = make_config(100, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        // Fire ignition
        for _ in 0..3 {
            engine.tick(200, 0.9).unwrap();
        }
        assert!(matches!(engine.state(), ActivationState::Activated(_)));

        // More ticks should not produce new events
        let result = engine.tick(200, 0.9).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_reset_while_monitoring() {
        let mut engine = SnapEngine::new();
        engine.tick(200, 0.9).unwrap();
        assert_eq!(engine.current_tick(), 1);

        engine.reset().unwrap();
        assert_eq!(engine.current_tick(), 0);
        assert_eq!(engine.stable_ticks(), 0);
    }

    #[test]
    fn test_cannot_reset_while_activated() {
        let config = make_config(100, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        for _ in 0..3 {
            engine.tick(200, 0.9).unwrap();
        }
        assert!(engine.reset().is_err());
    }

    #[test]
    fn test_force_reset_after_activation() {
        let config = make_config(100, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        for _ in 0..3 {
            engine.tick(200, 0.9).unwrap();
        }
        assert!(matches!(engine.state(), ActivationState::Activated(_)));

        engine.force_reset();
        assert_eq!(engine.state(), &ActivationState::Monitoring);
        assert_eq!(engine.current_tick(), 0);
    }

    #[test]
    fn test_average_coherence() {
        let mut engine = SnapEngine::new();
        engine.tick(100, 0.8).unwrap();
        engine.tick(100, 0.9).unwrap();
        engine.tick(100, 1.0).unwrap();

        let avg = engine.average_coherence(3).unwrap();
        assert!((avg - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_average_coherence_empty() {
        let engine = SnapEngine::new();
        assert!(engine.average_coherence(5).is_none());
    }

    #[test]
    fn test_latest_coherence() {
        let mut engine = SnapEngine::new();
        assert!(engine.latest_coherence().is_none());

        engine.tick(100, 0.95).unwrap();
        assert!((engine.latest_coherence().unwrap() - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_config() {
        let mut engine = SnapEngine::new();
        let new_config = make_config(5000, 50, 0.9);
        engine.update_config(new_config).unwrap();
    }

    #[test]
    fn test_cannot_update_config_when_activated() {
        let config = make_config(100, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        for _ in 0..3 {
            engine.tick(200, 0.9).unwrap();
        }
        let new_config = make_config(5000, 50, 0.9);
        assert!(engine.update_config(new_config).is_err());
    }

    #[test]
    fn test_nodes_threshold_met() {
        let engine = SnapEngine::new();
        assert!(engine.nodes_threshold_met(10_000));
        assert!(engine.nodes_threshold_met(15_000));
        assert!(!engine.nodes_threshold_met(9_999));
    }

    #[test]
    fn test_coherence_stable() {
        let engine = SnapEngine::new();
        assert!(engine.coherence_stable(0.9));
        assert!(!engine.coherence_stable(0.8));
    }

    #[test]
    fn test_default() {
        let engine = SnapEngine::default();
        assert_eq!(engine.state(), &ActivationState::Monitoring);
    }

    #[test]
    fn test_error_display() {
        let err = SnapError::ThresholdNotMet {
            current: 500,
            required: 1000,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("500"));
        assert!(msg.contains("1000"));
    }

    #[test]
    fn test_ignition_event_display() {
        let event = GlobalIgnitionEvent {
            ignition_tick: 42,
            node_count: 10_000,
            coherence_value: 0.95,
            stable_ticks: 100,
        };
        let msg = format!("{}", event);
        assert!(msg.contains("tick=42"));
        assert!(msg.contains("nodes=10000"));
    }

    #[test]
    fn test_coherence_history_bounded() {
        let config = make_config(100, 3, 0.8);
        let mut engine = SnapEngine::with_config(config).unwrap();

        for i in 0..200 {
            engine.tick(200, 0.9 + (i % 10) as f64 * 0.01).unwrap();
        }
        assert!(engine.coherence_history.len() <= 100);
    }
}
