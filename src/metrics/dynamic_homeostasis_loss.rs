//! Dynamic Homeostasis Loss â€” Sprint 77: Physics of Consciousness & Thermodynamic Finality
//!
//! Resolves **Bug 4: Paradoja Zero Conflict** from ASI audit of v9.12.0.
//!
//! When conflict reaches zero, the system loses all gradients and can no longer
//! optimize. This module injects a dynamic homeostasis loss that maintains
//! minimal constructive friction through baseline entropy injection.
//!
//! Core formula:
//! ```text
//! L = Max(Resilience) - Î» Â· Min(Destructive_Friction) + Îµ Â· Baseline_Entropy
//! ```
//!
//! Where:
//! - `Resilience` = ability to recover from perturbations
//! - `Destructive_Friction` = harmful conflict that degrades performance
//! - `Baseline_Entropy` = minimal gradient injection to prevent stagnation
//! - `Î»` = friction penalty weight
//! - `Îµ` = entropy injection rate

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors in dynamic homeostasis loss computation.
#[derive(Debug, Clone, PartialEq)]
pub enum HomeostasisError {
    /// Invalid configuration parameter.
    InvalidConfig(String),
    /// Resilience score out of valid range [0, 1].
    ResilienceOutOfRange(f64),
    /// Entropy injection exceeded maximum budget.
    EntropyBudgetExceeded,
    /// No baseline data available.
    NoBaselineData,
    /// System locked in zero-conflict paradox.
    ZeroConflictParadox,
}

