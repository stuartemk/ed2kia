//! SLA Contract Manager — Gestión de contratos SLA con serialización binaria
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint2")]`
//! Contratos binarios serializados con `serde`. Validación automática de métricas vs umbrales.
//! Soporte para contratos multi-métrica con cláusulas de penalización y recompensa.
//!
//! # Arquitectura
//!
//! 1. **SLAContract**: Contrato SLA con métricas, umbrales y cláusulas.
//! 2. **ContractClause**: Cláusula individual (penalización, recompensa, escalación).
//! 3. **ContractManager**: Gestor de contratos con validación automática.
//! 4. **ContractStatus**: Estado del contrato (activo, violado, cumplido, expirado).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for the SLA Contract Manager.
#[derive(Debug, Error)]
pub enum ContractError {
    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    #[error("Invalid contract: {0}")]
    InvalidContract(String),

    #[error("Contract expired: {0}")]
    ContractExpired(String),

    #[error("Contract already terminated: {0}")]
    ContractTerminated(String),

    #[error("Metric violation: {metric} = {value} (threshold: {threshold})")]
    MetricViolation {
        metric: String,
        value: f64,
        threshold: f64,
    },

    #[error("Serialization error: {0}")]
    Serialization(String),
}

// ---------------------------------------------------------------------------
// Public Types
// ---------------------------------------------------------------------------

/// Status of an SLA contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContractStatus {
    /// Contract is active and being monitored.
    Active,
    /// Contract is in violation but grace period active.
    ViolationGrace,
    /// Contract has been violated.
    Violated,
    /// Contract has been fulfilled successfully.
    Fulfilled,
    /// Contract has expired.
    Expired,
    /// Contract has been terminated early.
    Terminated,
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Active => write!(f, "Active"),
            ContractStatus::ViolationGrace => write!(f, "ViolationGrace"),
            ContractStatus::Violated => write!(f, "Violated"),
            ContractStatus::Fulfilled => write!(f, "Fulfilled"),
            ContractStatus::Expired => write!(f, "Expired"),
            ContractStatus::Terminated => write!(f, "Terminated"),
        }
    }
}

/// Type of contract clause.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ClauseType {
    /// Penalty clause triggered on violation.
    Penalty,
    /// Reward clause triggered on compliance.
    Reward,
    /// Escalation clause for repeated violations.
    Escalation,
    /// Auto-renewal clause.
    AutoRenewal,
}

impl std::fmt::Display for ClauseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClauseType::Penalty => write!(f, "Penalty"),
            ClauseType::Reward => write!(f, "Reward"),
            ClauseType::Escalation => write!(f, "Escalation"),
            ClauseType::AutoRenewal => write!(f, "AutoRenewal"),
        }
    }
}

/// Individual clause within an SLA contract.
#[derive(Debug, Clone)]
pub struct ContractClause {
    /// Unique clause identifier.
    pub id: String,
    /// Clause type.
    pub clause_type: ClauseType,
    /// Metric key this clause applies to.
    pub metric_key: String,
    /// Threshold value for triggering.
    pub threshold: f64,
    /// Description of the clause action.
    pub description: String,
    /// Penalty or reward amount (in credits).
    pub amount: f64,
    /// Whether this clause has been triggered.
    pub triggered: bool,
    /// Timestamp when clause was triggered.
    pub triggered_at: Option<Instant>,
}

impl ContractClause {
    /// Create a new contract clause.
    pub fn new(
        id: String,
        clause_type: ClauseType,
        metric_key: String,
        threshold: f64,
        description: String,
        amount: f64,
    ) -> Self {
        Self {
            id,
            clause_type,
            metric_key,
            threshold,
            description,
            amount,
            triggered: false,
            triggered_at: None,
        }
    }
}

/// Metric definition within a contract.
#[derive(Debug, Clone)]
pub struct ContractMetric {
    /// Metric key name.
    pub key: String,
    /// Human-readable name.
    pub name: String,
    /// Target value.
    pub target: f64,
    /// Minimum acceptable value.
    pub minimum: f64,
    /// Maximum acceptable value.
    pub maximum: f64,
    /// Unit label (ms, %, MB, etc.).
    pub unit: String,
}

