//! Global Safeguards â€” Sprint 58
//!
//! Planetary-scale ethical safety protocols for the Noosphere network.
//! Implements Ethical Quarantine (topological isolation) and Global Collective
//! Byzantine_Eviction (coordinated rollback to last known homeostatic state).
//!
//! **Core Protocols:**
//!
//! **Ethical Quarantine:**
//! Automatic topological isolation of sub-networks showing a drastic drop
//! in their Noospheric Health (NH). Quarantined sub-networks cannot propagate
//! Macro-Concepts until NH recovers above the release threshold.
//!
//! **Global Collective Byzantine_Eviction:**
//! The mathematical "panic button." If the global resonance field inverts
//! toward the Lower Focus, the network executes a coordinated rollback to
//! its last verified homeostatic checkpoint.
//!
//! **Feature Gate:** `v4.0-snap-activation`

use std::collections::HashMap;

/// Errors specific to global safeguard operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SafeguardError {
    /// Invalid configuration.
    InvalidConfig(String),
    /// Cannot quarantine an already quarantined sub-network.
    AlreadyQuarantined(u128),
    /// Cannot release a sub-network that is not quarantined.
    NotQuarantined(u128),
    /// Byzantine_Eviction already triggered; cannot perform further actions.
    Byzantine_EvictionActive,
    /// No checkpoint available for rollback.
    NoCheckpointAvailable,
    /// Insufficient data to make a safeguard decision.
    InsufficientData,
}

impl std::fmt::Display for SafeguardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafeguardError::InvalidConfig(msg) => write!(f, "Safeguard config invalid: {}", msg),
            SafeguardError::AlreadyQuarantined(id) => {
                write!(f, "Sub-network {} already quarantined", id)
            }
            SafeguardError::NotQuarantined(id) => write!(f, "Sub-network {} not quarantined", id),
            SafeguardError::Byzantine_EvictionActive => {
                write!(f, "Global Byzantine_Eviction is active")
            }
            SafeguardError::NoCheckpointAvailable => {
                write!(f, "No checkpoint available for rollback")
            }
            SafeguardError::InsufficientData => {
                write!(f, "Insufficient data for safeguard decision")
            }
        }
    }
}

/// Quarantine state for a sub-network.
#[derive(Debug, Clone, PartialEq)]
pub enum QuarantineState {
    /// Sub-network is operating normally.
    Active,
    /// Sub-network is isolated due to low NH.
    Quarantined {
        /// Tick when quarantine started.
        since_tick: u64,
        /// NH value at quarantine time.
        nh_at_quarantine: f64,
        /// Reason for quarantine.
        reason: String,
    },
}

/// Global safeguard state.
#[derive(Debug, Clone, PartialEq)]
pub enum SafeguardState {
    /// All systems operating normally.
    Nominal,
    /// One or more sub-networks are quarantined.
    PartialQuarantine {
        /// Number of quarantined sub-networks.
        quarantined_count: usize,
    },
    /// Global Byzantine_Eviction is active; network is rolling back.
    Byzantine_EvictionActive {
        /// Tick when Byzantine_Eviction was triggered.
        triggered_tick: u64,
        /// Target checkpoint tick.
        target_checkpoint: u64,
    },
}

/// Configuration for global safeguards.
#[derive(Debug, Clone)]
pub struct SafeguardConfig {
    /// NH threshold below which quarantine is triggered (default: 0.3).
    pub quarantine_threshold: f64,
    /// NH threshold below which quarantine is released (default: 0.5).
    pub quarantine_release_threshold: f64,
    /// NH threshold below which global Byzantine_Eviction is triggered (default: 0.1).
    pub Byzantine_Eviction_threshold: f64,
    /// Number of consecutive ticks below Byzantine_Eviction threshold before trigger (default: 5).
    pub Byzantine_Eviction_consecutive_ticks: u32,
    /// Maximum number of checkpoints to retain (default: 50).
    pub max_checkpoints: usize,
    /// Minimum ticks between checkpoints (default: 100).
    pub checkpoint_interval: u32,
}

