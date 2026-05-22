//! Circuit Optimizer — Adaptive circuit selection and optimization for ZKP generation.
//!
//! Selects the optimal circuit type based on statement complexity, available resources,
//! and historical performance data. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use crate::zkp::async_zkp_v5::{CircuitType, ZKPStatement};
use std::collections::HashMap;

/// Performance profile for a circuit type.
#[derive(Debug, Clone)]
pub struct CircuitProfile {
    /// Average generation time in milliseconds.
    pub avg_gen_time_ms: f64,
    /// Average verification time in milliseconds.
    pub avg_verify_time_ms: f64,
    /// Total proofs generated with this circuit.
    pub total_proofs: u64,
    /// Success rate (0.0 - 1.0).
    pub success_rate: f64,
    /// Complexity range this circuit handles best.
    pub optimal_complexity_range: (f64, f64),
}

impl CircuitProfile {
    pub fn new(circuit: &CircuitType) -> Self {
        let (low, high) = match circuit {
            CircuitType::Membership => (0.0, 0.3),
            CircuitType::RangeProof => (0.2, 0.5),
            CircuitType::Commitment => (0.4, 0.7),
            CircuitType::CrossPoolAggregation => (0.6, 0.85),
            CircuitType::IncrementalAccumulator => (0.7, 0.95),
            CircuitType::Custom => (0.0, 1.0),
        };
        Self {
            avg_gen_time_ms: 0.0,
            avg_verify_time_ms: 0.0,
            total_proofs: 0,
            success_rate: 1.0,
            optimal_complexity_range: (low, high),
        }
    }

    /// Update profile with new proof data.
    pub fn update(&mut self, gen_time_ms: f64, verify_time_ms: f64, success: bool) {
        self.total_proofs += 1;
        self.avg_gen_time_ms = (self.avg_gen_time_ms * (self.total_proofs - 1) as f64
            + gen_time_ms)
            / self.total_proofs as f64;
        self.avg_verify_time_ms = (self.avg_verify_time_ms * (self.total_proofs - 1) as f64
            + verify_time_ms)
            / self.total_proofs as f64;
        if !success {
            self.success_rate =
                (self.success_rate * (self.total_proofs - 1) as f64) / self.total_proofs as f64;
        }
    }

    /// Score for this circuit given a complexity target. Higher is better.
    pub fn score_for(&self, complexity: f64) -> f64 {
        let (low, high) = self.optimal_complexity_range;
        let in_range = if complexity >= low && complexity <= high {
            1.0
        } else {
            let dist = if complexity < low {
                low - complexity
            } else {
                complexity - high
            };
            (1.0 - dist.clamp(0.0, 1.0)).max(0.1)
        };
        in_range * self.success_rate * (1.0 / (1.0 + self.avg_gen_time_ms / 1000.0))
    }
}

/// Configuration for the circuit optimizer.
#[derive(Debug)]
pub struct CircuitOptimizerConfig {
    /// Enable adaptive selection.
    pub adaptive_enabled: bool,
    /// Minimum proofs before adaptive kicks in.
    pub min_samples: u64,
    /// Fallback circuit when no data available.
    pub default_circuit: CircuitType,
}

impl Default for CircuitOptimizerConfig {
    fn default() -> Self {
        Self {
            adaptive_enabled: true,
            min_samples: 10,
            default_circuit: CircuitType::Membership,
        }
    }
}

/// Circuit Optimizer — selects optimal circuit based on complexity and profiles.
#[cfg(feature = "v1.4-sprint1")]
pub struct CircuitOptimizer {
    config: CircuitOptimizerConfig,
    profiles: HashMap<CircuitType, CircuitProfile>,
}

#[cfg(feature = "v1.4-sprint1")]
impl CircuitOptimizer {
    pub fn new(config: CircuitOptimizerConfig) -> Self {
        let mut profiles = HashMap::new();
        let circuits = [
            CircuitType::Membership,
            CircuitType::RangeProof,
            CircuitType::Commitment,
            CircuitType::CrossPoolAggregation,
            CircuitType::IncrementalAccumulator,
            CircuitType::Custom,
        ];
        for &circuit in &circuits {
            profiles.insert(circuit, CircuitProfile::new(&circuit));
        }
        Self { config, profiles }
    }

    /// Select the best circuit for the given statement.
    pub fn select_circuit(&self, statement: &ZKPStatement) -> CircuitType {
        if !self.config.adaptive_enabled {
            return self.config.default_circuit;
        }

        let complexity = statement.complexity_score;
        let mut best_circuit = self.config.default_circuit;
        let mut best_score = 0.0;

        for (circuit, profile) in &self.profiles {
            if profile.total_proofs < self.config.min_samples {
                continue;
            }
            let score = profile.score_for(complexity);
            if score > best_score {
                best_score = score;
                best_circuit = *circuit;
            }
        }

        best_circuit
    }