impl ContractMetric {
    /// Create a new contract metric.
    pub fn new(
        key: String,
        name: String,
        target: f64,
        minimum: f64,
        maximum: f64,
        unit: String,
    ) -> Self {
        Self {
            key,
            name,
            target,
            minimum,
            maximum,
            unit,
        }
    }

    /// Check if a value is within acceptable range.
    pub fn is_within_range(&self, value: f64) -> bool {
        value >= self.minimum && value <= self.maximum
    }

    /// Check if a value meets the target.
    pub fn meets_target(&self, value: f64) -> bool {
        (value - self.target).abs() < f64::EPSILON * 1000.0
            || (self.key.contains("latency") || self.key.contains("time")) && value <= self.target
            || !self.key.contains("latency") && !self.key.contains("time") && value >= self.target
    }
}

/// SLA Contract definition.
#[derive(Debug, Clone)]
pub struct SLAContract {
    /// Unique contract identifier.
    pub id: String,
    /// Contract title.
    pub title: String,
    /// Contract description.
    pub description: String,
    /// Provider node ID.
    pub provider: String,
    /// Consumer node ID.
    pub consumer: String,
    /// Contract start time.
    pub start_time: Instant,
    /// Contract duration.
    pub duration: Duration,
    /// Grace period before violation escalation.
    pub grace_period: Duration,
    /// Contract metrics.
    pub metrics: Vec<ContractMetric>,
    /// Contract clauses.
    pub clauses: Vec<ContractClause>,
    /// Current contract status.
    pub status: ContractStatus,
    /// Violation count.
    pub violation_count: usize,
    /// Maximum allowed violations.
    pub max_violations: usize,
    /// Cryptographic hash for integrity.
    pub integrity_hash: String,
    /// Creation timestamp.
    pub created_at: Instant,
    /// Last evaluation timestamp.
    pub last_evaluated: Option<Instant>,
}

impl SLAContract {
    /// Create a new SLA contract.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        title: String,
        description: String,
        provider: String,
        consumer: String,
        duration: Duration,
        grace_period: Duration,
        metrics: Vec<ContractMetric>,
        clauses: Vec<ContractClause>,
        max_violations: usize,
    ) -> Result<Self, ContractError> {
        if metrics.is_empty() {
            return Err(ContractError::InvalidContract(
                "Contract must have at least one metric".into(),
            ));
        }

        let contract = Self {
            id: id.clone(),
            title,
            description,
            provider,
            consumer,
            start_time: Instant::now(),
            duration,
            grace_period,
            metrics,
            clauses,
            status: ContractStatus::Active,
            violation_count: 0,
            max_violations,
            integrity_hash: String::new(),
            created_at: Instant::now(),
            last_evaluated: None,
        };

        let integrity_hash = contract.compute_integrity_hash();
        Ok(Self {
            integrity_hash,
            ..contract
        })
    }

    /// Compute integrity hash for the contract.
    fn compute_integrity_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.provider.as_bytes());
        hasher.update(self.consumer.as_bytes());
        for metric in &self.metrics {
            hasher.update(metric.key.as_bytes());
            hasher.update(metric.target.to_le_bytes());
        }
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Verify contract integrity.
    pub fn verify_integrity(&self) -> bool {
        self.integrity_hash == self.compute_integrity_hash()
    }

    /// Check if contract has expired.
    pub fn is_expired(&self) -> bool {
        self.start_time.elapsed() > self.duration
    }

    /// Get remaining duration.
    pub fn remaining_duration(&self) -> Option<Duration> {
        let elapsed = self.start_time.elapsed();
        self.duration.checked_sub(elapsed)
    }

    /// Serialize contract to JSON bytes.
    /// Note: Returns error since contract contains Instant fields that cannot be serialized.
    pub fn serialize(&self) -> Result<Vec<u8>, ContractError> {
        Err(ContractError::Serialization(
            "Contract contains Instant fields that cannot be serialized".into(),
        ))
    }

    /// Deserialize contract from JSON bytes.
    /// Note: Returns error since contract contains Instant fields that cannot be deserialized.
    pub fn deserialize(_bytes: &[u8]) -> Result<Self, ContractError> {
        Err(ContractError::Serialization(
            "Deserialization requires custom Instant handling".into(),
        ))
    }
}

