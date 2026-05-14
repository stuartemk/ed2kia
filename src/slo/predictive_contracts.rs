//! Predictive Contracts — Contratos predictivos para SLO/SLA v3
//!
//! Define contratos binarios con evaluación basada en ventanas deslizantes
//! y predicción lineal. Soporta múltiples partes, penalizaciones automáticas
//! y escalado de severidad.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint3")]`

#[cfg(feature = "v1.2-sprint3")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.2-sprint3")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.2-sprint3")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "v1.2-sprint3")]
use thiserror::Error;
#[cfg(feature = "v1.2-sprint3")]
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Error)]
pub enum ContractError {
    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    #[error("Invalid penalty: {0}")]
    InvalidPenalty(String),

    #[error("Contract already expired")]
    ContractExpired,

    #[error("Insufficient data for evaluation")]
    InsufficientData,

    #[error("Part not found: {0}")]
    PartNotFound(String),
}

// ---------------------------------------------------------------------------
// Contract Status
// ---------------------------------------------------------------------------

/// Estado de un contrato predictivo.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContractStatus {
    Active,
    Warning,
    Breached,
    Resolved,
    Expired,
}

#[cfg(feature = "v1.2-sprint3")]
impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Active => write!(f, "Active"),
            ContractStatus::Warning => write!(f, "Warning"),
            ContractStatus::Breached => write!(f, "Breached"),
            ContractStatus::Resolved => write!(f, "Resolved"),
            ContractStatus::Expired => write!(f, "Expired"),
        }
    }
}

// ---------------------------------------------------------------------------
// Contract Part
// ---------------------------------------------------------------------------

/// Parte en un contrato (proveedor o consumidor).
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPart {
    pub id: String,
    pub role: String,
    pub weight: f64,
}

#[cfg(feature = "v1.2-sprint3")]
impl ContractPart {
    pub fn new(id: String, role: String, weight: f64) -> Self {
        Self { id, role, weight }
    }
}

// ---------------------------------------------------------------------------
// Penalty Record
// ---------------------------------------------------------------------------

/// Registro de penalización aplicada.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyRecord {
    pub timestamp: u64,
    pub severity: u32,
    pub amount: f64,
    pub reason: String,
}

#[cfg(feature = "v1.2-sprint3")]
impl PenaltyRecord {
    pub fn new(severity: u32, amount: f64, reason: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            timestamp,
            severity,
            amount,
            reason,
        }
    }
}

// ---------------------------------------------------------------------------
// Predictive Contract Config
// ---------------------------------------------------------------------------

/// Configuración de un contrato predictivo.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveContractConfig {
    /// ID único del contrato.
    pub contract_id: String,
    /// SLO asociado.
    pub slo_name: String,
    /// Valor objetivo del SLO.
    pub target: f64,
    /// Umbral de advertencia.
    pub warning_threshold: f64,
    /// Umbral de breach.
    pub breach_threshold: f64,
    /// Penalización base por breach.
    pub base_penalty: f64,
    /// Multiplicador de severidad por breach consecutivo.
    pub severity_multiplier: f64,
    /// Ventana de evaluación en segundos.
    pub evaluation_window: u64,
    /// Duración del contrato en segundos.
    pub duration_seconds: u64,
    /// Puntos mínimos para predicción.
    pub min_prediction_points: usize,
}

#[cfg(feature = "v1.2-sprint3")]
impl Default for PredictiveContractConfig {
    fn default() -> Self {
        Self {
            contract_id: Default::default(),
            slo_name: Default::default(),
            target: 99.9,
            warning_threshold: 0.95,
            breach_threshold: 0.90,
            base_penalty: 100.0,
            severity_multiplier: 1.5,
            evaluation_window: 30,
            duration_seconds: 86400,
            min_prediction_points: 10,
        }
    }
}

// ---------------------------------------------------------------------------
// Contract Evaluation Result
// ---------------------------------------------------------------------------

/// Resultado de evaluación de contrato.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvaluation {
    pub status: ContractStatus,
    pub current_value: f64,
    pub predicted_value: f64,
    pub confidence: f64,
    pub severity: u32,
    pub penalty: f64,
    pub action: String,
}

// ---------------------------------------------------------------------------
// Predictive Contract
// ---------------------------------------------------------------------------