    /// Record proof result to update profiles.
    pub fn record_result(
        &mut self,
        circuit: &CircuitType,
        gen_time_ms: f64,
        verify_time_ms: f64,
        success: bool,
    ) {
        if let Some(profile) = self.profiles.get_mut(circuit) {
            profile.update(gen_time_ms, verify_time_ms, success);
        }
    }

    /// Get profile for a circuit type.
    pub fn get_profile(&self, circuit: &CircuitType) -> Option<&CircuitProfile> {
        self.profiles.get(circuit)
    }

    /// Get all profiles.
    pub fn all_profiles(&self) -> &HashMap<CircuitType, CircuitProfile> {
        &self.profiles
    }

    /// Reset all profiles.
    pub fn reset(&mut self) {
        let circuits: Vec<CircuitType> = self.profiles.keys().cloned().collect();
        for circuit in circuits {
            self.profiles.insert(circuit, CircuitProfile::new(&circuit));
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for CircuitOptimizer {
    fn default() -> Self {
        Self::new(CircuitOptimizerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_statement(complexity: f64) -> ZKPStatement {
        ZKPStatement {
            statement_id: "test".to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: "hash".to_string(),
            circuit_type: CircuitType::Membership,
            source_pool: "pool-1".to_string(),
            priority: 1,
            complexity_score: complexity,
        }
    }

    #[test]
    fn test_profile_creation() {
        let profile = CircuitProfile::new(&CircuitType::Membership);
        assert_eq!(profile.total_proofs, 0);
        assert_eq!(profile.success_rate, 1.0);
        assert_eq!(profile.optimal_complexity_range, (0.0, 0.3));
    }

    #[test]
    fn test_profile_update() {
        let mut profile = CircuitProfile::new(&CircuitType::Membership);
        profile.update(100.0, 10.0, true);
        assert_eq!(profile.total_proofs, 1);
        assert_eq!(profile.avg_gen_time_ms, 100.0);
        assert_eq!(profile.success_rate, 1.0);
    }

    #[test]
    fn test_profile_update_failure() {
        let mut profile = CircuitProfile::new(&CircuitType::Membership);
        profile.update(100.0, 10.0, true);
        profile.update(200.0, 20.0, false);
        assert_eq!(profile.total_proofs, 2);
        assert!(profile.success_rate < 1.0);
    }

    #[test]
    fn test_profile_score_in_range() {
        let profile = CircuitProfile::new(&CircuitType::Membership);
        // Membership range is 0.0-0.3
        let score = profile.score_for(0.15);
        assert!(score > 0.0);
    }

    #[test]
    fn test_optimizer_creation() {
        let optimizer = CircuitOptimizer::default();
        assert_eq!(optimizer.all_profiles().len(), 6);
    }

    #[test]
    fn test_optimizer_select_default() {
        let config = CircuitOptimizerConfig {
            adaptive_enabled: false,
            ..Default::default()
        };
        let optimizer = CircuitOptimizer::new(config);
        let stmt = make_statement(0.5);
        let circuit = optimizer.select_circuit(&stmt);
        assert_eq!(circuit, CircuitType::Membership);
    }

    #[test]
    fn test_optimizer_record_result() {
        let mut optimizer = CircuitOptimizer::default();
        optimizer.record_result(&CircuitType::Membership, 100.0, 10.0, true);
        let profile = optimizer.get_profile(&CircuitType::Membership).unwrap();
        assert_eq!(profile.total_proofs, 1);
    }

    #[test]
    fn test_optimizer_reset() {
        let mut optimizer = CircuitOptimizer::default();
        optimizer.record_result(&CircuitType::Membership, 100.0, 10.0, true);
        optimizer.reset();
        let profile = optimizer.get_profile(&CircuitType::Membership).unwrap();
        assert_eq!(profile.total_proofs, 0);
    }

    #[test]
    fn test_config_default() {
        let config = CircuitOptimizerConfig::default();
        assert!(config.adaptive_enabled);
        assert_eq!(config.min_samples, 10);
    }

    #[test]
    fn test_all_circuit_ranges() {
        let m = CircuitProfile::new(&CircuitType::Membership);
        assert_eq!(m.optimal_complexity_range, (0.0, 0.3));

        let r = CircuitProfile::new(&CircuitType::RangeProof);
        assert_eq!(r.optimal_complexity_range, (0.2, 0.5));

        let c = CircuitProfile::new(&CircuitType::Commitment);
        assert_eq!(c.optimal_complexity_range, (0.4, 0.7));

        let a = CircuitProfile::new(&CircuitType::CrossPoolAggregation);
        assert_eq!(a.optimal_complexity_range, (0.6, 0.85));

        let i = CircuitProfile::new(&CircuitType::IncrementalAccumulator);
        assert_eq!(i.optimal_complexity_range, (0.7, 0.95));

        let cu = CircuitProfile::new(&CircuitType::Custom);
        assert_eq!(cu.optimal_complexity_range, (0.0, 1.0));
    }
}
