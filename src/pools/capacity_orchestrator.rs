//! Capacity Orchestrator v4 — Capacity orchestration with demand prediction and static fallback.
//!
//! Coordinates pool allocation, routing decisions, and capacity planning
//! across multiple chains with predictive scaling and confidence-based fallback.
//!
//! Feature-gated with `v1.5-sprint1`.

#[cfg(feature = "v1.5-sprint1")]
mod internal {

    use std::collections::{HashMap, VecDeque};
    use std::fmt;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub enum OrchestratorError {
        PoolNotFound(String),
        RouteNotFound(String),
        ConfidenceTooLow(f64),
        CapacityExceeded(f64),
        InvalidParameter(String),
        FallbackActivated(String),
    }

    impl fmt::Display for OrchestratorError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                OrchestratorError::PoolNotFound(id) => {
                    write!(f, "Pool not found: {}", id)
                }
                OrchestratorError::RouteNotFound(id) => {
                    write!(f, "Route not found: {}", id)
                }
                OrchestratorError::ConfidenceTooLow(conf) => {
                    write!(f, "Confidence too low: {:.3}", conf)
                }
                OrchestratorError::CapacityExceeded(cap) => {
                    write!(f, "Capacity exceeded: {:.1}", cap)
                }
                OrchestratorError::InvalidParameter(msg) => {
                    write!(f, "Invalid parameter: {}", msg)
                }
                OrchestratorError::FallbackActivated(reason) => {
                    write!(f, "Fallback activated: {}", reason)
                }
            }
        }
    }

    impl std::error::Error for OrchestratorError {}

    // ---------------------------------------------------------------------------
    // Configuration
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct OrchestratorConfig {
        pub max_pools: usize,
        pub max_routes: usize,
        pub min_confidence: f64,
        pub demand_prediction_horizon: usize,
        pub prediction_alpha: f64,
        pub fallback_to_static: bool,
        pub capacity_buffer: f64,
        pub scaling_threshold_high: f64,
        pub scaling_threshold_low: f64,
    }

    impl Default for OrchestratorConfig {
        fn default() -> Self {
            Self {
                max_pools: 50,
                max_routes: 200,
                min_confidence: 0.0,
                demand_prediction_horizon: 10,
                prediction_alpha: 0.3,
                fallback_to_static: true,
                capacity_buffer: 0.15,
                scaling_threshold_high: 0.85,
                scaling_threshold_low: 0.3,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Core Structures
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct OrchestratorState {
        pub pool_id: String,
        pub total_capacity: f64,
        pub used_capacity: f64,
        pub utilization: f64,
        pub demand_history: VecDeque<f64>,
        pub predicted_demand: f64,
        pub ema_demand: f64,
        pub confidence: f64,
        pub active: bool,
    }

    impl OrchestratorState {
        pub fn new(pool_id: String, total_capacity: f64) -> Self {
            let mut demand_history = VecDeque::new();
            for _ in 0..10 {
                demand_history.push_back(0.0);
            }
            Self {
                pool_id,
                total_capacity,
                used_capacity: 0.0,
                utilization: 0.0,
                demand_history,
                predicted_demand: 0.0,
                ema_demand: 0.0,
                confidence: 0.0,
                active: true,
            }
        }

        pub fn available_capacity(&self) -> f64 {
            self.total_capacity - self.used_capacity
        }

        pub fn update_utilization(&mut self) {
            if self.total_capacity > 0.0 {
                self.utilization = self.used_capacity / self.total_capacity;
            } else {
                self.utilization = 0.0;
            }
        }

        pub fn record_demand(&mut self, demand: f64, alpha: f64) {
            self.demand_history.push_back(demand);
            if self.demand_history.len() > 50 {
                self.demand_history.pop_front();
            }
            // Update EMA demand
            if self.ema_demand == 0.0 {
                self.ema_demand = demand;
            } else {
                self.ema_demand = alpha * demand + (1.0 - alpha) * self.ema_demand;
            }
            // Update confidence based on sample depth
            let samples = self.demand_history.len() as f64;
            self.confidence = (samples / 50.0).min(1.0);
        }

        pub fn predict_demand(&self, horizon: usize, alpha: f64) -> f64 {
            if self.demand_history.is_empty() {
                return self.ema_demand;
            }
            let recent: Vec<f64> = self.demand_history.iter().cloned().collect();
            let len = recent.len().min(horizon);
            if len == 0 {
                return self.ema_demand;
            }
            // Weighted average with exponential decay
            let mut weighted_sum = 0.0;
            let mut weight_total = 0.0;
            for (i, &val) in recent.iter().rev().take(horizon).enumerate() {
                let weight = alpha.powi(i as i32);
                weighted_sum += val * weight;
                weight_total += weight;
            }
            if weight_total > 0.0 {
                weighted_sum / weight_total
            } else {
                self.ema_demand
            }
        }

        pub fn needs_scaling(&self, threshold_high: f64, threshold_low: f64) -> ScalingAction {
            if self.utilization >= threshold_high {
                ScalingAction::ScaleUp
            } else if self.utilization <= threshold_low && self.used_capacity > 0.0 {
                ScalingAction::ScaleDown
            } else {
                ScalingAction::NoOp
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ScalingAction {
        ScaleUp,
        ScaleDown,
        NoOp,
    }

    impl fmt::Display for ScalingAction {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ScalingAction::ScaleUp => write!(f, "ScaleUp"),
                ScalingAction::ScaleDown => write!(f, "ScaleDown"),
                ScalingAction::NoOp => write!(f, "NoOp"),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct RoutingDecision {
        pub target_pool: String,
        pub confidence: f64,
        pub estimated_capacity: f64,
        pub used_fallback: bool,
        pub scaling_action: ScalingAction,
    }

    #[derive(Debug, Clone)]
    pub struct OrchestratorStats {
        pub total_decisions: u64,
        pub fallback_count: u64,
        pub scale_up_count: u64,
        pub scale_down_count: u64,
        pub avg_confidence: f64,
        pub total_demand_predicted: u64,
    }

    impl Default for OrchestratorStats {
        fn default() -> Self {
            Self {
                total_decisions: 0,
                fallback_count: 0,
                scale_up_count: 0,
                scale_down_count: 0,
                avg_confidence: 0.0,
                total_demand_predicted: 0,
            }
        }
    }

    impl OrchestratorStats {
        pub fn record_decision(
            &mut self,
            confidence: f64,
            used_fallback: bool,
            action: &ScalingAction,
        ) {
            self.total_decisions += 1;
            if used_fallback {
                self.fallback_count += 1;
            }
            match action {
                ScalingAction::ScaleUp => self.scale_up_count += 1,
                ScalingAction::ScaleDown => self.scale_down_count += 1,
                ScalingAction::NoOp => {}
            }
            // Update average confidence
            let n = self.total_decisions as f64;
            self.avg_confidence = self.avg_confidence + (confidence - self.avg_confidence) / n;
        }

        pub fn record_prediction(&mut self) {
            self.total_demand_predicted += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Orchestrator
    // ---------------------------------------------------------------------------

    pub struct CapacityOrchestrator {
        config: OrchestratorConfig,
        states: HashMap<String, OrchestratorState>,
        static_fallback_pools: Vec<String>,
        stats: OrchestratorStats,
    }

    impl CapacityOrchestrator {
        pub fn new(config: OrchestratorConfig) -> Self {
            Self {
                config,
                states: HashMap::new(),
                static_fallback_pools: Vec::new(),
                stats: OrchestratorStats::default(),
            }
        }

        pub fn register_pool(
            &mut self,
            pool_id: String,
            capacity: f64,
        ) -> Result<(), OrchestratorError> {
            if self.states.len() >= self.config.max_pools {
                return Err(OrchestratorError::CapacityExceeded(self.states.len() as f64));
            }
            if capacity <= 0.0 {
                return Err(OrchestratorError::InvalidParameter(
                    "Capacity must be positive".to_string(),
                ));
            }
            let state = OrchestratorState::new(pool_id.clone(), capacity);
            self.states.insert(pool_id, state);
            Ok(())
        }

        pub fn remove_pool(&mut self, pool_id: &str) -> Result<(), OrchestratorError> {
            if self.states.remove(pool_id).is_none() {
                return Err(OrchestratorError::PoolNotFound(pool_id.to_string()));
            }
            Ok(())
        }

        pub fn update_demand(
            &mut self,
            pool_id: &str,
            demand: f64,
        ) -> Result<(), OrchestratorError> {
            let state = self
                .states
                .get_mut(pool_id)
                .ok_or_else(|| OrchestratorError::PoolNotFound(pool_id.to_string()))?;
            state.record_demand(demand, self.config.prediction_alpha);
            state.used_capacity = demand;
            state.update_utilization();
            Ok(())
        }

        pub fn predict_demand_for_pool(&mut self, pool_id: &str) -> Result<f64, OrchestratorError> {
            let predicted = {
                let state = self
                    .states
                    .get(pool_id)
                    .ok_or_else(|| OrchestratorError::PoolNotFound(pool_id.to_string()))?;
                state.predict_demand(
                    self.config.demand_prediction_horizon,
                    self.config.prediction_alpha,
                )
            };
            self.stats.record_prediction();
            Ok(predicted)
        }

        pub fn decide(&self, required_capacity: f64) -> Result<RoutingDecision, OrchestratorError> {
            if self.states.is_empty() {
                return Err(OrchestratorError::PoolNotFound(
                    "No pools registered".to_string(),
                ));
            }

            // Find best pool by available capacity and confidence
            let mut best_state: Option<&OrchestratorState> = None;
            let mut best_score = f64::NEG_INFINITY;

            for state in self.states.values() {
                if !state.active {
                    continue;
                }
                let available = state.available_capacity();
                if available < required_capacity {
                    continue;
                }
                // Score based on available capacity and confidence
                let score = available * state.confidence;
                if score > best_score {
                    best_score = score;
                    best_state = Some(state);
                }
            }

            match best_state {
                Some(state) => {
                    let confidence = state.confidence;
                    if confidence < self.config.min_confidence {
                        if self.config.fallback_to_static {
                            // Fallback to static pool
                            if let Some(fallback_id) = self.static_fallback_pools.first() {
                                if let Some(fallback_state) = self.states.get(fallback_id) {
                                    let action = fallback_state.needs_scaling(
                                        self.config.scaling_threshold_high,
                                        self.config.scaling_threshold_low,
                                    );
                                    return Ok(RoutingDecision {
                                        target_pool: fallback_id.clone(),
                                        confidence: fallback_state.confidence,
                                        estimated_capacity: fallback_state.available_capacity(),
                                        used_fallback: true,
                                        scaling_action: action,
                                    });
                                }
                            }
                            // No fallback available, return error
                            return Err(OrchestratorError::FallbackActivated(format!(
                                "No static fallback available, confidence: {:.3}",
                                confidence
                            )));
                        } else {
                            // Fallback disabled — return error
                            return Err(OrchestratorError::ConfidenceTooLow(confidence));
                        }
                    }
                    let action = state.needs_scaling(
                        self.config.scaling_threshold_high,
                        self.config.scaling_threshold_low,
                    );
                    Ok(RoutingDecision {
                        target_pool: state.pool_id.clone(),
                        confidence: state.confidence,
                        estimated_capacity: state.available_capacity(),
                        used_fallback: false,
                        scaling_action: action,
                    })
                }
                None => Err(OrchestratorError::CapacityExceeded(required_capacity)),
            }
        }

        pub fn set_static_fallback(&mut self, pool_ids: Vec<String>) {
            self.static_fallback_pools = pool_ids;
        }

        pub fn deactivate_pool(&mut self, pool_id: &str) -> Result<(), OrchestratorError> {
            let state = self
                .states
                .get_mut(pool_id)
                .ok_or_else(|| OrchestratorError::PoolNotFound(pool_id.to_string()))?;
            state.active = false;
            Ok(())
        }

        pub fn activate_pool(&mut self, pool_id: &str) -> Result<(), OrchestratorError> {
            let state = self
                .states
                .get_mut(pool_id)
                .ok_or_else(|| OrchestratorError::PoolNotFound(pool_id.to_string()))?;
            state.active = true;
            Ok(())
        }

        pub fn get_state(&self, pool_id: &str) -> Option<&OrchestratorState> {
            self.states.get(pool_id)
        }

        pub fn get_all_states(&self) -> &HashMap<String, OrchestratorState> {
            &self.states
        }

        pub fn record_decision(
            &mut self,
            confidence: f64,
            used_fallback: bool,
            action: &ScalingAction,
        ) {
            self.stats
                .record_decision(confidence, used_fallback, action);
        }

        pub fn get_stats(&self) -> &OrchestratorStats {
            &self.stats
        }

        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        pub fn pool_count(&self) -> usize {
            self.states.len()
        }

        pub fn active_pool_count(&self) -> usize {
            self.states.values().filter(|s| s.active).count()
        }
    }

    impl Default for CapacityOrchestrator {
        fn default() -> Self {
            Self::new(OrchestratorConfig::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> OrchestratorConfig {
            OrchestratorConfig {
                max_pools: 10,
                max_routes: 50,
                min_confidence: 0.5,
                demand_prediction_horizon: 5,
                prediction_alpha: 0.3,
                fallback_to_static: true,
                capacity_buffer: 0.15,
                scaling_threshold_high: 0.8,
                scaling_threshold_low: 0.3,
            }
        }

        #[test]
        fn test_orchestrator_creation() {
            let orch = CapacityOrchestrator::default();
            assert_eq!(orch.pool_count(), 0);
        }

        #[test]
        fn test_orchestrator_with_config() {
            let config = make_config();
            let orch = CapacityOrchestrator::new(config);
            assert_eq!(orch.pool_count(), 0);
        }

        #[test]
        fn test_register_pool() {
            let mut orch = CapacityOrchestrator::default();
            let result = orch.register_pool("pool_1".to_string(), 100.0);
            assert!(result.is_ok());
            assert_eq!(orch.pool_count(), 1);
        }

        #[test]
        fn test_register_pool_invalid_capacity() {
            let mut orch = CapacityOrchestrator::default();
            let result = orch.register_pool("pool_1".to_string(), 0.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_register_pool_max_reached() {
            let mut orch = CapacityOrchestrator::new(make_config());
            for i in 0..10 {
                let pool_id = format!("pool_{}", i);
                let result = orch.register_pool(pool_id, 100.0);
                assert!(result.is_ok());
            }
            let result = orch.register_pool("pool_overflow".to_string(), 100.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_remove_pool() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            let result = orch.remove_pool("pool_1");
            assert!(result.is_ok());
            assert_eq!(orch.pool_count(), 0);
        }

        #[test]
        fn test_remove_pool_not_found() {
            let mut orch = CapacityOrchestrator::default();
            let result = orch.remove_pool("nonexistent");
            assert!(result.is_err());
        }

        #[test]
        fn test_update_demand() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            let result = orch.update_demand("pool_1", 50.0);
            assert!(result.is_ok());
            let state = orch.get_state("pool_1").unwrap();
            assert!((state.used_capacity - 50.0).abs() < 0.001);
            assert!((state.utilization - 0.5).abs() < 0.001);
        }

        #[test]
        fn test_update_demand_pool_not_found() {
            let mut orch = CapacityOrchestrator::default();
            let result = orch.update_demand("nonexistent", 50.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_predict_demand() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            orch.update_demand("pool_1", 50.0).unwrap();
            orch.update_demand("pool_1", 60.0).unwrap();
            orch.update_demand("pool_1", 55.0).unwrap();
            let predicted = orch.predict_demand_for_pool("pool_1").unwrap();
            assert!(predicted > 0.0);
        }

        #[test]
        fn test_predict_demand_pool_not_found() {
            let mut orch = CapacityOrchestrator::default();
            let result = orch.predict_demand_for_pool("nonexistent");
            assert!(result.is_err());
        }

        #[test]
        fn test_decide_basic() {
            let mut orch = CapacityOrchestrator::new(make_config());
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            // Build confidence
            for i in 0..20 {
                orch.update_demand("pool_1", 30.0 + (i as f64)).ok();
            }
            let decision = orch.decide(50.0).unwrap();
            assert_eq!(decision.target_pool, "pool_1");
            assert!(!decision.used_fallback);
        }

        #[test]
        fn test_decide_no_pools() {
            let orch = CapacityOrchestrator::default();
            let result = orch.decide(50.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_decide_insufficient_capacity() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            for _ in 0..20 {
                orch.update_demand("pool_1", 95.0).ok();
            }
            let result = orch.decide(50.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_decide_fallback() {
            let config = OrchestratorConfig {
                min_confidence: 0.9,
                fallback_to_static: true,
                ..make_config()
            };
            let mut orch = CapacityOrchestrator::new(config);
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            orch.register_pool("fallback_1".to_string(), 200.0).unwrap();
            orch.set_static_fallback(vec!["fallback_1".to_string()]);
            // Build some confidence on fallback
            for _ in 0..30 {
                orch.update_demand("fallback_1", 50.0).ok();
            }
            // pool_1 has low confidence, should fallback
            let decision = orch.decide(50.0).unwrap();
            assert!(decision.used_fallback);
            assert_eq!(decision.target_pool, "fallback_1");
        }

        #[test]
        fn test_decide_fallback_disabled() {
            let config = OrchestratorConfig {
                min_confidence: 0.9,
                fallback_to_static: false,
                ..make_config()
            };
            let mut orch = CapacityOrchestrator::new(config);
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            let result = orch.decide(50.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_deactivate_pool() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            let result = orch.deactivate_pool("pool_1");
            assert!(result.is_ok());
            let state = orch.get_state("pool_1").unwrap();
            assert!(!state.active);
        }

        #[test]
        fn test_activate_pool() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            orch.deactivate_pool("pool_1").unwrap();
            let result = orch.activate_pool("pool_1");
            assert!(result.is_ok());
            let state = orch.get_state("pool_1").unwrap();
            assert!(state.active);
        }

        #[test]
        fn test_scaling_action_scale_up() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            for _ in 0..30 {
                orch.update_demand("pool_1", 90.0).ok();
            }
            let state = orch.get_state("pool_1").unwrap();
            let action = state.needs_scaling(0.8, 0.3);
            assert_eq!(action, ScalingAction::ScaleUp);
        }

        #[test]
        fn test_scaling_action_scale_down() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            for _ in 0..30 {
                orch.update_demand("pool_1", 20.0).ok();
            }
            let state = orch.get_state("pool_1").unwrap();
            let action = state.needs_scaling(0.8, 0.3);
            assert_eq!(action, ScalingAction::ScaleDown);
        }

        #[test]
        fn test_scaling_action_no_op() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            for _ in 0..30 {
                orch.update_demand("pool_1", 50.0).ok();
            }
            let state = orch.get_state("pool_1").unwrap();
            let action = state.needs_scaling(0.8, 0.3);
            assert_eq!(action, ScalingAction::NoOp);
        }

        #[test]
        fn test_stats_recording() {
            let mut orch = CapacityOrchestrator::default();
            orch.record_decision(0.8, false, &ScalingAction::ScaleUp);
            orch.record_decision(0.9, true, &ScalingAction::ScaleDown);
            let stats = orch.get_stats();
            assert_eq!(stats.total_decisions, 2);
            assert_eq!(stats.fallback_count, 1);
            assert_eq!(stats.scale_up_count, 1);
            assert_eq!(stats.scale_down_count, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut orch = CapacityOrchestrator::default();
            orch.record_decision(0.8, false, &ScalingAction::NoOp);
            orch.reset_stats();
            let stats = orch.get_stats();
            assert_eq!(stats.total_decisions, 0);
        }

        #[test]
        fn test_orchestrator_state_new() {
            let state = OrchestratorState::new("pool_1".to_string(), 100.0);
            assert_eq!(state.pool_id, "pool_1");
            assert_eq!(state.total_capacity, 100.0);
            assert!(state.active);
        }

        #[test]
        fn test_available_capacity() {
            let mut state = OrchestratorState::new("pool_1".to_string(), 100.0);
            state.used_capacity = 40.0;
            assert!((state.available_capacity() - 60.0).abs() < 0.001);
        }

        #[test]
        fn test_confidence_builds() {
            let mut state = OrchestratorState::new("pool_1".to_string(), 100.0);
            assert!((state.confidence - 0.0).abs() < 0.001);
            for i in 0..25 {
                state.record_demand(50.0 + i as f64, 0.3);
            }
            assert!(state.confidence > 0.5);
        }

        #[test]
        fn test_ema_demand() {
            let mut state = OrchestratorState::new("pool_1".to_string(), 100.0);
            state.record_demand(100.0, 0.3);
            let first_ema = state.ema_demand;
            state.record_demand(120.0, 0.3);
            assert!(state.ema_demand > first_ema);
        }

        #[test]
        fn test_config_default() {
            let config = OrchestratorConfig::default();
            assert_eq!(config.max_pools, 50);
            assert_eq!(config.min_confidence, 0.0);
            assert!(config.fallback_to_static);
        }

        #[test]
        fn test_stats_default() {
            let stats = OrchestratorStats::default();
            assert_eq!(stats.total_decisions, 0);
            assert_eq!(stats.fallback_count, 0);
        }

        #[test]
        fn test_error_display() {
            let err = OrchestratorError::PoolNotFound("test".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("test"));
        }

        #[test]
        fn test_scaling_action_display() {
            let action = ScalingAction::ScaleUp;
            let msg = format!("{}", action);
            assert_eq!(msg, "ScaleUp");
        }

        #[test]
        fn test_active_pool_count() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            orch.register_pool("pool_2".to_string(), 100.0).unwrap();
            orch.deactivate_pool("pool_2").unwrap();
            assert_eq!(orch.active_pool_count(), 1);
        }

        #[test]
        fn test_get_all_states() {
            let mut orch = CapacityOrchestrator::default();
            orch.register_pool("pool_1".to_string(), 100.0).unwrap();
            let states = orch.get_all_states();
            assert_eq!(states.len(), 1);
        }

        #[test]
        fn test_orchestrator_default() {
            let orch = CapacityOrchestrator::default();
            assert_eq!(orch.pool_count(), 0);
        }
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;