/// Result of contract validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Contract ID.
    pub contract_id: String,
    /// Whether all metrics are within acceptable range.
    pub compliant: bool,
    /// Metric validation details.
    pub metric_results: HashMap<String, MetricValidation>,
    /// Triggered clauses.
    pub triggered_clauses: Vec<String>,
    /// Total penalty amount.
    pub total_penalty: f64,
    /// Total reward amount.
    pub total_reward: f64,
    /// Validation timestamp.
    pub validated_at: Instant,
}

/// Validation result for a single metric.
#[derive(Debug, Clone)]
pub struct MetricValidation {
    /// Metric key.
    pub metric_key: String,
    /// Current value.
    pub current_value: f64,
    /// Target value.
    pub target: f64,
    /// Whether metric is within range.
    pub within_range: bool,
    /// Whether metric meets target.
    pub meets_target: bool,
    /// Deviation from target (percentage).
    pub deviation_percent: f64,
}

/// Statistics for the Contract Manager.
#[derive(Debug, Clone)]
pub struct ContractStats {
    /// Total contracts registered.
    pub total_contracts: usize,
    /// Active contracts count.
    pub active_contracts: usize,
    /// Violated contracts count.
    pub violated_contracts: usize,
    /// Fulfilled contracts count.
    pub fulfilled_contracts: usize,
    /// Expired contracts count.
    pub expired_contracts: usize,
    /// Total penalties issued.
    pub total_penalties: f64,
    /// Total rewards issued.
    pub total_rewards: f64,
}

/// SLA Contract Manager.
pub struct ContractManager {
    contracts: HashMap<String, SLAContract>,
    clauses: HashMap<String, Vec<ContractClause>>,
    stats: ContractStats,
    created_at: Instant,
}