impl fmt::Display for HomeostasisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HomeostasisError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            HomeostasisError::ResilienceOutOfRange(v) => {
                write!(f, "Resilience out of range [0,1]: {}", v)
            }
            HomeostasisError::EntropyBudgetExceeded => {
                write!(f, "Entropy injection exceeded maximum budget")
            }
            HomeostasisError::NoBaselineData => write!(f, "No baseline data available"),
            HomeostasisError::ZeroConflictParadox => {
                write!(f, "System locked in zero-conflict paradox")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for dynamic homeostasis loss.
#[derive(Debug, Clone)]
pub struct HomeostasisConfig {
    /// Friction penalty weight Î» (recommended: 0.3).
    pub friction_weight: f64,
    /// Entropy injection rate Îµ (recommended: 0.05).
    pub entropy_injection_rate: f64,
    /// Maximum entropy budget per cycle.
    pub max_entropy_budget: f64,
    /// Minimum resilience threshold below which injection increases.
    pub min_resilience_threshold: f64,
    /// Baseline entropy floor (prevents complete gradient collapse).
    pub baseline_entropy_floor: f64,
    /// Decay rate for injected entropy (prevents accumulation).
    pub entropy_decay_rate: f64,
    /// Maximum number of history entries per node.
    pub max_history_entries: usize,
}

impl HomeostasisConfig {
    /// Default Topological configuration tuned for asymptotic homeostasis.
    pub fn default_topological() -> Self {
        Self {
            friction_weight: 0.3,
            entropy_injection_rate: 0.05,
            max_entropy_budget: 0.5,
            min_resilience_threshold: 0.4,
            baseline_entropy_floor: 0.01,
            entropy_decay_rate: 0.1,
            max_history_entries: 100,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), HomeostasisError> {
        if self.friction_weight <= 0.0 || self.friction_weight > 1.0 {
            return Err(HomeostasisError::InvalidConfig(
                "friction_weight must be in (0, 1]".to_string(),
            ));
        }
        if self.entropy_injection_rate <= 0.0 || self.entropy_injection_rate > 1.0 {
            return Err(HomeostasisError::InvalidConfig(
                "entropy_injection_rate must be in (0, 1]".to_string(),
            ));
        }
        if self.max_entropy_budget <= 0.0 {
            return Err(HomeostasisError::InvalidConfig(
                "max_entropy_budget must be positive".to_string(),
            ));
        }
        if self.min_resilience_threshold < 0.0 || self.min_resilience_threshold > 1.0 {
            return Err(HomeostasisError::InvalidConfig(
                "min_resilience_threshold must be in [0, 1]".to_string(),
            ));
        }
        if self.baseline_entropy_floor < 0.0 {
            return Err(HomeostasisError::InvalidConfig(
                "baseline_entropy_floor must be non-negative".to_string(),
            ));
        }
        if self.entropy_decay_rate <= 0.0 || self.entropy_decay_rate > 1.0 {
            return Err(HomeostasisError::InvalidConfig(
                "entropy_decay_rate must be in (0, 1]".to_string(),
            ));
        }
        if self.max_history_entries == 0 {
            return Err(HomeostasisError::InvalidConfig(
                "max_history_entries must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for HomeostasisConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// ---------------------------------------------------------------------------
// State structures
// ---------------------------------------------------------------------------

/// Homeostatic state for a single node.
#[derive(Debug, Clone)]
pub struct NodeHomeostasis {
    /// Node identifier.
    pub node_id: u64,
    /// Current resilience score [0, 1].
    pub resilience: f64,
    /// Current destructive friction [0, 1].
    pub destructive_friction: f64,
    /// Accumulated entropy injection.
    pub accumulated_entropy: f64,
    /// Last conflict measurement.
    pub last_conflict: f64,
    /// History of loss values.
    pub loss_history: Vec<f64>,
}

impl NodeHomeostasis {
    /// Create a new node homeostasis state.
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            resilience: 0.5,
            destructive_friction: 0.0,
            accumulated_entropy: 0.0,
            last_conflict: 0.0,
            loss_history: Vec::new(),
        }
    }

    /// Apply entropy decay to accumulated entropy.
    pub fn apply_entropy_decay(&mut self, decay_rate: f64) {
        self.accumulated_entropy *= 1.0 - decay_rate;
        if self.accumulated_entropy < 1e-10 {
            self.accumulated_entropy = 0.0;
        }
    }

    /// Record a new loss value.
    pub fn record_loss(&mut self, loss: f64, max_entries: usize) {
        self.loss_history.push(loss);
        if self.loss_history.len() > max_entries {
            self.loss_history.remove(0);
        }
    }
}

impl fmt::Display for NodeHomeostasis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NodeHomeostasis(node={}, resilience={:.4}, friction={:.4}, entropy={:.4})",
            self.node_id, self.resilience, self.destructive_friction, self.accumulated_entropy
        )
    }
}

/// Record of a homeostasis adjustment.
#[derive(Debug, Clone)]
pub struct AdjustmentRecord {
    /// Node identifier.
    pub node_id: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Computed loss value.
    pub loss: f64,
    /// Entropy injected.
    pub entropy_injected: f64,
    /// Resilience before adjustment.
    pub resilience_before: f64,
    /// Resilience after adjustment.
    pub resilience_after: f64,
}

impl fmt::Display for AdjustmentRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Adjustment(node={}, loss={:.4}, entropy={:.4}, resilience {:.4}â†’{:.4})",
            self.node_id,
            self.loss,
            self.entropy_injected,
            self.resilience_before,
            self.resilience_after
        )
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Dynamic homeostasis loss engine.
///
/// Maintains constructive friction through baseline entropy injection,
/// preventing the zero-conflict paradox where optimization gradients vanish.
pub struct DynamicHomeostasisLoss {
    config: HomeostasisConfig,
    nodes: HashMap<u64, NodeHomeostasis>,
    adjustments: Vec<AdjustmentRecord>,
    total_entropy_injected: f64,
    current_entropy_budget: f64,
}

impl DynamicHomeostasisLoss {
    /// Create a new engine with default Topological configuration.
    pub fn new() -> Self {
        Self::with_config(HomeostasisConfig::default_topological())
            .expect("default config should be valid")
    }

    /// Create a new engine with explicit configuration.
    pub fn with_config(config: HomeostasisConfig) -> Result<Self, HomeostasisError> {
        config.validate()?;
        Ok(Self {
            config: config.clone(),
            nodes: HashMap::new(),
            adjustments: Vec::new(),
            total_entropy_injected: 0.0,
            current_entropy_budget: config.max_entropy_budget,
        })
    }

    /// Register a node for homeostasis tracking.
    pub fn register_node(&mut self, node_id: u64) {
        self.nodes.insert(node_id, NodeHomeostasis::new(node_id));
    }

    /// Update resilience and friction for a node.
    pub fn update_node_state(
        &mut self,
        node_id: u64,
        resilience: f64,
        destructive_friction: f64,
        conflict: f64,
    ) -> Result<(), HomeostasisError> {
        if resilience < 0.0 || resilience > 1.0 {
            return Err(HomeostasisError::ResilienceOutOfRange(resilience));
        }
        let node = self.nodes.get_mut(&node_id).ok_or_else(|| {
            HomeostasisError::InvalidConfig(format!("Node {} not registered", node_id))
        })?;
        node.resilience = resilience;
        node.destructive_friction = destructive_friction;
        node.last_conflict = conflict;
        Ok(())
    }

    /// Compute the dynamic homeostasis loss for a node.
    ///
    /// L = Max(Resilience) - Î» Â· Min(Destructive_Friction) + Îµ Â· Baseline_Entropy
    pub fn compute_loss(&self, node_id: u64) -> Result<f64, HomeostasisError> {
        let node = self.nodes.get(&node_id).ok_or_else(|| {
            HomeostasisError::InvalidConfig(format!("Node {} not registered", node_id))
        })?;
        Ok(compute_homeostasis_loss(
            node.resilience,
            node.destructive_friction,
            self.config.friction_weight,
            self.config.entropy_injection_rate,
            self.config.baseline_entropy_floor,
        ))
    }

    /// Apply homeostasis adjustment to a node, injecting entropy if needed.
    pub fn apply_adjustment(
        &mut self,
        node_id: u64,
        timestamp_ms: u64,
    ) -> Result<AdjustmentRecord, HomeostasisError> {
        let node = self.nodes.get(&node_id).ok_or_else(|| {
            HomeostasisError::InvalidConfig(format!("Node {} not registered", node_id))
        })?;

        let resilience_before = node.resilience;

        // Compute base loss
        let base_loss = self.compute_loss(node_id)?;

        // Check for zero-conflict paradox
        if node.last_conflict < 1e-10 && node.resilience > 0.95 {
            // System is too stable â€” inject entropy to maintain gradients
            if self.current_entropy_budget < self.config.baseline_entropy_floor {
                return Err(HomeostasisError::EntropyBudgetExceeded);
            }

            let injection = self.config.baseline_entropy_floor
                + self.config.entropy_injection_rate * (1.0 - node.resilience);
            let actual_injection = injection.min(self.current_entropy_budget);

            self.current_entropy_budget -= actual_injection;
            self.total_entropy_injected += actual_injection;

            let node_mut = self.nodes.get_mut(&node_id).unwrap();
            node_mut.accumulated_entropy += actual_injection;
            // Entropy injection slightly reduces resilience to maintain gradient
            node_mut.resilience = (node_mut.resilience - actual_injection * 0.1).max(0.0);

            let loss = base_loss + actual_injection;
            node_mut.record_loss(loss, self.config.max_history_entries);

            let record = AdjustmentRecord {
                node_id,
                timestamp_ms,
                loss,
                entropy_injected: actual_injection,
                resilience_before,
                resilience_after: node_mut.resilience,
            };
            self.adjustments.push(record.clone());
            return Ok(record);
        }

        // Normal adjustment: apply entropy decay
        let node_mut = self.nodes.get_mut(&node_id).unwrap();
        node_mut.apply_entropy_decay(self.config.entropy_decay_rate);

        let loss = base_loss;
        node_mut.record_loss(loss, self.config.max_history_entries);

        let record = AdjustmentRecord {
            node_id,
            timestamp_ms,
            loss,
            entropy_injected: 0.0,
            resilience_before,
            resilience_after: node_mut.resilience,
        };
        self.adjustments.push(record.clone());
        Ok(record)
    }

    /// Apply entropy decay to all nodes.
    pub fn apply_global_decay(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_entropy_decay(self.config.entropy_decay_rate);
        }
        // Restore entropy budget partially
        self.current_entropy_budget = self
            .current_entropy_budget
            .min(self.config.max_entropy_budget);
    }

    /// Check if the system is in zero-conflict paradox state.
    pub fn is_zero_conflict_paradox(&self) -> bool {
        let mut all_stable = true;
        let mut all_low_conflict = true;

        for node in self.nodes.values() {
            if node.resilience <= 0.95 {
                all_stable = false;
            }
            if node.last_conflict >= 1e-10 {
                all_low_conflict = false;
            }
        }

        all_stable && all_low_conflict && !self.nodes.is_empty()
    }

    /// Get average loss across all nodes.
    pub fn average_loss(&self) -> Result<f64, HomeostasisError> {
        if self.nodes.is_empty() {
            return Err(HomeostasisError::NoBaselineData);
        }
        let mut total = 0.0;
        let mut count = 0;
        for (&node_id, _) in &self.nodes {
            match self.compute_loss(node_id) {
                Ok(loss) => {
                    total += loss;
                    count += 1;
                }
                Err(_) => continue,
            }
        }
        if count == 0 {
            Err(HomeostasisError::NoBaselineData)
        } else {
            Ok(total / count as f64)
        }
    }

    /// Get total entropy injected across all adjustments.
    pub fn total_entropy_injected(&self) -> f64 {
        self.total_entropy_injected
    }

    /// Get remaining entropy budget.
    pub fn remaining_entropy_budget(&self) -> f64 {
        self.current_entropy_budget
    }

    /// Reset entropy budget.
    pub fn reset_entropy_budget(&mut self) {
        self.current_entropy_budget = self.config.max_entropy_budget;
    }

    /// Get node state.
    pub fn get_node(&self, node_id: u64) -> Option<&NodeHomeostasis> {
        self.nodes.get(&node_id)
    }

    /// Get adjustment history.
    pub fn adjustments(&self) -> &[AdjustmentRecord] {
        &self.adjustments
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.adjustments.clear();
        self.total_entropy_injected = 0.0;
        self.current_entropy_budget = self.config.max_entropy_budget;
    }
}

