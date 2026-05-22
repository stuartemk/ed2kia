//! Dashboard v3 — Motor de estado unificado para cross-chain, DAO y fine-tuning
//!
//! Agrega métricas de consenso cross-chain, gobernanza DAO, fine-tuning distribuido
//! y contratos SLO/SLA predictivos en un único snapshot unificado.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

/// Error del dashboard v3
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DashboardV3Error {
    #[error("Métrica no disponible: {0}")]
    MetricUnavailable(String),
    #[error("Error de agregación: {0}")]
    AggregationError(String),
    #[error("Límite de tasa excedido")]
    RateLimitExceeded,
    #[error("Fuente de datos no registrada: {0}")]
    DataSourceNotRegistered(String),
}

/// Tipo de métrica del dashboard v3
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetricV3 {
    // Cross-Chain
    CrossChainActiveProposals,
    CrossChainValidatorsOnline,
    CrossChainAvgConfirmationTime,
    CrossChainBridgeTps,
    // DAO
    DaoActiveProposals,
    DaoVoterParticipation,
    DaoQuorumRate,
    DaoExecutionPending,
    // Fine-Tuning
    TrainingEpochProgress,
    TrainingLoss,
    TrainingGradientNorm,
    TrainingNodesActive,
    // SLO/SLA
    SloComplianceRate,
    SloPredictiveBreaches,
    SloWarningCount,
    // General
    NetworkLatencyP99,
    NetworkThroughput,
    NetworkErrorRate,
}

impl std::fmt::Display for MetricV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricV3::CrossChainActiveProposals => write!(f, "cross_chain.active_proposals"),
            MetricV3::CrossChainValidatorsOnline => write!(f, "cross_chain.validators_online"),
            MetricV3::CrossChainAvgConfirmationTime => {
                write!(f, "cross_chain.avg_confirmation_time")
            }
            MetricV3::CrossChainBridgeTps => write!(f, "cross_chain.bridge_tps"),
            MetricV3::DaoActiveProposals => write!(f, "dao.active_proposals"),
            MetricV3::DaoVoterParticipation => write!(f, "dao.voter_participation"),
            MetricV3::DaoQuorumRate => write!(f, "dao.quorum_rate"),
            MetricV3::DaoExecutionPending => write!(f, "dao.execution_pending"),
            MetricV3::TrainingEpochProgress => write!(f, "training.epoch_progress"),
            MetricV3::TrainingLoss => write!(f, "training.loss"),
            MetricV3::TrainingGradientNorm => write!(f, "training.gradient_norm"),
            MetricV3::TrainingNodesActive => write!(f, "training.nodes_active"),
            MetricV3::SloComplianceRate => write!(f, "slo.compliance_rate"),
            MetricV3::SloPredictiveBreaches => write!(f, "slo.predictive_breaches"),
            MetricV3::SloWarningCount => write!(f, "slo.warning_count"),
            MetricV3::NetworkLatencyP99 => write!(f, "network.latency_p99"),
            MetricV3::NetworkThroughput => write!(f, "network.throughput"),
            MetricV3::NetworkErrorRate => write!(f, "network.error_rate"),
        }
    }
}

/// Valor de métrica con metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValueV3 {
    pub metric: MetricV3,
    pub value: f64,
    pub timestamp_ms: u64,
    pub source: Option<String>,
}

impl MetricValueV3 {
    pub fn new(metric: MetricV3, value: f64, source: Option<String>) -> Self {
        Self {
            metric,
            value,
            timestamp_ms: current_timestamp_ms(),
            source,
        }
    }
}

/// Categoría de fuente de datos
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataSource {
    CrossChainConsensus,
    BridgeValidator,
    DaoGovernance,
    DistributedTraining,
    SloEngine,
    Custom(String),
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataSource::CrossChainConsensus => write!(f, "cross_chain_consensus"),
            DataSource::BridgeValidator => write!(f, "bridge_validator"),
            DataSource::DaoGovernance => write!(f, "dao_governance"),
            DataSource::DistributedTraining => write!(f, "distributed_training"),
            DataSource::SloEngine => write!(f, "slo_engine"),
            DataSource::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Resumen de sección cross-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainSummary {
    pub active_proposals: usize,
    pub validators_online: usize,
    pub avg_confirmation_time_ms: f64,
    pub bridge_tps: f64,
}

/// Resumen de sección DAO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoSummary {
    pub active_proposals: usize,
    pub voter_participation_rate: f64,
    pub quorum_rate: f64,
    pub execution_pending: usize,
}