/// Contrato predictivo con evaluación automática.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveContract {
    pub config: PredictiveContractConfig,
    pub parts: Vec<ContractPart>,
    pub status: ContractStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub metric_history: VecDeque<f64>,
    pub consecutive_breaches: u32,
    pub penalties: Vec<PenaltyRecord>,
    pub total_penalty: f64,
}

#[cfg(feature = "v1.2-sprint3")]
impl PredictiveContract {
    pub fn new(config: PredictiveContractConfig, parts: Vec<ContractPart>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            config: config.clone(),
            parts,
            status: ContractStatus::Active,
            created_at: now,
            expires_at: now + config.duration_seconds,
            metric_history: VecDeque::with_capacity(config.min_prediction_points.max(60)),
            consecutive_breaches: 0,
            penalties: Vec::new(),
            total_penalty: 0.0,
        }
    }

    /// Verifica si el contrato ha expirado.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }

    /// Registra un valor de métrica.
    pub fn record_metric(&mut self, value: f64) {
        self.metric_history.push_back(value);
    }

    /// Calcula predicción lineal simple.
    fn predict(&self) -> Option<(f64, f64)> {
        let history: Vec<f64> = self.metric_history.iter().cloned().collect();
        let n = history.len();
        let min_points = self.config.min_prediction_points;

        if n < min_points {
            return None;
        }

        let sum_x: f64 = (0..n).map(|i| i as f64).sum();
        let sum_y: f64 = history.iter().sum();
        let sum_xy: f64 = (0..n).map(|i| i as f64 * history[i]).sum();
        let sum_x2: f64 = (0..n).map(|i| {
            let x = i as f64;
            x * x
        }).sum();

        let denom = n as f64 * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 {
            return None;
        }

        let b = (n as f64 * sum_xy - sum_x * sum_y) / denom;
        let a = (sum_y - b * sum_x) / n as f64;
        let predicted = a + b * (n as f64);

        // R²
        let mean_y = sum_y / n as f64;
        let ss_tot: f64 = history.iter().map(|y| (y - mean_y).powi(2)).sum();
        let ss_res: f64 = history
            .iter()
            .enumerate()
            .map(|(i, y)| {
                let p = a + b * i as f64;
                (y - p).powi(2)
            })
            .sum();

        let r_squared = if ss_tot.abs() < 1e-10 {
            0.0
        } else {
            (1.0 - ss_res / ss_tot).clamp(0.0, 1.0)
        };

        Some((predicted, r_squared))
    }

    /// Evalúa el contrato y aplica penalizaciones si es necesario.
    pub fn evaluate(&mut self) -> Result<ContractEvaluation, ContractError> {
        if self.is_expired() {
            self.status = ContractStatus::Expired;
            return Err(ContractError::ContractExpired);
        }

        if self.metric_history.is_empty() {
            return Err(ContractError::InsufficientData);
        }

        let current = *self.metric_history.back().unwrap();
        let target = self.config.target;
        let warning = target * self.config.warning_threshold;
        let breach = target * self.config.breach_threshold;

        let (predicted, confidence) = self.predict().unwrap_or((current, 0.0));

        let (status, severity, penalty, action) = if current >= target {
            self.consecutive_breaches = 0;
            self.status = ContractStatus::Active;
            (ContractStatus::Active, 0, 0.0, "none")
        } else if current >= warning {
            self.status = ContractStatus::Warning;
            (ContractStatus::Warning, 0, 0.0, "alert")
        } else if current >= breach {
            self.consecutive_breaches += 1;
            let severity = self.consecutive_breaches;
            let penalty = self.config.base_penalty
                * self.config.severity_multiplier.powi((severity - 1) as i32);
            self.total_penalty += penalty;

            let record = PenaltyRecord::new(
                severity,
                penalty,
                format!("Breach #{}: value={:.2}", severity, current),
            );
            self.penalties.push(record);
            self.status = ContractStatus::Breached;
            (ContractStatus::Breached, severity, penalty, "penalty_applied")
        } else {
            self.consecutive_breaches += 1;
            let severity = self.consecutive_breaches;
            let penalty = self.config.base_penalty
                * self.config.severity_multiplier.powi((severity - 1) as i32)
                * 2.0;
            self.total_penalty += penalty;

            let record = PenaltyRecord::new(
                severity,
                penalty,
                format!("Critical breach #{}: value={:.2}", severity, current),
            );
            self.penalties.push(record);
            self.status = ContractStatus::Breached;
            (ContractStatus::Breached, severity, penalty, "critical_penalty")
        };

        Ok(ContractEvaluation {
            status,
            current_value: current,
            predicted_value: predicted,
            confidence,
            severity,
            penalty,
            action: action.to_string(),
        })
    }

    /// Resuelve el contrato manualmente.
    pub fn resolve(&mut self) {
        self.status = ContractStatus::Resolved;
        info!(
            contract = %self.config.contract_id,
            penalty = self.total_penalty,
            "contract resolved"
        );
    }
}