impl Default for SafeguardConfig {
    fn default() -> Self {
        Self {
            quarantine_threshold: 0.3,
            quarantine_release_threshold: 0.5,
            Byzantine_Eviction_threshold: 0.1,
            Byzantine_Eviction_consecutive_ticks: 5,
            max_checkpoints: 50,
            checkpoint_interval: 100,
        }
    }
}

impl SafeguardConfig {
    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), SafeguardError> {
        if self.quarantine_threshold >= self.quarantine_release_threshold {
            return Err(SafeguardError::InvalidConfig(
                "quarantine_threshold must be < quarantine_release_threshold".to_string(),
            ));
        }
        if self.Byzantine_Eviction_threshold >= self.quarantine_threshold {
            return Err(SafeguardError::InvalidConfig(
                "Byzantine_Eviction_threshold must be < quarantine_threshold".to_string(),
            ));
        }
        if self.Byzantine_Eviction_consecutive_ticks == 0 {
            return Err(SafeguardError::InvalidConfig(
                "Byzantine_Eviction_consecutive_ticks must be > 0".to_string(),
            ));
        }
        if self.max_checkpoints == 0 {
            return Err(SafeguardError::InvalidConfig(
                "max_checkpoints must be > 0".to_string(),
            ));
        }
        if self.checkpoint_interval == 0 {
            return Err(SafeguardError::InvalidConfig(
                "checkpoint_interval must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// A saved homeostatic checkpoint for potential rollback.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Tick number when checkpoint was saved.
    pub tick: u64,
    /// Global NH at checkpoint time.
    pub nh_value: f64,
    /// Number of active nodes at checkpoint time.
    pub active_nodes: usize,
    /// Snapshot of sub-network states (sub_network_id â†’ NH).
    pub sub_network_states: HashMap<u128, f64>,
}

/// Result of a safeguard evaluation tick.
#[derive(Debug, Clone)]
pub struct SafeguardResult {
    /// Current safeguard state.
    pub state: SafeguardState,
    /// Sub-networks newly quarantined this tick.
    pub quarantined: Vec<u128>,
    /// Sub-networks released from quarantine this tick.
    pub released: Vec<u128>,
    /// Whether Byzantine_Eviction was triggered this tick.
    pub Byzantine_Eviction_triggered: bool,
    /// Checkpoint saved this tick (if any).
    pub checkpoint_saved: Option<Checkpoint>,
}

/// Global Safeguards Engine â€” Monitors and enforces ethical boundaries.
#[derive(Debug)]
pub struct GlobalSafeguards {
    config: SafeguardConfig,
    state: SafeguardState,
    current_tick: u64,
    /// Sub-network ID â†’ Quarantine state.
    quarantine_map: HashMap<u128, QuarantineState>,
    /// Saved checkpoints for rollback.
    checkpoints: Vec<Checkpoint>,
    /// Consecutive ticks below Byzantine_Eviction threshold.
    Byzantine_Eviction_counter: u32,
    /// Last checkpoint tick.
    last_checkpoint_tick: u64,
}

impl GlobalSafeguards {
    /// Create with default configuration.
    pub fn new() -> Self {
        Self {
            config: SafeguardConfig::default(),
            state: SafeguardState::Nominal,
            current_tick: 0,
            quarantine_map: HashMap::new(),
            checkpoints: Vec::new(),
            Byzantine_Eviction_counter: 0,
            last_checkpoint_tick: 0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: SafeguardConfig) -> Result<Self, SafeguardError> {
        config.validate()?;
        Ok(Self {
            config,
            state: SafeguardState::Nominal,
            current_tick: 0,
            quarantine_map: HashMap::new(),
            checkpoints: Vec::new(),
            Byzantine_Eviction_counter: 0,
            last_checkpoint_tick: 0,
        })
    }

    /// Evaluate safeguards for one tick given global and sub-network NH values.
    ///
    /// # Arguments
    /// * `global_nh` - Current global Noospheric Health.
    /// * `sub_networks` - Map of sub-network ID â†’ current NH.
    pub fn evaluate(
        &mut self,
        global_nh: f64,
        sub_networks: &HashMap<u128, f64>,
    ) -> Result<SafeguardResult, SafeguardError> {
        if let SafeguardState::Byzantine_EvictionActive { .. } = self.state {
            return Err(SafeguardError::Byzantine_EvictionActive);
        }

        self.current_tick += 1;
        let mut result = SafeguardResult {
            state: self.state.clone(),
            quarantined: Vec::new(),
            released: Vec::new(),
            Byzantine_Eviction_triggered: false,
            checkpoint_saved: None,
        };

        // Check for Byzantine_Eviction first
        if global_nh < self.config.Byzantine_Eviction_threshold {
            self.Byzantine_Eviction_counter += 1;
            if self.Byzantine_Eviction_counter >= self.config.Byzantine_Eviction_consecutive_ticks {
                // Trigger global Byzantine_Eviction
                let target = self.get_latest_checkpoint_tick()?;
                self.state = SafeguardState::Byzantine_EvictionActive {
                    triggered_tick: self.current_tick,
                    target_checkpoint: target,
                };
                result.state = self.state.clone();
                result.Byzantine_Eviction_triggered = true;
                return Ok(result);
            }
        } else {
            self.Byzantine_Eviction_counter = 0;
        }

        // Evaluate each sub-network for quarantine
        for (&id, &nh) in sub_networks {
            let current_state = self
                .quarantine_map
                .get(&id)
                .cloned()
                .unwrap_or(QuarantineState::Active);

            match current_state {
                QuarantineState::Active => {
                    if nh < self.config.quarantine_threshold {
                        self.quarantine_map.insert(
                            id,
                            QuarantineState::Quarantined {
                                since_tick: self.current_tick,
                                nh_at_quarantine: nh,
                                reason: format!(
                                    "NH {:.4} below {:.4}",
                                    nh, self.config.quarantine_threshold
                                ),
                            },
                        );
                        result.quarantined.push(id);
                    }
                }
                QuarantineState::Quarantined { .. } => {
                    if nh >= self.config.quarantine_release_threshold {
                        self.quarantine_map.insert(id, QuarantineState::Active);
                        result.released.push(id);
                    }
                }
            }
        }

        // Update overall state
        let quarantined_count = self
            .quarantine_map
            .values()
            .filter(|s| matches!(s, QuarantineState::Quarantined { .. }))
            .count();

        if quarantined_count > 0 {
            self.state = SafeguardState::PartialQuarantine { quarantined_count };
        } else {
            self.state = SafeguardState::Nominal;
        }
        result.state = self.state.clone();

        // Save checkpoint if interval elapsed
        if self.current_tick - self.last_checkpoint_tick >= self.config.checkpoint_interval as u64 {
            let checkpoint = Checkpoint {
                tick: self.current_tick,
                nh_value: global_nh,
                active_nodes: sub_networks.len(),
                sub_network_states: sub_networks.clone(),
            };
            self.checkpoints.push(checkpoint.clone());
            if self.checkpoints.len() > self.config.max_checkpoints {
                self.checkpoints.remove(0);
            }
            self.last_checkpoint_tick = self.current_tick;
            result.checkpoint_saved = Some(checkpoint);
        }

        Ok(result)
    }

    /// Manually quarantine a sub-network.
    pub fn quarantine_sub_network(
        &mut self,
        sub_network_id: u128,
        nh_value: f64,
        reason: String,
    ) -> Result<(), SafeguardError> {
        if let SafeguardState::Byzantine_EvictionActive { .. } = self.state {
            return Err(SafeguardError::Byzantine_EvictionActive);
        }
        if let Some(QuarantineState::Quarantined { .. }) = self.quarantine_map.get(&sub_network_id)
        {
            return Err(SafeguardError::AlreadyQuarantined(sub_network_id));
        }
        self.quarantine_map.insert(
            sub_network_id,
            QuarantineState::Quarantined {
                since_tick: self.current_tick,
                nh_at_quarantine: nh_value,
                reason,
            },
        );
        Ok(())
    }

    /// Manually release a sub-network from quarantine.
    pub fn release_sub_network(&mut self, sub_network_id: u128) -> Result<(), SafeguardError> {
        if let SafeguardState::Byzantine_EvictionActive { .. } = self.state {
            return Err(SafeguardError::Byzantine_EvictionActive);
        }
        match self.quarantine_map.get(&sub_network_id) {
            None | Some(QuarantineState::Active) => {
                return Err(SafeguardError::NotQuarantined(sub_network_id));
            }
            Some(QuarantineState::Quarantined { .. }) => {
                self.quarantine_map
                    .insert(sub_network_id, QuarantineState::Active);
            }
        }
        Ok(())
    }

    /// Get the quarantine state of a sub-network.
    pub fn get_quarantine_state(&self, sub_network_id: u128) -> Option<&QuarantineState> {
        self.quarantine_map.get(&sub_network_id)
    }

    /// Get the latest checkpoint tick.
    fn get_latest_checkpoint_tick(&self) -> Result<u64, SafeguardError> {
        self.checkpoints
            .last()
            .map(|c| c.tick)
            .ok_or(SafeguardError::NoCheckpointAvailable)
    }

    /// Get the latest checkpoint.
    pub fn latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }

    /// Get all checkpoints.
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Get the current safeguard state.
    pub fn state(&self) -> &SafeguardState {
        &self.state
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the Byzantine_Eviction counter.
    pub fn Byzantine_Eviction_counter(&self) -> u32 {
        self.Byzantine_Eviction_counter
    }

    /// Get the number of quarantined sub-networks.
    pub fn quarantined_count(&self) -> usize {
        self.quarantine_map
            .values()
            .filter(|s| matches!(s, QuarantineState::Quarantined { .. }))
            .count()
    }

    /// Reset all safeguards to initial state.
    pub fn reset(&mut self) {
        self.state = SafeguardState::Nominal;
        self.current_tick = 0;
        self.quarantine_map.clear();
        self.checkpoints.clear();
        self.Byzantine_Eviction_counter = 0;
        self.last_checkpoint_tick = 0;
    }

    /// Force reset even during Byzantine_Eviction (emergency).
    pub fn force_reset(&mut self) {
        self.reset();
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: SafeguardConfig) -> Result<(), SafeguardError> {
        if let SafeguardState::Byzantine_EvictionActive { .. } = self.state {
            return Err(SafeguardError::Byzantine_EvictionActive);
        }
        config.validate()?;
        self.config = config;
        Ok(())
    }
}

impl Default for GlobalSafeguards {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sub_networks(count: usize, nh: f64) -> HashMap<u128, f64> {
        (0..count).map(|i| (i as u128, nh)).collect()
    }

    #[test]
    fn test_safeguards_creation() {
        let s = GlobalSafeguards::new();
        assert_eq!(s.state(), &SafeguardState::Nominal);
        assert_eq!(s.current_tick(), 0);
    }

    #[test]
    fn test_safeguards_custom_config() {
        let config = SafeguardConfig {
            quarantine_threshold: 0.4,
            quarantine_release_threshold: 0.6,
            Byzantine_Eviction_threshold: 0.2,
            Byzantine_Eviction_consecutive_ticks: 3,
            max_checkpoints: 20,
            checkpoint_interval: 50,
        };
        let s = GlobalSafeguards::with_config(config).unwrap();
        assert_eq!(s.config.quarantine_threshold, 0.4);
    }

    #[test]
    fn test_invalid_config_thresholds() {
        let config = SafeguardConfig {
            quarantine_threshold: 0.6,
            quarantine_release_threshold: 0.4,
            ..Default::default()
        };
        assert!(GlobalSafeguards::with_config(config).is_err());
    }

    #[test]
    fn test_nominal_evaluation() {
        let mut s = GlobalSafeguards::new();
        let sub_networks = make_sub_networks(5, 0.8);
        let result = s.evaluate(0.85, &sub_networks).unwrap();
        assert_eq!(result.state, SafeguardState::Nominal);
        assert!(result.quarantined.is_empty());
        assert!(result.released.is_empty());
        assert!(!result.Byzantine_Eviction_triggered);
    }

    #[test]
    fn test_quarantine_triggered() {
        let mut s = GlobalSafeguards::new();
        let mut sub_networks = make_sub_networks(5, 0.8);
        sub_networks.insert(3, 0.2); // Below quarantine threshold

        let result = s.evaluate(0.7, &sub_networks).unwrap();
        assert_eq!(result.quarantined, vec![3]);
        assert!(matches!(
            result.state,
            SafeguardState::PartialQuarantine { .. }
        ));
    }

    #[test]
    fn test_quarantine_released() {
        let mut s = GlobalSafeguards::new();
        // First tick: quarantine
        let mut sub_networks = make_sub_networks(3, 0.8);
        sub_networks.insert(1, 0.2);
        s.evaluate(0.7, &sub_networks).unwrap();

        // Second tick: recover
        sub_networks.insert(1, 0.6);
        let result = s.evaluate(0.75, &sub_networks).unwrap();
        assert_eq!(result.released, vec![1]);
    }

    #[test]
    fn test_Byzantine_Eviction_triggered() {
        let config = SafeguardConfig {
            Byzantine_Eviction_consecutive_ticks: 3,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.05);

        // Tick 1
        s.evaluate(0.05, &sub_networks).unwrap();
        assert_eq!(s.Byzantine_Eviction_counter(), 1);

        // Tick 2
        s.evaluate(0.05, &sub_networks).unwrap();
        assert_eq!(s.Byzantine_Eviction_counter(), 2);

        // Tick 3 â€” Byzantine_Eviction!
        let result = s.evaluate(0.05, &sub_networks).unwrap();
        assert!(result.Byzantine_Eviction_triggered);
        assert!(matches!(
            s.state(),
            SafeguardState::Byzantine_EvictionActive { .. }
        ));
    }

    #[test]
    fn test_Byzantine_Eviction_counter_resets() {
        let config = SafeguardConfig {
            Byzantine_Eviction_consecutive_ticks: 5,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.5);

        s.evaluate(0.05, &sub_networks).unwrap();
        assert_eq!(s.Byzantine_Eviction_counter(), 1);

        // Recover
        s.evaluate(0.5, &sub_networks).unwrap();
        assert_eq!(s.Byzantine_Eviction_counter(), 0);
    }

    #[test]
    fn test_checkpoint_saved() {
        let config = SafeguardConfig {
            checkpoint_interval: 10,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.8);

        for _ in 0..10 {
            s.evaluate(0.8, &sub_networks).unwrap();
        }
        // Checkpoint should be saved at tick 10
        assert!(s.latest_checkpoint().is_some());
        assert_eq!(s.latest_checkpoint().unwrap().tick, 10);
    }

    #[test]
    fn test_manual_quarantine() {
        let mut s = GlobalSafeguards::new();
        s.quarantine_sub_network(1, 0.2, "Manual quarantine".to_string())
            .unwrap();
        assert!(matches!(
            s.get_quarantine_state(1),
            Some(QuarantineState::Quarantined { .. })
        ));
    }

    #[test]
    fn test_double_quarantine_error() {
        let mut s = GlobalSafeguards::new();
        s.quarantine_sub_network(1, 0.2, "First".to_string())
            .unwrap();
        let err = s.quarantine_sub_network(1, 0.2, "Second".to_string());
        assert_eq!(err.unwrap_err(), SafeguardError::AlreadyQuarantined(1));
    }

    #[test]
    fn test_release_non_quarantined_error() {
        let mut s = GlobalSafeguards::new();
        let err = s.release_sub_network(1);
        assert_eq!(err.unwrap_err(), SafeguardError::NotQuarantined(1));
    }

    #[test]
    fn test_manual_release() {
        let mut s = GlobalSafeguards::new();
        s.quarantine_sub_network(1, 0.2, "Test".to_string())
            .unwrap();
        s.release_sub_network(1).unwrap();
        assert_eq!(s.get_quarantine_state(1), Some(&QuarantineState::Active));
    }

    #[test]
    fn test_no_operation_during_Byzantine_Eviction() {
        let config = SafeguardConfig {
            Byzantine_Eviction_consecutive_ticks: 2,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.05);

        s.evaluate(0.05, &sub_networks).unwrap();
        s.evaluate(0.05, &sub_networks).unwrap(); // Byzantine_Eviction triggered

        assert!(s.evaluate(0.05, &sub_networks).is_err());
    }

    #[test]
    fn test_reset() {
        let mut s = GlobalSafeguards::new();
        s.quarantine_sub_network(1, 0.2, "Test".to_string())
            .unwrap();
        s.reset();
        assert_eq!(s.state(), &SafeguardState::Nominal);
        assert_eq!(s.quarantined_count(), 0);
    }

    #[test]
    fn test_force_reset_during_Byzantine_Eviction() {
        let config = SafeguardConfig {
            Byzantine_Eviction_consecutive_ticks: 2,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.05);

        s.evaluate(0.05, &sub_networks).unwrap();
        s.evaluate(0.05, &sub_networks).unwrap(); // Byzantine_Eviction
        assert!(matches!(
            s.state(),
            SafeguardState::Byzantine_EvictionActive { .. }
        ));

        s.force_reset();
        assert_eq!(s.state(), &SafeguardState::Nominal);
    }

    #[test]
    fn test_quarantined_count() {
        let mut s = GlobalSafeguards::new();
        s.quarantine_sub_network(1, 0.2, "Test".to_string())
            .unwrap();
        s.quarantine_sub_network(2, 0.1, "Test".to_string())
            .unwrap();
        assert_eq!(s.quarantined_count(), 2);
    }

    #[test]
    fn test_checkpoint_bounded() {
        let config = SafeguardConfig {
            checkpoint_interval: 5,
            max_checkpoints: 5,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.8);

        for _ in 0..50 {
            s.evaluate(0.8, &sub_networks).unwrap();
        }
        assert!(s.checkpoints().len() <= 5);
    }

    #[test]
    fn test_update_config() {
        let mut s = GlobalSafeguards::new();
        let config = SafeguardConfig {
            quarantine_threshold: 0.4,
            quarantine_release_threshold: 0.6,
            Byzantine_Eviction_threshold: 0.2,
            Byzantine_Eviction_consecutive_ticks: 3,
            max_checkpoints: 20,
            checkpoint_interval: 50,
        };
        s.update_config(config).unwrap();
        assert_eq!(s.config.quarantine_threshold, 0.4);
    }

    #[test]
    fn test_cannot_update_config_during_Byzantine_Eviction() {
        let config = SafeguardConfig {
            Byzantine_Eviction_consecutive_ticks: 2,
            ..Default::default()
        };
        let mut s = GlobalSafeguards::with_config(config).unwrap();
        let sub_networks = make_sub_networks(3, 0.05);

        s.evaluate(0.05, &sub_networks).unwrap();
        s.evaluate(0.05, &sub_networks).unwrap(); // Byzantine_Eviction

        let new_config = SafeguardConfig::default();
        assert!(s.update_config(new_config).is_err());
    }

    #[test]
    fn test_default() {
        let s = GlobalSafeguards::default();
        assert_eq!(s.state(), &SafeguardState::Nominal);
    }

    #[test]
    fn test_error_display() {
        let err = SafeguardError::AlreadyQuarantined(42);
        let msg = format!("{}", err);
        assert!(msg.contains("42"));
    }

    #[test]
    fn test_config_validate_valid() {
        let config = SafeguardConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_no_checkpoint_error() {
        let s = GlobalSafeguards::new();
        // No checkpoints saved yet
        assert!(s.latest_checkpoint().is_none());
    }
}