/// Resumen de sección fine-tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSummary {
    pub epoch_progress: f64,
    pub current_loss: f64,
    pub gradient_norm: f64,
    pub nodes_active: usize,
}

/// Resumen de sección SLO/SLA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloSummary {
    pub compliance_rate: f64,
    pub predictive_breaches: usize,
    pub warning_count: usize,
}

/// Snapshot unificado del dashboard v3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV3Snapshot {
    pub timestamp_ms: u64,
    pub cross_chain: CrossChainSummary,
    pub dao: DaoSummary,
    pub training: TrainingSummary,
    pub slo: SloSummary,
    pub network_latency_p99_ms: f64,
    pub network_throughput: f64,
    pub network_error_rate: f64,
    pub total_metrics: usize,
    pub alert_count: usize,
}

/// Configuración del dashboard v3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV3Config {
    pub max_metrics_history: usize,
    pub rate_limit_per_second: usize,
    pub alert_threshold_loss: f64,
    pub alert_threshold_latency_ms: f64,
    pub alert_threshold_error_rate: f64,
}

impl Default for DashboardV3Config {
    fn default() -> Self {
        Self {
            max_metrics_history: 1000,
            rate_limit_per_second: 50,
            alert_threshold_loss: 2.0,
            alert_threshold_latency_ms: 500.0,
            alert_threshold_error_rate: 0.05,
        }
    }
}

/// Alerta generada por el dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub alert_id: String,
    pub metric: MetricV3,
    pub value: f64,
    pub threshold: f64,
    pub message: String,
    pub timestamp_ms: u64,
    pub acknowledged: bool,
}

/// Estado del dashboard v3
pub struct DashboardV3State {
    config: DashboardV3Config,
    metrics: Vec<MetricValueV3>,
    alerts: Vec<DashboardAlert>,
    alert_counter: u64,
    last_rate_check_ms: u64,
    requests_this_second: usize,
}

impl DashboardV3State {
    pub fn new() -> Self {
        Self::with_config(DashboardV3Config::default())
    }

    pub fn with_config(config: DashboardV3Config) -> Self {
        Self {
            config,
            metrics: Vec::new(),
            alerts: Vec::new(),
            alert_counter: 0,
            last_rate_check_ms: current_timestamp_ms(),
            requests_this_second: 0,
        }
    }

    /// Registra una métrica
    pub fn record_metric(&mut self, metric: MetricV3, value: f64, source: Option<String>) {
        self.check_rate_limit();
        let entry = MetricValueV3::new(metric.clone(), value, source);
        self.metrics.push(entry);
        self.enforce_history_limit();
        self.check_alerts(&metric, value);
        debug!("Métrica registrada: {} = {}", metric, value);
    }

    /// Genera un snapshot unificado
    pub fn get_snapshot(&mut self) -> Result<DashboardV3Snapshot, DashboardV3Error> {
        self.check_rate_limit();

        let cross_chain = self.aggregate_cross_chain();
        let dao = self.aggregate_dao();
        let training = self.aggregate_training();
        let slo = self.aggregate_slo();

        let latency = self.get_metric_value(&MetricV3::NetworkLatencyP99, 0.0);
        let throughput = self.get_metric_value(&MetricV3::NetworkThroughput, 0.0);
        let error_rate = self.get_metric_value(&MetricV3::NetworkErrorRate, 0.0);

        let unacknowledged_alerts = self.alerts.iter().filter(|a| !a.acknowledged).count();

        Ok(DashboardV3Snapshot {
            timestamp_ms: current_timestamp_ms(),
            cross_chain,
            dao,
            training,
            slo,
            network_latency_p99_ms: latency,
            network_throughput: throughput,
            network_error_rate: error_rate,
            total_metrics: self.metrics.len(),
            alert_count: unacknowledged_alerts,
        })
    }