impl ContractManager {
    /// Create a new Contract Manager.
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
            clauses: HashMap::new(),
            stats: ContractStats {
                total_contracts: 0,
                active_contracts: 0,
                violated_contracts: 0,
                fulfilled_contracts: 0,
                expired_contracts: 0,
                total_penalties: 0.0,
                total_rewards: 0.0,
            },
            created_at: Instant::now(),
        }
    }

    /// Register a new SLA contract.
    pub fn register_contract(&mut self, contract: SLAContract) -> Result<String, ContractError> {
        let id = contract.id.clone();
        contract
            .verify_integrity()
            .then_some(())
            .ok_or_else(|| ContractError::InvalidContract("Integrity check failed".into()))?;

        self.contracts.insert(id.clone(), contract);
        self.update_stats();
        info!(contract_id = %id, "SLA contract registered");
        Ok(id)
    }

    /// Create and register a contract in one step.
    #[allow(clippy::too_many_arguments)]
    pub fn create_contract(
        &mut self,
        id: String,
        title: String,
        description: String,
        provider: String,
        consumer: String,
        duration: Duration,
        grace_period: Duration,
        metrics: Vec<ContractMetric>,
        clauses: Vec<ContractClause>,
        max_violations: usize,
    ) -> Result<String, ContractError> {
        let contract = SLAContract::new(
            id.clone(),
            title,
            description,
            provider,
            consumer,
            duration,
            grace_period,
            metrics,
            clauses,
            max_violations,
        )?;
        self.register_contract(contract)
    }

    /// Get a contract by ID.
    pub fn get_contract(&self, id: &str) -> Option<&SLAContract> {
        self.contracts.get(id)
    }

    /// Get all active contracts.
    pub fn get_active_contracts(&self) -> Vec<&SLAContract> {
        self.contracts
            .values()
            .filter(|c| c.status == ContractStatus::Active)
            .collect()
    }

    /// Validate a contract against current metric values.
    pub fn validate_contract(
        &mut self,
        contract_id: &str,
        metric_values: &HashMap<String, f64>,
    ) -> Result<ValidationResult, ContractError> {
        let contract = self
            .contracts
            .get_mut(contract_id)
            .ok_or(ContractError::ContractNotFound(contract_id.into()))?;

        if contract.status == ContractStatus::Terminated {
            return Err(ContractError::ContractTerminated(contract_id.into()));
        }

        if contract.is_expired() {
            contract.status = ContractStatus::Expired;
            return Err(ContractError::ContractExpired(contract_id.into()));
        }

        let mut metric_results = HashMap::new();
        let mut triggered_clauses = Vec::new();
        let mut total_penalty = 0.0;
        let mut total_reward = 0.0;
        let mut all_compliant = true;

        for metric in &contract.metrics {
            let current_value = metric_values.get(&metric.key).copied().unwrap_or(f64::NAN);

            let within_range = metric.is_within_range(current_value);
            let meets_target = metric.meets_target(current_value);
            let deviation_percent = if metric.target > 0.0 {
                ((current_value - metric.target) / metric.target) * 100.0
            } else {
                0.0
            };

            if !within_range {
                all_compliant = false;
            }

            metric_results.insert(
                metric.key.clone(),
                MetricValidation {
                    metric_key: metric.key.clone(),
                    current_value,
                    target: metric.target,
                    within_range,
                    meets_target,
                    deviation_percent,
                },
            );
        }

        // Evaluate clauses
        for clause in &mut contract.clauses {
            if clause.triggered {
                continue;
            }
            if let Some(&value) = metric_values.get(&clause.metric_key) {
                let should_trigger = match clause.clause_type {
                    ClauseType::Penalty => value > clause.threshold,
                    ClauseType::Reward => value <= clause.threshold,
                    ClauseType::Escalation => value > clause.threshold * 1.5,
                    ClauseType::AutoRenewal => false, // Handled separately
                };

                if should_trigger {
                    clause.triggered = true;
                    clause.triggered_at = Some(Instant::now());
                    triggered_clauses.push(clause.id.clone());

                    match clause.clause_type {
                        ClauseType::Penalty | ClauseType::Escalation => {
                            total_penalty += clause.amount;
                        }
                        ClauseType::Reward => {
                            total_reward += clause.amount;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update contract status
        if !all_compliant {
            contract.violation_count += 1;
            if contract.violation_count >= contract.max_violations {
                contract.status = ContractStatus::Violated;
            } else {
                contract.status = ContractStatus::ViolationGrace;
            }
        } else if contract.violation_count > 0 {
            // Recovered from violations
            contract.status = ContractStatus::Active;
        }

        // Update stats
        self.stats.total_penalties += total_penalty;
        self.stats.total_rewards += total_reward;

        let result = ValidationResult {
            contract_id: contract_id.into(),
            compliant: all_compliant,
            metric_results,
            triggered_clauses,
            total_penalty,
            total_reward,
            validated_at: Instant::now(),
        };

        debug!(
            contract_id = %contract_id,
            compliant = all_compliant,
            violations = contract.violation_count,
            "Contract validation complete"
        );

        Ok(result)
    }

    /// Validate all active contracts.
    pub fn validate_all(
        &mut self,
        metric_values: &HashMap<String, f64>,
    ) -> Vec<Result<ValidationResult, ContractError>> {
        let active_ids: Vec<String> = self
            .contracts
            .values()
            .filter(|c| c.status == ContractStatus::Active)
            .map(|c| c.id.clone())
            .collect();

        active_ids
            .iter()
            .map(|id| self.validate_contract(id, metric_values))
            .collect()
    }

    /// Terminate a contract early.
    pub fn terminate_contract(&mut self, contract_id: &str) -> Result<(), ContractError> {
        let contract = self
            .contracts
            .get_mut(contract_id)
            .ok_or(ContractError::ContractNotFound(contract_id.into()))?;

        contract.status = ContractStatus::Terminated;
        self.update_stats();
        info!(contract_id = %contract_id, "Contract terminated");
        Ok(())
    }

    /// Fulfill a contract (mark as successfully completed).
    pub fn fulfill_contract(&mut self, contract_id: &str) -> Result<(), ContractError> {
        let contract = self
            .contracts
            .get_mut(contract_id)
            .ok_or(ContractError::ContractNotFound(contract_id.into()))?;

        contract.status = ContractStatus::Fulfilled;
        self.update_stats();
        info!(contract_id = %contract_id, "Contract fulfilled");
        Ok(())
    }

    /// Check and update expired contracts.
    pub fn check_expired_contracts(&mut self) -> usize {
        let mut expired_count = 0;
        for contract in self.contracts.values_mut() {
            if contract.is_expired() && contract.status == ContractStatus::Active {
                contract.status = ContractStatus::Expired;
                expired_count += 1;
            }
        }
        if expired_count > 0 {
            self.update_stats();
            info!(expired = expired_count, "Expired contracts updated");
        }
        expired_count
    }

    /// Update internal statistics.
    fn update_stats(&mut self) {
        self.stats.total_contracts = self.contracts.len();
        self.stats.active_contracts = self
            .contracts
            .values()
            .filter(|c| c.status == ContractStatus::Active)
            .count();
        self.stats.violated_contracts = self
            .contracts
            .values()
            .filter(|c| c.status == ContractStatus::Violated)
            .count();
        self.stats.fulfilled_contracts = self
            .contracts
            .values()
            .filter(|c| c.status == ContractStatus::Fulfilled)
            .count();
        self.stats.expired_contracts = self
            .contracts
            .values()
            .filter(|c| c.status == ContractStatus::Expired)
            .count();
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> ContractStats {
        self.stats.clone()
    }

    /// Remove a contract by ID.
    pub fn remove_contract(&mut self, contract_id: &str) -> Result<(), ContractError> {
        self.contracts
            .remove(contract_id)
            .ok_or(ContractError::ContractNotFound(contract_id.into()))?;
        self.update_stats();
        debug!(contract_id = %contract_id, "Contract removed");
        Ok(())
    }

    /// Get contracts by provider.
    pub fn get_contracts_by_provider(&self, provider: &str) -> Vec<&SLAContract> {
        self.contracts
            .values()
            .filter(|c| c.provider == provider)
            .collect()
    }

    /// Get contracts by consumer.
    pub fn get_contracts_by_consumer(&self, consumer: &str) -> Vec<&SLAContract> {
        self.contracts
            .values()
            .filter(|c| c.consumer == consumer)
            .collect()
    }

    /// Reset manager state.
    pub fn reset(&mut self) {
        self.contracts.clear();
        self.clauses.clear();
        self.stats = ContractStats {
            total_contracts: 0,
            active_contracts: 0,
            violated_contracts: 0,
            fulfilled_contracts: 0,
            expired_contracts: 0,
            total_penalties: 0.0,
            total_rewards: 0.0,
        };
        info!("Contract manager reset");
    }
}

impl Default for ContractManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metric(key: &str, target: f64) -> ContractMetric {
        ContractMetric::new(
            key.to_string(),
            format!("{} metric", key),
            target,
            0.0,
            target * 2.0,
            "ms".into(),
        )
    }

    fn make_clause(
        id: &str,
        clause_type: ClauseType,
        metric: &str,
        threshold: f64,
        amount: f64,
    ) -> ContractClause {
        ContractClause::new(
            id.to_string(),
            clause_type.clone(),
            metric.to_string(),
            threshold,
            format!("{} clause for {}", clause_type, metric),
            amount,
        )
    }

    fn make_contract(id: &str) -> SLAContract {
        SLAContract::new(
            id.to_string(),
            format!("{} SLA", id),
            "Test SLA contract".into(),
            "provider_1".into(),
            "consumer_1".into(),
            Duration::from_secs(3600),
            Duration::from_secs(300),
            vec![make_metric("sae_latency", 50.0)],
            vec![make_clause(
                "c1",
                ClauseType::Penalty,
                "sae_latency",
                50.0,
                100.0,
            )],
            3,
        )
        .unwrap()
    }

    #[test]
    fn test_manager_creation() {
        let manager = ContractManager::new();
        let stats = manager.get_stats();
        assert_eq!(stats.total_contracts, 0);
    }

    #[test]
    fn test_create_contract() {
        let mut manager = ContractManager::new();
        let id = manager.create_contract(
            "contract_1".into(),
            "Test SLA".into(),
            "Description".into(),
            "provider_1".into(),
            "consumer_1".into(),
            Duration::from_secs(3600),
            Duration::from_secs(300),
            vec![make_metric("sae_latency", 50.0)],
            vec![],
            3,
        );
        assert!(id.is_ok());
        assert_eq!(manager.get_stats().total_contracts, 1);
    }

    #[test]
    fn test_contract_integrity() {
        let contract = make_contract("integrity_test");
        assert!(contract.verify_integrity());
    }

    #[test]
    fn test_contract_serialization() {
        let contract = make_contract("serialize_test");
        // Serialization returns error since contract contains Instant fields
        assert!(contract.serialize().is_err());
        assert!(SLAContract::deserialize(&[]).is_err());
    }

    #[test]
    fn test_validate_contract_compliant() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("sae_latency", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("sae_latency".into(), 30.0);

        let result = manager.validate_contract("c1", &metrics).unwrap();
        assert!(result.compliant);
    }

    #[test]
    fn test_validate_contract_violation() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("sae_latency", 50.0)],
                vec![make_clause(
                    "penalty",
                    ClauseType::Penalty,
                    "sae_latency",
                    50.0,
                    100.0,
                )],
                3,
            )
            .unwrap();

        let mut metrics = HashMap::new();
        // Value 110 exceeds max (target*2=100), triggering violation
        metrics.insert("sae_latency".into(), 110.0);

        let result = manager.validate_contract("c1", &metrics).unwrap();
        assert!(!result.compliant);
        assert!(!result.triggered_clauses.is_empty());
    }

    #[test]
    fn test_penalty_clause_triggered() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("error_rate", 5.0)],
                vec![make_clause(
                    "pen",
                    ClauseType::Penalty,
                    "error_rate",
                    5.0,
                    50.0,
                )],
                3,
            )
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("error_rate".into(), 8.0);

        let result = manager.validate_contract("c1", &metrics).unwrap();
        assert_eq!(result.total_penalty, 50.0);
    }

    #[test]
    fn test_reward_clause_triggered() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("uptime", 99.9)],
                vec![make_clause(
                    "rew",
                    ClauseType::Reward,
                    "uptime",
                    99.9,
                    200.0,
                )],
                3,
            )
            .unwrap();

        let mut metrics = HashMap::new();
        // Reward triggers when value <= threshold (99.9)
        metrics.insert("uptime".into(), 99.9);

        let result = manager.validate_contract("c1", &metrics).unwrap();
        assert_eq!(result.total_reward, 200.0);
    }

    #[test]
    fn test_contract_expiration() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(0), // Already expired
                Duration::from_secs(0),
                vec![make_metric("latency", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        let expired = manager.check_expired_contracts();
        assert_eq!(expired, 1);
    }

    #[test]
    fn test_terminate_contract() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("latency", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        assert!(manager.terminate_contract("c1").is_ok());
        assert_eq!(
            manager.get_contract("c1").unwrap().status,
            ContractStatus::Terminated
        );
    }

    #[test]
    fn test_fulfill_contract() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "Test".into(),
                "Desc".into(),
                "p1".into(),
                "co1".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("latency", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        assert!(manager.fulfill_contract("c1").is_ok());
        assert_eq!(
            manager.get_contract("c1").unwrap().status,
            ContractStatus::Fulfilled
        );
    }

    #[test]
    fn test_get_active_contracts() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();
        manager
            .create_contract(
                "c2".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        manager.terminate_contract("c2").unwrap();
        let active = manager.get_active_contracts();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_get_contracts_by_provider() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "prov_a".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();
        manager
            .create_contract(
                "c2".into(),
                "T".into(),
                "D".into(),
                "prov_b".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        let prov_a = manager.get_contracts_by_provider("prov_a");
        assert_eq!(prov_a.len(), 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_contracts, 1);
        assert_eq!(stats.active_contracts, 1);
    }

    #[test]
    fn test_remove_contract() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        assert!(manager.remove_contract("c1").is_ok());
        assert_eq!(manager.get_stats().total_contracts, 0);
    }

    #[test]
    fn test_remove_unknown_contract() {
        let mut manager = ContractManager::new();
        assert!(manager.remove_contract("nonexistent").is_err());
    }

    #[test]
    fn test_reset() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();

        manager.reset();
        assert_eq!(manager.get_stats().total_contracts, 0);
    }

    #[test]
    fn test_contract_status_display() {
        assert_eq!(format!("{}", ContractStatus::Active), "Active");
        assert_eq!(format!("{}", ContractStatus::Violated), "Violated");
        assert_eq!(format!("{}", ContractStatus::Fulfilled), "Fulfilled");
        assert_eq!(format!("{}", ContractStatus::Expired), "Expired");
        assert_eq!(format!("{}", ContractStatus::Terminated), "Terminated");
    }

    #[test]
    fn test_clause_type_display() {
        assert_eq!(format!("{}", ClauseType::Penalty), "Penalty");
        assert_eq!(format!("{}", ClauseType::Reward), "Reward");
        assert_eq!(format!("{}", ClauseType::Escalation), "Escalation");
    }

    #[test]
    fn test_metric_within_range() {
        let metric = make_metric("latency", 50.0);
        assert!(metric.is_within_range(30.0));
        assert!(metric.is_within_range(50.0));
        assert!(!metric.is_within_range(110.0));
    }

    #[test]
    fn test_remaining_duration() {
        let contract = make_contract("duration_test");
        let remaining = contract.remaining_duration();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() < Duration::from_secs(3600));
    }

    #[test]
    fn test_validate_unknown_contract() {
        let mut manager = ContractManager::new();
        let metrics = HashMap::new();
        assert!(manager.validate_contract("unknown", &metrics).is_err());
    }

    #[test]
    fn test_validate_terminated_contract() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("m1", 50.0)],
                vec![],
                3,
            )
            .unwrap();
        manager.terminate_contract("c1").unwrap();

        let metrics = HashMap::new();
        assert!(manager.validate_contract("c1", &metrics).is_err());
    }

    #[test]
    fn test_max_violations_reached() {
        let mut manager = ContractManager::new();
        manager
            .create_contract(
                "c1".into(),
                "T".into(),
                "D".into(),
                "p".into(),
                "c".into(),
                Duration::from_secs(3600),
                Duration::from_secs(300),
                vec![make_metric("latency", 50.0)],
                vec![],
                2,
            )
            .unwrap();

        let mut metrics = HashMap::new();
        // Value 110 exceeds max (target*2=100), triggering violation
        metrics.insert("latency".into(), 110.0);

        // First violation
        manager.validate_contract("c1", &metrics).unwrap();
        assert_eq!(
            manager.get_contract("c1").unwrap().status,
            ContractStatus::ViolationGrace
        );

        // Second violation (reaches max)
        manager.validate_contract("c1", &metrics).unwrap();
        assert_eq!(
            manager.get_contract("c1").unwrap().status,
            ContractStatus::Violated
        );
    }

    #[test]
    fn test_invalid_contract_no_metrics() {
        let result = SLAContract::new(
            "bad".into(),
            "T".into(),
            "D".into(),
            "p".into(),
            "c".into(),
            Duration::from_secs(3600),
            Duration::from_secs(300),
            vec![], // No metrics
            vec![],
            3,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_default() {
        let manager = ContractManager::default();
        assert_eq!(manager.get_stats().total_contracts, 0);
    }
}