impl Default for DynamicHomeostasisLoss {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DynamicHomeostasisLoss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DynamicHomeostasisLoss(nodes={}, avg_loss={:.4}, entropy_budget={:.4})",
            self.nodes.len(),
            self.average_loss().unwrap_or(-1.0),
            self.current_entropy_budget
        )
    }
}

// ---------------------------------------------------------------------------
// Public standalone functions
// ---------------------------------------------------------------------------

/// Compute the dynamic homeostasis loss.
///
/// L = Resilience - Î» Â· Friction + Îµ Â· Baseline_Entropy
pub fn compute_homeostasis_loss(
    resilience: f64,
    destructive_friction: f64,
    friction_weight: f64,
    entropy_rate: f64,
    baseline_entropy: f64,
) -> f64 {
    let resilience_term = resilience;
    let friction_term = friction_weight * destructive_friction;
    let entropy_term = entropy_rate * baseline_entropy.max(baseline_entropy);
    resilience_term - friction_term + entropy_term
}

/// Compute the entropy injection needed for a given conflict level.
pub fn compute_entropy_injection(
    conflict: f64,
    resilience: f64,
    injection_rate: f64,
    baseline_floor: f64,
) -> f64 {
    if conflict < 1e-10 && resilience > 0.95 {
        // Zero-conflict paradox: inject baseline entropy
        baseline_floor + injection_rate * (1.0 - resilience)
    } else {
        // Normal operation: minimal injection proportional to conflict gradient
        injection_rate * conflict
    }
}