// ---------------------------------------------------------------------------
// Contract Manager
// ---------------------------------------------------------------------------

/// Gestor de contratos predictivos.
#[cfg(feature = "v1.2-sprint3")]
pub struct ContractManager {
    contracts: HashMap<String, PredictiveContract>,
}

#[cfg(feature = "v1.2-sprint3")]
impl ContractManager {
    /// Crea un nuevo gestor.
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    /// Registra un contrato.
    pub fn register_contract(
        &mut self,
        config: PredictiveContractConfig,
        parts: Vec<ContractPart>,
    ) {
        let contract = PredictiveContract::new(config.clone(), parts);
        self.contracts.insert(config.contract_id.clone(), contract);
        info!(contract = %config.contract_id, "contract registered");
    }

    /// Remueve un contrato.
    pub fn remove_contract(&mut self, id: &str) -> Result<(), ContractError> {
        self.contracts
            .remove(id)
            .map(|_| ())
            .ok_or(ContractError::ContractNotFound(id.to_string()))
    }

    /// Registra métrica para un contrato.
    pub fn record_metric(&mut self, contract_id: &str, value: f64) -> Result<(), ContractError> {
        let contract = self
            .contracts
            .get_mut(contract_id)
            .ok_or(ContractError::ContractNotFound(contract_id.to_string()))?;
        contract.record_metric(value);
        debug!(contract = %contract_id, value, "metric recorded");
        Ok(())
    }

    /// Evalúa un contrato.
    pub fn evaluate_contract(
        &mut self,
        contract_id: &str,
    ) -> Result<ContractEvaluation, ContractError> {
        let contract = self
            .contracts
            .get_mut(contract_id)
            .ok_or(ContractError::ContractNotFound(contract_id.to_string()))?;
        contract.evaluate()
    }

    /// Evalúa todos los contratos.
    pub fn evaluate_all(&mut self) -> HashMap<String, Result<ContractEvaluation, ContractError>> {
        let ids: Vec<String> = self.contracts.keys().cloned().collect();
        let mut results = HashMap::new();
        for id in ids {
            results.insert(id.clone(), self.evaluate_contract(&id));
        }
        results
    }

    /// Obtiene un contrato.
    pub fn get_contract(&self, id: &str) -> Option<&PredictiveContract> {
        self.contracts.get(id)
    }

    /// Retorna el número de contratos activos.
    pub fn active_count(&self) -> usize {
        self.contracts
            .values()
            .filter(|c| c.status == ContractStatus::Active)
            .count()
    }

    /// Retorna el total de penalizaciones.
    pub fn total_penalties(&self) -> f64 {
        self.contracts.values().map(|c| c.total_penalty).sum()
    }
}