    /// Acknowledge una alerta
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> bool {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.acknowledged = true;
            info!("Alerta reconocida: {}", alert_id);
            true
        } else {
            false
        }
    }

    /// Obtiene alertas no reconocidas
    pub fn get_active_alerts(&self) -> Vec<&DashboardAlert> {
        self.alerts.iter().filter(|a| !a.acknowledged).collect()
    }

    /// Limpia alertas antiguas
    pub fn clear_old_alerts(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let before = self.alerts.len();
        self.alerts
            .retain(|a| now.saturating_sub(a.timestamp_ms) <= max_age_ms);
        before - self.alerts.len()
    }

    fn aggregate_cross_chain(&self) -> CrossChainSummary {
        CrossChainSummary {
            active_proposals: self.get_metric_value(&MetricV3::CrossChainActiveProposals, 0.0)
                as usize,
            validators_online: self.get_metric_value(&MetricV3::CrossChainValidatorsOnline, 0.0)
                as usize,
            avg_confirmation_time_ms: self
                .get_metric_value(&MetricV3::CrossChainAvgConfirmationTime, 0.0),
            bridge_tps: self.get_metric_value(&MetricV3::CrossChainBridgeTps, 0.0),
        }
    }

    fn aggregate_dao(&self) -> DaoSummary {
        DaoSummary {
            active_proposals: self.get_metric_value(&MetricV3::DaoActiveProposals, 0.0) as usize,
            voter_participation_rate: self.get_metric_value(&MetricV3::DaoVoterParticipation, 0.0),
            quorum_rate: self.get_metric_value(&MetricV3::DaoQuorumRate, 0.0),
            execution_pending: self.get_metric_value(&MetricV3::DaoExecutionPending, 0.0) as usize,
        }
    }

    fn aggregate_training(&self) -> TrainingSummary {
        TrainingSummary {
            epoch_progress: self.get_metric_value(&MetricV3::TrainingEpochProgress, 0.0),
            current_loss: self.get_metric_value(&MetricV3::TrainingLoss, 0.0),
            gradient_norm: self.get_metric_value(&MetricV3::TrainingGradientNorm, 0.0),
            nodes_active: self.get_metric_value(&MetricV3::TrainingNodesActive, 0.0) as usize,
        }
    }

    fn aggregate_slo(&self) -> SloSummary {
        SloSummary {
            compliance_rate: self.get_metric_value(&MetricV3::SloComplianceRate, 1.0),
            predictive_breaches: self.get_metric_value(&MetricV3::SloPredictiveBreaches, 0.0)
                as usize,
            warning_count: self.get_metric_value(&MetricV3::SloWarningCount, 0.0) as usize,
        }
    }

    fn get_metric_value(&self, metric: &MetricV3, default: f64) -> f64 {
        self.metrics
            .iter()
            .rev()
            .find(|m| &m.metric == metric)
            .map(|m| m.value)
            .unwrap_or(default)
    }

    fn check_alerts(&mut self, metric: &MetricV3, value: f64) {
        match metric {
            MetricV3::TrainingLoss => {
                if value > self.config.alert_threshold_loss {
                    self.create_alert(
                        metric.clone(),
                        value,
                        self.config.alert_threshold_loss,
                        format!(
                            "Pérdida de entrenamiento ({:.4}) supera el umbral ({})",
                            value, self.config.alert_threshold_loss
                        ),
                    );
                }
            }
            MetricV3::NetworkLatencyP99 => {
                if value > self.config.alert_threshold_latency_ms {
                    self.create_alert(
                        metric.clone(),
                        value,
                        self.config.alert_threshold_latency_ms,
                        format!(
                            "Latencia P99 ({:.1}ms) supera el umbral ({}ms)",
                            value, self.config.alert_threshold_latency_ms
                        ),
                    );
                }
            }
            MetricV3::NetworkErrorRate => {
                if value > self.config.alert_threshold_error_rate {
                    self.create_alert(
                        metric.clone(),
                        value,
                        self.config.alert_threshold_error_rate,
                        format!(
                            "Tasa de error ({:.4}) supera el umbral ({})",
                            value, self.config.alert_threshold_error_rate
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    fn create_alert(&mut self, metric: MetricV3, value: f64, threshold: f64, message: String) {
        self.alert_counter += 1;
        let alert = DashboardAlert {
            alert_id: format!("alert-{}", self.alert_counter),
            metric,
            value,
            threshold,
            message,
            timestamp_ms: current_timestamp_ms(),
            acknowledged: false,
        };
        self.alerts.push(alert);
    }

    fn check_rate_limit(&mut self) {
        let now = current_timestamp_ms();
        if now.saturating_sub(self.last_rate_check_ms) >= 1000 {
            self.last_rate_check_ms = now;
            self.requests_this_second = 0;
        }
        self.requests_this_second += 1;
    }

    fn enforce_history_limit(&mut self) {
        if self.metrics.len() > self.config.max_metrics_history {
            let excess = self.metrics.len() - self.config.max_metrics_history;
            self.metrics.drain(..excess);
        }
    }
}

impl Default for DashboardV3State {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = DashboardV3State::new();
        assert_eq!(dashboard.metrics.len(), 0);
        assert_eq!(dashboard.alerts.len(), 0);
    }

    #[test]
    fn test_dashboard_with_config() {
        let config = DashboardV3Config {
            max_metrics_history: 100,
            ..Default::default()
        };
        let dashboard = DashboardV3State::with_config(config);
        assert_eq!(dashboard.config.max_metrics_history, 100);
    }

    #[test]
    fn test_record_metric() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 0.5, Some("trainer".into()));
        assert_eq!(dashboard.metrics.len(), 1);
    }

    #[test]
    fn test_snapshot_generation() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 0.5, None);
        dashboard.record_metric(MetricV3::TrainingNodesActive, 5.0, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.total_metrics, 2);
        assert_eq!(snapshot.training.current_loss, 0.5);
        assert_eq!(snapshot.training.nodes_active, 5);
    }

    #[test]
    fn test_alert_generation() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 3.0, None);
        let alerts = dashboard.get_active_alerts();
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_alert_acknowledgment() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 3.0, None);
        let alert_id = dashboard.alerts[0].alert_id.clone();
        assert!(dashboard.acknowledge_alert(&alert_id));
        assert_eq!(dashboard.get_active_alerts().len(), 0);
    }

    #[test]
    fn test_clear_old_alerts() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 3.0, None);
        // Use max_age_ms=1 to ensure the alert (created just now) gets cleaned
        std::thread::sleep(std::time::Duration::from_millis(2));
        let removed = dashboard.clear_old_alerts(1);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_metric_v3_display() {
        let metric = MetricV3::TrainingLoss;
        assert_eq!(metric.to_string(), "training.loss");
    }

    #[test]
    fn test_data_source_display() {
        let source = DataSource::CrossChainConsensus;
        assert_eq!(source.to_string(), "cross_chain_consensus");
    }

    #[test]
    fn test_config_default() {
        let config = DashboardV3Config::default();
        assert_eq!(config.max_metrics_history, 1000);
        assert_eq!(config.rate_limit_per_second, 50);
    }

    #[test]
    fn test_dashboard_default() {
        let dashboard = DashboardV3State::default();
        assert_eq!(dashboard.metrics.len(), 0);
    }

    #[test]
    fn test_history_limit_enforcement() {
        let config = DashboardV3Config {
            max_metrics_history: 5,
            ..Default::default()
        };
        let mut dashboard = DashboardV3State::with_config(config);
        for i in 0..10 {
            dashboard.record_metric(MetricV3::TrainingLoss, i as f64, None);
        }
        assert_eq!(dashboard.metrics.len(), 5);
    }

    #[test]
    fn test_multiple_alerts() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 3.0, None);
        dashboard.record_metric(MetricV3::NetworkLatencyP99, 600.0, None);
        assert_eq!(dashboard.get_active_alerts().len(), 2);
    }

    #[test]
    fn test_snapshot_defaults() {
        let mut dashboard = DashboardV3State::new();
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.cross_chain.active_proposals, 0);
        assert_eq!(snapshot.dao.active_proposals, 0);
        assert_eq!(snapshot.training.current_loss, 0.0);
        assert_eq!(snapshot.slo.compliance_rate, 1.0);
    }

    #[test]
    fn test_no_alert_for_normal_values() {
        let mut dashboard = DashboardV3State::new();
        dashboard.record_metric(MetricV3::TrainingLoss, 0.5, None);
        assert_eq!(dashboard.get_active_alerts().len(), 0);
    }
}