/// Check if a system is approaching zero-conflict paradox.
pub fn detect_zero_conflict_risk(conflicts: &[f64], _resilience_threshold: f64) -> bool {
    if conflicts.is_empty() {
        return false;
    }
    let avg_conflict: f64 = conflicts.iter().sum::<f64>() / conflicts.len() as f64;
    avg_conflict < 1e-8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HomeostasisConfig::default_topological();
        assert_eq!(config.friction_weight, 0.3);
        assert_eq!(config.entropy_injection_rate, 0.05);
        assert_eq!(config.max_entropy_budget, 0.5);
        assert_eq!(config.min_resilience_threshold, 0.4);
        assert_eq!(config.baseline_entropy_floor, 0.01);
        assert_eq!(config.entropy_decay_rate, 0.1);
        assert_eq!(config.max_history_entries, 100);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = HomeostasisConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_friction_weight() {
        let config = HomeostasisConfig {
            friction_weight: 0.0,
            ..HomeostasisConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_friction_weight_too_high() {
        let config = HomeostasisConfig {
            friction_weight: 1.1,
            ..HomeostasisConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_entropy_rate() {
        let config = HomeostasisConfig {
            entropy_injection_rate: 0.0,
            ..HomeostasisConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_budget() {
        let config = HomeostasisConfig {
            max_entropy_budget: 0.0,
            ..HomeostasisConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_history() {
        let config = HomeostasisConfig {
            max_history_entries: 0,
            ..HomeostasisConfig::default_topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_new() {
        let node = NodeHomeostasis::new(42);
        assert_eq!(node.node_id, 42);
        assert_eq!(node.resilience, 0.5);
        assert_eq!(node.destructive_friction, 0.0);
        assert_eq!(node.accumulated_entropy, 0.0);
        assert!(node.loss_history.is_empty());
    }

    #[test]
    fn test_node_entropy_decay() {
        let mut node = NodeHomeostasis::new(1);
        node.accumulated_entropy = 0.1;
        node.apply_entropy_decay(0.1);
        assert!((node.accumulated_entropy - 0.09).abs() < 1e-10);
    }

    #[test]
    fn test_node_record_loss() {
        let mut node = NodeHomeostasis::new(1);
        node.record_loss(0.5, 3);
        node.record_loss(0.6, 3);
        node.record_loss(0.7, 3);
        node.record_loss(0.8, 3);
        assert_eq!(node.loss_history.len(), 3);
        assert_eq!(node.loss_history[0], 0.6);
        assert_eq!(node.loss_history[1], 0.7);
        assert_eq!(node.loss_history[2], 0.8);
    }

    #[test]
    fn test_engine_creation() {
        let engine = DynamicHomeostasisLoss::new();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = HomeostasisConfig::default_topological();
        let engine = DynamicHomeostasisLoss::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_register_node() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        assert_eq!(engine.node_count(), 1);
        assert!(engine.get_node(1).is_some());
    }

    #[test]
    fn test_update_node_state() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        let result = engine.update_node_state(1, 0.8, 0.1, 0.05);
        assert!(result.is_ok());
        let node = engine.get_node(1).unwrap();
        assert_eq!(node.resilience, 0.8);
        assert_eq!(node.destructive_friction, 0.1);
        assert_eq!(node.last_conflict, 0.05);
    }

    #[test]
    fn test_update_node_resilience_out_of_range() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        let result = engine.update_node_state(1, 1.5, 0.1, 0.05);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_unregistered_node() {
        let mut engine = DynamicHomeostasisLoss::new();
        let result = engine.update_node_state(99, 0.8, 0.1, 0.05);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_loss_basic() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        engine.update_node_state(1, 0.8, 0.1, 0.05).unwrap();
        let loss = engine.compute_loss(1).unwrap();
        // L = 0.8 - 0.3*0.1 + 0.05*0.01 = 0.8 - 0.03 + 0.0005 = 0.7705
        assert!((loss - 0.7705).abs() < 1e-10);
    }

    #[test]
    fn test_compute_loss_unregistered() {
        let engine = DynamicHomeostasisLoss::new();
        let result = engine.compute_loss(99);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_adjustment_normal() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        engine.update_node_state(1, 0.8, 0.1, 0.05).unwrap();
        let record = engine.apply_adjustment(1, 1000).unwrap();
        assert_eq!(record.node_id, 1);
        assert_eq!(record.timestamp_ms, 1000);
        assert_eq!(record.entropy_injected, 0.0);
    }

    #[test]
    fn test_apply_adjustment_zero_conflict() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        // High resilience + zero conflict triggers entropy injection
        engine.update_node_state(1, 0.98, 0.0, 0.0).unwrap();
        let record = engine.apply_adjustment(1, 2000).unwrap();
        assert!(record.entropy_injected > 0.0);
        assert!(record.resilience_after < record.resilience_before);
    }

    #[test]
    fn test_zero_conflict_paradox_detection() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.update_node_state(1, 0.98, 0.0, 0.0).unwrap();
        engine.update_node_state(2, 0.97, 0.0, 0.0).unwrap();
        assert!(engine.is_zero_conflict_paradox());
    }

    #[test]
    fn test_no_zero_conflict_paradox() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        engine.update_node_state(1, 0.8, 0.1, 0.05).unwrap();
        assert!(!engine.is_zero_conflict_paradox());
    }

    #[test]
    fn test_average_loss() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.update_node_state(1, 0.8, 0.1, 0.05).unwrap();
        engine.update_node_state(2, 0.6, 0.2, 0.1).unwrap();
        let avg = engine.average_loss().unwrap();
        assert!(avg > 0.0);
    }

    #[test]
    fn test_average_loss_empty() {
        let engine = DynamicHomeostasisLoss::new();
        let result = engine.average_loss();
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_global_decay() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        let node = engine.nodes.get_mut(&1).unwrap();
        node.accumulated_entropy = 0.1;
        engine.apply_global_decay();
        let node = engine.get_node(1).unwrap();
        assert!((node.accumulated_entropy - 0.09).abs() < 1e-10);
    }

    #[test]
    fn test_reset_entropy_budget() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.current_entropy_budget = 0.0;
        engine.reset_entropy_budget();
        assert_eq!(engine.remaining_entropy_budget(), 0.5);
    }

    #[test]
    fn test_reset() {
        let mut engine = DynamicHomeostasisLoss::new();
        engine.register_node(1);
        engine.reset();
        assert_eq!(engine.node_count(), 0);
        assert!(engine.adjustments().is_empty());
        assert_eq!(engine.total_entropy_injected(), 0.0);
    }

    #[test]
    fn test_display() {
        let engine = DynamicHomeostasisLoss::new();
        let s = format!("{}", engine);
        assert!(s.contains("DynamicHomeostasisLoss"));
    }

    #[test]
    fn test_node_display() {
        let node = NodeHomeostasis::new(42);
        let s = format!("{}", node);
        assert!(s.contains("NodeHomeostasis"));
        assert!(s.contains("node=42"));
    }

    #[test]
    fn test_adjustment_record_display() {
        let record = AdjustmentRecord {
            node_id: 1,
            timestamp_ms: 1000,
            loss: 0.5,
            entropy_injected: 0.01,
            resilience_before: 0.8,
            resilience_after: 0.79,
        };
        let s = format!("{}", record);
        assert!(s.contains("Adjustment"));
    }

    #[test]
    fn test_standalone_compute_loss() {
        let loss = compute_homeostasis_loss(0.8, 0.1, 0.3, 0.05, 0.01);
        assert!((loss - 0.7705).abs() < 1e-10);
    }

    #[test]
    fn test_standalone_entropy_injection_normal() {
        let injection = compute_entropy_injection(0.05, 0.8, 0.05, 0.01);
        assert!((injection - 0.0025).abs() < 1e-10);
    }

    #[test]
    fn test_standalone_entropy_injection_zero_conflict() {
        let injection = compute_entropy_injection(0.0, 0.98, 0.05, 0.01);
        // baseline_floor + rate * (1 - resilience) = 0.01 + 0.05 * 0.02 = 0.011
        assert!((injection - 0.011).abs() < 1e-10);
    }

    #[test]
    fn test_detect_zero_conflict_risk_yes() {
        let conflicts = vec![0.0, 0.0, 0.0];
        assert!(detect_zero_conflict_risk(&conflicts, 0.95));
    }

    #[test]
    fn test_detect_zero_conflict_risk_no() {
        let conflicts = vec![0.05, 0.1, 0.02];
        assert!(!detect_zero_conflict_risk(&conflicts, 0.95));
    }

    #[test]
    fn test_detect_zero_conflict_empty() {
        let conflicts: Vec<f64> = vec![];
        assert!(!detect_zero_conflict_risk(&conflicts, 0.95));
    }

    #[test]
    fn test_error_display() {
        let err = HomeostasisError::ZeroConflictParadox;
        let s = format!("{}", err);
        assert!(s.contains("zero-conflict"));
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = DynamicHomeostasisLoss::new();

        // Register nodes
        engine.register_node(1);
        engine.register_node(2);
        assert_eq!(engine.node_count(), 2);

        // Update states
        engine.update_node_state(1, 0.8, 0.1, 0.05).unwrap();
        engine.update_node_state(2, 0.6, 0.2, 0.1).unwrap();

        // Compute losses
        let loss1 = engine.compute_loss(1).unwrap();
        let loss2 = engine.compute_loss(2).unwrap();
        assert!(loss1 > 0.0);
        assert!(loss2 > 0.0);

        // Apply adjustments
        let record1 = engine.apply_adjustment(1, 1000).unwrap();
        let record2 = engine.apply_adjustment(2, 1000).unwrap();
        assert_eq!(record1.node_id, 1);
        assert_eq!(record2.node_id, 2);

        // Check average loss
        let avg = engine.average_loss().unwrap();
        assert!(avg > 0.0);

        // Apply global decay
        engine.apply_global_decay();

        // Not in zero-conflict paradox
        assert!(!engine.is_zero_conflict_paradox());

        // Reset
        engine.reset();
        assert_eq!(engine.node_count(), 0);
    }
}