#[cfg(feature = "v1.2-sprint3")]
impl Default for ContractManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(id: &str) -> PredictiveContractConfig {
        PredictiveContractConfig {
            contract_id: id.to_string(),
            slo_name: "latency".into(),
            target: 100.0,
            warning_threshold: 0.8,
            breach_threshold: 0.6,
            base_penalty: 100.0,
            severity_multiplier: 1.5,
            evaluation_window: 30,
            duration_seconds: 86400,
            min_prediction_points: 5,
        }
    }

    fn make_parts() -> Vec<ContractPart> {
        vec![
            ContractPart::new("provider-1".into(), "provider".into(), 1.0),
            ContractPart::new("consumer-1".into(), "consumer".into(), 1.0),
        ]
    }

    #[test]
    fn test_manager_creation() {
        let manager = ContractManager::new();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_register_contract() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        assert!(manager.get_contract("c1").is_some());
    }

    #[test]
    fn test_remove_contract() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        assert!(manager.remove_contract("c1").is_ok());
        assert!(manager.get_contract("c1").is_none());
    }

    #[test]
    fn test_remove_missing() {
        let mut manager = ContractManager::new();
        assert!(manager.remove_contract("missing").is_err());
    }

    #[test]
    fn test_record_metric() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        assert!(manager.record_metric("c1", 95.0).is_ok());
    }

    #[test]
    fn test_evaluate_compliant() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for _ in 0..10 {
            manager.record_metric("c1", 100.0).unwrap();
        }
        let result = manager.evaluate_contract("c1").unwrap();
        assert_eq!(result.status, ContractStatus::Active);
        assert_eq!(result.penalty, 0.0);
    }

    #[test]
    fn test_evaluate_warning() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for _ in 0..10 {
            manager.record_metric("c1", 85.0).unwrap();
        }
        let result = manager.evaluate_contract("c1").unwrap();
        assert_eq!(result.status, ContractStatus::Warning);
    }

    #[test]
    fn test_evaluate_breach() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for _ in 0..10 {
            manager.record_metric("c1", 50.0).unwrap();
        }
        let result = manager.evaluate_contract("c1").unwrap();
        assert_eq!(result.status, ContractStatus::Breached);
        assert!(result.penalty > 0.0);
    }

    #[test]
    fn test_escalating_penalty() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for _ in 0..5 {
            manager.record_metric("c1", 50.0).unwrap();
            manager.evaluate_contract("c1").unwrap();
        }
        let contract = manager.get_contract("c1").unwrap();
        assert!(contract.total_penalty > 100.0);
    }

    #[test]
    fn test_resolve_contract() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for _ in 0..10 {
            manager.record_metric("c1", 50.0).unwrap();
        }
        manager.evaluate_contract("c1").unwrap();
        let contract = manager.contracts.get_mut("c1").unwrap();
        contract.resolve();
        assert_eq!(contract.status, ContractStatus::Resolved);
    }

    #[test]
    fn test_evaluate_all() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("a"), make_parts());
        manager.register_contract(make_config("b"), make_parts());
        manager.record_metric("a", 100.0).unwrap();
        manager.record_metric("b", 50.0).unwrap();
        let results = manager.evaluate_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_prediction() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for i in 0..10 {
            manager.record_metric("c1", (100 - i) as f64).unwrap();
        }
        let result = manager.evaluate_contract("c1").unwrap();
        assert!(result.predicted_value < result.current_value);
    }

    #[test]
    fn test_contract_expiration() {
        let config = PredictiveContractConfig {
            duration_seconds: 0,
            ..make_config("c1")
        };
        let mut manager = ContractManager::new();
        manager.register_contract(config, make_parts());
        manager.record_metric("c1", 100.0).unwrap();
        assert!(manager.evaluate_contract("c1").is_err());
    }

    #[test]
    fn test_total_penalties() {
        let mut manager = ContractManager::new();
        manager.register_contract(make_config("c1"), make_parts());
        for _ in 0..10 {
            manager.record_metric("c1", 50.0).unwrap();
        }
        manager.evaluate_contract("c1").unwrap();
        assert!(manager.total_penalties() > 0.0);
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", ContractStatus::Active), "Active");
        assert_eq!(format!("{}", ContractStatus::Breached), "Breached");
    }

    #[test]
    fn test_error_display() {
        let err = ContractError::ContractNotFound("x".into());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_config_default() {
        let config = PredictiveContractConfig::default();
        assert_eq!(config.target, 99.9);
    }

    #[test]
    fn test_manager_default() {
        let manager = ContractManager::default();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_penalty_record() {
        let record = PenaltyRecord::new(1, 100.0, "test".into());
        assert_eq!(record.severity, 1);
        assert_eq!(record.amount, 100.0);
    }

    #[test]
    fn test_part_creation() {
        let part = ContractPart::new("p1".into(), "provider".into(), 1.0);
        assert_eq!(part.id, "p1");
    }
}
